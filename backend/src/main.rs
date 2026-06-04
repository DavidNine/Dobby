//! dobby-backend — single-machine system-monitoring dashboard backend.
//!
//! B7 — assembly entry point. Wires the independent modules into a runnable
//! binary:
//!
//! 1. initialise `tracing` logging,
//! 2. load [`Config`] from the environment,
//! 3. open the SQLite [`SqliteRepository`] (shared via `Arc`),
//! 4. construct a [`SysinfoCollector`],
//! 5. spawn the background [`Sampler`] (sample + cleanup loops),
//! 6. build the axum [`Router`] (B6) injecting the shared repo + CORS,
//! 7. bind a TCP listener and serve, with graceful shutdown on Ctrl-C.

// Several module APIs (e.g. `FakeCollector`, `SqliteRepository::open_in_memory`,
// `Sampler::run`) plus a collector import are exercised only by unit tests, so
// they read as dead code / unused in the plain binary build. Silence those
// without resorting to `deny(warnings)`.
#![allow(dead_code)]
#![allow(unused)]

mod domain;
mod config;
mod collector;
mod storage;
mod sampler;
mod api;
mod terminal;
mod docker;

use std::sync::Arc;

use crate::collector::SysinfoCollector;
use crate::config::Config;
use crate::sampler::Sampler;
use crate::storage::{MetricsRepository, SqliteRepository};

#[tokio::main]
async fn main() {
    // 1. Logging. A plain `fmt` subscriber at INFO; `tracing-subscriber`'s
    //    `env-filter` feature is not enabled, so we set a fixed default level
    //    rather than parsing `RUST_LOG`.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    if let Err(e) = run().await {
        tracing::error!(error = %e, "fatal: backend failed to start");
        std::process::exit(1);
    }
}

/// Application body, separated from `main` so failures bubble up as a single
/// `Result` that `main` logs and turns into a non-zero exit code.
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 2. Configuration.
    let config = Config::load()?;
    tracing::info!(
        bind = %config.bind,
        port = config.port,
        sample_interval_secs = config.sample_interval_secs,
        retention_days = config.retention_days,
        cleanup_interval_secs = config.cleanup_interval_secs,
        db_path = %config.db_path,
        "configuration loaded"
    );

    // 3. Storage: open (creates table + index) and share as a trait object.
    let repo: Arc<dyn MetricsRepository + Send + Sync> =
        Arc::new(SqliteRepository::open(&config.db_path)?);
    tracing::info!(db_path = %config.db_path, "storage opened");

    // 4. Collector.
    let collector = Box::new(SysinfoCollector::new());

    // 5. Background sampler (sample + cleanup loops). It clones the repo Arc
    //    internally, so we hand it a clone and keep our own for the HTTP layer.
    Sampler::new(collector, Arc::clone(&repo), &config).spawn();
    tracing::info!(
        sample_interval_secs = config.sample_interval_secs,
        cleanup_interval_secs = config.cleanup_interval_secs,
        "sampler started"
    );

    // 6. HTTP router (B6) with the shared repo + CORS.
    let router = api::build_router(Arc::clone(&repo), &config.cors_origins);

    // 7. Bind and serve.
    let addr = format!("{}:{}", config.bind, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let local_addr = listener.local_addr()?;
    tracing::info!(%local_addr, "HTTP API listening");

    // 8. Graceful shutdown on Ctrl-C.
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("shutdown complete");
    Ok(())
}

/// Resolve once Ctrl-C (SIGINT) is received, triggering graceful shutdown.
async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => tracing::info!("Ctrl-C received; shutting down gracefully"),
        Err(e) => tracing::error!(error = %e, "failed to install Ctrl-C handler"),
    }
}
