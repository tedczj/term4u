# Term4u 一期验收记录

本目录保存一期改造前基线与最终同一 HEAD 的逐项验收证据。原始命令输出不做摘要替代；最终证据在 `final/` 中生成后按 C1–C18 索引。

## 改造前冻结基线

冻结时间：2026-09-01T08:19:12Z（UTC）  
冻结 HEAD：`47110e95e5ffe9029c325bc013a59b657ada0251`

| 基线类别 | 证据 |
|---|---|
| HEAD 与工作区状态 | `baseline/repository-state.txt`。创建本目录前首次执行的 `git status --short` 输出为空；文件同时保存 tracked-worktree 的可重放命令结果。 |
| FIRST_REAL_USE | `baseline/repository-state.txt`，值为 `未发生`；权威标记仍在 `../baseline/README.md`。 |
| 测试清单 | `../baseline/phase1-before.txt`，由 `./script/test_inventory snapshot phase1-before` 生成，共 9,777 项。 |
| local_only Cargo tree | `baseline/cargo-tree-local-only.txt`，含命令、完整输出与退出码。 |
| cargo tree 黑名单命中 | `baseline/cargo-tree-blacklist.txt`，记录 MCP/rmcp、禁止 AWS SDK 与 Sentry 的改造前命中。 |
| 源码/manifest 黑名单 | `baseline/blacklist-scan.txt`，记录产品入口、MCP、AWS 与 Sentry 的改造前状态。 |
| 现有网络及构建链绕行 | `baseline/known-bypasses.txt`，记录裸 reqwest、socket/listener/DNS、字体 URL fallback 与构建链下载点。 |

## 最终验收索引

最终验证必须全部针对同一个 `git rev-parse HEAD`。每个日志需保存命令、UTC 时间、退出码及原始输出；尚未生成的证据不得标记为通过。

| 条件 | 最终证据 |
|---|---|
| C1 | 本文件与 `baseline/` |
| C2–C18 | 待最终实现和验证后写入 `final/manifest.md` |
