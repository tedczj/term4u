# 本机 GUI 启动与基本自测（已通过，非 V1 验收）

2026-09-12，在本机 macOS arm64 完成 Term4u GUI 构建、打包、实机操作及退出后重新启动。
基础提交 `2a18b733dc78daa846e36ee41392f234472c7534`；修复尚未提交。源码差异与产物身份见 [artifact.json](artifact.json)。

## 直接运行

从仓库根目录执行：

```bash
open target/debug/bundle/osx/Term4u.app
```

重新构建并运行（当前磁盘有限，沿用本轮已验证的构建参数）：

```bash
export PATH="$HOME/.cargo/bin:$PATH"
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=8 ./script/run
```

`./script/run --dont-open` 可只构建打包。macOS 开发脚本只生成 `.app`，不再额外生成 DMG。
本轮已生成的旧 DMG 目录移入用户废纸篓 `term4u-build-dmg-20260912-1320`，可恢复。

目前打开的应用使用隔离配置 `WARP_DATA_PROFILE=gui-selftest-launchservices-20260912`，
测试 shell 使用 `/tmp/term4u-gui-selftest-20260912/zsh` 作为 ZDOTDIR。
关闭后直接打开 `.app` 会使用正常 Term4u 配置；调试隔离配置不代表用户原有数据迁移已验收。

## 已修复

- 删除 repo_metadata 两处失效遥测调用、专用事件模块及因此无用的依赖，保留本地 warning。
- 去除 SyncedInputState、ActiveSession、ToastStack 重复注册；补回 LSP 管理器初始化。
- Ctrl-D 只在空命令输入框绑定 EOF，保留 PTY 快捷键校验；恢复编辑器 Ctrl-C 事件传递。
- 菜单只生成已注册动作的入口，收敛空分隔项；恢复本地设置、标签新增和切换的快捷键。
- 修复终端尺寸依据文本宽度收缩、输入框零高度、背景与标签对比度问题。
- shell 状态变化时把焦点交给终端或输入框；恢复交互命令的字符/控制字符转发。
- 激活、新增、关闭及移动标签时同步键盘焦点，避免键盘仍指向不可见的旧标签。

## 实机自测

以下通过 Computer Use 操作真实 GUI，观察实际输出；不是脚本直接向 PTY 注入命令。
最终主体自测对应 [runtime-11](term4u-gui-runtime-11.log)，重打包后的 macOS 打开验证对应
[launchservices-runtime](launchservices-runtime.log)。截图保存在本次会话的 Computer Use 工具记录中。

| 场景 | 操作与观察 | 结果 |
|---|---|---|
| 冷启动 | 隔离配置启动；窗口、输入框、zsh bootstrap 正常 | PASS |
| 命令输入与输出 | `echo FIRST` → `FIRST` | PASS |
| 新标签 | Cmd-T，新 shell 执行 `echo SECOND` → `SECOND` | PASS |
| 标签切换 | 点击第一标签，历史仍在；`echo SWITCHOK` 输出在第一标签 | PASS |
| 左右分栏 | Cmd-D；右栏执行 `echo RIGHT`，左右内容独立且宽度正常 | PASS |
| Ctrl-C | 运行 `sleep 120`，提前 Ctrl-C；显示 `^C`，随后 `echo INTERRUPTED` 正常 | PASS |
| 交互命令 | 运行 `cat`，键入 `HELLO` 并回车，观察输入及 cat 回显 | PASS |
| Ctrl-D | 结束 cat 后返回输入框；`echo READY` → `READY` | PASS |
| 正常退出 | Cmd-Q，SQLite writer、LSP、terminal server 正常关闭，进程退出 | PASS |
| 重新打开 | 用 macOS `open -n --env ...` 打开正式打包产物，`echo REOPENED` → `REOPENED` | PASS |

## 工程验证

所有 Cargo 构建使用 Rust 1.92.0、`CARGO_PROFILE_DEV_DEBUG=0`、`CARGO_INCREMENTAL=0`、`CARGO_BUILD_JOBS=8`。
本机 Xcode 26.6 (17F113)、Metal Toolchain 17F109；安装后清除 xcrun 工具发现缓存，真实 Metal 编译通过。

| 检查 | 结果与证据 |
|---|---|
| `cargo build --locked -p warp --bin term4u --no-default-features --features local_only` | exit 0，[构建日志](term4u-build-gui-11.log) |
| `./script/run --dont-open` | exit 0，[打包日志](term4u-bundle-final.log) |
| `./script/format`、`./script/format --check`、`git diff --check` | exit 0 |
| `cargo clippy -p repo_metadata --lib --features local_fs -- -D warnings` | exit 0，[日志](term4u-repo-metadata-clippy-clt.log)；使用已装 CLT，修复源码后续未变 |
| 网络边界扫描及 self-test、内联测试模块检查、许可证配置同步 | exit 0 |
| `codesign --verify --deep --strict` | exit 0，本地 ad-hoc 签名 |

未运行全 workspace 测试、GUI integration harness 或全项目严格 Clippy。
当前 app 构建仍报告 744 个既有警告；不把本轮基本操作自测当作完整工程门禁通过。

## 历史失败与范围边界

初始构建缺 Metal，用户安装 Xcode 并完成首次许可后解决。旧日志保留当时失败，不代表最终产物失败。
运行时先后复现重复 singleton、LSP 未注册、Ctrl-D 绑定校验、旧菜单动作缺失、布局压缩和焦点问题；
对应失败日志保留在本目录，最终以 runtime-11 与重新打开记录为准。

已知窗口/标签/分栏重启恢复仍未修复：SQLite Snapshot 仍为空操作，读取的 app_state 仍为 None。
旧数据迁移、完整本地功能、Linux/TUI、零网络正式验收、M7 品牌发布及 MIT 路线均未在本轮关闭。
既有 `.pi/goal-events.jsonl` 修改保留；未 commit/push。
