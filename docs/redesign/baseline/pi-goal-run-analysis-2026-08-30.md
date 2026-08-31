# Pi Goal 任务运行分析（2026-08-30）

## 1. 分析范围

本报告针对 `term4u` 仓库中自 2026-08-30 01:00 PDT 左右启动的 Pi Goal 任务，判断其运行健康度、实际进展和耗时来源。

分析时只进行了只读诊断，没有中断或修改正在运行的 Pi 任务。主要数据源：

- Pi session：`~/.pi/agent/sessions/--Users-ted-workspace-term4u--/2026-08-30T07-54-15-710Z_01a051a9-771e-7588-b5ec-65dfa93b77ae.jsonl`
- Goal debug log：`.pi/goal-events.jsonl`
- 构建日志：`/tmp/term4u-*.log`、`/tmp/r3-*.log`、`/tmp/r4-*.log`
- Git worktree 状态、文件修改时间和实时进程信息

统计拆分的固定窗口为：

- Goal 创建：2026-08-30 01:00:52 PDT
- 窗口结束：2026-08-30 06:07:56 PDT
- 总时长：5 小时 07 分 04 秒

运行健康状态随后继续观察至 06:16 PDT。

## 2. 总体结论

任务整体运行正常，持续产生有效进展，没有卡死。

- Goal phase 始终为 `active`，未进入 `blocked`、`paused` 或 `no-progress`。
- 已发生 4 次自动续跑，每次 `noProgressStreak` 都为 0。
- 文件修改、检查、构建和运行时验证贯穿整个观察窗口。
- 06:07 启动的 `term4u-tui` 构建于 06:13 成功，用时 5 分 28 秒；构建结束后模型立即继续检查和编辑，Goal 日志更新到至少 06:16:30。
- 任务尚未完成最终验收，C1/C2 仍为 `unverified`。

这里的 `unverified` 不等于没有进展。当前 Goal Extension 是 M1 控制面，criteria 通常要到最终完成申请时才会整体审计，不会自动把每个中间里程碑标记为 passed。

## 3. 已完成和正在推进的工作

根据 session 中各轮阶段性总结、实际文件及验证日志，进展大致如下。

### 3.1 已完成或基本完成

1. M0 基线文件、测试清单和许可证门禁脚本已经建立。
2. M1 `offline_hard` 网络硬拒绝、离线 resolver 和相关守卫测试已经实现。
3. M2 `ConnectivityMode`、`term4u` / `term4u-tui` 双入口及 feature 改造已经实现。
4. M3 `ServerApiProvider` 双后端、13 个 trait / 178 个方法的 `OfflineApi` 和本地初始化路径已经实现。
5. M4a 遥测宏 no-op、Rudder/TelemetryCollector/event store、OTLP、Sentry 等出网链路已经大量拆除。
6. M4b GUI/TUI 自动更新、changelog、远程实验、IAP、语音转录等实现已经删除或替换为离线兼容状态。
7. 多组定向测试和构建已经出现退出码 0，包括网络守卫测试、`term4u` check 和多次 `term4u-tui` build。

### 3.2 正在推进

1. M4c/M4d 的共享会话、remote server、ambient/cloud environment 等剩余拆除。
2. TUI 实际启动路径中的本地 singleton 注册和云依赖消除。
3. dead code、warning、clippy 和完整验收收敛。
4. 最终 A1–A12 验收和 C1/C2 证据整理。

### 3.3 Worktree 规模

观察时 Git 变更规模为：

- 304 个 tracked 文件发生变化
- 3,545 行新增
- 32,778 行删除
- 状态分类约为 155 个删除、149 个修改、33 个 untracked
- 仓库 tracked 文件约 6,285 个，其中 Rust 文件约 4,016 个
- `target/` 目录约 38GB

这说明任务不是停留在分析阶段，而是在进行大规模实现和删除工作。

## 4. 运行连续性

Goal 自动续跑边界如下（PDT）：

| 阶段 | 时间范围 | 大致时长 |
|---|---|---:|
| 初始 Goal run | 01:00:52–03:15:51 | 2 小时 15 分 |
| 自动续跑 1 | 03:15:51–04:18:22 | 1 小时 03 分 |
| 自动续跑 2 | 04:18:22–05:13:40 | 55 分钟 |
| 自动续跑 3 | 05:13:40–06:01:54 | 48 分钟 |
| 自动续跑 4 | 06:01:54 后 | 分析结束时仍在运行 |

4 次 `agent_settled → send_continuation → agent_start` 的调度切换合计约 9 毫秒，每次约 2–3 毫秒，没有可见的 Goal 调度等待。

运行过程并非每一步都单调成功：日志中有多次编译失败、精确 edit 匹配失败和运行时 singleton 缺失，随后继续修复和重跑。整体轨迹属于持续的“修改—检查—修复—重建”，而不是空转。

## 5. 墙钟时间拆分

在 5 小时 07 分 04 秒的固定统计窗口内：

| 耗时部分 | 时间 | 占比 |
|---|---:|---:|
| LLM/API/agent 请求准备、推理和响应 | 2 小时 55 分 19 秒 | 57.1% |
| 工具执行（按并行区间合并） | 2 小时 11 分 45 秒 | 42.9% |
| 其中 build/test/format | 约 2 小时 03 分 | 约占总时间 40%，占工具时间 93% 以上 |
| Goal 自动续跑调度 | 约 9 毫秒 | 可忽略 |

“LLM/API/agent”包含请求准备、远端推理、流式响应和少量本地处理，不能完全等同于纯模型服务器计算时间，但能代表每次工具结果返回后等待下一次 assistant response 的串行阶段。

## 6. LLM 侧耗时判断

任务使用：

- Provider：`openai-codex`
- Model：`gpt-5.6-sol`
- Thinking level：`high`

固定窗口内共有约 747 次底层 assistant response：

- 平均延迟：14.1 秒
- 中位数：12.5 秒
- P90：26.4 秒
- 最大值：84.0 秒

单次调用多数并不异常慢，主要问题是调用数量非常大且完全串行。

### 6.1 上下文膨胀

单次 usage 的总上下文从约 10K 增长到约 809K tokens，期间没有 compaction entry。

延迟变化明显：

- 前两小时平均约 9 秒/次
- 上下文超过约 530K 后，平均约 19–22 秒/次

因此后半程每次 LLM 调用延迟约为前期的两倍。上下文增长来自数百次 read、bash、edit 和编译输出积累，而不是 Goal 状态快照：`custom` Goal entry 不参与模型上下文。

每次 Goal continuation 约 3.1K 字符，当前仅注入 4 次，相比 809K token 上下文可以忽略。

### 6.2 Provider 瞬时错误

观察到十余次：

- `fetch failed`
- `WebSocket error`
- `terminated`

Pi 自身均在约 2 秒后恢复并重新启动低层 agent run。固定窗口内，这部分连同失败请求等待合计只有约 2.5 分钟，不是主要耗时来源。

## 7. 工具和构建侧耗时判断

工具时间的绝大部分用于 Rust 构建、检查、测试和格式化。

典型长任务包括：

- 基线 `cargo nextest list`：约 15 分 25 秒
- 基线 workspace 构建/测试链接：约 15 分 45 秒
- `term4u-tui` 首次阶段构建：约 9 分 48 秒
- 后续增量构建：通常约 2.5–5.5 分钟

后半程为定位 TUI 启动时缺失的本地 singleton，进行了多轮“修改一个启动问题—重建二进制—实际启动—发现下一个问题”。这些重建每次都会重新编译巨型 `warp` crate，是工具侧耗时的主要来源。

日志还显示部分 Cargo 命令并行运行并共享同一 target，出现：

```text
Blocking waiting for file lock on package cache
Blocking waiting for file lock on build directory
```

这会造成一定锁竞争。并行命令缩短了部分批次的总墙钟，但并非所有并行构建都真正提高吞吐。

## 8. Goal 模式是否是速度瓶颈

### 8.1 不是直接速度瓶颈

从调度时序看，Goal 自动续跑的直接开销接近零：4 次续跑切换合计约 9 毫秒。

Goal 状态以 `custom` entry 持久化，不进入 LLM 上下文；debug log 和 session 状态写入的数据量也不足以解释小时级耗时。

Goal 模式的主要作用是：

- 自动继续下一轮；
- 反复注入完整目标和验收约束；
- 要求在证据不足时继续，而不是在某次 build 通过后提前结束。

因此它会让任务持续更久，但这是执行范围和质量要求导致的总工作量增加，不是调度器阻塞。

### 8.2 Goal 控制面存在的问题

#### 自动轮次口径较粗

状态显示 `autoTurnsUsed=4/20`，但内部已经发生约 747 次 LLM response。`autoTurnsUsed` 只统计高层 continuation，不统计一个 agent run 内部的工具循环。

因此“20 轮”并不能约束底层 LLM 调用次数，也不能代表剩余运行时间。单个自动轮可以持续一小时并包含上百次模型调用。

#### 没有墙钟硬限制

当前契约预算为：

- 自动续跑：20 轮
- Token：30,000,000

没有 wall-clock hard cap。观察末尾 Goal 计费约为 4.73M/30M，尚不足 16%，所以从预算上允许继续运行很久。

#### 墙钟统计严重少算

实际运行已超过 5 小时，但 Goal 状态在约 06:05 只记录：

```text
timeUsedMs = 2,620,920
```

即约 43 分 41 秒。

原因可由 Goal Extension 实现解释：

- 每次 `agent_start` 都覆盖 `runStartedAt`
- 只在最终 `agent_settled` 时累计 `Date.now() - runStartedAt`
- Pi 的 provider 自动重试会再次触发 `agent_start`
- 重试前已经消耗的整段时间因此被丢失

所以当前 wall time 只能算作不可靠的展示值，不能用来判断任务是否运行过久。

#### 初始估算偏差过大

契约估算为：

- Size：L
- 8–16 turns
- 20K–50K tokens

实际 Goal 计费已约 4.73M tokens，超过估算上限约 95 倍。该 estimate 是 advisory，不直接限制执行，但不能为用户提供可靠的成本预期。

## 9. 根因归类

### Goal 模式

- 不是墙钟耗时的直接瓶颈。
- 但自动轮次口径过粗、无墙钟上限、wall time 少算、估算严重偏低，使任务看起来“只跑了 4 轮”，实际上已经执行了数百次底层调用并持续数小时。

### LLM/agent

- 是最大的直接墙钟时间来源，占约 57%。
- 主要原因不是单次请求异常，而是 747 次串行调用、`high` reasoning 和 809K 上下文。
- Provider 瞬时错误存在，但占比很低。

### 任务本身

- 是调用数量和构建数量大的根本驱动因素。
- 详细设计覆盖 M0–M4、跨 crate feature、178 个 OfflineApi 方法、大规模模块物理删除、运行时验证和 A1–A12 验收。
- 当前已修改三百多个文件，并产生 38GB 构建目录，5 小时属于可解释范围。

综合判断：

> 任务本身规模巨大，模型采用细粒度串行工具循环，Rust 构建又很重；Goal 模式让它持续追求完整验收，但 Goal 调度器本身没有造成显著延迟。

## 10. 完成风险

`docs/redesign/baseline/README.md` 和 `risk-log.md` 明确记录 M0 尚未关闭：

- 主机缺完整 Xcode/Metal，首次全量构建退出 101
- 全量测试链接曾因 57GiB 可用磁盘耗尽退出 101
- GUI 手工场景和真实旧 DB 样本尚未完整建立

虽然当前定向 check、TUI build 和守卫测试已经通过，但最终 A1–A12/C2 要求全部适用验证退出码为 0。若完整 Xcode/Metal 环境未补齐，这可能成为最终验收的真实阻塞点。

## 11. 优化建议

### 11.1 当前任务

1. 在合适的稳定点执行 `/goal pause` → `/compact` → `/goal resume`，降低约 809K 的上下文。
2. 编译修错阶段将 thinking 从 `high` 降到 `medium`，最终审计阶段再恢复 `high`。
3. 避免多个 Cargo 命令并行共享同一 target；优先合并为一个检查命令或顺序使用增量缓存。
4. 将修改批量收敛后再进行完整 build，减少每修一个 singleton 就重编译巨型 crate 的次数。
5. 补齐完整 Xcode/Metal 和足够磁盘，避免最终门禁因基础设施无法完成。
6. 关注 30M token / 20 auto-round 的宽松预算；当前配置仍允许继续运行很多小时。

### 11.2 后续类似任务

1. 将 M0、M1、M2、M3、M4 拆为独立 milestone goal，减少单 session 上下文膨胀。
2. 每个 milestone 使用机械化验收标准并在完成后创建新 session/handoff。
3. 为 Goal Extension 增加 wall-clock hard cap 或软停阈值。
4. 修复 provider retry 导致 `timeUsedMs` 少算的问题。
5. 额外展示底层 LLM turn/tool-call 数，避免只显示高层 auto-round。
6. 在上下文达到 300K–500K 时提示或自动触发 compaction。
7. 改进 estimate，至少根据仓库规模、criteria 数量、验证命令和历史平均每轮消耗给出量级合理的区间。

## 12. 最终判断

1. **任务没有卡死，进度一直在整体推进。**
2. **LLM/agent 串行阶段是最大直接耗时，约占 57%。**
3. **Rust build/test 占约 40% 总墙钟，是第二大耗时。**
4. **Goal 自动续跑调度开销可忽略，不是速度瓶颈。**
5. **Goal 的预算口径、墙钟统计和初始估算存在明显可观测性问题。**
6. **任务规模和基础设施条件决定了 5 小时并不反常，但最终完整验收仍有环境阻塞风险。**

## 13. 10:51 PDT 复评更新

> 本节是对前述 06:07:56 固定窗口结论的增量复评，不改写原始观察值。
> 新快照采集于 2026-08-30 10:51 PDT；任务仍在运行，数据会继续变化。

### 13.1 更新结论

任务仍未卡死，而且 06:07 后完成了大批真正的物理删除；当前工作重心已经从
TUI 启动链 singleton 修复，转为 M4 大规模删除后的测试编译与引用收敛。

截至快照时：

- Goal 仍为 `active`，`autoTurnsUsed=4/20`，没有新增高层 continuation；同一个
  auto round 已继续执行数百次底层响应。
- Goal 计费约为 6.51M/30M tokens，C1/C2 仍为 `unverified`。
- 当前仍有活跃 `rustc` 进程，session 和 Goal debug log 持续更新。
- 任务尚未进入 A1–A12 的最终逐项验收阶段。

### 13.2 相对 06:07 的进展

| 指标 | 06:07 快照 | 10:51 快照 |
|---|---:|---:|
| Goal 总运行墙钟 | 5 小时 07 分 | 约 9 小时 51 分 |
| tracked 变更文件 | 304 | 786 |
| untracked 文件 | 33 | 51 |
| diff 新增/删除 | +3,545 / -32,778 | +3,970 / -149,063 |
| `target/` | 约 38GB | 约 63GB |
| 底层 assistant response | 约 747 | 约 1,427 |
| 已完成工具调用 | 约 868 | 约 1,559 |
| Goal token 计费 | 约 4.73M | 约 6.51M |

已经确认消失的主要路径包括：

- `app/src/autoupdate`
- `app/src/remote_server`
- `crates/remote_server`
- `app/src/ai/ambient_agents`
- `app/src/ai/cloud_environments`
- `crates/computer_use`（类型边界改由本地小 crate 承担）
- `crates/warp_tui/src/autoupdate.rs`
- 原 `app/src/terminal/shared_session`

`app/src/terminal/session_sharing` 仍保留约 29 个文件，说明共享会话相关收敛尚未完全结束。
遥测目录已经大幅缩小，但仍存在少量本地兼容/日志相关文件，不能仅按目录名判断 M4a
已完成最终审计。

新增的成功证据包括：

- `cargo check -p warp --bin term4u --no-default-features --features local_only` 成功，
  用时约 8 分 56 秒；
- `cargo check -p warp --bin term4u --features offline_hard` 成功，用时约 7 分 22 秒；
- `term4u-tui` 多次增量构建成功；
- TUI 8 秒 smoke test 退出码为 0，采样时 lsof 输出为空。

测试编译也在明确收敛：

- 第一轮 `cargo test -p warp --lib --features offline_hard --no-run` 有 71 个错误；
- 10:40 左右降至 8 个错误；
- 10:51 前的最新一轮降至 1 个错误、26 个 warning，但仍以退出码非 0 结束。

因此这不是空转，但“主二进制能 check/TUI 能启动”仍不能替代完整自测通过。

### 13.3 新增窗口的墙钟拆分

从原报告固定截止点 06:07:56 到本次 session 最新事件 10:51:24，共约 4 小时 43 分：

| 耗时部分 | 时间 | 占比 |
|---|---:|---:|
| LLM/API/agent 串行阶段 | 约 3 小时 09 分 | 66.6% |
| 工具执行（并行区间合并） | 约 1 小时 35 分 | 33.4% |
| build/test/format | 约 1 小时 31 分 | 占工具时间约 96% |

该窗口新增约 681 次 assistant response 和 691 次已完成工具调用。assistant response
平均等待约 16.7 秒，中位数约 14.5 秒。工具调用中包含约 43 个 Cargo/Rust/format
相关命令。

与前五小时相比，直接耗时进一步向 LLM 串行阶段倾斜。原因不是单次调用突然失常，
而是物理删除期间继续采用“搜一个符号—读一个文件—改一块—再检查”的细粒度循环。

### 13.4 compaction 的效果和再次膨胀

06:35 PDT 左右发生了一次 compaction：

- compaction 前上下文约 850K tokens；
- compaction 后第一轮降至约 34K；
- 随后约四小时内又增长到 620K 以上。

延迟也随之变化：

- compaction 前的新增窗口平均约 23.5 秒/response；
- compaction 后前两小时约 14.8 秒；
- 后两小时重新升至约 17 秒。

这证明 compaction 有效，但单个超大 Goal 在同一 session 内继续进行数百次 read/edit/build
后，上下文会很快重新积累。10:40 左右还出现一次 prompt cache miss，单次重新计入约
614K input tokens，使 Goal 计费从约 5.88M 跳至约 6.49M。

### 13.5 当前真正慢点

1. **LLM 的细粒度串行修改仍是最大直接墙钟来源。** 新增窗口约占 67%。
2. **Rust 重编译是第二大来源。** 结构性删除频繁改变 `warp` 巨型 crate，每批修复都要
   重新 check/build/test-link；新增窗口工具时间几乎全部花在这里。
3. **进入了高返工密度的测试接线阶段。** 批量删除模块及测试后，需要逐项清理 test-only
   re-export、`#[path]` 模块、feature 组合和残余类型引用。错误数 71→8→1 表明在收敛，
   但仍未达到测试可编译。
4. **warning 和验收债务尚未处理。** 最近成功的 app check 仍报告约 166–250 个 warning；
   尚未生成 `tests-m4.txt`、`dead-code-m4.txt` 或最终验收证据。
5. **磁盘风险上升。** `target/` 已从约 38GB 增长到约 63GB，磁盘剩余约 128GB；后续
   release 和全量 test-link 仍可能再次触发空间问题。

Goal 调度器依然不是速度瓶颈。`autoTurnsUsed` 仍显示 4，并不是任务没有继续，而是 06:01
之后的单个高层 run 一直没有 settled；这个 run 内已经容纳六百余次底层模型响应。

### 13.6 对完成度的重新判断

完成度必须拆开评估，不能仅按删除行数判断：

- 结构性实现与物理删除：约 **80%–90%**；
- 编译、测试和 warning 收敛：约 **40%–55%**；
- A1–A12 正式验收：**低于 20%**；
- 按 Goal 最终合同口径综合估计：约 **65%–75%**。

尚未看到以下最终证据：

- A1 `local_only` release build；
- A2 正式 debug build（当前成功记录主要是 `cargo check`）；
- 当前 worktree 上的完整 presubmit；
- `test_inventory diff baseline m4`；
- release 产物的 60 秒零外连和零 DNS；
- A3 手工 13 场景、A6/A7、旧 DB、A11 本地日志三项；
- 最终许可证边界复验及 C1 逐条追踪清单。

综合判断更新为：

> 代码改造主体已经越过中点并接近 M4 结构删除尾声，但当前只接近“实现大体成形”，
> 尚未接近“合同可宣告完成”。短期瓶颈是最后的测试编译错误和 warning/clippy 收敛；
> 随后的主要耗时会转为 A1–A12 的 release、全量测试和运行时证据。完整 Xcode/Metal、
> release GUI 和全量链接环境仍可能成为最终硬阻塞。
