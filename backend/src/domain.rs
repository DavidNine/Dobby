//! B1 — Domain types.
//!
//! Cross-module, framework-agnostic data structures and the unified error type.
//! Depends on nothing (not sysinfo / axum / sqlite).

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A single point-in-time metrics snapshot as produced by the collector and
/// stored by the repository.
///
/// All time values are Unix epoch seconds (UTC); network rates are bytes/sec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricSample {
    /// Unix epoch seconds (UTC).
    pub timestamp: i64,
    /// CPU utilisation, 0.0 ~ 100.0.
    pub cpu_percent: f32,
    /// Total physical memory in bytes.
    pub mem_total_bytes: u64,
    /// Used physical memory in bytes.
    pub mem_used_bytes: u64,
    /// Memory utilisation, 0.0 ~ 100.0.
    pub mem_percent: f32,
    /// Download rate, bytes/sec.
    pub net_rx_rate_bps: f64,
    /// Upload rate, bytes/sec.
    pub net_tx_rate_bps: f64,
}

/// A downsampled point returned by historical range queries.
///
/// Times are Unix epoch seconds (UTC); network rates are bytes/sec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricPoint {
    /// Unix epoch seconds (UTC), the bucket timestamp.
    pub timestamp: i64,
    /// CPU utilisation, 0.0 ~ 100.0.
    pub cpu_percent: f32,
    /// Memory utilisation, 0.0 ~ 100.0.
    pub mem_percent: f32,
    /// Download rate, bytes/sec.
    pub net_rx_bps: f64,
    /// Upload rate, bytes/sec.
    pub net_tx_bps: f64,
}

/// The time range of a historical query. The wire representation is the
/// canonical string form shared verbatim by frontend and backend
/// (`"1h"` / `"6h"` / `"24h"` / `"7d"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRange {
    /// Last 1 hour.
    Hour1,
    /// Last 6 hours.
    Hour6,
    /// Last 24 hours.
    Hour24,
    /// Last 7 days.
    Day7,
}

impl TimeRange {
    /// The canonical wire string for this range.
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeRange::Hour1 => "1h",
            TimeRange::Hour6 => "6h",
            TimeRange::Hour24 => "24h",
            TimeRange::Day7 => "7d",
        }
    }

    /// The total span of the range in seconds.
    pub fn range_secs(&self) -> i64 {
        match self {
            TimeRange::Hour1 => 3_600,
            TimeRange::Hour6 => 21_600,
            TimeRange::Hour24 => 86_400,
            TimeRange::Day7 => 604_800,
        }
    }

    /// The downsampling bucket width in seconds (per HLD §B4, target ~360 points).
    pub fn bucket_secs(&self) -> i64 {
        match self {
            TimeRange::Hour1 => 10,
            TimeRange::Hour6 => 60,
            TimeRange::Hour24 => 240,
            TimeRange::Day7 => 1_800,
        }
    }
}

/// Error returned when parsing a [`TimeRange`] from a string fails.
///
/// Kept deliberately small and decoupled from [`AppError`] so the HTTP layer
/// (B6) can map a parse failure to a 400 without coupling the domain's string
/// parsing to the unified error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRangeError;

impl std::fmt::Display for ParseRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid time range (expected one of: 1h, 6h, 24h, 7d)"
        )
    }
}

impl std::error::Error for ParseRangeError {}

impl FromStr for TimeRange {
    type Err = ParseRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1h" => Ok(TimeRange::Hour1),
            "6h" => Ok(TimeRange::Hour6),
            "24h" => Ok(TimeRange::Hour24),
            "7d" => Ok(TimeRange::Day7),
            _ => Err(ParseRangeError),
        }
    }
}

/// The single unified error type used across the backend modules.
///
/// B6 maps each variant to an appropriate HTTP status.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Configuration loading/validation failure.
    #[error("configuration error: {0}")]
    Config(String),
    /// Storage (persistence/query) failure.
    #[error("storage error: {0}")]
    Storage(String),
    /// Metrics collection failure.
    #[error("collect error: {0}")]
    Collect(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_range_parses_each_valid_value() {
        assert_eq!("1h".parse::<TimeRange>().unwrap(), TimeRange::Hour1);
        assert_eq!("6h".parse::<TimeRange>().unwrap(), TimeRange::Hour6);
        assert_eq!("24h".parse::<TimeRange>().unwrap(), TimeRange::Hour24);
        assert_eq!("7d".parse::<TimeRange>().unwrap(), TimeRange::Day7);
    }

    #[test]
    fn time_range_rejects_invalid_value() {
        assert!("99x".parse::<TimeRange>().is_err());
        assert!("".parse::<TimeRange>().is_err());
        assert!("1H".parse::<TimeRange>().is_err());
        assert!("3600".parse::<TimeRange>().is_err());
        assert_eq!("nope".parse::<TimeRange>(), Err(ParseRangeError));
    }

    #[test]
    fn time_range_bucket_secs_mapping() {
        assert_eq!(TimeRange::Hour1.bucket_secs(), 10);
        assert_eq!(TimeRange::Hour6.bucket_secs(), 60);
        assert_eq!(TimeRange::Hour24.bucket_secs(), 240);
        assert_eq!(TimeRange::Day7.bucket_secs(), 1_800);
    }

    #[test]
    fn time_range_range_secs_mapping() {
        assert_eq!(TimeRange::Hour1.range_secs(), 3_600);
        assert_eq!(TimeRange::Hour6.range_secs(), 21_600);
        assert_eq!(TimeRange::Hour24.range_secs(), 86_400);
        assert_eq!(TimeRange::Day7.range_secs(), 604_800);
    }

    #[test]
    fn time_range_as_str_round_trips_with_parse() {
        for r in [
            TimeRange::Hour1,
            TimeRange::Hour6,
            TimeRange::Hour24,
            TimeRange::Day7,
        ] {
            assert_eq!(r.as_str().parse::<TimeRange>().unwrap(), r);
        }
    }

    #[test]
    fn metric_sample_serde_round_trips() {
        let s = MetricSample {
            timestamp: 1_700_000_000,
            cpu_percent: 12.5,
            mem_total_bytes: 16_000_000_000,
            mem_used_bytes: 8_000_000_000,
            mem_percent: 50.0,
            net_rx_rate_bps: 1024.0,
            net_tx_rate_bps: 512.0,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: MetricSample = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn metric_point_serde_round_trips() {
        let p = MetricPoint {
            timestamp: 1_700_000_000,
            cpu_percent: 33.3,
            mem_percent: 44.4,
            net_rx_bps: 2048.0,
            net_tx_bps: 1024.0,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: MetricPoint = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn app_error_display_messages() {
        assert_eq!(
            AppError::Config("bad port".into()).to_string(),
            "configuration error: bad port"
        );
        assert_eq!(
            AppError::Storage("locked".into()).to_string(),
            "storage error: locked"
        );
        assert_eq!(
            AppError::Collect("no nic".into()).to_string(),
            "collect error: no nic"
        );
    }
}
