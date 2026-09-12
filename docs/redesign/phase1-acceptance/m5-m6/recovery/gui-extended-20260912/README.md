# 本地 GUI 与自动测试回归 · 2026-09-12

本次目标是让本机 GUI 可以启动并交互自测。**本地 GUI 已验证，但完整 R0–R6 / V1 与全仓预提交门禁尚未完成。本次按用户“先 commit & push”的明确要求保存并推送阶段结果，不表示全仓门禁通过。**

## 运行

当前应用：`target/debug/bundle/osx/Term4u.app`。构建、bundle 及 ad-hoc 签名已完成。

```bash
PATH="$HOME/.cargo/bin:$PATH" CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 ./script/run
```

本轮使用隔离的 `WARP_DATA_PROFILE` 和临时 ZDOTDIR 自测。未覆盖用户的 `.pi/goal-events.jsonl`，未清理用户数据库或历史迁移。

## 已执行的 GUI 验证

通过 Computer Use 操作真实应用并观察渲染画面：

- PTY 命令、Ctrl-C、cat/Ctrl-D、Vim 输入和保存、原生粘贴。
- 标签页切换、新窗口、分栏、调整分隔条、关闭分栏；焦点切换后的输出落在正确分栏。
- 字体大小、设置搜索及本地隐私状态；设置页重复打开不会创建重复窗格。
- 输出滚动、跨行/中文选择复制、输入复制粘贴、窗口缩放和原生全屏。
- 正常退出后恢复窗口、标签页、分栏、工作目录和终端块；活动标签页和活动窗口正确。
- PageUp/PageDown 从中间翻到开头、再翻到结尾。自动回归先复现编辑器抢占按键的问题，再验证终端收到正确动作。
- 用临时 `.zshrc` 延迟 15 秒初始化 shell；提前提交的命令在 shell 就绪后执行一次。Enter 在 08:53:34 UTC，Bootstrapped 在 08:53:42 UTC，屏幕显示一次 `QUEUEDBEFOREBOOT`。

屏幕观察记录位于本次 Computer Use 会话；`gui-current-artifact.json` 记录实际运行二进制的 SHA256 与检查范围。快键注入时必须在创建窗格后刷新应用状态，防止自动化在系统尚未处理焦点变化时把同一串文本注入两个窗口/窗格。

## 自动测试与数据

- `local-library-tests.log`：**1599 passed / 0 failed / 3 原有 ignored**。不等同于完整 workspace 测试通过。
- 新增真实回归：SQLite 窗口布局和任意终端字节往返、损坏快照保护、编辑器焦点下的分页、初始化前命令排队。
- 原有 3 个忽略测试涉及并发编辑选区和 workflow fuzzy-match 排序，未新增 ignore。
- `deleted-feature-tests.json`：逐项说明删除的 83 个旧云功能测试；`migrated-tests.json`：10 个新旧测试 ID 对应。其余修改保留本地断言，完整测试清单不能以失败的空清单代替。
- `legacy-historical-migration-comparison.json`：隔离旧 DB 与应用 69 个历史迁移后的 SQL 对照，46 张历史表行集合全部保留。
- `corrupt-snapshot-check.json`：损坏快照不阻止启动，退出后原内容仍保留。
- 新快照保证正常退出恢复；不能据此声称已实现旧窗口格式导入或崩溃恢复。

测试构建遭遇过磁盘不足：`rust-objcopy` 失败时 Cargo 仍返回成功，产物不可执行。这些尝试不作为通过证据。最终通过 `cargo rustc ... --profile test -- -C strip=none` 生成真实 Mach-O 测试程序并运行，未关闭测试、断言或编译诊断。详见 `local-library-test-result.json`。

## 仍未完成

- 全仓严格预提交检查：最近 workspace Clippy 库目标 290 个、库测试目标 294 个错误，涉及旧 integration harness 的已删除云接口与遗留 lint。默认 GUI Clippy 同样未通过（库目标 612 个、库测试目标 555 个错误）；completer 的严格 Clippy 通过。这些不是人工确认项，也没有标记通过。
- 完整 V1 的 TUI、Linux、网络归因、外链/Windows 清理、完整本地 workflow/notebook/LSP/日志功能验收仍按第 11 章执行。
- 多窗口恢复后，Computer Use 曾出现剩余窗口的鼠标定位失败；已验证恢复数据及键盘切换，仍不能把该鼠标路径记为通过。

早期 `checks.json`、`candidate-checks.json` 等保留各自执行时点的历史结果；本页和 `local-library-test-result.json` 给出最新本地回归边界。

剩余问题分类见 [Clippy 与集成测试待办](remaining-checks.md)。
