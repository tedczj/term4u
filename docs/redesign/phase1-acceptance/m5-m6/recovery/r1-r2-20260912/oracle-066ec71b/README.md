# 066ec71b 真实运行数据库样本

源码：干净 detached `066ec71b736fc3755e29f58f733deadbdac3d1af`，公开 OSS bin `warp-oss`。
没有修改旧产品源码，未把旧 cloud/integration 依赖恢复到当前产品中。

## 生成与验收

- 使用新的 `r1r2-oracle-066ec71b-20260913` profile；构建、启动均移除 WARP_USER_SECRET。
  原版 secure-storage data_domain 按 profile 隔离。通过 Skip 进入未登录终端，未登录或创建账号。
- 原版 UI 创建两个标签页、三个终端 pane，并执行 FIRST / SECOND / SPLIT 三个 marker 命令；
  SECOND 的最后命令为 false，保留非零退出码。original.txt 为 ORACLE_DISK。
- 正常退出后 SQLite backup 冻结为 original.sqlite，SHA256 见 original-snapshot.json；
  原始数据库含 1 window、2 tabs、3 terminal panes、3 blocks、3 commands。
- 当前版本在另一个新 profile 读取数据库副本与原版 settings.toml，恢复两个标签页和水平分栏。
  执行 MIGRATED_WRITE_OK，current.txt 为 MIGRATED_DISK；退出、再启动后该命令仍可用 Ctrl-R 搜索。
- migration-comparison.json 对原有八张表逐行逐字段对比：原有行全部原样保留。
  原始冻结样本 SHA 不变，原 settings.toml 字节不变，两份 DB integrity_check 都为 ok。
  blocks 新增内部运行记录，commands 从 3 增至 4；新恢复状态写入 local_app_snapshots。
- 原版 UI 一度接受 Oracle Second Tab 名称，但退出后原库 tabs.custom_title 与
  pane_leaves.custom_vertical_tabs_title 仍为 NULL；本真实样本不覆盖持久化自定义标题。

## 构建限制与失败记录

离线 metadata 缺协议 cache；在线读取公开依赖后 check 通过。
内部 bin warp 虽构建成功，但运行会要求内部 warp-channel-config；改用同提交公开 OSS bin。
磁盘不足导致链接失败，及一次 cargo bundle 返回 0 但 rust-objcopy 报错、产物损坏；
这些日志保留为失败，未用该产物生成样本。

最终有效构建：

```
env -u WARP_USER_SECRET CARGO_TARGET_DIR=/Users/tedczj/workspace/term4u/target \
  CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 \
  cargo rustc -p warp --bin warp-oss --features channel_versions/default -- -C strip=none
```

channel_versions/default 与 workspace 默认成员产生的 feature 合并一致；仅最后 bin 禁止 strip。
将有效 Mach-O 复制到 bundle 后，执行原版的本地 rpath/plist/资源步骤与 ad-hoc codesign，
通过 codesign --verify --deep --strict；没有运行会下载 skills 的旧 script/run。
最终运行二进制 SHA、profile、PID 和时间见 run.json；当前版本两次运行见 current-run-*.json。

截图有已知 CUA 增量采集限制：用户确认实际当前 Term4u 窗口文字完整，原文记录在相邻
`gui-workspace/physical-screen-observation.json`。数据库、输出文件及 AX 文本独立保留。
