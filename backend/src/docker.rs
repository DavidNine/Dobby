//! B9 — Docker container management.
//!
//! Thin async wrapper over the local Docker daemon (via `bollard`, talking to
//! the `/var/run/docker.sock` unix socket) exposing:
//!
//!   - `GET  /api/docker/containers`              — list all containers,
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
use bollard::models::ContainerSummary;
use bollard::query_parameters::ListContainersOptionsBuilder;
use serde::Serialize;

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
