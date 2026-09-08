# Term4u 一期验收记录

本目录保存一期改造前基线与最终验收证据。当前施工与候选源码/证据提交规则见
[11 · 本地化收敛施工单](../11-本地化收敛施工单.md#delivery)。原始输出不以摘要替代。
历史一期 C1–C18 与本次收敛 C1–C10 是不同编号体系，不应混用。

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

最终验证针对同一候选 `SOURCE_HEAD`；归档提交仅可新增证据/状态文档，不改变验证输入。
每个日志保存命令、UTC、退出码及原始输出，未生成的证据不得标记为通过。
当前收敛使用 `m5-m6/final/manifest.md` 按 11 章 C1–C10 验收；根 `final/manifest.md`
负责映射 V0 遗留验收与历史一期条件到实际证据，不复制另一套日志。

| 条件 | 最终证据 |
|---|---|
| C1 | 本文件与 `baseline/` |
| C2–C18 | 待最终实现和验证后写入 `final/manifest.md` |
