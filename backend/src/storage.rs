//! B4 — Storage.
//!
//! SQLite (rusqlite, bundled) write / range query (with downsampling) / cleanup
//! behind the [`MetricsRepository`] trait.
//!
//! The repository is intended to be shared across the sampler thread (B5) and
//! the async HTTP layer (B6) as `Arc<dyn MetricsRepository + Send + Sync>`.
//! rusqlite's `Connection` is not `Sync`, so we wrap it in a `std::sync::Mutex`
//! inside [`SqliteRepository`]; this makes the `&self` trait methods work and
//! the struct `Send + Sync`.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::domain::{AppError, MetricPoint, MetricSample, TimeRange};

/// Abstract persistence/query interface for metric samples.
///
/// `Send + Sync` supertraits let callers store this behind
/// `Arc<dyn MetricsRepository + Send + Sync>` and share it across threads.
pub trait MetricsRepository: Send + Sync {
    /// Persist a single sample.
    fn insert(&self, sample: &MetricSample) -> Result<(), AppError>;
    /// Return the most recent sample (largest `ts`), or `None` if empty.
    fn latest(&self) -> Result<Option<MetricSample>, AppError>;
    /// Return downsampled points for the given range (relative to now).
    fn query_range(&self, range: TimeRange) -> Result<Vec<MetricPoint>, AppError>;
    /// Delete rows older than `cutoff_ts`; return the number of rows removed.
    fn delete_older_than(&self, cutoff_ts: i64) -> Result<u64, AppError>;
}

/// Pure helper mapping a [`TimeRange`] to its downsampling bucket width in
/// seconds (1h→10, 6h→60, 24h→240, 7d→1800). Delegates to the domain so the
/// range→bucket mapping stays in one place yet remains independently testable.
pub fn bucket_secs_for(range: TimeRange) -> i64 {
    range.bucket_secs()
}

/// SQLite-backed [`MetricsRepository`].
pub struct SqliteRepository {
    conn: Mutex<Connection>,
}

const CREATE_TABLE_SQL: &str = "\
CREATE TABLE IF NOT EXISTS metrics (
    ts               INTEGER NOT NULL,
    cpu_percent      REAL    NOT NULL,
    mem_total_bytes  INTEGER NOT NULL,
    mem_used_bytes   INTEGER NOT NULL,
    mem_percent      REAL    NOT NULL,
    net_rx_rate_bps  REAL    NOT NULL,
    net_tx_rate_bps  REAL    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_metrics_ts ON metrics(ts);";

impl SqliteRepository {
    /// Open (or create) the database at `path`, creating the table + index.
    pub fn open(path: &str) -> Result<Self, AppError> {
        let conn = Connection::open(path).map_err(to_storage_err)?;
        Self::init(conn)
    }

    /// Open an in-memory database (`:memory:`), creating the table + index.
    /// Primarily for tests.
    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory().map_err(to_storage_err)?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, AppError> {
        conn.execute_batch(CREATE_TABLE_SQL).map_err(to_storage_err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Lock the connection mutex, mapping a poisoned lock to a storage error.
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.conn
            .lock()
            .map_err(|e| AppError::Storage(format!("connection mutex poisoned: {e}")))
    }

    /// Testability seam: like [`query_range`] but with an explicit `now_ts`
    /// so downsampling behaviour is deterministic in tests.
    ///
    /// [`query_range`]: MetricsRepository::query_range
    pub fn query_range_since(
        &self,
        range: TimeRange,
        now_ts: i64,
    ) -> Result<Vec<MetricPoint>, AppError> {
        let bucket = bucket_secs_for(range);
        let start_ts = now_ts - range.range_secs();

        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT (ts / ?1) * ?1 AS bucket_start, \
                        AVG(cpu_percent), \
                        AVG(mem_percent), \
                        AVG(net_rx_rate_bps), \
                        AVG(net_tx_rate_bps) \
                 FROM metrics \
                 WHERE ts >= ?2 \
                 GROUP BY ts / ?1 \
                 ORDER BY bucket_start",
            )
            .map_err(to_storage_err)?;

        let rows = stmt
            .query_map([bucket, start_ts], |row| {
                Ok(MetricPoint {
                    timestamp: row.get(0)?,
                    cpu_percent: row.get::<_, f64>(1)? as f32,
                    mem_percent: row.get::<_, f64>(2)? as f32,
                    net_rx_bps: row.get(3)?,
                    net_tx_bps: row.get(4)?,
                })
            })
            .map_err(to_storage_err)?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(to_storage_err)?);
        }
        Ok(out)
    }
}

impl MetricsRepository for SqliteRepository {
    fn insert(&self, sample: &MetricSample) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO metrics \
             (ts, cpu_percent, mem_total_bytes, mem_used_bytes, mem_percent, \
              net_rx_rate_bps, net_tx_rate_bps) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                sample.timestamp,
                sample.cpu_percent,
                sample.mem_total_bytes as i64,
                sample.mem_used_bytes as i64,
                sample.mem_percent,
                sample.net_rx_rate_bps,
                sample.net_tx_rate_bps,
            ],
        )
        .map_err(to_storage_err)?;
        Ok(())
    }

    fn latest(&self) -> Result<Option<MetricSample>, AppError> {
        let conn = self.lock()?;
        let result = conn.query_row(
            "SELECT ts, cpu_percent, mem_total_bytes, mem_used_bytes, \
                    mem_percent, net_rx_rate_bps, net_tx_rate_bps \
             FROM metrics \
             ORDER BY ts DESC \
             LIMIT 1",
            [],
            |row| {
                Ok(MetricSample {
                    timestamp: row.get(0)?,
                    cpu_percent: row.get::<_, f64>(1)? as f32,
                    mem_total_bytes: row.get::<_, i64>(2)? as u64,
                    mem_used_bytes: row.get::<_, i64>(3)? as u64,
                    mem_percent: row.get::<_, f64>(4)? as f32,
                    net_rx_rate_bps: row.get(5)?,
                    net_tx_rate_bps: row.get(6)?,
                })
            },
        );

        match result {
            Ok(sample) => Ok(Some(sample)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(to_storage_err(e)),
        }
    }

    fn query_range(&self, range: TimeRange) -> Result<Vec<MetricPoint>, AppError> {
        let now_ts = now_unix_secs();
        self.query_range_since(range, now_ts)
    }

    fn delete_older_than(&self, cutoff_ts: i64) -> Result<u64, AppError> {
        let conn = self.lock()?;
        let affected = conn
            .execute("DELETE FROM metrics WHERE ts < ?1", [cutoff_ts])
            .map_err(to_storage_err)?;
        Ok(affected as u64)
    }
}

/// Current Unix epoch seconds (UTC).
fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Map any rusqlite error to the unified [`AppError::Storage`].
fn to_storage_err(e: rusqlite::Error) -> AppError {
    AppError::Storage(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ts: i64, cpu: f32, mem: f32, rx: f64, tx: f64) -> MetricSample {
        MetricSample {
            timestamp: ts,
            cpu_percent: cpu,
            mem_total_bytes: 16_000_000_000,
            mem_used_bytes: 8_000_000_000,
            mem_percent: mem,
            net_rx_rate_bps: rx,
            net_tx_rate_bps: tx,
        }
    }

    #[test]
    fn insert_then_latest_returns_most_recent_sample() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        repo.insert(&sample(100, 10.0, 20.0, 1.0, 2.0)).unwrap();
        repo.insert(&sample(300, 30.0, 40.0, 3.0, 4.0)).unwrap();
        repo.insert(&sample(200, 20.0, 30.0, 5.0, 6.0)).unwrap();

        let latest = repo.latest().unwrap().unwrap();
        assert_eq!(latest, sample(300, 30.0, 40.0, 3.0, 4.0));
    }

    #[test]
    fn latest_on_empty_db_returns_none() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        assert_eq!(repo.latest().unwrap(), None);
    }

    #[test]
    fn bucket_secs_for_maps_all_ranges() {
        assert_eq!(bucket_secs_for(TimeRange::Hour1), 10);
        assert_eq!(bucket_secs_for(TimeRange::Hour6), 60);
        assert_eq!(bucket_secs_for(TimeRange::Hour24), 240);
        assert_eq!(bucket_secs_for(TimeRange::Day7), 1_800);
    }

    #[test]
    fn query_range_1h_bucket_10_is_original_resolution() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        // now = 10_000. Three distinct 10s slots, each a single point.
        // ts 9000 -> bucket 9000, ts 9010 -> bucket 9010, ts 9025 -> bucket 9020.
        repo.insert(&sample(9000, 10.0, 50.0, 100.0, 200.0)).unwrap();
        repo.insert(&sample(9010, 20.0, 60.0, 300.0, 400.0)).unwrap();
        repo.insert(&sample(9025, 30.0, 70.0, 500.0, 600.0)).unwrap();

        let points = repo
            .query_range_since(TimeRange::Hour1, 10_000)
            .unwrap();

        // Each distinct 10s slot is its own point; no aggregation collapsing.
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].timestamp, 9000);
        assert_eq!(points[1].timestamp, 9010);
        assert_eq!(points[2].timestamp, 9020);
        assert_eq!(points[0].cpu_percent, 10.0);
        assert_eq!(points[1].cpu_percent, 20.0);
        assert_eq!(points[2].cpu_percent, 30.0);
        assert_eq!(points[2].net_rx_bps, 500.0);
    }

    #[test]
    fn query_range_6h_groups_and_averages_within_bucket() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        // 6h range, bucket = 60s, now = 10_000 so start_ts = 10_000 - 21_600 < 0.
        // Bucket A: ts 6000 & 6030 -> 6000/60 = 100 -> bucket_start 6000.
        //   cpu avg (10+30)/2 = 20, mem avg (40+60)/2 = 50,
        //   rx avg (100+300)/2 = 200, tx avg (1+3)/2 = 2.
        // Bucket B: ts 6120 -> bucket_start 6120, single point.
        repo.insert(&sample(6000, 10.0, 40.0, 100.0, 1.0)).unwrap();
        repo.insert(&sample(6030, 30.0, 60.0, 300.0, 3.0)).unwrap();
        repo.insert(&sample(6120, 50.0, 80.0, 500.0, 5.0)).unwrap();

        let points = repo
            .query_range_since(TimeRange::Hour6, 10_000)
            .unwrap();

        assert_eq!(points.len(), 2);

        // Bucket start timestamps are the window starts.
        assert_eq!(points[0].timestamp, 6000);
        assert_eq!(points[1].timestamp, 6120);

        // Averaged values for bucket A.
        assert_eq!(points[0].cpu_percent, 20.0);
        assert_eq!(points[0].mem_percent, 50.0);
        assert_eq!(points[0].net_rx_bps, 200.0);
        assert_eq!(points[0].net_tx_bps, 2.0);

        // Single-point bucket B keeps its values.
        assert_eq!(points[1].cpu_percent, 50.0);
        assert_eq!(points[1].net_rx_bps, 500.0);
    }

    #[test]
    fn query_range_24h_bucket_240_groups_correctly() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        // 24h range, bucket = 240s. now = 100_000.
        // Bucket at 48000: ts 48000, 48100, 48239 all share 48000/240 = 200.
        //   cpu avg (12+24+36)/3 = 24.
        // Bucket at 48240: ts 48240 -> 48240/240 = 201 -> bucket_start 48240.
        repo.insert(&sample(48000, 12.0, 10.0, 0.0, 0.0)).unwrap();
        repo.insert(&sample(48100, 24.0, 20.0, 0.0, 0.0)).unwrap();
        repo.insert(&sample(48239, 36.0, 30.0, 0.0, 0.0)).unwrap();
        repo.insert(&sample(48240, 99.0, 90.0, 0.0, 0.0)).unwrap();

        let points = repo
            .query_range_since(TimeRange::Hour24, 100_000)
            .unwrap();

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].timestamp, 48000);
        assert_eq!(points[0].cpu_percent, 24.0);
        assert_eq!(points[0].mem_percent, 20.0);
        assert_eq!(points[1].timestamp, 48240);
        assert_eq!(points[1].cpu_percent, 99.0);
    }

    #[test]
    fn query_range_excludes_rows_before_start_ts() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        // 1h range, now = 10_000 => start_ts = 6_400. Rows with ts < 6400 excluded.
        repo.insert(&sample(6000, 10.0, 10.0, 0.0, 0.0)).unwrap(); // excluded
        repo.insert(&sample(6400, 20.0, 20.0, 0.0, 0.0)).unwrap(); // included (>=)
        repo.insert(&sample(9000, 30.0, 30.0, 0.0, 0.0)).unwrap(); // included

        let points = repo
            .query_range_since(TimeRange::Hour1, 10_000)
            .unwrap();

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].timestamp, 6400);
        assert_eq!(points[1].timestamp, 9000);
    }

    #[test]
    fn delete_older_than_removes_only_old_rows_and_returns_count() {
        let repo = SqliteRepository::open_in_memory().unwrap();
        repo.insert(&sample(100, 1.0, 1.0, 0.0, 0.0)).unwrap();
        repo.insert(&sample(200, 2.0, 2.0, 0.0, 0.0)).unwrap();
        repo.insert(&sample(300, 3.0, 3.0, 0.0, 0.0)).unwrap();

        // Delete rows with ts < 250 -> removes ts 100 & 200 (count 2).
        let removed = repo.delete_older_than(250).unwrap();
        assert_eq!(removed, 2);

        // Remaining: only ts 300.
        let latest = repo.latest().unwrap().unwrap();
        assert_eq!(latest.timestamp, 300);

        // Deleting again with same cutoff removes nothing.
        let removed_again = repo.delete_older_than(250).unwrap();
        assert_eq!(removed_again, 0);
    }

    #[test]
    fn repository_is_send_sync_object_safe() {
        // Compile-time assertion that the trait object meets the shared-repo
        // requirement for B5/B6.
        fn assert_shareable(_: std::sync::Arc<dyn MetricsRepository + Send + Sync>) {}
        let repo: std::sync::Arc<dyn MetricsRepository + Send + Sync> =
            std::sync::Arc::new(SqliteRepository::open_in_memory().unwrap());
        assert_shareable(repo);
    }
}
