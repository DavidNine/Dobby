# 系統監控儀表板 — 概要設計文件 (High-Level Design)

> 文件版本：v1.0　|　建立日期：2026-06-03
> 上游依據：[`proposal.md`](../proposal.md)（需求文件 v1.0）
> 本文件將需求拆解為**互相獨立、可獨立測試**的模組，並定義各模組的職責、介面與資料契約。

---

## 0. 本次設計訪談補充確認

在需求文件之外，本設計另確認以下三項設計層級決策：

| 設計議題 | 決定 |
|----------|------|
| 長時間範圍歷史資料 | **由 API 自動降抽樣 (downsampling)**：依查詢範圍決定聚合粒度，控制回傳點數 |
| 系統參數（埠口 / 取樣間隔 / 保留天數等） | **可透過環境變數或設定檔調整** → 獨立出 **Config 模組** |
| CPU 儲存粒度 | **只儲存整體 CPU%**（不存各核心），schema 從簡 |

---

## 1. 設計目標與原則

1. **模組獨立、可單獨測試**：每個模組對外只透過明確的介面（Rust 用 `trait`、TypeScript 用 interface）溝通，依賴以抽象注入，能在不啟動真實系統 / DB / 網路的情況下單元測試。
2. **單向依賴**：模組依賴關係為單向、無循環。上層依賴下層的「抽象」而非「實作」。
3. **核心邏輯與 I/O 邊界分離**：取樣（讀系統）、儲存（讀寫 DB）、對外服務（HTTP）三個 I/O 邊界各自隔離；純運算（降抽樣、速率換算、格式化）抽成不碰 I/O 的純函式，最易測試。
4. **失敗隔離**：單次取樣或寫入失敗只記錄日誌、不使整個程式崩潰（呼應需求非功能項「可靠性」）。

---

## 2. 系統整體架構

系統分為兩個獨立部署的服務：**Backend（Rust binary）** 與 **Frontend（React SPA）**，兩者僅透過 HTTP JSON API 溝通，可各自獨立開發、測試與啟動。

```
            ┌──────────────────────────── Backend (Rust) ────────────────────────────┐
            │                                                                          │
  系統 ────▶│  [Collector] ──MetricSample──▶ [Sampler] ──▶ [Storage(trait)]──▶ SQLite │
 (sysinfo)  │       ▲                            │ (每10s)        ▲                     │
            │       │                            │ (每1h清理)      │ 查詢/降抽樣          │
            │   讀系統指標                                         │                     │
            │                                              [HTTP API (axum)]            │
            │                                                     ▲                     │
            └─────────────────────────────────────────────────────┼─────────────────────┘
                                                                  │ HTTP (JSON) + CORS
            ┌──────────────────────────── Frontend (React) ───────┼─────────────────────┐
            │  [API Client] ◀──useMetricsPolling(每10s)──▶ [State/Range]                 │
            │       │                                          │                         │
            │       ▼                                          ▼                         │
            │  [Format Utils(純函式)] ───▶ [UI Components: Cards / Charts / RangeSelector]│
            └──────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Backend 模組設計

Backend 共拆為 6 個模組。下表為總覽，後續逐一展開。

| # | 模組 | 職責 | 對外介面（抽象） | 依賴 | 可測試方式 |
|---|------|------|------------------|------|-----------|
| B1 | **Domain（領域型別）** | 定義共用資料結構與錯誤型別 | 純資料型別 | 無 | 不需測（型別） |
| B2 | **Config** | 載入並驗證設定 | `Config::load()` | Domain | 餵不同環境變數驗證解析與預設值 |
| B3 | **Collector** | 從系統取得一次指標快照 | `trait MetricsCollector` | Domain | 速率換算純函式 + Fake collector |
| B4 | **Storage** | 寫入、查詢（含降抽樣）、清理 | `trait MetricsRepository` | Domain | in-memory SQLite |
| B5 | **Sampler** | 背景排程：定時取樣寫入 + 定時清理 | `Sampler::run()` | Collector, Storage, Config | 注入 Fake collector/repo + 加速時鐘 |
| B6 | **HTTP API** | 路由、handler、DTO、CORS | axum `Router` | Storage(trait), Config | 注入 Fake repo，對 Router 發測試請求 |

依賴方向：`HTTP API / Sampler → Storage(trait) / Collector(trait) → Domain`；`Config → Domain`。皆為單向、無循環。

---

### B1. Domain（領域型別）

放置跨模組共用、與框架無關的純資料結構，是各模組之間的「共同語言」。

**核心型別（概念定義）：**

```rust
/// 單一時間點的指標快照
struct MetricSample {
    timestamp: i64,          // Unix epoch 秒
    cpu_percent: f32,        // 0.0 ~ 100.0
    mem_total_bytes: u64,
    mem_used_bytes: u64,
    mem_percent: f32,        // 0.0 ~ 100.0
    net_rx_rate_bps: f64,    // 下載速率，bytes/sec
    net_tx_rate_bps: f64,    // 上傳速率，bytes/sec
}

/// 歷史查詢的時間範圍
enum TimeRange { Hour1, Hour6, Hour24, Day7 }

/// 統一錯誤型別
enum AppError { Config(..), Storage(..), Collect(..) }
```

> 設計重點：Domain **不依賴** sysinfo / axum / sqlite 任何套件，確保上層可自由替換實作。

---

### B2. Config 模組

**職責**：集中管理所有可調參數，從環境變數（或 `.env` / 設定檔）載入，提供合理預設並驗證。

| 參數 | 環境變數 | 預設值 | 說明 |
|------|----------|--------|------|
| 監聽位址 | `MONITOR_BIND` | `0.0.0.0` | 綁定位址（區網存取） |
| 監聽埠口 | `MONITOR_PORT` | `8080` | API 埠口 |
| 取樣間隔 | `MONITOR_SAMPLE_INTERVAL_SECS` | `10` | 背景取樣秒數 |
| 保留天數 | `MONITOR_RETENTION_DAYS` | `7` | 歷史保留天數 |
| 清理間隔 | `MONITOR_CLEANUP_INTERVAL_SECS` | `3600` | 清理舊資料的週期 |
| 資料庫路徑 | `MONITOR_DB_PATH` | `./monitor.db` | SQLite 檔案路徑 |
| 允許來源 | `MONITOR_CORS_ORIGINS` | `*` | CORS 允許來源 |

**對外介面**：`Config::load() -> Result<Config, AppError>`
**測試方式**：設定不同環境變數，驗證解析、預設值套用、非法值（如 interval=0）被拒絕。為純邏輯，無 I/O。

---

### B3. Collector 模組

**職責**：封裝 `sysinfo`，每次呼叫回傳一個當下的 `MetricSample`。

**對外介面（抽象）**：

```rust
trait MetricsCollector {
    fn collect(&mut self) -> Result<MetricSample, AppError>;
}
```

**內部要點：**
- **CPU**：取整體使用率（不分核心）。
- **記憶體**：total / used 與換算百分比。
- **網路速率換算（純函式，獨立可測）**：sysinfo 提供的是累計位元組。Collector 需保存「上一次的累計值與時間戳」，本次以
  `rate = (本次累計 − 上次累計) / (本次時間 − 上次時間)`
  計算 rx/tx 速率。
  - 第一次取樣無前值 → 速率以 `0` 回傳。
  - 處理計數器重置（如介面重啟導致本次 < 上次）→ 該次速率以 `0` 回傳，避免負值。

**測試方式：**
1. 把「速率換算」抽成純函式 `compute_rate(prev, curr, dt)`，用固定輸入測試一般情況、首次、計數器回繞。
2. 提供 `FakeCollector` 回傳預設序列，供 Sampler 測試使用。
3. 真實 `SysinfoCollector` 以 smoke test（能跑、值在合理範圍）驗證。

---

### B4. Storage 模組

**職責**：負責 SQLite 的寫入、歷史查詢（含降抽樣）、過期清理。

**對外介面（抽象）**：

```rust
trait MetricsRepository {
    fn insert(&self, sample: &MetricSample) -> Result<(), AppError>;
    fn latest(&self) -> Result<Option<MetricSample>, AppError>;
    fn query_range(&self, range: TimeRange) -> Result<Vec<MetricPoint>, AppError>; // 已降抽樣
    fn delete_older_than(&self, cutoff_ts: i64) -> Result<u64, AppError>;
}
```

**資料庫 schema（單表）：**

```sql
CREATE TABLE IF NOT EXISTS metrics (
    ts               INTEGER NOT NULL,  -- Unix epoch 秒
    cpu_percent      REAL    NOT NULL,
    mem_total_bytes  INTEGER NOT NULL,
    mem_used_bytes   INTEGER NOT NULL,
    mem_percent      REAL    NOT NULL,
    net_rx_rate_bps  REAL    NOT NULL,
    net_tx_rate_bps  REAL    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_metrics_ts ON metrics(ts);
```

**降抽樣 (Downsampling) 設計（核心演算法，可獨立測試）：**

目標：任何範圍回傳的點數控制在約 **N（預設 ≈ 360 點）** 以內，圖表才順。

- 依範圍決定每個聚合桶 (bucket) 的秒數 `bucket_secs`：

  | 範圍 | 原始點數(約) | bucket 秒數 | 聚合後點數(約) |
  |------|------|-------------|------|
  | 1 小時 | 360 | 10（不聚合） | 360 |
  | 6 小時 | 2,160 | 60 | 360 |
  | 24 小時 | 8,640 | 240 | 360 |
  | 7 天 | 60,480 | 1,800 | 336 |

- 以 SQL `GROUP BY (ts / bucket_secs)` 對每個桶取 **平均值（CPU%、mem%、速率）**，桶時間取桶起點。
- 1 小時範圍 bucket=10 等同原始解析度，不損失精度。

> 演算法（給定 range → bucket_secs、SQL 聚合）抽為純邏輯，可用固定資料集驗證聚合正確性。

**測試方式**：用 **in-memory SQLite**（`:memory:`）建表、塞入已知樣本，驗證 insert / latest / query_range（含各範圍的桶大小與平均值）/ delete_older_than 的行為。完全不需真實檔案或系統。

---

### B5. Sampler 模組（背景排程）

**職責**：協調 Collector 與 Storage 的背景常駐任務。

**行為：**
- **取樣迴圈**：每 `sample_interval_secs` 呼叫 `collector.collect()`，成功則 `repo.insert()`。
- **清理迴圈**：每 `cleanup_interval_secs` 呼叫 `repo.delete_older_than(now − retention)`。
- **失敗隔離**：任一次 collect / insert / delete 失敗 → 記 log、跳過該次、迴圈繼續（不 panic、不中斷服務）。

**對外介面**：`Sampler::new(collector, repo, config).run()`（spawn 為 tokio 背景任務）。

**測試方式**：注入 `FakeCollector` + `FakeRepository`（或 in-memory repo），並以可控/加速的時間來源，驗證「固定時間內被呼叫 N 次」「collect 失敗時不影響後續迴圈」「清理用正確的 cutoff」。因依賴皆為注入的抽象，無需真實系統或時鐘。

---

### B6. HTTP API 模組

**職責**：定義 axum 路由、處理請求、序列化回應、套用 CORS。**只依賴 `MetricsRepository` 抽象**，不直接碰 DB。

**端點與資料契約：**

| 方法 | 路徑 | 說明 | 回應 |
|------|------|------|------|
| GET | `/api/health` | 健康檢查 | `{ "status": "ok" }` |
| GET | `/api/metrics/current` | 最新一筆 | `CurrentResponse` |
| GET | `/api/metrics/history?range=1h\|6h\|24h\|7d` | 歷史（已降抽樣） | `HistoryResponse` |

**回應 JSON 範例：**

```jsonc
// GET /api/metrics/current
{
  "timestamp": 1717400000,
  "cpu": { "percent": 23.5 },
  "memory": { "total_bytes": 17179869184, "used_bytes": 8589934592, "percent": 50.0 },
  "network": { "rx_bps": 125000.0, "tx_bps": 34000.0 }
}

// GET /api/metrics/history?range=24h
{
  "range": "24h",
  "bucket_secs": 240,
  "points": [
    { "timestamp": 1717310000, "cpu_percent": 20.1, "mem_percent": 48.0, "net_rx_bps": 100000, "net_tx_bps": 20000 }
    // ...約 360 筆
  ]
}
```

**錯誤回應**：非法 `range` → `400`；查無資料 → `current` 回 `204`/空、`history` 回空陣列；內部錯誤 → `500 { "error": "..." }`。

**CORS**：以 `tower-http::cors` 套用，允許來源由 Config 提供。

**測試方式**：注入 `FakeRepository`，用 axum 的測試工具對 `Router` 發送請求，斷言狀態碼、JSON 結構與 `range` 參數驗證；完全不需啟動真實伺服器或 DB。

---

## 4. Frontend 模組設計

Frontend 以「資料層 / 狀態層 / 呈現層」分層，純邏輯（格式化）獨立成可單元測試的模組。

| # | 模組 | 職責 | 對外介面 | 依賴 | 可測試方式 |
|---|------|------|----------|------|-----------|
| F1 | **API Client** | 封裝對 Backend 的 HTTP 呼叫，回傳型別化資料 | `getCurrent()` / `getHistory(range)` | 型別定義 | mock `fetch`，驗證 URL / 解析 |
| F2 | **Types** | TS 型別（對應 API 契約） | interface | 無 | 不需測 |
| F3 | **Format Utils** | 純函式格式化（bytes→人類可讀、bps→KB/s、%、時間軸） | `formatBytes()` 等 | 無 | 純函式單元測試 |
| F4 | **usePolling Hook** | 每 10 秒輪詢、管理 loading/error/data、頁面隱藏時可暫停 | `useMetricsPolling()` | API Client | React Testing Library + 假計時器 + mock client |
| F5 | **UI Components** | 純呈現元件 | props in | F3 | 給定 props 快照 / 渲染測試 |
| F6 | **Dashboard 容器** | 組合狀態與元件、管理選定範圍 | — | F4, F5 | 整合測試 |

**F5 UI 元件清單：**
- `MetricCard`：單一指標的目前數值大卡（CPU% / RAM / 網路速率）。
- `MetricLineChart`：Chart.js 折線圖封裝（吃 points 陣列）。
- `RangeSelector`：1h / 6h / 24h / 7d 切換鈕。
- `StatusBar`：連線狀態 / 最後更新時間 / 錯誤提示。

**設計重點：**
- F3（格式化）與 F1（資料）是純邏輯，不碰 React，最容易測。
- F5 元件皆為「吃 props、無副作用」的純呈現元件，與資料抓取解耦 → 可單獨用 Storybook/快照驗證。
- 輪詢間隔、API base URL 由前端設定（`.env` / Vite env，如 `VITE_API_BASE`），與 Backend 對齊。

---

## 5. 端到端資料流 (Sequence)

**(A) 取樣寫入（背景，每 10s）**
```
Sampler 計時器到點 → Collector.collect() → MetricSample
                  → Storage.insert(sample) → SQLite
（失敗則記 log 並跳過，迴圈續行）
```

**(B) 清理（背景，每 1h）**
```
Sampler 清理計時器到點 → Storage.delete_older_than(now − 7天)
```

**(C) 前端讀取（每 10s）**
```
usePolling 計時器 → API Client.getCurrent()/getHistory(range)
   → HTTP GET /api/metrics/... → HTTP API handler
   → Storage.latest()/query_range(range)（降抽樣）→ JSON
   → Format Utils → UI Components 重新渲染
```

---

## 6. 跨模組約定與一致性

- **時間**：一律使用 **Unix epoch 秒 (UTC)** 作為跨模組與 API 的時間單位；時區換算只在前端顯示層處理。
- **網路單位**：API 一律用 **bytes/sec**；KB/s、MB/s 的換算只在前端 Format Utils。
- **range 取值**：`1h` / `6h` / `24h` / `7d`，前後端共用同一組字串。
- **錯誤語意**：Backend 用一致的 `AppError` 與 HTTP 狀態碼對應表（見 B6）。

---

## 7. 各模組獨立測試策略總表

| 模組 | 測試替身 / 環境 | 不需要 |
|------|----------------|--------|
| Config | 設定環境變數 | DB / 網路 / 系統 |
| Collector | 純函式測速率換算；真實實作做 smoke test | DB / HTTP |
| Storage | in-memory SQLite (`:memory:`) | 系統 / HTTP |
| Sampler | Fake Collector + Fake/in-memory Repo + 可控時鐘 | 真實系統 / 真實 DB |
| HTTP API | Fake Repository + axum 測試請求 | DB / 真實伺服器 |
| F1/F3 | mock fetch / 純函式 | 瀏覽器 / 後端 |
| F4/F5 | React Testing Library + 假計時器 + mock client | 真實後端 |

> 由於所有 I/O 邊界（系統、DB、HTTP）皆以抽象介面注入，每個模組都能在不啟動其他模組的情況下單獨測試，達成需求中「模組互相獨立、可獨立測試」的目標。

---

## 8. 專案結構建議

```
.
├── proposal.md
├── doc/
│   └── high-level-design.md        ← 本文件
├── backend/                        ← Rust 專案（獨立服務）
│   └── src/
│       ├── main.rs                 ← 組裝：載入 Config、起 Sampler、起 HTTP
│       ├── domain.rs               ← B1
│       ├── config.rs               ← B2
│       ├── collector.rs            ← B3（trait + Sysinfo 實作 + Fake）
│       ├── storage.rs              ← B4（trait + SQLite 實作）
│       ├── sampler.rs              ← B5
│       └── api.rs                  ← B6（Router + handlers + DTO）
└── frontend/                       ← React + Vite 專案（獨立服務）
    └── src/
        ├── api/client.ts           ← F1
        ├── api/types.ts            ← F2
        ├── utils/format.ts         ← F3
        ├── hooks/usePolling.ts     ← F4
        ├── components/             ← F5
        └── pages/Dashboard.tsx     ← F6
```

---

## 9. 範圍與未決事項

- **範圍**：完全對齊 `proposal.md` 第 7 節（單機；CPU/RAM/網路；7 天；前後端分離；無警示、無登入）。
- **本設計的明確取捨**：CPU 只存整體；長範圍 API 降抽樣（平均聚合）；參數可由 Config 調整。
- **未決事項**：無。下一步可進入**詳細設計 / 實作**（定義各 trait 的完整簽章、SQL 語句、API DTO 的逐欄位細節）。
```
