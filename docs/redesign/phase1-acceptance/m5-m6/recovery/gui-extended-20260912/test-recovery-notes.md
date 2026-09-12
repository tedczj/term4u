# 测试编译收敛记录（进行中）

当前推进提交前门禁，未 commit/push。

- 终端测试初始化改为当前本地模型；设置测试复用 `register_all_settings`，仍使用内存 preferences。
- MockTerminalManager 的测试构造器适配当前本地参数；保留原有终端测试辅助入口。
- 编辑器、查找、文件树及 code review 测试删除已移除云功能的初始化依赖；测试断言未放宽。
- code review 的 pending comment fixture 直接构造相同字段的本地类型，不再经过已删除 Agent SDK DTO。
- 删除只用于已移除 multi-agent 协议的 `test_util/ai_agent_tasks.rs` fixture helper；该文件没有测试用例。
- `cargo check -p warp --lib --tests --no-default-features --features local_only` 第一轮仍有 687 个编译错误。

未移除或忽略保留行为的测试。后续按原始编译诊断继续修复，完成后生成真实 test inventory。

## 2026-09-12 本地库测试实跑

- 已迁移终端 history/block/lifecycle、布局配置、设置导航、编辑器、命令搜索、文件窗格和本地控制测试。
- `deleted-feature-tests.json` 逐项记录 83 个仅覆盖已删除云功能的测试；`migrated-tests.json` 记录 10 个新旧测试 ID 对应。源文件级清单与可执行测试清单是不同证据。
- 本地 Markdown 文件的 undo-close/文件监听测试迁到当前 CodeView；验证同一文件在隐藏/恢复期间继续被跟踪，永久关闭后释放。不能据此声称 Markdown 渲染预览已恢复。
- 自动测试发现并修复：PaneGroup 没有接收同步焦点回调；PageUp/PageDown 没有注册为可编辑快捷键。窗口布局及原始终端字节的 SQLite 快照回归、损坏快照保护回归新增 2 项。
- 真实测试运行：**1597 passed / 0 failed / 3 原有 ignored**。未新增 ignore，未放宽本地行为断言。类型检查成功不能替代这项结果。
- 测试构建最初遇到 `rust-objcopy: No space left on device`；Cargo 虽返回成功，产物并非可执行文件，该两次构建不作为通过证据。清理本轮重复 DMG staging 后，使用 `cargo rustc -p warp --lib --profile test --no-default-features --features local_only -- -C strip=none` 生成真实 Mach-O 测试程序并运行。此参数只保留符号，不关闭测试或检查。
- 全 workspace 的严格 Clippy 仍失败：库目标 318 个、库测试目标 322 个错误，包含旧 integration harness 的云接口引用与遗留 unused/lint；尚未 commit/push，未宣称完整 V1 通过。
- 删除无调用方、仅用于云 embedding 同步和云 AI memory 的 integration helper 模块 `codebase_context`、`rules`；这些模块没有独立测试 ID，不删除本地文件/规则文件测试。

后续回归：分页按键冲突已用自动测试复现并修复；初始化前提交命令改为等待 Bootstrapped 后执行。完整本地库测试更新为 **1599/0/3**，GUI 慢启动验证已通过。新预提交日志为 `presubmit-current.log`，workspace Clippy 库目标 290 个、库测试目标 294 个错误，尚未完成完整验收。
