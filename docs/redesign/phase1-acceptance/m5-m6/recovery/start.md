# R0 · 开工记录（进行中，不是验收 PASS）

- 开工 UTC：2026-09-05T16:34:05Z。
- 开工 HEAD：`dc768549c8ba15a74e0658f44c714da0a5f3b05d`，分支 `main`。
- 唯一施工依据：[11 · 本地化收敛施工单](../../../11-本地化收敛施工单.md)，R0–R6 / M5–M6；不包含 M7/MIT。
- 初始用户改动：[status](initial-status.log)、[未暂存 patch](initial-diff.log)、[暂存 patch](initial-index-diff.log)。保留全部初始修改和 `.pi/`，不以清理工作树为由覆盖或删除。
- 各命令 UTC/命令/退出码：[initial-commands.json](initial-commands.json)，对应完整原始输出见同名 `.log`。
- `git diff --check` 初始退出 0；`rustc --version`、`cargo --version`、`cargo nextest --version` 退出 127。
- [首次 library check](library-check-initial.log) 退出 127（`cargo: command not found`），没有产生编译诊断，当前编译错误数量未知。
- 仓库固定工具链：`rust-toolchain.toml` 的 Rust 1.92.0，minimal + rustfmt/clippy/rust-analyzer。未安装/升级工具链或系统依赖，未更改全局环境。
- [环境](environment-discovery.log)：macOS arm64 / Darwin 25.2.0，CommandLineTools 存在；常规 PATH、`~/.cargo`、`~/.rustup`、Homebrew Rust/Cargo 位置未找到工具；`oz-dev`、tmux 不可用，tcpdump 命令存在但尚未验证捕获权限。
- [磁盘](disk.log)：约 11 GiB 可用，不能假定足够完成 debug/release/workspace 全套构建。未删除用户缓存或使用备份卷。
- [历史提交可读取](historical-objects.log)：`066ec71b736fc3755e29f58f733deadbdac3d1af` 和 `94912b78` 均为 commit。真实旧样本尚未生成，fixture 不能代替其整体验收。
- 仓库 DB fixture 存在（`crates/integration/tests/data/*.sqlite`）；基线只做 schema/计数/哈希检查。[fixture-baseline.json](fixture-baseline.json) 和 [migration-baseline-sha256.txt](migration-baseline-sha256.txt) 已归档。
- 首次 `mode=ro` 检视仍让 SQLite 在四个 WAL-mode fixture 旁创建了空 WAL/SHM。立即核对全部原始 DB 哈希未变，只移除本轮新建 sidecar，并在临时目录的副本上重做全部 15 个 schema/计数/完整性检查；结果一致，见 [隔离重检](fixture-isolation-recheck.log)。后续所有 DB 操作均使用隔离副本。

## 当前可执行修复顺序（静态清点，不是编译诊断）

1. `app/src/persistence/block_list.rs`：仍导入删除的 Agent blocklist，需改用现有 terminal 本地恢复类型；删除 Agent 查询执行/写入，保留原表及原始行。
2. `app/src/persistence/sqlite.rs`：仍引用 cloud models/provider、旧 ModelEvent、旧 snapshot 字段。需保住历史/本地 notebook/workflow/窗口恢复，不能用整体跳过加载或清表替代。
3. `app/src/lib.rs` 初始化 / Settings / search：去除删除模块残留，接好本地 stores。
4. terminal / workspace / pane / root：同步本地 snapshot 和生命周期，禁止新增 terminal model 锁。
5. tests / bin / integration / feature unification：取得真正 Cargo 诊断后逐项追踪；测试清单失败必须记 FAIL，不生成空基线。

## Inventory 失败保护

`script/test_inventory` 保留 Cargo stderr 和退出码；JSON 必须含非空实际测试；snapshot 成功后才原子替换，失败不新建或覆盖旧清单。
新增 `script/lib/test_inventory_tests.py`，并接入 presubmit clippy 前，原有门禁均保留。
[9 项回归](inventory-failure-regressions.log) exit 0，[shell syntax](inventory-shell-syntax.log) exit 0；[真实 inventory 重跑](inventory-after-fix.log) exit 127，明确显示 Cargo 缺失，不算测试全集通过。

## 外部前提和授权（未完成）

已向负责人请求固定 Rust 工具链及验证工具安装授权、构建空间和 Linux runner 访问方式；未获得授权之前不安装、不修改系统。
GUI 实机、可归因 DNS/进程捕获、防火墙第二道防线和 Linux 环境尚未验证可用。
最终 SOURCE_HEAD 需单独提交授权；远端可达性如需推送另行授权。当前未授权 commit/push/PR/tag/防火墙变更，均未执行。
环境缺失不等同于验收通过；继续静态修复和可运行的便宜检查。

## 索引

[逐项施工矩阵](requirements-matrix.md)；最终验收清单将建立于 `../final/manifest.md`，在同一获授权候选上实际执行后才可标 PASS。
