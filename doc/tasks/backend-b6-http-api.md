# B6 — HTTP API（路由 / handler / CORS）

> 對應設計：HLD §3 B6、§6
> 職責：定義 axum 路由、處理請求、序列化回應、套用 CORS。只依賴 `MetricsRepository` 抽象。
> 依賴：B4 Storage(trait)、B2 Config。

## 完成定義 (DoD)
- 三個端點實作完成、回應符合資料契約。
- CORS 套用、range 驗證正確。
- 注入 FakeRepository，對 Router 發測試請求通過。

## 子任務
- [x] 定義回應 DTO（`serde::Serialize`）：
  - [x] `CurrentResponse`：`{ timestamp, cpu:{percent}, memory:{total_bytes,used_bytes,percent}, network:{rx_bps,tx_bps} }`
  - [x] `HistoryResponse`：`{ range, bucket_secs, points:[{timestamp,cpu_percent,mem_percent,net_rx_bps,net_tx_bps}] }`
- [x] 建立 `Router`，注入 repo（`State`），含路由：
  - [x] `GET /api/health` → `{ "status": "ok" }`
  - [x] `GET /api/metrics/current` → 最新一筆；查無資料回 `204`/空
  - [x] `GET /api/metrics/history?range=...` → 降抽樣序列；查無資料回空陣列
- [x] range 參數驗證：非法 `range` → `400`
- [x] 內部錯誤 → `500 { "error": "..." }`（由 `AppError` 對應狀態碼）
- [x] 套用 `tower-http::cors`，允許來源由 Config 提供

## 測試（注入 FakeRepository + axum 測試請求）
- [x] `/api/health` 回 200 與正確 JSON
- [x] `/api/metrics/current` 有資料回 200 + 正確結構；無資料回 204/空
- [x] `/api/metrics/history?range=24h` 回 200 + 正確結構與 `bucket_secs`
- [x] 非法 range（如 `?range=99x`）回 400
- [x] 內部錯誤回 500 + `{error}`
