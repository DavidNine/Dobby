# B5 — Sampler（背景排程）

> 對應設計：HLD §3 B5、§5(A)(B)
> 職責：協調 Collector 與 Storage 的背景常駐任務（取樣 + 清理）。
> 依賴：B3 Collector(trait)、B4 Storage(trait)、B2 Config。

## 完成定義 (DoD)
- 取樣與清理迴圈可運作。
- 單次失敗不中斷迴圈（失敗隔離）。
- 以注入的 Fake 依賴 + 可控時鐘完成測試。

## 子任務
- [x] 定義 `Sampler::new(collector, repo, config)`（依賴以泛型/trait object 注入）
- [x] 實作取樣迴圈：每 `sample_interval_secs` → `collect()` 成功則 `insert()`
- [x] 實作清理迴圈：每 `cleanup_interval_secs` → `delete_older_than(now - retention_days)`
- [x] 失敗隔離：collect / insert / delete 任一失敗 → 記 log、跳過該次、迴圈續行（不 panic）
- [x] 提供 `run()`：spawn 為 tokio 背景任務（兩個迴圈並行）
- [x] 時間來源可抽換（讓測試能加速/控制時鐘）

## 測試（注入 Fake Collector + Fake/in-memory Repo + 可控時鐘）
- [x] 固定時間內取樣被呼叫 N 次（次數符合間隔）
- [x] collect 失敗時不影響後續迴圈（後續仍正常取樣）
- [x] insert 失敗時迴圈續行
- [x] 清理用正確的 cutoff（`now - retention`）
