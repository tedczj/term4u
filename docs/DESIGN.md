# Term4u 完整设计与实施基准

> 唯一设计、状态与施工依据。设计整理日期：2026-09-14（UTC）。
> 实现评估基线：`eef716cd69b84e0676eb90ad3ee776397aa6b515`。
> 本文的“最终设计”指唯一采用的目标方案，不表示产品已完成；当前 V1 尚未验收关闭。

## 导航

- [1. 目标与边界](#goals)
- [2. 最终架构与模块契约](#architecture)
- [3. 数据、身份与资源](#data)
- [4. 依赖与许可证边界](#dependencies)
- [5. 唯一完成状态表](#status)
- [6. 剩余施工步骤](#implementation)
- [7. 完整验证程序](#verification)
- [8. 验收条件 C1–C10](#acceptance)
- [9. 证据、文档与变更纪律](#evidence)
- [10. V2 发布与后续演进](#release)

<a id="goals"></a>
## 1. 目标与边界

Term4u 是 Warp 客户端的本地化衍生产品。上游来源为 `warpdotdev/warp`，基线为
`066ec71b736fc3755e29f58f733deadbdac3d1af`；项目仓库为 `tedczj/term4u`。
保留本地终端、分栏、文件树、预览、编辑与本地数据能力；删除云控制面，不能只关闭开关。

| ID | 产品必须满足的条件 |
|---|---|
| H1 | 产品构建不包含遥测传输、崩溃上传、云 Agent、服务端认证、Drive 同步、共享、更新等已删除实现 |
| H2 | 启动、恢复、空闲和退出均不创建 Warp Server HTTP/SSE/WebSocket 客户端或后台云重试任务 |
| H3 | 产品网络出口只允许 loopback；非 loopback 请求在统一守卫中硬拒绝，不进行外部 DNS 解析 |
| H4 | 将来新增的产品网络调用也必须经过守卫；构造器检查、依赖/产物扫描和运行观测共同防止回归 |
| H5 | 用户主动运行的 shell、git、ssh、CLI agent 是独立 PTY 子进程；Term4u 不替用户限制其网络 |

Term4u **不是沙箱**。静默运行的产品零外连指标与用户主动执行联网命令分开验证。
构建工具下载依赖也不等于产品运行时联网，但不得恢复运行/构建脚本自动拉取并执行 skills 的链路。

产品只支持 **macOS**，必验目标是 **macOS arm64**。不承诺 Linux、Windows 或 WASM 产品。
Linux 专属源码、依赖和 CI/打包配置后续单独清理；当前不得以取消 Linux 验证冒充 Linux PASS。
Windows 专属实现和依赖属于本轮收尾，保留共享路径处理例外及历史迁移。

唯一采用**路线 A**：物理删除 Warp Agent UI/SDK/执行层和云协议，不保留 ServerApi/OfflineApi
双后端、假 URL、空 token、同名空壳或静默重试。阶段内不开发新的本地 Agent 协议。

交付含义固定：V0 是本地可用；V1 是本地功能、云清理及 C1–C10 完整验收；V2 是品牌与发布。
完整 MIT 重实现是独立后续战略，不等于 V2，也不自动纳入本轮施工。

<a id="architecture"></a>
## 2. 最终架构与模块契约

```text
GUI: app/ (term4u)                 TUI: crates/warp_tui (term4u-tui)
        │                                  │
        ├── workspace / pane / settings ────┤
        ├── terminal / editor / file search / local diff
        ├── workflows / notebooks / local_objects / env_vars
        └── 本地持久化、PATH LSP、本地日志、bundled skills
                         │
          warp_core / warpui / warpui_core / persistence / warp_terminal
                         │
       本地文件与 SQLite / macOS / PTY / 受守卫保护的 loopback 通信

用户主动命令 ── PTY 子进程 ── 用户自行控制的网络（不属于产品云后端）
```

### 2.1 前端和公共层

保留 `app/` 与 `crates/` 工作区结构及内部 `warp_*` 名称，不全仓重命名。
GUI 使用 WarpUI 的 Element/View、GPU/WGSL 和 macOS `.app`；TUI 使用 cell-grid/TuiElement，
不依赖 GUI 截图验证。两者共享本地实体、终端与持久化能力，而不是共享一个云账户后台。

实体由 App/Entity/Handle 管理；View 使用已有 context、action 和锁传递。
不得在终端调用链新增重入 `model.lock()`；跨异步更新确认实体仍有效，不能留下死锁或幽灵操作。
GUI 鼠标状态在构造阶段持有，不能每次 render 创建新状态导致点击失效。

### 2.2 启动和恢复

入口先确定 Term4u 身份和离线 feature，再安装公共本地能力、持久化及前端所需 manager。
读取设置、终端历史、窗口/pane 快照和本地对象后恢复 UI；不可跳过整个恢复流程换取启动成功。
旧云对象保留为原始数据，但不再创建 cloud singleton、认证客户端、同步队列或 Agent pane。
退出时保存本地编辑和窗口状态，停止产品工作任务，不生成待上传文件。

### 2.3 保留能力的可观察契约

| 模块/入口 | 必须保留的行为与失败处理 |
|---|---|
| `terminal/`、`workspace/`、pane/root | PTY/shell、输出、Ctrl-C、退出、tab/split/window 恢复；split shell 退出只关闭自身；关闭/Undo 注册表一致 |
| 终端输入与显示 | Tab 唯一补全/公共前缀/候选循环；保留光标后文字并丢弃过期异步结果；目录/Git/PS1 提示；clear 清可见区域但保留回滚历史；点击焦点与拖选互不破坏 |
| editor/file tree/search/code review | 打开、全文搜索、编辑、保存、本地 diff、code review 和 Undo；未保存关闭允许取消，不能悄悄丢内容 |
| `workflows/` | 本地及项目 `.warp/workflows` 加载、编辑、参数默认值/覆盖值、输入/执行；必填参数缺失留在输入框；保留名称/描述/参数字段 |
| `notebooks/`、`local_objects/` | 旧 SQLite/JSON 加载、编辑、真实换行、保存、退出重开；本地新版本优先，不能被旧行反向覆盖 |
| `env_vars/` | 保留本地值解析和 shell 展开；不保留 Drive collection、权限或分享语义 |
| Settings/menu/URI | 保留 Appearance、Features、Keybindings 等本地设置及搜索；删除账户、团队、计费、推荐、Platform/API key、Environments、Agent Profiles、Drive/共享/cloud run/handoff/升级入口 |
| Privacy/auth | Privacy 展示 LocalPrivacyPolicy 的固定本地值；不提供可开启云传输的开关；本地 auth 无云用户/凭据；旧解析所需值类型保持中立 |
| TUI | 本地 zero state/transcript/tab、输出/Ctrl-C/快捷键/退出；`/` 是 shell 字符，不增加 slash 云菜单 |
| LSP | 现有五种 server 只从 PATH 查找；命中、缺失、不可执行分别处理；缺失只提示手工安装，不查询版本或下载安装 |
| 日志与 skills | 正常日志、debug Rust panic 日志、本地 UI ZIP 导出；bundled/local skills 只读本地文件，不上传或自动更新 |

### 2.4 删除面和网络出口

删除 Drive/cloud object、云 Agent/blocklist、GraphQL/ServerApi/OfflineApi、Firebase、远程/共享会话、
MCP、AWS provider、computer use、voice、遥测 collector/uploader、自动更新和在线 skills/LSP 下载链。
允许确有本地消费者的 ID/值类型和无传输能力的 no-op 遥测兼容类型；不得保留云 provider。
每个残留按消费者和模块挂载审计；未挂载孤儿源码要清理，但不能误报成当前编译错误。

`offline_hard` 的 HTTP/WebSocket/DNS 保护不得被默认 feature、测试依赖或 TUI feature unification
绕过。IPv4/IPv6 loopback、主机名解析、代理、重定向和实际连接对端均纳入测试；禁止依赖域名黑名单
代替默认拒绝。守卫拒绝是兜底，验收仍要求正常静默运行不发起外连、DNS 和重试。

删除产品内建 Privacy/Data Management/Docs/Issues/Slack/Drive/Oz/计费等上游跳转；
用户主动点击终端输出或文件中的普通 URL 保留。外链扫描必须区分硬编码产品入口与用户数据。

<a id="data"></a>
## 3. 数据、身份与资源

### 3.1 不再变更的运行时身份

| 项 | 固定值 |
|---|---|
| AppId | `AppId::new("dev", "term4u", "Term4u")` |
| GUI/TUI 二进制 | `term4u` / `term4u-tui` |
| 日志名 | `term4u.log` / `term4u-tui.log` |
| macOS bundle id | `dev.term4u.Term4u` |
| URL scheme | `term4u` |
| keyring namespace | `term4u` |

身份已在 `15a202db` 落地并已有真实使用，不再安排改名或重置数据目录。
从当前实现确认数据/日志路径；debug 测试使用隔离 `WARP_DATA_PROFILE`，release 使用隔离 HOME。
不得杜撰 `TERM4U_DATA_DIR`，不得测试用户真实目录或旧 Warp 凭据。
Term4u keyring 仅用于实际需要的本地 secret，不读取/迁移旧 Warp keyring、`WARP_USER_SECRET`、
Firebase/refresh token。

### 3.2 持久化契约

保留历史 migrations、表、未知字段和原始云 JSON/列；不得清表、删除迁移或用新空库掩盖兼容性问题。
迁移与开工 HEAD 及既定迁移保护基线 `94912b78` 对照，既审查删除，也审查修改。
本地 JSON/TOML 的名称、命令、参数、缺省字段和未知字段兼容；workspace/LSP metadata 往返保留
已有时间戳与 server 偏好。损坏/未知对象可跳过，但不能阻止其他数据恢复，warning 不含正文和 secret。

已删除的 Agent/云行保持 opaque、不可见，不引入旧协议依赖重新解码。
每次回归使用只读原件的隔离副本，比较迁移前后关键字段、历史条数、写入及重启后的结果。

### 3.3 当前回归输入（不是历史施工记录）

- `test-data/localization/phase1-before.txt`：原始测试 ID 基线，9777 项；禁止改成当前清单规避丢失检测。
- `test-data/localization/deleted-test-ids.txt`：已明确登记的累计消失 ID；新增删除需单独批准和理由。
- `test-data/localization/oracle-066ec71b/`：真实旧版隔离样本的 `original.sqlite`、
  `original-settings.toml`、`original-snapshot.json`、`original-table-counts.json`。
- `crates/persistence/fixtures/legacy/`：已迁入的 15 个历史 SQLite fixture，保持原字节。

上述输入由原位置直接复用 Git blob 迁入，不重写数据库和 ID 内容。
原始真实样本由上游 `066ec71b` 的 OSS 程序生成，含 2 个 tab、3 个 terminal pane、3 条命令；
SQLite SHA256 为 `5c88f21ed0ad702fe48237248ecdeab9e3602eddabef250f741e6f77dbcee5f0`。
原样本 WAL 为零长度；不保留无内容 WAL 和进程共享内存文件。测试时从副本启动并重新生成所需文件。
旧自定义标题未成功持久化，不能将该样本当作旧自定义标题的运行时覆盖。

### 3.4 平台和资源删除边界

Windows 专属文件、模块挂载、纯 Windows target/build dependencies 应删除。
例外只保留 `crates/command/src/windows.rs`、`crates/warp_util/src/path/windows.rs` 和历史 migrations；
这些例外服务本地路径/命令兼容，不代表支持 Windows。混合依赖保留 macOS 必需部分。
Linux 清理不得误删 macOS 共享分支；不全仓清洗无害内联 cfg。
资源必须按真实消费者审计：bundled/local skills、字体和其他本地功能输入不能作为文档垃圾删除。

<a id="dependencies"></a>
## 4. 依赖与许可证边界

工作区默认维持 AGPL-3.0-only，`crates/warpui` 和 `crates/warpui_core` 保留原 MIT 标识与版权；
`warpui_extras` 不属于 MIT 岛。`LICENSE-AGPL`、`LICENSE-MIT` 和第三方声明不得随文档清理删除。
MIT 岛当前不等于依赖闭包已经全为 MIT；`script/license_boundary_allowlist.txt` 仅登记既有精确边，
不得扩大成整个目录/组织豁免。`crates-local/` 不接受搬运或改名后的 AGPL 实现。

清理失效 workspace/member/dependency/feature/patch，检查正常、开发和 test-util 依赖。
TUI 对 warp 的正常/开发依赖保持 `default-features = false`，显式 local_only/tui，开发再含 test-util。
`email_address` 固定 rev `b3e5205b4efdc83832f230d8eb6894fd00a9ec32`，manifest/lock 保持一致。
删除 `deny.toml` 的组织级 allow-org，逐个登记保留 git URL、rev、用途、许可证来源和必要声明。
同步 about/deny/NOTICES 输入，不进行无关版本升级，不手改 Cargo.lock checksum。

已删除八个云 crate 和两条协议依赖不再重新删除：应证明 workspace、正常依赖树、源码、lockfile、
release 二进制及 bundle 全部无残留。`aws-lc-rs` / `aws-lc-sys` 属于 TLS 后端，不按 AWS SDK 误删。
`sts.googleapis.com` 按实际用途审计，不能误分类为 AWS STS。

<a id="status"></a>
## 5. 唯一完成状态表

“批次已验收”仅对该批次记录的源码/产物有效；“实现已有”不等于最终验收；未执行/缺环境为未验证。
以下是实现评估基线的状态，更新进度只修改本节，不再新增独立 status、milestone、handoff 或 todo。

| ID | 当前状态 | 已有事实 | 剩余工作 |
|---|---|---|---|
| R0 | 基础条件已验证；每轮重做预检 | 本机 Rust/Cargo、受控抓包、系统 DNS 和完整进程树归因已建立 | 当前环境工具/空间/权限预检；不能沿用已消失的临时工具目录 |
| R1 | 批次已验收 | 本地类型、初始化、终端/workspace/Settings 调用链收敛；GUI/TUI 和关联 targets 可构建 | 不重做接口接通；后续改动按 §7 回归 |
| R2 | 批次已验收 | 本地功能、真实旧 DB 迁移/写入/重启、15 个 fixtures、日志/ZIP、PATH LSP、真实 GUI/TUI | 不重新寻找旧 DB；维护上述行为并在最终候选重验 |
| GUI-shell 增量 | 实现与局部验证完成 | `eef716cd` 的补全、prompt、clear、焦点修复；12 focused tests、三组 Clippy、构建/bundle/codesign 有记录 | 该次完整 presubmit 在缺 clang-format 处中断；光标闪烁可见相位仍需人工确认 |
| R3 | 部分实现，未关闭 | Settings 已以本地页面为主，Privacy 使用固定本地策略；TUI 本地化已有 | 云 UI/action/URI/认证/flag/keybinding 的全面负向测试与孤儿源码清理 |
| R4 | 未关闭 | 产品身份已改；Windows 尾项仍存在 | 外链 checker/self-test/presubmit 接入、允许清单、Windows 例外审计 |
| R5 | 部分实现，未关闭 | 云依赖主体删除、精确 pin/TUI features；R1/R2 测试删除 ID 已审计 | 显式 git 来源、当前 inventory、累计源码删除集及全闭包负证明 |
| R6 / V1 | 未验收 | 部分批次工程/实机/网络证据可追溯 | 同一候选全部 §7 与 C1–C10；当前没有最终验收 manifest |
| M7 / V2 | 未完成，不在本轮执行 | 运行时身份已完成；文档入口在本次统一 | macOS 品牌资源、许可证材料、发布流程与首个正式产物 |
| 完整 MIT 路线 | 未启动 | 保留两个 MIT 岛与原创代码边界 | 仅在另行批准后启动独立重实现 |

### 5.1 已确认的具体差距

`persistence/block_list.rs` 已改成本地终端类型，不再把旧 Agent 导入列为当前编译错误。
`settings_view/tab_menu.rs` 仍引用 CloudModel，但未由 Settings 根模块挂载，属于 R3 孤儿源码。
`script/check_external_product_links` 不存在；`deny.toml` 仍有组织级放行；R4–R6 不能判完成。
当前只整理文档和其依赖的回归工具/数据，不顺带实现上述产品清理。

R1/R2 的 `d3fd6fdc` 批次记录：app 1512 passed / 3 skipped，核心/TUI focused 703 / 2，
workspace nextest 4752 / 20，completer v2 131 / 4，完整 presubmit 及三组严格 Clippy 通过。
测试 ID 基线 9777、当时当前 4772、累计消失 5089、新增 84；这些不是最新 HEAD 的统计。
用户当时已授权退役 integration 与迁移接口测试的整文件删除，并记录逐项 ID；不得恢复退役框架，
也不得把该授权扩大为未来任意删保留行为测试的许可。

网络批次仅对指定 debug GUI/TUI 有效：GUI 620 秒、TUI 65 秒，含启动和正常退出的完整派生树
未见包/DNS/重试/待发文件。这不替代 release/bundle、完整 S1–S13、系统防火墙或 C1–C10。

### 5.2 状态依据

以下为固定源码提交上的证据，只用于追溯以上完成声明，不是另一套设计或施工入口。
当前分支不再保留过期设计、失败尝试日志和重复截图；Git 历史不重写。

- [R0 采集与进程/DNS 归因](https://github.com/tedczj/term4u/blob/eef716cd69b84e0676eb90ad3ee776397aa6b515/docs/redesign/phase1-acceptance/m5-m6/recovery/network-es-20260912/README.md)
- [R1/R2 批次 checkpoint 与原始日志索引](https://github.com/tedczj/term4u/blob/d3fd6fdc7b91dc42dca99348deaa5827917fc05a/docs/redesign/phase1-acceptance/m5-m6/recovery/r1-r2-20260912/current-checkpoint.json)
- [最新 GUI-shell 局部验收及限制](https://github.com/tedczj/term4u/blob/eef716cd69b84e0676eb90ad3ee776397aa6b515/docs/redesign/phase1-acceptance/m5-m6/recovery/gui-shell-20260913/README.md)
- [已授权测试删除审计](https://github.com/tedczj/term4u/blob/d3fd6fdc7b91dc42dca99348deaa5827917fc05a/docs/redesign/phase1-acceptance/m5-m6/recovery/r1-r2-20260912/test-removal-audit.json)

<a id="implementation"></a>
## 6. 剩余施工步骤

顺序固定：**环境预检 → R3 → R4/R5 → R6 → 关闭 V1 → 经授权进入 M7**。
R1/R2 作为回归契约，不再重新大规模删除/重建；若回归失败，修复该失败再继续。

### 6.1 环境预检

记录 HEAD、UTC、初始工作树、rust-toolchain.toml 指定工具链、空间和权限；先找已安装工具，
不擅自升级或修改系统配置。确认 Cargo/nextest、clang-format、仓库指定 wgslfmt、GUI/PTY、
macOS 捕获权限及隔离数据副本。历史 /tmp 工具消失需要恢复，不能删除检查绕过。
先完成 §7.1 并重跑完整 presubmit 获取当前基线；按失败根因分组，不引用旧错误数量。

### 6.2 R3：UI、认证、动作和 TUI 收尾

按 Settings 页面/搜索 → Command Palette/keybindings → 菜单/workspace actions → URI/deeplink
→ auth 值类型 → TUI 的顺序核对。先确认模块和消费者，再删未挂载孤儿文件、云 flag/快捷键。
重点包含 `app/src/settings_view/`、`settings/`、`app_menus.rs`、workspace/root 和 URI 处理入口。
对 `drive`、`team`、`billing`、`api key`、`warp agent` 搜索及对应 deeplink 加拒绝/不可见测试；
对本地设置搜索、文件/终端操作加成功测试。不能留下灰色云按钮、“登录后使用”壳或幽灵结果。
Privacy 固定本地值、auth 无云凭据；TUI 固定宽度 render-to-lines 与真实 PTY 操作均验证。
出口：源码/注册表审计、正反向测试、GUI 可见结果与 TUI 屏幕文本齐全，且 R1/R2 不回归。

### 6.3 R4：产品外链和 Windows 尾项

实现 `script/check_external_product_links`，包含允许/拒绝样例及 `--self-test`，区分产品硬编码跳转
和用户 URL；扫描读文件/命令错误必须失败，不能 `|| true` 吞错。接入 presubmit 的 Clippy 之前。
逐个保留外链登记路径、触发方式、理由与批准人，不能笼统允许整个域名/目录。

审计 app、warpui、warpui_extras、prevent_sleep、warp_terminal、warp_util 的 Windows 文件、
模块及 target/build dependencies，剩余严格等于 §3.4 例外。检查 embed-resource、dunce 等
失去消费者的依赖；已删除项不重做，混合 target 保留 macOS 所需部分。
出口：network/external 两种 checker 正常检查和 self-test 全过，Windows 与外链表可逐项复核。

### 6.4 R5：依赖、供应链和删除清单

执行 §4 的 manifest/feature/git 来源收敛和 §7.3 负证明；不恢复 integration crate 或云协议。
`script/deletion_set.txt` 是累计实际源码删除范围，不能继续使用单批快照或另建同名设计副本。
测试分类工具 `script/lib/classify_tests.py` 只辅助识别 A（删除）、B（断裂候选）、C（行为漂移候选），
不代替编译、真实测试 ID 或人工批准。它只读取同一份 deletion_set。

运行实际 `script/test_inventory`，相对于原始 ID 基线精确核对消失集与已批准集，两者必须相等；
新 ID 与保留行为覆盖另行审查。清单/JSON/编译生成失败不能发布空快照，旧快照不能被失败覆盖。
新删除须理由与批准，保留行为测试不得通过 ignore、dead_code、弱断言或 blanket allowlist 变绿。
出口：当前 ID 差异、逐项理由/授权、累计删除集、依赖来源/许可证及所有结构负证明齐全。

### 6.5 R6：同一候选完整验收

全部修复后经授权冻结源码/manifest/lock/测试/脚本/资源为 `SOURCE_HEAD`。
针对同一候选执行 §7 全部命令和实机矩阵，逐项满足 C1–C10。任一输入变动即重新冻结重验。
保存失败和修复关系，但当前状态只更新 §5；不能拼接旧批次 PASS，也不能宣布“只差验证所以完成”。
正式证据 manifest 只索引观测结果与本文 ID，不复制另一份目标或验收条件。

<a id="verification"></a>
## 7. 完整验证程序

全部在仓库根目录执行；记录命令、UTC、平台、工具版本、退出码和原始输出。
管道使用 `set -euo pipefail`；未执行为 NOT_RUN，环境不足为 INCOMPLETE，产品失败为 FAIL。
命令列在这里不表示已通过。临时 focused 过滤必须实际命中测试，最终不替代完整集合。

### 7.1 预检、编译和 focused 全集

```bash
git rev-parse HEAD
git status --short --branch
git diff --check
rustc --version
cargo --version
cargo nextest --version
df -h .
cargo check -p warp --lib --no-default-features --features local_only
cargo check -p warp --all-targets --tests --no-default-features --features local_only
cargo check -p warp_tui --all-targets --tests
cargo check -p ai -p persistence -p warp_terminal -p lsp --all-targets --tests
cargo nextest run -p warp --no-default-features --features local_only,test-util
cargo nextest run -p warp_tui -p ai -p persistence -p warp_terminal -p lsp
./script/test_inventory
./script/check_network_boundaries
```

### 7.2 全部工程门禁与构建

```bash
./script/format
./script/format --check
git diff --check
./script/check_no_inline_test_modules
./script/check_network_boundaries --self-test
./script/check_network_boundaries
./script/check_external_product_links --self-test
./script/check_external_product_links
./script/check_license_boundaries
./script/check_license_config_sync
python3 script/lib/test_inventory_tests.py
./script/test_inventory
cargo clippy -p warp --all-targets --tests --no-default-features --features local_only -- -D warnings
cargo clippy -p warp_tui --all-targets --tests -- -D warnings
./script/presubmit
cargo test --workspace
cargo test --workspace --doc
cargo deny check sources licenses bans advisories
cargo clippy -p warpui_core --features tui --all-targets -- -D warnings
cargo build -p warpui_core --bin tui_integration --features tui
cargo clippy -p editor -p warp_tui --features test-util --benches -- -D warnings
cargo build --workspace
cargo build --locked -p warp --bin term4u --features offline_hard
cargo build --locked -p warp --bin term4u --no-default-features --features local_only
cargo build --locked -p warp_tui --bin term4u-tui \
  --no-default-features --features offline_hard,standalone
cargo build --release --locked -p warp --bin term4u --no-default-features --features local_only
cargo build --release --locked -p warp_tui --bin term4u-tui \
  --no-default-features --features offline_hard,standalone
```

presubmit 必须保留三组严格 Clippy：workspace（排除单独测的 warp_completer）、default GUI、
default completer；还包括 clang-format、WGSL、workspace nextest（排除 command-signatures-v2）、
completer v2 和 doc tests。inventory 失败处理单测不等于真实 ID 全集检查。
TUI integration/bench 组合是额外门禁，不因 presubmit 未覆盖而省略；不使用不支持的 --all-features。

### 7.3 结构、源码和依赖负证明

以下代码在同一个 Bash 会话按序运行。`no_match` 只接受 rg 的“无匹配”退出码 1，不接受扫描错误。

```bash
set -euo pipefail
SOURCE_HEAD=$(git rev-parse HEAD)
E="verification/$SOURCE_HEAD"
mkdir -p "$E"
no_match() {
  local rc=0
  "$@" || rc=$?
  if [ "$rc" -ne 1 ]; then
    echo "FAIL: expected no matches (exit 1), got exit $rc" >&2
    return 1
  fi
}
BLACKLIST='sentry|opentelemetry|rudder|warp_server_client|warp_server_auth|warp_multi_agent|cloud_object|firebase|session-sharing|computer_use|voice_input|remote_server|\bmcp\b|rmcp|aws-sdk|aws-smithy'
cargo metadata --locked --no-deps --format-version 1 > "$E/metadata.json"
jq -r '.packages[].name' "$E/metadata.json" > "$E/workspace-packages.txt"
no_match rg -i "$BLACKLIST" "$E/workspace-packages.txt"
cargo tree --locked -p warp --no-default-features --features local_only -e normal > "$E/cargo-tree.txt"
cargo tree --locked -p warp_tui --no-default-features --features offline_hard -e normal > "$E/cargo-tree-tui.txt"
no_match rg -i "$BLACKLIST" "$E/cargo-tree.txt" "$E/cargo-tree-tui.txt"
no_match rg -n 'name = "(warp_server_client|warp_server_auth|firebase|warp_multi_agent_client|cloud_object_client|cloud_object_models|cloud_object_persistence|cloud_objects|session-sharing-protocol|warp_multi_agent_api)"' Cargo.lock
no_match rg -n 'session-sharing-protocol|warp-proto-apis|branch = "main"' Cargo.toml app/Cargo.toml crates --glob Cargo.toml
no_match rg -n 'ServerApiProvider|OfflineApi|trait (AIClient|ObjectClient|AuthClient)|warp_server_client|warp_server_auth|warp_multi_agent_client|warp_multi_agent_api|session_sharing_types' app/src crates --glob '*.rs' --glob Cargo.toml
no_match rg -n 'CloudModel|CloudViewModel|SyncQueue|UpdateManager|ObjectClient|http_client' app/src/local_objects app/src/workflows app/src/notebooks app/src/env_vars --glob '*.rs'
no_match rg -n 'fetch_latest_server_metadata|install_from_github|fetch_npm_package_version|GITHUB_API_URL' crates/lsp app/src --glob '*.rs'
for path in \
  app/src/drive app/src/cloud_object app/src/server/cloud_objects app/src/server/sync_queue.rs \
  app/src/settings/cloud_preferences_syncer.rs app/src/ai/agent app/src/ai/agent_sdk \
  app/src/ai/agent_management app/src/ai/blocklist app/src/ai_assistant \
  app/src/server/graphql app/src/server/server_api.rs app/src/server/server_api \
  app/src/server/offline_api.rs crates/warp_tui/src/cloud_run.rs crates/warp_tui/src/handoff \
  crates/warp_server_client crates/warp_server_auth crates/firebase crates/warp_multi_agent_client \
  crates/cloud_object_client crates/cloud_object_models crates/cloud_object_persistence crates/cloud_objects \
  crates/integration; do
  test ! -e "$path" || { echo "FAIL: removed path remains: $path" >&2; exit 1; }
done
```

另归档 auth/UI/actions、Windows 文件及 target blocks、全部内建外链、features、下载脚本、
迁移 diff 和真实测试 ID 扫描。文本无命中不代替运行与依赖闭包；旧表名/opaque JSON 不代表云实现。

### 7.4 发布产物与资源

沿用 §7.3 的 E/no_match，扫描真实发布候选；记录 GUI/TUI 和 bundle 全资源文件 hash 清单。

```bash
PATTERN='app\.warp\.dev|rtc\.app\.warp\.dev|sessions\.app\.warp\.dev|oz\.warp\.dev|releases\.warp\.dev|rudder|sentry|otlp|opentelemetry|firebase|AIzaSy|warp drive|session sharing|aws bedrock|mcp'
strings target/release/term4u > "$E/gui-strings.txt"
strings target/release/term4u-tui > "$E/tui-strings.txt"
no_match rg -i "$PATTERN" "$E/gui-strings.txt" "$E/tui-strings.txt"
cargo bundle --release --bin term4u --no-default-features --features local_only
no_match rg -a -n -i "$PATTERN" target/release/bundle/osx/Term4u.app
shasum -a 256 target/release/term4u target/release/term4u-tui
codesign --verify --deep --strict target/release/bundle/osx/Term4u.app
```

真实假阳性按精确字符串、来源、保留理由和负责人批准记录，保留原始命中和过滤后结果。
不能整体忽略 warp/mcp、全部 resources 或某依赖组织。扫描产物必须与实机运行 hash 相同。
签名校验不等于 Apple 公证或正式发布；身份/scheme/keyring 和签名状态单独记录。

### 7.5 GUI、TUI、数据与本地功能矩阵

| ID | macOS GUI 场景 | 最小观察 |
|---|---|---|
| S1 | 冷启动 | 正确身份，无登录/云启动依赖 |
| S2 | tab | 新建/切换/关闭/Undo，位置正确 |
| S3 | split | 新建/调整/退出单个 shell，其他 pane 不丢 |
| S4 | 命令与输入 | 输出、Ctrl-C、补全、prompt/PS1/Git、clear/回滚 |
| S5 | 文件树 | 打开目录与文件，局部状态恢复 |
| S6 | 文本预览/编辑 | 预览、编辑保存、未保存取消关闭、本地搜索/diff |
| S7 | 设置与菜单 | 本地项可用，云搜索/菜单/URI 不可见不可调用 |
| S8 | 失焦/聚焦 | 鼠标点击回输入，拖选保留，光标可见及闪烁人工确认 |
| S9 | 睡眠/唤醒 | 会话与 UI 保留，无后台重连/重试 |
| S10 | 关窗与恢复 | 注册表、tab/pane/左面板模式及宽度正确 |
| S11 | 退出/重启 | 历史、notebook/workflow、本地编辑与窗口状态往返 |
| S12 | debug Rust panic | 真实 panic 入本地日志；无崩溃上传，不能用 SIGSEGV 替代 |
| S13 | 空闲十分钟 | 无外连/DNS/重试/待发文件，UI 与进程退出正常 |

另逐项验证 §2.3 所有保留契约，尤其 workflow 参数和项目加载、notebook 真实换行与重启、
code review/Undo、日志 UI ZIP CRC、本地 env 展开、bundled skills 字节一致性。
LSP 五种类型分别测试 PATH 命中/缺失/不可运行；伪 server 记录启动参数，伪 curl/wget/npx/npm
下载调用日志应为空。真实旧 DB 和 fixtures 在隔离副本完成迁移/写入/再次启动，对照内容和字段。

TUI 必须在真实交互 PTY 中运行 `./script/run-tui`，保存屏幕文本，验证启动、输出、Ctrl-C、
tab/新 tab、外部 CLI 和正常退出；固定宽度 render-to-lines 仅是补充，GUI 测试不能代替 TUI。
截图捕获缺字与实际物理窗口缺字必须区分，不能凭不可靠增量截图修改渲染器或虚报结果。

### 7.6 网络与第二道防线

隔离 HOME/profile、干净 shell、恶意代理配置下运行真实候选 GUI/TUI，各至少 60 秒，另覆盖 S13。
先验证捕获权限和受控流量可见：包、系统 DNS client PID/request ID、短命子进程三条链路可关联。
`script/capture_macos_network.py` 从用户已授权终端手工 sudo 启动；应用必须由普通用户另行启动。
脚本返回 COLLECTED_NOT_YET_VERIFIED 仅表示采集完成，必须继续分析才能判定。

捕获启动到正常退出及收尾窗口的主进程与完整派生树，包含 exec/PID 版本，不能只按进程名或
进程组 grep；核对事件序号、解析错误、内核丢包、采集退出码和所有派生进程退出覆盖。
检查 TCP/UDP/WebSocket、DNS 53/853、系统代理解析、后台重试以及待发遥测/崩溃文件。
空文件必须与成功校准、归因、无丢包和正常收尾共同判读。零外连要求是零非 loopback 请求，
不是被网络守卫拒绝若干次后无成功连接。

系统防火墙作为第二道防线，确认规则有效并保留空的拒绝日志；改规则需授权。
静默场景不主动跑联网命令；用户 PTY 联网能力另外验证，不计入产品静默外连数。
系统级 ES/网络日志可能含其他进程参数/环境，原始文件只留本机受限目录，不上传未经脱敏的全文。

<a id="acceptance"></a>
## 8. 验收条件 C1–C10

以下是唯一验收定义。当前最终候选各项均未整体关闭；§5 的批次通过不能直接填作最终 PASS。

| ID | 完成条件 | 证据要求 |
|---|---|---|
| C1 | GUI/TUI 及关联 crate 的 default/local_only/test-util 全组合通过 | §7.1/7.2 check/build、命令/退出码与候选身份 |
| C2 | 终端、Ctrl-C、tab/split/window、编辑/文件/搜索/diff 行为保留 | 对应本地测试、GUI S1–S11 和真实 TUI |
| C3 | workflows/notebooks/env-vars 和旧 DB 可读写恢复 | 样本/fixture、字段对照、迁移审计、写入/重启 |
| C4 | 云 UI/auth/Agent/Drive/共享及动作死引用清零，本地设置/TUI 正常 | 正反向测试、注册表/源码扫描、可见结果 |
| C5 | PATH LSP、skills/日志完整，外链和 Windows 尾项关闭 | 五种 LSP 分支/零下载、日志三场景、资源/外链/平台审计 |
| C6 | 云 crate/协议/ServerApi 在所有闭包无残留且供应链明确 | metadata、GUI/TUI tree、source/lock、pin、显式 git 来源、cargo deny |
| C7 | 无未批准测试消失，保留覆盖未弱化，完整工程门禁通过 | 实际 inventory、ID/批准对照、累计删除集、format/clippy/presubmit/workspace/doc/额外组合 |
| C8 | release 二进制/bundle 无未批准黑名单；静默运行零外连/DNS/重试/待发文件 | hash、strings/resources、归因捕获、S1–S13、系统防火墙 |
| C9 | macOS arm64 全部最小矩阵通过 | 同一源码的工程/实机/旧数据/网络/签名/身份/scheme/keyring 记录 |
| C10 | R0–R6 全收敛且证据可追溯，不把缺环境/WIP 当完成 | 同一 SOURCE_HEAD manifest、输入 hash、初始/最终 diff/status、本文状态更新 |

C1–C10 全部通过后，才能把 §5 的 V1 标为完成并关闭 V0 严格验收，不再维护第二个 V0 manifest。
M7/V2 和完整 MIT 不因 V1 完成而自动完成。

<a id="evidence"></a>
## 9. 证据、文档与变更纪律

本文件同时承担目标、架构、已完成/未完成和施工验收说明。README 只提供介绍与入口；AGENTS/
CONTRIBUTING 只提供工程/协作规则，不复制状态表或另建方案。更改设计直接更新本文，不创建
带日期的替代设计、历史分支方案、handoff、milestone 或 todo。过期说明、孤儿截图/日志/临时脚本
直接从工作树清理，历史需要时查看 Git；测试输入和产品资源按真实消费者保留。

新验收原始输出放 `verification/<SOURCE_HEAD>/`，manifest 使用机器可读数据，至少包含：
源码提交、设计版本、工具/平台、feature/build 参数、输入及产物 hash、C-ID、命令、UTC、退出码、
日志路径、观察/限制、PASS/FAIL/NOT_RUN/INCOMPLETE。数据文件不重新定义设计或维护另一份待办。
同一候选的失败尝试保留以解释最终结果；候选失效后不再保留成当前分支上的独立状态记录。
归档前脱敏；不能用模型总结代替原始日志，也不需要在 manifest 写入它自身提交 hash。

提交证据可在 SOURCE_HEAD 后作为不改变构建/测试/运行输入的提交；若脚本、样本、配置或资源
变化，旧证据不自动覆盖新输入。本次文档清理也迁移了回归工具/数据路径，因此要重新验证工具，
不冒称产品最终验收通过。

不得覆盖用户已有工作树或真实数据。提交、push、PR、tag、发布、特权采集和系统安装各按用户
授权范围执行；文档 PR 的授权不等于 merge/tag/发布授权。只清理已明确退役的仓库记录，不为了
空 git status 删除其他用户文件。已有编码、终端锁与测试纪律继续适用，不能靠删门禁变绿。

继续开发指令：

```text
按 docs/DESIGN.md 执行环境预检、R3、R4/R5、R6，保留已验收的 R1/R2 行为，不恢复云空壳或退役 integration。完成全部 §7 并满足 C1–C10，按同一候选归档原始证据；只在本文件更新状态。缺环境/未执行如实记录，禁止削减验证。M7/V2、完整 MIT 和发布操作不自动展开；提交与特权操作按明确授权执行。
```

<a id="release"></a>
## 10. V2 发布与后续演进

### 10.1 M7 / V2：V1 关闭后执行的同一设计

| 工作包 | 实施内容与出口 |
|---|---|
| macOS 品牌资源 | 从实际消费者出发处理 bundled Warp SVG、about/logo、channel 图标、安装图片、Dock 插件及字体品牌字形；用独立 Term4u 资产替换，验证构建/资源引用及可见结果；不再改 §3.1 身份 |
| 文档与归属 | README/贡献/安全/行为准则及 authors/联系入口统一 Term4u；来源、基线和原版权保留；删除上游宣传/支持/发布承诺，不暗示获得上游背书 |
| 许可证材料 | 生成目录许可证地图和 NOTICES、核对 MIT 岛声明与第三方依赖；保留 AGPL 源码与适用分发材料，不把 fork 改写成 MIT |
| 构建与发布流程 | 清理依赖上游内部 secrets/服务的 CI 和发布脚本，建立 macOS 可复现打包、签名状态/公证策略与源码对应；不创建 Windows/Linux 新发布路线 |
| 发布验收 | 复核名称/品牌、真实 `.app` 资源与身份、全部 V1 回归、许可证材料、源码/产物 hash；经单独授权创建首个 tag/release，记录实际执行结果 |

现行工程门禁沿用 §7，M7 只增加品牌/许可证/发布检查，不另建冲突的最低标准。
用户运行时保持本地离线，不增加自动更新。发布工作不能靠删除法律归属字符串达到品牌扫描零命中。

### 10.2 可选完整 MIT 重实现

只有在 V1/V2 完成且另行批准后进入，当前无实施承诺和未经校验的工作量估计。
先从本文保留行为确定新产品范围和黑盒验收；审计两座 MIT 岛的直接/传递依赖，再以独立实现或
经核验的合适第三方替换 AGPL 依赖。保留来源、许可证和实现过程记录，不复制 AGPL 代码到 MIT。
原版 MIT 部分仍需其版权声明；“MIT 岛存在”不是“最终应用已经 MIT”的证据。

未来确需 agent_protocol/local_agent_bridge/local_fs_service 时，再定义协议、认证/权限、
进程与文件边界和行为测试并更新本文；当前不创建空 crate，也不恢复 Warp 云 API 为其铺路。
