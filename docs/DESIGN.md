# Term4u 完整设计与实施基准

> 唯一设计、状态与施工依据；更新：2026-09-20。
> 当前实施分支：`dev-20260920`；当前产品范围仅 macOS Apple Silicon / ARM64。
> 原已完成批次审计：`dev-202609014` / `65a012a7ac344e05c7e09c51a253cbc661e5ac35`；不是本轮候选证据。
> 原 R0/R1/R2 和原 GUI-shell 批次已关闭；本地交互保全 L0、R3–R6 与 V1 尚未关闭。
> “最终设计”表示只采用这一套目标方案，不表示所有功能已经实现或验收。

## 导航

- [1. 目标与边界](#goals)
- [2. 最终架构与保留契约](#architecture)
- [3. 数据、身份与资源](#data)
- [4. 依赖与许可证边界](#dependencies)
- [5. 唯一完成状态与分支审计](#status)
- [6. 当前施工顺序与本地功能恢复](#implementation)
- [7. 完整验证程序](#verification)
- [8. 验收条件 C1–C10](#acceptance)
- [9. 证据与变更纪律](#evidence)
- [10. V2 发布与后续演进](#release)

<a id="goals"></a>
## 1. 目标与边界

Term4u 是 Warp 客户端的本地化衍生产品。上游 `warpdotdev/warp`，来源提交
`066ec71b736fc3755e29f58f733deadbdac3d1af`；项目仓库 `tedczj/term4u`。
保留本地终端、分栏、文件树、预览、编辑和数据能力；删除云控制面，而不是删除与云代码相邻的本地能力。

| ID | 产品必须满足的条件 |
|---|---|
| H1 | 产品构建不包含遥测传输、崩溃上传、云 Agent、服务端认证、Drive 同步、共享、更新等已删除实现 |
| H2 | 启动、恢复、空闲和退出不创建 Warp Server HTTP/SSE/WebSocket 客户端或后台云重试任务 |
| H3 | 产品网络出口只允许 loopback；非 loopback 请求在统一守卫中硬拒绝，不进行外部 DNS 解析 |
| H4 | 新增产品网络调用必须经过守卫；构造器检查、依赖/产物扫描和运行观测共同防止回归 |
| H5 | 用户主动运行的 shell、git、ssh、CLI agent 是独立 PTY 子进程；Term4u 不替用户限制其网络 |

Term4u 不是沙箱。产品静默零外连、用户主动执行联网命令、构建工具下载依赖分别验证。
不得恢复运行/构建脚本自动拉取并执行 skills 的链路，也不得把用户自己的 CLI agent 当作应删除的 Warp Agent。

**当前只支持 macOS + Apple Silicon（ARM64，Rust target `aarch64-apple-darwin`）。**
Intel Mac / `x86_64-apple-darwin`、Rosetta、Universal Binary、Linux、Windows、WASM 均不属于当前
产品支持或验收范围。GUI 与 TUI 都遵守这一范围。历史兼容代码仍在不代表承诺支持；其后续清理见
[§3.3](#platform)。本次 L0 不要求完成兼容层清理，也不得把未验证平台记为 PASS。

只采用路线 A：删除 Warp Agent UI/SDK/执行层和云协议；不恢复 ServerApi/OfflineApi 双后端、
假 URL、空 token、同名空壳或静默重试。不新增本地 Agent 协议来替代本地终端功能。

V0 表示本地可用，V1 表示本地保留契约、云清理和 C1–C10 完整验收，V2 表示品牌与发布。
完整 MIT 重实现属于另行批准的后续战略，不属于本轮。批次关闭不等于整个产品的功能等价性认证。

<a id="architecture"></a>
## 2. 最终架构与保留契约

```text
GUI app/ (term4u)                  TUI crates/warp_tui (term4u-tui)
       |                                      |
       +-- workspace / pane / local settings --+
       +-- terminal / editor / search / local diff
       +-- workflows / notebooks / local_objects / env_vars
       +-- local persistence / PATH LSP / local logs / bundled skills
                          |
        warp_core / warpui / warpui_core / persistence / warp_terminal
                          |
      local files / SQLite / macOS / PTY / guarded loopback communication

User-requested command -> independent PTY child -> networking controlled by the user
```

### 2.1 公共层与启动

保留 `app/`、`crates/` 和内部 `warp_*` 名称。GUI 使用 Element/View、GPU/WGSL 与 `.app`；
TUI 使用 cell-grid/TuiElement，不能用 GUI 截图代替真实 PTY。共享本地实体和数据，而非云账号后台。

沿用 App/Entity/Handle、context/action 和锁传递；不在终端调用链增加重入 `model.lock()`。
GUI MouseStateHandle 在构造期持有；异步回调验证当前实体、会话和请求，不能用过期结果覆盖新输入。

入口确定 Term4u 身份与离线 feature，再安装本地持久化和前端 manager；恢复设置、历史、窗口、
pane 和本地对象。不得跳过恢复换取启动成功。旧云行原样保留但不启动云 singleton/同步队列。
退出保存本地状态并停止任务，不创建待上传文件。

### 2.2 必须保留的本地行为

| 面 | 保留契约 |
|---|---|
| PTY、shell、外部 CLI | 启动、输出、Ctrl-C/Ctrl-D、退出；普通 git/ssh/CLI agent、前后台程序、alternate screen、终端模式和尺寸变化可用 |
| 命令输入 | Unicode、光标/选择处插入、Undo、多行/换行、自动增长、Vim/光标/字体/行高偏好；多行上下移动不能被历史抢走 |
| 历史与补全 | 复用现有 History 的 shell/会话数据；上下回看及草稿恢复、历史搜索与本地建议；Tab 唯一/公共前缀/候选循环，保留后缀，丢弃过期结果 |
| 提示符与输出 | 目录/Git/PS1 提示、clear 清可见区但保留历史、查找/选择/复制、块导航和折叠；大输出滚动与原生 CLI 不应退化 |
| 菜单与焦点 | 右键菜单、按键导航、Esc 关闭、Enter 只执行所选动作；关闭恢复合理焦点，Find/编辑器主动获得的焦点不被抢回 |
| 剪贴板与文件拖入 | 用户粘贴按光标/选区插入、路径按实际 shell 转义；拖入文件作为路径，不变成 Agent 附件；原生程序 bracketed paste 单独验证 |
| 输出链接与本地文件 | 用户主动点击输出 URL、OSC 8 链接及文件/行号打开保留；与删除产品内建 Warp 推广/云入口严格区分 |
| 窗口与 pane | tab/split/新窗/关窗/Undo/重启恢复；单个 shell 退出只关自身；注册表、宽度、左面板和标题一致 |
| 编辑/文件树/搜索/diff | 打开、预览、全文搜索、编辑保存、未保存取消关闭、本地 code review 与 Undo |
| workflow | 本地及项目 `.warp/workflows` 加载、编辑、参数默认/覆盖值和执行；缺少必填参数留在输入框；名称/描述/参数字段保留 |
| notebook/env vars | 旧 SQLite/JSON 读写、真实换行、重启恢复、本地新值不被旧行覆盖；环境变量解析/展开保持，不保留 Drive 权限/分享语义 |
| LSP/skills | 现有五种 server 只查 PATH，命中/缺失/不可执行均处理；缺失只提示手工安装，不查版本、不下载；skills 仅读已打包/本地内容 |
| 日志与本地隐私 | 正常/debug Rust panic/UI ZIP 日志仍在本地；终端响铃与用户本地通知不属于遥测；本地隐私显示设置应真实作用于渲染 |
| 设置与 UI | Appearance/Features/Keybindings/本地隐私与搜索有效；不能留下可点击但空操作的本地项；固定关闭云传输不等于删除本地设置 |

该表是保留目标，不是全量通过声明。具体已验收项和仍需补回的项目只在 §5/§6 标记。

### 2.3 删除面和网络守卫

删除 Drive/cloud object、云 Agent/blocklist、GraphQL/ServerApi/OfflineApi、Firebase、远程/共享
控制面、MCP、AWS provider、computer use、voice、遥测 collector/uploader、自动更新与在线下载链。
允许确有本地消费者的中立 ID/值类型及无传输能力的 no-op 遥测兼容类型；不保留云 provider。

删除不能按旧目录名机械执行。混合模块先拆本地行为，再删云消费者；未挂载孤儿文件不等于活动功能，
但替代实现缺少原本地行为仍是回归。每项本地能力都要追踪“动作/设置 -> 处理 -> 可观察结果 -> 测试”。

`offline_hard` 的 HTTP/WebSocket/DNS 保护不能被 default、test-util 或 TUI feature unification
绕过。IPv4/IPv6 loopback、域名、代理、重定向和实际连接对端均测试；不能用域名黑名单替代默认拒绝。
守卫拒绝只是兜底，正常静默运行仍须不发起外连/DNS/重试。

删除产品内建 Privacy/Data Management/Docs/Issues/Slack/Drive/Oz/计费等上游跳转；保留用户主动
点击普通 URL。OSC 剪贴板读取等能力恢复时保留权限/信任边界，不为功能恢复盲目放开宿主数据读取。

<a id="data"></a>
## 3. 数据、身份与资源

### 3.1 固定运行时身份

| 项 | 固定值 |
|---|---|
| AppId | `AppId::new("dev", "term4u", "Term4u")` |
| GUI/TUI | `term4u` / `term4u-tui` |
| 日志 | `term4u.log` / `term4u-tui.log` |
| macOS bundle id | `dev.term4u.Term4u` |
| URL scheme / keyring namespace | `term4u` / `term4u` |

身份已在 `15a202db` 落地并真实使用，不再改名或重置目录。数据/日志路径以当前实现为准；
debug 用隔离 `WARP_DATA_PROFILE`，release 用隔离 HOME。不杜撰 `TERM4U_DATA_DIR`。
只使用隔离数据副本；不读取/迁移旧 Warp keyring、`WARP_USER_SECRET`、Firebase/refresh token。
Term4u keyring 仅用于实际需要的本地 secret。

### 3.2 持久化与回归输入

保留历史 migrations、表、未知字段和原始云 JSON/列，不清表、不删迁移、不以空库掩盖兼容问题。
迁移同时与开工 HEAD 和 `94912b78` 比较删除及修改。保留 JSON/TOML 名称、命令、参数、缺省/未知
字段，以及 workspace/LSP 时间戳和偏好；损坏对象不阻断其他对象恢复，warning 不含正文/secret。
Agent/云行保持 opaque 且不可见，不恢复旧协议解码。

| 输入 | 用途 |
|---|---|
| `test-data/localization/phase1-before.txt` | 不可重置的 9777 项原始测试 ID 基线 |
| `test-data/localization/deleted-test-ids.txt` | 已批准累计消失 ID；新增删除必须另行批准 |
| `test-data/localization/oracle-066ec71b/` | 原始 `original.sqlite`、settings TOML、snapshot 与 table-counts JSON |
| `crates/persistence/fixtures/legacy/` | 15 个历史 SQLite fixture，保持原字节 |

原真实样本来自 `066ec71b` OSS 程序，包含 2 个 tab、3 个 terminal pane、3 条命令；SQLite SHA256
为 `5c88f21ed0ad702fe48237248ecdeab9e3602eddabef250f741e6f77dbcee5f0`。原 WAL 为空；不保留
无内容 WAL/进程共享内存，测试从副本重新生成。旧自定义标题未成功持久化，不宣称该样本覆盖该行为。
输入完整性检查不是 GUI 迁移/写入/重启验收；后者需对副本比较关键字段、历史条数和重启结果。

<a id="platform"></a>
### 3.3 平台与资源

**平台决策（2026-09-20）：macOS Apple Silicon only。** 本轮只为 `aarch64-apple-darwin`
实现、集成验证和产出候选；开发容器为 Linux 不会使 Linux 成为产品平台。

兼容清理作为后续工作保留，不在本次 L0 扩大施工：审计并移除 Intel/Rosetta/universal 构建与打包、
非目标平台专属 GUI/backend、target dependencies、CI/安装/发布流程，以及仅服务这些目标的兼容抽象。
先追踪真实消费者再删除；macOS ARM64 必需的 POSIX/Unix 共享层、SSH/远端路径语义、历史持久化数据、
migrations 和许可证声明不得按平台名称误删。保留例外需有具体消费者，不得永久保留无用途兼容层。

以下为已有平台清理约束；其未关闭状态继续追踪，本轮可以不做：

清理 Windows 专属文件、模块挂载和纯 Windows target/build dependencies；仅保留
`crates/command/src/windows.rs`、`crates/warp_util/src/path/windows.rs` 和历史 migrations 例外。
混合依赖保留 macOS 必需部分；不全仓清洗无害 cfg。Linux 代码清理不得误删共享实现。
字体、skills、shell 集成与本地功能资源按实际消费者保留，不当作文档垃圾删除。

<a id="dependencies"></a>
## 4. 依赖与许可证边界

默认 AGPL-3.0-only；`crates/warpui`、`crates/warpui_core` 保留原 MIT 标识/版权，warpui_extras
不属于 MIT 岛。保留 LICENSE-AGPL、LICENSE-MIT 与第三方声明；不能移动/改名 AGPL 代码冒充原创 MIT。
MIT 岛不等于依赖闭包全为 MIT，`script/license_boundary_allowlist.txt` 仅登记已有精确边。
`crates-local/` 不接受 AGPL 实现；不扩大为整个目录或组织豁免。

清理失效 workspace/member/dependency/feature/patch，检查 normal/dev/test-util。
TUI 对 warp 正常/开发依赖保持 default-features=false、local_only/tui，开发含 test-util。
`email_address` 固定 rev `b3e5205b4efdc83832f230d8eb6894fd00a9ec32`，manifest/lock 一致。
删除 deny.toml 的 allow-org，逐个记录 git URL/rev/用途/许可证来源及声明；同步 about/deny/NOTICES。
不做无关升级、不手改 lockfile checksum。

八个云 crate 与两条协议依赖主体已删，剩余工作是证明 workspace、normal tree、source、lock、
release/bundle 全闭包无残留。aws-lc-rs/aws-lc-sys 是 TLS 后端，不按 AWS SDK 误删；
sts.googleapis.com 按实际消费者审查，不误分类为 AWS STS。

<a id="status"></a>
## 5. 唯一完成状态与分支审计

### 5.1 当前状态

| ID | 状态 | 关闭范围或剩余工作 |
|---|---|---|
| R0 原批次 | CLOSED | 工具环境、受控采集、系统 DNS 与派生树归因的建设已验收；每轮预检属于维护，不重开原批次 |
| R1 原批次 | CLOSED | 已核验的本地类型/初始化/编译调用链；后续新代码仍跑回归 |
| R2 原批次 | CLOSED | 已有 GUI/TUI、本地数据与功能验收按原范围关闭；不是所有上游本地交互都已保留的证明 |
| 原 GUI-shell | CLOSED | `eef716cd` 的补全/prompt/clear/基础焦点；完整 presubmit 与用户 H1/S8 已补齐，无遗留人工 H1 |
| L0 本地交互保全 | IN_PROGRESS | 本地输入、粘贴、菜单及 IME 聚焦测试、完整 presubmit 和集中 GUI/PTY 复核已通过；本轮五项失败已消除，完整未关闭范围见 §6.2.15 |
| R3-F1 菜单焦点 | PARTIALLY_VERIFIED | 打开/关闭焦点与 Esc/Enter/Find 路由测试已通过；跨 pane、目标销毁及完整实机矩阵仍待验收 |
| R3 | OPEN | 本地功能恢复后再收尾云 UI/action/URI/auth/flag/keybinding 与孤儿源码，正反向测试不能省略 |
| R4 | OPEN | 外链检查器/self-test/presubmit 接入、允许清单、Windows 例外审计 |
| R5 | PARTIAL | 云依赖主体/pin/TUI features 已有；供应链、当前测试清单和累计源码删除范围仍需按最终候选核验 |
| R6 / V1 | NOT_ACCEPTED | 同一最终候选完整 §7 与 C1–C10，含 runtime/network/release/bundle，不拼历史 PASS |
| M7 / V2 | NOT_STARTED | 品牌资源、许可证材料、macOS 发布流程和正式产物 |
| 完整 MIT | NOT_STARTED | 仅在另行批准后独立重实现 |

本轮新代码的实现状态与原批次关闭分开。不得把旧测试通过数写成修复后已通过，也不得因发现新的
本地回归就要求用户重做无关旧数据/H1 验收；新增缺口有独立 ID、修复和自动测试。

### 5.2 dev-202609014 已有变更

审计时该分支 HEAD 为 `65a012a7ac344e05c7e09c51a253cbc661e5ac35`，与已测
`000d17a7b2efb78e639882d803f57aba21ba7a73` 的 Git tree 都是
`59432b0cc1d129d9a2e111a1625589524f40eb54`。这是源树等价，不是声称在分支合并提交上重新跑过测试。

相对 `e25f9c174eb38458fb4c4f7a3edcd9260fb6bc5b` 的 11 个变动路径为：

| 文件组 | 分支已有工作 |
|---|---|
| resource_center/sections.rs | 本地面板标题调整 |
| terminal/alt_screen/alt_screen_element.rs、blockgrid_element.rs、mod.rs | 原生/块输出的右键入口与菜单定位 |
| terminal/view.rs、view/action.rs、view/context_menu.rs | 本地复制/粘贴/复制块/插入输入/查找/分栏菜单；未恢复云菜单 |
| terminal/local_view_tests.rs | 六个新增菜单/鼠标/动作测试 |
| block_list_viewport.rs、prompt_render_helper.rs、waterfall_gap_element.rs | 删除三份未挂载旧实现；删除这些孤儿不等于替代实现已有全部原功能 |

本轮恢复以该树为底稿，不覆盖分支既有工作。所发现的多项输入/显示退化早于这 11 个文件的增量，
不能把所有丢失行为都归因于最后一次 merge 或三份孤儿文件删除。

### 5.3 原批次关闭依据

[机器可读复核索引](../verification/000d17a7b2efb78e639882d803f57aba21ba7a73/batch-review.json)
保存原始上传包、patch、两次结果和日志引用的 hash 及范围；原报告不改写。
用户在 macOS 被测 `000d17a7` 上的结果：app 1525 passed/3 skipped，core/TUI 703/2，
presubmit workspace 4765/20，completer v2 131/4；三组 Clippy、clang-format、WGSL、doc tests、
完整 presubmit、GUI/TUI build、bundle/codesign 均通过。12 个原 GUI-shell 与 6 个菜单测试有 PASS；
实际 inventory 9777 baseline、4785 current、5089 removed、97 added。H1/S8 用户 PASS，隔离实例正常退出。

复核了 34 条自动命令、11 条续跑命令及其日志 SHA256、退出码和引用关系。用户报告不是签名远程
证明；执行器指纹与会话挂载附件有差异且上传包不含实际执行器字节，因此不声明执行器/二进制的远程认证。
本次 DB 记录是输入完整性；没有新网络捕获或 TUI 全流程。此前 R0/R2 原始实机验收按各自原范围保留：

- [R0 归因与指定 debug 产物观测](https://github.com/tedczj/term4u/blob/eef716cd69b84e0676eb90ad3ee776397aa6b515/docs/redesign/phase1-acceptance/m5-m6/recovery/network-es-20260912/README.md)
- [R1/R2 checkpoint](https://github.com/tedczj/term4u/blob/d3fd6fdc7b91dc42dca99348deaa5827917fc05a/docs/redesign/phase1-acceptance/m5-m6/recovery/r1-r2-20260912/current-checkpoint.json)
- [原 GUI-shell 记录](https://github.com/tedczj/term4u/blob/eef716cd69b84e0676eb90ad3ee776397aa6b515/docs/redesign/phase1-acceptance/m5-m6/recovery/gui-shell-20260913/README.md)
- [授权测试删除审计](https://github.com/tedczj/term4u/blob/d3fd6fdc7b91dc42dca99348deaa5827917fc05a/docs/redesign/phase1-acceptance/m5-m6/recovery/r1-r2-20260912/test-removal-audit.json)

这些固定提交链接是证据，不是另一套活动设计。R0 的 GUI 620 秒/TUI 65 秒零包/DNS记录不能代替
最终 release/bundle、S1–S13 和系统防火墙。过去授权退役 integration 不扩大为任意删本地测试。

<a id="implementation"></a>
## 6. 当前施工顺序与本地功能恢复

顺序：**环境预检 -> L0 本地交互保全 -> R3 -> R4/R5 -> R6 -> V1 -> 经授权 M7**。
L0 有未关闭的日常交互回归时，优先补回，不继续以大规模删代码追求“干净”。不重做原 R0/R1/R2 施工。

### 6.1 环境预检

记录 HEAD/UTC/初始 diff、pinned Rust、空间、Cargo/nextest、clang-format、指定 wgslfmt、GUI/PTY
与隔离副本条件。先找已装工具，不擅自升级/安装或删门禁；新源码按 §7.1 与 presubmit 获取真实结果。
本轮本地编辑环境是 Linux，无 Cargo/macOS；通过分支 CI 的源码归档取得完整源码，已核对归档
Git tree 与分支一致。macOS Apple Silicon 验证由 `.github/workflows/term4u-l0.yml` 执行；
必须按该次 artifact 的 source-head 判定结果，不能把编译成功、旧候选通过或框架事件测试等同于实屏验收。

### 6.2 L0：本地交互保全的开发与验收设计

> 本节是本文件的唯一 L0 设计；已合并原详细交付件，不另维护 roadmap。
> 本轮实施在 `dev-20260920`，从 `ff4b67a3afed9f72203ea2ef3f8cb8b72d7af1c2` 增量修改。
> 设计审查基线：`main / 24abe29d372c1f1cd607615de48ba2bc741852f0`。
> 上游行为和代码参考基线：`066ec71b736fc3755e29f58f733deadbdac3d1af`。
> 本节含实施规格与 §6.2.15 当前实现台账；台账没有关闭的场景仍为 OPEN/NOT_RUN。
> “代码已写”与“同一候选验证通过”分开登记；66 个场景 ID 不是声称已有 66 个通过的测试。
> 原 R0/R1/R2/原 GUI-shell 不重开施工；L0 关闭也不替代最终 R6/C1–C10。

#### 6.2.1 范围与现状

本轮目标是恢复 §2.2 已承诺的本地行为，不扩展云 Agent、账号、Drive、远程控制面或新 Agent 协议。外部 CLI 仍是普通 PTY 子进程。旧数据库、迁移、未知字段、产品身份和已有测试保持不变。

| ID | 审查基线时可确认的进度（当前分支见 §6.2.15） | 本轮完成出口 |
|---|---|---|
| L0-01 | 基础历史回看、草稿/光标恢复及测试已写 | 历史来源与顺序明确；真实方向键、历史搜索、本地建议及异步失效完整 |
| L0-02 | `Input::insert_text` 已使用编辑器插入语义 | 编辑器插入与原生 PTY paste 分开；bracketed paste、控制字符、Undo 完整 |
| L0-03 | autogrow、soft wrap、行高、Vim、光标选项已接回 | 真实布局、偏好动态变化、尺寸通知与不同输入分支验证 |
| L0-04 | 菜单焦点修复及 Esc/Enter/Find 测试已写 | 菜单、Find、编辑器、跨 pane 与命令完成的焦点所有权闭合 |
| L0-05 | 路径动作处理和 `TerminalSizeElement` OS 事件入口均已存在 | 命中/事件冒泡/反馈/跨 pane/真实 Finder 投递正确；不是从零搭建拖入 |
| L0-06 | 点击/悬浮关键动作仍有空分支 | URL、OSC 8、文件/行号完整；只有用户动作才能打开 |
| L0-07 | MarkedText 空分支；剪贴板/响铃事件在 view 中仅 notify | 分别关闭 IME、OSC 剪贴板权限、本地响铃/通知三个子项 |
| L0-08 | 本地隐私渲染选项已接回 | 块/图片/选择/查找/clear/滚动语义及可测性能完整 |
| L0-09 | 尚无完整入口到结果清单 | 所有保留的设置/菜单/快捷键都有实际消费者及正向测试 |

现有 `input_local_tests.rs` 6 个和 `local_interaction_tests.rs` 7 个测试必须先验证；旧 `input_tests.rs`、`local_view_tests.rs` 及批准测试基线不删除、不弱化。

#### 6.2.2 原始代码参考索引

所有 O 编号链接固定到原始提交；不是当前 main 的文件存在性声明。大文件用符号定位，不依赖可能漂移的行号。

| 编号 | 原始文件及定位点 | 参考用途 | 不可整体带回的内容 |
|---|---|---|---|
| O1 | [terminal/input.rs][o1]；`EditorOptions`、编辑器事件、历史/补全、`InputDropTargetData` | 输入路由、选项、菜单与输入框组合 | AI input、Agent 状态、附件、云建议 |
| O2 | [terminal/input/classic.rs][o2]；`render_classic_input`、`DropTarget::new`、`add_input_suggestions_overlays`、Vim 状态 | 原始输入布局、提示符、DropTarget 接线 | AgentView/AI attachment 分支；不能只恢复旧 import 使之编译 |
| O3 | [terminal/history.rs][o3]；`History`、`ShellHost`、`ReadHistoryFileState`、`HistoryEvent` | shell 文件和 SQLite 历史、并发加载、会话隔离 | CloudModel/CloudViewModel、云 workflow ID 解码 |
| O4 | [terminal/history/up_arrow.rs][o4]；`sort_and_dedupe_suggestions`、`up_arrow_suggestions_for_terminal_surface` | 命令历史去重、排序、忽略项和跨会话语义 | AIQuery、BlocklistAIHistoryModel、AISettings、Agent feature |
| O5 | [util/clipboard.rs][o5]；`clipboard_content_with_escaped_paths` | 区分纯文本与文件路径，只对路径做 shell 转义 | 不适用；仍需验证消费者传入参数 |
| O6 | [terminal/terminal_size_element.rs][o6]；`dispatch_event`、`mouse_position_is_in_bounds` | DragFiles/DragFileExit/DragAndDropFiles、区域判断和布局通知 | 原有事件消费方式也要重新审查，不能因为上游如此就认定正确 |
| O7 | [terminal/links.rs][o7]；`should_directly_open_link`、`directly_open_link_keybinding_string` | macOS Cmd-click 与普通点击/提示的区别 | 此文件只是修饰键助手，不能当成完整链接解析器 |
| O8 | [terminal/view.rs][o8]；按 Paste、MarkedText、ClipboardStore、ClipboardLoad、Bell、ClickOnGrid、MaybeLinkHover、ContextMenu 定位相关处理和调用者 | 原生 paste、IME、模型事件、链接和焦点的旧行为对照 | 大型文件混有云/Agent；禁止整文件恢复或将旧接口做空壳 |
| O9 | [terminal/block_list_viewport.rs][o9]；`ScrollState::update`、`ScrollLines`、`ScrollPosition`、`ViewportState` | 滚动锚点、顶部/底部/瀑布输入、可见范围和高度索引 | 与当前不保留功能绑定的分支；只迁移本地算法 |

当前实现优先复用：`app/src/terminal/input.rs`、`history.rs`、`view.rs`、`view/context_menu.rs`、`view/action.rs`、`terminal_size_element.rs`、`blockgrid_element.rs`、`alt_screen/alt_screen_element.rs`、`app/src/util/clipboard.rs`。

特别说明：main 中虽然仍有 `history/up_arrow.rs` 的旧源码，但不能因路径存在就重新挂载。它含 AI/云关联；当前 `history.rs` 已采用本地化依赖。先复用当前 History，对 O4 只提取命令排序/去重规则并补测试。

可在本地仓库执行以下只读定位，不 checkout 历史版本覆盖工作树：

```bash
ORIGINAL=066ec71b736fc3755e29f58f733deadbdac3d1af
BASE=24abe29d372c1f1cd607615de48ba2bc741852f0
git cat-file -e "$ORIGINAL^{commit}"
git show "$ORIGINAL:app/src/terminal/view.rs" \
  | rg -n 'Paste|MarkedText|ClipboardStore|ClipboardLoad|Bell|ClickOnGrid|MaybeLinkHover|ContextMenu'
git show "$ORIGINAL:app/src/terminal/input.rs" \
  | rg -n 'EditorOptions|Navigate|History|InputDropTargetData|Completion'
git diff "$ORIGINAL" "$BASE" -- \
  app/src/terminal/input.rs app/src/terminal/history.rs \
  app/src/terminal/view.rs app/src/terminal/terminal_size_element.rs
```

`git cat-file` 失败只表示该对象本地不可用，不得把不存在当成已审阅。参考原文件时记录实际采用的符号/行为及新测试 ID。AGPL 原实现只在对应 AGPL 代码区域内迁移，不移入 MIT `crates-local/`。

#### 6.2.3 共用工程约束与接口

**不新增第二个 TerminalModel，不新建通用事件总线。** 模式、会话、焦点、编辑器 revision 能从现有模型读取就直接使用；新状态仅补现有结构不能表达的局部信息。

事件流固定为：平台/PTY 事件 → 已有 Element/ModelEvent → TerminalView/Input → 决策 → 编辑器或 PTY/平台副作用。禁止 OS 事件直接拼 shell 命令执行，也禁止同一输入同时写编辑器和 PTY。

| 输入/焦点情况 | 文本/粘贴接收者 | Up/Down、Enter、Esc 归属 |
|---|---|---|
| 命令编辑器 | 现有 EditorView，单个用户编辑事务 | 编辑器/历史；明确提交才 ExecuteCommand |
| 原生程序或 alternate screen | PTY 输入策略 | 程序终端协议；不被 GUI 历史截获 |
| 菜单/Find/其他编辑器获得焦点 | 实际焦点拥有者 | 不泄漏到终端草稿，不在后台命令完成时抢回 |
| IME composing | 当前拥有焦点的输入组件 | 候选确认不等于命令提交；原生分支只在 commit 时写 PTY |
| 已退出/被替换的 session | 无旧请求副作用 | 旧补全、权限回调、拖入目标全部失效 |

新增异步操作需绑定 `(terminal_view_id, session_id, session_generation, request_id)`；修改编辑内容的结果再绑定编辑器内容和选择区 revision。已有请求计数可扩展，禁止重复造两套失效机制。shell 重建、tab/pane 关闭、历史导航、输入编辑、焦点转移按实际副作用使相应请求失效。

所有 `model.lock()` 内只读取/修改模型并生成小型快照；释放锁后才能更新其他 view、调用系统剪贴板/opener、弹权限提示或发跨实体事件。不能用测试能编译替代死锁路径审查。

建议只在逻辑实际需要时拆出以下**新拟**私有子模块；不是声称文件已存在：

| 新拟模块 | 职责与约束 |
|---|---|
| `terminal/view/local_paste.rs` | 单一粘贴规划函数，返回编辑器插入或 PTY 字节；键盘输入不能绕道此模块 |
| `terminal/view/local_links.rs` | 已解析 Link → 允许的用户动作；opener 可替换，不发预取请求 |
| `terminal/view/local_ime.rs` | 原生终端组合态和候选定位适配；不重写通用 Editor IME |
| `terminal/view/local_clipboard.rs` | OSC 读写授权、大小限制、回调绑定 |
| `terminal/view/local_viewport.rs` | 本地可见范围和高度/滚动锚点；优先复用现有索引 |

提取模块的同时在 `view.rs` 显式 `mod` 挂载。测试使用独立 `*_tests.rs`，禁止仅添加未挂载文件。拟新增测试名统一 `l0_01_...` 到 `l0_09_...`，便于真实 inventory 与过滤器核对；已有测试不为统一命名而删除重建。

本地 debug 日志按 `l0_id / view_id / session_generation / request_id / route / outcome / reason / elapsed_ms` 记录。不得记录命令正文、剪贴板、IME 组合文本、完整 URL、文件真实路径或 OSC payload。测试断言也不得通过把用户私密内容写入公开日志获得证据。

#### 6.2.4 L0-01：历史导航、历史搜索和本地建议

**参考：O1、O3、O4；落点：当前 `input.rs`、`history.rs`、`view.rs::handle_input_event`，以及实际挂载的本地历史/建议 UI。**

实施步骤：

1. 保留当前 `HistoryNavigation`、`Input::navigate_history` 与 History 数据源。回看从当前会话映射的 ShellHost 取得命令；不新建 JSON/SQLite 历史副本，不混入 Agent query，也不跨主机错用命令。
2. 将顺序/去重写成可单测的本地命令策略。既有会话历史语义为准；O4 仅用于对照“重复项保留最新、忽略项、会话顺序”。不能把字符串排序替代历史时间顺序。跨会话是否包含、匹配模式以已保留的设置为准，不静默移除设置。
3. 第一次 Up 固定本次候选快照、草稿和光标；前缀匹配使用进入遍历时的草稿，不随着候选文本变化。Down 越过最新项恢复原草稿/光标，选区不被覆盖；无匹配不改文本。
4. 多行编辑器只在真实导航边界传播历史动作。软换行边界、逻辑行边界与 Vim 上下键分别测试；不能所有 Up/Down 都交给 History。
5. 用户编辑历史结果后结束本次遍历；新的遍历以编辑后的内容为草稿。单纯移动光标的处理与当前编辑器语义一致；异步补全不得覆盖草稿或历史项。
6. 历史文件加载过程中执行的新命令必须保留，复用 History 的加载/合并机制。候选快照不会在一次回看中途重排；下一轮回看可获取新增历史。
7. 本地历史搜索必须提供查询、选择、取消、填回输入框和草稿恢复。若旧 UI 混有云类型，保留现有本地搜索模型、做本地适配，不重新挂载整套旧菜单。选中结果默认填回输入，不隐式执行。
8. 本地建议复用当前 completer；Tab 的唯一命中/公共前缀/循环候选/后缀保留与历史状态互斥，取消或输入改变立即失效。只恢复文件/命令/历史等本地建议，不调用云服务。

| 测试 ID | 输入/操作 | 必须断言 |
|---|---|---|
| L0-01-T01 | Up/Up/Down/Down | 候选顺序正确；草稿和原光标完全恢复 |
| L0-01-T02 | Unicode 前缀、无匹配、非空选区 | 不破坏 UTF-8/选择区；无匹配无变化 |
| L0-01-T03 | 多行首/中/末行与软换行真实按键 | 只在规定边界进入历史；其他情况只移动光标 |
| L0-01-T04 | 会话 A 回看后切换 B | 不复用 A 的遍历状态；ShellHost/会话策略正确 |
| L0-01-T05 | 编辑历史结果，再 Down/Up | 不恢复已废弃的旧遍历状态 |
| L0-01-T06 | 历史异步加载期间执行命令 | 新命令不丢失、不重复、不被旧数据覆盖 |
| L0-01-T07 | 请求补全→开始历史回看→旧结果返回 | 旧结果被丢弃；草稿不被覆盖 |
| L0-01-T08 | 搜索→选择/取消；Tab 候选循环 | 选择只填回；取消恢复；补全保留后缀 |

新增测试扩展 `input_local_tests.rs` 和 `local_interaction_tests.rs`；History 合并/排序测试放在实际模型对应测试模块。既有 6 个 input 新测试继续保留。

#### 6.2.5 L0-02：光标插入与原生终端粘贴

**参考：O1、O5、O8 的 Paste 分支；落点：`Input::insert_text`、`TerminalView` Paste/DragAndDropFiles 分支、新拟 `local_paste.rs`。**

粘贴规划接口表达为以下语义，而非引入新业务框架：

```text
prepare_local_paste(input_snapshot, payload, source, paste_policy)
  -> EditorInsert(text)
   | PtyWrite(bytes, session_identity)
   | NeedsConfirmation(reason, request_identity)
   | Rejected(reason)
```

`source` 区分剪贴板文本、剪贴板文件、Finder 文件；禁止对普通文本进行 shell escape，禁止把文件当 Agent 附件。剪贴板只读取一次。文本尺寸限制沿用现有约束；没有约束的新增入口必须先定义可配置上限并测试，不能无限分配。

编辑器分支：复用 `EditorAction::UserInsert`，在选择区/光标插入；一次 paste 为一个 Undo 事务，保留后缀和换行，不调用 submit/ExecuteCommand。不通过先 `set_buffer_text` 再移动光标拼接来模拟插入。

原生 PTY 分支：从当前 TerminalModel 读取 bracketed-paste 模式。开启时以 `ESC[200~` 开始、`ESC[201~` 结束；关闭时不额外套标记。整个 paste 必须在 PTY 写队列中作为一个逻辑事务排队，禁止首尾标记与另一次按键/粘贴交错。不得把 clipboard paste 当成输入按键逐字符模拟。

默认兼容策略：普通文本保留字面内容；不要为方便测试擅自把换行变为空格。换行转换若已有明确平台/终端设置则保留该策略并单测；没有现有转换契约时沿用当前输入字节，不新加隐式 CR/LF 转换。含 ESC、NUL 或 bracket 终止序列的粘贴进入明确的拒绝/确认路径，默认不写 PTY；界面展示风险类别而不是自动删除字节后悄悄执行。多行且 bracket 模式关闭时使用已有安全确认偏好；缺少该偏好时新入口默认确认一次，用户确认才按字面交给程序。

注意：**“粘贴不执行命令”只可作为命令编辑器的硬断言。** 原生 shell/程序收到换行后的行为由它控制，不能对 mode-off 的原生程序宣称绝不会执行。Term4u 自己不得额外发 Enter/ExecuteCommand。

| 测试 ID | 场景 | 必须断言 |
|---|---|---|
| L0-02-T01 | 光标中部 Unicode/多行 paste | 选区、后缀正确；一次 Undo 恢复 |
| L0-02-T02 | 输入带文件样式但实际纯文本 | 不错误 shell escape |
| L0-02-T03 | bracketed mode on | 精确字节 `200~ + payload + 201~`，仅一组 |
| L0-02-T04 | mode off、普通单行 | 无标记、不额外追加换行 |
| L0-02-T05 | mode off、多行确认/取消 | 取消零 PTY 写；确认仅原定 payload |
| L0-02-T06 | ESC/NUL/嵌入终止序列 | 默认不写；无静默截断/隐藏改写 |
| L0-02-T07 | 空内容、mode 在确认前变化、session 重建 | 空内容零写；过期请求取消，不能发到新 session |
| L0-02-T08 | paste 同时发生输入/第二次 paste | 写队列保持事务边界和次序 |

除 planner 单测外，至少从 `TerminalAction::Paste` 进入、订阅真实 `WriteBytesToPty`；再用受控 PTY 子进程切换模式并记录字节。不可只测试一个与真实分支无关的 helper。

#### 6.2.6 L0-03：输入布局、光标与本地偏好

**参考：O1 的 EditorOptions、O2 的输入布局/Vim/prompt；落点：当前 `Input::new/render`、`TerminalView::after_layout`、`TerminalSizeElement`。**

保留 autogrow、soft wrap、settings line-height、Vim 和 cursor preferences 接线，补齐设置变更订阅及布局验证。禁止把 options=true 当成完成。

使用现有 EditorView 计算输入实际高度；输入增高后输出区真实尺寸变化，PTY resize 只基于完成布局后的有效尺寸，不出现负数/零尺寸抖动，也不因同尺寸重复通知形成循环。输入超高时由编辑器内部滚动，不将输出区无限压缩。

字体、字号、行高、光标形状/闪烁、Vim 状态栏、PS1 与输入定位模式使用当前设置值。PinnedToTop/Bottom/Waterfall 若仍作为保留的可选项存在，必须接到当前 renderer；不得选择只修默认布局而保留其他可点击空设置。

| 测试 ID | 场景 | 必须断言 |
|---|---|---|
| L0-03-T01 | 一行→多行→Undo | 输入增高/回缩，输出区尺寸随之变化 |
| L0-03-T02 | 长英文/CJK/emoji，窄 pane | 软换行与光标位置一致，没有截断/越界 |
| L0-03-T03 | 字号/行高动态修改、重启 | 当场生效且持久化；相同尺寸不重复 resize |
| L0-03-T04 | Vim insert/normal，上下键 | 普通导航不误入历史，键盘行为与编辑模式一致 |
| L0-03-T05 | 光标形状/闪烁、焦点 A→B | 只有实际焦点拥有者按设置显示活动光标 |
| L0-03-T06 | PS1 与所有保留输入定位模式 | 提示符、菜单和编辑器位置正确，设置不是空操作 |

布局/状态测试之外，真实 macOS 检查一次 CJK+emoji、字体切换、分栏缩放；截图不能替代 geometry、caret 和 resize 断言。

#### 6.2.7 L0-04：菜单与焦点所有权

**参考：O8 的菜单/焦点行为；优先继续完善当前 `view/context_menu.rs`、`view.rs`、已有 Menu 组件，而不是恢复旧 view。**

菜单打开时记录有效的返回目标并取得键盘焦点。关闭时仅当当前焦点仍归该菜单/其子 view 所有才恢复；如果菜单动作已打开 Find/编辑器或用户已点击另一 pane，则新焦点优先。返回目标已销毁时回到当前有效 pane，不能使用过期 handle。

Enter 只执行当前菜单项；菜单关闭和业务动作有确定次序，不能关闭后把同一个 Enter 再传播给终端。Esc 关闭但保留草稿。菜单禁用项不得执行。打开第二个菜单时替换旧菜单状态，不留下第二个键盘拥有者。

命令完成、补全返回、wakeup、布局变化和 shell determined 都不能从菜单/Find/其他 pane 抢焦点。焦点判定统一复用现有 FocusContext/句柄，不能维护与框架焦点相矛盾的布尔值。

| 测试 ID | 场景 | 必须断言 |
|---|---|---|
| L0-04-T01 | 打开→Esc | 菜单关闭；草稿不变；返回原合法目标 |
| L0-04-T02 | 打开 Copy command→Enter | 仅复制；ExecuteCommand 计数为 0 |
| L0-04-T03 | 菜单 Find→Enter | Find 保持焦点；关闭菜单不抢回 |
| L0-04-T04 | 菜单/Find 打开时命令完成/补全返回 | 焦点不变 |
| L0-04-T05 | 菜单打开→点另一个 pane→关闭旧菜单 | 另一 pane 保持焦点 |
| L0-04-T06 | 禁用项、无选中项、目标 pane 关闭 | 不执行草稿；无悬空引用；无 panic |
| L0-04-T07 | 菜单上下键、鼠标选择、外部点击 | 路由正确；原有六个鼠标测试继续通过 |

测试必须走 `app.dispatch_keystroke` 并断言当前焦点和副作用；直接调用 `close_context_menu` 的测试只是补充。

#### 6.2.8 L0-05：文件拖入的完整投递链

**参考：O2 的 DropTarget 与 O6 的 OS 事件分发、O5 的路径转义；落点：当前 `terminal_size_element.rs`、`view.rs`、`input.rs`。**

main 已存在 DragFiles/DragFileExit/DragAndDropFiles 入口。重点补全而非重建：

1. 明确区域归属：输出区域、输入区域均可作为本 pane 的文件投递区；菜单等前景控件有优先权。用实际布局 rect 和坐标变换进行 hit-test，不能将子元素内容 bounds 当作整个 pane 的可投递区。
2. `DragFiles` 在本区域内时更新本 pane 的 hover；离开时清理。`Drop` 未命中本区域且子元素没有消费时必须继续冒泡/分发，不能返回 true 吞掉其他 pane 的投递。`DragFileExit` 清理相关 transient 状态，不执行输入。
3. 确认当前 Element 与 InputDropTarget 不会各消费一次，保证一次 OS drop 只产生一次插入。原生/编辑器模式用同一目标选择策略；关闭/切换会话使旧目标失效。
4. 只接受 OS 文件路径列表；多文件逐个按实际 shell 转义，再用空格连接。不要手写只适用于 bash 的引号替换。无法无损表示的路径不得 `to_string_lossy` 后悄悄插入；使用已有可无损转义能力，无法表达则明确拒绝该路径并保留草稿。
5. 编辑器按原光标/选区插入；原生程序复用 L0-02 的 PTY paste 策略。hover 不抢焦点；成功 drop 可聚焦目标 pane，但不自动提交命令。图片文件也是路径，不是 Agent attachment。
6. Start/StopFileDropTarget 驱动真实可见的投递反馈，文件离开、drop、取消、pane 销毁时都清除。

| 测试 ID | 场景 | 必须断言 |
|---|---|---|
| L0-05-T01 | OS DragFiles→DragExit | 反馈出现/清除；输入和 PTY 均无副作用 |
| L0-05-T02 | 输出/输入区与区域边缘投递 | 每个合法区域恰好插入一次 |
| L0-05-T03 | A/B pane，先经过 A 后投到 B | A 不吞事件、不插入；只有 B 收到 |
| L0-05-T04 | 空格、单引号、双引号、CJK、换行路径 | 按 shell 得到原路径；不产生额外命令 |
| L0-05-T05 | 原生程序、bracketed mode on/off | 复用 paste 字节策略，不额外执行 |
| L0-05-T06 | 非文件载荷、空列表、无法无损表示的路径 | 拒绝/忽略有明确定义，不污染草稿 |
| L0-05-T07 | 投递前关闭 pane/切换 session | 过期请求无效果，无 panic |

先走 Element 的真实分发/hit-test 自动测试，再做一次 Finder 多文件拖入的 macOS smoke；当前直接调用 DragAndDropFiles 的测试不能替代前者。

#### 6.2.9 L0-06：输出链接与本地文件打开

**参考：O7 的修饰键规则、O8 的点击/悬浮处理；复用当前模型 `grid_handler::Link` 及文件目标类型，不从零为输出添加另一套正则解析器。**

职责分成“坐标命中/现有模型识别→链接分类→动作策略→opener”。BlockGridElement 和 AltScreenElement 都将模型坐标、block 身份、修饰键传入当前 view；selection drag 与 link click 互斥，拖选结束不能顺便打开链接。

普通 URL、OSC 8 显式链接、本地文件与行/列分别保留。OSC 8 的实际 target 和显示文本不一致时，提示实际 target；绝不按显示文本替换 target。相对文件路径使用产出该 block 的 cwd，不使用后来切换的 cwd；旧 session/SSH 路径不能错误当成本机文件。

macOS Cmd-click 直接执行允许的打开动作；普通点击沿用“展示提示/选择”的交互，不能全部变成自动打开。hover 只改变指针/本地提示，不联网、不 DNS、不读取整文件。鼠标移走、内容变更、block 移除时清理 tooltip，并使旧请求失效。

URL 默认允许 http/https；其他 scheme 只有在保留契约明确且有对应安全实现时逐项支持，不泛化允许任意 scheme。`javascript:`、`data:` 和恢复已删除云入口的 deep link 禁止。文件打开走现有 `OpenFileWithTarget`/代码编辑器事件，不拼接 `open ...` shell 字符串；本地文件不存在时提示，不自动下载。

打开系统浏览器是用户触发的外部动作，不把浏览器流量伪装成产品静默流量，也不能借此恢复内建 Warp 跳转。自动测试使用 fake opener，不启动浏览器或外联。

| 测试 ID | 场景 | 必须断言 |
|---|---|---|
| L0-06-T01 | URL hover、普通点击、Cmd-click | hover 0 次打开；只有规定动作 1 次 |
| L0-06-T02 | OSC 8 显示文本和 target 不同 | 使用真实 target；提示可见真实目标 |
| L0-06-T03 | file:line:column、含空格相对路径、旧 cwd | 正确 FileTarget 与行列；不错误绑定新 cwd |
| L0-06-T04 | 拖选 URL 后抬鼠标 | 不打开链接，选区保持 |
| L0-06-T05 | 危险 scheme、缺文件、不可定位远端路径 | 拒绝/提示，0 shell 执行和 0 外部请求 |
| L0-06-T06 | block 删除/重排、resize、切换 session | 不用旧坐标打开另一个链接 |
| L0-06-T07 | alt-screen 与普通 block 各重复测试 | 两条渲染路径都可用；鼠标报告程序不被误截获 |

#### 6.2.10 L0-07：IME、OSC 剪贴板与本地终端事件

拆成三个独立子项；不能以“已处理 ModelEvent”笼统关闭。

**L0-07A：IME。参考 O1/O8 的编辑器和 MarkedText 行为；落点：平台已分发事件→TerminalSizeElement/TerminalView→原生组合态，通用 Editor IME 继续由 EditorView 负责。**

原生组合态至少表达 Idle/Composing 和当前所属 session。SetMarkedText 仅更新组合显示，不写 PTY；commit 将最终文本写入一次并清理 marked 状态；cancel 清理且零写。不能把 composition 中的 KeyDown 和随后 TypedCharacters 都发送，造成候选确认时重复输入或意外 Enter。

候选窗口定位从真实 caret/grid bounds 转为平台坐标；复用现有平台文本范围转换，不能把 UTF-16 offset 直接用作 UTF-8 切片。跨 pane/关闭/转入菜单时按平台已确认的 commit/cancel 语义结束，绝不把 A pane 未提交组合发送到 B。IME Open 时历史、补全、Enter-submit 的快捷键不能抢候选操作。

**L0-07B：OSC 剪贴板。参考 O8 的 ClipboardStore/Load；落点：现有 ANSI 解析/模型事件与 view 消费者，新拟 local_clipboard。**

写宿主剪贴板和读取宿主剪贴板是不同授权；复用已经存在的本地信任/权限设置，绝不能为了补功能统一设为允许。没有已建立授权时默认拒绝读取；写入默认需要明确允许或一次交互授权。静默/后台场景不自动弹出持续权限请求。

授权以请求和当前 PTY/session 身份绑定；权限回调延迟时原 session 已退出则取消，不发到新进程。使用现有 OSC 解析与响应编码，不在 UI 再写一套半兼容 parser。读取在允许后才调用宿主 clipboard；拒绝/超时按现有协议编码发空响应或终止，但不得返回正文。限制解码后 payload 大小、未完成序列缓冲及排队数；无原限额时本轮建议采用 1 MiB payload、每 session 最多 1 个待授权请求，超限明确拒绝并单测。该数值是本轮新增策略，不声称是旧实现的值。

复合 `Pc` selection、无效 Base64、分段到达、BEL/ST 终止必须沿用 parser 支持规则并覆盖；不支持的目标不得偷偷映射到宿主剪贴板。TUI 不能不经审核把原始 OSC 直接透传给外层终端绕过本轮权限。

**L0-07C：响铃/通知/光标。参考 O8 的 Bell/PluggableNotification/CursorBlinkingChange；落点：现有本地设置与通知服务。**

逐项登记唯一消费者，避免 parser/model/view 双重响铃。按本地设置决定声音、视觉提示和通知；区分前台 pane/后台 pane/非活动窗口，不强制聚焦。高频 Bell 合并/限流；错误仅本地记录，不增加上传能力。光标模式变化影响实际渲染。ImageReceived 在 L0-08 处理；其他 notify-only 事件逐项判定是否已有其他消费者，不能全部重复实现。

| 测试 ID | 场景 | 必须断言 |
|---|---|---|
| L0-07-T01 | IME 多次更新→commit | 更新期 PTY 0 字节，最终文本恰好 1 次 |
| L0-07-T02 | IME cancel、候选 Enter/Esc | 不误提交命令，不重复写入 |
| L0-07-T03 | 中文、组合重音、emoji、跨 pane | 字符/光标范围正确，组合不串 pane |
| L0-07-T04 | OSC write 允许/拒绝 | fake clipboard 只在允许时修改 |
| L0-07-T05 | OSC read 允许/拒绝/无交互能力 | 未授权不读取 clipboard，不泄漏内容 |
| L0-07-T06 | 授权前 session 退出/重建 | 旧响应不写新 PTY |
| L0-07-T07 | OSC 无效编码、超限、分段、不同终止符 | 有界内存、正确拒绝/组帧，不阻塞后续输出 |
| L0-07-T08 | Bell 开关、前后台、连续一百次 | 按设置生效、限流、不抢焦点、不重复消费 |
| L0-07-T09 | TUI 收到 OSC/Bell | 不绕过权限；终端显示/退出正常 |

自动测试需从平台事件/ANSI 字节进入，而不仅手工构造最终 ModelEvent。macOS 实机只补自动化不能证明的系统输入法候选定位/组合行为，不要求用户重跑所有旧场景。

#### 6.2.11 L0-08：块、图片、选择、隐私与滚动性能

**参考：O9 的滚动锚点和高度索引、O8 的本地输出行为；落点：`BlockHeightSummary`、`handle_wakeup`、`render_blocks`、`render_alt_screen`、BlockGridElement/AltScreenElement。**

先明确几何与索引，再恢复 UI：模型 block 身份、原始 grid 坐标、显示坐标、viewport 坐标之间只保留一套可测转换。折叠、soft wrap、clear gap、恢复分隔符、图片高度变化都通过它影响选择、查找跳转和鼠标命中。不能让渲染、Find 和 selection 各自维护一份行数算法。

滚动有两种基本状态：跟随最新输出、固定用户阅读锚点。用户上滚后新输出不拉回底部；回到底部才恢复跟随。折叠或图片高度变化时锚定同一模型位置，不仅保存会随布局漂移的像素值。当前 `render_alt_screen` 的焦点、pane/session 状态和 view id 必须来自真实状态，不能保留固定 true/默认 pane 状态替代判断。

clear/Ctrl-L/Cmd-K 不是同义词：分别确认 shell 发来的清屏序列、原生程序控制键、现有 GUI 清屏 action 的原行为；不得把原生程序的 Ctrl-L 强行截获成 GUI 清屏。清可见区不删除 History 或数据库，clear 后输出、查找、滚动的范围按既有保留行为测试。

恢复本地块导航、展开/折叠、文本选择与复制；图片只消费已解析的本地图片数据，不根据输出 URL 自动下载。图片尺寸、解码内存与缓存设上限，损坏图不影响后续文字，异步解码完成后校验 block/session 身份。

隐私显示设置必须影响渲染；但不能仅因为画面打码就宣称剪贴板/导出已打码。复制原文、复制脱敏文本、日志/导出的策略逐入口登记，沿用明确的旧行为；新安全默认不允许后台导出泄漏正文。用户明确复制原文的能力不得被无声替换为不可用。测试使用合成 secret，不用真实凭据。

性能实施顺序：

1. 对当前全 blocks 构造和 gap 后缀扫描加计数器/基准，先记录真实开销。
2. `clear_gap_rows` 使用一次后缀累计或现有高度索引，避免每个 gap 再扫一遍后续 blocks。
3. 优先复用 `BlockHeightSummary`/现有树索引查可见范围；只构造 viewport 加小范围 overscan 元素，不把 ClippedScrollable 当成已实现虚拟化。
4. 输出追加、折叠、resize、图片完成分别增量更新高度；全量重算只有在确实影响整个布局时使用。避免渲染期长时间持有 TerminalModel 锁。
5. 拆分后运行旧选择/清屏/Find 测试，不靠删除场景换性能。

| 测试 ID | 场景 | 必须断言 |
|---|---|---|
| L0-08-T01 | clear 后执行新命令、滚动与重启 | 可见行为正确；历史和数据库未删除 |
| L0-08-T02 | 跨 block/CJK/换行/矩形选择 | 复制文本与模型范围一致，画面高亮正确 |
| L0-08-T03 | 查找→跳转→折叠/展开→resize | 仍定位同一内容，不跳错行/错 block |
| L0-08-T04 | 用户上滚时连续输出；重新到底部 | 阅读位置保持；显式到底后恢复跟随 |
| L0-08-T05 | 合法/损坏/超限图片，异步完成 | 位置/高度正确，错误局部化，无网络下载 |
| L0-08-T06 | 隐私设置开关、hover、复制、导出 | 每个入口符合已登记策略，没有虚假的全链路脱敏声明 |
| L0-08-T07 | alt screen 选择、鼠标报告、退出恢复 | 不串状态、焦点真实，普通输出恢复正确 |
| L0-08-T08 | 100/1000/10000 个 block，每个 20 行，重复 clear | 元素构造数量随可见区而非总历史线性增长；无 G×N 后缀扫描 |
| L0-08-T09 | 同一候选持续输出、滚动、输入 | 记录输入延迟/帧耗时/CPU/内存及原始数据 |

性能不是本轮已测结论。建议在固定 macOS arm64 机器、相同 build/profile/字号/窗口尺寸下预热后测 3 次，记录中位数与 P95。提出的交互目标为输入可见响应 P95 ≤50ms、滚动帧 P95 ≤33ms；这是待实测的工程预算，不是产品已达成指标。CI 自动门禁优先采用工作量计数/复杂度断言；实际机器未达预算不得直接标 PASS，应定位瓶颈或经明确设计变更调整预算。

#### 6.2.12 L0-09：本地设置、菜单与入口保全

**参考：O1/O2/O8 中纯本地消费者，以及当前 Settings/Command Palette/keybindings；不是恢复所有上游 UI。**

实施时建立机器可读的行为证据清单，结果放 `verification/<SOURCE_HEAD>/l0/local-contract.json`，仅记录对应本节条目、源码位置、测试结果，不另存需求/施工状态。对每个保留项登记：

```text
local_behavior_id → setting/action ID → 注册处 → handler → 状态/渲染/OS结果
                  → 持久化键（如有）→ 测试 ID → GUI/TUI 适用范围
```

至少覆盖历史搜索/本地建议、编辑/粘贴、右键偏好、字体/行高/光标/Vim、PS1/输入定位、块操作、查找、文件打开、剪贴板权限、响铃通知、本地隐私。对窗口/tab/split/本地编辑/文件树等其他 §2.2 契约做受影响回归，不为 L0 重新实施已关闭的数据批次。

入口有而 handler no-op、设置保存成功但渲染不读取、快捷键只在错误 context 生效、重启恢复回默认，都判失败。不能为了“没有空操作”直接删除本来承诺保留的功能；超出本轮能力必须保持 OPEN 并明确说明，不冒充完成。

| 测试 ID | 场景 | 必须断言 |
|---|---|---|
| L0-09-T01 | 逐个保留 action 从菜单/快捷键进入 | 到达同一真实效果；错误 context 不执行 |
| L0-09-T02 | 设置修改→立即观察→重启 | 显示/行为真实变化且持久化 |
| L0-09-T03 | Settings 搜索与 Command Palette 搜索 | 本地项可用；不把云历史/Agent 项重新带回 |
| L0-09-T04 | 输入/渲染各保留选项组合 | 不出现可点击空分支和互相覆盖的设置 |
| L0-09-T05 | GUI/TUI 适用项分别验证 | GUI 测试不替代 TUI；不适用项有原因而非虚假 PASS |

#### 6.2.13 测试设施、批次和执行命令

**新增测试设施只服务本地行为。** 复用 `App::test`、`initialize_app_for_terminal_view`、`add_window_with_id_and_terminal`、`app.dispatch_keystroke` 与现有 PTY 能力，不恢复退役 integration crate。

新拟受控 PTY fixture 只做：设置终端模式、输出固定 ANSI/OSC/OSC8、读原始输入字节、返回结构化记录并可靠退出。必须 raw/noecho、超时、finally 恢复 termios，所有内容为测试数据。它是普通子进程，不访问网络，不读取真实用户 clipboard/文件。测试模式应覆盖 bracket on/off、alternate screen、鼠标报告、OSC read/write、BEL、UTF-8。

通过依赖注入或现有测试平台提供 fake clipboard/opener/通知/时钟。验证副作用次数、参数、当前会话及禁止行为；不能仅断言 notify 发生。OS file-drop 测试从 DispatchedEvent/hit-test 进入，IME 自动测试从平台事件适配层进入。

| 批次 | 内容 | 依赖/退出条件 |
|---|---|---|
| A | 验证现有 13 个新增测试与旧输入/菜单测试；记录当前候选 | 先消除当前编译/格式错误；不能把旧候选 PASS 搬过来 |
| B | 共用路由/请求失效，L0-01/02/03 | 确定编辑器与原生输入边界、粘贴 planner、历史策略 |
| C | L0-04/05/07A | 在 B 上验证真实焦点、拖入及 IME；避免三处各写一种路由 |
| D | L0-06/07B/07C | 链接和系统副作用独立授权，可在接口确定后并行 |
| E | L0-08/09 | 恢复本地输出/设置并跑性能；所有入口与结果清单闭合 |
| F | L0 全量集成和集中 macOS smoke | 关闭 L0 各子项；之后再进入 R3–R6 收敛，不提前宣称 V1 |

开工只记录候选，不重置用户工作树：

```bash
set -euo pipefail
SOURCE_HEAD=$(git rev-parse HEAD)
git status --short --branch
git diff --check
rustc --version
cargo --version
cargo nextest --version
```

第一批使用现有测试名：

```bash
cargo check --locked -p warp --all-targets --tests \
  --no-default-features --features local_only
cargo check --locked -p warp_tui --all-targets --tests
cargo nextest list -p warp --no-default-features --features local_only,test-util \
  -E 'test(terminal::input::local_tests) | test(terminal::view::local_interaction_tests) | test(terminal::input::tests) | test(terminal::view::tests)'
cargo nextest run -p warp --no-default-features --features local_only,test-util \
  -E 'test(terminal::input::local_tests) | test(terminal::view::local_interaction_tests) | test(terminal::input::tests) | test(terminal::view::tests)'
```

本节新增测试实际挂载且命名完成之后，才使用以下过滤器；仅执行这些命令不表示测试已存在：

```bash
cargo nextest list -p warp --no-default-features --features local_only,test-util \
  -E 'test(l0_)'
cargo nextest run -p warp --no-default-features --features local_only,test-util \
  -E 'test(l0_)'
```

执行者必须核对每个 T-ID 映射到了实际 test ID、测试数非零、没有非批准 skip/ignore。表中一个 T-ID 可由参数化多测试实现，不以机械凑用例数量替代行为覆盖。此表共有 66 个测试场景 ID，包含自动事件/PTY/布局测试及性能场景，另有集中实机检查。

每个可合入批次保留完整工程门禁，最终 L0 执行下列全集及 §7 的受影响组合：

```bash
./script/format
./script/format --check
git diff --check
./script/check_no_inline_test_modules
./script/check_network_boundaries --self-test
./script/check_network_boundaries
./script/check_license_boundaries
./script/check_license_config_sync
python3 script/lib/test_inventory_tests.py
./script/test_inventory
./script/presubmit
cargo nextest run -p warp --no-default-features --features local_only,test-util
cargo nextest run -p warp_tui -p ai -p persistence -p warp_terminal -p lsp
cargo test --workspace --doc
cargo build --locked -p warp --bin term4u --no-default-features --features local_only
cargo build --locked -p warp_tui --bin term4u-tui \
  --no-default-features --features offline_hard,standalone
```

`presubmit` 的全部既定 Clippy/格式/测试要求不删；inventory 失败处理单测不能替代真实 inventory。遇到工具缺失标 INCOMPLETE，不能擅自安装/升级 pinned toolchain 或把测试改弱。这里的网络 checker 通过只是一层静态检查，不等于新功能已完成 R6 网络认证。

集中 macOS smoke 仅补以下自动化空白：系统中文输入法的候选位置/确认/取消；Finder 跨 pane 多文件投递；真实字体/光标/缩放；原生编辑器或 CLI 的 paste；真实 TUI 输出/中断/退出。使用隔离数据副本，不要求再次完成无关旧 H1/R0/R1/R2 人工步骤。

#### 6.2.14 L0 关闭条件

每项 L0-ID 具备：当前实现位置、参考 O-ID、实际测试名、执行命令/退出码、候选源码/工具/feature、原始日志和限制。状态分别记录 IMPLEMENTED_NOT_VERIFIED、PASS、FAIL、NOT_RUN、INCOMPLETE；文档或方案完成不能填产品 PASS。

证据写入 `verification/<SOURCE_HEAD>/l0/`：`manifest.json`、测试清单及输出、受控 PTY 记录、合成数据快照、脱敏实机结果、性能原始数据。未提交工作树执行必须同时保存 diff hash；冻结候选时工作树/资源/脚本/fixture 必须和测试输入对应，不能只记录 HEAD 忽略本地改动。

L0 关闭要求：L0-01–09 及 L0-07A/B/C 全部有结果；无保留功能的空 handler/空设置；真实 OS/PTY 路径已覆盖；已知高频交互阻塞清零；旧测试与批准删除基线保持；没有为了恢复本地能力重新挂载云/Agent；日志和副作用不泄漏真实数据。最终 R6 仍对同一最终产品候选完成 C1–C10。

[o1]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/terminal/input.rs
[o2]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/terminal/input/classic.rs
[o3]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/terminal/history.rs
[o4]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/terminal/history/up_arrow.rs
[o5]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/util/clipboard.rs
[o6]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/terminal/terminal_size_element.rs
[o7]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/terminal/links.rs
[o8]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/terminal/view.rs
[o9]: https://github.com/tedczj/term4u/blob/066ec71b736fc3755e29f58f733deadbdac3d1af/app/src/terminal/block_list_viewport.rs

协议参考：XTerm Control Sequences，bracketed paste 与 OSC 52：
https://invisible-island.net/xterm/ctlseqs/ctlseqs.html

#### 6.2.15 本地交互实现及验证结果

**本轮五项失败已消除，整个 L0 仍未关闭。** 以下实现已在本地 macOS Apple Silicon 编译、
运行聚焦回归及完整 presubmit；集中 GUI/PTY 验证结果列于表后。原测试 ID 与断言保留，
未列入本轮实测的场景仍按 OPEN / NOT_RUN 管理。

| 范围 | 本轮实际修改位置及实现 | 验证映射与仍未关闭的内容 |
|---|---|---|
| L0-01 | `input.rs`：历史按最新重复项去重但保留时间顺序；程序替换/清空/追加立即使补全失效；编辑器失焦和 shell 生命周期使旧结果失效 | 保留 `input::tests` 与原六个 local_tests；新增 `l0_01_history_keeps_the_latest_duplicate_without_losing_chronology`、`l0_01_programmatic_draft_changes_invalidate_async_results_synchronously`。后续已补充历史搜索来源/修订绑定、加载刷新和原生 Ctrl-R 路由，见下方历史搜索验证记录；软换行/Vim 边界和全部设置组合审计仍 OPEN |
| L0-02 | `view/local_io.rs`：统一编辑器/原生粘贴规划；bracketed paste 整组单次写入；拒绝 ESC/NUL 等控制注入；mode-off 多行必须确认；1 MiB 限制；请求/会话/块/模式/焦点变化拒绝过期确认；文件剪贴板共用路径插入 | `view::local_io::tests::l0_02_*` 验证 parser 模式、一次 PTY 写、原始内核 PTY 字节、拒绝/取消/过期确认。真实 vim/readline 等子程序行为仍需集中 smoke |
| L0-03 | `terminal_size_element.rs`：输出尺寸与全 pane 文件投递区域分离；不投递零/非有限 resize；拖入提示使用 overlay，不挤压 PTY | 真实字体/偏好/软换行/resize 组合仍 OPEN；不能用上述接线代替布局全面通过 |
| L0-04 | `view/context_menu.rs` 与 `view/action.rs`：关闭后以 deferred typed action 等待焦点队列；generation 阻止旧关闭回调处理新菜单；只在菜单仍拥有焦点时恢复；可返回原 Find | 修正原三项测试的观察时机：`ctx.focus` 是队列效果，先结束 update 再执行原焦点断言，不删除断言；继续使用真实 keystroke 分发。跨 pane/销毁目标完整矩阵仍 OPEN |
| L0-05 | `terminal_size_element.rs`、`alt_screen_element.rs`、`view.rs`：全 pane 唯一文件入口，未命中不吞事件；半开边界；hover/exit 不阻断其他 pane；无效 UTF-8 路径整组拒绝；图片仅作普通路径 | `l0_05_platform_drop_hits_input_and_output_once_but_not_outside` 从 Presenter 平台事件进入；`l0_05_non_utf8_drop_is_atomic_and_never_lossy`。真实 Finder 及完整 split-pane smoke 仍 NOT_RUN |
| L0-06 | 已挂载并适配 `view/link_detection.rs`；普通输出和备用屏幕接回 hover、点击与安全 opener；异步文件识别、历史 cwd/来源、内置编辑器列号转换已接通 | 11 项 `l0_06_*` 与恢复的 5 项原路径测试；macOS 点击、拖选、文件行列和重启复核见下。删除/重排与备用屏幕文件、鼠标报告组合仍待补齐，L0-06 不记全矩阵 PASS |
| L0-07A | 原生组合事件分路、Cocoa 矩形 ABI、UTF-16 选区及延迟关闭会话已修正；普通/备用网格均发布当前帧 caret anchor；编辑器绘制和自动滚动共用映射后的组合子选区 | 多行范围/滚动自动回归及唯一 bundle/PID 的分步提交/取消、跨行长草稿实测见后续记录；旧身份不可靠样本不作为候选结论；系统候选窗精确位置与跨 pane 完整矩阵仍 OPEN |
| L0-07B/C | OSC52 权限及最终写入保护、Bell/注意通知限流、原生光标模式见后续记录；长命令完成通知和绑定本次运行/终端身份的点击定位已接回 | 自动回归及未授权系统回调、TUI 不透传已验证；长命令开关/阈值 UI 已实测；系统授权投递/真实点击以及完整声音、光标实机矩阵仍未关闭 |
| L0-08 | 高度树定位可见 block；构造 viewport 加 3 行 overscan，共用模型 padding/gap/Find/selection 坐标；按 block 身份锚定阅读位置并在隐藏、恢复、过滤和软换行 resize 后重映射 | 块级构造计数、隐藏块锚点、软换行 resize、输出暂停/跟随已自动验证；实机输出锚点、GUI 清屏快捷键/复制高亮、单个巨大 grid、折叠 UI、图片、隐私出口及输入/帧 P95 仍 OPEN |
| L0-09 | README、AGENTS、本文 §3.3/#platform 明确仅 macOS Apple Silicon；多余平台兼容清理列为后续工作 | 保留设置/菜单所有入口消费者审计仍 OPEN；本轮不批量删除平台分支，不扩展其他平台 |

参考代码仍以 §6.2.2 的 O1–O9 固定原提交为准；本轮不以新模块名称冒充上游原位置。

**本地验证结论（2026-09-22）：**

验证基于 `1d2119d732ce79db6292d668477826348d55a8c2` 加本轮源码差异执行；运行时的
未提交工作树由证据中的 `source.patch.gz` 解压内容及 SHA-256 固定，提交/合并后通过源文件一致性核对关联。
在 macOS 26.2 / Apple Silicon / Rust 1.92.0 复现本轮五项失败后完成以下修复：

- `terminal_input_state` 优先识别已激活的备用屏幕，允许 shell 初始化期间的原生程序接收输入；
  退出备用屏幕后仍按原有 bootstrap 状态选择编辑器或运行中命令。粘贴、旧确认失效及 IME
  共用这一判断，不以修改测试 bootstrap 状态避开实际路由缺陷。
- 多行导航测试补齐字形布局夹具：`App::test` 的字体后端返回空字形，原用例因此把上一行末列
  算成 0。复用编辑器已有 `TextFrame::mock`，保留原 Up 光标 3 的断言及真实按键分发，增加
  初始光标 7 和 Down 返回 7 的断言；生产导航算法未改。真实 GUI 显示 `oneX` / `twoY`。
- 修正同批 local I/O / IME 代码的 Clippy 问题，使用已有 `instant::Instant`，简化过期请求
  判断及 cursor anchor 的多余借用；无新增 ignore、删除测试或改动测试删除基线。

最终 `local_only,test-util` 应用测试 1549 passed / 3 原有 ignored，包含全部五项；
`./script/presubmit` exit 0（workspace 4789 passed / 20 原有 skipped，completer v2
131 passed / 4 原有 skipped，三组严格 Clippy、全部格式和 doc tests 通过）；额外
`local_only` Clippy 通过。真实 inventory 通过，当前 4809 项，原批准删除 5089 项。

最终 Debug GUI 经签名校验并使用隔离 profile 实测：初始化前的 raw PTY 收到精确 bracketed
中文多行字节；编辑器 Up/Down 保留多行草稿；原生确认框 Cancel 后收到零字节，Paste 后仅收到
一份完整文本。系统简体拼音在原最终二进制上实测 PASS：可见带下划线的组合文本，提交
“中文”仅增加 6 个 UTF-8 字节，Escape 取消下一段组合后零新增字节。初次实机出现直接发送
字母的现象，诊断确认当时窗口实际输入源为 ABC；全局拼音设置不能代替窗口输入上下文检查。
切换窗口输入源后完成复核。macOS `host_view.m`、`objc/window.m`、`mac/window.rs` 与固定
Warp 原版一致，临时诊断已移除，恢复后的最终二进制 SHA-256 与原受测构建一致。
真实 TUI 交互、其他 L0 项及 R6 全矩阵不在这次关闭范围。

**后续链接恢复（2026-09-23，本地候选验证）：**

本候选基于 `ccb42d69`，继续保持 L0 为 IN_PROGRESS。参考 O7/O8，重新挂载现有
`link_detection.rs` 的文件候选验证与五项原路径测试；URL/OSC 8 仍由 TerminalModel 识别，
未增加另一套输出正则或云依赖。当前改变如下：

- BlockGridElement 与 AltScreenElement 传递实际网格身份、坐标和修饰键；普通点击只选择/提示，
  Cmd-click 或点击提示才打开 http/https。OSC 8 提示显示真实 target，拒绝其他 scheme。
  外部副作用在释放模型锁后执行，自动测试使用平台 fake opener。
- 修复前方未命中网格在鼠标抬起时提前清掉共享拖选状态，以及备用屏幕一帧内拖选仍使用旧渲染
  状态的问题；拖选抬起不会打开链接。原有菜单、焦点、输入回归继续保留。
- 文件扫描在后台运行并可取消；结果核对请求代次、会话、块身份、原候选内容与 cwd。移走、
  resize、会话变化或离开 pane 使旧请求失效；同 pane 内输出点击返回输入框不会误取消请求。
  文件通过既有 OpenFileWithTarget 进入用户所选编辑器。内置编辑器转换为零基列号，
  外部编辑器保持诊断文本的一基列号。
- 块完成保存与窗口快照共用 `TerminalView::serialize_block`，从真实会话或既有恢复标记保存
  本地/远端来源；不再把所有完成块直接标为本地。来源未知的历史块不推定为本地，也不回填旧数据。
  实机已确认新块的来源保存在 SQLite 快照中，重启后历史文件链接仍可打开。
- 五项恢复测试的原 ID 和断言未改名/删除；仅从 `deleted-test-ids.txt` 撤回这五项删除许可。
  不可变 `phase1-before.txt` 未改，删除许可数由 5089 减为 5084，真实 inventory 为 4825 项。

当前候选执行 `cargo nextest run --locked -p warp --no-default-features --features
local_only,test-util`：1565 passed / 3 原有 skipped；严格 local_only Clippy、GUI build、
真实 inventory 均 exit 0。`./script/presubmit` exit 0：workspace 4805 passed / 20 原有
skipped，completer v2 131 passed / 4 原有 skipped，三组 Clippy、Rust/C/WGSL 格式与 doc tests
均通过。前一候选曾出现一次 `test_pane_focus_on_close` 的 nextest LEAK 标记；聚焦复跑及
本候选全量均未再出现，原始运行日志保留，不以该标记冒充内存泄漏结论。

实机使用隔离 profile `l0-links-20260923` 和独立签名 Debug bundle：普通输出及备用屏幕
均显示 URL/OSC 8 的真实目标；普通点击和拖选后本机 HTTP 日志无新增请求，显式点击提示后
各目标只收到一次请求（另有浏览器自身 favicon 请求）。备用屏幕 Return 退出后正常输出恢复。
文件设置搜索可找到编辑器选项，修改为内置编辑器立即生效并跨重启保留；含空格/CJK 路径
`:7:3` 打开后，实际输入 X 得到第 7 行的 `liXne seven`，Undo 恢复，磁盘原文件未改。

原始日志、各命令退出码、候选补丁/哈希和 GUI 测试脚本位于
`/Volumes/未命名/term4u-l0-20260922/`：最终工程日志前缀为 `l0-links-verified-`，
证据索引为 `l0-links-evidence.json`，GUI/HTTP 证据在 `links-gui/`；
原始视觉观察为本线程 CUA 截图，未单独导出图片文件。此证据只覆盖上述场景，不是 L0 或 R6
整体认证。L0-06 的块删除/重排完整矩阵、备用屏幕文件与鼠标报告交叉组合尚未关闭；
L0-01 历史搜索、L0-03/04/05 实机矩阵、L0-07 余项、L0-08/09 仍按原台账继续推进。


[本地原始证据与候选哈希](../verification/1d2119d732ce79db6292d668477826348d55a8c2/l0/local-20260922/manifest.json)
绑定源码 diff、构建产物、命令/退出码和 PTY 捕获；初始失败与中间诊断日志不作为最终 PASS。


**后续历史搜索与输入路由验证（2026-09-23，本地候选验证）：**

在上面的链接候选上继续修复 L0-01。保留现有 CommandSearchView、History 和本地 completer，
不新增历史存储或替换已有搜索 UI：

- 搜索结果绑定发起终端的弱引用、会话、活动块及编辑器内容/选择区修订。接受结果前重新核对，
  不把旧结果填到后来切换的 tab/pane；草稿修改后即使文本恢复原样，也拒绝旧结果。
  普通 Enter 只填回命令，显式执行动作仍使用原有执行入口。
- 搜索已经打开但 History 尚未加载完成时，订阅对应会话的 Initialized 事件，重建本地数据源并
  重跑当前查询，保留用户已输入的查询与过滤器。旧会话的加载事件不会重置当前搜索。
- Command History 的可用状态定义在拥有快捷键的 Workspace 上，使快捷键和 macOS 菜单分发
  使用相同判定。此前只在终端子上下文判定时，父 Workspace 仍能截获 Ctrl-R；已通过菜单
  分发的正反测试复现并修复。原生程序不再被历史搜索抢走 Ctrl-R。
- EditorElement 的文本、IME 与修饰键事件使用当前窗口的实际焦点，不再依赖上一帧
  ViewSnapshot.is_focused。使用完整旧 Presenter 画面，在搜索取消且真实焦点已回到输入框、
  尚未重绘时发送 TypedCharacters，输入仍到达原草稿。旧缓存焦点比较候选在同一测试中失败；
  未修改旧测试断言、忽略项或删除许可。

新增六项实际测试位于 workspace/view_tests.rs 和 search/command_search/view_tests.rs：
`l0_01_history_accept_fills_origin_without_execution_and_cancel_keeps_cursor`、
`l0_01_history_result_does_not_follow_a_tab_switch`、
`l0_01_history_result_rejects_a_changed_then_restored_draft`、
`l0_01_ctrl_r_is_not_captured_by_history_in_a_native_program`、
`l0_01_history_loading_refreshes_the_current_search_query`、
`l0_01_cancelled_search_accepts_text_before_the_next_frame`。
原有历史导航和异步/目录补全测试保留。

当前候选 local_only,test-util 应用测试为 1571 passed / 3 原有 skipped；严格 local_only
Clippy、GUI build、实际 inventory 均 exit 0。完整 presubmit exit 0：workspace 4811 passed /
20 原有 skipped，completer v2 131 passed / 4 原有 skipped，全部既定 Clippy、格式和 doc tests
通过。inventory 4831 项，删除许可仍为 5084 项，原始 baseline 不变。

隔离 profile `l0-history-20260923` 的 GUI 实测：历史搜索可见独立 HISTFILE 的种子命令和
本会话命令；Enter 只填回，SQLite 中两条已执行样本命令的次数均保持 1；取消保留草稿与光标。
本地文件 Tab 菜单可从 alpha 循环至 beta，光标后的 SUFFIX 保留，未执行草稿。
备用屏幕和普通输出模式中的真实 raw PTY 探针均收到精确单字节 `12`（十六进制 Ctrl-R），
不打开搜索。原先 GUI 的单字符 x 观察受到系统拼音组合态影响：日志显示 SetMarkedText 后
ClearMarkedText，不能据此声称普通字符丢失；切到直接输入拉丁字符的输入源后，Escape 紧接 x
正确生成原光标位置的 `echo L0HISTORxY`。旧帧焦点缺陷以独立事件测试为准。

日志前缀 `/Volumes/未命名/term4u-l0-20260922/l0-history-verified-`，证据索引
`l0-history-evidence.json`，GUI 脚本和原始 PTY 字节在 `history-gui/`；
截图为本线程 CUA 原始输出，未另行导出。L0 仍为 IN_PROGRESS：本轮不把 raw PTY Ctrl-R
当作完整 vim/readline 验收，L0-01 的软换行/Vim/全部设置组合、L0-06 剩余矩阵、L0-07
候选位置/OSC52/Bell/TUI、L0-08 渲染/性能和 L0-09 全入口审计均继续保留未关闭状态。


**后续 OSC52 解析边界与 TUI 验证（2026-09-23，本地候选验证）：**

在既有默认 Deny / WriteOnly / ReadWrite 策略上增加解析边界，没有提高默认授权：
OSC52 只接受空 selection（按既有规则视为 c）或单个 c/p/s，拒绝复合/未知目标和多余参数，
不再把 cp/c0 等首字符为 c 的请求静默映射成宿主剪贴板。p/s 仍是原有 Selection 类型，
GUI 不把它们映射为宿主 Clipboard。

新增共享常量约束 OSC52 解码后最多 1 MiB，并在解码前限制 Base64 长度、解码后再次核对长度，
超限不产生 ClipboardStore 事件。VTE 的 std 配置使用动态 OSC 缓冲且没有逐字节 OSC 回调；
Processor 现在限制连续没有 performer 回调的字节为 16 MiB，超限重建 VTE 并丢弃余下序列，
直到 BEL、取消控制或新的 ESC 恢复解析，不派发截断前缀。普通可打印文本与逐字节 DCS/APC
回调不会消耗这一累积预算；DCS/APC 消费者和图片整体资源预算仍属于后续对应验收。

新增八项测试覆盖分段 BEL/ST、复合目标及额外参数、编码与解码边界、未终止超长 OSC、
BEL/新 ESC 恢复、超过 16 MiB 的普通输出继续解析。1 MiB 的合法写入仍成功，1 MiB + 1
被拒绝。保留 VTE 原有 ESC 结束 OSC 的语义，未另写一套终止符解析器。聚焦 L0-07 测试
11 passed；local_only,test-util 应用测试 1573 passed / 3 原有 skipped；严格 local_only
Clippy 与实际 inventory 均 exit 0。完整 presubmit exit 0：workspace 4819 passed（其中 1 项 leaky）/
20 原有 skipped、completer v2 131 passed / 4 原有 skipped，所有既定 Clippy、格式和 doc tests
通过；inventory 4839，删除许可仍为 5084，原始 baseline 未改。
原有纯命令格式测试 `test_vscode_with_line_and_column` 在本轮 workspace 运行中带一次
nextest LEAK 标记，聚焦复跑通过且无该标记；保留两次日志，不把它解释为已证实的内存泄漏，
也不修改测试或放宽门禁。

通过当前 `./script/run-tui --locked -- --help` 构建并准备 standalone/offline_hard 资源后，
在独立 profile `l0-osc-tui-20260923` 和 120×40 的真实 PTY 中运行 TUI。子 shell 执行测试程序，
发送 OSC52 写入、读取和 Bell，再输出 L0-TUI-DONE：外层原始输出没有 OSC52 序列，也没有
测试命令后的 Bell；子程序读取响应为空；DONE 正常渲染，Ctrl-Q 退出码为 0。
TUI 当前使用模型网格重绘、剪贴板 ModelEvent 没有宿主消费者，此次验证的是默认拒绝路径。

工程日志前缀 `/Volumes/未命名/term4u-l0-20260922/l0-osc-`，证据索引
`l0-osc-evidence.json`；`osc-tui/outer.raw` 保存外层原始字节，`outer-text.txt` 是用于
阅读的去 CSI 文本，`response.json` 和 `result.json` 保存响应/退出结果。没有 tmux，使用
tui-verify-change 技能的本地 PTY 后备方法，没有调用云服务或退役登录流程。

L0 仍为 IN_PROGRESS。这里没有关闭 GUI 授权入口、待授权请求/排队上限与过期会话完整矩阵，
也没有把默认静音的 TUI 样本当成 Bell 允许/禁用、前后台及限流全部通过；IME 候选定位、
vim/readline、Finder、焦点/布局、图片/渲染性能与全设置入口继续按原台账验收。


**后续 OSC52 权限入口与请求生命周期（2026-09-23，本地候选验证）：**

Privacy 页增加独立可搜索的 OSC52 设置，使用原有 Deny / WriteOnly / ReadWrite，默认仍为
Deny；标题放在 PageType 标题槽，原静态本地隐私说明保留为另一个 widget。界面说明写入会替换
剪贴板、读写也允许读取，并明确适用于 GUI。不存在逐次授权弹窗，使用用户持久化的明确选择。

ChannelEventListener 现在最多保留 8 个在途 OSC52 请求，在解码前申请队列位置；请求的共享
生命周期标记一直保留到所有消费者和 PTY 回复结束，超限拒绝，不让后台事件队列无限积累内容。
标记同时绑定原事件源与代次：创建 PTY 重置来源、实际切换 shell 会话推进代次，退出/重置及
权限变化令旧请求失效。同会话的常规 prompt 不废弃请求，旧会话迟到的退出不影响当前会话。

剪贴板读取回复不再退化成普通用户输入：ClipboardResponse 经 TerminalView、PtyIntent、
PtyController、Message 一直携带来源标记到 EventLoop。最终写入及每次部分写入前均重新核对；
过期的完整回复或剩余部分被丢弃，后续正常输入保持 FIFO。回复的 Debug 不包含正文。
GUI 消费者仍在访问剪贴板前检查当前权限、来源和终端退出状态，系统剪贴板操作不持有模型锁。

新增 11 项自动测试覆盖：独立筛选、三种设置动作到真实 OSC 消费者的完整路径（fake clipboard）、
20 个排队读取只保留 8 个且消费后恢复、终端重置/退出与迟到事件、不同来源/会话、
已编码回复遇权限撤回、最终 writer 拒绝过期回复、部分写入后的代次变化及回复脱敏。
本轮聚焦 22 passed；local_only,test-util 应用测试 1581 passed / 3 原有 skipped；
严格 local_only Clippy、GUI build、TUI build、实际 inventory 均 exit 0。
完整 presubmit exit 0：workspace 4830 passed / 20 原有 skipped、completer v2 131 passed /
4 原有 skipped，全部既定 Clippy、格式和 doc tests 通过。inventory 4850，删除许可仍为5084，
原始 baseline 不变。

隔离 GUI profile `l0-permission-20260923` 实测：OSC52 搜索仅留下 Privacy 的权限 widget，
默认 Deny；Write only 与 Read and write 均能保存，重启后 Read and write 恢复，最后恢复
Deny 并核对 TOML。真实 GUI 测试只改权限配置，剪贴板读写内容和协议回复通过 fake clipboard
及最终 writer 测试验证。新 profile `l0-permission-tui-20260923` 的 120×40 真实 PTY 复核：
写入/读取 OSC52 不透传外层，读取响应为空，DONE 继续显示，Ctrl-Q 退出码 0。

**实机未关闭项：** Privacy 标题在部分筛选更新、下拉选择或恢复后不显示，重新选择 Privacy
或打开下拉菜单会恢复；相同实例中 Features 的 audible → audible bell 对照未复现。
PageType 的标题和过滤单测通过不能代替这项像素结果。该问题保留为 L0-09 显示差异待查，
所以本轮不宣布权限页面全面验收或 L0 关闭。后续继续处理此项、IME 候选/范围、Bell/通知、
vim/readline/Finder/分屏/字体与 resize、渲染/性能和全设置入口。

后续隔离 profile `l0-title-20260923` 的诊断进一步区分了应用帧与桌面截图：标题消失时，
布局、场景的 7 个字形、图集内容及最终 Metal 参数不变。启用 Metal API/GPU 校验，并在
提交后等待完成、显式同步纹理后读回，22 帧的标题区域颜色和透明度均完整；导出的帧可见
Privacy，但同期 CUA 截图仍缺标题。暂不能据此判定实际屏幕、窗口合成或截图环节中的
故障归属，也不能将其标为 PASS。证据索引为
`/Volumes/未命名/term4u-l0-20260922/l0-title-evidence.json`，包含同步读回帧、日志及诊断补丁。
临时布局/场景/Metal 诊断已移除，没有据此修改产品渲染代码。

日志前缀 `/Volumes/未命名/term4u-l0-20260922/l0-permission-final-`，索引
`l0-permission-evidence.json`，GUI 记录在 `permission-gui/`，TUI 原始字节和结果在
`permission-tui/`。本候选补丁包含两个新增测试文件，不只包含已跟踪文件的 diff。

**后续 IME 范围、光标与原生会话（2026-09-23，本地候选验证）：**

修正 `warp_ime_position` 的 Objective-C 声明/调用：与 Rust 一致按值传递 NSRect，原来错误地
传入指针。Cocoa 客户端保留并返回实际 UTF-16 选区；平台边界转换为字符偏移，处理 surrogate
边界、越界和长度溢出。终端组合光标使用选区终点的显示列宽，不再无条件移至整段文本末尾。
普通输出网格接回唯一活动网格的原生光标，复用既有 cursor_display_point 的裁剪规则；
普通/备用网格隐藏光标时仍更新 IME 锚点，锚点只保留当前帧，避免复用过期位置。

macOS 延迟关闭 IME 绑定组合会话：旧关闭请求不取消其后新开始的组合，但同一旧会话的文本
更新仍会关闭。主线程测试直接实例化隔离的原生 NSTextInputClient，并实际排空 GCD 队列；
该竞态、原生选区、UTF-16 转换及组合光标均保留修复前失败结果。普通网格另用完整 Presenter
事件/绘制路径检查选区移动、隐藏光标和取消恢复，使用已完成 bootstrap 的测试会话。
原有裁剪测试的 `0..0` 改为真实末尾选区 `8..8`，保留原裁剪断言和测试 ID。

新增测试共 5 项；聚焦 27 passed；local_only,test-util 全量应用测试 1582 passed /
3 原有 skipped。两组额外 local_only Clippy、GUI/TUI build 与实际 inventory 均 exit 0。
完整 presubmit exit 0：workspace 4835 passed / 20 原有 skipped、completer v2 131 passed /
4 原有 skipped，既定格式、Clippy 与 doc tests 全部通过。inventory 4855，批准删除仍为5084，
原始 baseline 未改。保留前一次因裁剪测试选区不匹配而失败的 presubmit 日志。

最终 GUI profile `l0-ime-delivery-20260923` 的普通网格和备用屏幕实测：连续执行拼音输入、
空格确认、另一段组合的 Escape 取消及退出，两个真实 PTY 均只收到一份“你好”的 6 字节
UTF-8，取消没有新增字节。分步执行并在中间观察截图的编辑器组合曾有只得到空格的样本。
**后续发现旧测试副本被重新启动且未带隔离变量，与其他副本共用 bundle ID；这些旧 GUI 样本
缺少父进程链，不能可靠归属指定候选，已由下方身份复核替代。** 原始文件保留，不将旧样本
解释为已证实的产品回归。原生候选窗口位置、多行和跨 pane 完整矩阵仍未验收。
临时诊断已移除；L0 继续为 IN_PROGRESS。

工程日志前缀 `/Volumes/未命名/term4u-l0-20260922/l0-ime-closed-`，证据索引
`l0-ime-evidence.json`；最终 GUI 样本位于 `ime-delivery-gui/`，早期失败/诊断样本分别保留在
`ime-gui/` 与 `ime-final-gui/`。TUI 构建不替代系统中文输入法验收。
独立 profile `l0-ime-tui-final-20260923` 在 120×40 真实 PTY 中启动 TUI：外部 Python 子程序
收到精确“中文”参数，原始输出中两个汉字位于同一行第 1/3 列，Ctrl-Q 退出码 0；证据保留在
`ime-tui-final/`。该样本验证直接 UTF-8 输入与输出，不声称验证了 TUI 的系统拼音组合。

后续唯一实例复核：停止意外重开的旧测试进程，保留其全部运行数据；使用独立 bundle ID
`dev.term4u.l0.ime-ring`、profile `l0-ime-ring-production-20260923`，进程和子程序祖先链
均核对到 GUI PID 62751。测试 bundle 与上述无诊断候选的 33 个可加载 Mach-O section
逐项相同，差异仅为隔离身份及重新签名。编辑器的分步拼音提交收到精确“你好”；普通网格和
备用屏幕在中间截图、读取回执后再确认，组合期均为零字节，确认恰好一份“你好”，随后
Escape 取消不增加字节。证据索引 `l0-ime-identity-evidence.json` 保存身份、进程链和阶段回执。
退出后使用进程清单核对，不再对已退出的应用调用会触发重新启动的绑定 API。
Privacy 标题在该唯一实例截图中仍偶有缺失，继续按应用帧/截图差异待查，不据此改渲染器。

**组合内范围与自动滚动（2026-09-23，本地候选验证）：**

本地 drawable selection 先将 IME 字符偏移映射到缓冲区，再转换显示坐标；绘制时不再把偏移
直接加到列，也不再取整段组合末尾所在的行。自动滚动使用相同的子选区，并继续只跟踪已完成
的本地选择，保持待完成鼠标选择的原处理方式。缓冲区仍保留整段 marked selection，用于后续
替换、提交或取消，不改变正文和撤销边界。

新增 3 项回归分别覆盖三视觉行中的前段 caret、显式换行/emoji 的渲染位置，以及水平/垂直
自动滚动；修复前均有失败记录。最终测试从 Presenter 平台 SetMarkedText 事件进入，再检查
范围、绘制或滚动，不只直接调用内部更新方法。local_only,test-util 应用测试 1585 passed /
3 原有 skipped；严格 local_only Clippy、GUI build、实际 inventory 均 exit 0。
完整 presubmit exit 0：workspace 4838 passed / 20 原有 skipped，completer v2 131 passed /
4 原有 skipped，全部既定格式、Clippy、doc tests 通过。inventory 4858，删除许可仍为5084，
原始 baseline 不变。

唯一 bundle `dev.term4u.l0.ime-wrap`、profile `l0-ime-wrap-20260923`、GUI PID 77990 实测：
97 字符命令前缀后的拼音组合跨两行；执行组合内左移、Escape 取消、再次输入并确认后，子程序
收到完整的 68 个前缀 x 加一份“你好”，共 74 个 UTF-8 字节，祖先链核对到该 GUI。
这验证了长草稿保全和最终提交，不把截图中未可靠观察到的候选窗精确位置算作通过。
日志前缀 `/Volumes/未命名/term4u-l0-20260922/l0-ime-wrap-verified-`，证据索引
`l0-ime-wrap-evidence.json`，原始实机回执在 `ime-wrap-gui/`。L0 仍为 IN_PROGRESS。

**响铃、注意通知与原生光标（2026-09-23，本地候选验证）：**

Bell 使用现有声音开关并按终端 250ms 限流；后台注意通知复用
`notifications.preferences.mode` / `is_needs_attention_enabled`。只有 Enabled 且发送终端不是
活动窗口的当前 pane 时才发送桌面通知。OSC 9/777 使用通知声音偏好，Bell 的通知不再播放
第二次声音。桌面通知按终端 1 秒限流，平台错误提示按 30 秒限流；不自动聚焦或在后台请求
系统权限。Features 新增三个独立可搜索的控制项，保留既有复合偏好的其他字段。

终端提供的通知标题/正文在进入 UI 队列前限制为既有的 40/120 个字符；有效 OSC 通知的
解析日志不再记录内容，错误提示也不展示平台错误正文。允许后的 OSC 通知展示程序明确提供
的消息，不读取或自动导出终端历史；这不构成终端所有复制/导出入口的脱敏验收。
macOS 发送接口在成功和失败时都回收 Rust 回调；未配置权限单独返回相应结果。通知交给系统
立即投递，避免连续更新同一标识时反复推迟显示，消费者负责限流。

备用屏幕的 IME 光标锚点改为实际 TerminalView ID；焦点不再固定为 true。原生网格在程序
请求闪烁、全局开关允许且当前窗口/终端有焦点时，以 500ms 半周期请求重绘；输入、焦点和
模式变化重置周期，组合输入期间稳定显示，隐藏光标仍保留锚点。分屏与活动 session 的其余
渲染状态仍需按 L0-08 的真实状态要求继续核对。

新增 9 项自动回归覆盖 ANSI 通知/Bell、100 次限流、前台/后台及不抢焦点、入队前长度上限、
平台错误与回调释放、设置筛选/字段保全、备用屏幕锚点和闪烁策略。另实际运行重绘计时器，
在 Scene 中观察到可见/不可见两个阶段且锚点保留。local_only,test-util 应用测试 1593 passed /
3 原有 skipped；严格 local_only Clippy、GUI/TUI build、实际 inventory 均 exit 0。
完整 presubmit exit 0：workspace 4847 passed / 20 原有 skipped、completer v2 131 passed /
4 原有 skipped，全部既定格式、Clippy、doc tests 通过。inventory 4867，删除许可5084，
原始 baseline 保持不变。

唯一 bundle `dev.term4u.l0.events` / profile `l0-terminal-events-20260923` 实测了独立搜索、
声音偏好保存/重启以及真实 PTY 发送 100 BEL 后继续显示和返回。稳定光标、隐藏、重新显示
模式已操作；连续截图未可靠覆盖完整闪烁周期，不以计时器单测替代该像素验收，也未做声学录音。
GUI PID 18223 的子程序祖先链经核对后，从后台发出 OSC777；未授予系统通知权限时出现
正确的本地提示，焦点保持在 Settings，没有自动权限申请。最终关闭该测试 profile 的通知
意图和声音 Bell，并恢复通知声音偏好；没有修改系统通知权限。

独立 TUI profile `l0-events-tui-20260923` 的 120×40 真实 PTY 验证：OSC9、OSC777、OSC52
以及连续 BEL 不透传外层，剪贴板读取响应为空，中文参数/输出正确，Ctrl-Q 退出码 0。
本轮不将系统允许后的通知展示、通知点击定位或长命令完成通知标为完成；后两者的消费者仍
待补齐。工程日志前缀 `/Volumes/未命名/term4u-l0-20260922/l0-terminal-events-complete-`
仅代表这轮工程检查，证据索引 `l0-terminal-events-evidence.json`；GUI/TUI 原始样本分别在
`terminal-events-gui/`、`terminal-events-tui/`。系统授权后的投递、点击定位和实机通长命令样本仍 OPEN。
L0 继续为 IN_PROGRESS。

**完成通知与点击来源（2026-09-23，本地候选验证）：**

只消费正常 User block 的 BlockCompleted：使用已结束 block 的真实起止时间计算时长，按
`is_long_running_enabled` 与既有 `long_running_threshold` 判断，不与注意通知开关捆绑。
完成通知有独立的限流记录，避免刚发生的 Bell/OSC 注意通知吞掉完成通知；正文使用通用提示，
不导出命令或输出。序列化与时长读取完成后先释放 TerminalModel 锁，再发送通知和事件。
Features 增加独立搜索的长命令通知开关，并显示现有阈值，不改动其他复合偏好。

新通知携带本次运行的随机标识和 TerminalView ID。系统点击回调通过同一入口查找当前窗口、
工作区和仍在 tab 列表中的 pane，使用现有 typed FocusPane 路径切换 tab、聚焦 pane 并显示窗口。
旧运行、关闭目标、不存在的终端和无法证明运行身份的旧 BlockOrigin 数据均不触发定位。
保留旧数据的反序列化形状，不把可跨进程复用的数字 ID 当作有效来源；此处没有删除历史数据。

新增 2 项回归从 ANSI CommandFinished 或实际通知响应入口进入，覆盖短/长命令、独立开关、
三个真实完成事件、释放模型锁、旧运行/旧格式/缺目标/关闭目标及当前 tab/pane 定位；既有
注意通知与设置字段保全测试一起通过。local_only,test-util 应用测试 1595 passed /
3 原有 skipped，严格 local_only Clippy、GUI build、实际 inventory 均 exit 0。
完整 presubmit exit 0：workspace 4849 passed / 20 原有 skipped、completer v2 131 passed /
4 原有 skipped，全部既定格式、Clippy、doc tests 通过。inventory 4869，删除许可5084，
原始 baseline 未改。

隔离 bundle `dev.term4u.l0.notification-routing`、profile `l0-notification-routing-20260923`
的 GUI PID 42354 已验证长命令设置搜索只保留一项、显示配置的 1 秒阈值，关闭后正确保存，
阈值与注意通知关闭值保持不变，随后恢复完成通知开关。系统通知主开关仍为 Unset；已提出
测试应用的 macOS 通知权限授权问题，未收到确认前不授予权限。真实投递/点击样本尚未执行，
准备好的子程序未启动；不以自动点击入口测试替代系统投递验收。
日志前缀 `/Volumes/未命名/term4u-l0-20260922/l0-notification-routing-`，证据索引
`l0-notification-routing-evidence.json`。L0 仍为 IN_PROGRESS，其他实机矩阵继续推进。


**可见区构造、高度索引与阅读锚点（2026-09-23）：**

`render_blocks` 按 `BlockHeightSummary` 定位窗口与上下各 3 行 overscan，只复制相交的 grid；
跳过零高度历史子树，以空白元素保留完整滚动高度。已删除视图独立的 gap 后缀扫描；总高度、
网格 padding/偏移和 Find 跳转使用模型索引，与 selection 的 BlockListPoint 坐标一致。
隐藏/恢复历史块和过滤高度更新复用增量高度处理及 clear gap 调整；保留 live block 更新断言。
alt screen 的 pane/session 状态取自 PaneFocusHandle，不再固定为默认/Active。

固定 800×600、80 列、每块恰好 20 个显示行，分别测 100/1000/10000 块、0/1/2 次 clear、
顶部/中部/底部，每组 3 次。10000 块旧版构造 20000 个 grid，新版最多 5 个、访问最多 3 个
索引项；27 个样本中位数分别为 429690µs 与 89µs，P95 为 434465µs 与 130µs。
这是构造/释放元素的测试耗时，不能替代输入可见延迟、GPU 帧、CPU/内存或 §6.2.11 实机预算。
原始样本与旧/新源码快照存于 `/Volumes/未命名/term4u-l0-20260922/`，索引前缀
`l0-viewport-`；初次非固定行宽基线由 `l0-viewport-baseline-20-rows` 严格夹具结果取代。

隔离 GUI `dev.term4u.l0.viewport` / profile `l0-viewport-20260923` 已核对 PID、环境和命令
父进程链：显示 250 行中英文输出、上滚中段、Find 定位/高亮 ROW-0040、shell clear 后执行新
命令，以及 clear 后再次找到旧内容。最终构建 PID 67957 重启同一隔离 profile 后再次恢复并
定位 ROW-0040。Cmd+K 未对应本仓库 clear 快捷键，相关画面作废；不把
截图省略未变化内容当作清屏行为。GUI 清屏菜单/快捷键及跨行选择高亮仍需独立验收。
新增 5 项回归覆盖构造量、隐藏索引、Find 坐标、非活动会话鼠标报告及 Clear hook→命令完成。
最终 local_only,test-util 应用测试 1600 passed / 3 原有 skipped，严格 local_only Clippy、
GUI build、实际 inventory 均 exit 0。完整 presubmit exit 0：workspace 4854 passed /
20 原有 skipped，completer v2 131 passed / 4 原有 skipped；原始 baseline 9777、删除许可
5084 未改，实际 inventory 4874。证据索引为 `l0-viewport-evidence.json`。
新增锚点会按稳定 BlockId、grid 和原始文本位置保存读者所在行；隐藏更早 block、恢复、soft-wrap resize
期间用 GridHandler 既有平铺文本偏移转换回映射位置，保留行内偏移。锚点被截断/删除时回落到原滚动值，
不会解析成重排后的另一个 BlockIndex。追加输出测试确认用户上滚 0.75 行即暂停跟随，明确到底后
恢复跟随。新增 5 项锚点回归覆盖隐藏/恢复、双向软换行、空行、宽字符、索引截断/无效位置和暂停跟随。
最终 local_only,test-util 应用与 warp_terminal 测试 2157 passed / 5 原有 skipped；严格 local_only
Clippy、GUI build、实际 inventory 均 exit 0。完整 presubmit exit 0：workspace 4859 passed /
20 原有 skipped，completer v2 131 passed / 4 原有 skipped；inventory 4879，删除许可 5084，
原始 baseline 9777 未变。锚点批次原始日志与源码快照在 `/Volumes/未命名/term4u-l0-20260922/`，
索引 `l0-scroll-anchor-`。这些是自动模型回归；上滚、追加、窗口 resize 的 GUI 实机读线保持仍未验收。
单个巨大 grid、折叠 UI、图片、隐私出口、清屏入口与最终 R6 同一候选矩阵继续 OPEN，L0 仍为 IN_PROGRESS。

### 6.3 R3：UI、认证、动作和 TUI

先完成 L0，再按 Settings/搜索 -> Command Palette/keybindings -> 菜单/workspace -> URI/deeplink
-> auth 值类型 -> TUI 收尾。先确认模块与消费者，再删云孤儿/flag/快捷键；不把本地处理器一并掏空。
`settings_view/tab_menu.rs` 的 CloudModel 是未挂载孤儿，不是当前编译错误；本地 persistence/block_list
已替换旧 Agent 导入，不重做该修复。

对 drive/team/billing/api key/warp agent 搜索及对应 deeplink 做拒绝/不可见测试，对本地设置、文件、
终端操作做成功测试；无灰按钮、登录壳或幽灵搜索。Privacy 固定关闭云传输，保留真实本地设置。
TUI `/` 保持 shell 字符，不增 slash 云菜单；固定宽度测试与真实 PTY 均必验。
出口是源码/注册表、正反向测试、真实 GUI/TUI 证据完整，不只是云字符串搜索零命中。

### 6.4 R4：内建外链与 Windows 尾项

实现 script/check_external_product_links，提供允许/拒绝样例、--self-test、扫描出错失败；接入
presubmit 的 Clippy 之前。区分内建上游跳转与用户 URL；保留项逐条记录路径、触发方式、理由、批准人。
审计 app/warpui/warpui_extras/prevent_sleep/warp_terminal/warp_util 的 Windows 文件、模块与 target，
剩余严格等于 §3.3 例外；清理失去消费者的 embed-resource/dunce 等，混合 target 不误删 macOS。
出口：network/external checker 与 self-test 通过、外链和 Windows 审计表完整。

### 6.5 R5：依赖、供应链与删除清单

执行 §4 和 §7.3 全闭包审查；script/deletion_set.txt 表示累计实际删除范围，不是单批设计副本。
分类工具只给 A/B/C 候选，不替代编译或人工授权。实际 test_inventory 对原始基线计算消失集，必须
与批准集精确相等；新 ID 与行为覆盖另审。失败不能生成空清单或覆盖快照。
禁止用 ignore/dead_code/弱断言/blanket allowlist/删保留行为测试获得 PASS；不恢复已退役 integration。

### 6.6 R6：最终同一候选验收

全部修复后冻结源码、manifest、lock、脚本、测试、样本、资源到 SOURCE_HEAD；执行 §7 全矩阵并满足
C1–C10。输入变化重新冻结并跑受影响验证及最终门禁，不拼旧批次 PASS。manifest 只索引结果和设计 ID。

<a id="verification"></a>
## 7. 完整验证程序

从仓库根目录执行，记录命令、UTC、平台、工具、退出码及原始输出；管道 set -euo pipefail。
未执行 NOT_RUN，环境不足 INCOMPLETE，产品失败 FAIL。以下命令列出不代表通过。

### 7.1 预检、编译与 focused 全集

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

presubmit 保留 workspace（排除单独测的 warp_completer）、default GUI、default completer 三组严格
Clippy，以及 clang-format、WGSL、workspace nextest（排除 command-signatures-v2）、completer v2、
doc tests。inventory 失败处理单测不代替真实 ID 全集。TUI integration/bench 是额外门禁，不能省略；
不使用不支持的 --all-features 代替明确组合。

### 7.3 结构、源码与依赖负证明

在同一 Bash 会话按序运行，扫描失败不能当无匹配：

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

另归档 auth/UI/actions、Windows/target、内建外链、feature、下载脚本、迁移 diff 和测试 ID 审计。
文本扫描不能替代运行/依赖闭包或 L0 本地行为保全，opaque JSON/旧表名不等于云实现。

### 7.4 发布产物与资源

沿用 E/no_match，扫描实际发布候选；记录 GUI/TUI 与 bundle 全资源 hash：

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

实际 target_directory 以 metadata/构建配置为准；用了自定义目录须在以上扫描命令中一致替换，
不可扫描旧 target 下的其他产物。真实假阳性逐条记字符串、来源、理由、批准、原始命中和过滤结果；
不整体忽略 warp/mcp/resources/组织。签名校验不等于 Apple 公证，身份/scheme/keyring 单独验证。

### 7.5 GUI、TUI、数据与本地行为

| ID | GUI 场景 | 最小观察 |
|---|---|---|
| S1 | 冷启动 | 正确身份，无登录/云启动依赖 |
| S2 | tab | 新建、切换、关闭、Undo 及位置 |
| S3 | split | 新建、调整、退出一个 shell 不丢其他 pane |
| S4 | 命令与输入 | 输出、Ctrl-C、历史、补全、PS1/Git、clear/回滚及 L0 输入/粘贴 |
| S5 | 文件树 | 打开目录/文件和局部状态恢复 |
| S6 | 预览/编辑 | 编辑保存、未保存取消关闭、搜索/diff |
| S7 | 设置/菜单 | 本地项真实可用、云菜单/搜索/URI 不可用，菜单按键不泄漏到终端 |
| S8 | 焦点 | 点击回输入、拖选保留、光标可见/闪烁；原用户 H1 即该项的既有批次确认 |
| S9 | 睡眠/唤醒 | 会话与 UI 保留，无后台重连/重试 |
| S10 | 关窗/恢复 | 注册表、tab/pane/左面板模式和宽度 |
| S11 | 退出/重启 | 历史、notebook/workflow、本地编辑及窗口状态往返 |
| S12 | debug Rust panic | 真实 panic 写本地日志，无上传；不能用 SIGSEGV 替代 |
| S13 | 空闲十分钟 | 无外连/DNS/重试/待发文件，UI 和退出正常 |

逐项验证 §2.2/§6.2，尤其 workflow 参数与项目加载、notebook 换行/重启、code review/Undo、
UI ZIP CRC、本地 env 展开、skills 字节一致性。LSP 五种类型分别测 PATH 命中/缺失/不可执行；伪
server 记录参数，伪 curl/wget/npx/npm 下载日志为空。旧 DB/fixture 副本迁移、写入、重启后对照字段。

TUI 在真实交互 PTY 运行 ./script/run-tui，保存屏幕文本，验证输出、Ctrl-C、tab/新 tab、外部 CLI、
退出。render-to-lines 是补充，GUI 不代替 TUI；不恢复退役 integration。增量截图缺字与实际画面
缺字要区分，不凭不可靠截图调整渲染器。原批次不重做不代表新修复或最终 R6 无需回归。

### 7.6 网络与第二道防线

隔离 HOME/profile、干净 shell、恶意代理下运行候选 GUI/TUI，各至少 60 秒，另覆盖 S13。先验证
采集权限和受控流量：包、系统 DNS client PID/request ID、短命子进程三条链路可关联。
script/capture_macos_network.py 由用户授权终端手工 sudo 运行，应用另以普通用户启动。
COLLECTED_NOT_YET_VERIFIED 只是采集完成，不是 PASS。

捕获启动到正常退出及收尾窗口的主进程和完整派生树，包含 exec/PID 版本；核对事件序号、解析错误、
内核丢包、采集退出码、全部派生进程退出。检查 TCP/UDP/WebSocket、DNS53/853、代理解析、后台重试
和待发遥测/崩溃文件。空文件必须结合校准/归因/无丢包/正常收尾判断，不能只 grep 进程名或进程组。
零外连是零非 loopback 请求，不是请求被守卫拒绝后没有连接成功。

系统防火墙规则须有效并保留空拒绝日志，改规则需授权。静默场景不跑联网命令，用户 PTY 联网另测。
原始系统 ES/网络日志可能含其他进程环境/参数，只留本机受限目录，不公开未经脱敏的全文。

<a id="acceptance"></a>
## 8. 验收条件 C1–C10

这是唯一最终验收定义。当前各项未整体关闭，§5 原批次关闭不可直接填作最终 PASS。

| ID | 完成条件 | 证据 |
|---|---|---|
| C1 | GUI/TUI/关联 crate 的 default/local_only/test-util 组合通过 | §7.1/7.2 check/build、命令/退出码与身份 |
| C2 | 终端/Ctrl-C/tab/split/window/编辑/文件/搜索/diff 及本地交互保留 | 本地测试、L0 关闭、GUI S1–S11、真实 TUI |
| C3 | workflows/notebooks/env-vars 与旧 DB 可读写恢复 | fixture/字段对照/迁移审计/写入/重启 |
| C4 | 云 UI/auth/Agent/Drive/共享及动作死引用清零，本地设置/TUI 正常 | 正反测试、注册表/源码与可见结果 |
| C5 | PATH LSP、skills/日志完整，外链和 Windows 关闭 | 五种 LSP 分支/零下载、日志三场景、资源/外链/平台审计 |
| C6 | 云 crate/协议/ServerApi 全闭包无残留且供应链明确 | metadata、GUI/TUI tree、source/lock、pin、显式 git 来源、cargo deny |
| C7 | 无未批准测试消失、保留覆盖未弱化、完整工程门禁通过 | inventory/ID授权、累计删除集、format/clippy/presubmit/workspace/doc/额外组合 |
| C8 | release/bundle 无未批准黑名单，静默零外连/DNS/重试/待发文件 | hash、strings/resources、归因捕获、S1–S13、防火墙 |
| C9 | macOS arm64 全部最小矩阵通过 | 同一源码工程/实机/数据/网络/签名/身份/scheme/keyring |
| C10 | 原批次保持关闭、L0/R3–R6 收敛且证据可追溯，不把缺环境/WIP 当完成 | 同一 SOURCE_HEAD manifest、输入hash、初始/最终diff/status、本文状态 |

全部通过后方可把 V1 标为完成并关闭 V0 严格验收，不另维护第二个 V0 manifest。M7/V2/MIT 不自动完成。

<a id="evidence"></a>
## 9. 证据与变更纪律

需求、架构、状态和施工只在本文维护；README 是入口，AGENTS/CONTRIBUTING 是工程协作规则。
不创建日期版替代设计、handoff、milestone 或 todo。过期说明和孤儿日志/截图/临时脚本清理，历史查
Git；真实测试输入和产品资源按消费者保留。复核 JSON 仅为本文引用的原始证据索引，不新增任务表。

新原始输出归档 verification/<SOURCE_HEAD>/，机器可读 manifest 至少记录源码/设计版本、工具/平台、
feature/build 参数、输入/产物hash、C-ID、命令、UTC、退出码、日志路径、观察/限制和结果状态。
不得以模型总结替代原始日志，公开前脱敏。报告不要求写自己的提交hash；证据提交不得改变验证输入。
候选变化时按新输入验证，不能自动套用旧 PASS，也不为文档-only归档反复让用户完成同一人工步骤。

保护用户工作树/真实数据/.pi，不为干净状态 reset 或删用户文件。commit/push/PR/merge/tag/release/
系统安装/特权采集按各自授权执行；PR授权不等于merge。源码含新修复时必须跑新回归，未执行就保持
Draft/NOT_RUN，不降低 AGENTS 的 format/Clippy 和最终工程门禁。

继续开发指令：

```text
按 docs/DESIGN.md 先完成 L0 本地功能保全，验证第一批输入/历史/菜单修复，并补齐链接、原生粘贴、文件拖入和本地终端事件。非云/非 Warp Agent 功能不因旧目录混合而删除。随后完成 R3、R4/R5、R6，满足全部 C1–C10；只在本设计更新状态。原 R0/R1/R2/原 GUI-shell 不重做施工，新的回归不能冒充已关闭。保留用户 PTY CLI、本地数据/日志/LSP，不恢复云空壳或退役 integration；提交/特权/发布按明确授权。
```

<a id="release"></a>
## 10. V2 发布与后续演进

### 10.1 M7 / V2

V1 关闭后，按同一设计处理 macOS 品牌资源：SVG/logo/About/channel图标/安装图片/Dock插件/字体字形，
先查消费者，再用独立 Term4u 资产替换；不再次改变运行时身份。README/贡献/安全/行为准则/authors/
联系入口统一项目身份，保留上游来源、基线和版权，不暗示上游背书。
生成许可证地图、MIT 岛声明和 NOTICES，保留 AGPL 分发材料。清理依赖上游 secrets/服务的 CI 与发布
脚本，建立可复现 macOS Apple Silicon（仅 `aarch64-apple-darwin`）打包、签名/公证策略、
源码与产物对应；不新增 Intel/Rosetta/Universal Binary、Windows/Linux/WASM 发布路线。
发布前跑完整 V1 回归及品牌/名称/资源/许可证检查，经独立授权才创建 tag/release；不加自动更新。

### 10.2 可选完整 MIT 重实现

只有 V1/V2 完成且另行批准后进入。按本文保留行为写黑盒规格，审计两座 MIT 岛的直接/传递依赖，
用独立实现或经核验的第三方替换 AGPL；不搬运 AGPL 到 MIT。保留来源、许可和实现过程记录。
未来确需 agent_protocol/local_agent_bridge/local_fs_service 再在本文定义协议、权限、进程/文件边界
与测试，当前不建空 crate、不恢复 Warp 云 API。MIT 岛存在不是最终应用已经 MIT 的证据。
