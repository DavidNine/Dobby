//! B2 — Config.
//!
//! Loads and validates tunable parameters from environment variables with
//! sensible defaults. Pure logic, no I/O beyond reading the process env.

use crate::domain::AppError;

/// All tunable parameters for the backend, loaded from the environment.
#[derive(Debug, Clone)]
pub struct Config {
    /// Bind address (env `MONITOR_BIND`, default `0.0.0.0`).
    pub bind: String,
    /// Listen port (env `MONITOR_PORT`, default `8080`).
    pub port: u16,
    /// Metrics sampling interval in seconds (env `MONITOR_SAMPLE_INTERVAL_SECS`, default `10`).
    pub sample_interval_secs: u64,
    /// Data retention window in days (env `MONITOR_RETENTION_DAYS`, default `7`).
    pub retention_days: u64,
    /// Cleanup job interval in seconds (env `MONITOR_CLEANUP_INTERVAL_SECS`, default `3600`).
    pub cleanup_interval_secs: u64,
    /// SQLite database path (env `MONITOR_DB_PATH`, default `./monitor.db`).
    pub db_path: String,
    /// Allowed CORS origins (env `MONITOR_CORS_ORIGINS`, default `*`).
    pub cors_origins: String,
}

impl Config {
    /// Load configuration from the real process environment.
    ///
    /// Missing variables fall back to defaults; a present-but-invalid value is
    /// an error.
    pub fn load() -> Result<Config, AppError> {
        from_lookup(|k| std::env::var(k).ok())
    }
}

/// Parse a string env value into a type, mapping failure to `AppError::Config`.
///
/// Only called when the variable is present, so a parse failure here is always
/// a genuine "present-but-unparseable" error (never a silent default fallback).
fn parse_field<T>(key: &str, raw: &str) -> Result<T, AppError>
where
    T: std::str::FromStr,
{
    raw.trim().parse::<T>().map_err(|_| {
        AppError::Config(format!("invalid value for {key}: {raw:?}"))
    })
}

/// Core parsing/default/validation logic, parameterised over a lookup closure.
///
/// Kept separate from [`Config::load`] so tests can drive it with a
/// deterministic in-memory map instead of mutating the global process env
/// (which is racy under cargo's parallel test threads).
fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Config, AppError> {
    let bind = get("MONITOR_BIND").unwrap_or_else(|| "0.0.0.0".to_string());

    let port = match get("MONITOR_PORT") {
        Some(v) => parse_field::<u16>("MONITOR_PORT", &v)?,
        None => 8080,
    };

    let sample_interval_secs = match get("MONITOR_SAMPLE_INTERVAL_SECS") {
        Some(v) => parse_field::<u64>("MONITOR_SAMPLE_INTERVAL_SECS", &v)?,
        None => 10,
    };

    let retention_days = match get("MONITOR_RETENTION_DAYS") {
        Some(v) => parse_field::<u64>("MONITOR_RETENTION_DAYS", &v)?,
        None => 7,
    };

    let cleanup_interval_secs = match get("MONITOR_CLEANUP_INTERVAL_SECS") {
        Some(v) => parse_field::<u64>("MONITOR_CLEANUP_INTERVAL_SECS", &v)?,
        None => 3600,
    };

    let db_path = get("MONITOR_DB_PATH").unwrap_or_else(|| "./monitor.db".to_string());

    let cors_origins = get("MONITOR_CORS_ORIGINS").unwrap_or_else(|| "*".to_string());

    if sample_interval_secs == 0 {
        return Err(AppError::Config(
            "MONITOR_SAMPLE_INTERVAL_SECS must be greater than 0".to_string(),
        ));
    }
    if retention_days == 0 {
        return Err(AppError::Config(
            "MONITOR_RETENTION_DAYS must be greater than 0".to_string(),
        ));
    }
    if cleanup_interval_secs == 0 {
        return Err(AppError::Config(
            "MONITOR_CLEANUP_INTERVAL_SECS must be greater than 0".to_string(),
        ));
    }

    Ok(Config {
        bind,
        port,
        sample_interval_secs,
        retention_days,
        cleanup_interval_secs,
        db_path,
        cors_origins,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Build a lookup closure backed by an in-memory map.
    fn map_lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn empty_lookup_yields_all_defaults() {
        let cfg = from_lookup(map_lookup(&[])).unwrap();
        assert_eq!(cfg.bind, "0.0.0.0");
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.sample_interval_secs, 10);
        assert_eq!(cfg.retention_days, 7);
        assert_eq!(cfg.cleanup_interval_secs, 3600);
        assert_eq!(cfg.db_path, "./monitor.db");
        assert_eq!(cfg.cors_origins, "*");
    }

    #[test]
    fn custom_values_override_every_default() {
        let cfg = from_lookup(map_lookup(&[
            ("MONITOR_BIND", "127.0.0.1"),
            ("MONITOR_PORT", "9000"),
            ("MONITOR_SAMPLE_INTERVAL_SECS", "5"),
            ("MONITOR_RETENTION_DAYS", "30"),
            ("MONITOR_CLEANUP_INTERVAL_SECS", "600"),
            ("MONITOR_DB_PATH", "/var/lib/monitor.db"),
            ("MONITOR_CORS_ORIGINS", "https://example.com"),
        ]))
        .unwrap();
        assert_eq!(cfg.bind, "127.0.0.1");
        assert_eq!(cfg.port, 9000);
        assert_eq!(cfg.sample_interval_secs, 5);
        assert_eq!(cfg.retention_days, 30);
        assert_eq!(cfg.cleanup_interval_secs, 600);
        assert_eq!(cfg.db_path, "/var/lib/monitor.db");
        assert_eq!(cfg.cors_origins, "https://example.com");
    }

    #[test]
    fn zero_sample_interval_is_rejected() {
        let err = from_lookup(map_lookup(&[("MONITOR_SAMPLE_INTERVAL_SECS", "0")])).unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
    }

    #[test]
    fn zero_retention_days_is_rejected() {
        let err = from_lookup(map_lookup(&[("MONITOR_RETENTION_DAYS", "0")])).unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
    }

    #[test]
    fn zero_cleanup_interval_is_rejected() {
        let err = from_lookup(map_lookup(&[("MONITOR_CLEANUP_INTERVAL_SECS", "0")])).unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
    }

    #[test]
    fn non_numeric_port_is_rejected() {
        let err = from_lookup(map_lookup(&[("MONITOR_PORT", "abc")])).unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
    }

    #[test]
    fn out_of_range_port_is_rejected() {
        let err = from_lookup(map_lookup(&[("MONITOR_PORT", "70000")])).unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
    }

    #[test]
    fn non_numeric_interval_is_rejected() {
        let err =
            from_lookup(map_lookup(&[("MONITOR_SAMPLE_INTERVAL_SECS", "xyz")])).unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
    }
}
