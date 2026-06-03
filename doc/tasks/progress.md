# 系統監控儀表板 — 總體進度 (Progress)

> 文件版本：v1.0　|　建立日期：2026-06-03
> 上游依據：[`proposal.md`](../../proposal.md)、[`high-level-design.md`](../high-level-design.md)
> 本檔以 check list 追蹤每個模組的整體完成狀態。各模組的子任務細節見對應的 `doc/tasks/<module>.md`。

---

## 進度總覽

依「**由下而上、依賴先行**」的順序開發：先共用型別與設定，再 I/O 邊界（取樣 / 儲存 / API），最後背景排程與前端。

### 專案初始化
- [x] [00 — 專案骨架與環境](./00-project-setup.md)

### Backend（Rust）
- [x] [B1 — Domain（領域型別）](./backend-b1-domain.md)
- [x] [B2 — Config（設定）](./backend-b2-config.md)
- [x] [B3 — Collector（指標收集）](./backend-b3-collector.md)
- [x] [B4 — Storage（儲存 / 降抽樣 / 清理）](./backend-b4-storage.md)
- [x] [B5 — Sampler（背景排程）](./backend-b5-sampler.md)
- [x] [B6 — HTTP API（路由 / handler / CORS）](./backend-b6-http-api.md)
- [x] [B7 — main.rs（組裝啟動）](./backend-b7-main.md)

### Frontend（React + Vite）
- [x] [F2 — Types（TS 型別）](./frontend-f2-types.md)
- [x] [F3 — Format Utils（格式化純函式）](./frontend-f3-format-utils.md)
- [x] [F1 — API Client（HTTP 封裝）](./frontend-f1-api-client.md)
- [x] [F4 — usePolling Hook（輪詢）](./frontend-f4-use-polling.md)
- [x] [F5 — UI Components（純呈現元件）](./frontend-f5-ui-components.md)
- [x] [F6 — Dashboard（容器頁面）](./frontend-f6-dashboard.md)

### 整合驗證
- [x] [99 — 端到端整合測試](./99-integration.md)

---

## 建議開發順序（依賴圖）

```
00 專案骨架
  │
  ├─ Backend:  B1 → B2
  │              ├─ B3 (Collector)
  │              ├─ B4 (Storage)
  │              └─ B3+B4 → B5 (Sampler)
  │                 B4    → B6 (HTTP API)
  │                 B2+B5+B6 → B7 (main 組裝)
  │
  └─ Frontend: F2 → F3 → F1 → F4 → F5 → F6
                                          │
                                  99 端到端整合測試
```

> 規則：每個模組完成（含單元測試通過）後，將上方對應項目打勾，並同步把該模組檔案內所有子任務打勾。
