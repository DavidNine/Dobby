# B2 — Config（設定）

> 對應設計：HLD §3 B2
> 職責：集中管理可調參數，從環境變數載入、提供預設並驗證。
> 依賴：B1 Domain（`AppError`）。為純邏輯，無 I/O。

## 完成定義 (DoD)
- `Config::load()` 可從環境變數解析，套用預設，拒絕非法值。

## 子任務
- [x] 定義 `struct Config`，欄位涵蓋下表所有參數
- [x] 實作 `Config::load() -> Result<Config, AppError>`：讀環境變數、套預設、轉型
- [x] 驗證規則：`sample_interval_secs > 0`、`retention_days > 0`、`cleanup_interval_secs > 0`、`port` 合法
- [x] 解析失敗 / 非法值 → 回 `AppError::Config`

### 參數對照
| 欄位 | 環境變數 | 預設 |
|------|----------|------|
| bind | `MONITOR_BIND` | `0.0.0.0` |
| port | `MONITOR_PORT` | `8080` |
| sample_interval_secs | `MONITOR_SAMPLE_INTERVAL_SECS` | `10` |
| retention_days | `MONITOR_RETENTION_DAYS` | `7` |
| cleanup_interval_secs | `MONITOR_CLEANUP_INTERVAL_SECS` | `3600` |
| db_path | `MONITOR_DB_PATH` | `./monitor.db` |
| cors_origins | `MONITOR_CORS_ORIGINS` | `*` |

## 測試
- [x] 未設任何環境變數 → 全部回預設值
- [x] 設定自訂值 → 正確解析覆蓋預設
- [x] 非法值（如 `MONITOR_SAMPLE_INTERVAL_SECS=0`、非數字 port）→ 回 `AppError::Config`
