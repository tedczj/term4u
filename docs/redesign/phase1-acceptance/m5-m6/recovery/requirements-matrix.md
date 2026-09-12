# R0–R6 逐项矩阵

原文：[施工单](../../../11-本地化收敛施工单.md)。以下仅为导航；`待验`/`进行中` 不代表 PASS。
施工单 C1–C10 与 Goal C1–C6 是两套 ID；此表验收列使用施工单 ID。

| 项 | 要求 | 状态 | 实现/验证/证据 | 验收 |
|---|---|---|---|---|
| R0.1 | HEAD/UTC/工作树/工具链/磁盘/PATH | 基线已刷新；空间已恢复 | [最新记录](r0-20260912/README.md)：Rust/Cargo 1.92.0，nextest 已补齐；真实 inventory 曾遇磁盘不足；[后续缓存清理](network-20260912/cache-cleanup.json)已恢复空间，清单尚未重跑 | C10 |
| R0.2 | library check 原始日志及根因分组 | PASS（仅 library check） | [新日志](r0-20260912/library-check.log) exit 0；[诊断分组](r0-20260912/diagnostic-groups.json)，全仓 Clippy 仍失败 | C1 |
| R0.3 | recovery 新记录，不覆盖历史 | 完成 | [新目录](r0-20260912/README.md)，保留历史 baseline/batches | C10 |
| R0.4 | 清单生成失败不当空清单 | 失败保护 PASS；真实清单 FAIL | [9 项回归](r0-20260912/inventory-regressions.log)；[真实失败](r0-20260912/inventory-real.log) exit 101，不覆盖 baseline | C7 |
| R0.5 | GUI/旧 DB/本机 macOS 网络/授权前提 | INCOMPLETE | [当前条件](r0-20260912/README.md)：本次 commit/push 已授权；[管理员受控抓包通过](r0-capture-20260912/README.md)，空间已恢复；[DNS 已校准](network-20260912/README.md)，TCC 子进程覆盖/正式收尾仍待落实；真实旧样本归 R2.6；Linux 不再要求 | C3/C9/C10 |
| R1.1 | 持久化本地类型与 terminal opaque 旧行 | 进行中 | block_list/sqlite/model；需补回归 | C1/C3 |
| R1.2 | notebook/workflow/文件初始化及恢复 | 待验 | app lib/local_objects/notebooks/workflows | C1/C3 |
| R1.3 | Settings/menu/search 去死引用 | 待验 | app settings/settings_view/search/app_menus | C1/C4 |
| R1.4 | terminal 生命周期/锁不增加 | 待验 | app terminal | C1/C2 |
| R1.5 | workspace/pane/root/窗口恢复 | 待验 | app workspace/root_view/app_state | C1/C2 |
| R1.6 | ai/graphql/server 保留项消费者逐项说明 | 待验 | 全树消费关系审计 | C6 |
| R1.7 | tests/bin/integration/feature unification | 待验 | §4.1 全部 check，GUI/TUI 实际启动 | C1 |
| R2.1 | PTY/shell/输出/Ctrl-C/退出/外部 CLI/tab/split/重启 | 待验 | unit/integration + GUI/TUI 实机 | C2 |
| R2.2 | 文件树/搜索/打开/编辑保存/code review/diff | 待验 | unit/integration + GUI 实机 | C2 |
| R2.3 | local/project workflow 字段及加载执行兼容 | 待验 | 历史 oracle/fixture + `.warp/workflows` 实际执行 | C3 |
| R2.4 | notebook SQLite/JSON/编辑/重启/覆盖优先级 | 待验 | store/manager/editor 单测及实机 | C3 |
| R2.5 | env-var 值解析/shell 展开；去 collection 云面 | 待验 | 新旧字段对照及 shell 验证 | C3 |
| R2.6 | 旧 DB 迁移/历史设置窗口/写入重启/损坏隔离 | 待验 | 仓库 fixtures + 066ec71b 真样本或明确批准替代 | C3 |
| R2.7 | 未知与缺省字段/旧正文与列不丢 | 待验 | 独立测试与迁移 diff | C3 |
| R2.8 | 正常日志/debug panic/UI zip，无上传且 warning 无正文密钥 | 待验 | 三场景实际日志/zip | C5 |
| R2.9 | 五 LSP PATH 命中/缺失/手工提示/零下载 | 待验 | fake PATH server 参数，curl/wget/npx/npm 零日志 | C5 |
| R2.10 | bundled/local skills 可读且不更新 | 待验 | 资源清单/实机/下载脚本审计 | C5 |
| R2.11 | 历史 oracle 仅隔离 worktree，不恢复产品依赖 | 待验 | oracle/fixture 来源和命令 | C3/C6 |
| R3.1 | Settings/Palette/menu/actions/URI 云入口全清 | 待验 | 列举所有入口、测试与截图 | C4 |
| R3.2 | Privacy 固定 LocalPrivacyPolicy；本地设置保留 | 待验 | filter tests/GUI | C4 |
| R3.3 | 五云搜索词无结果，本地搜索仍有效，无幽灵入口 | 待验 | Settings 实机与单测 | C4 |
| R3.4 | 无云用户凭据，不读迁移旧 keyring/env/token | 待验 | auth 调用链/环境/身份审计 | C4 |
| R3.5 | TUI zero state/transcript/tab；slash 普通 shell | 待验 | 固定宽度 render/快捷键注销/真实交互屏幕 | C4 |
| R4.1 | 内建产品外链清理，用户链接仍可打开 | 待验 | open_url/URL 审计 | C5 |
| R4.2 | external checker/拒绝允许样例/self-test/扫描失败关闸 | 待验 | script + presubmit clippy 前接入 | C5/C7 |
| R4.3 | 每条外链路径/触发/理由 allowlist | 待验 | final/external-link-allowlist.txt | C5 |
| R4.4 | Windows 文件/模块/纯 target/build dep 清理 | 待验 | 最终全文件/target block 审计 | C5 |
| R4.5 | Windows 仅两个工具文件与历史 migration 例外；混合依赖保留 | 待验 | manifest/file diff；无用 embed-resource/dunce 清理 | C5 |
| R5.1 | workspace/member/dep/feature/patch/dev dep 闭包 | 待验 | metadata/GUI+TUI normal tree/源码/lock | C6 |
| R5.2 | TUI normal/dev warp 显式关闭 default、local_only/tui/test-util | 待验 | manifest + 实际 unified builds | C1/C6 |
| R5.3 | email_address 精确 rev 和 lock 一致 | 待验 | b3e5205b4efdc83832f230d8eb6894fd00a9ec32 | C6 |
| R5.4 | deny allow-org 删除，保留 git 全显式与许可证依据 | 待验 | deny/about/NOTICES 输入/cargo deny | C6 |
| R5.5 | 全 WIP 消失 ID 精确全集及逐项删除/迁移说明 | 待验 | 与 baseline/phase1-before.txt 实际清单对照 | C7 |
| R5.6 | 逐批删除集及累计 script/deletion_set.txt | 待验 | M5/M6 全 diff 审计 | C7 |
| R6.1 | 获授权冻结 SOURCE_HEAD，改输入即重新验证 | 待授权 | 未 commit/push，不提前冻结 | C10 |
| R6.2 | §4.1 所有 check/focused nextest/inventory/network | 待验 | 原始命令/UTC/exit/log，不能历史拼接 | C1/C7 |
| R6.3 | §4.2 全部 format/clippy/presubmit/workspace/doc/deny/F7–F9/build | 待验 | 全量原始日志，presubmit 不削弱 | C1/C6/C7 |
| R6.4 | §4.3 metadata/tree/源码/lock/结构负证明 | 待验 | 每命令 rc，rg 无匹配与失败分开 | C6 |
| R6.5 | auth/UI/Windows/外链/features/下载/历史 migration 审计 | 待验 | migrations 与 94912b78 及开工 HEAD 比较修改/删除 | C3/C4/C5 |
| R6.6 | GUI/TUI release strings + 实际 bundle/resources + SHA256 | 待验 | 假阳性仅负责人批准精确项 | C8 |
| R6.7 | GUI S1–S13 及所有 R2 流程 | 待验 | 操作/观察/截图；含 debug Rust panic、十分钟空闲 | C2/C3/C5/C8 |
| R6.8 | TUI 真实 terminal 启动/输出/Ctrl-C/tab/新 tab/退出 | 待验 | run-tui + tmux 或 PTY 屏幕，不以 GUI harness 代替 | C2/C4 |
| R6.9 | 隔离 HOME/恶意代理下候选 GUI/TUI 各 60s 零网络 | 待验 | TCP/UDP/WS/可归因 DNS53/853/子进程/重试/待发文件 | C8 |
| R6.10 | 第二道防火墙拒绝日志为空；规则先授权 | 待授权/待验 | 不以守卫拒绝当零请求，用户主动命令另测 | C8 |
| R6.11 | macOS arm64 全门禁/实机/DB/网络/LSP/bundle/签名/scheme/身份 | 待验 | final/macos.md | C9 |
| R6.12 | Linux 验证 | 不适用（2026-09-12 用户决定） | 仅支持 macOS；Linux 代码/依赖/构建/CI 后续清理，见 [登记](r0-20260912/README.md#linux-后续清理登记)，不记 Linux PASS | C9 |
| R6.13 | final C1–C10 全索引/源码证据关系/diff/status 审计 | 待验 | final/manifest.md 与 repository-state.txt | C10 |
| R6.14 | V0 索引/08 章/risk-log 同步；不宣称 M7/MIT 完成 | 待验 | phase1-acceptance/final/manifest.md 引用本批，不复制日志 | C10 |
