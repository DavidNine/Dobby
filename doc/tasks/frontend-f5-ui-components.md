# F5 — UI Components（純呈現元件）

> 對應設計：HLD §4 F5
> 職責：吃 props、無副作用的純呈現元件，與資料抓取解耦。
> 依賴：F3 Format Utils。

## 完成定義 (DoD)
- 四個元件實作完成，皆為純 props-in。
- 給定 props 的渲染 / 快照測試通過。
- 基本 RWD（桌機 / 手機皆可讀）。

## 子任務
- [x] `MetricCard`：單一指標目前數值大卡（標題 + 大數字 + 單位），用 F3 格式化
- [x] `MetricLineChart`：Chart.js 折線圖封裝（吃 points 陣列；x 軸時間、y 軸值）
- [x] `RangeSelector`：`1h / 6h / 24h / 7d` 切換鈕，回呼選中的 range
- [x] `StatusBar`：連線狀態 / 最後更新時間 / 錯誤提示
- [x] 以 TailwindCSS 維持簡潔風格 + RWD（手機單欄、桌機多欄）

## 測試（給定 props 渲染 / 快照）
- [x] `MetricCard` 依 props 顯示正確格式化數值
- [x] `MetricLineChart` 吃 points 正常渲染（可 mock chart.js）
- [x] `RangeSelector` 點擊觸發正確 range 回呼、標示目前選中
- [x] `StatusBar` 顯示更新時間與錯誤狀態
