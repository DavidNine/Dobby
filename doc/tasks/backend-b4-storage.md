# B4 — Storage（儲存 / 降抽樣 / 清理）

> 對應設計：HLD §3 B4
> 職責：SQLite 寫入、歷史查詢（含降抽樣）、過期清理。
> 依賴：B1 Domain。

## 完成定義 (DoD)
- `trait MetricsRepository` 定義完成。
- SQLite 實作通過 in-memory（`:memory:`）測試。
- 降抽樣聚合正確。

## 子任務
- [x] 定義 `trait MetricsRepository`：
  - `fn insert(&self, sample: &MetricSample) -> Result<(), AppError>`
  - `fn latest(&self) -> Result<Option<MetricSample>, AppError>`
  - `fn query_range(&self, range: TimeRange) -> Result<Vec<MetricPoint>, AppError>`（已降抽樣）
  - `fn delete_older_than(&self, cutoff_ts: i64) -> Result<u64, AppError>`
- [x] 建表與索引（啟動時 `CREATE TABLE IF NOT EXISTS metrics ...` + `idx_metrics_ts`）
- [x] 實作 `SqliteRepository`（可接受 `:memory:` 或檔案路徑）
- [x] 實作 `insert`：寫入單筆 sample
- [x] 實作 `latest`：取 `ts` 最大的一筆
- [x] 實作 `query_range`：
  - [x] 純函式 `bucket_secs_for(range)`：1h→10、6h→60、24h→240、7d→1800
  - [x] SQL `GROUP BY (ts / bucket_secs)`，對 cpu%/mem%/rx/tx 取平均，桶時間取桶起點
  - [x] 依範圍計算起始時間（`now - range_secs`）作為 where 條件
- [x] 實作 `delete_older_than`：刪除 `ts < cutoff`，回傳刪除筆數
- [x] 失敗統一轉 `AppError::Storage`

## 測試（全用 in-memory SQLite）
- [x] insert 後 latest 回最新一筆
- [x] 無資料時 latest 回 `None`
- [x] query_range 各範圍 bucket 大小正確、平均值聚合正確（用已知資料集）
- [x] 1h 範圍 bucket=10 等同原始解析度
- [x] delete_older_than 只刪過期、回傳正確刪除筆數
