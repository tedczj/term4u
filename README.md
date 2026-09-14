# Term4u

A local terminal for macOS, derived from Warp.

**[完整设计、完成状态、剩余施工与验收](docs/DESIGN.md)** 是唯一实施依据。
仓库不维护并行的历史方案、施工单或进度文档；变更直接更新该设计。

Term4u 保留本地 GUI/TUI 终端、标签页与分栏、文件树、编辑/预览、搜索、本地 diff、
workflows、notebooks、PATH LSP 和本地持久化。产品目标是删除云控制面并以代码结构保护离线边界，
不是简单关闭上游开关。实现进度、已验证范围和未完成验收以设计中的状态表为准，不能把目标当成已通过认证。

Term4u 不提供 Warp 的云账户、内建云 Agent、Drive 同步、共享会话或自动更新。
外部 CLI agent、git、ssh 等作为普通 PTY 子进程运行，网络由用户及该工具控制。
**Term4u 不是沙箱，不限制用户主动启动的命令联网。**

## Build and run

Only macOS is a supported product platform. Use the pinned toolchain and prerequisites described in
[the design](docs/DESIGN.md#verification) and [AGENTS.md](AGENTS.md).

```bash
./script/run       # GUI
./script/run-tui   # TUI
./script/presubmit # Engineering checks; not the entire product acceptance matrix
```

No upstream download page is a Term4u release. Packaging and release requirements are defined in
[the same design](docs/DESIGN.md#release).

## Source and licensing

This project derives from `warpdotdev/warp` at
`066ec71b736fc3755e29f58f733deadbdac3d1af`. It is not affiliated with, endorsed by, or sponsored by
the upstream vendor. Upstream names are used only to identify origin and retained internal symbols.

The inherited code remains under [AGPL-3.0-only](LICENSE-AGPL), except `crates/warpui` and
`crates/warpui_core`, which retain their [MIT license](LICENSE-MIT) and original copyright notices.
This is not a claim that the application or the islands' dependency closure is entirely MIT.

See [CONTRIBUTING.md](CONTRIBUTING.md) for collaboration and verification rules.
