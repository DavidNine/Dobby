# B1 — Domain（領域型別）

> 對應設計：HLD §3 B1
> 職責：定義跨模組共用、與框架無關的純資料結構與錯誤型別。
> 依賴：無。**不得**依賴 sysinfo / axum / sqlite。

## 完成定義 (DoD)
- 所有核心型別定義完成、可編譯。
- 型別不引入任何 I/O 套件。

## 子任務
- [x] 定義 `struct MetricSample`：`timestamp: i64`、`cpu_percent: f32`、`mem_total_bytes: u64`、`mem_used_bytes: u64`、`mem_percent: f32`、`net_rx_rate_bps: f64`、`net_tx_rate_bps: f64`
- [x] 定義 `struct MetricPoint`（降抽樣後查詢結果用）：`timestamp`、`cpu_percent`、`mem_percent`、`net_rx_bps`、`net_tx_bps`
- [x] 定義 `enum TimeRange { Hour1, Hour6, Hour24, Day7 }`
- [x] 為 `TimeRange` 實作由字串解析（`"1h"`/`"6h"`/`"24h"`/`"7d"` → `TimeRange`），非法值回錯誤
- [x] 為 `TimeRange` 提供「範圍秒數」與「bucket 秒數」對照（1h→10、6h→60、24h→240、7d→1800）
- [x] 定義 `enum AppError { Config(..), Storage(..), Collect(..) }`，並實作 `Display` / `Error`
- [x] 為需序列化的型別加上 `serde::Serialize`/`Deserialize`（視 API 模組需要）

## 測試
- [x] `TimeRange` 字串解析：合法值正確、非法值回錯誤
- [x] `TimeRange` → bucket 秒數對照正確
