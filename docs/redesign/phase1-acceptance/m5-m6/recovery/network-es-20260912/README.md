# R0 本机 DNS 与 GUI/TUI 静默网络验证 · PASS

**本次指定 debug 产物的启动、静默运行和正常退出窗口通过。**
用户从已授权的 iTerm 手动 sudo 启动采集，应用由普通用户运行。
GUI 完成 620 秒观测后通过 Cmd-Q 正常退出；TUI 在真实 120×40 PTY 中运行 65 秒并 Ctrl-Q 正常退出。
两者主进程及完整派生进程树均为零网络包、零 DNS 请求，日志未见重试、上传或待发文件。

## 结果

| 项目 | 本轮结果 |
|---|---|
| 系统 DNS 代理归因 | 受控 getaddrinfo 查询可由 mDNSResponder 的 client PID 和请求 ID 归因 |
| 短命子进程校准 | 同一子进程在 ES、UDP 包和系统 DNS 日志三处均有证据 |
| GUI 进程树 | 114 个 PID；启动、十分钟空闲、正常退出，网络包 0 / DNS 0 |
| TUI 进程树 | 114 个 PID；65 秒静默及正常退出，网络包 0 / DNS 0 |
| 进程事件完整性 | 10,333 条事件；各事件类型及全局序号均连续，JSON 解析错误 0 |
| 采集收尾 | tcpdump exit 0；DNS/ES 按计划 SIGTERM 结束，无 SIGKILL |
| 内核丢包 | 0；101,907 packets captured，103,622 packets received by filter |
| 退出覆盖 | 两个主进程 stat 0；两棵树中全部 PID 最后均有 exit 事件，没有遗留子进程 |
| 隔离数据 | 两个独立 HOME/profile，干净 zsh 配置，HTTP(S)/ALL_PROXY 指向无效 loopback 代理 |

总包数包括这台机器上其他应用与校准探针的流量，**不是 Term4u 的外连数量**。
额外检查覆盖整个采集时段（含应用退出后的尾部）：两棵已归因进程树仍均无包或 DNS 请求。

## 可追溯证据

- [最终结果](result.json)：树中 PID、进程版本数、时间、退出覆盖、采集状态和原始日志 SHA256。
- [采集就绪与校准](capture-ready.json)、[正常收尾](capture-result.json)、[包统计](packet-statistics.log)。
- [短命子进程控制](short-child-control.json)、[跨三条链路的控制证据](short-child-control-evidence.json)。
- [GUI 执行与产物 SHA256](gui-final-run.json)、[正常退出操作](gui-quit.json)、[派生事件](gui-lineage.json)。
- [TUI 执行与产物 SHA256](tui-final-run.json)、[真实终端文本](tui-screen.txt)、[派生事件](tui-lineage.json)。
- [GUI 文件清单](gui-files.json)、[TUI 文件清单](tui-files.json)及本目录应用日志。
- `gui-packets.log` / `tui-packets.log` / `gui-dns.log` / `tui-dns.log` 是按完整进程树提取的结果，
  零长度与通过的采集/校准/序号/丢包检查共同使用，不单独以空文件判 PASS。
- `*-executed.py.txt` 为一次性执行/分析脚本审计副本，不是通用入口。

原始系统级日志仅保存在本机私有目录 `/private/tmp/term4u-network-wsyko7mj`，权限 0600。
它们可能包含其他进程的参数和环境，未提交到 Git；提交的进程派生证据只含时间、路径、PID、
PID version 和序号。分析使用父子关系、exec 版本转换及 responsible audit token 还原进程树。

## R0 与后续批次边界

R0 的本机捕获条件和系统 DNS/子进程归因缺口已关闭；之前的 TCC、进程组过滤及自动采集强杀
失败记录保留为历史，不与本轮 PASS 混用。后续使用 `script/capture_macos_network.py` 手动 sudo 入口。

本次是已记录 SHA256 的 debug GUI/TUI；不代表 release/bundle、全部 S1–S13、旧 DB 兼容、
第二道系统防火墙拒绝日志或 C1–C10 全部通过。这些仍由 R2/R6 各自验收。
严格 Clippy 的既有失败保留在前轮基线，本次没有修改 Rust 源码或豁免完整产品验收。

## 刷新后的失败基线

`./script/test_inventory list` [实际重跑](inventory-current.json)退出 101，当前是 134 个编译错误，
涉及旧 cloud_objects/blocklist/server ids/integration 等引用，不再是缺 Rust、nextest 或磁盘不足。
[原始日志](inventory-current.log)与[诊断分组](inventory-diagnostics.json)已保存；没有产生空清单或覆盖历史 baseline。
它属于后续 R1/R6 编译及全集测试收敛，不影响本轮已执行产物的网络观测结论。
`./script/format --check` 退出 0；Git 差异和本页证据链接检查通过。
