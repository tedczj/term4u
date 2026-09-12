# Notebook 最终结果

2026-09-13 最终构建已原生复测旧正文、Enter 换行、编辑、退出、重启，结果通过。
见 final-result.json、final-after-quit.json、final-restored.png、final-edited.png、final-restarted.png。
临时渲染探针已删除；用户确认实际窗口文字完整。以下为此前失败尝试，保留历史而非当前状态。

# 原生 GUI 观察（尚未通过）

- 隔离 profile 从真实旧 notebook fixture 启动，显示 First Notebook / Notebook 1 content。
- File > Open Notebook 已列出 First Notebook；选择后定位到现有窗格，没有新增 tab。
- 编辑已写入 ~/.term4u-r1r2-gui-notebooks-20260912/notebooks/legacy-12.json。
- 发现 Enter 不换行；新增回归已复现，设置正文 EnterAction 后单测通过，仍须原生重测。
- 发现聚焦/编辑后旧文字消失，缩放窗口后恢复。截图 edit-repaint-problem.png 留证；未标 GUI PASS。
- 已正常退出首个 GUI 进程（PID 21097）。下一次构建包含临时 term4u_scene_probe 日志，仅用于诊断，必须在提交前删除。
