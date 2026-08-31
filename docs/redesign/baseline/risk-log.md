# 风险复盘

## M0

| 字段 | 内容 |
|---|---|
| 里程碑 | M0（尚未关闭） |
| 触发的风险 | RK1 首次全量构建失败；新增 RK15 主机缺少完整 Xcode/Metal；新增 RK16 全量 test-link 需要超过 57 GiB 可用空间 |
| 实际影响 | `cargo build --workspace --all-targets` 退出 101；无法建立 GUI 手工场景和真实旧 DB 样本；nextest 测试清单可由 detached baseline worktree生成，但全量执行尚无结果 |
| 已完成缓解 | 安装缺失的 protobuf；保留所有失败日志；测试清单固定为 11,651 项；采用仓库内改造前 SQLite fixtures 作为临时 DB 回归网 |
| 后续动作 | 在完整 Xcode 且有更大构建盘的 macOS runner 补跑 M0/M4 门禁，不得把当前基础设施失败列为产品测试豁免 |
