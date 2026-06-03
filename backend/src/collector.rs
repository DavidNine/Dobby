//! B3 — Collector.
//!
//! Wraps `sysinfo` to produce a single [`MetricSample`] snapshot per call.
//!
//! Exposes the [`MetricsCollector`] abstraction so downstream modules (the B5
//! Sampler) can depend on the trait rather than the concrete sysinfo-backed
//! implementation, and so tests can substitute [`FakeCollector`].
//!
//! Conventions (per project contract / HLD §B3):
//! - `timestamp` is Unix epoch **seconds** (UTC).
//! - Network rates are **bytes/sec**, derived from sysinfo's cumulative byte
//!   counters via the pure [`compute_rate`] function.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sysinfo::{
    CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System,
    MINIMUM_CPU_UPDATE_INTERVAL,
};

use crate::domain::{AppError, MetricSample};

/// Abstraction over "produce one current metrics snapshot".
///
/// `collect` takes `&mut self` because concrete collectors (e.g. the
/// sysinfo-backed one) carry mutable state across calls — refreshed system
/// handles and the previous network counters used for rate computation.
pub trait MetricsCollector {
    /// Produce a single point-in-time [`MetricSample`] for "now".
    fn collect(&mut self) -> Result<MetricSample, AppError>;
}

/// Compute a byte/sec rate from two cumulative byte counters and the elapsed
/// time between them.
///
/// This is a pure function (no I/O, no state) so it can be unit-tested in
/// isolation, per HLD §B3.
///
/// Policy:
/// - Normal case: `(curr_bytes - prev_bytes) / dt_secs`.
/// - `dt_secs <= 0` (zero or non-positive interval) → `0.0`. This guards
///   against division by zero / nonsense negative intervals.
/// - Counter reset / wrap (`curr_bytes < prev_bytes`, e.g. an interface
///   restarted) → `0.0`, so we never report a negative rate.
///
/// The "very first sample, no previous value" case is the caller's
/// responsibility (see [`SysinfoCollector::collect`], which reports `0.0` until
/// it has a stored previous reading). `compute_rate` itself is still safe if a
/// caller passes `prev_bytes == curr_bytes` (yields `0.0`).
pub fn compute_rate(prev_bytes: u64, curr_bytes: u64, dt_secs: f64) -> f64 {
    if dt_secs <= 0.0 {
        return 0.0;
    }
    if curr_bytes < prev_bytes {
        // Counter reset / interface restart: avoid reporting a negative rate.
        return 0.0;
    }
    let delta = curr_bytes - prev_bytes;
    delta as f64 / dt_secs
}

/// Current wall-clock time as Unix epoch seconds (UTC).
fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        // Before the epoch should never happen on a sane host; clamp to 0.
        .unwrap_or(0)
}

/// Real, sysinfo-backed [`MetricsCollector`].
///
/// Holds a long-lived [`System`] and [`Networks`] so successive `collect`
/// calls observe the deltas sysinfo needs (notably CPU usage, which requires
/// two refreshes spaced by [`MINIMUM_CPU_UPDATE_INTERVAL`]).
pub struct SysinfoCollector {
    system: System,
    networks: Networks,
    /// Previous cumulative network totals and the timestamp they were taken at:
    /// `(rx_bytes, tx_bytes, epoch_secs)`. `None` until the first `collect`.
    prev_net_total: Option<(u64, u64, i64)>,
}

impl SysinfoCollector {
    /// Construct a collector and prime sysinfo's internal state.
    ///
    /// CPU policy: sysinfo computes CPU usage as a delta between two refreshes.
    /// We perform an initial CPU refresh here and sleep for
    /// [`MINIMUM_CPU_UPDATE_INTERVAL`] so that the *first* `collect` call has a
    /// valid (non-zero-by-construction) CPU reading rather than the
    /// always-zero value a single refresh yields.
    pub fn new() -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );

        // Prime CPU: first refresh establishes a baseline; CPU usage is only
        // meaningful on the *second* refresh (done in `collect`).
        system.refresh_cpu_usage();
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);

        // `new_with_refreshed_list` populates the interface list and seeds the
        // cumulative totals so the first delta is sensible.
        let networks = Networks::new_with_refreshed_list();

        Self {
            system,
            networks,
            prev_net_total: None,
        }
    }
}

impl Default for SysinfoCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector for SysinfoCollector {
    fn collect(&mut self) -> Result<MetricSample, AppError> {
        // --- CPU (overall, not per-core) ---
        self.system.refresh_cpu_usage();
        let cpu_percent = self.system.global_cpu_usage();

        // --- Memory (bytes) ---
        self.system.refresh_memory();
        let mem_total_bytes = self.system.total_memory();
        let mem_used_bytes = self.system.used_memory();
        if mem_total_bytes == 0 {
            return Err(AppError::Collect(
                "sysinfo reported zero total memory".to_string(),
            ));
        }
        let mem_percent = (mem_used_bytes as f64 / mem_total_bytes as f64 * 100.0) as f32;

        // --- Network: sum cumulative counters across all interfaces ---
        self.networks.refresh(true);
        let mut rx_total: u64 = 0;
        let mut tx_total: u64 = 0;
        for (_name, data) in &self.networks {
            rx_total = rx_total.saturating_add(data.total_received());
            tx_total = tx_total.saturating_add(data.total_transmitted());
        }

        let timestamp = now_epoch_secs();

        let (net_rx_rate_bps, net_tx_rate_bps) = match self.prev_net_total {
            // First sample: no previous reading → rates are 0.
            None => (0.0, 0.0),
            Some((prev_rx, prev_tx, prev_ts)) => {
                let dt = (timestamp - prev_ts) as f64;
                (
                    compute_rate(prev_rx, rx_total, dt),
                    compute_rate(prev_tx, tx_total, dt),
                )
            }
        };

        // Store the current cumulative totals for the next call's delta.
        self.prev_net_total = Some((rx_total, tx_total, timestamp));

        Ok(MetricSample {
            timestamp,
            cpu_percent,
            mem_total_bytes,
            mem_used_bytes,
            mem_percent,
            net_rx_rate_bps,
            net_tx_rate_bps,
        })
    }
}

/// A scripted [`MetricsCollector`] for tests (notably the B5 Sampler).
///
/// Yields a preset sequence of `Result<MetricSample, AppError>` in order; each
/// `collect` call pops the next item. Entries may be `Err`, letting tests
/// exercise failure handling. Once the sequence is exhausted, `collect` returns
/// an [`AppError::Collect`] so over-consumption is visible rather than silent.
pub struct FakeCollector {
    sequence: std::collections::VecDeque<Result<MetricSample, AppError>>,
}

impl FakeCollector {
    /// Build a fake collector that will return `items` in order, one per
    /// `collect` call.
    pub fn new(items: Vec<Result<MetricSample, AppError>>) -> Self {
        Self {
            sequence: items.into_iter().collect(),
        }
    }

    /// Convenience constructor from a sequence of successful samples.
    pub fn from_samples(samples: Vec<MetricSample>) -> Self {
        Self::new(samples.into_iter().map(Ok).collect())
    }

    /// Number of scripted responses still to be returned.
    pub fn remaining(&self) -> usize {
        self.sequence.len()
    }
}

impl MetricsCollector for FakeCollector {
    fn collect(&mut self) -> Result<MetricSample, AppError> {
        match self.sequence.pop_front() {
            Some(result) => result,
            None => Err(AppError::Collect(
                "FakeCollector sequence exhausted".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ts: i64, cpu: f32) -> MetricSample {
        MetricSample {
            timestamp: ts,
            cpu_percent: cpu,
            mem_total_bytes: 16_000_000_000,
            mem_used_bytes: 8_000_000_000,
            mem_percent: 50.0,
            net_rx_rate_bps: 0.0,
            net_tx_rate_bps: 0.0,
        }
    }

    // --- compute_rate ---

    #[test]
    fn compute_rate_normal_case() {
        // 2000 bytes over 2 seconds = 1000 B/s.
        assert_eq!(compute_rate(1_000, 3_000, 2.0), 1_000.0);
        // 500 bytes over 0.5 s = 1000 B/s.
        assert_eq!(compute_rate(0, 500, 0.5), 1_000.0);
    }

    #[test]
    fn compute_rate_zero_or_negative_dt_returns_zero() {
        // dt == 0 (would divide by zero) → guarded to 0.
        assert_eq!(compute_rate(100, 500, 0.0), 0.0);
        // dt < 0 (nonsense interval) → guarded to 0.
        assert_eq!(compute_rate(100, 500, -1.0), 0.0);
    }

    #[test]
    fn compute_rate_counter_wrap_returns_zero() {
        // curr < prev (interface restart / counter reset) → 0, never negative.
        assert_eq!(compute_rate(5_000, 100, 2.0), 0.0);
    }

    #[test]
    fn compute_rate_no_change_is_zero() {
        // prev == curr → 0 (also the safe "first-sample-ish" degenerate case).
        assert_eq!(compute_rate(1_234, 1_234, 5.0), 0.0);
    }

    // --- SysinfoCollector smoke test (against the real system, per HLD §7) ---

    #[test]
    fn sysinfo_collector_smoke() {
        let mut c = SysinfoCollector::new();
        let s = c.collect().expect("collect should succeed on a real host");

        // CPU in [0, 100] with a small epsilon for floating point slack.
        assert!(
            s.cpu_percent >= 0.0 && s.cpu_percent <= 100.0 + 1e-3,
            "cpu_percent out of range: {}",
            s.cpu_percent
        );

        // Memory invariants.
        assert!(s.mem_total_bytes > 0, "mem_total should be positive");
        assert!(
            s.mem_used_bytes <= s.mem_total_bytes,
            "mem_used {} > mem_total {}",
            s.mem_used_bytes,
            s.mem_total_bytes
        );
        assert!(
            s.mem_percent >= 0.0 && s.mem_percent <= 100.0 + 1e-3,
            "mem_percent out of range: {}",
            s.mem_percent
        );

        // First sample → rates are exactly 0.
        assert_eq!(s.net_rx_rate_bps, 0.0);
        assert_eq!(s.net_tx_rate_bps, 0.0);

        // Timestamp is a plausible recent epoch-seconds value (> 2021-01-01).
        assert!(s.timestamp > 1_600_000_000, "timestamp looks wrong: {}", s.timestamp);

        // A second collect should also succeed; rates must be finite and >= 0.
        std::thread::sleep(Duration::from_millis(50));
        let s2 = c.collect().expect("second collect should succeed");
        assert!(s2.net_rx_rate_bps.is_finite() && s2.net_rx_rate_bps >= 0.0);
        assert!(s2.net_tx_rate_bps.is_finite() && s2.net_tx_rate_bps >= 0.0);
    }

    // --- FakeCollector ---

    #[test]
    fn fake_collector_returns_sequence_in_order() {
        let mut fake = FakeCollector::from_samples(vec![
            sample(100, 10.0),
            sample(200, 20.0),
            sample(300, 30.0),
        ]);
        assert_eq!(fake.remaining(), 3);
        assert_eq!(fake.collect().unwrap().timestamp, 100);
        assert_eq!(fake.collect().unwrap().cpu_percent, 20.0);
        assert_eq!(fake.collect().unwrap().timestamp, 300);
        // Exhausted → Err rather than panic/silent.
        assert!(matches!(fake.collect(), Err(AppError::Collect(_))));
    }

    #[test]
    fn fake_collector_surfaces_configured_error() {
        let mut fake = FakeCollector::new(vec![
            Ok(sample(1, 1.0)),
            Err(AppError::Collect("boom".to_string())),
            Ok(sample(3, 3.0)),
        ]);
        assert_eq!(fake.collect().unwrap().timestamp, 1);
        let err = fake.collect().unwrap_err();
        assert!(matches!(err, AppError::Collect(ref m) if m == "boom"));
        // Sequence continues after the error.
        assert_eq!(fake.collect().unwrap().timestamp, 3);
    }
}
