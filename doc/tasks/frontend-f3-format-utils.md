# F3 — Format Utils（格式化純函式）

> 對應設計：HLD §4 F3、§6（單位換算只在前端）
> 職責：純函式格式化，不碰 React，最易測試。
> 依賴：無。

## 完成定義 (DoD)
- 所有格式化函式為純函式並通過單元測試。

## 子任務
- [x] `formatBytes(bytes)`：→ 人類可讀（B/KB/MB/GB）
- [x] `formatRate(bps)`：bytes/sec → KB/s、MB/s 自動選單位
- [x] `formatPercent(value)`：→ 帶 1 位小數的百分比字串
- [x] `formatTimestamp(ts)`：Unix epoch 秒 → 本地時間顯示（時區換算只在此層）
- [x] `formatTimeAxis(ts, range)`：依範圍決定軸標籤格式（時:分 / 月/日）

## 測試
- [x] `formatBytes`：0、1023、1024、MB、GB 邊界
- [x] `formatRate`：低/高速率單位切換正確
- [x] `formatPercent`：小數位與邊界（0、100）
- [x] `formatTimestamp` / `formatTimeAxis`：固定輸入產生預期字串
