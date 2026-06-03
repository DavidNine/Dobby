# 00 — 專案骨架與環境

> 對應設計：HLD §8 專案結構建議
> 目標：建立前後端兩個獨立專案的最小可編譯骨架，後續模組才有落腳處。

## 完成定義 (DoD)
- `backend/` 可 `cargo build` 通過（即使只是空殼）。
- `frontend/` 可 `npm run dev` 啟動空白頁。
- 兩個專案目錄結構符合 HLD §8。

## 子任務

### Backend 骨架
- [x] 建立 `backend/` 並 `cargo init --bin`
- [x] 在 `Cargo.toml` 加入依賴：`axum`、`tokio`（features = full）、`sysinfo`、`serde`/`serde_json`、`tower-http`（features = cors）、SQLite（`rusqlite` 或 `sqlx`，擇一並記錄決定）
- [x] 加入測試/輔助依賴：`anyhow` 或 `thiserror`（錯誤）、logging（`tracing` + `tracing-subscriber`）
- [x] 建立空模組檔：`src/domain.rs`、`config.rs`、`collector.rs`、`storage.rs`、`sampler.rs`、`api.rs`，並在 `main.rs` 以 `mod` 宣告
- [x] `cargo build` 通過

### Frontend 骨架
- [x] 以 Vite 建立 React + TypeScript 專案於 `frontend/`
- [x] 安裝並設定 TailwindCSS
- [x] 安裝 `chart.js` 與 `react-chartjs-2`
- [x] 建立目錄：`src/api/`、`src/utils/`、`src/hooks/`、`src/components/`、`src/pages/`
- [x] 設定 `VITE_API_BASE` 環境變數（`.env`，預設指向 `http://localhost:8080`）
- [x] 設定測試環境（Vitest + React Testing Library）
- [x] `npm run dev` 可啟動空白頁

### 共用約定（寫入 README 或註解）
- [x] 記錄跨模組約定：時間用 Unix epoch 秒(UTC)；網路單位 bytes/sec；range 字串 `1h`/`6h`/`24h`/`7d`（HLD §6）
