//! B5 — Sampler (background scheduler).
//!
//! Coordinates the B3 [`MetricsCollector`] and the B4 [`MetricsRepository`] as
//! two concurrent, long-lived `tokio` background loops:
//!
//! - **Sample loop**: every `sample_interval` → `collector.collect()`; on `Ok`
//!   → `repo.insert(&sample)`.
//! - **Cleanup loop**: every `cleanup_interval` → `repo.delete_older_than(now -
//!   retention_days * 86_400)`.
//!
//! **Failure isolation** (HLD §1.4, §B5, prompt §8): any single `collect`,
//! `insert`, or `delete_older_than` failure is logged (`tracing`) and the loop
//! continues to the next tick — it never panics and never breaks the loop.
//!
//! # Design choices
//!
//! - **Collector ownership**: the collector is taken as
//!   `Box<dyn MetricsCollector + Send>`. `collect` needs `&mut self`, so the
//!   sample-loop task takes sole ownership of the boxed collector and mutates it
//!   directly — no extra locking, and the trait object keeps `Sampler::new`
//!   monomorphisation-free.
//! - **Repository sharing**: the repo is an
//!   `Arc<dyn MetricsRepository + Send + Sync>`, cloned into both loop tasks and
//!   shareable with the HTTP layer (B6).
//! - **Testability / clock**: both loops are spawned as `tokio` tasks driven by
//!   [`tokio::time::interval`], so tests run under a **paused** virtual clock
//!   (`#[tokio::test(start_paused = true)]`) and step time with
//!   [`tokio::time::advance`] + [`tokio::task::yield_now`]. The cleanup cutoff's
//!   "now" is supplied by an injectable `now_fn` closure so the cutoff value is
//!   deterministic and directly assertable, decoupled from the wall clock.

use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::collector::MetricsCollector;
use crate::config::Config;
use crate::storage::MetricsRepository;

/// Seconds in a day, used to convert `retention_days` into a cutoff offset.
const SECS_PER_DAY: i64 = 86_400;

/// Type of the "current Unix epoch seconds" source used to compute the cleanup
/// cutoff. Boxed so tests can inject a deterministic clock.
type NowFn = Box<dyn Fn() -> i64 + Send + Sync>;

/// Current wall-clock time as Unix epoch seconds (UTC).
fn now_unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Background scheduler that owns a collector + a shared repository and drives
/// the periodic sampling and cleanup loops.
pub struct Sampler {
    collector: Box<dyn MetricsCollector + Send>,
    repo: Arc<dyn MetricsRepository + Send + Sync>,
    sample_interval: Duration,
    cleanup_interval: Duration,
    /// Retention window, already converted to seconds.
    retention_secs: i64,
    /// Source of "now" (epoch secs) for the cleanup cutoff. Injectable for tests.
    now_fn: NowFn,
}

impl Sampler {
    /// Construct a sampler from a collector, a shared repository, and the
    /// relevant [`Config`] values (`sample_interval_secs`,
    /// `cleanup_interval_secs`, `retention_days`).
    ///
    /// The collector is taken as a boxed trait object so the sample loop can own
    /// it and call `collect(&mut self)` without extra synchronisation.
    pub fn new(
        collector: Box<dyn MetricsCollector + Send>,
        repo: Arc<dyn MetricsRepository + Send + Sync>,
        config: &Config,
    ) -> Self {
        Self {
            collector,
            repo,
            sample_interval: Duration::from_secs(config.sample_interval_secs),
            cleanup_interval: Duration::from_secs(config.cleanup_interval_secs),
            retention_secs: config.retention_days as i64 * SECS_PER_DAY,
            now_fn: Box::new(now_unix_secs),
        }
    }

    /// Override the "now" source used to compute the cleanup cutoff.
    ///
    /// Test seam: lets a test inject a fixed clock and assert the exact cutoff
    /// passed to `delete_older_than`.
    #[cfg(test)]
    fn with_now_fn(mut self, now_fn: NowFn) -> Self {
        self.now_fn = now_fn;
        self
    }

    /// Spawn the two background loops as `tokio` tasks and return a handle that
    /// completes when both loops finish (in normal operation, never — they loop
    /// forever).
    pub fn spawn(self) -> JoinHandle<()> {
        let Sampler {
            collector,
            repo,
            sample_interval,
            cleanup_interval,
            retention_secs,
            now_fn,
        } = self;

        let sample_repo = Arc::clone(&repo);
        let sample_handle = tokio::spawn(sample_loop(collector, sample_repo, sample_interval));

        let cleanup_handle =
            tokio::spawn(cleanup_loop(repo, cleanup_interval, retention_secs, now_fn));

        tokio::spawn(async move {
            // If either loop ever terminates (it shouldn't), join both so the
            // supervising handle reflects completion.
            let _ = sample_handle.await;
            let _ = cleanup_handle.await;
        })
    }

    /// Convenience wrapper that spawns the loops and discards the handle.
    pub fn run(self) {
        let _ = self.spawn();
    }
}

/// The sampling loop: collect → insert, every `interval`, with failure isolation.
async fn sample_loop(
    mut collector: Box<dyn MetricsCollector + Send>,
    repo: Arc<dyn MetricsRepository + Send + Sync>,
    interval: Duration,
) {
    let mut ticker = tokio::time::interval(interval);
    // `interval` fires immediately on the first tick by default; that first tick
    // is consumed here so the first *sample* happens after one full interval,
    // matching "every N seconds".
    ticker.tick().await;
    loop {
        ticker.tick().await;
        match collector.collect() {
            Ok(sample) => {
                if let Err(e) = repo.insert(&sample) {
                    tracing::error!(error = %e, "sampler: insert failed; skipping tick");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "sampler: collect failed; skipping tick");
            }
        }
    }
}

/// The cleanup loop: delete rows older than `now - retention`, every `interval`,
/// with failure isolation.
async fn cleanup_loop(
    repo: Arc<dyn MetricsRepository + Send + Sync>,
    interval: Duration,
    retention_secs: i64,
    now_fn: NowFn,
) {
    let mut ticker = tokio::time::interval(interval);
    ticker.tick().await; // consume the immediate first tick
    loop {
        ticker.tick().await;
        let cutoff = now_fn() - retention_secs;
        if let Err(e) = repo.delete_older_than(cutoff) {
            tracing::error!(error = %e, cutoff, "sampler: cleanup delete failed; skipping tick");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use crate::collector::FakeCollector;
    use crate::domain::{AppError, MetricPoint, MetricSample, TimeRange};

    fn sample(ts: i64) -> MetricSample {
        MetricSample {
            timestamp: ts,
            cpu_percent: 10.0,
            mem_total_bytes: 16_000_000_000,
            mem_used_bytes: 8_000_000_000,
            mem_percent: 50.0,
            net_rx_rate_bps: 0.0,
            net_tx_rate_bps: 0.0,
        }
    }

    /// Build a config; tests pass the timing fields they care about.
    fn config(sample_secs: u64, cleanup_secs: u64, retention_days: u64) -> Config {
        Config {
            bind: "0.0.0.0".into(),
            port: 8080,
            sample_interval_secs: sample_secs,
            retention_days,
            cleanup_interval_secs: cleanup_secs,
            db_path: ":memory:".into(),
            cors_origins: "*".into(),
        }
    }

    /// A recording fake repository: counts successful inserts, can be scripted
    /// to fail insert a given number of times first, and records every cutoff
    /// passed to `delete_older_than`.
    struct RecordingRepo {
        insert_count: AtomicUsize,
        /// Number of remaining inserts that should fail before succeeding.
        insert_failures_left: Mutex<usize>,
        delete_cutoffs: Mutex<Vec<i64>>,
    }

    impl RecordingRepo {
        fn new() -> Self {
            Self {
                insert_count: AtomicUsize::new(0),
                insert_failures_left: Mutex::new(0),
                delete_cutoffs: Mutex::new(Vec::new()),
            }
        }

        fn with_insert_failures(failures: usize) -> Self {
            let r = Self::new();
            *r.insert_failures_left.lock().unwrap() = failures;
            r
        }

        fn inserts(&self) -> usize {
            self.insert_count.load(Ordering::SeqCst)
        }

        fn cutoffs(&self) -> Vec<i64> {
            self.delete_cutoffs.lock().unwrap().clone()
        }
    }

    impl MetricsRepository for RecordingRepo {
        fn insert(&self, _sample: &MetricSample) -> Result<(), AppError> {
            let mut left = self.insert_failures_left.lock().unwrap();
            if *left > 0 {
                *left -= 1;
                return Err(AppError::Storage("scripted insert failure".into()));
            }
            self.insert_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn latest(&self) -> Result<Option<MetricSample>, AppError> {
            Ok(None)
        }

        fn query_range(&self, _range: TimeRange) -> Result<Vec<MetricPoint>, AppError> {
            Ok(Vec::new())
        }

        fn delete_older_than(&self, cutoff_ts: i64) -> Result<u64, AppError> {
            self.delete_cutoffs.lock().unwrap().push(cutoff_ts);
            Ok(0)
        }
    }

    /// Advance the paused clock by `interval`, `n` times, yielding between each
    /// step so spawned loop tasks get scheduled and run a tick body.
    async fn advance_ticks(interval: Duration, n: usize) {
        for _ in 0..n {
            tokio::time::advance(interval).await;
            // Yield a few times so the woken loop task runs its tick body
            // (collect/insert/delete) before we advance again.
            for _ in 0..4 {
                tokio::task::yield_now().await;
            }
        }
    }

    // --- Test 1: collect/insert invoked N times matching the interval. ---
    #[tokio::test(start_paused = true)]
    async fn collect_insert_invoked_n_times_within_window() {
        let interval = Duration::from_secs(10);
        let collector = Box::new(FakeCollector::from_samples(
            (0..20).map(sample).collect(),
        ));
        let repo = Arc::new(RecordingRepo::new());
        let repo_dyn: Arc<dyn MetricsRepository + Send + Sync> = repo.clone();

        // Huge cleanup interval so the cleanup loop never fires during the test.
        let cfg = config(10, 1_000_000, 7);
        Sampler::new(collector, repo_dyn, &cfg).spawn();

        // Let the loops reach their first `tick().await` (the immediate tick).
        tokio::task::yield_now().await;

        advance_ticks(interval, 5).await;

        // 5 intervals elapsed → exactly 5 samples inserted.
        assert_eq!(repo.inserts(), 5, "expected 5 inserts after 5 intervals");
    }

    // --- Test 2: a collect() failure on one tick does not stop the loop. ---
    #[tokio::test(start_paused = true)]
    async fn collect_failure_does_not_stop_loop() {
        let interval = Duration::from_secs(10);
        // Ok, Ok, Err (middle), Ok, Ok → 4 successful collects → 4 inserts.
        let collector = Box::new(FakeCollector::new(vec![
            Ok(sample(1)),
            Ok(sample(2)),
            Err(AppError::Collect("boom".into())),
            Ok(sample(4)),
            Ok(sample(5)),
        ]));
        let repo = Arc::new(RecordingRepo::new());
        let repo_dyn: Arc<dyn MetricsRepository + Send + Sync> = repo.clone();

        let cfg = config(10, 1_000_000, 7);
        Sampler::new(collector, repo_dyn, &cfg).spawn();
        tokio::task::yield_now().await;

        advance_ticks(interval, 5).await;

        // 5 ticks, 1 collect error in the middle → 4 inserts; the inserts after
        // the error prove the loop kept running.
        assert_eq!(repo.inserts(), 4, "loop must continue past a collect error");
    }

    // --- Test 3: an insert() failure does not stop the loop. ---
    #[tokio::test(start_paused = true)]
    async fn insert_failure_does_not_stop_loop() {
        let interval = Duration::from_secs(10);
        let collector = Box::new(FakeCollector::from_samples(
            (0..10).map(sample).collect(),
        ));
        // First insert fails, the rest succeed.
        let repo = Arc::new(RecordingRepo::with_insert_failures(1));
        let repo_dyn: Arc<dyn MetricsRepository + Send + Sync> = repo.clone();

        let cfg = config(10, 1_000_000, 7);
        Sampler::new(collector, repo_dyn, &cfg).spawn();
        tokio::task::yield_now().await;

        advance_ticks(interval, 5).await;

        // 5 ticks: tick 1 insert fails (not counted), ticks 2-5 succeed → 4.
        assert_eq!(repo.inserts(), 4, "loop must continue past an insert error");
    }

    // --- Test 4: cleanup uses the correct cutoff (now - retention). ---
    #[tokio::test(start_paused = true)]
    async fn cleanup_uses_correct_cutoff() {
        let cleanup_interval = Duration::from_secs(60);
        let collector = Box::new(FakeCollector::from_samples(
            (0..5).map(sample).collect(),
        ));
        let repo = Arc::new(RecordingRepo::new());
        let repo_dyn: Arc<dyn MetricsRepository + Send + Sync> = repo.clone();

        // retention 7 days; sample interval huge so the sample loop stays quiet.
        let cfg = config(1_000_000, 60, 7);
        let fixed_now: i64 = 2_000_000_000;
        Sampler::new(collector, repo_dyn, &cfg)
            .with_now_fn(Box::new(move || fixed_now))
            .spawn();
        tokio::task::yield_now().await;

        // Fire the cleanup loop twice.
        advance_ticks(cleanup_interval, 2).await;

        let expected = fixed_now - 7 * SECS_PER_DAY;
        let cutoffs = repo.cutoffs();
        assert_eq!(cutoffs.len(), 2, "cleanup should have fired twice");
        assert!(
            cutoffs.iter().all(|&c| c == expected),
            "cleanup cutoff must be now - retention ({expected}); got {cutoffs:?}"
        );
    }
}
