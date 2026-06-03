# F4 — usePolling Hook（輪詢）

> 對應設計：HLD §4 F4、§5(C)
> 職責：每 10 秒輪詢 API、管理 loading/error/data、頁面隱藏時可暫停。
> 依賴：F1 API Client。

## 完成定義 (DoD)
- Hook 能定時輪詢並暴露 data / loading / error。
- 以假計時器 + mock client 測試通過。

## 子任務
- [x] `useMetricsPolling(range)`：回傳 `{ current, history, loading, error, lastUpdated }`
- [x] 掛載時立即抓一次，之後每 10 秒（間隔可由設定 / env 調整）輪詢
- [x] `range` 改變時重新抓 history
- [x] 頁面隱藏（`document.hidden` / `visibilitychange`）時暫停輪詢，回到前景時恢復
- [x] 卸載時清除計時器與進行中的請求
- [x] 錯誤時保留上一筆資料、設定 error 狀態

## 測試（React Testing Library + 假計時器 + mock client）
- [x] 掛載立即抓一次
- [x] 每 10 秒再抓（推進假計時器驗證呼叫次數）
- [x] range 改變觸發重新抓 history
- [x] 頁面隱藏暫停、恢復後繼續
- [x] client 拋錯時 error 被設定且不崩潰
