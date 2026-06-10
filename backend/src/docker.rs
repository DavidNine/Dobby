//! B9 — Docker container management.
//!
//! Thin async wrapper over the local Docker daemon (via `bollard`, talking to
//! the `/var/run/docker.sock` unix socket) exposing:
//!
//!   - `GET  /api/docker/containers`              — list all containers,
//!   - `GET  /api/docker/containers/{id}`          — inspect one container
//!     (ports with host bindings, mounts, networks, command, restart policy…),
//!   - `POST /api/docker/containers/{id}/{action}` — lifecycle action, where
//!     `action ∈ { restart, start, stop }`.
//!
//! Graceful degradation: when the daemon is missing/unreachable the *list*
//! endpoint still returns HTTP 200 with `available:false` and an `error`
//! message, so the UI can render a clear "Docker unavailable" state instead of
//! a hard failure. Action endpoints, being explicit mutations, surface daemon
//! errors as 5xx.
//!
//! Security note: reaching the Docker socket is effectively root-level control
//! of the host. This shares the same unauthenticated `0.0.0.0` surface as the
//! terminal endpoint; the risk ceiling is unchanged (that is already a root
//! shell), but if/when auth is added it must cover these POST actions too.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bollard::errors::Error as DockerError;
use bollard::models::{ContainerInspectResponse, ContainerSummary};
use bollard::query_parameters::{InspectContainerOptions, ListContainersOptionsBuilder};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::api::AppState;

// ---------------------------------------------------------------------------
// Response DTOs
// ---------------------------------------------------------------------------

/// A published port mapping on a container.
#[derive(Debug, Serialize)]
struct PortDto {
    private: u16,
    public: Option<u16>,
    #[serde(rename = "type")]
    proto: String,
}

/// One container row.
#[derive(Debug, Serialize)]
struct ContainerDto {
    /// Full container id.
    id: String,
    /// Primary name without Docker's leading `/` (falls back to the short id).
    name: String,
    /// Image reference, e.g. `nginx:1.27`.
    image: String,
    /// Canonical lowercase state: created|running|paused|restarting|exited|
    /// removing|dead|stopping (or `unknown`). Drives the status light.
    state: String,
    /// Human-readable detail from Docker, e.g. `Up 2 hours`.
    status: String,
    /// Creation time, Unix epoch seconds.
    created: i64,
    /// Published port mappings.
    ports: Vec<PortDto>,
}

/// `GET /api/docker/containers` response.
#[derive(Debug, Serialize)]
struct ContainersResponse {
    /// Whether the Docker daemon was reachable.
    available: bool,
    /// Error detail when `available` is false; otherwise null.
    error: Option<String>,
    containers: Vec<ContainerDto>,
}

/// `POST /api/docker/containers/{id}/{action}` success response.
#[derive(Debug, Serialize)]
struct ActionResponse {
    ok: bool,
    id: String,
    action: String,
}

impl From<ContainerSummary> for ContainerDto {
    fn from(c: ContainerSummary) -> Self {
        let id = c.id.unwrap_or_default();

        // Docker names look like ["/web"]; take the first and strip the slash.
        let name = c
            .names
            .and_then(|names| names.into_iter().next())
            .map(|n| n.trim_start_matches('/').to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| id.chars().take(12).collect());

        // Docker lists each mapping once per address family (IPv4 + IPv6), so
        // the same private/public/proto can appear twice. A BTreeSet dedupes
        // and gives a stable order.
        let ports = c
            .ports
            .unwrap_or_default()
            .into_iter()
            .map(|p| {
                let proto = p
                    .typ
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "tcp".to_string());
                (p.private_port, p.public_port, proto)
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|(private, public, proto)| PortDto {
                private,
                public,
                proto,
            })
            .collect();

        ContainerDto {
            id,
            name,
            image: c.image.unwrap_or_default(),
            state: c
                .state
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            status: c.status.unwrap_or_default(),
            created: c.created.unwrap_or(0),
            ports,
        }
    }
}

// --- Container detail (inspect) DTOs ---------------------------------------

/// One published-port binding row in the detail view. Unlike the list's
/// [`PortDto`], every host binding (per address) is kept, so e.g. an IPv4 and
/// an IPv6 binding of the same container port show up as two rows.
#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct PortBindingDto {
    /// Port inside the container.
    container_port: u16,
    /// `tcp` | `udp` | `sctp`.
    protocol: String,
    /// Host address the port is bound to (null for an exposed-only port).
    host_ip: Option<String>,
    /// Host port (null for an exposed-only port).
    host_port: Option<u16>,
}

/// A filesystem mount on the container (bind mount, volume, tmpfs…).
#[derive(Debug, Serialize)]
struct MountDto {
    /// `bind` | `volume` | `tmpfs` | `npipe` | …
    #[serde(rename = "type")]
    typ: Option<String>,
    /// Volume name when the mount is a named volume.
    name: Option<String>,
    /// Host-side source path (empty/null for tmpfs).
    source: Option<String>,
    /// Path inside the container.
    destination: Option<String>,
    /// User-supplied mount options, e.g. `ro,z`.
    mode: Option<String>,
    /// Whether the mount is writable.
    rw: Option<bool>,
}

/// The container's attachment to one Docker network.
#[derive(Debug, Serialize)]
struct NetworkDto {
    /// Network name, e.g. `bridge` or a compose network.
    name: String,
    ip_address: Option<String>,
    gateway: Option<String>,
    mac_address: Option<String>,
}

/// `GET /api/docker/containers/{id}` response — the detail subset of
/// `docker inspect` the UI renders.
#[derive(Debug, Serialize)]
struct ContainerDetailsDto {
    id: String,
    name: String,
    /// Image reference from the container config (falls back to the digest).
    image: String,
    /// Canonical lowercase state, as in the list endpoint.
    state: String,
    /// Exit code of the last run (meaningful for exited containers).
    exit_code: Option<i64>,
    /// RFC 3339 creation time.
    created: Option<String>,
    /// RFC 3339 time of the last start / exit.
    started_at: Option<String>,
    finished_at: Option<String>,
    /// Restart policy name, e.g. `unless-stopped` (null when unset/`no`).
    restart_policy: Option<String>,
    restart_count: i64,
    platform: Option<String>,
    /// Resolved command line: path + args.
    command: Option<String>,
    working_dir: Option<String>,
    env: Vec<String>,
    /// BTreeMap for a stable, sorted serialisation order.
    labels: BTreeMap<String, String>,
    ports: Vec<PortBindingDto>,
    mounts: Vec<MountDto>,
    networks: Vec<NetworkDto>,
}

impl From<ContainerInspectResponse> for ContainerDetailsDto {
    fn from(c: ContainerInspectResponse) -> Self {
        let id = c.id.unwrap_or_default();
        let name = c
            .name
            .map(|n| n.trim_start_matches('/').to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| id.chars().take(12).collect());

        let config = c.config;
        let state = c.state;
        let host_config = c.host_config;
        let network_settings = c.network_settings;

        // Friendly image ref lives in Config.Image; top-level Image is a digest.
        let image = config
            .as_ref()
            .and_then(|cfg| cfg.image.clone())
            .or(c.image)
            .unwrap_or_default();

        // PortMap: "80/tcp" -> Option<Vec<PortBinding>>. A key with no bindings
        // is an exposed-but-unpublished port; keep it with null host fields.
        let mut ports: Vec<PortBindingDto> = network_settings
            .as_ref()
            .and_then(|ns| ns.ports.clone())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(key, bindings)| {
                let (port, proto) = key.split_once('/')?;
                let container_port: u16 = port.parse().ok()?;
                let protocol = proto.to_string();
                let rows = match bindings {
                    Some(list) if !list.is_empty() => list
                        .into_iter()
                        .map(|b| PortBindingDto {
                            container_port,
                            protocol: protocol.clone(),
                            host_ip: b.host_ip.filter(|ip| !ip.is_empty()),
                            host_port: b.host_port.and_then(|p| p.parse().ok()),
                        })
                        .collect(),
                    _ => vec![PortBindingDto {
                        container_port,
                        protocol,
                        host_ip: None,
                        host_port: None,
                    }],
                };
                Some(rows)
            })
            .flatten()
            .collect();
        ports.sort();
        ports.dedup();

        let mut mounts: Vec<MountDto> = c
            .mounts
            .unwrap_or_default()
            .into_iter()
            .map(|m| MountDto {
                typ: m.typ,
                name: m.name,
                source: m.source.filter(|s| !s.is_empty()),
                destination: m.destination,
                mode: m.mode.filter(|s| !s.is_empty()),
                rw: m.rw,
            })
            .collect();
        mounts.sort_by(|a, b| a.destination.cmp(&b.destination));

        let mut networks: Vec<NetworkDto> = network_settings
            .and_then(|ns| ns.networks)
            .unwrap_or_default()
            .into_iter()
            .map(|(network_name, ep)| NetworkDto {
                name: network_name,
                ip_address: ep.ip_address.filter(|s| !s.is_empty()),
                gateway: ep.gateway.filter(|s| !s.is_empty()),
                mac_address: ep.mac_address.filter(|s| !s.is_empty()),
            })
            .collect();
        networks.sort_by(|a, b| a.name.cmp(&b.name));

        // Resolved command: the actual path + args Docker ran.
        let command = c.path.map(|path| {
            let args = c.args.unwrap_or_default();
            if args.is_empty() {
                path
            } else {
                format!("{path} {}", args.join(" "))
            }
        });

        // Treat Docker's default "no"/empty policy as "none set".
        let restart_policy = host_config
            .as_ref()
            .and_then(|hc| hc.restart_policy.as_ref())
            .and_then(|rp| rp.name)
            .map(|n| n.to_string())
            .filter(|n| !n.is_empty() && n != "no");

        ContainerDetailsDto {
            id,
            name,
            image,
            state: state
                .as_ref()
                .and_then(|s| s.status)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            exit_code: state.as_ref().and_then(|s| s.exit_code),
            created: c.created,
            started_at: state
                .as_ref()
                .and_then(|s| s.started_at.clone())
                .filter(|t| !t.starts_with("0001-")),
            finished_at: state
                .and_then(|s| s.finished_at)
                .filter(|t| !t.starts_with("0001-")),
            restart_policy,
            restart_count: c.restart_count.unwrap_or(0),
            platform: c.platform,
            command,
            working_dir: config
                .as_ref()
                .and_then(|cfg| cfg.working_dir.clone())
                .filter(|s| !s.is_empty()),
            env: config
                .as_ref()
                .and_then(|cfg| cfg.env.clone())
                .unwrap_or_default(),
            labels: config
                .and_then(|cfg| cfg.labels)
                .map(|l| l.into_iter().collect())
                .unwrap_or_default(),
            ports,
            mounts,
            networks,
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/docker/containers` — list every container (running or not).
pub async fn list_containers(State(state): State<AppState>) -> Response {
    let Some(docker) = state.docker.as_ref() else {
        return unavailable("Docker client was not initialised");
    };

    let options = ListContainersOptionsBuilder::new().all(true).build();
    match docker.list_containers(Some(options)).await {
        Ok(list) => {
            let mut containers: Vec<ContainerDto> =
                list.into_iter().map(ContainerDto::from).collect();
            // Newest first — stable, predictable ordering for the table.
            containers.sort_by(|a, b| b.created.cmp(&a.created));
            Json(ContainersResponse {
                available: true,
                error: None,
                containers,
            })
            .into_response()
        }
        // Daemon down / socket missing: degrade gracefully (HTTP 200).
        Err(e) => unavailable(&e.to_string()),
    }
}

/// `GET /api/docker/containers/{id}` — detailed info for one container.
///
/// Unlike the list endpoint this does not degrade to a 200: the UI only asks
/// for details from an already-rendered row, so daemon errors surface as 5xx
/// (and an unknown id as Docker's own 404).
pub async fn inspect_container(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let Some(docker) = state.docker.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Docker client was not initialised" })),
        )
            .into_response();
    };

    match docker
        .inspect_container(&id, None::<InspectContainerOptions>)
        .await
    {
        Ok(details) => Json(ContainerDetailsDto::from(details)).into_response(),
        Err(e) => map_docker_error(e),
    }
}

/// `POST /api/docker/containers/{id}/{action}` — run a lifecycle action.
pub async fn container_action(
    State(state): State<AppState>,
    Path((id, action)): Path<(String, String)>,
) -> Response {
    let Some(docker) = state.docker.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Docker client was not initialised" })),
        )
            .into_response();
    };

    let result = match action.as_str() {
        "restart" => docker.restart_container(&id, None).await,
        "start" => docker.start_container(&id, None).await,
        "stop" => docker.stop_container(&id, None).await,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("unsupported action: {other:?} (expected restart|start|stop)")
                })),
            )
                .into_response();
        }
    };

    match result {
        Ok(()) => Json(ActionResponse {
            ok: true,
            id,
            action,
        })
        .into_response(),
        // Docker replies 304 Not Modified when the container is already in the
        // requested state (e.g. start an already-running one): treat as success.
        Err(DockerError::DockerResponseServerError {
            status_code: 304, ..
        }) => Json(ActionResponse {
            ok: true,
            id,
            action,
        })
        .into_response(),
        Err(e) => map_docker_error(e),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a 200 "daemon unavailable" list response (graceful degradation).
fn unavailable(message: &str) -> Response {
    Json(ContainersResponse {
        available: false,
        error: Some(message.to_string()),
        containers: Vec::new(),
    })
    .into_response()
}

/// Map a Docker action error to an HTTP status: preserve Docker's own status
/// (e.g. 404 for an unknown container), default to 502 Bad Gateway otherwise.
fn map_docker_error(e: DockerError) -> Response {
    let status = match &e {
        DockerError::DockerResponseServerError { status_code, .. } => {
            StatusCode::from_u16(*status_code).unwrap_or(StatusCode::BAD_GATEWAY)
        }
        _ => StatusCode::BAD_GATEWAY,
    };
    (status, Json(serde_json::json!({ "error": e.to_string() }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard::models::{
        ContainerConfig, ContainerState, ContainerStateStatusEnum, EndpointSettings, HostConfig,
        MountPoint, NetworkSettings, PortBinding, RestartPolicy, RestartPolicyNameEnum,
    };
    use std::collections::HashMap;

    /// A representative `docker inspect` payload covering ports (published,
    /// dual-stack and exposed-only), mounts, networks and host config.
    fn inspect_fixture() -> ContainerInspectResponse {
        let mut ports = HashMap::new();
        ports.insert(
            "80/tcp".to_string(),
            Some(vec![
                PortBinding {
                    host_ip: Some("0.0.0.0".into()),
                    host_port: Some("8080".into()),
                },
                PortBinding {
                    host_ip: Some("::".into()),
                    host_port: Some("8080".into()),
                },
            ]),
        );
        // Exposed but not published.
        ports.insert("9090/tcp".to_string(), None);

        let mut networks = HashMap::new();
        networks.insert(
            "bridge".to_string(),
            EndpointSettings {
                ip_address: Some("172.17.0.2".into()),
                gateway: Some("172.17.0.1".into()),
                mac_address: Some("02:42:ac:11:00:02".into()),
                ..Default::default()
            },
        );

        ContainerInspectResponse {
            id: Some("deadbeef1234".into()),
            name: Some("/web".into()),
            created: Some("2026-06-01T10:00:00.000000000Z".into()),
            path: Some("nginx".into()),
            args: Some(vec!["-g".into(), "daemon off;".into()]),
            restart_count: Some(2),
            platform: Some("linux".into()),
            state: Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                exit_code: Some(0),
                started_at: Some("2026-06-10T08:00:00Z".into()),
                finished_at: Some("0001-01-01T00:00:00Z".into()),
                ..Default::default()
            }),
            host_config: Some(HostConfig {
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                    maximum_retry_count: None,
                }),
                ..Default::default()
            }),
            config: Some(ContainerConfig {
                image: Some("nginx:1.27".into()),
                working_dir: Some("/srv".into()),
                env: Some(vec!["PATH=/usr/bin".into()]),
                labels: Some(HashMap::from([(
                    "com.example.app".to_string(),
                    "web".to_string(),
                )])),
                ..Default::default()
            }),
            mounts: Some(vec![MountPoint {
                typ: Some("bind".into()),
                source: Some("/srv/data".into()),
                destination: Some("/data".into()),
                mode: Some("rw".into()),
                rw: Some(true),
                ..Default::default()
            }]),
            network_settings: Some(NetworkSettings {
                ports: Some(ports),
                networks: Some(networks),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn details_dto_maps_core_fields() {
        let dto = ContainerDetailsDto::from(inspect_fixture());

        assert_eq!(dto.id, "deadbeef1234");
        assert_eq!(dto.name, "web"); // leading slash stripped
        assert_eq!(dto.image, "nginx:1.27"); // Config.Image, not the digest
        assert_eq!(dto.state, "running");
        assert_eq!(dto.command.as_deref(), Some("nginx -g daemon off;"));
        assert_eq!(dto.restart_policy.as_deref(), Some("unless-stopped"));
        assert_eq!(dto.restart_count, 2);
        assert_eq!(dto.working_dir.as_deref(), Some("/srv"));
        assert_eq!(dto.started_at.as_deref(), Some("2026-06-10T08:00:00Z"));
        // Docker's zero time means "never finished" → null.
        assert_eq!(dto.finished_at, None);
        assert_eq!(dto.env, vec!["PATH=/usr/bin".to_string()]);
        assert_eq!(dto.labels.get("com.example.app").unwrap(), "web");
    }

    #[test]
    fn details_dto_maps_ports_mounts_networks() {
        let dto = ContainerDetailsDto::from(inspect_fixture());

        // 80/tcp keeps both host bindings; 9090/tcp survives as exposed-only.
        assert_eq!(dto.ports.len(), 3);
        assert_eq!(dto.ports[0].container_port, 80);
        assert_eq!(dto.ports[0].host_ip.as_deref(), Some("0.0.0.0"));
        assert_eq!(dto.ports[0].host_port, Some(8080));
        assert_eq!(dto.ports[1].host_ip.as_deref(), Some("::"));
        assert_eq!(dto.ports[2].container_port, 9090);
        assert_eq!(dto.ports[2].host_port, None);

        assert_eq!(dto.mounts.len(), 1);
        let m = &dto.mounts[0];
        assert_eq!(m.typ.as_deref(), Some("bind"));
        assert_eq!(m.source.as_deref(), Some("/srv/data"));
        assert_eq!(m.destination.as_deref(), Some("/data"));
        assert_eq!(m.rw, Some(true));

        assert_eq!(dto.networks.len(), 1);
        let n = &dto.networks[0];
        assert_eq!(n.name, "bridge");
        assert_eq!(n.ip_address.as_deref(), Some("172.17.0.2"));
        assert_eq!(n.gateway.as_deref(), Some("172.17.0.1"));
    }

    #[test]
    fn details_dto_handles_empty_inspect() {
        let dto = ContainerDetailsDto::from(ContainerInspectResponse {
            id: Some("cafebabe000011112222".into()),
            ..Default::default()
        });

        assert_eq!(dto.name, "cafebabe0000"); // falls back to the short id
        assert_eq!(dto.state, "unknown");
        assert_eq!(dto.command, None);
        assert_eq!(dto.restart_policy, None);
        assert!(dto.ports.is_empty());
        assert!(dto.mounts.is_empty());
        assert!(dto.networks.is_empty());
    }
}
