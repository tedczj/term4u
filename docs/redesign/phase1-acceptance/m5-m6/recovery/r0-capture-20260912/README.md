# R0 本机受控抓包验证 · 2026-09-12

**PASS：管理员认证后的本机抓包、受控 TCP/UDP 和直接发包进程归因。**
用户本轮明确授权管理员操作，经 macOS 系统认证执行有限时长的 tcpdump；退出 0。
仅捕获 loopback 上受控源端口/测试 TCP 端口。未改防火墙、BPF 设备权限或 sudo 配置。

- [授权执行结果](authorization-result.json)：UTC、完整捕获命令、退出码。
- [受控探针](probes.json)：Python PID 83755，UDP 源端口 49328，目标 49327/53/853；
  TCP 49329 完成连接及测试字节往返。
- [原始捕获](capture.log)：16 packets captured，0 packets dropped by kernel。
  PKTAP 元数据中的 `Python:83755` 与探针 PID 一致；53 端口解码为 `r0.invalid` 查询，
  接收侧可见 mDNSResponder PID 185。TCP 握手可见。
- [核验及日志 SHA256](verification.json)：非空、零内核丢包、PID、TCP、UDP 与 53/853 端口均通过。
- tcpdump 已退出，未留下常驻抓包进程。PKTAP 在 loopback 同时记录入站/出站，16 包不代表 16 次请求。

本次保存文本包记录，没有保存 pcap-ng。853 测试是受控 UDP 探针，不是真实 DNS-over-TLS。
直接 DNS 发包归因通过不等于经系统 resolver 的原始调用进程归因通过；系统 resolver、
外部接口、GUI/TUI 各 60 秒静默场景和第二道防线仍待正式验证。不能据此标记 C8 PASS。

本次解除“无管理员抓包权限”的阻塞；R0 仍有磁盘空间与完整网络归因条件待落实。
