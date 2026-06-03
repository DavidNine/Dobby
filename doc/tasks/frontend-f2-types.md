# F2 — Types（TS 型別）

> 對應設計：HLD §4 F2
> 職責：定義對應 Backend API 契約的 TypeScript 型別。
> 依賴：無。不需測試。

## 完成定義 (DoD)
- 型別與 Backend B6 的 DTO 一一對應。

## 子任務
- [x] `interface CurrentResponse`：`timestamp`、`cpu:{percent}`、`memory:{total_bytes,used_bytes,percent}`、`network:{rx_bps,tx_bps}`
- [x] `interface HistoryPoint`：`timestamp`、`cpu_percent`、`mem_percent`、`net_rx_bps`、`net_tx_bps`
- [x] `interface HistoryResponse`：`range`、`bucket_secs`、`points: HistoryPoint[]`
- [x] `type Range = '1h' | '6h' | '24h' | '7d'`
