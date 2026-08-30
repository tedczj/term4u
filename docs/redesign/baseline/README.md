# baseline/ —— M0 基线快照目录

> **当前状态：占位。** 本目录下除本文件外**尚无内容**——`docs/redesign/` 是设计
> 文档，M0 尚未执行。下表列出 M0 完成时本目录应当包含的产物。

<a id="s1"></a>
## 一、M0 应产出的文件

| 文件 | 产生命令 | 说明 | 参考 |
|---|---|---|---|
| `environment.txt` | [07 §7.2.4](../07-测试与验证策略.md#s7-2-4) | 工具链版本 / 平台 / **冻结的命令** | 冻结的是命令，不是数字 |
| `tests.txt` | `./script/test_inventory snapshot baseline` | 全量测试清单（`cargo nextest list`） | [07 §7.2.3](../07-测试与验证策略.md#s7-2-3) |
| `known-fail.txt` | 跑三遍 nextest 取并集 | **首次即失败 / flake 的项。先接受现实，不修** | 同上 |
| `ignored.txt` | `grep -rn '#\[ignore' app crates --include='*.rs'` | 87 个 `#[ignore]` 及理由 | 同上 |
| `deleted-tests.txt` | `./script/test_inventory classify` | 类 A 白名单。**测试清单 diff 的唯一豁免依据** | [07 §7.3.1](../07-测试与验证策略.md#s7-3-1) |
| `class-a.txt` / `class-b.txt` / `class-c-candidates.txt` | 同上 | 三分类明细 | [07 §7.3](../07-测试与验证策略.md#s7-3) |
| `build-first.log` | `cargo build --workspace --all-targets` | 首次全量构建，含耗时 | [07 §7.2.1](../07-测试与验证策略.md#s7-2-1) |
| `presubmit-first.log` | `./script/presubmit` | 首次 presubmit | [07 §7.2.2](../07-测试与验证策略.md#s7-2-2) |
| `nextest-first.log` | `cargo nextest run --no-fail-fast --workspace --exclude command-signatures-v2` | | 同上 |
| `git-deps-licenses.txt` | [04 §1.1](../04-残余依赖与许可证架构.md#s1-1) 的核验脚本 | **6 个 warpdotdev git 依赖的许可证 + commit hash**。没有 hash 就没有证据 | [02 §5.3](../02-现状审计.md#s5-3) |
| `sum-tree-provenance.txt` | [06 §5.2](../06-阶段2-完整MIT路线.md#s5-2) | `crates/sum_tree` 是否为 Zed 衍生的判定 + 两侧 commit hash | [02 §3.4](../02-现状审计.md#s3-4) |
| `manual-scenarios-m0.md` | 手工执行 | **13 个场景的改造前基线。只此一次机会** | [07 §7.11](../07-测试与验证策略.md#s7-11) |
| `legacy-datadir/` | 用改造前的 `warp-oss` 真实使用后拷出 | DB 可加载性回归样本 | [07 §7.9.2](../07-测试与验证策略.md#s7-9-2) |
| `risk-log.md` | 每个里程碑追加 | 风险复盘 | [08 §3.1](../08-实施顺序与里程碑.md#s3-1) |
| `name-availability.txt` | M7 复验 | crates.io / GitHub 名称可用性 + 日期 | [04 §9.3](../04-残余依赖与许可证架构.md#s9-3) |
| `blocklist-decision.md` | M5 | `app/src/ai/blocklist` 走哪条路线 + 三个判据的实测值 | [03 §12.3](../03-阶段1-云模块删除与离线化.md#s12-3) |
| `dead-code-m<N>.txt` | 每个里程碑 | clippy 的 `dead_code` 清单，当**下一批删除候选**用 | [07 §7.7.3](../07-测试与验证策略.md#s7-7-3) |
| `tests-m<N>.txt` | 每个里程碑 | 该里程碑的测试清单快照，用于 diff | [07 §7.3.3](../07-测试与验证策略.md#s7-3-3) |

<a id="s2"></a>
## 二、`FIRST_REAL_USE` 标记

```
FIRST_REAL_USE: 未发生
```

> **含义**：这一行从"未发生"变成一个日期的那一刻起，**六处运行时身份的改名窗口
> 关闭**——之后再改名需要写数据目录迁移并轮换已存密钥。
>
> **规则**：在第一次用 term4u 打开一个你在乎其历史的会话之前，
> [05 §6.1](../05-阶段1-仓库与品牌.md#s6-1) 的 R1–R6 必须已经定稿。
>
> 详见 [08 §2.1](../08-实施顺序与里程碑.md#s2-1)。

<a id="s3"></a>
## 三、大文件的处理

`legacy-datadir/` 可能有几十到几百 MB。**不要直接提交进 git。** 两个选择：

| 方案 | 做法 |
|---|---|
| 只提交必要的 SQLite 文件 | 从 `legacy-datadir/` 里挑出数据库文件单独提交，并在本文件记录来源与大小 |
| 完全不入 git | 放在 `.gitignore` 里，在本文件记录它在本机的绝对路径与产生方式 |

无论哪种，**必须在本文件里记下它的产生方式**，否则换台机器就无法重建。
