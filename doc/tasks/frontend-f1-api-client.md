# F1 — API Client（HTTP 封裝）

> 對應設計：HLD §4 F1
> 職責：封裝對 Backend 的 HTTP 呼叫，回傳型別化資料。
> 依賴：F2 Types。

## 完成定義 (DoD)
- `getCurrent()` / `getHistory(range)` 可呼叫正確 URL 並回傳型別化資料。
- mock fetch 測試通過。

## 子任務
- [x] 由 `import.meta.env.VITE_API_BASE` 取得 API base URL
- [x] `getCurrent(): Promise<CurrentResponse | null>`（204/空 → null）
- [x] `getHistory(range): Promise<HistoryResponse>` → `GET /api/metrics/history?range=<range>`
- [x] 錯誤處理：非 2xx 拋出可辨識錯誤（供 Hook 顯示）
- [x] 回應解析為對應 TS 型別

## 測試（mock `fetch`）
- [x] `getCurrent` 組出正確 URL、解析回應
- [x] `getHistory` 帶上正確 `range` query
- [x] 204 → 回 null
- [x] 非 2xx → 拋錯
