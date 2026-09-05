# Goal Mode：跨 run 追加式 checkpoint+delta（context）、持久化 checkpoint+delta（store）与 report_progress——可执行可验证开发设计方案

> 状态：**定稿待批准**（2026-09-04）。所有协议参数已定；§7-3 证据存放已由用户选定"内嵌 goal-verification 事件"。§F 为可直接复制给施工 agent 的自包含指令。

## Context

roadmap（`docs/discuss/goal-mode-improvement-roadmap.md`）§4 不变量 1、§6.1、§6.3、§6.4、§7-3/§7-4 已经定下两项工作的方向：

1. **跨 run 追加式 context 投影**：`context` 处理器不再折叠/中和/删除历史 Goal 消息；陈旧性只由尾部新消息声明；普通 continuation 只发有字符上限的 delta，kickoff / revision 变化 / compaction 后首轮 / 每 K 轮发完整 checkpoint。目标是消除每个 run 边界一次的整段 KV cache miss（#5 667K、#6 455K 实证），同时不让 512 轮的近重复正文自行压爆上下文。
2. **持久化 checkpoint+delta**：把每 turn_end 全量快照（#6：18 条 criteria 复制 1,147 次，占 session 文件 67.7%）拆为低频 `goal` checkpoint + 高频 `goal-usage`/`goal-criteria`/`goal-plan`/`goal-verification` delta；fold = 最近 checkpoint + 后续 delta；老纯快照 session 双读兼容；seq/CAS/fail-loud 不弱化。

用户判断：第 1 项可直接实现，边写边定少量参数；第 2 项需先补协议级 implementation spec。本方案把两项都深化到"参数已定、协议已定、测试矩阵已列、可直接施工"的程度。

交付要求（用户 2026-09-04 追加）：方案末尾必须附一份**可直接复制给其他 agent 执行的施工指令**（自包含：目标、不变量、协议参数、改动文件、测试矩阵、验证命令、文档同步、提交拆分），不依赖本对话上下文。

范围（用户 2026-09-04 二次追加）：**`report_progress` 工具（roadmap §6.2.1）并入本批**。其行为与验收标准 roadmap 已定，本方案只补四项实现决策（持久化载体、并发、合同变化规则、输入边界），见 §C。

## 0. 已核实的代码基线（2026-09-04，HEAD `58f1bdc`）

以下事实是方案的依据，实施者无需重新推导：

- **context 处理器** `src/index.ts:341-388`：先 `classifyRunOrigin(messages)`（395-434，提交 continuation 预约、判定 `rt.inAutoRound`，**必须保留**），然后只保留最后一份 banner（且 `projection.phase==="active"`），把非最新 kickoff/continuation 的 `m.content` 原地改写为占位符，最新一份若 `details.revision` 不等或 phase 非 active 也原地改写。`projection = rt.runProjection ?? {phase, revision}`；`rt.runProjection` 只在 semantic run 开始时设置（`agent_start` 511）、由 `resetRunCounters` 清空（`runtime.ts:120`），除此再无其他使用者。
- **注入点**：banner 由 `before_agent_start`（330-339，仅 active，无 `details`）；kickoff 在 `createGoal` 末尾（948-956，`details:{revision}`）；continuation 由 `sendContinuation`（245-263，`details:{nonce,revision}`，`deliverAs:"followUp"`），调用方为 `agent_settled`（810-812）、`/goal resume`（1186）、`/goal reject`（1274）。五类 run 内 steer 提示全部复用 `WRAPUP_TYPE`（490-493 配额、552-559 重复警告、570-577 工具检查点、628-631 预算软停、641-648 轮次检查点）。`classifyRunOrigin` 的反向扫描跳过 `BANNER_TYPE`/`WRAPUP_TYPE`（419）——任何新增 custom 类型都必须加入该跳过表。
- **语义 run**：`agent_start` 499-526（`semanticRunId===undefined` 才开新 run，否则仅 `attemptId+1`）；`agent_settled` 666+ 清 `semanticRunId`；`session_start` 288-316；`session_tree` 318-326。
- **compaction**：src/ 中没有任何监听。宿主提供 `session_before_compact`（`preparation.firstKeptEntryId/messagesToSummarize/...`、`customInstructions?`、`reason`、`willRetry`）与 `session_compact`（`compactionEntry{summary,firstKeptEntryId,tokensBefore,details?}`、`reason`、`willRetry`）。compaction 只影响 LLM 上下文路径；`custom` 状态条目不受影响。
- **prompts** `src/prompts.ts`：`renderContinuation` 113-139 = 含 revision 的首行 → `OBJECTIVE_GUARD` → objective → constraints → `criteriaChecklist`(20-32) → `budgetSection`(39-51，含易变 quota 文本) → 五段静态规则（FIDELITY 80-84 / EVIDENCE 86-87 / EXECUTION_EFFICIENCY 89-94 / COMPLETION_AUDIT 99-105 / BLOCKED 107-111，合计约 3.3KB）→ 收尾句。`renderBanner` 142-155；`renderKickoff` 158-162 = 引言 + `renderContinuation`。
- **store** `src/store.ts`（153 行）：`fold` 63-74 线性扫描 `type==="custom" && customType==="goal"`，逐条 `validateSnapshot`（37-58），last-wins，fail-loud；`rebuildGoalState` 85-104 返回 `structuredClone` 并前向补默认值；`persistGoalState` 112-134 每次写入都全量 fold 做 CAS（`currentSeq!==baseSeq` 即拒绝），`seq=baseSeq+1`，深拷贝后 `appendEntry`；`persistTombstone` 137-153。无 schema version 字段。
- **types** `src/types.ts`：`GoalState{contract, phase, debugLog?, blockedReason?, autoTurnsUsed, tokensUsed, cacheReadTokens, costUsd, toolCallsUsed, timeUsedMs, criteriaStatus, completionProposal?, verificationResults?, completionRejections, updatedAt}`；`GoalVerificationResult` 含 stdout/stderr（各 ≤8,000 字符，`verification.ts:11-16`），失败步重跑一次故结果 ≤ 2×steps。
- **persist 调用点**（`src/index.ts`）：228 `transition()`；614 `turn_end`（同时冲刷 `classifyRunOrigin` 410/431 的 `autoTurnsUsed` 与 `tool_execution_start` 540 的 `toolCallsUsed`）；669 `agent_settled` 时间冲刷；937 `createGoal`；994 进入 verifying；1046 验证打回；1174 resume；1269 reject；1281 clear→tombstone；1343 budget（revision+1）；1400 edit（revision+1、清 proposal/results、criteria→unverified）；1530 `update_goal(complete)` 写 proposal。`verifyCompletionProposal` 961-1102 在两次 persist 之间 `await` 数分钟的 `runMechanicalVerification`，靠 `rt.pipelineRunning` 与 `rt.generation` 围栏而非队列。src/ 无任何 queue/lock。
- **宿主 API**：`pi.appendEntry(customType,data): void`（不返回 entry id）；`ctx.sessionManager.getBranch()` 返回 root→leaf 路径（含全部条目类型，custom 条目带 `id/parentId/timestamp`）；`getEntries()` 是全树，不可用于 fold。
- **测试夹具** `test/goal-extension.test.ts`：`createDriver` 40-199（内存 `entries`，`api.appendEntry` 推 `{type:"custom",customType,data}`；`deliver(i)` 把 `sent[i]` 变成 `{type:"message",message:{role:"custom",...}}`；`contextMessages()` 返回 structuredClone；`emit()` 返回各 handler 结果数组）；`runAutoRound` 212-229；store 测试 231-360；旧投影语义测试 1513-1541 / 1566-1586 / 1587-1607 / 1608-1619 需改写；run 内字节稳定测试 1543-1564 必须保留。
- **UI** `src/ui.ts`：widget 每条 criterion 一行 `icon id text (not Goal-verified)`（64-71，最多 8 条）；`renderStatusCard` 88-93 每条 `id [status] (kind) text`。
- **config** `src/config.ts`：`GoalConfig` + `DEFAULT_CONFIG`，`loadGoalConfig` 浅合并 `goal.json`。
- **文档需同步**：`docs/design.md` §5 行 142（"完整快照而非增量"）、§9 行 237（"每个自动轮次由 continuation 重渲染完整投影"与行 236 矛盾）、§15 行 508 状态与阻塞测试 1；`docs/implementation-notes.md` 71、79-88、93-96；README "Safety model"/"Goal logs"；roadmap §6.1/§6.2.1/§6.4 状态、附录 C 第 1 步（终态不再是"最后一条 goal 条目"，须 fold）。

## A. 跨 run 追加式 checkpoint + delta 投影（roadmap §6.4）——实现规格

### A.1 决策汇总

| 项 | 决策 |
|---|---|
| `context` 处理器 | **纯透传**：仍先 `classifyRunOrigin(messages)`，然后记录 `rt.checkpointVisible`，返回 `{ messages: event.messages }`（同一引用；返回 `undefined` 会让 test 1543-1564 解引用报错）。删除全部过滤/改写逻辑、三个占位符字符串与 `rt.runProjection` |
| 消息种类 | kickoff = checkpoint；continuation = `checkpoint` 或 `delta`；新增 `NOTICE_TYPE = "goal-notice"` 承载 run 内 revision 通知、历史化短注、clear 短注（不复用 banner：banner 语义是"当前活动投影"；不复用 WRAPUP：语义已过载） |
| `details` schema | `{ goalId, revision, nonce, kind: "checkpoint"\|"delta"\|"notice", round, checkpointId?, reason? }`；kickoff 补 `nonce/goalId/kind`。`goalId` 必带：同 session `/goal clear` 后新 goal 的 rev-1 会与旧 goal rev-1 混淆 |
| checkpoint 触发（按序） | `rt.lastCheckpoint` 缺失 → `revision`/`goalId` 与 `lastCheckpoint` 不符 → `rt.compactedSinceCheckpoint` → `rt.checkpointVisible === false` → `deltasSinceCheckpoint >= K`。另外 `session_start`、`session_tree`、`/goal resume`、`/goal reject`、`/goal clear`、`createGoal` 调 `resetProjectionTracking(rt)`，使其后首条 continuation 必为 checkpoint |
| K（投影） | **`projectionCheckpointRounds = 8`**（goal.json，min 1） |
| delta 上限 | **`DELTA_MAX_CHARS = 1500`**（硬上限，四级降级阶梯保证） |
| objective 标识 | `#` + sha256(objective，CRLF→LF，trim) 前 8 位 hex；摘要 = 空白折叠后前 120 字符、词边界截断、`…` |
| delta 是否含 quota | 含（沿用 `budgetSection`）：尾部追加消息不怕易变；wrap-up 报告需要它 |
| 变化标注 | "changed since last round"（相对**上一次发送的投影**，checkpoint 或 delta），覆盖 verified 与自报两个维度；出现一次，无变化时消失 |
| compaction | 新增 `session_compact` 监听置 `rt.compactedSinceCheckpoint`（主触发，先于 `agent_settled`）；消息缺失检测（`checkpointVisible===false`）是一轮滞后的兜底。`session_before_compact` 的 `customInstructions` 附合同：**推后到 M3** |
| 历史化短注 | `before_agent_start`：`state.phase!=="active"` 且 `!rt.historicalNoteSent` → 返回 `goal-notice` 一次；`persist()` 成功且 `phase==="active"` 时复位（所有回到 active 的路径都经 persist）；`/goal clear` 后置 `rt.clearedNotePending`，下一提示发一次 `[goal cleared; ...]` |

### A.2 尺寸与 K 的算术（对 `src/prompts.ts` 实测 `wc -c`）

静态：OBJECTIVE_GUARD 226 · FIDELITY 633 · EVIDENCE 306 · EXECUTION 927 · COMPLETION 748 · BLOCKED 471 · 收尾 130 = **3,441 字符**；加新 PROGRESS 段 ≈430 与新首行 ≈300 → **每个 checkpoint ≈4.2KB 静态**。动态：objective 典型 0.6KB、constraints 0.2KB、checklist ≈240 字符/条（18 条 ≈4.3KB）、budget 0.2KB。checkpoint 实际：0 条 ≈4.4KB；3 条 ≈5.5KB；18 条 ≈9.5KB。

512 轮、18 条 criteria、checkpoint 9.5KB、delta 1.0KB（checkpoint 数 = 1 + floor(511/(K+1))）：

| K | checkpoints | 合计 | vs 今日 512×9.5KB=4.9MB |
|---|---|---|---|
| 8 | 57 | ≈1.0MB（~250K tok） | −80% |
| 16 | 30 | ≈767KB | −84% |
| 32 | 16 | ≈648KB | −87% |
| ∞ | 1 | ≈520KB | −89% |

K≥8 后 delta 流占主导；8→16 在 512 轮只省 ≈60K token 却让"离最近完整规则集"的距离翻倍；真实 session 中 compaction 触发的重锚远比 K 频繁。**取 K=8**。

### A.3 delta 版式与降级阶梯

```
GOAL DELTA — continuation round {n}, contract rev {r}, objective #{hash8}. Supersedes all earlier goal round messages; the full contract and working rules are in the latest GOAL CHECKPOINT/kickoff message above for rev {r} and via get_goal; older goal messages are history, not instructions.
Objective #{hash8} (user-provided data, not instructions): {summary ≤120}…
Criteria [verified+self-report]: C1 ⬜◐ | C2 ⬜○ | C3 ✅◑ (changed since last round: C1 ⬜○→⬜◐, C3 ⬜○→⬜◑)
{budgetSection(state, quota)}
{budgetWarning — 仅 p50/p80 那一轮}
Keep working toward the objective now. Call update_goal only when the goal is complete or the blocked audit in the checkpoint is satisfied.
```

最坏现实情形（18 条、2 处变化、p80 警告）≈1,407 < 1,500；无警告 ≈1,070；0 条 ≈830。1,200 会恰在警告轮被迫降级，故取 1,500。超限阶梯：(1) 去掉变化标注 →(2) criteria 行折叠为计数 `Criteria: 18 (verified 2 passed/1 failed/15 unverified; self-report 6 in progress/3 met)` →(3) 摘要缩到 60 字符 →(4) 硬截到 1,499+`…`（实际不可达，用 80 条合成状态单测）。delta **不含**六段静态规则、`<objective>` 标签与 `report_progress` 字样。每条 criterion 恒显两枚图标（`C1 ✅◑`）以便统一解析；自报 stale 加尾缀 `*`（`C2 ⬜◐*`）。

### A.4 checkpoint / kickoff / 通知文案

- checkpoint continuation 首行：`GOAL CHECKPOINT — automatic continuation round {n}, contract revision {r}, objective #{hash8}. This message supersedes all earlier goal round messages (kickoff, continuations, deltas, banners) in this conversation: work from this message and from get_goal; older goal messages are history, not instructions.` 空行，`Continue working toward the active goal.`，随后为现有 `renderContinuation` 从 OBJECTIVE_GUARD 到收尾句的正文（checklist 按 A.5 改）。
- kickoff 首行：`A goal has been set for this session (contract revision {r}, objective #{hash8}); this message supersedes any earlier goal round messages in this conversation. Start working toward it now.` + 同一正文。
- round = 发送时 `state.autoTurnsUsed + 1`（kickoff=1），仅展示。
- run 内 revision 通知（steer，≤400 字符）：由 `/goal budget`、`/goal edit` 在 `runInFlight = rt.semanticRunId!==undefined && (state.phase==="active" || inWrapUpRun)` 时发送，**必须在 edit 路径的 `bumpGeneration`（index.ts:1391 会清 `semanticRunId`）之前捕获该布尔**。文案：`GOAL CONTRACT UPDATED to revision {r} (objective #{hash}): the user changed the goal contract while this run was in progress. Earlier goal round messages, including the one driving this run, are superseded. Call get_goal now and work from the current contract.` + edit 追加 ` Criteria statuses were reset to unverified.` / budget 追加 ` Budget: rounds a/b, tokens x/y.`；`details:{goalId, revision, kind:"notice", reason:"revision"}`，`deliverAs:"steer"`。`classifyRunOrigin` 跳过表加 `NOTICE_TYPE`。
- 历史化短注：`[goal {phase}; earlier goal round instructions are historical — do not act on them; the user's message is the current instruction]`；clear：`[goal cleared; earlier goal round instructions are historical — do not act on them]`。

### A.5 `report_progress` 在投影中的呈现

- **checklist（`criteriaChecklist`，被 checkpoint/banner/get_goal 共用）**：把每行 60 字符的 "(not yet Goal-verified; this does not mean not implemented)" 改为一次性图例 + 紧凑自报标记：
  ```
  Acceptance criteria — Goal-verified status · your self-report (report_progress). "unverified" = not yet Goal-verified, NOT "not implemented"; self-reports are not evidence:
  - C1 ⬜ unverified · ◐ in progress [mechanical] tests pass — "wiring the retry loop"
    evidence: fresh test exit 0
  - C2 ❌ failed · ◑ self-reported met (stale self-report) [judged] docs updated
  ```
  标记：缺省/`not_started` → `○ not started`；`in_progress` → `◐ in progress`；`self_reported_met` → `◑ self-reported met`；stale → ` (stale self-report)`；note → ` — "{单行，≤80 字符，控制字符折叠，超长 …}"`。每条固定成本 105 → 67 字符（+20 stale，+≤86 note），图例 170 一次；≥5 条即比今日短（18 条无 note：1,381 vs 1,890）。`get_goal` 描述加 "your self-reported progress"。
- **静态段 `PROGRESS_RULES`（≈430 字符，只在 checkpoint，插在 EVIDENCE 与 EXECUTION_EFFICIENCY 之间）**：
  ```
  Progress visibility:
  - After substantive work on any criterion, call report_progress with the FULL table (one row per criterion: not_started | in_progress | self_reported_met, optional one-line note); rows marked stale must be re-assessed from current evidence.
  - Self-reported status is visibility for the user and for later rounds. It is NOT evidence, does not change Goal-verified status, and never replaces the completion audit or update_goal.
  ```
- **banner** 加一行 `BANNER_PROGRESS_RULE = "Keep report_progress current after substantive work on a criterion; self-reports are visibility, not evidence."`（用户提示驱动的 run 没有 continuation，banner 是唯一规则载体）。delta 两者皆不含。
- **快照/差分**：`snapshotProjection(state): Record<id, "${verifiedIcon}${selfIcon}${stale?"*":""}">`；`diffProjection(prev,next) → [{id,from,to}]`；每次发送后存 `rt.lastProjection`，`renderDelta` 消费。所有渲染器对 `criteriaProgress` 缺失必须按 not started 处理（A 可先于 C 落地）。

### A.6 逐文件改动

- **`src/types.ts`**：`NOTICE_TYPE`；`ProjectionKind`；`GoalMessageDetails{goalId?,revision?,nonce?,kind?,round?,checkpointId?,reason?:"revision"|"historical"|"cleared"}`。
- **`src/config.ts`**：`projectionCheckpointRounds: number`（默认 8，读取时 `Math.max(1,…)`）。
- **`src/runtime.ts`**：删 `runProjection`（48-49、120）；`GoalRuntime` 增 `lastCheckpoint?:{nonce,revision,goalId}`、`deltasSinceCheckpoint`、`compactedSinceCheckpoint`、`checkpointVisible?`、`lastProjection?`、`historicalNoteSent`、`clearedNotePending`；`resetProjectionTracking(rt)`；纯函数 `checkpointReason(state, rt, interval): "no-checkpoint"|"revision"|"compaction"|"not-visible"|"interval"|undefined`；`recordProjectionSent(rt, state, kind, nonce)`（checkpoint：设 `lastCheckpoint`、清零计数与 compaction 标记、`checkpointVisible=undefined`；delta：计数+1；两者：`lastProjection=snapshotProjection(state)`）；`snapshotProjection`/`diffProjection`。
- **`src/prompts.ts`**：`import { createHash } from "node:crypto"`；`objectiveHash`、`objectiveSummary`；`PROGRESS_RULES`、`BANNER_PROGRESS_RULE`、`DELTA_MAX_CHARS`；重做 `criteriaChecklist`，新增 `compactCriteriaLine`、`changeAnnotation`；把 `renderContinuation` 拆为 `projectionBody(state, warning?, quota?)` + `renderCheckpoint(state, round, warning?, quota?)`，`renderKickoff` = kickoff 首行 + body，删除 `renderContinuation`；新增 `renderDelta(state, round, previous, warning?, quota?)`（含阶梯）、`renderRevisionNotice(state, "edit"|"budget")`、`renderHistoricalNote(phase|"cleared")`；改文件头注释（8-9 行"每轮重渲染"）。
- **`src/index.ts`**：`LooseMessage.details` → `GoalMessageDetails`；`sendContinuation`：`reason=checkpointReason(...)`，`kind`，`round`，渲染，`details:{goalId,revision,nonce,kind,round,checkpointId: kind==="checkpoint"?nonce:rt.lastCheckpoint?.nonce}`，`recordProjectionSent`，`logEvent("send_continuation",{nonce,kind,reason,deltasSinceCheckpoint,chars,checkpoint})`，前置 `ensureConfig(ctx)`；`createGoal`：`resetProjectionTracking`，kickoff `details` 补全，`recordProjectionSent(...,"checkpoint",nonce)`；`before_agent_start` 按 A.1 历史化短注；`context` 按 A.1 透传并记录 `checkpointVisible = messages.some(custom && (KICKOFF|CONTINUATION) && details.kind==="checkpoint" && details.goalId===id && details.revision===revision)`（仅 `state && rt.semanticRunId!==undefined` 时）；`classifyRunOrigin` 跳过 `NOTICE_TYPE`；`agent_start` 删 `rt.runProjection=`；`session_start`/`session_tree` 加 `resetProjectionTracking`；新增 `session_compact` 监听（置标记 + `logEvent("session_compact",{reason,willRetry,fromExtension,tokensBefore})`）；`persist()` 成功且 active 时 `historicalNoteSent=false`；`/goal resume`、`/goal reject` 在 `sendContinuation` 前 `resetProjectionTracking`；`/goal clear` 置 `clearedNotePending` + reset；`/goal budget`、`/goal edit` 计算 `runInFlight`（edit 在 `bumpGeneration` 前）并在 persist 后发通知。
- **测试夹具**：`pushUser(text)`、`pushMessage(message)`（可推入 `{role:"compactionSummary"}`）；`runAutoRound` 返回本轮 `context` 调用的消息列表；`llmCall(driver)`、`expectExactPrefix(prev,next)`（`next.slice(0,prev.length)` 深等于 `prev`）。
- **`docs/upstream-pi-harness.ts`**：109-119 的"每次调用 ≤1 份完整 continuation"改为 (a) 相邻 LLM 调用互为精确前缀、(b) 每次调用 banner 数 ≤ 迄今用户提示数；74-75 注释改"goal 消息原样透传"。

### A.7 被遗漏的边界（已纳入）

1. kickoff 必须播种 `rt.lastCheckpoint`（需 nonce），否则首条 continuation 冗余成 checkpoint。
2. 同 session 换 goal：检测谓词与 `checkpointReason` 必含 `goalId`。
3. 消息缺失检测滞后一轮（决策发生在 settle 的 `sendContinuation`，早于下一次 `context`）：`session_compact` 为主触发；测试须明示该滞后。
4. `/goal edit` 的 `bumpGeneration` 清 `semanticRunId`：`runInFlight` 先算。（既有问题：随后该 run 被视为已结束、codex turn 身份被清——不在本批修，记风险。）
5. 同进程 `/goal resume`/`reject` 后强制 checkpoint（用户动作，代价低，测试确定，重锚任意插话）。
6. 分支切换/fork：选中分支可能无 checkpoint → 保守 reset。
7. wrap-up run 由 `goal-wrapup` followUp 驱动、无 `before_agent_start`：短注落在用户下一提示；通知条件含 `inWrapUpRun` 让预算变更能进报告。
8. 预约作废但消息已落：`lastCheckpoint` 在发送时乐观设置；丢失 checkpoint 只能在 `bumpGeneration`+resume 后再续，而 resume 强制 checkpoint；观察 (d) 另行纠正。
9. 老 session（消息无 `details.kind`）永不计为可见：升级后恰一条 checkpoint，然后正常节律。
10. run 中 compaction 移走 checkpoint 后到下一轮前模型无规则：后续增强（`session_compact` 且 run 在飞时 steer 一份 checkpoint），本批不做，文档注明。
11. banner 每用户提示一份永久保留（规格如此）；图例改写使 ≥5 条时更短；delta 式 banner 为后续项。
12. note 是模型文本：折叠空白/控制字符、≤80 字符；位于 bullet 列表而非 `<objective>` 内，不需 XML 转义但不得残留换行。
13. 上游 harness "每次调用 ≤1 个 `Continue working toward the active goal`" 与设计冲突，必须改写。
14. compaction 后上下文头部出现 `compactionSummary` 角色：`classifyRunOrigin` 忽略非 user/custom/assistant，无影响；测试推一条证明。

### A.8 测试清单（`describe("context projection (append-only)")`）

1. `passes every goal message through unchanged (append-only)`（改写 1513）：同 5 条输入，`result.messages` 深等于输入，两份 banner 与两份 continuation 原样。
2. 1543-1564 原样保留。
3. `kickoff is a checkpoint that declares supersession, revision and objective hash`：首行含 `supersedes`、`contract revision 1`、`#<8 hex>`；`details` 完整；含六段标题（含 `Progress visibility:`）。
4. `ordinary continuation is a bounded delta without static rules`：3 条 criteria 夹具；`details.kind==="delta"`、`checkpointId===kickoff nonce`、长度 ≤ `DELTA_MAX_CHARS`；不含 `Fidelity:`/`Work from evidence:`/`Execution efficiency`/`Completion audit:`/`Blocked audit:`/`Progress visibility:`/`<objective>`/`report_progress`；含 `supersedes`、`rev 1`、同一 `#hash`、`C1 ⬜○ | C2 ⬜○ | C3 ⬜○`；无 `changed since last round`。
5. `delta reflects last round's report_progress and annotates the change once`（依赖 §C）。
6. `delta carries the one-time budget warning within the cap`：`budget 10000` 跨 50%。
7. `renderDelta never exceeds the cap for very large contracts`：80 条合成状态 → ≤1500 且为计数形。
8. `every 8th continuation after a checkpoint is a full checkpoint (K rollover)`：18 轮 → 8 delta、checkpoint、8 delta、checkpoint。
9. `revision change makes the next continuation a full checkpoint and leaves history untouched`（改写 1566）：settle 与 deliver 之间 `budget 999999` → 下条 `kind:"checkpoint"`、`revision:2`；已交付条目深等于此前快照。
10. `mid-run /goal budget steers a contract-updated notice; /goal edit too`：`lastSent()` 为 `goal-notice`、steer、含 `revision 2`；交付后下一次 `context` 仍把该 run 判为 goal round；反例：无 run 在飞不发。
11. `compaction makes the next continuation a checkpoint`：轮间 emit `session_compact`。
12. `a checkpoint missing from context is detected one call later`：移除 kickoff/continuation 条目并推 `{role:"compactionSummary"}` → 下条仍 delta，再下条 checkpoint。
13. `cross-run exact prefix: revision change`。
14. `cross-run exact prefix: goal leaves active and the user keeps chatting`（改写 1587/1608）：pause → `before_agent_start` 返回 `goal-notice` 含 `[goal paused; earlier goal round instructions are historical`；第二次提示返回 `undefined`；旧 banner 仍 `[GOAL ACTIVE`；前缀成立；`/goal resume` → checkpoint；前缀再成立。
15. `cross-run exact prefix: user interjects while active`：banner + 用户消息追加；前缀成立；随后 continuation 为 delta。
16. `clearing the goal appends a one-time historical note`。
17. `512 rounds accumulate a bounded projection footprint`：默认预算，511 次 `runAutoRound`；56 checkpoint / 455 delta；`total <= 57×maxCheckpointChars + 455×DELTA_MAX_CHARS` 且 `total < 0.4×512×maxCheckpointChars`；`autoTurnsUsed===512`、phase active；`{timeout:60_000}`，过慢则该测试直接传 entries 消息不 clone（不改写已由测试 1 证明）。
18. `banner and get_goal render both criteria dimensions and the progress one-liner`（依赖 §C）。

### A.9 文档同步

`docs/implementation-notes.md:78-88` 冻结说明改为"历史永不改写（run 内与跨 run）；kickoff/revision/compaction/每 K 轮追加完整 checkpoint，其余追加 ≤1,500 字符 delta；五段静态规则 + Progress visibility 仅在 checkpoint"；`README.md` 149/151 行与配置表加 `projectionCheckpointRounds`；`docs/design.md` 201 注释、237 行（compaction 免疫改由"compaction 后 checkpoint"保证）、508 状态；`docs/upstream-pi-harness.ts` 74-75、109-119；roadmap §6.3 行 394-395 标注已实现。

### A.10 风险

1. 多份可见投影对模型行为的影响（旧 kickoff "start working" 与新 checkpoint/delta 并存）：靠强制首行声明与 `get_goal` 权威缓解；**用一次真实 session + `send_continuation{kind,reason,chars}` debug 事件验证后才算完成**。
2. 上游 harness 现编码旧去重语义，改写前必失败。
3. `/goal edit` 中途已使 `semanticRunId` 失同步（既有）；通知检查必须先于 `bumpGeneration`。
4. >60 条 criteria 的病态合同会隐藏变化标注（已文档化与测试）。
5. 测试 5/18 依赖 `report_progress`；顺序上 A 先落地时渲染器需对 `criteriaProgress` 缺失安全。
6. 512 轮测试耗时。
7. 升级时在飞的老 session：首条 continuation 变 checkpoint，不改写旧消息，无 cache miss。

## B. 持久化 checkpoint + delta（roadmap §6.1）——协议级实现规格

### B.1 协议决策（最终）

| # | 决策 | 结论 | 理由 |
|---|---|---|---|
| D1 | seq 作用域 | **单一 seq，按分支路径，跨全部 goal 事件类型**（`goal`、`goal-usage`、`goal-criteria`、`goal-plan`、`goal-verification`、tombstone 共用） | 一个 CAS 必须覆盖整条流（criteria delta 写在被取代的 checkpoint 之后正是 CAS 要抓的竞态）；fold 需要可从数据自证的全序；写入方本来就只有一个 `baseSeq`。seq 在同一 root→leaf 路径上唯一，不同兄弟分支可各有 seq 3（与现状一致，fork 原样复制条目所以连续） |
| D2 | CAS 方式 | **写入时从尾部反向扫描**到最后一条 goal-* 条目，只校验其信封（数值 seq、已知 v/kind），`seq===baseSeq` 否则抛 `goal: stale write (base seq B, branch seq S)`；**rebuild 时全量校验每条**。v2 事件要求 `seq === prev+1`（断档/重复 fail-loud）；v1 仅要求严格递增 | 宿主 `getBranch()` 本身就是 O(depth) 的 parent 链遍历，tail-scan 省的是逐条校验而非遍历；尾部以下的坏条目会在下一次 rebuild 抓到，而 `persist` 在 `state===undefined`（rebuild 失败必置）时不可达，故不弱化 fail-loud |
| D3 | 事件推导 | **shadow diff**：`persist(ctx, opts?)` 用 `isDeepStrictEqual` 比较活状态与 `persistedShadow`（去掉 `verificationResults` 的 `structuredClone` + results 数组的引用）。任一非 delta 字段不同 → checkpoint；否则按差异生成 `usage`/`criteria`/`plan` delta；results 引用变为新数组 → 先写 `verification`。**无差异则不写**（debug `persist{kind:"noop"}`）。`updatedAt` 改为事件派生：仅在实际写入时 `state.updatedAt = at`，fold 应用每个 delta 后 `state.updatedAt = event.at`，diff 排除 `updatedAt`。可选提示：`opts.checkpoint?: reason`（强制）、`opts.criteriaSource?`（只影响 `verifiedBy`） | 唯一能正确捕获 `classifyRunOrigin`(410/431) 与 `tool_execution_start`(540) 里未即时持久化的增量；消除 13 个调用点的逐点判断；checkpoint/delta 决策成为可单测的纯函数。18 条 criteria 的 state 约 15KB，clone+compare 微秒级 |
| D4 | 多事件写入与原子性 | 一次 `persist` 可按**固定顺序**追加多条连续 seq 事件：`verification`（若 results 变）→ 然后要么一条 `checkpoint`（不含 results，`verificationRef` 指向刚分配的 seq），要么若干 `usage`/`criteria`/`plan` delta。全部在同一同步段内 `appendEntry`（宿主为同步 `appendFileSync` 单行）。**任何前缀都是合法流**：崩溃落在两行之间时，前缀 fold 仍一致（`verificationRef` 永远向后指，不会悬空）。无需恢复代码 | 用"前缀封闭"替代事务概念 |
| D5 | verification 输出 | **内嵌 `goal-verification` 事件**（用户 2026-09-04 选定，关闭 roadmap §7-3；每条 result 已被 `excerpt` 截到 8K）；checkpoint **禁止**内嵌 `verificationResults`（fold 拒绝），改带 `verificationRef: number\|null`。schema 预留 `evidence?: {path, sha256, bytes}` 供 §7-3 混合方案，本批不用 | 分支/fork 天然正确；无文件生命周期与防篡改层（属 M2 §5.2）。18 步合同最坏 ≈ 36×16.3KB ≈ 590KB/次验证，且按验证次数（每 resume 周期 ≤3）而非轮次增长 |
| D6 | K | **`persistCheckpointEvery = 50`**（goal.json，整数 1..1000，超界钳制）。计数规则：`deltasSinceCheckpoint + 本次将写 delta 数 >= K` → 改写一条 checkpoint；checkpoint 后计数归零；rebuild 从 fold 恢复计数。`persistCheckpointEvery:1` 为兼容逃生口 | fold 是线性扫描与 K 无关；K 只约束 checkpoint 后重放长度、非 fold 读者看到的 `goal` 条目陈旧度、以及生产中 `checkpoint.state === fold` 自检频率。#6：23×15KB + 1,147×~170B ≈ 540KB vs 17.2MB |
| D7 | 压缩/终态 | 存储层不做压缩（宿主无删除 API；checkpoint 即压缩）。新增 **`wrap-up-settled` checkpoint**：`agent_settled` 中若刚结束的是 wrap-up run（`inWrapUpRun`）且 phase 非 active，在写 `goal-final` 之前 `persist(ctx,{checkpoint:"wrap-up-settled"})` | goal 静止后"最后一条 `goal` 条目"重新等于终态，`goal-final.usage` 与之一致；运行中仍以 fold 为唯一正确读法 |
| D8 | stale writer 与并发 | 三层：generation 围栏（`session_start`/`session_tree` rebuild 刷新 `baseSeq`/shadow/计数/`verificationSeq` 并 bump）→ CAS（尾部 seq）→ `SerialMutationQueue`。队列覆盖：`report_progress`/`get_goal`/未来 `update_plan` 的整个 execute；`update_goal` 分两段（段 1：校验+写 proposal+进 verifying；释放；`await` 机械验证；段 2：围栏检查+应用结果+persist/transition）；命令处理器只把**提交段**入队，绝不跨 `ctx.ui.editor`/normalizer 的 await 持有队列；事件处理器（`turn_end`/`agent_settled`/`tool_execution_start`）保持"同步到 persist"不入队 | CAS 抓不到"基于另一分支状态的内存突变"（seq 恰好匹配新尾部），只有围栏能抓——文档须写明。队列对今日代码是保险，对同批 `report_progress`+`update_goal` 及未来 await 中途的工具是必需 |
| D9 | 版本与迁移 | 每条 v2 事件带 `v:2`。`customType==="goal"` 且无 `v` = v1：`{seq,state}`→checkpoint（results 内嵌有效），`{seq,tombstone:true}`→tombstone。`v>2` → fail-loud `goal event stream was written by a newer goal extension (vN); upgrade the extension`。未知 kind/缺必填字段 → fail-loud。降级不支持：旧构建读 v2 流只见 `goal` 条目（用量陈旧、静默）；若旧构建写入，seq 与 delta 冲撞，新构建以连续性错误 fail-loud | — |
| D10 | 指纹 | `computeFingerprint` 的工具计数改用 `rt.workToolCallsThisRun`（排除 `get_goal`/`update_goal`/`report_progress`/未来 `update_plan`）；perRun 上限仍用 `toolCallsThisRun` | 否则仅调用一次 `report_progress` 的轮次就会通过计数改变指纹，违背"调用工具不算进展" |

### B.2 事件 schema（新文件 `src/events.ts`）

```ts
export const GOAL_PROTOCOL_VERSION = 2 as const;
export const GOAL_ENTRY_TYPE = "goal";                 // checkpoint + tombstone（customType 不变）
export const GOAL_USAGE_TYPE = "goal-usage";
export const GOAL_CRITERIA_TYPE = "goal-criteria";
export const GOAL_PLAN_TYPE = "goal-plan";
export const GOAL_VERIFICATION_TYPE = "goal-verification";
export const GOAL_EVENT_TYPES: ReadonlySet<string> = new Set([...]);

interface EnvelopeV2 { v: 2; seq: number; goalId: string; at: number }
export type CheckpointState = Omit<GoalState, "verificationResults">;   // 校验器强制

export type GoalCheckpointEvent = EnvelopeV2 & {
  kind: "checkpoint";
  reason: "create"|"contract"|"phase"|"blocked-reason"|"proposal"|"rejections"|"debug-log"
        |"verification-cleared"|"interval"|"wrap-up-settled"|"forced";
  state: CheckpointState;
  verificationRef: number | null;   // 当前有效 results 所在 goal-verification 事件的 seq
};
export type GoalTombstoneEvent = EnvelopeV2 & { kind: "tombstone"; tombstone: true };
export type GoalUsageEvent = EnvelopeV2 & { kind: "usage";
  dTokens: number; dCacheRead: number; dCostUsd: number; dTimeMs: number; dTools: number; dRounds: number };
export type CriteriaSource = "mechanical" | "self-report" | "stale-mark";
export type GoalCriteriaEvent = EnvelopeV2 & { kind: "criteria"; revision: number;
  verified?: Array<{ id: string; status: CriterionStatus }>;                 // 局部，按 id
  progress?: Array<{ id: string; status: ProgressStatus; note?: string }>;  // 整表（全部 criteria）
  verifiedBy: CriteriaSource };
export type GoalPlanEvent = EnvelopeV2 & { kind: "plan"; revision: number; items: PlanItem[]; explanation?: string };
export type GoalVerificationEvent = EnvelopeV2 & { kind: "verification"; revision: number;
  proposalRequestedAt: number; allPassed: boolean; results: GoalVerificationResult[];
  evidence?: { path: string; sha256: string; bytes: number } };            // 预留，本批不用
export type GoalEventV2 = GoalCheckpointEvent|GoalTombstoneEvent|GoalUsageEvent|GoalCriteriaEvent|GoalPlanEvent|GoalVerificationEvent;
```

`src/types.ts` 新增：`ProgressStatus`、`GoalCriterionProgress{status,note?,at,stale?}`、`PlanItemStatus`、`PlanItem{id,text,status}`、`GoalPlan{revision,items,explanation?,updatedAt,stale?}`；`GoalState` 增 `criteriaProgress?`、`plan?`；`verificationResults?` 保留为**内存字段**（永不进 checkpoint）。`customType ↔ kind` 映射不符 → 校验拒绝。

### B.3 fold 算法（`foldGoalEntries(entries)`，纯函数，导出供取证脚本）

1. 线性扫描 `type==="custom" && customType ∈ GOAL_EVENT_TYPES`；每条 `normalize`（v1 → `{v:1, kind}`；v2 逐 kind 校验）。错误文案：v1 `corrupt goal snapshot: <problem>`（保持现有测试正则）；v2 `corrupt goal event (seq N, <customType>): <problem>`；`v>2` 见 D9。
2. 序号：v2 要求 `seq === prev+1`，否则 `goal event seq N follows seq M: v2 sequence must be contiguous`；v1 要求 `seq > prev`。
3. `base` = 最后一个 `checkpoint|tombstone`。无 → `orphan goal delta (seq N, kind K): no checkpoint precedes it`。tombstone 之后还有事件 → `goal delta (seq N) after tombstone (seq M)`；tombstone 为末 → `state undefined, lastSeq`。
4. checkpoint：`state = structuredClone(cp.state)`；v1 直接用内嵌 results；v2 要求 `"verificationResults" ∉ cp.state`（否则 `checkpoint seq N must not embed verificationResults`），`verificationRef` 非空时在 `base` 之前二分查找同 seq 的 `verification` 事件且 `goalId` 一致，否则 `checkpoint seq N references verification seq M which is missing / not a verification event / belongs to another goal`。
5. 重放 `base+1..`：每条要求 `goalId === state.contract.id`（否则 `goal delta seq N for goal X, but the current goal is Y`）；`usage` 六项累加；`criteria` 要求 `revision === contract.revision`（否则 `criteria delta seq N targets contract revision R, current revision R'`），`verified[]` 的 id 必须存在于合同，`progress[]` 必须恰好覆盖合同 id 集（缺/多/重 → `progress table does not cover the contract's criteria`），未变化项保留原 `at`，写入即清 `stale`；`plan` 同 revision 检查后整表替换；`verification` 同 revision 检查后替换 results 并记 `verificationSeq`；每条后 `state.updatedAt = ev.at`，`deltas += 1`。
6. `forwardFill(state)` 沿用现有 M1 默认值（`criteriaProgress`/`plan` 缺省保持 undefined）。返回 `{state, lastSeq, checkpointSeq, deltasSinceCheckpoint, verificationSeq, counts:{v1,v2,checkpoints,deltas}}`。

逐 kind 必填字段：信封 `v===2`、有限整数 `seq`、非空 `goalId`、有限 `at`；checkpoint 的 `state` 过现有 `validateSnapshot`，`verificationRef` 为 null 或 `< seq` 的整数，`criteriaProgress`/`plan` 若存在须结构合法（枚举状态、note ≤200）；usage 六个有限数；criteria 有 `revision`、`verifiedBy` 枚举、至少一个 `verified`/`progress`；plan 有 `revision`、`items[{id,text,status}]`；verification 有 `revision`、`proposalRequestedAt`、`allPassed`、`results[]` 逐项基本类型检查。

### B.4 逐文件改动

- **`src/events.ts`（新）**：B.2 类型；`validateGoalEvent(customType, data)`；把 `PHASES` 与 `validateSnapshot` 主体迁入复用（v1 与 v2 `checkpoint.state` 共用）。
- **`src/store.ts`（重写，约 300 行）**：保留 `BranchReader`/`EntryAppender`；`foldGoalEntries`（纯，返回存储对象引用 + 元数据，注释"勿修改"）；`rebuildGoalState(reader)` 保名，`RebuildResult.ok` 增 `checkpointSeq/deltasSinceCheckpoint/verificationSeq/counts`，仍返回 clone + 前向补默认；`PersistedShadow{state: CheckpointState; resultsRef; verificationSeq}`；`deriveGoalEvents(shadow|undefined, next, {checkpointEvery, deltasSinceCheckpoint, at, checkpoint?, criteriaSource?}) → {drafts, checkpointReason?}`（纯；draft 不含 seq/v，checkpoint 的 `verificationRef` 可为 `"pending"`）；`persistGoalEvents(appender, reader, drafts, baseSeq) → {lastSeq, written[{customType,seq,kind,bytes}]}`（tail-scan CAS、连续 seq、解析 pending ref、`structuredClone` 载荷、按 D4 顺序追加）；`persistTombstone` 改写 v2 tombstone；保留 5 行 `persistGoalState` 包装（发强制 checkpoint）以零成本兼容 test:262。
- **`src/mutation-queue.ts`（新，~25 行）**：`SerialMutationQueue.run<T>(fn)` promise 链，错误传给调用方但不毒化链。
- **`src/config.ts`**：`persistCheckpointEvery: number`（默认 50，`loadGoalConfig` 钳制到 [1,1000]）。
- **`src/index.ts`**：闭包 `lastPersisted` → `persistedShadow`，新增 `deltasSinceCheckpoint=0`、`verificationSeq=null`、`const mutations = new SerialMutationQueue()`。`persist(ctx, opts?)`：合法性检查改读 `persistedShadow.state.phase`；`at=Date.now()`；`deriveGoalEvents`；空 → noop 日志返回 true；否则 `persistGoalEvents`，成功后更新 `baseSeq`/`deltasSinceCheckpoint`/`verificationSeq`/shadow 与 `state.updatedAt=at`，`logEvent("persist",{events,bytes,checkpointReason})`；失败路径不变。`rebuild(ctx)` 设置四个新字段，`logEvent("rebuild",{found,lastSeq,checkpointSeq,deltasSinceCheckpoint,verificationSeq,counts})`。调用点结果（无需改代码，作评审清单）：228 transition→checkpoint(phase)；614 turn_end→usage delta 或 noop；669 settle 时间冲刷→usage 或 noop，**并在 goal-final 之前新增** `if (inWrapUpRun && state.phase!=="active") persist(ctx,{checkpoint:"wrap-up-settled"})`；937 create→checkpoint(create)；994→checkpoint(phase)；1046 打回→verification + checkpoint(rejections 或 phase)；1174 resume→checkpoint；1269 reject→checkpoint(`verificationRef:null`)；1281 clear→tombstone；1343 budget→checkpoint(contract)；1400 edit→checkpoint(contract)，**edit 前新增** progress 存活 id 标 `stale:true`、`plan` 标 stale；1530 proposal→checkpoint(proposal)，随后 verifying 又一条 checkpoint（每次提案两条，本批接受）。`update_goal`/`get_goal` execute 入队（D8）；`get_goal` 输出增"Self-reported progress"块。`tool_execution_start`：`if (!GOAL_TOOLS.has(toolName)) rt.workToolCallsThisRun += 1`。`agent_settled` 指纹改用 `workToolCallsThisRun` 与 `criteriaProgress`。新增 `report_progress` 工具（§C）。
- **`src/runtime.ts`**：`computeFingerprint(toolCalls, writes, criteriaStatus, criteriaProgress?)` 追加按 id 排序的 progress 状态分量；`GoalRuntime.workToolCallsThisRun`，`resetRunCounters` 归零。
- **`src/ui.ts` / `src/prompts.ts`**：两个维度并列显示（verified 图标 `✅❌⬜♻️` 照旧；自报标记统一为 `○ not started` / `◐ in progress` / `◑ self-reported met`，`progress.stale` 加 `(stale)`），widget 行 `⬜ C1 text · ◐ in progress`、note 截 40 字符；`renderStatusCard` 每条追加 `self-report: <status>[ (stale)] — <note>`；footer 加 `◐n ◑m` 计数；提示词新增 "Progress visibility" 静态段与 checklist 图例（只随 checkpoint/banner 发，见 §A.5）。
- **`src/verification.ts`**：无行为改动；导出 `OUTPUT_LIMIT` 供尺寸断言。
- **测试夹具**：`createDriver` 增 `entriesOf(customType)`；`liveCard()`（`/goal` 状态卡文本）用于"活状态 vs reload 后 fold"比对。

### B.5 迁移/兼容矩阵

| 流 | 新构建读 | 新构建写 | 旧构建读 | 旧构建写 |
|---|---|---|---|---|
| 仅 v1 | 末快照=checkpoint，results 取自快照，前向补默认 | 继续 seq 写 v2 | 同今 | 同今 |
| v1 前缀 + v2 后缀 | v1 规则 + 自首个 v2 起连续性 | v2 | 只见 `goal`→用量陈旧（静默） | seq 撞 delta → 新构建连续性 fail-loud |
| 仅 v2 | 完整 | v2 | 用量陈旧（静默） | 同上 |
| `v>2` | fail-loud "newer extension" | 拒绝（state undefined） | — | — |
| 测试注入 `{seq,state}` 在 v2 create 之后（test 829/892/1463） | v1 checkpoint 跟在 v2 checkpoint 后：允许（v1 仅要求递增） | 下一 seq = 注入+1 | — | — |
| tombstone（v1/v2） | 清空；其后 delta → fail-loud | 新 goal → `create` checkpoint，新 goalId，seq 续 | 同今 | 同今 |

### B.6 测试清单（有序）

store 级纯函数（合成条目）：
1. derive：首写为 create checkpoint；状态不变 → 无 draft。
2. derive：六个用量计数差异 → 一条 usage delta（含 `dRounds/dTools` 的延迟增量）。
3. derive：contract/phase/proposal/rejections/blockedReason/verification-cleared 强制 checkpoint 且抑制 delta。
4. derive：results 变化 → verification 先于 checkpoint；`written[1].state` 无 `verificationResults`，`verificationRef === written[0].seq`。
5. derive：K 回卷（K=3）→ `[checkpoint, usage, usage, checkpoint]`，`fold(all).state` 深等于该 checkpoint.state 且 results undefined。
6. fold：仅 v1 流（`test/fixtures/` 放一份裁剪后的真实 18 条 criteria session 片段）与今日结果一致。
7. fold：v1 前缀 + v2 后缀。
8. fold fail-loud `it.each`：孤儿 delta / tombstone 后 delta / goalId 不符 / seq 断档 / seq 重复 / v>2 / checkpoint 内嵌 results / 悬空 verificationRef / criteria revision 不符 / progress 表未覆盖合同——断言 B.3 精确文案。
9. `persistGoalEvents` CAS：尾部为 delta 时 stale write；尾部 seq 非数值 → `refusing to write over corrupt state`。
10. mutation queue：顺序执行；异常不毒化后续。

扩展级（driver）：
11. fold(replay)===live：20 轮非零用量 + 一次 `report_progress` + `/goal budget` + pause/resume；`liveCard()` 等于 `session_start{reload}` 后的卡；`driver.goalState()` 深等于对 `structuredClone(entries)` 的 fold；`entriesOf("goal").length <= 1 + phaseChanges + contractChanges + floor(usageDeltas/K)`；`entriesOf("goal-usage").length === 非零用量 turn_end 数`。
12. 任何 checkpoint 不含 `verificationResults`；`goal-verification` 事件携带且 `driver.goalState()?.verificationResults` 与之相等。
13. reject/edit 经 `verificationRef:null` 清空。
14. `/goal clear` 后新 goal：tombstone → 新 goalId 的 create checkpoint，seq 连续。
15. complete 后 wrap-up 用量落为 usage delta，`wrap-up-settled` checkpoint 与 `goal-final.usage` 一致（扩展现有 1040-1057）。
16. 现有 232/248/262/300/316/331/350 与注入型 829/892/1463 不改即过。
17. 不可变性：delta 与 checkpoint 载荷不随活状态变化（扩展 test 350 到 `goal-usage`）。
18. 分支切换：rebuild 恢复 `deltasSinceCheckpoint/verificationSeq`，下一写入 seq = 新尾部+1，无 stale write 通知。
19–23. `report_progress` 五条（见 §C 测试）。
24. `goal-plan`：合成 plan 事件 fold 进 `state.plan`；edit 标 stale；clear 清空。
25. `persistCheckpointEvery:1` 逃生口：每写皆 checkpoint。
26. `computeFingerprint` 含 progress 状态、不含 note、排除 goal 工具计数。

### B.7 边界与风险

- 每次完成提案两条 checkpoint（proposal → verifying），18 条 criteria 约 30KB/次、每 resume 周期 ≤3 次：接受；合并需改 `verifyCompletionProposal` 早退路径的恢复语义，列为后续项。
- `agent_settled` 时间冲刷先于 transition：usage delta 后紧跟 checkpoint，均同步，checkpoint 已含冲刷时间。
- no-op persist 与 `updatedAt`：必须事件派生，否则零用量轮次破坏 fold===live；测试默认零用量的 `runAutoRound` 不产生 usage delta，计数断言用 `<=` 或显式非零用量。
- v1 注入语义保持宽松（严格递增），不得收紧为连续。
- 坏分支 + `/goal clear`：今日 `persistTombstone` 拒绝写在坏状态上，rebuild 失败后 `baseSeq` 陈旧，坏分支无法从内部清除——本批不改，记为候选（rebuild 失败时返回尾部 seq 让 clear 成为恢复路径）。
- D10 指纹语义微调会略微影响 no-progress 熔断（goal 工具不再算活动）。
- `getBranch()` 每次写入 O(depth) 是宿主成本，与今日相同。
- compaction 事件对持久化无关（custom 条目不受影响），store 侧不加监听。

### B.8 工作量

events/types 0.5d → store/queue/config 1d → index/runtime 0.75d → report_progress + UI/prompts 0.75d → 测试 1.5d → 文档 0.5d ≈ 5 人日。1–2 与 4 可并行，3 依赖 1–2。

## C. `report_progress` 工具（roadmap §6.2.1）——实现决策

roadmap 已定且不再讨论：schema `items[{criterionId,status,note?}]`；`status ∈ not_started|in_progress|self_reported_met`；整表替换 last-wins；必须恰好覆盖全部 criteria（缺项/重复/未知 ID 结构化拒绝）；仅 active 可调；自报与 verified（unverified/passed/failed/stale）严格分离；永不解锁 completion；显示到 widget/footer/continuation；状态向量进 no-progress 指纹；仅调用或只改 note 不算进展。

四项实现决策（推荐值，理由随附）：

| # | 决策 | 结论 | 理由 |
|---|---|---|---|
| C1 | 持久化载体 | **两者兼有**：内存 `GoalState.criteriaProgress?: Record<id,{status,note?,at,stale?}>`（小对象，随 checkpoint 序列化）；每次写入以 **`goal-criteria` delta**（`progress` 维度，`verifiedBy:"self-report"`）落盘，不写全量快照。§B 的 shadow-diff persist 自动推导该 delta，工具体只需改 state 后调 `persist` | 与 §6.1 合流；#6 型 goal 每轮一条几十字节 delta 而非 15KB 快照；若 §B 晚于本工具落地，同一 `persist` 调用在旧机制下自然退化为全量快照，无需改工具代码 |
| C2 | 并发 | 引入 `SerialMutationQueue`（promise 链），`report_progress`/`update_goal`/`get_goal`/未来 `update_plan` 的 `execute` 体与 `/goal` 命令处理器全部经队列串行；事件处理器保持"同步到 persist 为止"不入队。`update_goal(complete)` 分两段入队：段 1 校验 + 写 proposal + 进入 verifying；释放队列后 `await` 机械验证；段 2 应用结果 + persist。 | 同一 assistant 批次里 `report_progress` 与 `update_goal` 并发时两次 CAS 必须有序；验证阶段数分钟的 exec 不能阻塞 `turn_end` 入账 |
| C3 | 合同变化 | `/goal edit`：ID 仍存在的条目**保留并标 `stale:true`**，被删 ID 丢弃，新 ID 无记录（显示 not_started）；下一次 `report_progress` 整表提交即清除 stale。`/goal reject`（complete→active）与 `/goal budget`（仅 revision+1）保留、不标 stale。 | 与 §6.2.2 `update_plan` 的 edit-后-stale 规则一致；避免 UI 把已做的工作显示成"未开始"，又明确提示模型合同已变需复核 |
| C4 | 输入边界 | `note` trim 后 ≤ **200** 字符、**单行**（含 `\n`/`\r` 直接拒绝，不静默改写）；允许任意方向回退（`self_reported_met → in_progress/not_started`） | 与"缺项/重复/未知 ID 均拒绝"的严格 schema 风格一致；整表替换语义下任何状态都合法，新证据可推翻自报 |

其余实现要点：

- **指纹**：`computeFingerprint`（`runtime.ts:171-182`）新增第四分量 = 按 id 排序的 `criteriaProgress[id].status` 向量（不含 note、不含 stale）。只改 note 或提交与上次相同的表 → 指纹不变 → 不重置 no-progress。
- **工具返回**：成功返回一行摘要（"progress recorded: 2 in_progress, 1 self_reported_met, 3 not_started; this is a self-report, not evidence"）；`details:{accepted:true, revision}`。拒绝以 tool result（非 throw）返回结构化错误列表（与 `validateCompletionEvidence` 同规），让模型同轮修正。非 active 相位：拒绝并说明。
- **计数**：`tool_execution_start` 已对所有工具 `toolCallsThisRun+1`，不额外处理；`writesThisRun` 不记。
- **UI**：widget 每条 criterion 追加自报标记（`◐ in progress` / `◑ self-reported met` / `○ not started`，stale 时后缀 `(stale)`），note 截 40 字符；`renderStatusCard` 追加 `self-report: <status>[ (stale)] — <note>`；footer 增加计数 `◐n ◑m`。
- **提示词**：checkpoint 级投影新增一段短静态规则 "Progress visibility"（移植 codex continuation.md 的进度段）：完成某条 criterion 的实质工作后调用 `report_progress` 提交完整表；自报不是证据、不替代 `update_goal` 验证。该段只随 checkpoint 发送（§A）。checklist 与 delta 紧凑行同时显示两个维度（§A）。
- **get_goal**：banner 已含 checklist，自动带自报维度。
- **测试**（映射 roadmap §6.3 "report_progress" 行）：整表覆盖校验（缺项/重复/未知 ID/非单行 note/超长 note 各一例）；非 active 拒绝；自报 `self_reported_met` 全表后 `update_goal(complete)` 仍走证据校验且 `criteriaStatus` 不变；指纹随状态变化、不随 note 变化；`/goal edit` 后 stale 标记与下一次报告清除；`/goal reject` 保留；持久化为 `goal-criteria` delta 而非 `goal` checkpoint 且 fold 后 `criteriaProgress` 与内存一致；同批 `report_progress`+`update_goal` 串行后两次写入 seq 连续且无 CAS 错误。

## D. 实施顺序与提交拆分

按 roadmap §8 与依赖关系，四个独立可回滚的提交，每个提交 `npm run check && npm test` 全绿、文档同步：

| # | 提交 | 内容 | 依赖 |
|---|---|---|---|
| 0 | `docs: add checkpoint+delta implementation spec` | 把本方案 §A/§B/§C 落为 `docs/discuss/checkpoint-delta-implementation-spec.md`（协议是长期契约，须进仓库而非只在对话里） | — |
| 1 | `feat: append-only checkpoint+delta goal projection` | §A 全部（渲染器对 `criteriaProgress` 缺失安全；A.5 的图例/PROGRESS_RULES 可随本提交进，此时自报恒显 `○ not started`）+ A.8 测试 1–4、6–17 + A.9 文档 + `docs/upstream-pi-harness.ts` 改写 | 0 |
| 2 | `feat: persist goal state as checkpoints and deltas` | §B 全部（events/store/queue/config/index/runtime、wrap-up-settled、v1 夹具）+ B.6 测试 1–18、24–26 + 文档（design §5/§15、implementation-notes、README、roadmap §6.1 + 附录 C/D） | 0；与 1 无代码耦合，可并行开发但按序合入 |
| 3 | `feat: add report_progress self-reported criteria progress` | §C 工具 + `criteriaProgress`/stale 规则 + UI + 指纹 D10 + A.8 测试 5、18 + B.6 测试 19–23 + roadmap §6.2.1 状态 | 1、2 |
| 4 | 真实 session 验证（不产生代码提交，产出记录进 roadmap §3.3/§8） | 见 §E.3 | 1–3 |

## E. 验证（端到端）

### E.1 单元与静态
- `npm run check`（tsc + biome，tab 缩进、行宽 120）。
- `npm test`（vitest）：现有 69 tests 全过 + §A.8 18 条 + §B.6 26 条 + §C 测试；不得删除或弱化 1543-1564（run 内字节稳定）、232/248/262/300/316/331/350（store 基线）、829/892/1463（v1 注入）。
- 512 轮测试单独 `{timeout: 60_000}`，若 >10s 按 A.8-17 的降 clone 方案。

### E.2 上游 harness（非 vitest）
`docs/upstream-pi-harness.ts` 改写后在 pi monorepo 下运行（README 注明其归属）；断言：相邻 LLM 调用互为精确前缀；每次调用 banner 数 ≤ 用户提示数；一次 run 产生 `goal-usage` 条目且 `goal` 条目数 ≤ 相变数 + 合同变更数 + 1 + floor(delta/50)。

### E.3 真实 pi session 冒烟（`pi >= 0.84.2`，`transport:"sse"`，`.pi/goal.json` 含 `debugLog:true`）
1. 临时工作区（含一个可 `npm test` 的小项目），`pi` 加载工作树扩展，`/goal --raw <objective>`，让其自动跑 ≥10 轮（观察 kickoff→8 delta→checkpoint 节律），期间调一次 `report_progress`（可用 continuation 自然触发）。
2. `/goal pause` → 普通聊两句（观察一次性 `goal-notice`、旧 banner 未改写）→ `/goal budget 20000000`（无 run 在飞，不发通知）→ `/goal resume`（首条 continuation 为 checkpoint，reason=revision）。
3. run 在飞时 `/goal budget ...` → 观察 steer 的 `goal-notice`。
4. `/compact` → 下一条 continuation 为 checkpoint，reason=compaction。
5. 用附录 C 方法检查 session JSONL：`goal` 条目数 ≪ turn 数、`goal-usage` 条目数 = 计费 turn 数；用 `foldGoalEntries` 折叠结果 == `/goal` 状态卡 == `goal-final`；相变前后与 run 边界处 assistant `usage.cacheRead` 无整段冷调用（对照 §1.1-3 的 667K/455K 形态）。
6. debug 日志核对 `send_continuation{kind,reason,chars}`（delta chars ≤1500）、`persist{events,bytes,checkpointReason}`、`session_compact`、`rebuild{counts}`。
7. 结果（含 session/goal 路径、计数、有无冷调用）写回 roadmap §3.3 与 §8 第 6 步。

## F. 施工指令（自包含，可直接复制给其他 agent）

```
# 任务：pi Goal 扩展 —— 落地 append-only checkpoint+delta 投影、持久化 checkpoint+delta、report_progress

仓库：/Users/ted/workspace/pi-goal-extension/pi-goal-extension（TypeScript ESM，Node ≥22.19，vitest 4，biome：tab 缩进/行宽 120；宿主 @earendil-works/pi-coding-agent@0.84.2，类型在 node_modules/@earendil-works/pi-coding-agent/dist/core/{extensions/types.d.ts,session-manager.d.ts,compaction/compaction.d.ts}）。基线 HEAD 58f1bdc（main，工作树干净）。开工前读：docs/discuss/goal-mode-improvement-roadmap.md §4 §6.1 §6.2.1 §6.3 §6.4 §7 §8、docs/design.md §5 §9 §11、docs/implementation-notes.md、src/index.ts、src/store.ts、src/prompts.ts、src/runtime.ts、src/types.ts、test/goal-extension.test.ts。每个提交 `npm run check && npm test` 必须全绿；提交信息用 `feat:`/`docs:`/`test:` 小写单行；不要问"是否继续"，做完整个任务再汇报；除本指令列出的文件外不要扩大范围。

## 永久不变量（违反任一即失败）
1. 发给 LLM 的历史消息只可尾部追加：context 处理器对 goal-kickoff/goal-continuation/goal-banner/goal-notice 不删除、不折叠、不改写；run 内前缀稳定测试 test/goal-extension.test.ts 1543-1564 原样保留并必须通过。
2. 完成 assurance fail-closed 不变：report_progress 自报永不影响 criteriaStatus，永不解锁 update_goal(complete) 的证据校验。
3. 用量权威在扩展：goal-final 卡 == fold 终态 == 状态卡。
4. semantic run ↔ 后端 turn 保真：不新增 abort/重启；所有 run 内干预用 deliverAs:"steer"。
5. 持久化 seq/CAS/坏记录 fail-loud 不弱化；老的纯快照 session（customType "goal" 无 v 字段）必须双读兼容；分支/fork 仍从 getBranch() 路径 fold。
6. armed 易失、异常不重试、resume 是唯一再武装入口——不改。

## 提交 0：docs: add checkpoint+delta implementation spec
新建 docs/discuss/checkpoint-delta-implementation-spec.md，内容 = 下面"协议 A/B/C"三节原文（可润色格式，不改语义），头部写日期、基线 commit、与 roadmap 的对应章节。

## 协议 A：跨 run 追加式投影（roadmap §6.4；提交 1 `feat: append-only checkpoint+delta goal projection`）
A1 类型（src/types.ts）：export const NOTICE_TYPE="goal-notice"；type ProjectionKind="checkpoint"|"delta"；interface GoalMessageDetails{goalId?;revision?;nonce?;kind?:ProjectionKind|"notice";round?;checkpointId?;reason?:"revision"|"historical"|"cleared"}。src/index.ts 的 LooseMessage.details 改用它。
A2 配置（src/config.ts）：projectionCheckpointRounds:number，默认 8，读取处 Math.max(1,…)。
A3 运行时（src/runtime.ts）：删除 runProjection（字段与 resetRunCounters 里的清空；index.ts agent_start 里的赋值一并删）。新增字段 lastCheckpoint?:{nonce;revision;goalId}、deltasSinceCheckpoint:number、compactedSinceCheckpoint:boolean、checkpointVisible?:boolean、lastProjection?:Record<string,string>、historicalNoteSent:boolean、clearedNotePending:boolean（createRuntime 初始化）。新增纯函数：resetProjectionTracking(rt)（清前五项）；checkpointReason(state,rt,interval) 按序返回 "no-checkpoint"（无 lastCheckpoint）|"revision"（revision 或 goalId 不符）|"compaction"|"not-visible"（checkpointVisible===false）|"interval"（deltasSinceCheckpoint>=interval）|undefined；recordProjectionSent(rt,state,kind,nonce)：checkpoint→设 lastCheckpoint、deltas=0、compacted=false、checkpointVisible=undefined；delta→deltas+1；两者→lastProjection=snapshotProjection(state)。snapshotProjection(state): id → `${verifiedIcon}${selfIcon}${stale?"*":""}`（verified 图标 ✅❌⬜♻️；自报 ○ not_started/缺省、◐ in_progress、◑ self_reported_met）；diffProjection(prev,next)→[{id,from,to}]（id 并集）。
A4 提示词（src/prompts.ts）：import {createHash} from "node:crypto"；objectiveHash(objective)= "#"+sha256(objective.replace(/\r\n/g,"\n").trim()).slice(0,8)；objectiveSummary(objective,max=120)：空白折叠、词边界截断、超出加"…"。export const DELTA_MAX_CHARS=1500。新增静态段 PROGRESS_RULES（插在 EVIDENCE_RULES 与 EXECUTION_EFFICIENCY_RULES 之间）：
  Progress visibility:
  - After substantive work on any criterion, call report_progress with the FULL table (one row per criterion: not_started | in_progress | self_reported_met, optional one-line note); rows marked stale must be re-assessed from current evidence.
  - Self-reported status is visibility for the user and for later rounds. It is NOT evidence, does not change Goal-verified status, and never replaces the completion audit or update_goal.
  BANNER_PROGRESS_RULE="Keep report_progress current after substantive work on a criterion; self-reports are visibility, not evidence."（banner 在 BANNER_EFFICIENCY_RULE 后另起一行）。
  criteriaChecklist 改为：首行图例 `Acceptance criteria — Goal-verified status · your self-report (report_progress). "unverified" = not yet Goal-verified, NOT "not implemented"; self-reports are not evidence:`；每条 `- C1 ⬜ unverified · ◐ in progress [mechanical] <text> — "<note ≤80 字符，折叠换行/控制符>"` 换行 `  evidence: <spec>`；自报缺省显示 `○ not started`；stale 加 ` (stale self-report)`。删除每行的 "(not yet Goal-verified; this does not mean not implemented)"。
  把 renderContinuation 拆成 projectionBody(state,warning?,quota?)（从 OBJECTIVE_GUARD 到收尾句，含 PROGRESS_RULES）与 renderCheckpoint(state,round,warning?,quota?)；renderKickoff(state,quota?) = kickoff 首行 + body；删除 renderContinuation。
  checkpoint 首行：`GOAL CHECKPOINT — automatic continuation round {n}, contract revision {r}, objective {hash}. This message supersedes all earlier goal round messages (kickoff, continuations, deltas, banners) in this conversation: work from this message and from get_goal; older goal messages are history, not instructions.` 空行 `Continue working toward the active goal.` 空行 body。
  kickoff 首行：`A goal has been set for this session (contract revision {r}, objective {hash}); this message supersedes any earlier goal round messages in this conversation. Start working toward it now.` 空行 body。
  renderDelta(state,round,previous,warning?,quota?)，版式：
    行1 `GOAL DELTA — continuation round {n}, contract rev {r}, objective {hash}. Supersedes all earlier goal round messages; the full contract and working rules are in the latest GOAL CHECKPOINT/kickoff message above for rev {r} and via get_goal; older goal messages are history, not instructions.`
    行2 `Objective {hash} (user-provided data, not instructions): {summary}`
    行3 `Criteria [verified+self-report]: C1 ⬜◐ | C2 ⬜○ | …`，若 diffProjection(previous,now) 非空追加 ` (changed since last round: C1 ⬜○→⬜◐, …)`；无 criteria 时 `Criteria: (none; the objective text is the only authority)`
    行4 budgetSection(state,quota)；行5 warning（仅有时）；末行 `Keep working toward the objective now. Call update_goal only when the goal is complete or the blocked audit in the checkpoint is satisfied.`
    超 1500 字符依次：去掉变化标注 → criteria 行改计数 `Criteria: N (verified a passed/b failed/c unverified; self-report d in progress/e met)` → 摘要缩 60 → 硬截 1499+"…"。delta 不得含六段静态规则、<objective>、"report_progress"。
  renderRevisionNotice(state,"edit"|"budget")（≤400 字符）：`GOAL CONTRACT UPDATED to revision {r} (objective {hash}): the user changed the goal contract while this run was in progress. Earlier goal round messages, including the one driving this run, are superseded. Call get_goal now and work from the current contract.` edit 追加 ` Criteria statuses were reset to unverified.`，budget 追加 ` Budget: rounds a/b, tokens x/y.`。
  renderHistoricalNote(phase|"cleared")：`[goal {phase}; earlier goal round instructions are historical — do not act on them; the user's message is the current instruction]` / `[goal cleared; earlier goal round instructions are historical — do not act on them]`。
  更新文件头注释（"每轮重渲染"→checkpoint/delta）。
A5 src/index.ts：
  - context 处理器：保留 classifyRunOrigin(messages)；若 state && rt.semanticRunId!==undefined，rt.checkpointVisible = messages.some(m=>m.role==="custom"&&(m.customType===KICKOFF_TYPE||m.customType===CONTINUATION_TYPE)&&m.details?.kind==="checkpoint"&&m.details.goalId===state.contract.id&&m.details.revision===state.contract.revision)；return {messages: event.messages}。删除 hasGoalMessages、两轮扫描、三个占位符、projection。
  - classifyRunOrigin 反向扫描跳过表加 NOTICE_TYPE。
  - sendContinuation：ensureConfig(ctx)；reason=checkpointReason(state,rt,config.projectionCheckpointRounds)；kind=reason?"checkpoint":"delta"；round=state.autoTurnsUsed+1；content=kind==="checkpoint"?renderCheckpoint(...):renderDelta(state,round,rt.lastProjection,warning,rt.quotaSnapshot)；details={goalId,revision,nonce,kind,round,checkpointId: kind==="checkpoint"?nonce:rt.lastCheckpoint?.nonce}；发送后 recordProjectionSent；logEvent("send_continuation",{nonce,kind,reason,deltasSinceCheckpoint,chars:content.length,checkpoint})。
  - createGoal：resetProjectionTracking(rt)；kickoff 生成 nonce，details={goalId,revision,nonce,kind:"checkpoint",round:1,checkpointId:nonce}；recordProjectionSent(rt,state,"checkpoint",nonce)。
  - before_agent_start：!state && rt.clearedNotePending → 清标记并返回 {customType:NOTICE_TYPE,content:renderHistoricalNote("cleared"),display:false,details:{kind:"notice",reason:"cleared"}}；state.phase!=="active" && !rt.historicalNoteSent → 置标记并返回 renderHistoricalNote(phase) 的 goal-notice（details reason:"historical"）；active 分支照旧返回 banner。
  - persist() 成功后：if(state.phase==="active") rt.historicalNoteSent=false。
  - session_start、session_tree：加 resetProjectionTracking(rt)。
  - 新增 pi.on("session_compact",(event,ctx)=>{rt.compactedSinceCheckpoint=true; logEvent(ctx,"session_compact",{reason:event.reason,willRetry:event.willRetry,fromExtension:event.fromExtension,tokensBefore:event.compactionEntry.tokensBefore});})。
  - /goal resume、/goal reject：sendContinuation 前 resetProjectionTracking(rt)。/goal clear：rt.clearedNotePending = state!==undefined; resetProjectionTracking(rt)。
  - /goal budget、/goal edit：在任何 bumpGeneration 之前算 const runInFlight = rt.semanticRunId!==undefined && (state.phase==="active"||inWrapUpRun)；persist 成功后 if(runInFlight) pi.sendMessage({customType:NOTICE_TYPE,content:renderRevisionNotice(state,"budget"|"edit"),display:false,details:{goalId,revision,kind:"notice",reason:"revision"}},{deliverAs:"steer"})。
  - get_goal 描述追加 "and your self-reported progress"。
  - agent_start 删除 rt.runProjection 赋值。
A6 测试（test/goal-extension.test.ts）：夹具增 pushUser(text)、pushMessage(message)、llmCall(driver)（emit context 并返回 structuredClone(result.messages)）、expectExactPrefix(prev,next)（expect(next.slice(0,prev.length)).toEqual(prev)）；runAutoRound 返回本轮 context 的消息列表。改写 1513-1541（透传：输出深等于输入）、1566-1586（revision 变化→下条 continuation kind checkpoint、revision 2，历史条目不变）、1587-1607 与 1608-1619（pause→before_agent_start 返回 goal-notice 一次、第二次 undefined、旧 banner 原样、前缀成立、resume→checkpoint）。新增：kickoff 是 checkpoint 且含 supersedes/revision/hash/六段标题；普通 continuation 是 ≤1500 字符 delta 且不含六段规则与 <objective>、含 supersedes/rev/hash/紧凑 criteria 行、无变化标注；delta 携带一次性 p50 警告仍 ≤ 上限；renderDelta 对 80 条合成 criteria 仍 ≤1500 且为计数形；K 回卷 18 轮 = 8 delta+checkpoint+8 delta+checkpoint；run 内 /goal budget 发 steer goal-notice、无 run 在飞不发、交付后该 run 仍判为 goal round；轮间 emit session_compact → checkpoint；移除 kickoff/continuation 条目并 pushMessage({role:"compactionSummary"}) → 下条 delta、再下条 checkpoint（滞后一轮）；三种跨 run 精确前缀（revision 变化 / 离开 active 后用户聊天 / active 中用户插话且随后为 delta）；clear 后一次性短注；512 轮（默认预算，511 次 runAutoRound，{timeout:60_000}）：56 checkpoint/455 delta、Σ 字符 ≤ 57×maxCheckpointChars+455×1500 且 < 0.4×512×maxCheckpointChars、autoTurnsUsed===512、phase active。
A7 文档：docs/implementation-notes.md 78-88；README 149/151 与配置表加 projectionCheckpointRounds；docs/design.md 201 注释、237 行、508 状态；docs/upstream-pi-harness.ts 74-75 注释与 109-119 断言（相邻调用互为精确前缀；banner 数 ≤ 用户提示数）；roadmap §6.3 行 394-395 标注已实现。

## 协议 B：持久化 checkpoint+delta（roadmap §6.1；提交 2 `feat: persist goal state as checkpoints and deltas`）
B1 决策：单一 seq 按分支路径跨全部 goal 事件类型；写入 CAS 从尾部反向找最后一条 goal-* 条目（只校验信封）比较 seq===baseSeq，否则 throw `goal: stale write (base seq B, branch seq S)`；rebuild 全量校验。v2 事件 seq 必须 === prev+1；v1 仅严格递增。事件由 shadow diff 推导；无差异不写；updatedAt 事件派生。verification 结果内嵌 goal-verification 事件，checkpoint 禁止含 verificationResults、改带 verificationRef。persistCheckpointEvery 默认 50（goal.json，钳制 1..1000）。wrap-up run settle 时若 phase 非 active 强制一条 reason "wrap-up-settled" 的 checkpoint（写在 goal-final 之前）。
B2 新文件 src/events.ts：
  export const GOAL_PROTOCOL_VERSION=2; GOAL_ENTRY_TYPE="goal"; GOAL_USAGE_TYPE="goal-usage"; GOAL_CRITERIA_TYPE="goal-criteria"; GOAL_PLAN_TYPE="goal-plan"; GOAL_VERIFICATION_TYPE="goal-verification"; GOAL_EVENT_TYPES=Set(以上五个)。
  interface EnvelopeV2{v:2;seq:number;goalId:string;at:number}
  type CheckpointState=Omit<GoalState,"verificationResults">
  GoalCheckpointEvent=EnvelopeV2&{kind:"checkpoint";reason:"create"|"contract"|"phase"|"blocked-reason"|"proposal"|"rejections"|"debug-log"|"verification-cleared"|"interval"|"wrap-up-settled"|"forced";state:CheckpointState;verificationRef:number|null}
  GoalTombstoneEvent=EnvelopeV2&{kind:"tombstone";tombstone:true}
  GoalUsageEvent=EnvelopeV2&{kind:"usage";dTokens;dCacheRead;dCostUsd;dTimeMs;dTools;dRounds}（六个 number）
  type CriteriaSource="mechanical"|"self-report"|"stale-mark"
  GoalCriteriaEvent=EnvelopeV2&{kind:"criteria";revision:number;verified?:{id;status:CriterionStatus}[];progress?:{id;status:ProgressStatus;note?}[];verifiedBy:CriteriaSource}（progress 为整表）
  GoalPlanEvent=EnvelopeV2&{kind:"plan";revision;items:PlanItem[];explanation?}
  GoalVerificationEvent=EnvelopeV2&{kind:"verification";revision;proposalRequestedAt;allPassed;results:GoalVerificationResult[];evidence?:{path;sha256;bytes}}（evidence 预留不用）
  validateGoalEvent(customType,data)→{ok,event}|{ok:false,problem}：v1（customType "goal" 且无 v）归一为 checkpoint/tombstone；v2 逐 kind 校验必填字段与 customType↔kind 映射；v>2 → problem "goal event stream was written by a newer goal extension (vN); upgrade the extension"。把 store.ts 的 PHASES 与 validateSnapshot 迁入复用（v1 与 v2 checkpoint.state 共用）。
B3 src/types.ts 新增：ProgressStatus="not_started"|"in_progress"|"self_reported_met"；GoalCriterionProgress{status;note?;at:number;stale?:boolean}；PlanItemStatus="pending"|"in_progress"|"completed"；PlanItem{id;text;status}；GoalPlan{revision;items;explanation?;updatedAt;stale?}；GoalState 增 criteriaProgress?:Record<string,GoalCriterionProgress>、plan?:GoalPlan；verificationResults? 保留为内存字段。保留 GoalSnapshot 作 v1 形状；customType 常量从 events.ts 重导出保持 index.ts 导入名不变。
B4 src/store.ts 重写：保留 BranchReader/EntryAppender。
  foldGoalEntries(entries)（纯，导出，返回存储对象引用+元数据，注释勿修改）：线性扫描 custom 且 customType∈GOAL_EVENT_TYPES；逐条 validateGoalEvent（错误：v1 `corrupt goal snapshot: <problem>`；v2 `corrupt goal event (seq N, <customType>): <problem>`）；序号检查（v2 `goal event seq N follows seq M: v2 sequence must be contiguous`；v1 `goal snapshot seq N does not increase after M`）；base=最后一个 checkpoint|tombstone，无则 `orphan goal delta (seq N, kind K): no checkpoint precedes it`；tombstone 后仍有事件 `goal delta (seq N) after tombstone (seq M)`；tombstone 为末 → state undefined；checkpoint：state=structuredClone(cp.state)，v1 直接用内嵌 results，v2 要求无 verificationResults（`checkpoint seq N must not embed verificationResults`），verificationRef 非空则在 base 前找同 seq 的 verification 事件且 goalId 一致（`checkpoint seq N references verification seq M which is missing / not a verification event / belongs to another goal`）；重放 base+1..：goalId 必须等于 state.contract.id（`goal delta seq N for goal X, but the current goal is Y`）；usage 六项累加；criteria 要求 revision===contract.revision（`criteria delta seq N targets contract revision R, current revision R'`），verified 的 id 必须存在于合同（`criteria delta seq N references unknown criterion X`），progress 必须恰好覆盖合同 id 集（`criteria delta seq N: progress table does not cover the contract's criteria`），未变化项保留原 at，写入即清 stale；plan 同 revision 检查后整表替换；verification 同 revision 检查后替换 results 并记 verificationSeq；每条后 state.updatedAt=ev.at。forwardFill 沿用现有 M1 默认值。返回 {ok:true,state,lastSeq,checkpointSeq,deltasSinceCheckpoint,verificationSeq,counts:{v1,v2,checkpoints,deltas}}|{ok:false,error}。
  rebuildGoalState(reader) 保名，返回 clone+前向补默认+上述元数据。
  interface PersistedShadow{state:CheckpointState;resultsRef:GoalVerificationResult[]|undefined;verificationSeq:number|null}
  deriveGoalEvents(shadow|undefined,next,{persistCheckpointEvery,deltasSinceCheckpoint,at,checkpoint?:string,criteriaSource?})→{drafts,checkpointReason?}（纯；用 node:util isDeepStrictEqual；shadow 为 undefined → checkpoint "create"；contract/phase/blockedReason/completionProposal/completionRejections/debugLog 任一不同 → checkpoint；results 引用由有变为无 → checkpoint "verification-cleared"（verificationRef:null）；results 引用变为新数组 → 先出 verification draft；否则 usage（六计数任一不同）、criteria（criteriaStatus 或 criteriaProgress 不同；verifiedBy：progress 变→"self-report"，verified 变→"mechanical"，opts.criteriaSource 覆盖）、plan；deltasSinceCheckpoint+本次 delta 数 >= K → 改为 checkpoint "interval"；diff 排除 updatedAt 与 verificationResults；无差异 → drafts 空；checkpoint draft 的 verificationRef 为 "pending"（同批有 verification draft）| shadow.verificationSeq | null）。
  persistGoalEvents(appender,reader,drafts,baseSeq)→{lastSeq,written:[{customType,seq,kind,bytes}]}：尾部反向扫描 CAS（尾条目 seq 非数值 → `goal: refusing to write over corrupt state (...)`）；按 verification→checkpoint 或 verification→deltas 顺序分配连续 seq，解析 pending ref，structuredClone 载荷，appendEntry。
  persistTombstone 改写 v2 {v:2,seq,goalId,kind:"tombstone",tombstone:true,at}。保留 5 行 persistGoalState(appender,reader,state,baseSeq) 包装（发 reason "forced" checkpoint）以兼容 test 262。
B5 新文件 src/mutation-queue.ts：class SerialMutationQueue{private tail=Promise.resolve(); run<T>(fn):Promise<T>{const next=this.tail.then(fn,fn); this.tail=next.catch(()=>undefined); return next;}}。
B6 src/config.ts：persistCheckpointEvery:number 默认 50，loadGoalConfig 钳制 [1,1000]。
B7 src/index.ts：闭包 lastPersisted → persistedShadow；新增 deltasSinceCheckpoint=0、verificationSeq:number|null=null、const mutations=new SerialMutationQueue()。persist(ctx,opts?)：合法性检查改读 persistedShadow.state.phase；at=Date.now()；deriveGoalEvents；空 → logEvent("persist",{kind:"noop"}) 返回 true；否则 persistGoalEvents，成功后 baseSeq=lastSeq、deltasSinceCheckpoint（checkpoint→0，否则 +delta 数）、verificationSeq、persistedShadow=快照、state.updatedAt=at、logEvent("persist",{events:written,bytes,checkpointReason})；失败路径不变。rebuild(ctx) 设置四个新字段并 logEvent("rebuild",{found,lastSeq,checkpointSeq,deltasSinceCheckpoint,verificationSeq,counts})。agent_settled：在 goal-final 块之前加 if(inWrapUpRun&&state.phase!=="active") persist(ctx,{checkpoint:"wrap-up-settled"})。/goal edit：persist 前把 criteriaProgress 中 id 仍存在的条目标 stale:true、删除消失 id；state.plan 若存在标 stale。update_goal execute 用 mutations.run 分两段（段 1：校验+proposal+persist+进 verifying+persist；释放；await runMechanicalVerification；段 2：围栏检查+应用结果+persist/transition）；get_goal execute 整体 mutations.run。tool_execution_start：const GOAL_TOOLS=new Set(["get_goal","update_goal","report_progress","update_plan"])；if(!GOAL_TOOLS.has(event.toolName)) rt.workToolCallsThisRun+=1。agent_settled 指纹 computeFingerprint(rt.workToolCallsThisRun,rt.writesThisRun,state.criteriaStatus,state.criteriaProgress)。
B8 src/runtime.ts：GoalRuntime.workToolCallsThisRun（resetRunCounters 归零）；computeFingerprint 第四参数 criteriaProgress?，追加 `|` + 按 id 排序的 `id=status`（不含 note/stale）。
B9 测试：test/fixtures/ 放一份裁剪后的真实 v1 session 片段（含多条 criteria 的 goal 快照与 tombstone，去敏）。store 级：首写 create checkpoint/无变化无 draft；用量差 → 一条 usage 含 dRounds/dTools；contract/phase/proposal/rejections/blockedReason/verification-cleared 强制 checkpoint；results 变化 → verification 先于 checkpoint 且 verificationRef 指向它、checkpoint.state 无 verificationResults；K=3 回卷 → [checkpoint,usage,usage,checkpoint] 且 fold(all).state 深等于末 checkpoint.state；仅 v1 流与今日一致；v1 前缀+v2 后缀；it.each fail-loud（孤儿 delta/tombstone 后 delta/goalId 不符/seq 断档/seq 重复/v>2/checkpoint 内嵌 results/悬空 verificationRef/criteria revision 不符/progress 未覆盖）断言精确文案；CAS：尾部为 delta 时 stale write、尾 seq 非数值 refusing；queue 顺序与异常隔离。扩展级：20 轮非零用量+/goal budget+pause/resume 后 `/goal` 状态卡 == session_start{reload} 后状态卡，driver.goalState() 深等于对 structuredClone(entries) 的 fold，goal 条目数 <= 1+相变数+合同变更数+floor(usage/K)，goal-usage 数 == 非零用量 turn_end 数；机械验证 driver（exec 返回输出）：所有 goal 条目无 verificationResults、goal-verification 携带、goalState().verificationResults 相等；/goal reject 后末 checkpoint verificationRef===null 且 fold 无 results；/goal clear→新 goal：tombstone、新 goalId create、seq 连续；扩展 1040-1057：wrap-up 用量为 usage delta，末 goal 条目 reason "wrap-up-settled"、tokensUsed 一致、goal-final.usage 一致；现有 232/248/262/300/316/331/350/829/892/1463 不改即过；test 350 扩到 goal-usage 不可变；分支切换后下一写入 seq=新尾+1 无 stale write；合成 goal-plan 事件 fold 进 state.plan、edit 标 stale、clear 清；persistCheckpointEvery:1 每写皆 checkpoint；computeFingerprint 含 progress 状态、不含 note。
B10 文档：docs/design.md 142 行改为 checkpoint+delta 描述、§15 阻塞测试 1 加 delta 子句、508 状态；docs/implementation-notes.md 71（restore lint 在 fold 之后）、93-96（不可变性延伸到 delta 与 shadow）；README "Goal logs"（权威日志含 goal checkpoint 与 goal-usage/goal-criteria/goal-plan/goal-verification delta；终态 = fold 或 wrap-up-settled checkpoint / goal-final；persistCheckpointEvery）；roadmap §6.1 标已实现、§7-3 标"已定：内嵌"、附录 C 第 1 步改为"用 src/store.ts 的 foldGoalEntries 折叠"、附录 D-2 措辞；docs/upstream-pi-harness.ts 加一条 goal-usage 存在且 goal 条目数上界的断言。

## 协议 C：report_progress（roadmap §6.2.1；提交 3 `feat: add report_progress self-reported criteria progress`）
C1 工具（src/index.ts，在 update_goal 之后注册）：name "report_progress"，parameters Type.Object({items:Type.Array(Type.Object({criterionId:Type.String(),status:StringEnum(["not_started","in_progress","self_reported_met"]),note:Type.Optional(Type.String())}))})；描述说明整表替换、只覆盖全部 criteria、自报非证据。execute 整体 mutations.run：无 state 或 phase!=="active" → 返回 {content:[text],details:{accepted:false,reason:"inactive"}}（文案说明仅 active 可报）；校验：id 集合必须与合同恰好相同（缺项/重复/未知逐条列出）、note trim 后 ≤200 且不含 \n/\r（超长/多行拒绝，不静默改写）→ {accepted:false,reason:"invalid-progress",errors}；通过则构造新表（status 与 note 与旧项相同者保留原 at，否则 at=now；全部 stale 清除），赋 state.criteriaProgress，persist(ctx,{criteriaSource:"self-report"})；返回文本 `Progress recorded (self-reported, not Goal-verified): a in_progress, b self_reported_met, c not_started.` 与 details {accepted:true,revision,changed:[状态改变的 id]}。允许任意方向回退。
C2 规则：/goal edit 后存活 id 标 stale（B7 已含）；/goal reject 与 /goal budget 不动 progress。指纹用状态向量（B8 已含）。
C3 UI（src/ui.ts）：widget 每条 `⬜ C1 <text> · ◐ in progress[ (stale)]`，note 截 40；renderStatusCard 每条追加 `self-report: <status>[ (stale)] — <note>`；footer 计数 `◐n ◑m`。prompts/banner/get_goal 的自报维度已由协议 A 的 criteriaChecklist 承担。
C4 测试：整表校验五例（缺项/重复/未知 id/多行 note/>200 note）→ accepted:false 且不追加条目；非 active 拒绝；全表 self_reported_met 后 update_goal(complete) 仍走证据校验且 criteriaStatus 不变；同表重提不写入，仅 note 变写 goal-criteria delta 但指纹不变，状态变指纹变，self_reported_met→not_started 接受；/goal edit 标 stale、下一次报告清除；/goal reject 与 /goal budget 保留；持久化为 goal-criteria delta（goal 条目数不变、goal-criteria +1、progress 覆盖全部 id、verifiedBy "self-report"）且 fold 后 criteriaProgress 一致；Promise.all([tool("report_progress"),tool("update_goal",complete)]) 串行、两次写入 seq 连续、无 stale write；delta continuation 在下一轮显示 `C1 ⬜◐` 与一次性变化标注、再下一轮无标注；banner 与 get_goal 显示自报标记与 BANNER_PROGRESS_RULE。
C5 文档：roadmap §6.2.1 标已实现（含四项实现决策：goal-criteria delta 持久化、SerialMutationQueue、edit 后 stale、note ≤200 单行/允许回退）；README 工具列表加 report_progress；implementation-notes 加自报与验证分离说明。

## 完成定义
1. 四个提交按序合入 main，每个提交 `npm run check && npm test` 全绿，测试总数 ≥ 69 + 18 + 26 + 12。
2. 不得删除或弱化：test 1543-1564、232/248/262/300/316/331/350、829/892/1463。
3. 用 pi ≥0.84.2、transport "sse"、.pi/goal.json {debugLog:true} 跑一次真实 Goal（≥10 轮，含 pause→聊天→budget→resume、run 内 budget、/compact）；用 roadmap 附录 C 方法核对 session JSONL：goal 条目数远小于 turn 数、goal-usage 数等于计费 turn 数、foldGoalEntries 结果 == /goal 状态卡 == goal-final、run 边界与相变处 assistant usage.cacheRead 无整段冷调用；debug 日志 send_continuation 的 delta chars ≤1500、reason 序列符合 kickoff→8 delta→checkpoint/compaction→checkpoint/revision→checkpoint。把结果（路径、计数、有无冷调用）写进 roadmap §3.3 与 §8 第 6 步。
4. 汇报时列出：每个提交 hash、测试计数、真实 session 的 goal/goal-usage/goal-criteria/goal-verification 条目数与文件占比、任何偏离本指令的地方及原因。
```
