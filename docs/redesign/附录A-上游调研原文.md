# 附录 A · 上游调研原文归档

本附录有三部分：

| 部分 | 内容 |
|---|---|
| [A.1 审计修正对照表](#sa-1) | 调研原文与前序计划中，被本轮审计推翻或修正的**14 处**，逐条给出证据 |
| [A.2 覆盖矩阵](#sa-2) | 调研十七节 + 最终模块处置表 → 本文档集各章的映射，逐项打勾 |
| [A.3 调研原文（逐字归档）](#sa-3) | 原文归档 |

---

<a id="sa-1"></a>
## A.1 审计修正对照表

> 规则：**调研原文与本审计冲突时，以审计为准**，并在此逐条登记，不静默覆盖。
> "证据"列的命令与行号均可复现。

| # | 调研原文 / 前序计划的说法 | 实测 | 证据 | 对设计的影响 | 落在哪章 |
|---|---|---|---|---|---|
| **C1** | 遥测宏与 trait 可以先改空、**之后删除** | **不能删。** 删 `crates/warp_core/src/telemetry.rs` = 改 **212 个文件 / 888 处** | `grep -rlE 'send_telemetry_from_ctx!\|send_telemetry_from_app_ctx!\|send_telemetry_sync_from_ctx!\|send_telemetry_sync_from_app_ctx!\|record_telemetry_from_ctx!\|record_telemetry_on_executor!' app crates --include='*.rs' \| wc -l` → 193；`register_telemetry_event!` → 24；去重合计 212 文件 / 888 处 | 永久保留 no-op shim（改动量 1 文件），只删 collector 与 Rudder 传输 | [03 §9](03-阶段1-云模块删除与离线化.md#s9)、[07 §7.6](07-测试与验证策略.md#s7-6) |
| **C2** | `crash_reporting` / `cocoa_sentry` / `autoupdate` **在 default feature 里**，需要"关掉" | **都不在 default**（含传递闭包） | `app/Cargo.toml:467`(`autoupdate`)、`:495`(`crash_reporting`)、`:487`(`cocoa_sentry`)；`default` 在 `:510-713`，203 项闭包中无此三者 | 不必"关"，直接删代码即可 | [02 §6.3](02-现状审计.md#s6-3) |
| **C3** | `bundled_skills` **不在 default** | **在 default**（`app/Cargo.toml:620`） | 同上 | 默认构建会打包 Warp 官方 skill，**必须显式处理** | [02 §6.3](02-现状审计.md#s6-3)、[03 §1.5](03-阶段1-云模块删除与离线化.md#s1-5)、[08 RK11](08-实施顺序与里程碑.md#s3) |
| **C4** | 存在 `ambient_agents` feature | **不存在。** 实际是 5 个不同名的：`ambient_agents_command_line:767`、`ambient_agents_image_upload:768`、`scheduled_ambient_agents:769`、`sync_ambient_plans:946`、`ambient_agents_rtc:989`——**五个全在 default** | `app/Cargo.toml` | feature 清单不能照抄调研，必须以 manifest 为准 | [02 §6.3](02-现状审计.md#s6-3) |
| **C5** | Sentry DSN / Rudder key 在仓库里 | **不在。** 只有 Firebase key 在（`config.rs:62`）。Sentry DSN 与 Rudder write key 是字段（`:150` / `:100`），值由私有 channel-config 生成器注入 | `crates/warp_core/src/channel/config.rs` | "`strings` 找不到 Sentry DSN"**在改造前就成立**，不能作为改造成功的证据 | [02 §4.1](02-现状审计.md#s4-1)、[07 §7.6.4](07-测试与验证策略.md#s7-6-4) |
| **C6** | `warpui_extras` 有 telemetry 子模块需要关闭 | **零遥测。** 遥测事件存储在 **MIT 岛内的 `warpui_core`** | `grep -rniE 'telemetry\|rudder' crates/warpui_extras` → 0；`crates/warpui_core/src/telemetry/`(3 文件) + `app_focus_telemetry{,_tests}.rs` | 改遥测 = 改 **MIT 文件**，需按 [04 §4.2](04-残余依赖与许可证架构.md#s4-2) 加修改说明 | [02 §7.11](02-现状审计.md#s7-11) |
| **C7** | 6 个 warpdotdev git 依赖是"闭源"的 | 仓库内 `about.toml` 的注释说它们"尚无显式许可证"；前轮上游实测 4 个已是 MIT/Apache-2.0。**两者不矛盾**（时间点不同），但**本轮未复验** | `about.toml:38-49`（且那份排除清单**只是注释，没有实际指令**） | 标 `【待核验】`；M0 强制核验并记 commit hash；核验前按"无许可证"保守对待 | [02 §5.3](02-现状审计.md#s5-3)、[04 §1.1](04-残余依赖与许可证架构.md#s1-1) |
| **C8** | git 依赖共 6 个 | **直接 git 依赖 14 个**（根 workspace 10 + 叶子 manifest 4），另有 13 个 `[patch.crates-io]` 覆盖与 3 个仅在 lock 里的传递 git crate | `grep -rn 'git = "' --include='Cargo.toml' .`；`grep -n 'source = "git+' Cargo.lock` | 多出的 8 个是上游 OSS 库的 warpdotdev fork，与云无关、全部保留；但 `[sources]` 收紧时必须逐一列出 | [02 §5.1](02-现状审计.md#s5-1)、[04 §2](04-残余依赖与许可证架构.md#s2) |
| **C9** | 守卫可以拦在 URI / Host 层 | **只能拦在 socket 对端地址层。** URI 层会误杀 3 个代理测试 | `crates/websocket/src/proxy_tests.rs:332-367`、`:369-402`、`:404-433`（loopback 代理 + 非 loopback 目标 URI）；`crates/websocket/src/proxy.rs:81` 唯一真实 socket 操作是连 127.0.0.1 | 拦截层不是权衡，是唯一解 | [02 §7.12](02-现状审计.md#s7-12)、[03 §7.1](03-阶段1-云模块删除与离线化.md#s7-1)、[07 §7.5](07-测试与验证策略.md#s7-5) |
| **C10** | 迁移与 schema 声明可删，"影响 3 文件 37 处" | **严重低估。** 删除集**之外**仍有 **119 个文件 / 474 处**引用那 17 张云表 | `crates/persistence/src/schema.rs`(21) + `model.rs`(35) + 其余存活文件 117 个 / 418 处 | 结论从"只删声明"改为**"连声明也不删"**——否则会连带逼你删掉 `app/src/workspaces/`、`settings_view/teams_page.rs` 等一整批模块 | [02 §7.15](02-现状审计.md#s7-15)、[03 §11](03-阶段1-云模块删除与离线化.md#s11) |
| **C11**（前序计划） | `initialize_app_for_terminal_view` 被 **107 个**测试文件依赖 | **23 个文件 / 309 处调用** | `grep -rl 'initialize_app_for_terminal_view' app crates --include='*.rs' \| wc -l` → 23 | choke point 修复顺序调整为 ① `state.rs` → ② `ServerApiProvider::new_for_test`（**57 文件**）→ ③ `test_util/terminal.rs`（23 文件）。③ 的价值是"40 个云单例集中在一处"，不是覆盖面 | [02 §7.6](02-现状审计.md#s7-6)、[07 §7.4.1](07-测试与验证策略.md#s7-4-1) |
| **C12**（前序计划） | `warpui_extras/secure_storage/linux.rs:109` 是 **keyring 服务名字面量** | **不是。** 那是一个**故意伪装成 URL 的 AES-256-GCM 回退密钥种子**：`Vec::from("https://releases.warp.dev/channel_versions.json")`。真正的 keyring 服务名由 `app/src/lib.rs:518` `secure_storage_service_name()` → `ChannelState::data_domain()` 派生 | `crates/warpui_extras/src/secure_storage/linux.rs:106-110` 的注释直言"choosing a value that will look inconspicuous in case someone chooses to scan our binary for strings" | ① keyring 命名空间**随 AppId 自动改变**，改 R1 即可，不必单独改这一行；② **这一行是 `strings` 离线验证的已知假阳性**，必须进白名单 | [02 §8.2](02-现状审计.md#s8-2)、[07 §7.6.3](07-测试与验证策略.md#s7-6-3)、[07 §7.12.2](07-测试与验证策略.md#s7-12-2) |
| **C13**（前序计划） | MIT 岛 = 368 文件 / 118k LOC；AGPL 闭包 = 6 crate / 10,430 LOC | MIT 岛 = **417 文件 / 125,158 行**（非测试 329 / 94,402）；AGPL 闭包 = **11 crate / 12,328 非测试行**（核心 6 个确为 10,430） | `find crates/warpui crates/warpui_core -name '*.rs' \| wc -l` 等 | 多出的 5 个（`settings_value`、`settings_value_derive`、`asset_cache`、`virtual-fs`、`websocket`）都在 dev-dep 或可选 feature 路径上，阶段 2 抽取时**必须显式登记** | [02 §3](02-现状审计.md#s3)、[04 §6](04-残余依赖与许可证架构.md#s6) |
| **C14**（前序计划） | `script/` 有 44 项；类 A ≈ 149 文件 / 60,078 行；类 B = 155 文件 | `script/` **42 项**（`ls -la` 的 44 含 `.` 与 `..`）；按本文档定义的删除集实测：类 A = **143 文件 / 63,886 行**，类 B = **151 文件 / 146,101 行** | `ls script/ \| wc -l` → 42；分类脚本见 [07 §7.3.4](07-测试与验证策略.md#s7-3-4) | 数量级一致，结论不变。**但文档采用可复现的实测值**，并把删除集定义与分类脚本一起提交，使数字可随代码更新 | [02 §7.7](02-现状审计.md#s7-7)、[02 §8.3](02-现状审计.md#s8-3) |

<a id="sa-1-1"></a>
### A.1.1 调研原文中被证实的关键判断

修正之外，以下判断经实测**完全成立**，是本设计的支柱：

| 判断 | 证据 |
|---|---|
| `warp-oss` 仍指向生产云 | `app/src/bin/oss.rs:16-17`、`crates/warp_tui/src/bin/oss.rs:19-20` |
| `server_config` / `oz_config` 是非 Option 必填字段 | `crates/warp_core/src/channel/config.rs:16,:18` |
| 不能用假 URL | `state.rs:296` `server_root_domain()` 的 `.expect("Server root URL should be valid")` 就是会 panic 的证据 |
| **`eventsource` 会绕过普通 `send()` 流程** | `crates/http_client/src/lib.rs:499` 直接调 `self.wrapped.eventsource()`（:504/:525），不经 `execute_inner`（:370） |
| `initialize_app` 是一个巨型混合函数 | `app/src/lib.rs:1456-2606`，**1,151 行** |
| `gui = ["voice_input"]` | `app/Cargo.toml:741` |
| WebSocket 支持系统代理 | `crates/websocket/src/native.rs:36` `proxy::resolve_proxy` |
| HTTP 请求自动附带客户端指纹 | `crates/http_client/src/lib.rs:283-363`：Client-ID、版本、OS 类别/名称/版本、Linux 内核版本、W3C traceparent |
| 隐私三项默认全开且与云同步 | `app/src/settings/privacy.rs:87-117` 三个 `default: true`；`:376` `fetch_or_update_settings`、`:684` `update_server_with_local_settings` |
| `install_remote_server.sh` 会在 SSH 远端下载 tarball | `crates/remote_server/src/install_remote_server.sh` |
| 集成 harness 已经离线 | `crates/integration/src/bin/integration.rs:27-60`、`app/src/bin/integration.rs:29-47`（`192.0.2.0:9`） |

<a id="sa-1-2"></a>
### A.1.2 调研遗漏项（本审计新增）

| # | 遗漏 | 说明 | 落在哪章 |
|---|---|---|---|
| **N1** | **`skills-lock.json` + `script/resolve_common_skills` 的构建期网络** | `script/resolve_common_skills:36` 用 `curl -fsSL` 从 `raw.githubusercontent.com/warpdotdev/common-skills` 下载脚本，`:44` 用 `bash` **执行**；默认 ref 是浮动的 `main`。调用方含 `script/bootstrap`（默认开）与 **`script/run`（每次都跑）** | [02 §5.5](02-现状审计.md#s5-5)、[05 §5.2](05-阶段1-仓库与品牌.md#s5-2) |
| **N2** | **`local_tty` / `local_fs` 不是普通 Cargo feature** | `app/build.rs:231-235` 直接给 `app` 打 `cfg`，**不传播依赖侧的 feature**。尤其 `crates/persistence/src/lib.rs:4-6` 的 `MIGRATIONS` gated on `local_fs`——漏开就没有迁移 | [02 §6.5](02-现状审计.md#s6-5)、[03 §6.3](03-阶段1-云模块删除与离线化.md#s6-3)、[08 RK10](08-实施顺序与里程碑.md#s3) |
| **N3** | **`ServerApiProvider` 已经切成 13 个 trait / 178 个方法** | `app/src/server/server_api.rs:1424-1480`。这是 null-object 设计最好的着力点，也让工作量可精确估算 | [03 §8.2](03-阶段1-云模块删除与离线化.md#s8-2) |
| **N4** | **`--workspace` 覆盖 `default-members`** | presubmit 因此拉进 `crates/integration` 的 331 个 e2e；`cargo test` 不带 `--workspace` 则不跑。这是"本地绿、presubmit 红"的头号来源 | [02 §7.3](02-现状审计.md#s7-3)、[07 §7.1.3](07-测试与验证策略.md#s7-1-3) |
| **N5** | **`deny.toml` / `about.toml` 的 `private = { ignore = true }` 盲区** | workspace crate 从不参与许可证检查 → AGPL 渗入 MIT 岛无工具能抓 | [02 §1.4](02-现状审计.md#s1-4)、[04 §5](04-残余依赖与许可证架构.md#s5) |
| **N6** | **`rpm` 包模板把 Warp 服务条款写成 License** | `resources/linux/rpm/{app,cli}/warp.spec.template:12` `License: https://warp.dev/terms-of-service`——与 AGPL 事实不符 | [02 §8.4](02-现状审计.md#s8-4)、[04 §9.5 B11](04-残余依赖与许可证架构.md#s9-5) |
| **N7** | **基线本身就有缺陷** | `script/lint_powershell:8` 指向不存在的 `.PSScriptAnalyzerSettings.psd1`；`script/install_cargo_test_deps:9` 提到不存在的 `.github/workflows/ci.yml` | [02 §7.9](02-现状审计.md#s7-9)、[07 §7.2.2](07-测试与验证策略.md#s7-2-2) |
| **N8** | **3 个 bench 的 `required-features = ["test-util"]` 是隐藏破绽** | 现在因为 `required-features` 未满足而被 clippy 跳过；改动 `test-util` 定义会让它们突然进入 clippy 范围 | [07 §7.1.4](07-测试与验证策略.md#s7-1-4)、[07 §7.10 F9](07-测试与验证策略.md#s7-10) |
| **N9** | **`crates/integration/tests/data/` 的 10 个保留 fixture 是现成的 DB 回归网** | 它们是改造前生成的真实 SQLite 文件；删表声明会让它们加载失败——这是 C10 结论的独立佐证 | [07 §7.8.5](07-测试与验证策略.md#s7-8-5) |

---

<a id="sa-1-3"></a>
### A.1.3 M0 §0.1 执行后的修正（fetch 上游之后才知道的事）

审计是在**剥离了 dotfiles 的快照**上做的。`git fetch upstream master` 之后，
上游自带的 15 个 dotfile 条目 / 109 个文件回来了，以下 6 条结论随之改变。

| # | 审计时的说法 | fetch 之后的事实 | 影响 | 落在哪章 |
|---|---|---|---|---|
| **E1** | "仓库里没有 `.github/`，所以 CI 的定义只能是 `script/presubmit`" | `.github/workflows/ci.yml` **882 行 / 9 个 job**。**CI 才是绿色基线的定义**，presubmit 只是单平台本地子集 | 基线必须按 CI 的命令集建立（`--locked`、`NEXTEST_PROFILE=ci`、同样的 `RUSTFLAGS`、`WARPUI_USE_REAL_DISPLAY_IN_INTEGRATION_TESTS=1`），否则基线与门禁不是同一个东西 | [02 §7.9](02-现状审计.md#s7-9)、[07 §7.2.2](07-测试与验证策略.md#s7-2-2) |
| **E2** | "全仓库没有迁移 / schema 测试" | Rust 侧确实没有，但 CI 有 **`database-migration` job**（`ci.yml:535-560`）：跑完 141 个迁移后 `diff -u old-schema.rs <(diesel print-schema ...)` | **强化**了 C10 的结论：删表声明会直接打红这个 job，删迁移又破坏旧 DB 可加载性——两头堵死，只能都不动 | [02 §7.15](02-现状审计.md#s7-15)、[07 §7.9.3](07-测试与验证策略.md#s7-9-3) |
| **E3** | 两处"基线缺陷"：`script/lint_powershell:8` 与 `script/install_cargo_test_deps:9` 引用不存在的文件 | **两个文件上游都有**（`.PSScriptAnalyzerSettings.psd1`、`.github/workflows/ci.yml`）。不是缺陷，是快照剥离的假象 | `known-fail.txt` 不要预写这两项 | [02 §8.5](02-现状审计.md#s8-5)、[07 §7.2.2](07-测试与验证策略.md#s7-2-2) |
| **E4** | `.gitignore` / `.github` 是否要从零起草——待 fetch 后决定 | 上游全都有，且还有 `.config/nextest.toml`、`.cargo/config.toml`、`.clippy.toml`、`.rustfmt.toml`、`.gitattributes` 等**直接定义构建与测试语义**的文件 | 决策树走完：**一律基于上游裁剪**，不从零起草 | [05 §2](05-阶段1-仓库与品牌.md#s2) |
| **E5** | （未察觉） | **`.gitattributes` 有 LFS 规则**，`crates/input_classifier/models/onnx/bert_tiny_v{1,2,3}.onnx` 是 133 字节的 LFS 指针，真实对象在 warpdotdev 的 LFS 服务器 | fork 只带走指针、带不走对象。**M4 构建前必须解决**，否则 `crates/input_classifier` 构建失败 | [05 §2.1](05-阶段1-仓库与品牌.md#s2-1) |
| **E6** | （未察觉） | **`.clippy.toml` 的 `disallowed-types` 禁用 `std::process::Command`**，要求用 `command::blocking::Command` | [04 §6](04-残余依赖与许可证架构.md#s6) 里"阶段 2 把 `crates/command` 换成 `std::process::Command`"的方案在阶段 1 的 clippy 门禁下过不了；阶段 2 不受此约束 | [02 §8.5](02-现状审计.md#s8-5) |

**另有一条是设计文档自身的缺陷，不是审计错误**：

| # | 问题 | 修正 |
|---|---|---|
| **E7** | 05 §1.2 原本写 `git reset --hard upstream/master` 后用 `git status` 验收快照真实性。**这是错的**——HEAD 未出生时 29 个快照条目全是 untracked，而 `reset --hard` 会**静默覆盖** untracked 文件，任何差异会被当场销毁，随后的 `git status` 必然为空，验收变成空转 | 改为：先用一次性索引（`GIT_INDEX_FILE=$TMP git read-tree` + `git diff`）做纯只读比对，再用 `git symbolic-ref` + `git reset --mixed`（不动工作区）建立 `main`，最后 `git checkout -- .` 取回 dotfiles。见 [05 §1.2](05-阶段1-仓库与品牌.md#s1-2) |

**实际执行结果**：非 dotfile 差异 **0** 条，109 条差异全部是缺失的 dotfile，
反向多出的只有 `docs/` 与 `todo.md`。**快照 ≡ `upstream/master` (`066ec71`) 剥离 dotfiles。**

---

<a id="sa-2"></a>
## A.2 覆盖矩阵

<a id="sa-2-1"></a>
### A.2.1 调研十七节 → 章节

| 调研节 | 标题 | 覆盖章节 | 状态 |
|---|---|---|---|
| 开篇结论 | 不要直接运行 `warp-oss` | [01 §1.1](01-背景与目标.md#s1-1) | ✅ |
| 开篇 5 条 | `warp-local-only / offline_hard` 的五条要求 | [01 §3](01-背景与目标.md#s3)（H1–H5） | ✅ |
| **一** | 最先修改的五个总入口 | | |
| 一.1 | 新建本地版启动入口 | [03 §3](03-阶段1-云模块删除与离线化.md#s3) | ✅（改名 `local_only.rs` → `term4u.rs`） |
| 一.2 | 改造 `ChannelConfig` | [03 §4](03-阶段1-云模块删除与离线化.md#s4) | ✅（采用 enum 方案；九个访问器逐一列出） |
| 一.3 | 拆分 `initialize_app` | [03 §5](03-阶段1-云模块删除与离线化.md#s5) | ✅ |
| 一.4 | 重做 Cargo Features | [03 §6](03-阶段1-云模块删除与离线化.md#s6) | ✅（+ 修正 C2/C3/C4，+ 遗漏项 N2） |
| 一.5 | 增加统一网络出口拦截 | [03 §7](03-阶段1-云模块删除与离线化.md#s7) | ✅（+ 修正 C9：拦截层） |
| **二** | 可以直接删除的上报模块 | | |
| 二.A | RudderStack Telemetry | [03 §9](03-阶段1-云模块删除与离线化.md#s9) | ✅（+ 修正 C1：trait 不可删） |
| 二.B | Privacy Telemetry 设置与提示 UI | [03 §10](03-阶段1-云模块删除与离线化.md#s10) | ✅ |
| **三** | 崩溃与追踪上报 | | |
| 三.A | Sentry | [03 §1.1](03-阶段1-云模块删除与离线化.md#s1-1) | ✅（+ 修正 C2/C5） |
| 三.B | OpenTelemetry / Cloud Agent Tracing | [03 §1.1](03-阶段1-云模块删除与离线化.md#s1-1) | ✅ |
| **四** | Warp 云服务控制面 | | |
| 四.1 | `ServerApiProvider` 与 Server API | [03 §8](03-阶段1-云模块删除与离线化.md#s8) | ✅（**修正调研建议**：采用 null object 而非"不注册"；+ 遗漏项 N3） |
| 四.2 | Auth / Firebase / 账号 / SSO | [03 §1.2](03-阶段1-云模块删除与离线化.md#s1-2) | ✅ |
| 四.3 | Server Experiments 与远程 Feature Flags | [03 §1.2](03-阶段1-云模块删除与离线化.md#s1-2)、[02 §6.6](02-现状审计.md#s6-6) | ✅ |
| **五** | 内置 Warp Agent 与 AI 模块 | [03 §1.3](03-阶段1-云模块删除与离线化.md#s1-3)、[03 §12](03-阶段1-云模块删除与离线化.md#s12) | ✅（blocklist 三条路线） |
| **六** | Computer Use / 录屏 / Artifact Upload | [03 §12.4](03-阶段1-云模块删除与离线化.md#s12-4) | ✅ |
| **七** | Warp Drive / 云对象 / 同步 | [03 §1.4](03-阶段1-云模块删除与离线化.md#s1-4)、[03 §11](03-阶段1-云模块删除与离线化.md#s11) | ✅（+ 修正 C10：迁移与表声明都不删） |
| **八** | 共享会话与团队功能 | [03 §1.4](03-阶段1-云模块删除与离线化.md#s1-4)、[04 §1.3](04-残余依赖与许可证架构.md#s1-3)（E1） | ✅ |
| **九** | MCP 模块如何拆 | [03 §1.3](03-阶段1-云模块删除与离线化.md#s1-3)、[03 §1.5](03-阶段1-云模块删除与离线化.md#s1-5) | ✅ |
| **十** | 自动更新与 Changelog | [03 §1.4](03-阶段1-云模块删除与离线化.md#s1-4) | ✅ |
| **十一** | SSH Remote Server 与远端组件下载 | [03 §1.4](03-阶段1-云模块删除与离线化.md#s1-4) | ✅ |
| **十二** | Voice Input | [03 §1.3](03-阶段1-云模块删除与离线化.md#s1-3)、[03 §6.4](03-阶段1-云模块删除与离线化.md#s6-4) | ✅ |
| **十三** | 其他会主动联网的入口 | | |
| 十三.1 | 在线字体与资源缓存 | [03 §1.5](03-阶段1-云模块删除与离线化.md#s1-5)、[08 M5 5.8](08-实施顺序与里程碑.md#m5) | ✅ |
| 十三.2 | LSP 自动下载 | [03 §1.5](03-阶段1-云模块删除与离线化.md#s1-5)、[08 M5 5.7](08-实施顺序与里程碑.md#m5) | ✅ |
| 十三.3 | 打开网页的菜单项 | [03 §8.4](03-阶段1-云模块删除与离线化.md#s8-4)、[08 M5 5.9](08-实施顺序与里程碑.md#m5) | ✅ |
| 十三.4 | 插件与 Bundled Skills | [03 §1.5](03-阶段1-云模块删除与离线化.md#s1-5)、[05 §5.2](05-阶段1-仓库与品牌.md#s5-2) | ✅（+ 修正 C3、遗漏项 N1） |
| **十四** | 建议保留的本地核心模块 | [03 §1.5](03-阶段1-云模块删除与离线化.md#s1-5) | ✅（+ 修正 C6：`warpui_extras` 无遥测） |
| **十五** | 推荐的删除/替换顺序（8 步） | [08](08-实施顺序与里程碑.md)（M1–M6 与 8 步一一对应） | ✅ |
| **十六** | 代码检索清单（6 组 rg） | [07 §7.6.2](07-测试与验证策略.md#s7-6-2)、[07 §7.12](07-测试与验证策略.md#s7-12) | ✅（**改造为可执行的验收命令**，并加白名单处理假阳性） |
| **十七** | 首次真实使用前的验证标准 | [07](07-测试与验证策略.md)（全章） | ✅（大幅扩展：三分类、choke point、清单 diff、编译矩阵、13 场景、6 条离线验证） |
| **最终模块处置表** | 34 行 | [03 §1](03-阶段1-云模块删除与离线化.md#s1) | ✅（见 [A.2.2](#sa-2-2)） |
| **许可证结构（两张图）** | 阶段 1 / 阶段 2 | [01 §4](01-背景与目标.md#s4)、[06 §1.1](06-阶段2-完整MIT路线.md#s1-1) | ✅ |

<a id="sa-2-2"></a>
### A.2.2 最终模块处置表 → 本文档处置

| 调研表的行 | 调研的处理 | 本文档的处置 | 差异说明 | 落点 |
|---|---|---|---|---|
| `app/src/server/telemetry/` | 直接删除 | 删（V0） | — | [03 §1.1](03-阶段1-云模块删除与离线化.md#s1-1) |
| `crates/warp_core/src/telemetry.rs` | 先改空宏，**后删除** | **shim，永久保留** | **C1**：删 = 212 文件 / 888 处 | 同上 |
| `crates/warpui_core/src/telemetry/` | 删除或本地空实现 | shim | 位于 **MIT 岛**，改动需加修改说明（C6） | 同上 |
| `app/src/crash_reporting/` | 直接删除 | 删（V0） | — | 同上 |
| `app/src/tracing/native.rs` | 删除 | 删（V0） | — | 同上 |
| `app/src/tracing/cloud_agent_auth.rs` | 删除 | 删（V0） | — | 同上 |
| `app/src/autoupdate/` | 直接删除 | 删（V0） | — | [03 §1.4](03-阶段1-云模块删除与离线化.md#s1-4) |
| `app/src/server/server_api*` | Offline 构建不编译，后删除 | **空实现（null object）** | 决策 D4 要求保 UI 外壳；且 57 个测试依赖 `new_for_test()` | [03 §8](03-阶段1-云模块删除与离线化.md#s8) |
| `app/src/server/cloud_objects/` | 删除 | 删（V0） | — | [03 §1.2](03-阶段1-云模块删除与离线化.md#s1-2) |
| `app/src/server/sync_queue.rs` | 删除 | 删（V0） | — | 同上 |
| `app/src/server/experiments/` | 删除 | 删（V0） | — | 同上 |
| `app/src/server/iap_identity_minter.rs` | 删除 | 删（V0） | — | 同上 |
| `app/src/server/voice_transcriber.rs` | 删除 | 删（V0） | — | 同上 |
| `app/src/auth/` | 删云认证；暂留本地身份空实现 | 同 | — | 同上 |
| `app/src/ai/ambient_agents/` | 删除 | 删（V0） | — | [03 §1.3](03-阶段1-云模块删除与离线化.md#s1-3) |
| `app/src/ai/cloud_*` | 删除 | 删（V0） | 实际路径：`cloud_environments/`、`cloud_agent_config/`（**目录**）、`cloud_agent_settings.rs` | 同上 |
| `app/src/ai/artifacts/` | 删除 | 删（V0） | — | 同上 |
| `app/src/ai/agent_sdk/` | 不用内置 Agent 时删除 | 删（V0）——58 文件 / 32,628 行 | — | 同上 |
| `app/src/ai/blocklist/` | 不用时删除；否则改接本地 Runtime | **V0 路线 B（保留 + 降级），V1 决策** | 三条路线量化比较 | [03 §12](03-阶段1-云模块删除与离线化.md#s12) |
| `app/src/ai_assistant/` | 删除 | 删（V0） | — | [03 §1.3](03-阶段1-云模块删除与离线化.md#s1-3) |
| `app/src/drive/` | 不需要时删除 | 删（V0） | — | [03 §1.4](03-阶段1-云模块删除与离线化.md#s1-4) |
| `app/src/cloud_object/` | 需要本地对象时改本地存储；否则删除 | 删（V0）；**但表声明与迁移全部保留** | **C10** | [03 §11](03-阶段1-云模块删除与离线化.md#s11) |
| `app/src/terminal/shared_session/` | 删除 | 删（V0） | 连带剥离 E1 | [04 §1.3](04-残余依赖与许可证架构.md#s1-3) |
| `app/src/remote_server/` | 删除 | 删（V0） | — | [03 §1.4](03-阶段1-云模块删除与离线化.md#s1-4) |
| `crates/remote_server/` | 删除 | 删（V0） | — | 同上 |
| Cloud MCP / Gallery / OAuth | 删除 | 删（V0） | — | [03 §1.3](03-阶段1-云模块删除与离线化.md#s1-3) |
| File-based stdio MCP | 可保留 | 保留 | — | [03 §1.5](03-阶段1-云模块删除与离线化.md#s1-5) |
| `crates/http_client` | 保留 + 非 loopback 硬拒绝 | 同，**拦在 socket 对端地址层**，且 `eventsource` 单独加 | **C9** | [03 §7](03-阶段1-云模块删除与离线化.md#s7) |
| `crates/websocket` | 可删除；保留时加硬拒绝 | **不能删**（`warp_errors` 的可选 feature 依赖它），加硬拒绝 | **C13**（它在 MIT 岛的 AGPL 闭包里） | 同上 |
| Terminal / PTY / Tabs / Panes | 保留 | 保留 | — | [03 §1.5](03-阶段1-云模块删除与离线化.md#s1-5) |
| File Tree / Editor / Preview | 保留 | 保留 | — | 同上 |
| Local SQLite / Settings | 保留，删云同步 | 同 | — | 同上 |
| Local Workflows | 保留 | 保留 | — | 同上 |
| LSP | 保留运行时，关自动下载 | 同 | — | 同上 |
| **最重要的三处** | `app/src/lib.rs` / `http_client/src/lib.rs` / `websocket/src/native.rs` | 同意，**并补第四处** `crates/http_client/src/lib.rs:499 eventsource` 与第五处 `crates/websocket/src/proxy.rs:81` | `eventsource` 绕过 `execute_inner`；代理走另一条 socket | [03 §7.2](03-阶段1-云模块删除与离线化.md#s7-2) |

<a id="sa-2-3"></a>
### A.2.3 调研十六节的 6 组检索 → 验收命令

| 调研的检索 | 本文档的对应验收 |
|---|---|
| 查所有 Warp 云域名 | [07 §7.12.2](07-测试与验证策略.md#s7-12-2)（**改为查二进制而非源码**，并加 `releases.warp.dev/channel_versions.json` 白名单——C12） |
| 查所有上报与追踪 | [07 §7.6.2](07-测试与验证策略.md#s7-6-2) T1/T2/T3（**不再用 `rg send_telemetry` == 0**——C1） |
| 查所有云 Client | [07 §7.12.1](07-测试与验证策略.md#s7-12-1)（改为 `cargo tree` 黑名单） |
| 查所有网络调用 | 由守卫的**默认拒绝**语义覆盖（H4），不再依赖 grep |
| 查所有云同步 | [08 M5](08-实施顺序与里程碑.md#m5) 的删除清单 |
| 查所有 Feature | [02 §6](02-现状审计.md#s6)（**以 manifest 为准**，不用 grep 猜——C3/C4） |

---

<a id="sa-3"></a>
## A.3 调研原文（逐字归档）

> **归档说明**：
>
> - 内容一字未改。
> - **唯一的机械变换**：原文的一级标题（`# `）统一降一级为二级（`## `），
>   以适配本文件的标题层级。变换命令：`sed 's/^# /## /'`。除此之外无任何改动。
> - 原文中的判断若与本审计冲突，以 [A.1](#a1-审计修正对照表) 为准；本节仅作溯源用途。
> - 原文写于本轮审计之前，其中若干路径与数字已在 A.1 中修正——**阅读本节时请对照 A.1**。

---

下面按 **Warp `warpdotdev/warp`，提交 `066ec71b736fc3755e29f58f733deadbdac3d1af`** 整理。

先给结论：

> **不要直接运行当前 `warp-oss`。**
> 它虽然把 RudderStack、Sentry、自动更新配置设成了 `None`，但仍然使用 `WarpServerConfig::production()` 和 `OzConfig::production()`，会初始化 Warp 生产云服务相关代码。GUI 与 TUI 的 OSS 入口都是如此。

你的目标应当是新建一个真正的：

```text
warp-local-only / offline_hard
```

它应满足：

```text
1. 编译时不包含遥测、Sentry、OTel、云 Agent、认证、Drive、共享、更新模块
2. 启动时不创建任何 Warp Server HTTP / SSE / WebSocket Client
3. Warp 主进程只允许 loopback，拒绝所有外部网络
4. 即使以后误合并了新的云调用，也会被统一网络出口拒绝
5. 本地 Shell 子进程是否联网，由你另行控制
```

---

## 一、最先修改的五个总入口

这些是整个改造的根节点。

## 1. 新建本地版启动入口

当前文件：

```text
app/src/bin/oss.rs
crates/warp_tui/src/bin/oss.rs
```

目前都包含：

```rust
server_config: WarpServerConfig::production(),
oz_config: OzConfig::production(),
telemetry_config: None,
crash_reporting_config: None,
autoupdate_config: None,
mcp_static_config: None,
```

问题是前两个字段仍然指向：

```text
https://app.warp.dev
wss://rtc.app.warp.dev/graphql/v2
wss://sessions.app.warp.dev
https://oz.warp.dev
Firebase Authentication
```

建议新增：

```text
app/src/bin/local_only.rs
```

不要把 URL 改成空字符串，也不要改成无效域名。应该让云配置在类型层面不存在，例如：

```rust
pub enum ConnectivityMode {
    Offline {
        allow_loopback: bool,
    },
    Cloud {
        server: WarpServerConfig,
        oz: OzConfig,
    },
}
```

本地入口：

```rust
ChannelConfig {
    app_id: AppId::new("local", "warp", "WarpLocalOnly"),
    logfile_name: "warp-local-only.log".into(),

    connectivity: ConnectivityMode::Offline {
        allow_loopback: true,
    },

    telemetry_config: None,
    crash_reporting_config: None,
    autoupdate_config: None,
    mcp_static_config: None,
}
```

不建议使用：

```rust
server_root_url: ""
server_root_url: "http://127.0.0.1:9"
server_root_url: "https://invalid.local"
```

因为当前很多代码默认这些 URL 一定存在、一定可解析，使用假地址容易产生：

* 重试循环
* 后台报错
* DNS 查询
* 回退到其他地址
* 未来代码误用

---

## 2. 改造 `ChannelConfig`

核心文件：

```text
crates/warp_core/src/channel/config.rs
crates/warp_core/src/channel/state.rs
```

当前 `ChannelConfig` 强制要求：

```rust
pub server_config: WarpServerConfig,
pub oz_config: OzConfig,
```

而 `ChannelState::init()` 即使是 OSS Channel，也默认填入生产云配置。

必须修改的 API：

```text
ChannelState::server_root_url()
ChannelState::ws_server_url()
ChannelState::rtc_http_url()
ChannelState::session_sharing_server_url()
ChannelState::oz_root_url()
ChannelState::workload_audience_url()
ChannelState::firebase_api_key()
ChannelState::iap_config()
ChannelState::server_root_domain()
```

推荐两种做法。

### 更推荐：使用枚举

```rust
pub enum ConnectivityMode {
    Offline,
    Cloud {
        server: WarpServerConfig,
        oz: OzConfig,
    },
}
```

云端方法返回：

```rust
Result<..., OfflineError>
```

或：

```rust
Option<...>
```

### 次选：配置字段改为 Option

```rust
pub server_config: Option<WarpServerConfig>,
pub oz_config: Option<OzConfig>,
```

但所有调用点都必须显式处理 `None`，不能：

```rust
unwrap_or_default()
expect("server config")
```

---

## 3. 拆分 `initialize_app`

最关键的启动文件：

```text
app/src/lib.rs
```

核心函数：

```text
run_internal()
initialize_app()
app_callbacks()
```

当前 `initialize_app()` 把本地终端、GUI、认证、Warp Server、遥测、AI、Drive、共享、更新全部混在同一个启动流程里。

不要在这个巨型函数里到处添加几十个：

```rust
#[cfg(not(feature = "offline_hard"))]
```

建议直接拆成：

```rust
fn initialize_common_app(...)
fn initialize_local_app(...)
fn initialize_cloud_app(...)
```

结构：

```rust
initialize_common_app(...);

#[cfg(feature = "offline_hard")]
initialize_local_app(...);

#[cfg(not(feature = "offline_hard"))]
initialize_cloud_app(...);
```

其中：

### `initialize_common_app`

只初始化：

```text
本地 Settings
本地 SQLite
PTY
Terminal
Tabs / Panes
Editor
File Tree
Local Shell
Themes
Keybindings
Local Workflows
本地文件搜索
本地 LSP
```

### `initialize_cloud_app`

放入：

```text
ServerApiProvider
AuthManager
UserWorkspaces
CloudModel 同步
TelemetryCollector
Sentry
Warp Agent
Ambient Agents
Cloud Drive
Shared Sessions
Autoupdate
IAP
Cloud MCP
Voice Transcriber
Server Experiments
```

`offline_hard` 构建根本不编译 `initialize_cloud_app()`。

---

## 4. 重做 Cargo Features

核心文件：

```text
app/Cargo.toml
```

当前默认 Feature 集合包含大量云功能，包括：

```text
agent_mode
viewing_shared_sessions
shared_with_me
global_ai_analytics_collection
mcp_oauth
api_key_management
web_search_ui
cloud_environments
ambient_agents
scheduled_ambient_agents
cloud_conversations
cloud_mode
conversation_artifacts
oz_handoff
cloud_runners
factory_mcp
remote_codebase_indexing
voice_input
```

因此不能继续使用普通默认构建：

```bash
cargo build -p warp --bin warp-oss
```

建议新增：

```toml
[features]
offline_hard = [
    "warp_core/offline_hard",
    "http_client/offline_hard",
    "websocket/offline_hard",
]

local_only = [
    "offline_hard",
    "local_tty",
    "local_fs",
    "settings_file",
]
```

本地 Fork 可以把：

```toml
default = ["local_only"]
```

或者始终：

```bash
cargo build \
  -p warp \
  --bin warp-local-only \
  --no-default-features \
  --features local_only
```

具体 UI Feature 后续逐个加，不要从当前 `default` 集合做减法。

### 特别注意 `gui`

当前定义为：

```toml
gui = ["voice_input"]
```

也就是说启用 GUI 会顺便编译 Voice Input，而启动流程又注册了 `ServerVoiceTranscriber`。

建议改成：

```toml
gui = []
voice_input = ["dep:voice_input"]
cloud_voice_input = ["voice_input"]
```

本地 GUI 不启用 `cloud_voice_input`。

---

## 5. 增加统一网络出口拦截

即使删掉了当前已知云模块，也必须保留最后一道防线。

主要 HTTP 出口：

```text
crates/http_client/src/lib.rs
```

关键方法：

```text
Client::get()
Client::post()
Client::put()
Client::patch()
Client::delete()
Client::execute()
Client::execute_inner()
RequestBuilder::send()
RequestBuilder::eventsource()
```

当前所有 Native HTTP 请求还会自动附带：

```text
Warp Client ID
Warp 版本
操作系统名称
操作系统版本
Linux Kernel 版本
可能的 Trace Context
```

真正执行网络请求的位置是：

```rust
Client::execute_inner()
```

建议在这里实现：

```text
offline_hard:
    localhost      允许
    127.0.0.0/8    允许
    ::1            允许
    Unix socket    允许
    其他地址        一律拒绝
```

还要单独处理：

```rust
RequestBuilder::eventsource()
```

因为 SSE 可能绕过普通 `send()` 流程直接创建长连接。

WebSocket 总出口：

```text
crates/websocket/src/native.rs
```

关键方法：

```rust
pub async fn connect(...)
```

当前它会直接建立 TCP/TLS/WebSocket，并支持系统代理。

同样需要在 `offline_hard` 下拒绝所有非 loopback 地址。

---

## 二、可以直接删除的上报模块

这些模块不属于本地终端核心，可以直接从本地构建中删除或不编译。

## A. RudderStack Telemetry

### 主要目录

```text
app/src/server/telemetry/
app/src/server/telemetry_ext.rs
crates/warp_core/src/telemetry.rs
crates/warpui_core/src/telemetry/
crates/warpui_core/src/app_focus_telemetry.rs
```

### 主要初始化入口

`app/src/lib.rs`：

```rust
let telemetry_collector = TelemetryCollector::new(server_api_clone);
telemetry_collector.initialize_telemetry_collection(ctx);
```

### 生命周期出口

`app_callbacks()` 中：

```text
on_become_active    → record_app_focus
on_resigned_active  → record_app_blur
on_will_terminate   → flush_telemetry_events_for_shutdown
on_should_close     → TelemetryEvent
```

### 实际网络发送

```text
app/src/server/telemetry/mod.rs
```

它会：

* 从全局队列取事件
* 区分 UGC 和非 UGC
* POST 到 RudderStack
* 退出时把未发送事件写入本地
* 下次启动重新发送

定时器：

```text
每 30 秒刷新事件
每 60 秒记录 Active Usage
退出时最多保存部分事件
启动时重发
```

### 建议处理

第一阶段：

```rust
#[cfg(feature = "offline_hard")]
macro_rules! send_telemetry_from_ctx {
    ($($tt:tt)*) => {{}};
}
```

以下宏全部变空：

```text
send_telemetry_from_ctx!
send_telemetry_from_app_ctx!
send_telemetry_sync_from_ctx!
send_telemetry_sync_from_app_ctx!
record_telemetry_from_ctx!
record_telemetry_on_executor!
```

第二阶段再删除事件定义和生产者。

### 遥测事件生产者

仓库中存在大量独立遥测文件，例如：

```text
app/src/tui/telemetry.rs
crates/ai/src/telemetry.rs
app/src/code/lsp_telemetry.rs
app/src/ai/agent/telemetry.rs
app/src/notebooks/telemetry.rs
app/src/ai/skills/telemetry.rs
app/src/antivirus/telemetry.rs
app/src/tab_configs/telemetry.rs
app/src/ai/ambient_agents/telemetry.rs
app/src/ai/agent_management/telemetry.rs
app/src/code_review/telemetry_event.rs
```

这些生产者不是第一优先级。只要：

```text
TelemetryCollector 不注册
TelemetryApi 不存在
底层宏为空
HTTP 出口拒绝外网
```

它们就不会外发。之后再为了减小代码量逐步删除。

---

## B. Privacy Telemetry 设置和提示 UI

文件：

```text
app/src/settings/privacy.rs
app/src/settings_view/privacy_page.rs
app/src/ai/blocklist/telemetry_banner.rs
```

当前默认值是：

```text
Telemetry = true
Crash Reporting = true
Cloud Conversation Storage = true
```

而且这些隐私配置会与云端同步。代码中还存在组织强制开启和 Agent Analytics 实验覆盖本地关闭值的路径。

本地版不要保留可变开关，建议改成不可变：

```rust
telemetry_enabled = false
crash_reporting_enabled = false
cloud_conversation_storage_enabled = false
```

更彻底的做法是：

```text
删除网络同步逻辑
删除 fetch_or_update_settings()
删除 update_server_with_local_settings()
保留一个固定的 LocalPrivacyPolicy
```

例如：

```rust
pub struct LocalPrivacyPolicy;

impl LocalPrivacyPolicy {
    pub const TELEMETRY_ENABLED: bool = false;
    pub const CRASH_REPORTING_ENABLED: bool = false;
    pub const CLOUD_STORAGE_ENABLED: bool = false;
}
```

不要让本地软件存在“以后服务器或实验可以重新开启”的路径。

---

## 三、可以直接删除的崩溃与追踪上报模块

## A. Sentry

目录：

```text
app/src/crash_reporting/
```

包括：

```text
app/src/crash_reporting/mod.rs
app/src/crash_reporting/mac.rs
app/src/crash_reporting/linux.rs
app/src/crash_reporting/sentry_minidump.rs
```

运行时会附带或可能附带：

```text
稳定用户 ID / 稳定匿名 ID
应用版本
GPU 信息
窗口系统
虚拟环境
杀毒软件名称
Breadcrumbs
异常和崩溃信息
Linux/Windows Minidump
```

### 删除方式

在 `app/Cargo.toml` 中不要启用：

```toml
crash_reporting
cocoa_sentry
heap_usage_tracking
log_expensive_frames_in_sentry
```

移除或确保不进入：

```rust
crash_reporting::init(ctx)
crash_reporting::uninit_sentry()
sentry::Hub::main()
sentry::integrations::anyhow::capture_anyhow(...)
```

`app/src/lib.rs` 中这些代码目前分布在：

```text
run_internal()
initialize_app()
app_callbacks()
```

### 可删除的构建脚本

```text
script/sentry_upload_dif.sh
script/sentry_create_release.sh
script/macos/update_sentry_cocoa
```

这些主要是发布构建用途，不影响本地终端。

---

## B. OpenTelemetry / Cloud Agent Tracing

文件：

```text
app/src/tracing.rs
app/src/tracing/native.rs
app/src/tracing/cloud_agent_auth.rs
```

环境变量：

```text
WARP_CLOUD_AGENT_OTLP_ENDPOINT
OTEL_SERVICE_NAME
OTEL_EXPORTER_OTLP_TRACES_TIMEOUT
OTEL_EXPORTER_OTLP_TIMEOUT
```

当前实现会在满足环境条件时构建 OTLP Exporter，并发送 Cloud Agent Trace。

建议：

* 删除 `native.rs`
* 删除 `cloud_agent_auth.rs`
* `tracing::init()` 固定安装 `NoSubscriber`
* 或只保留写本地文件的 tracing subscriber
* 删除以下依赖：

```text
opentelemetry
opentelemetry-http
opentelemetry-otlp
opentelemetry_sdk
tracing-opentelemetry
```

不要仅依赖“不设置环境变量”，因为你的目标是从构建层保证无法发送。

---

## 四、Warp 云服务控制面：删除或用空实现替换

## 1. `ServerApiProvider` 和 Server API

主要文件：

```text
app/src/server/server_api.rs
app/src/server/server_api/
```

子模块包括：

```text
ai
auth
block
download
factory
harness_support
integrations
managed_mcp
managed_secrets
object
presigned_upload
referral
team
tui_onboarding
workspace
```

顶层 Server 目录：

```text
app/src/server/
```

云相关模块：

```text
app/src/server/server_api.rs
app/src/server/server_api/
app/src/server/cloud_objects/
app/src/server/experiments/
app/src/server/graphql/
app/src/server/iap_identity_minter.rs
app/src/server/sync_queue.rs
app/src/server/telemetry/
app/src/server/voice_transcriber.rs
```

### 启动入口

`initialize_app()` 当前无条件创建：

```rust
ServerApiProvider::new(...)
let server_api = ...
let ai_client = ...
```

并随后把这些 Client 注入几乎所有云模块。

### 建议

不要在 `offline_hard` 下注册 `ServerApiProvider`。

不建议做一个“指向假地址的 ServerApi”。更好的方式：

```rust
#[cfg(not(feature = "offline_hard"))]
let server_api_provider = ...

#[cfg(feature = "offline_hard")]
let local_services = LocalServices::new(...);
```

删除 Server API 后，可以进一步从依赖中移除：

```text
warp_server_client
warp_graphql
cloud_object_client
warp_multi_agent_client
warp_managed_secrets
```

不过这些依赖目前被类型层广泛引用，建议在启动路径断开后逐步删除，而不是一开始一次性删光。

---

## 2. Auth、Firebase、账号和 SSO

目录：

```text
app/src/auth/
crates/warp_server_auth/
```

相关内容：

```text
AuthManager
AuthState
AuthStateProvider
Firebase Token
刷新 Token
SSO
设备登录
粘贴认证 Token
登录页面
退出登录
账号删除
```

`app/src/auth/mod.rs` 还会联动：

```text
CloudModel
UpdateManager
SyncQueue
SharedSessionManager
TeamUpdateManager
Agent 模型
NotebookManager
```

### 建议

删除：

```text
AuthManager
登录 UI
SSO
Firebase
Refresh Token
Device Authorization
账号数据管理入口
```

但不要立即删除所有 `AuthStateProvider` 类型，因为很多 UI 和模型可能默认读取它。

先提供固定本地身份：

```rust
pub struct LocalIdentity {
    pub local_user_id: LocalUserId,
}

pub struct LocalAuthStateProvider;
```

本地状态始终：

```text
is_logged_in = false 或 local-only
没有 ID Token
没有 Refresh Token
没有 Email
没有 Team
没有云端 UID
```

然后逐步把 UI 对 `UserUid`、`AuthStateProvider` 的依赖替换掉。

仅打开现有：

```toml
skip_login
```

**不够安全**。它只是绕过登录流程，不会阻止 `ServerApiProvider`、云对象、实验和其他客户端初始化。

---

## 3. Server Experiments 和远程 Feature Flags

目录：

```text
app/src/server/experiments/
app/src/experiments/
```

当前服务端实验可以改变：

```text
Agent Mode
Agent Analytics
Codebase Context
Session Sharing
Prompt Suggestions
Oz Harness
Mac Runners
其他功能标志
```

启动入口：

```rust
ServerExperiments::new_from_cache(...)
experiments::init(ctx)
```

建议：

* 删除远程实验获取
* 删除服务端动态开关
* 保留一个编译期固定 Feature Set
* 本地构建遇到未知 Feature，默认关闭
* 不从旧 SQLite 恢复之前缓存的云实验状态

---

## 五、内置 Warp Agent 与 AI 模块

如果你只需要：

```text
Terminal
Tabs
Panes
File Tree
Text Preview
运行 Codex / Claude Code / 其他 CLI Agent
```

那么 Warp 自己的内置 Agent 可以整体不编译。

主要入口：

```text
app/src/ai/
app/src/ai_assistant/
```

`app/src/ai/mod.rs` 当前包含：

```text
agent
agent_conversations_model
agent_management
ambient_agents
artifacts
blocklist
codebase_auto_indexing
custom_endpoints
custom_model_routers
get_relevant_files
harness_availability
llms
orchestration
predict
remote_agent_context
request_usage_model
restored_conversations
agent_sdk
cloud_agent_config
cloud_agent_settings
cloud_environments
connected_self_hosted_workers
execution_profiles
facts
mcp
voice
```

## 建议直接删除或关闭

```text
app/src/ai/agent/
app/src/ai/ambient_agents/
app/src/ai/agent_management/
app/src/ai/agent_conversations_model.rs
app/src/ai/artifacts/
app/src/ai/artifact_download.rs
app/src/ai/cloud_agent_config.rs
app/src/ai/cloud_agent_settings.rs
app/src/ai/cloud_environments/
app/src/ai/connected_self_hosted_workers.rs
app/src/ai/predict/
app/src/ai/request_usage_model.rs
app/src/ai/get_relevant_files/
app/src/ai/remote_agent_context/
app/src/ai/restored_conversations/
app/src/ai/conversation_rename.rs
app/src/ai/conversation_utils.rs
app/src/ai_assistant/
```

## `blocklist` 的处理

```text
app/src/ai/blocklist/
```

这是 Warp 内置 Agent 会话、工具调用、命令执行、文件编辑、Computer Use 的主要 UI 和执行层。

如果你不使用 Warp Agent，可以整体编译关闭。

如果以后计划接入自己的本地 Agent Runtime，则可以保留：

```text
Agent View UI
Block UI
Action rendering
Diff rendering
Approval UI
```

但需要替换：

```text
Warp Multi-Agent Client
Server Conversation Token
Cloud Artifact
Cloud Conversation
Cloud Orchestration
```

为自己的本地协议。

## 外部 CLI Agent 不受影响

删除 Warp 内置 Agent 不影响你在终端里运行：

```bash
codex
claude
gemini
opencode
pi
```

它们就是普通 PTY 子进程。

但这些外部 CLI Agent 是否向自己的服务商发送数据，是它们自己的行为，不受 Warp 本地化改造控制。

---

## 六、Computer Use、录屏与 Artifact Upload

建议整个删除。

相关文件：

```text
app/src/ai/blocklist/action_model/execute/request_computer_use.rs
app/src/ai/blocklist/action_model/execute/use_computer.rs
app/src/ai/blocklist/action_model/execute/start_recording.rs
app/src/ai/blocklist/action_model/execute/stop_recording.rs
app/src/ai/blocklist/action_model/recording_controller.rs
app/src/ai/blocklist/action_model/recording_finalize.rs
app/src/ai/blocklist/action_model/recording_telemetry.rs
app/src/ai/agent_sdk/artifact_upload.rs
```

相关依赖：

```text
computer_use
conversation_artifacts
```

录屏模块可以：

* 录制窗口或整个屏幕

* 生成 MP4

* 生成缩略图

* 上传为云端 Artifact

* 上传后删除本地文件

关闭 Feature：

```text
agent_mode_computer_use
background_computer_use
local_computer_use
recording_mode
conversation_artifacts
VideoRecording
BackgroundComputerUse
LocalComputerUse
```

如果完全不需要 Warp 内置 Agent，这组可以直接删除，不影响本地 Shell、分栏和文件编辑。

---

## 七、Warp Drive、云对象和同步

涉及目录：

```text
app/src/cloud_object/
app/src/drive/
app/src/server/cloud_objects/
app/src/server/sync_queue.rs
app/src/settings/cloud_preferences_syncer.rs
app/src/workspaces/update_manager.rs
```

启动入口包括：

```text
CloudModel::new(...)
SyncQueue::new(...)
UpdateManager::new(...)
TeamUpdateManager::new(...)
Listener::new(...)
CloudViewModel::new(...)
initialize_cloud_preferences_syncer(...)
```

## 可以直接删除

如果你不需要 Warp Drive、Notebook、云 Workflow：

```text
app/src/drive/
app/src/server/cloud_objects/
app/src/server/sync_queue.rs
app/src/settings/cloud_preferences_syncer.rs
CloudViewModel
UpdateManager
Listener
TeamUpdateManager
```

## 不建议直接删 `CloudObject` 类型的情况

Warp 当前把很多本地可编辑对象也建模为 CloudObject，例如：

```text
Workflow
Notebook
Folder
EnvVarCollection
AI Fact
MCP Server
```

如果你还想保留本地 Workflow 或 Notebook，建议：

```text
保留对象数据结构
删除 ObjectClient
删除 SyncQueue
删除 ServerId 依赖
删除 RTC Listener
改用本地 SQLite / 本地文件持久化
```

如果仅需要终端 + 文件树 +文本预览，最简单的是连 Warp Drive UI 一起删除，只保留：

```text
LocalWorkflows
普通文本编辑器
本地项目文件
```

---

## 八、共享会话和团队功能

删除：

```text
app/src/terminal/shared_session/
app/src/session_management/ 中的共享会话部分
session-sharing-protocol
```

启动入口：

```rust
terminal::shared_session::manager::Manager::new
terminal::shared_session::permissions_manager::SessionPermissionsManager::new
```

关闭 Feature：

```text
viewing_shared_sessions
creating_shared_sessions
session_sharing
session_sharing_acls
shared_session_long_running_commands
agent_shared_sessions
shared_with_me
team_workflows
team_features_override
```

还应删除或关闭：

```text
Teams Page
Team Client
Workspace Client
团队成员同步
邀请
共享 Block
共享 Object
```

---

## 九、MCP 模块如何拆

MCP 不能一刀切，因为你可能需要本地 MCP。

## 可以保留

```text
本地 stdio MCP
项目内 MCP 配置
FileMCPWatcher
FileBasedMCPManager
MCP 协议解析
本地子进程管理
```

当前初始化入口：

```rust
FileMCPWatcher::new
FileBasedMCPManager::new
```

## 必须删除

```text
TemplatableMCPServerManager
MCPGalleryManager
ManagedMcpClient
ManagedSecretsClient
Factory MCP
Warp-hosted MCP
MCP Cloud Installation
MCP OAuth
云端 MCP 模板同步
```

对应入口：

```rust
TemplatableMCPServerManager::new(...)
MCPGalleryManager::new
```

Server API 子模块：

```text
app/src/server/server_api/managed_mcp.rs
app/src/server/server_api/managed_secrets.rs
app/src/server/server_api/factory.rs
app/src/server/server_api/integrations.rs
```

关闭 Feature：

```text
mcp_oauth
mcp_debugging_ids
well_known_mcp_ids
factory_mcp
warp_managed_secrets
integration_command
provider_command
```

### 重要边界

本地 MCP Server 是独立进程。即使 Warp 主进程完全离线，MCP Server 自己仍可能联网。

因此严格模式还需要：

```text
只加载你审查过的本地 MCP
不给 MCP 注入云端 Token
对 MCP 子进程单独设置网络权限
禁用自动安装 MCP
禁用 Gallery
```

---

## 十、自动更新和 Changelog

可以直接删除：

```text
app/src/autoupdate/
app/src/changelog_model.rs
channel_versions
```

启动和退出入口：

```text
autoupdate::check_and_report_update_errors(ctx)
autoupdate::remove_old_executable()
AutoupdateState::register(ctx, server_api.clone())
autoupdate::spawn_child_if_necessary(ctx)
autoupdate::apply_pending_update(...)
ChangelogModel::new(server_api.clone())
```

关闭：

```text
autoupdate
autoupdate_ui_revamp
changelog
oz_changelog_updates
release_bundle 中的在线更新路径
```

本地 Fork 更新方式建议改为：

```text
手工 git pull
手工重新编译
或者使用你自己的签名发布包
```

---

## 十一、SSH Remote Server 和远端组件下载

如果你不需要 Warp 自己在 SSH 主机安装远端组件，可以直接删除：

```text
app/src/remote_server/
crates/remote_server/
```

初始化入口：

```rust
RemoteServerManager::new
RemoteCodebaseIndexModel::new
remote_server::wire_auth_token_rotation(ctx)
```

安装脚本：

```text
crates/remote_server/src/install_remote_server.sh
```

它会在 SSH 远端：

* 使用 curl 或 wget 下载 tarball
* 解压
* 安装 Oz / Remote Server 二进制
* 安装 bundled resources

关闭：

```text
SshRemoteServer
remote_codebase_indexing
remote_tty
cloud_runners
cloud_agent_runners
```

删除后普通系统 `ssh` 命令仍然可以在本地终端中使用，只是没有 Warp 自己的远端 Server 增强功能。

---

## 十二、Voice Input

删除：

```text
app/src/server/voice_transcriber.rs
app/src/ai/voice/
voice_input 依赖和 Feature
ServerVoiceTranscriber
VoiceTranscriber
```

当前初始化：

```rust
VoiceTranscriber::new(
    Arc::new(ServerVoiceTranscriber::new(server_api.clone()))
)
```

如果未来只想保留本地语音识别，应单独实现：

```text
LocalVoiceTranscriber
```

例如调用本机 Whisper，不经过 ServerApi。

---

## 十三、其他会主动联网但不一定属于“上报”的入口

你的要求是本地使用，所以这些也应处理。

## 1. 在线字体和资源缓存

启动代码中有：

```rust
ctx.set_fallback_font_source_provider(|url| {
    asset_cache::url_source(url)
});
```

改为：

```text
只加载内置字体
只加载本地字体
远程 URL 返回拒绝错误
```

## 2. LSP 自动下载

涉及：

```text
crates/lsp/src/install.rs
crates/lsp/src/servers/rust.rs
crates/lsp/src/servers/go.rs
crates/lsp/src/servers/clangd.rs
crates/lsp/src/servers/pyright.rs
crates/lsp/src/servers/typescript_language_server.rs
```

本地 LSP 本身可以保留，但必须：

```text
不自动下载
只查找 PATH 中已安装的 Language Server
缺失时提示用户手工安装
```

## 3. 打开网页

以下行为不一定上传数据，但会打开浏览器或访问外部站点：

```text
Privacy Policy
Data Management
登录/退出
Docs
GitHub Issue
Warp Drive Web Link
Oz Dashboard
```

删除相关菜单和 UI，可以避免误触。

## 4. 插件和 Bundled Skills

部分 Skill 可能包含：

```text
curl
wget
GitHub API
Warp Factory
Slack
Webhook
云 Agent
```

本地构建不要默认加载 Warp 官方在线 Skill。只加载你自己审查过的本地 Skill。

---

## 十四、建议保留的本地核心模块

以下是你想要的终端、分栏、文件浏览、文本预览的主要基础。

## 可以保留

```text
app/src/terminal/
    但排除 shared_session
    排除 Warp Agent 专用 View
    排除 cloud/telemetry hooks

crates/warp_terminal/

crates/warpui/
crates/warpui_core/
crates/warpui_extras/
    但关闭 telemetry 子模块

app/src/editor/
app/src/code/
    但移除 lsp_telemetry 和在线 LSP 下载

app/src/persistence/
    本地 SQLite

app/src/settings/
    排除 cloud_preferences_syncer
    排除云端 Privacy Sync

app/src/pane_group/
app/src/tab/
app/src/tab_configs/
app/src/themes/
app/src/keyboard/
app/src/keybindings 相关
app/src/default_terminal/
app/src/shell_indicator/
app/src/file/search 相关
app/src/workflows/local_workflows/
```

## `app/src/terminal/` 内建议排除

```text
terminal/shared_session/
terminal 中的 Agent Cloud 会话入口
遥测调用
共享 Block
云端 Session Restore
```

普通 PTY、Shell、Block 渲染、分栏和标签页可以保留。

---

## 十五、推荐的删除/替换顺序

不要先物理删除几百个文件。按这个顺序改，最容易保持可编译。

## 第 1 步：创建 `offline_hard`

修改：

```text
app/Cargo.toml
crates/warp_core/Cargo.toml
crates/http_client/Cargo.toml
crates/websocket/Cargo.toml
```

加入跨 crate Feature：

```text
offline_hard
```

---

## 第 2 步：建立网络硬拒绝

修改：

```text
crates/http_client/src/lib.rs
crates/websocket/src/native.rs
```

规则：

```text
Warp 主进程只允许 loopback
所有其他 HTTP/SSE/WebSocket 均拒绝
```

这一步完成后，即使后面漏删了模块，也不会真正外发。

---

## 第 3 步：创建真正的 Offline Channel

修改：

```text
crates/warp_core/src/channel/config.rs
crates/warp_core/src/channel/state.rs
app/src/bin/local_only.rs
```

让 Offline Channel 不存在：

```text
server_config
oz_config
firebase_api_key
rtc_server_url
session_sharing_server_url
```

---

## 第 4 步：拆分启动流程

修改：

```text
app/src/lib.rs
```

拆成：

```text
initialize_common_app
initialize_local_app
initialize_cloud_app
```

本地构建不执行：

```text
ServerApiProvider
AuthManager
TelemetryCollector
Crash Reporting
Server Experiments
Cloud Sync
Agent Cloud
Shared Sessions
Autoupdate
IAP
Cloud MCP
Voice
```

---

## 第 5 步：先删除叶子型模块

这些最容易删，不太影响本地终端：

```text
app/src/server/telemetry/
app/src/crash_reporting/
app/src/tracing/native.rs
app/src/tracing/cloud_agent_auth.rs
app/src/autoupdate/
app/src/server/experiments/
app/src/server/iap_identity_minter.rs
app/src/server/voice_transcriber.rs
app/src/terminal/shared_session/
app/src/ai/ambient_agents/
app/src/ai/cloud_environments/
```

---

## 第 6 步：删除云 Server 和认证

处理：

```text
ServerApiProvider
AuthManager
UserWorkspaces
TeamUpdateManager
ManagedSecretManager
IapManager
```

对仍需要身份类型的本地 UI，提供小型本地空实现。

---

## 第 7 步：删除 Warp Agent 和 Drive

如果你确定只运行外部 CLI Agent：

```text
编译关闭 app/src/ai/
编译关闭 app/src/ai_assistant/
删除 app/src/drive/
删除 cloud sync
```

如果想保留 Agent UI，再把它与本地 Agent Runtime 重新连接，不保留 Warp Cloud Client。

---

## 第 8 步：清理 Cargo 依赖

最终 `cargo tree` 中尽量不再出现：

```text
sentry
opentelemetry
opentelemetry-otlp
warp_server_client
warp_multi_agent_client
cloud_object_client
session-sharing-protocol
firebase
warp_managed_secrets
```

如果为了少量共享数据类型暂时还在依赖树里，也要保证：

```text
没有初始化
没有 Client
没有网络出口
没有后台任务
```

---

## 十六、代码检索清单

改造过程中使用下面几组检索。

## 查所有 Warp 云域名

```bash
rg -n \
  'app\.warp\.dev|rtc\.app\.warp\.dev|sessions\.app\.warp\.dev|oz\.warp\.dev|releases\.warp\.dev' \
  app crates resources
```

## 查所有上报和追踪

```bash
rg -n \
  'send_telemetry|record_telemetry|TelemetryCollector|TelemetryApi|Rudder|rudder|sentry::|crash_reporting|opentelemetry|OTLP|WARP_CLOUD_AGENT_OTLP_ENDPOINT' \
  app crates
```

## 查所有云 Client

```bash
rg -n \
  'ServerApiProvider|ServerApi::|AuthManager|ManagedSecretsClient|ManagedMcpClient|ObjectClient|TeamClient|WorkspaceClient|AIClient' \
  app crates
```

## 查所有网络调用

```bash
rg -n \
  'reqwest::|http_client::Client|websocket::connect|eventsource\(|connect_async|TcpStream|curl|wget|asset_cache::url_source' \
  app crates
```

## 查所有云同步

```bash
rg -n \
  'SyncQueue|UpdateManager|Listener::new|CloudPreferencesSyncer|initialize_cloud_preferences_syncer|rtc_server|session_sharing' \
  app crates
```

## 查所有 Feature

```bash
rg -n \
  'agent_mode|cloud_|ambient_|shared_|telemetry|crash_reporting|autoupdate|mcp_oauth|factory_mcp|voice_input|remote_codebase' \
  app/Cargo.toml crates
```

---

## 十七、首次真实使用前的验证标准

不要在带有真实 SSH Key、云凭据和项目代码的日常账号里第一次测试。

## 构建验证

```bash
cargo tree -p warp \
  --no-default-features \
  --features local_only \
  | rg 'sentry|opentelemetry|warp_server_client|warp_multi_agent_client|cloud_object_client|firebase|session-sharing'
```

理想结果为空。

如果不是空，需要判断它只是未使用的数据类型依赖，还是仍包含运行时 Client。

## 二进制字符串检查

```bash
strings target/release/warp-local-only \
  | rg 'app\.warp\.dev|rtc\.app\.warp\.dev|sessions\.app\.warp\.dev|oz\.warp\.dev|rudder|sentry|OTLP'
```

理想结果为空。

## 运行测试场景

至少覆盖：

```text
启动应用
打开/关闭多个标签页
分栏
运行命令
打开文件树
预览文本
打开设置
窗口失焦/重新聚焦
电脑睡眠/唤醒
关闭窗口
退出应用
崩溃一次测试构建
运行十分钟空闲
```

这些场景尤其重要，因为原版在：

```text
启动
焦点变化
退出
崩溃
定时器
更新检查
```

都有潜在外发入口。

## 网络验证

检查 Warp 主进程：

```bash
lsof -nP -iTCP -iUDP | grep -i warp
```

并使用系统级出站拦截作为第二道防线：

```text
WarpLocalOnly 主进程：禁止所有外网
允许 127.0.0.1 / ::1
Shell、git、ssh 等子进程根据需要单独授权
```

最终合格标准不是“请求失败”，而是：

```text
没有 DNS 查询
没有 TCP 外连
没有 UDP 外连
没有 WebSocket
没有后台重试
没有待发送遥测文件
没有 Sentry/OTel 初始化
```

---

## 最终模块处置表

| 模块                                      | 处理                            |
| --------------------------------------- | ----------------------------- |
| `app/src/server/telemetry/`             | 直接删除                          |
| `crates/warp_core/src/telemetry.rs`     | 先改空宏，后删除                      |
| `crates/warpui_core/src/telemetry/`     | 删除或本地空实现                      |
| `app/src/crash_reporting/`              | 直接删除                          |
| `app/src/tracing/native.rs`             | 删除                            |
| `app/src/tracing/cloud_agent_auth.rs`   | 删除                            |
| `app/src/autoupdate/`                   | 直接删除                          |
| `app/src/server/server_api*`            | Offline 构建不编译，后删除             |
| `app/src/server/cloud_objects/`         | 删除                            |
| `app/src/server/sync_queue.rs`          | 删除                            |
| `app/src/server/experiments/`           | 删除                            |
| `app/src/server/iap_identity_minter.rs` | 删除                            |
| `app/src/server/voice_transcriber.rs`   | 删除                            |
| `app/src/auth/`                         | 删除云认证；暂留本地身份空实现               |
| `app/src/ai/ambient_agents/`            | 删除                            |
| `app/src/ai/cloud_*`                    | 删除                            |
| `app/src/ai/artifacts/`                 | 删除                            |
| `app/src/ai/agent_sdk/`                 | 不用内置 Agent 时删除                |
| `app/src/ai/blocklist/`                 | 不用内置 Agent 时删除；否则改接本地 Runtime |
| `app/src/ai_assistant/`                 | 删除                            |
| `app/src/drive/`                        | 不需要本地 Notebook/Workflow 时删除   |
| `app/src/cloud_object/`                 | 需要本地对象时改本地存储；否则删除             |
| `app/src/terminal/shared_session/`      | 删除                            |
| `app/src/remote_server/`                | 删除                            |
| `crates/remote_server/`                 | 删除                            |
| Cloud MCP / Gallery / OAuth             | 删除                            |
| File-based stdio MCP                    | 可保留                           |
| `crates/http_client`                    | 保留，但增加非 loopback 硬拒绝          |
| `crates/websocket`                      | 可删除；保留时增加非 loopback 硬拒绝       |
| Terminal / PTY / Tabs / Panes           | 保留                            |
| File Tree / Editor / Preview            | 保留                            |
| Local SQLite / Settings                 | 保留，删除云同步                      |
| Local Workflows                         | 保留                            |
| LSP                                     | 保留运行时，关闭自动下载                  |

最重要的三处不是某个遥测开关，而是：

```text
app/src/lib.rs                 —— 阻止云模块初始化
crates/http_client/src/lib.rs  —— 阻止所有 HTTP/SSE 外发
crates/websocket/src/native.rs —— 阻止所有外部 WebSocket
```

只有这三层同时完成，再配合 `ChannelConfig::Offline` 和 `--no-default-features`，才能把“没有上报”从配置约定变成代码级保证。


对这个项目，现在采用这种授权结构

warp-local/
├── LICENSE-AGPL
├── LICENSE-MIT
├── LICENSES.md
│
├── app/                     # AGPL
├── crates/
│   ├── warp_terminal/       # AGPL
│   ├── warp_core/           # AGPL
│   ├── warpui/              # MIT
│   ├── warpui_core/         # MIT
│   │
│   ├── local_agent_bridge/  # 你自己的 MIT
│   ├── local_fs_service/    # 你自己的 MIT
│   └── agent_protocol/      # 你自己的 MIT

将来采用这种授权结构
your-project/
├── app/                    # 你重新实现 → MIT
├── warp_terminal/          # 你重新实现 → MIT
├── warp_core/              # 你重新实现 → MIT
├── local_agent_bridge/     # 你实现 → MIT
├── agent_protocol/         # 你实现 → MIT
├── local_fs_service/       # 你实现 → MIT
│
├── warpui/                 # Warp 原版 MIT
└── warpui_core/            # Warp 原版 MIT 调研结果 这里