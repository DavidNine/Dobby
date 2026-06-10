//! B6 — HTTP API.
//!
//! axum router, request handlers, response DTOs and CORS. Depends only on the
//! [`MetricsRepository`] abstraction (never touches the DB directly).
//!
//! The repository methods are synchronous (rusqlite under the hood), so every
//! repo call is offloaded onto a blocking thread via
//! [`tokio::task::spawn_blocking`] to keep the async runtime responsive.

use std::str::FromStr;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use crate::domain::{AppError, MetricPoint, MetricSample, TimeRange};
use crate::storage::MetricsRepository;

/// Shared, cloneable application state injected into handlers.
///
/// `pub(crate)` so the B9 Docker handlers (a sibling module) can read `docker`.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) repo: Arc<dyn MetricsRepository + Send + Sync>,
    /// Docker daemon client, or `None` when the socket could not be set up.
    /// `bollard::Docker` is internally `Arc`-backed, so cloning `AppState` is
    /// cheap and shares one client.
    pub(crate) docker: Option<bollard::Docker>,
}

// ---------------------------------------------------------------------------
// Response DTOs (match HLD §B6 JSON exactly)
// ---------------------------------------------------------------------------

/// CPU sub-object of [`CurrentResponse`].
#[derive(Debug, Serialize)]
struct CpuDto {
    percent: f32,
}

/// Memory sub-object of [`CurrentResponse`].
#[derive(Debug, Serialize)]
struct MemDto {
    total_bytes: u64,
    used_bytes: u64,
    percent: f32,
}

/// Network sub-object of [`CurrentResponse`].
#[derive(Debug, Serialize)]
struct NetDto {
    rx_bps: f64,
    tx_bps: f64,
}

/// `GET /api/metrics/current` response body.
#[derive(Debug, Serialize)]
struct CurrentResponse {
    timestamp: i64,
    cpu: CpuDto,
    memory: MemDto,
    network: NetDto,
}

impl From<MetricSample> for CurrentResponse {
    fn from(s: MetricSample) -> Self {
        CurrentResponse {
            timestamp: s.timestamp,
            cpu: CpuDto {
                percent: s.cpu_percent,
            },
            memory: MemDto {
                total_bytes: s.mem_total_bytes,
                used_bytes: s.mem_used_bytes,
                percent: s.mem_percent,
            },
            network: NetDto {
                rx_bps: s.net_rx_rate_bps,
                tx_bps: s.net_tx_rate_bps,
            },
        }
    }
}

/// A single downsampled point in [`HistoryResponse`].
#[derive(Debug, Serialize)]
struct PointDto {
    timestamp: i64,
    cpu_percent: f32,
    mem_percent: f32,
    net_rx_bps: f64,
    net_tx_bps: f64,
}

impl From<MetricPoint> for PointDto {
    fn from(p: MetricPoint) -> Self {
        PointDto {
            timestamp: p.timestamp,
            cpu_percent: p.cpu_percent,
            mem_percent: p.mem_percent,
            net_rx_bps: p.net_rx_bps,
            net_tx_bps: p.net_tx_bps,
        }
    }
}

/// `GET /api/metrics/history` response body.
#[derive(Debug, Serialize)]
struct HistoryResponse {
    range: String,
    bucket_secs: i64,
    points: Vec<PointDto>,
}

/// Query parameters for the history endpoint.
#[derive(Debug, Deserialize)]
struct HistoryQuery {
    /// Canonical range string; absent → 400 (treated as invalid).
    range: Option<String>,
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

/// HTTP-layer error, rendered as `{ "error": <message> }` with a status code.
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn internal(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        // Every AppError variant maps to a 500 with its display message.
        ApiError::internal(e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

/// Run a synchronous repository closure on a blocking thread, mapping both the
/// join error and the repository [`AppError`] into an [`ApiError`] (500).
async fn run_blocking<T, F>(f: F) -> Result<T, ApiError>
where
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(repo_result) => repo_result.map_err(ApiError::from),
        Err(join_err) => Err(ApiError::internal(format!(
            "internal task error: {join_err}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/health` → `{ "status": "ok" }`.
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

/// `GET /api/metrics/current` → latest sample, or 204 when there is none.
async fn current(State(state): State<AppState>) -> Result<Response, ApiError> {
    let repo = state.repo.clone();
    let latest = run_blocking(move || repo.latest()).await?;
    match latest {
        Some(sample) => Ok(Json(CurrentResponse::from(sample)).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

/// `GET /api/metrics/history?range=...` → downsampled series.
async fn history(
    State(state): State<AppState>,
    Query(q): Query<HistoryQuery>,
) -> Result<Response, ApiError> {
    let raw = q
        .range
        .ok_or_else(|| ApiError::bad_request("missing required query parameter: range"))?;
    let range = TimeRange::from_str(&raw)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    let repo = state.repo.clone();
    let points = run_blocking(move || repo.query_range(range)).await?;

    let body = HistoryResponse {
        range: range.as_str().to_string(),
        bucket_secs: range.bucket_secs(),
        points: points.into_iter().map(PointDto::from).collect(),
    };
    Ok(Json(body).into_response())
}

// ---------------------------------------------------------------------------
// Router + CORS
// ---------------------------------------------------------------------------

/// Build the CORS layer from the configured origins string.
///
/// `"*"` → permissive (any origin); otherwise the comma-separated list of
/// origins is parsed into an explicit allow-list. Only `GET` is allowed.
fn build_cors(cors_origins: &str) -> CorsLayer {
    use axum::http::Method;

    // GET for metrics/list reads; POST for Docker lifecycle actions (B9).
    let layer = CorsLayer::new().allow_methods([Method::GET, Method::POST]);

    if cors_origins.trim() == "*" {
        return layer.allow_origin(Any);
    }

    let origins: Vec<axum::http::HeaderValue> = cors_origins
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    if origins.is_empty() {
        // Misconfigured / unparseable list: fall back to permissive rather
        // than silently denying every browser request.
        layer.allow_origin(Any)
    } else {
        layer.allow_origin(origins)
    }
}

/// Build the axum [`Router`] with all routes, shared state and CORS applied.
pub fn build_router(
    repo: Arc<dyn MetricsRepository + Send + Sync>,
    cors_origins: &str,
) -> Router {
    // Set up the Docker client (B9). `connect_with_socket_defaults` only builds
    // the client; an unreachable daemon surfaces lazily at request time, where
    // handlers degrade gracefully — so a missing daemon never breaks startup.
    let docker = match bollard::Docker::connect_with_socket_defaults() {
        Ok(client) => Some(client),
        Err(e) => {
            tracing::warn!(error = %e, "docker client unavailable; container endpoints will report unavailable");
            None
        }
    };

    let state = AppState { repo, docker };

    Router::new()
        .route("/api/health", get(health))
        .route("/api/metrics/current", get(current))
        .route("/api/metrics/history", get(history))
        // B8 — interactive terminal over a WebSocket (PTY-backed bash).
        .route("/ws/terminal", get(crate::terminal::terminal_ws))
        // B9 — Docker container management.
        .route(
            "/api/docker/containers",
            get(crate::docker::list_containers),
        )
        .route(
            "/api/docker/containers/{id}",
            get(crate::docker::inspect_container),
        )
        .route(
            "/api/docker/containers/{id}/{action}",
            post(crate::docker::container_action),
        )
        .layer(build_cors(cors_origins))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `oneshot`

    /// Configurable test double for [`MetricsRepository`].
    ///
    /// Each method returns the preset value; setting an `err` makes the
    /// corresponding read return `AppError::Storage` to exercise the 500 path.
    struct FakeRepo {
        latest: Mutex<Result<Option<MetricSample>, AppError>>,
        points: Mutex<Result<Vec<MetricPoint>, AppError>>,
    }

    impl FakeRepo {
        fn with_latest(sample: Option<MetricSample>) -> Self {
            FakeRepo {
                latest: Mutex::new(Ok(sample)),
                points: Mutex::new(Ok(Vec::new())),
            }
        }

        fn with_points(points: Vec<MetricPoint>) -> Self {
            FakeRepo {
                latest: Mutex::new(Ok(None)),
                points: Mutex::new(Ok(points)),
            }
        }

        fn failing() -> Self {
            FakeRepo {
                latest: Mutex::new(Err(AppError::Storage("boom".into()))),
                points: Mutex::new(Err(AppError::Storage("boom".into()))),
            }
        }
    }

    // Clone the stored Result so the trait's `&self` methods can hand back an
    // owned value each call without consuming the preset.
    fn clone_result<T: Clone>(r: &Result<T, AppError>) -> Result<T, AppError> {
        match r {
            Ok(v) => Ok(v.clone()),
            Err(e) => Err(AppError::Storage(e.to_string())),
        }
    }

    impl MetricsRepository for FakeRepo {
        fn insert(&self, _sample: &MetricSample) -> Result<(), AppError> {
            Ok(())
        }
        fn latest(&self) -> Result<Option<MetricSample>, AppError> {
            clone_result(&self.latest.lock().unwrap())
        }
        fn query_range(&self, _range: TimeRange) -> Result<Vec<MetricPoint>, AppError> {
            clone_result(&self.points.lock().unwrap())
        }
        fn delete_older_than(&self, _cutoff_ts: i64) -> Result<u64, AppError> {
            Ok(0)
        }
    }

    fn sample() -> MetricSample {
        MetricSample {
            timestamp: 1_717_400_000,
            cpu_percent: 23.5,
            mem_total_bytes: 17_179_869_184,
            mem_used_bytes: 8_589_934_592,
            mem_percent: 50.0,
            net_rx_rate_bps: 125_000.0,
            net_tx_rate_bps: 34_000.0,
        }
    }

    fn router_with(repo: FakeRepo) -> Router {
        build_router(Arc::new(repo), "*")
    }

    /// Send a GET request through the router and return (status, parsed JSON).
    async fn get_json(router: Router, uri: &str) -> (StatusCode, serde_json::Value) {
        let resp = router
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, json)
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let (status, json) =
            get_json(router_with(FakeRepo::with_latest(None)), "/api/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json, serde_json::json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn current_with_data_returns_200_and_correct_structure() {
        let (status, json) = get_json(
            router_with(FakeRepo::with_latest(Some(sample()))),
            "/api/metrics/current",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json,
            serde_json::json!({
                "timestamp": 1_717_400_000_i64,
                "cpu": { "percent": 23.5 },
                "memory": {
                    "total_bytes": 17_179_869_184_u64,
                    "used_bytes": 8_589_934_592_u64,
                    "percent": 50.0
                },
                "network": { "rx_bps": 125_000.0, "tx_bps": 34_000.0 }
            })
        );
    }

    #[tokio::test]
    async fn current_with_no_data_returns_204() {
        let (status, json) = get_json(
            router_with(FakeRepo::with_latest(None)),
            "/api/metrics/current",
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        // 204 carries an empty body.
        assert_eq!(json, serde_json::Value::Null);
    }

    #[tokio::test]
    async fn history_24h_returns_200_with_structure_and_bucket_secs() {
        let points = vec![MetricPoint {
            timestamp: 1_717_310_000,
            cpu_percent: 20.1,
            mem_percent: 48.0,
            net_rx_bps: 100_000.0,
            net_tx_bps: 20_000.0,
        }];
        let (status, json) = get_json(
            router_with(FakeRepo::with_points(points)),
            "/api/metrics/history?range=24h",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["range"], "24h");
        assert_eq!(json["bucket_secs"], 240);
        let pts = json["points"].as_array().unwrap();
        assert_eq!(pts.len(), 1);
        assert_eq!(
            pts[0],
            serde_json::json!({
                "timestamp": 1_717_310_000_i64,
                "cpu_percent": 20.1,
                "mem_percent": 48.0,
                "net_rx_bps": 100_000.0,
                "net_tx_bps": 20_000.0
            })
        );
    }

    #[tokio::test]
    async fn history_with_no_data_returns_empty_array() {
        let (status, json) = get_json(
            router_with(FakeRepo::with_points(Vec::new())),
            "/api/metrics/history?range=1h",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["points"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn history_invalid_range_returns_400() {
        let (status, json) = get_json(
            router_with(FakeRepo::with_points(Vec::new())),
            "/api/metrics/history?range=99x",
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json.get("error").is_some());
    }

    #[tokio::test]
    async fn history_missing_range_returns_400() {
        let (status, json) = get_json(
            router_with(FakeRepo::with_points(Vec::new())),
            "/api/metrics/history",
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json.get("error").is_some());
    }

    #[tokio::test]
    async fn current_repo_error_returns_500_with_error_body() {
        let (status, json) =
            get_json(router_with(FakeRepo::failing()), "/api/metrics/current").await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(json.get("error").is_some());
    }

    #[tokio::test]
    async fn history_repo_error_returns_500_with_error_body() {
        let (status, json) = get_json(
            router_with(FakeRepo::failing()),
            "/api/metrics/history?range=6h",
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(json.get("error").is_some());
    }
}
