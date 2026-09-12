# R0 环境与基线刷新 · 2026-09-12

**状态：基线刷新完成，R0 环境准备仍 INCOMPLETE。** 当前仅支持 macOS；Linux 验证环境不再要求，
Linux 专属代码与配置后续清理。本机抓包权限和构建空间仍受阻，不能标成 R0 全部完成或 C1–C10 PASS。

本轮起点 `ee1308620c478643d63e865677598eb90d6ffc18`（main），UTC、命令及退出码见
[commands.json](commands.json)。仅更新文档及新证据目录，不覆盖旧 baseline/batches。
初始 `.pi/goal-events.jsonl` 的用户修改未纳入本轮提交。用户已授权本次 R0 commit/push，未请求 PR/tag。

## 当前环境与执行结果

- 本机 macOS 26.2 / arm64，Xcode 路径 `/Applications/Xcode.app/Contents/Developer`。
- Rust/Cargo 1.92.0 与 `rust-toolchain.toml` 一致；仅为本轮命令补入 `~/.cargo/bin`，未更改 shell 配置。
- 初次检测发现 nextest 缺失，按 `script/install_cargo_test_deps` 的安装命令执行
  `cargo binstall --secure --no-confirm --no-discover-github-token cargo-nextest`，退出 0；
  安装版本 0.9.144，[安装日志](nextest-install.log)与[版本实测](nextest-version.log)已保存。
  [environment.log](environment.log) 中 nextest 缺失描述的是安装前状态，组合命令的 exit 0 不代表每条子命令通过。
- 磁盘检测时约 491 MiB 可用；真实 inventory 构建因 `No space left on device` 失败，之后约 134 MiB。
  未清理用户文件或已有构建缓存。需先释放足够构建空间再运行全仓测试。

| 检查 | 结果 | 证据 |
|---|---|---|
| local_only library check | PASS，556 warnings | [原始日志](library-check.log) |
| 测试清单失败保护回归 | PASS，9 tests | [日志](inventory-regressions.log) |
| 真实测试清单生成 | FAIL，exit 101，磁盘不足；未生成或覆盖 baseline | [日志](inventory-real.log) |
| `./script/format --check` | PASS | [日志](format.log) |
| workspace Clippy | FAIL，lib 290 / lib test 294 errors | [日志](clippy-workspace.log) |
| 默认 GUI Clippy | FAIL，lib 612 / lib test 555 errors | [日志](clippy-gui.log) |
| completer Clippy | PASS | [日志](clippy-completer.log) |
| `./script/presubmit` | FAIL，停在 workspace Clippy，后续检查未执行 | [日志](presubmit.log) |
| origin/main 可达 | PASS，与开工 HEAD 一致 | [日志](remote.log) |

[diagnostic-groups.json](diagnostic-groups.json) 按诊断消息、文件路径分组。
当前 library 已无编译错误；主要 warning 为删除 feature 后的 cfg、未使用项和未满足 lint expectation。
严格 Clippy 另有遗留 lint 与旧测试/integration 接口错误；根因修复归 R1/R3/R5/R6，不能以 R0 基线记录代替修复。

## GUI、旧 DB 与本机网络条件

- GUI 的既有真实运行证据见 [gui-extended-20260912](../gui-extended-20260912/README.md)。
  本轮未重新操作 GUI，不把历史实机记录记成当前候选完整验收。
- 旧提交 `066ec71b`、`94912b78` 可读取，仓库 DB fixtures 已重新记录 SHA256，见
  [legacy-prerequisites.log](legacy-prerequisites.log)。已有 fixture/迁移对照可用；
  detached `066ec71b` 生成的真实旧样本仍无完成证据，保留为 R2.6 待做，不能以 fixture 替代。
- **网络测试环境确定为这台 macOS。** 实际执行 tcpdump 失败（exit 1）：BPF 权限被拒绝，
  `/dev/bpf*` 仅 root 可读，`sudo -n` 需要密码，见 [network-permission.log](network-permission.log)。
  `tcpdump -D` 能列接口不代表有抓包权限。未改系统防火墙或设备权限。
- 权限就绪后先抓受控 loopback TCP/UDP 流量，验证非空捕获及进程信息；再验证 DNS 53/853 的
  归因路径，无法归因则仍 INCOMPLETE。之后按第 11 章对隔离 HOME 的 GUI/TUI 各运行 60 秒、
  恶意代理、子进程及第二道防线拒绝日志执行正式测试。受控探针与产品静默窗口分别留证，
  不把空日志或权限错误当零外连。正式场景仍归 R6.9/R6.10。

## Linux 后续清理登记

Linux 不在产品支持范围，无需 runner 或 Linux PASS。后续独立清理以下残留并验证 macOS：

- Linux 专属源码与 `target_os = "linux"` 模块、依赖；共享实现及 macOS 条件分支不得误删。
- `script/linux/` 下 bootstrap、依赖安装、AppImage/deb/rpm/arch 打包与签名配置。
- Cargo target/build 配置、CI 中 Linux runner/job、发布资源与文档中的 Linux 支持声明。

本轮只确认范围并登记，不实施 Linux 代码或配置删除，不改写历史平台测试事实。
