# Term4u 完整设计与实施基准

> 唯一设计、状态与施工依据；更新：2026-09-14。
> 本轮分支审计：`dev-202609014` / `65a012a7ac344e05c7e09c51a253cbc661e5ac35`。
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

只支持 macOS，必验 macOS arm64；不承诺 Linux、Windows 或 WASM 产品。Windows 专属实现和
依赖仍须清理并保留明确共享例外；Linux 专属代码/依赖/CI/打包后续清理，不把取消验证写成 PASS。

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

### 3.3 平台与资源

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
| L0 本地交互保全 | IN_PROGRESS | 输入/历史/粘贴/设置/菜单第一批代码及 13 个新增测试已写入本变更；未在 macOS 编译运行，不能标 PASS；完整缺口见 §6.2 |
| R3-F1 菜单焦点 | FIX_IMPLEMENTED_NOT_VERIFIED | 打开聚焦 Menu、统一关闭并有条件恢复焦点、避免完成事件抢焦点；Esc/Enter/Find 测试待运行 |
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
本轮编辑环境没有 Cargo/macOS，也无法完整克隆；代码做了静态路径/API核对和差异检查，新增 Rust 测试
尚未运行。缺环境为 INCOMPLETE，不因旧 candidate 曾 PASS 就将新修复判完成。

### 6.2 L0：先保住本地交互

| ID / 优先级 | 发现、实现位置与下一步 | 验收出口 |
|---|---|---|
| L0-01 / P1 | input.rs 忽略 Editor Navigate，上下历史未接入；本变更接回现有 History、会话内前缀回看、草稿/光标恢复，保持多行内部导航 | 真实 Up/Down 路由、跨会话、编辑后退出回看、无匹配/选区保护、新输入不被过期补全覆盖；历史搜索/本地建议仍需审查，不以基础回看替代全部历史 UI |
| L0-02 / P1 | append_to_buffer 把输入追加末尾；本变更将用户插入/终端粘贴/选择插入切换到 editor UserInsert，使用现有 shell 路径转义 | 光标中部、替换选区、Unicode、多行、Undo、后缀保留且不自动执行；原生程序的 bracketed paste/控制字符策略另行补齐 |
| L0-03 / P1 | 通用 EditorOptions 默认关闭本地终端需要的 autogrow/soft_wrap/行高/Vim/光标偏好；本变更接回 | 多行导航、长行布局、偏好变更和 Vim 输入/普通 PTY 分支回归；不能以 options 已设置冒充实机布局通过 |
| L0-04 / P1 | 即 R3-F1；菜单没有接管键盘焦点，关闭未统一恢复；本变更打开聚焦 Menu、关闭按所有权恢复，命令完成不抢菜单/Find 焦点 | 实际聚焦状态下 Esc/Enter/Find 的按键路径测试；Enter 不能执行终端草稿；鼠标已有六测试继续保留 |
| L0-05 / P1 | DragAndDropFiles 原为空操作；本变更补动作处理和 shell 转义插入；外部文件拖入 hit-test/DropTarget 完整链路尚未确认 | Finder 多文件、空格/引号路径、输入中部/原生程序、跨 pane；现有新增动作测试不代替 OS 投递测试；不恢复 Agent 图片附件 |
| L0-06 / P1 | has_highlighted_link 固定 false，ClickOnGrid/MaybeLinkHover 等仍为空；未在本变更修复 | 接回普通 URL/OSC8/本地文件行号的解析、悬浮和用户点击；以替身 opener 断言点击才打开，恶意 scheme/无点击不触发；保留外链守卫边界 |
| L0-07 / P1 | ClipboardStore/Load、Bell 等只 notify；MarkedText 分支为空；这证明处理链缺口，尚不等于所有输入法场景均失败 | 对模型事件逐项补消费者与权限；OSC 读取不得无条件开放；IME 组合/提交/取消分开测，使用受控 PTY/事件注入，不能整包恢复旧 view |
| L0-08 / P1 | 简化渲染曾固定本地显示策略；本变更接回现有 get_secret_obfuscation_mode；块/图片/富内容/滚动和选择仍需保全审计 | 本地隐私设置应作用于画面；进一步核对检测/复制/导出路径。补图片、块导航/折叠、clear/Ctrl-L/Cmd-K、alt-screen 选择与大输出性能的正向验证 |
| L0-09 / P2 | 设置/UI 入口、历史搜索/建议及其他本地菜单尚无全面等价审计 | 对 §2.2 逐项登记源码消费者与测试；入口有而 handler 无操作、设置有而 render 不使用均为失败；无证据项不得写“未丢失” |

本变更只完成第一批接线，不宣称 L0-01–09 全部修好。新增 `input_local_tests.rs` 六个、
`local_interaction_tests.rs` 七个测试；原 input_tests/local_view_tests 不删、不改断言。
新测试覆盖插入/撤销/选择、历史草稿/会话、真实方向键、多行、粘贴不执行、菜单 Esc/Enter/Find、
文件路径动作。尚缺的 OS 拖入、原生 PTY paste、链接、IME/OSC/响铃及渲染测试必须补齐。

首轮自动命令（过滤器必须实际命中；最终仍跑完整集合）：

```bash
cargo nextest run -p warp --no-default-features --features local_only,test-util \
  -E 'test(terminal::input::local_tests) | test(terminal::view::local_interaction_tests) | test(terminal::input::tests) | test(terminal::view::tests)'
./script/format
./script/format --check
./script/test_inventory
./script/presubmit
```

未增加泛化人工清单。原 H1 已完成；新问题优先补自动事件/PTY/渲染测试。只有自动化不能证明的
具体实屏行为才安排一次集中 smoke，注明候选和未覆盖点，不让用户再重跑无关 R0/R1/R2 手工场景。
L0 已知阻塞关闭后才能继续按 R3–R6 宣告产品收敛。

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
脚本，建立可复现 macOS 打包、签名/公证策略、源码与产物对应；不新增 Windows/Linux 发布路线。
发布前跑完整 V1 回归及品牌/名称/资源/许可证检查，经独立授权才创建 tag/release；不加自动更新。

### 10.2 可选完整 MIT 重实现

只有 V1/V2 完成且另行批准后进入。按本文保留行为写黑盒规格，审计两座 MIT 岛的直接/传递依赖，
用独立实现或经核验的第三方替换 AGPL；不搬运 AGPL 到 MIT。保留来源、许可和实现过程记录。
未来确需 agent_protocol/local_agent_bridge/local_fs_service 再在本文定义协议、权限、进程/文件边界
与测试，当前不建空 crate、不恢复 Warp 云 API。MIT 岛存在不是最终应用已经 MIT 的证据。
