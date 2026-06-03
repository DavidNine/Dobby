# B7 — main.rs（組裝啟動）

> 對應設計：HLD §8（main.rs：載入 Config、起 Sampler、起 HTTP）
> 職責：把各模組組裝成可執行的 binary。
> 依賴：B2 Config、B4 Storage、B3 Collector、B5 Sampler、B6 HTTP API。

## 完成定義 (DoD)
- `cargo run` 能啟動服務：背景取樣寫入 DB、HTTP API 可回應。
- 綁定 `0.0.0.0:<port>`，同區網裝置可存取。

## 子任務
- [x] 初始化 logging（`tracing-subscriber`）
- [x] `Config::load()`，失敗則印錯並結束
- [x] 建立 `SqliteRepository`（依 `db_path`，建表）
- [x] 建立 `SysinfoCollector`
- [x] spawn `Sampler::run()` 為背景任務
- [x] 建立 HTTP `Router`（注入 repo + CORS），`bind` 到 `MONITOR_BIND:MONITOR_PORT`
- [x] 優雅關閉（選配：捕捉 Ctrl-C）

## 測試 / 驗證
- [x] 本機啟動後 `curl /api/health` 回 ok
- [x] 等待數個取樣週期後 `curl /api/metrics/current` 有資料
- [x] `curl /api/metrics/history?range=1h` 回時間序列
