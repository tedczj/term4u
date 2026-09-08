# R1 · 终端持久化第一批（未验收）

## 变更

- `app/src/persistence/block_list.rs` 改用 `terminal::model::SerializedBlockListItem`，移除删除功能的 Agent 查询反序列化、写入/清理、NLD/Agent prompt-history 和可见性更新入口。历史 migrations/schema/表没有删除或改写，不恢复 `crate::ai::blocklist`。
- terminal block 保存使用现有本地 `removed_feature_metadata` / `removed_feature_visibility` 字段；读 DB 不回写旧 JSON。
- 本地恢复值补 `PartialEq` 以满足现有 `AppState` 的比较要求；test_utils 引用本地类型。
- 终端持久化新增真实 SQLite 路径单测：读写/顺序、100 条上限、pane 范围删除、旧 Agent 原始行不解析不修改、损坏旧列不影响输出且 DB 原值不变、历史 DB fixture 副本迁移及读取。

## 验证

- [SQL smoke](r1-block-sql-smoke.log)：Python 内存 SQLite 顺序执行全部历史 up.sql 和新测试的 SQL setup，exit 0；仅证明 setup SQL 可执行，不是 Diesel/Rust 测试通过。
- [focused nextest](r1-block-focused.log)：exit 127，Cargo 缺失。没有声称新测试通过。
- [inventory](inventory-initial.log)：exit 1；没有生成或更新空基线/删除 ID 清单。
- [diff --check](r1-diff-check.log)：exit 0。
- [fixtures 基线](fixture-baseline.json)：15 个旧 SQLite fixture 只读 schema/条数/完整性/SHA256，未输出用户正文。
- [migration SHA256](migration-baseline-sha256.txt)、[开工 diff](migration-diff-opening.log)、[94912b78 diff](migration-diff-94912b78.log)：284 个历史 migration 文件，开工均未变更。

## 本批消失测试逐项解释（R5 最终 ID 全集仍需真实 inventory）

这些旧测试全部测试已按路线 A 删除的 Warp Agent 功能，非 shell command history；不是为了编译变绿删除保留功能测试。原始内容可从开工 HEAD 获取。以下 ID 前缀均为 `warp::persistence::block_list::tests::`。

| 旧测试 ID 后缀 | 删除功能/替代覆盖 |
|---|---|
| `upsert_ai_query_caps_table_and_evicts_oldest_first` | Agent query 写入/驱逐已移除；旧行不写不删由 `terminal_persistence_leaves_legacy_agent_queries_opaque` 覆盖 |
| `upsert_ai_query_stays_below_limit_without_evicting` | 同上，产品不再写入 Agent query，保留原始数据 |
| `upsert_ai_query_updates_existing_exchange_without_evicting` | 同上，不再更新 exchange；不保留云协议模型或伪实现 |
| `process_ai_queries_for_nld_history_match_filters_empty_and_whitespace_inputs_oldest_first` | 已删除 Warp Agent/NLD prompt-history 的匹配候选，不是本地 shell history |
| `process_ai_queries_for_uparrow_prompt_keeps_newest_capped_oldest_first` | 已删除 Agent prompt-history；本地 terminal history 另有回归，不能算被此测试替代 |
| `process_ai_queries_for_uparrow_prompt_keeps_all_when_under_cap` | 同上 |
| `empty_input_skip_filters_out_non_query_inputs` | 已删除 Agent SDK input → persisted query 转换 |

新增测试前缀相同：`terminal_blocks_round_trip_in_chronological_order`、`terminal_block_limit_retains_newest_commands`、`deleting_terminal_blocks_is_scoped_to_the_pane`、`terminal_persistence_leaves_legacy_agent_queries_opaque`、`restoring_blocks_does_not_rewrite_unknown_legacy_columns`、`old_terminal_fixture_remains_readable_after_migrations`。

## 未完成风险

`sqlite.rs` 仍调用已删除 Agent/cloud event 和 snapshot，尚需下一批修复；当前不可宣称 library/tests 能编译。
全 WIP 消失测试、旧字段重写路径、窗口 snapshot 的清表逻辑、历史真样本、运行回归仍需 R1/R2/R5/R6 完整处理。
