# F6 — Dashboard（容器頁面）

> 對應設計：HLD §4 F6、§5(C)
> 職責：組合狀態與元件、管理選定範圍。
> 依賴：F4 usePolling、F5 UI Components。

## 完成定義 (DoD)
- 單頁儀表板可運作：即時卡片 + 歷史折線圖 + 範圍切換 + 狀態列。
- 與真實 Backend 串接後畫面正確更新。

## 子任務
- [x] 管理 `range` state，傳入 `useMetricsPolling(range)`
- [x] 渲染 CPU / RAM / 網路三張 `MetricCard`（吃 current）
- [x] 渲染 CPU / RAM / 網路 `MetricLineChart`（吃 history.points）
- [x] 渲染 `RangeSelector`（改變 range）與 `StatusBar`（lastUpdated / error）
- [x] loading / error / 空資料的畫面處理
- [x] 版面 RWD：桌機多欄、手機單欄

## 測試 / 驗證
- [x] 整合測試：mock client 下，切換 range 觸發重新抓取、卡片與圖表更新
- [x] 與真實 Backend 串接：數值每 10 秒更新、切換範圍圖表正確
