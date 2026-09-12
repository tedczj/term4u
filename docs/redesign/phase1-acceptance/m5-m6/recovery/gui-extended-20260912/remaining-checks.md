# Clippy 与旧集成测试剩余项

基于本目录 `presubmit-current.log`、`default-gui-clippy-current.log` 的已执行结果。本次用户已知检查未通过，明确要求先 commit & push；此提交保存本地 GUI 修复及测试迁移成果，不能作为完整 V1 验收。

## 检查范围和计数

| 检查 | 结果 | 含义 |
|---|---|---|
| 本地 GUI 库测试 | 1599 通过、0 失败、3 原有忽略 | 已编译并实际执行 |
| 默认 GUI 严格 Clippy | lib 612、lib test 555 个错误 | 当前日志没有 Rust 类型/导入编译错误；主要是将 warning 按 error 处理的 lint |
| workspace 严格 Clippy | lib 290、lib test 294 个错误 | workspace feature 合并编译了旧 integration helper，包含类型编译错误及 lint |
| completer 严格 Clippy | 通过 | 独立按默认 features 执行 |

计数来自 Rust 编译器对不同目标的诊断，同一根因会在多个目标重复或引发连锁错误，不能相加当作独立待办或失败测试数量。旧集成测试目前主要卡在**编译阶段**，尚未获得完整运行结果。

## Clippy 类别

1. **删除功能后的遗留代码**：unused imports、unused variables、未使用函数/类型/枚举分支/字段。典型位置是 `settings_view/appearance_page.rs`、`pane_group`、`server/telemetry/events.rs`、`settings_view/settings_page.rs`、`tips/tip_view.rs`、`util/link_detection.rs` 和一些旧 UI 组件。需要区分云功能残留与尚未接通的本地功能，不能统一删除或 blanket allow。
2. **注释/属性残留**：删除函数后留下的 doc comment、空行和 cfg 属性；出现 empty-lines-after-doc-comments/outer-attributes。
3. **feature 与 lint 属性过期**：`warp_managed_secrets` 已不在当前 feature 定义中；`projects.rs` 中的 lint expectation 不再命中。
4. **普通 Rust/Clippy 问题**：大枚举分支、`&PathBuf` 参数、只 await 一次的 async block、重复 if 分支、可化简的循环/match、Default 初始化写法等。需逐条判断并进行最小修复。

## 旧 integration helper 类别

| 文件/目录（均在 `app/src/integration_testing`） | 仍使用的旧接口 | 处理方向 |
|---|---|---|
| `assertions.rs` | CloudModel、UserWorkspaces、Listener、UpdateManager、旧 server IDs；团队/云对象/WebSocket 状态 | 删除纯云场景及其调用者，保留本地断言 |
| `notebook/{step,assertion}.rs` | SyncId、CloudNotebookModel、云 revision/preferences、已移除的 rich editor 接口 | 将 notebook 创建/打开/编辑/恢复断言接到本地 NotebookStore/NotebookView；不能丢本地内容与渲染要求 |
| `workflow/{step,assertion}.rs` | CloudWorkflowModel、云创建/分享、旧 WorkflowOpenSource 参数、团队工作流状态 | 改为本地 workflow 身份/内容/文件与窗口打开路径；保留本地/项目工作流执行测试 |
| `input/{step,assertions}.rs` | AgentInputFooter、InputConfig/InputType、CLI Agent sessions、旧输入建议/context menu 接口 | 删除已移除 Agent 场景，迁移本地输入/补全/粘贴相关测试 |
| `terminal/{step,assertion}.rs`、`block/assertions.rs` | 旧 viewport、BlockVisibilityMode、TerminalViewState、旧方法/参数 | 迁移到当前 terminal/model/scroll 实现，保留 PTY、块、选择、滚动和生命周期断言 |
| `view_getters.rs` 等公共 helper | 已删除 AI panel/InputSuggestions 以及旧窗格取值方式 | 删除云 getter，更新本地 view getter，并同步 `crates/integration` 的调用方 |

`crates/integration/src/test.rs` 以及 `test/input.rs`、`test/workflows.rs` 等调用这些 helper 的场景，也要随接口迁移重新编译和执行。不能仅删除报错 helper 让检查变绿。

本次已移除无调用方的云 embedding 同步与云 AI memory helper（`codebase_context`、`rules`）；已迁移的库测试另有 `deleted-feature-tests.json` 和 `migrated-tests.json`。这些是已完成项，不应重新计入待办。

## 建议施工顺序

1. 修复 workspace 集成测试编译链，逐项对照保留行为，记录云场景删除或本地测试新 ID。
2. 清理过期 imports/变量/cfg/注释属性；审计 dead code 是否需要接通本地调用。
3. 跑 workspace 与默认 GUI 两种严格 Clippy，再跑全仓 nextest/doc tests 和保留的 GUI 集成场景。

TUI/Linux 实机、网络归因和完整 V1 验收是第 11 章的后续验证面，不应与这些 Clippy 编译错误混成一项。
