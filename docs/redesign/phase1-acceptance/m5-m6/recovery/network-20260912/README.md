# 本机 DNS 归因与应用静默网络验证 · 2026-09-12

**当前仍 INCOMPLETE：完整短命子进程追踪缺 TCC 完全磁盘访问权限。**
系统 DNS 代理归因已校准；GUI 实际运行 620 秒，TUI 两次各运行约 65 秒。
[最终分析](network-analysis.json)显示已追踪 PID 的网络包、DNS 记录和重试/上传日志命中均为 0；
但自动采集的 tcpdump 仍需 SIGKILL 收尾，缺失最终丢包统计，因此不能记为零外连 PASS。
手动 sudo 入口已实测能正常停止 tcpdump（exit 0），仍须先解决 TCC，再完整重跑。

## 手动 sudo 入口

在本机终端执行：

```bash
sudo /usr/bin/python3 /Users/tedczj/workspace/term4u/script/capture_macos_network.py
```

脚本仅支持 macOS，默认最多采集 900 秒。输出目录由 `mkdtemp` 新建，目录权限 0700，
日志权限 0600，交还 sudo 发起者读取。完整原始日志可能包含其他应用的网络或进程元数据，
仅留本机，发布证据前按测试进程和观测窗口提取。

只有 tcpdump、mDNSResponder 日志和 Endpoint Security 进程事件采集可用，且受控 UDP、
系统 DNS 与 exec 探针能归因时，才打印 `READY`。把目录发给协作代理，再由普通用户运行应用探针；
**不要用 sudo 启动 GUI/TUI**。结束时按 Ctrl-C，或以普通用户 `touch 输出目录/STOP`。
脚本停止并等待全部子进程，保存命令、退出码和 SHA256；强杀采集器或采集中途失败均记 BLOCKED。
`COLLECTED_NOT_YET_VERIFIED` 只代表采集结束，还需要分析时长、进程覆盖和丢包，不等于应用 PASS。

首次手动运行目录 `/private/tmp/term4u-network-nro3d26p`：sudo 成功，eslogger exit 1，
明确报 `ES_NEW_CLIENT_RESULT_ERR_NOT_PERMITTED`。脚本按预期阻止 READY 并清理其他采集器。
需要给**运行 sudo 的终端应用**开启“系统设置 → 隐私与安全 → 完全磁盘访问权限”，
完全退出并重开终端，再执行同一命令。sudo 不能代替 TCC 授权，也不应关闭 SIP。

## 已落实与证据边界

- [缓存盘点与清理](cache-cleanup.json)：用户批准后仅删除可重建的 `target/debug/deps` 和
  `.fingerprint`；GUI 两个产物哈希未变，释放约 4.6 GB，空间阻塞已解除。
- [产物来源](artifact-baseline.json)：现有 GUI 与新构建 TUI 的路径/SHA256；GUI 构建基线
  `ee130862` 到当前 `50694605` 的 app/crates/script/Cargo 输入差异为空。两者是 debug 产物，
  不代替 R6 release/bundle 完整验收。
- [TUI 构建](tui-build.log)成功，资源按 `script/run-tui` 的本地方式准备；
  [网络边界静态检查](network-boundaries.log)通过。
- [DNS 探针](dns-probe.json)调用真实 `getaddrinfo`；[系统日志](dns-calibration-attribution.log)
  用 client PID 和请求 ID 关联 A/AAAA 查询。掩码 qname 不影响调用进程归因。
- [GUI](gui-final-run.json)：隔离 HOME、WARP_DATA_PROFILE、空 zsh 配置、无效 loopback 代理，
  620 秒结束时应用仍活着；真实窗口已观察，日志确认 shell bootstrapped。
- [TUI](tui-final-run.json)：120×40 PTY，显示本地 terminal 零状态，约 65 秒静默，
  Ctrl-Q 正常退出（exit 0）。首轮 `tui-run.json` 仅等待退出 1 秒，未确认正常退出，
  因而追加本轮完整退出观测，没有覆盖首轮记录。
- 第一次 GUI 采集 `gui-run.json` 对应 `/tmp/term4u-gui-net-01`：采集器被 SIGKILL，
  缺少最终丢包统计，不计正式 PASS；调整后第二轮仍出现此问题，见 [收尾记录](capture-result.json)。
  第二轮所有采集进程均已结束；正式重跑改用用户手动 sudo 入口，不再使用 AppleScript 管理采集生命周期。
- [eslogger 错误](eslogger-unavailable.log)与 [DTrace 探针查询](dtrace-availability.json)：
  前者需 TCC 权限，后者被 SIP 拒绝。DTrace 命令虽返回 0，但没有可用探针，不算可用。
  macOS SDK 明确不支持 kqueue NOTE_TRACK；受控尝试得到 errno 45，不以此替代 ES。

`term4u-*.py.txt` 是本次一次性执行脚本的审计副本，包含当时的绝对路径和进程清理操作，
不是可重用入口。后续管理员操作统一使用 `script/capture_macos_network.py`。

## 尚未关闭

1. 从具备完全磁盘访问权限的终端采集完整 exec/fork/exit 事件，重跑应用窗口并审计短命子进程。
2. 完整 C8 的 release 产物、其他 GUI 操作场景、第二道系统防火墙拒绝日志仍不属于本次已通过项。
3. 全仓严格 Clippy 仍失败；本次网络验证不修改或豁免此门禁，未 push。
