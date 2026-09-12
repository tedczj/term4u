# R1 保留模块与消费者

本表是源码消费关系审计，不代替构建、测试与运行验证。

| 模块 | 本地用途与消费者 |
|---|---|
| `crates/ai/src/diff_validation` | 本地 diff 编辑/校验，由 `app/src/code` 与 `code_review` 使用 |
| `crates/ai/src/project_context` | `ProjectRulePath` 供 app SQLite 历史规则路径读写；按原有 SQLite 字段保持兼容 |
| `crates/ai/src/skills` | bundled/local skill 解析与文件读取；打包资源和文件编辑器仍可访问，旧 Agent skill manager 无消费者已删除 |
| `crates/ai/src/workspace` | SQLite 工作区元数据及时间排序，本次接回启动恢复与持久化 sender |
| `app/src/ai` | 本地 workspace/LSP 配置与旧 skill 来源值类型；无消费者的 Agent skill 缓存及空初始化函数已删 |
| `app/src/server/telemetry` | 仍被本地 UI/终端调用的类型与 no-op 宏；不建立连接、不上传 |

`crates/graphql` 不存在。`app/src/server/mod.rs` 只挂载 `telemetry`；孤立的旧 GraphQL 分享
`block.rs` 和云 ID 定义/测试已删除，见 `removed-server-orphans.json`。其他目录中未挂载的旧文件
不能作为产品运行时消费者；正常依赖树与完整构建已在最终门禁复核，见 normal-dependency-tree.log 和 presubmit-07.log。

`crates/ai` 的旧 GFM 格式化和 shell 路径辅助模块没有本地生产消费者，已连同专用测试删除；
本地 Markdown 编辑器使用 `crates/editor`、`crates/markdown_parser` 的实现。
