# B3 — Collector（指標收集）

> 對應設計：HLD §3 B3
> 職責：封裝 `sysinfo`，每次呼叫回傳當下的 `MetricSample`。
> 依賴：B1 Domain。

## 完成定義 (DoD)
- `trait MetricsCollector` 定義完成。
- 速率換算為獨立純函式並通過測試。
- 真實 `SysinfoCollector` 通過 smoke test。
- 提供 `FakeCollector` 供 B5 測試使用。

## 子任務
- [x] 定義 `trait MetricsCollector { fn collect(&mut self) -> Result<MetricSample, AppError>; }`
- [x] 實作純函式 `compute_rate(prev_bytes, curr_bytes, dt_secs) -> f64`
  - [x] 一般情況：`(curr - prev) / dt`
  - [x] 首次（無前值）→ 回 `0`
  - [x] 計數器回繞（`curr < prev`）→ 回 `0`（避免負值）
  - [x] `dt <= 0` 的防護
- [x] 實作 `SysinfoCollector`：保存上次網路累計位元組與時間戳
  - [x] CPU：整體使用率（不分核心）
  - [x] 記憶體：total / used / 換算 percent
  - [x] 網路：rx/tx 以 `compute_rate` 換算速率
  - [x] 注意 sysinfo CPU 需間隔刷新才有有效值（首次刷新處理）
- [x] 實作 `FakeCollector`：回傳預設 `MetricSample` 序列（含可模擬失敗）

## 測試
- [x] `compute_rate`：一般 / 首次 / 回繞 / dt=0 各情境
- [x] `SysinfoCollector` smoke test：能成功 collect、各值落在合理範圍（cpu 0–100、mem_used ≤ total）
