# R1/R2 收敛验收 · 2026-09-13

R1/R2 已完成实现与验收。本记录随收敛提交交付；R3–R6 和完整 V1 验收不在本次完成声明内。
范围为 macOS。基线为 `389b1946`，候选源码逐文件身份见 `candidate-source.json`。

用户明确要求完成 R1/R2、清理严格 Clippy 后 commit & push，并整文件删除旧云 integration
及本地接口迁移相关测试；覆盖不足随实现补充。该要求替代旧清单中的逐套件迁移安排。
已有可用本地测试继续保留。`.pi/goal-events.jsonl` 的用户修改不属于本次提交。

## 实现

- 删除旧 integration crate、测试入口、hooks、CI 配置，以及失去消费者的云/Agent 模块、
  类型、菜单入口、遥测计算和辅助方法。15 个历史 SQLite fixture 原字节迁到
  `crates/persistence/fixtures/legacy`，没有删除历史 migrations、表或旧原始数据。
- 恢复本地 Workspace、PTY、目录/会话状态通知、文件树、文件/全文搜索、代码编辑保存、
  本地 diff 与 CodeReview pane 恢复。接回终端查找、全屏终端鼠标/滚轮转发。
- 手动关闭标签页可在原位置恢复；分栏 shell 退出只关闭自身。窗口关闭和恢复同步注册表，
  未保存编辑支持取消关闭。每个标签页的左面板模式和宽度随旧快照往返。
- 本地及项目 workflow 加载、参数默认值/覆盖值、输入/执行接通；缺必填参数时留在输入框。
  workflow 编辑保留参数、描述等字段。env 值解析和 shell 展开保留。
- 旧 SQLite/JSON notebook 可打开、编辑、换行、退出重开，本地新版本不会被旧行反向覆盖。
  Workspace/LSP metadata 恢复与写回保留已有时间戳和语言服务器偏好。
- Workspace 使用既有 toast 组件显示 LSP 手工安装提示、设置错误及代码审查 Undo。
  Help 的 Export Logs 在后台生成本地 ZIP 并在 Finder 显示；调试构建提供 panic 日志测试。

## 工程检查

| 检查 | 结果 | 原始记录 |
|---|---|---|
| §4.1 library/all-targets/core/TUI check | 全部退出 0 | `check-*-01.json`、`check-local-targets-02.json` |
| local_only + test-util 完整 app 测试 | 1512 passed，3 原有 skipped | `test-local-all-02.log` |
| TUI/ai/persistence/warp_terminal/LSP focused | 703 passed，2 原有 skipped | `test-focused-all-02.log` |
| workspace、GUI、completer 严格 Clippy | 全部通过 `-D warnings` | `presubmit-07.log` |
| 完整 presubmit | format、边界检查、clang-format、WGSL、测试、doc tests 全过 | `presubmit-07.json` |
| workspace nextest | 4752 passed，20 原有 skipped | `presubmit-07.log` |
| completer v2 | 131 passed，4 原有 skipped | `presubmit-07.log` |
| 测试清单与历史 fixture | 清单核对通过；15 个 SHA 不变、integrity_check 全部 ok | `test-inventory-verify-01.log`、`fixture-integrity.json` |

缺少的 clang-format 和仓库指定版本 wgslfmt 只安装在 `/tmp` 的隔离工具目录，未修改系统安装。
早期 presubmit 的缺工具、ENOSPC 和收尾 lint 失败分别保留，没有改写成通过。
上游 command-signatures-v2 构建仍提示使用已有 JS 签名资源；本次没有修改该上游生成器。

## 实机与数据

| 场景 | 证据 |
|---|---|
| GUI PTY、Ctrl-C、外部 CLI、文件编辑保存、文件树、搜索、workflow、Find、diff 与恢复 | `gui-workspace/` 的阶段截图、输出 JSON、最终构建截图 |
| 标签页/窗口 Undo、split shell exit、未保存 Cancel、diff Undo | `undo-*`、`window-undo-*`、`split-exit-*`、`unsaved-close-*`、`final-undo-*` |
| 五种 LSP 的 PATH 成功/缺失/不可运行分支 | `supported_servers_tests.rs` 中五种类型矩阵；实际 clangd 初始化和四种缺失提示见 `gui-workspace/lsp-result.json` |
| 正常日志、debug panic、本地 UI ZIP | `gui-workspace/logs-and-split-result.json`；ZIP CRC 检查通过，未上传 |
| Notebook 旧正文、真实 Enter、编辑、退出、重启 | `gui-notebooks/final-result.json` 及 `final-*.png` |
| 最终 TUI 真实 PTY | `tui-live-attempt-04/`，输出/Ctrl-C/标签页/外部 CLI/正常退出全部通过 |
| bundled/local skills | `bundled-skills-integrity.json`：8 个包内文件与仓库源逐字节一致；本地读取/解析测试通过，无自动更新 |

指定旧提交 `066ec71b736fc3755e29f58f733deadbdac3d1af` 的原始 OSS 程序已真实运行，
生成 2 个标签页、3 个 terminal pane、3 条命令的隔离数据库。当前版从其副本迁移，新增命令后
退出重启，并从历史搜索找到新命令。旧行、设置与原始 SQLite SHA 保持不变，详见
`oracle-066ec71b/migration-comparison.json`。旧版手工修改的标题未持久化，因此没有将它
计作旧自定义标题运行时覆盖。旧源码 worktree 已移除，原始样本和失败尝试记录保留。

用户已确认“实际窗口文字完整”。CUA 增量截图偶有缺字，按捕获差异记录；Metal 探针和实验
全部撤回，没有据此修改渲染器。最终 Notebook 正文、diff 和提示均有可见内容截图。

## 测试删除审计

历史基线 9777 个 ID，当前 4772 个，累计消失 5089 个、新增 84 个。其中 1250 个消失 ID
此前已登记，本次补齐剩余清单。`test-removal-audit.json` 逐项列出 ID、分类与对应文件记录；
`removed-*.json` 保留整文件和函数删除依据。累计差异不等于这些测试都在本次删除，
旧测试消失也不等于已逐项迁移覆盖。

当前结论以 `current-checkpoint.json` 和上述最终日志为准。早期日志、失败截图与探针记录仅为
历史诊断，不代表仍有同一故障，也不替代后续 R3–R6 的验收。
