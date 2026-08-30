# tools/ —— 让文档里的数字可复现

本目录不是产品代码，是**让 [02](../02-现状审计.md) 与 [07](../07-测试与验证策略.md)
里的数字能被任何人重新算一遍**的工具。

| 文件 | 作用 |
|---|---|
| `deletion-set.txt` | V0 删除集的机器可读清单，与 [03 §1 的模块处置总表](../03-阶段1-云模块删除与离线化.md#s1) 一一对应 |
| `classify_tests.py` | 由删除集算出：删除集规模、测试三分类（类 A / 类 B / 类 C 候选）、类 B 热点 |

## 用法

```bash
cd "$(git rev-parse --show-toplevel)"
python3 docs/redesign/tools/classify_tests.py             # 汇总
python3 docs/redesign/tools/classify_tests.py --detail    # 附每条路径的明细
python3 docs/redesign/tools/classify_tests.py --out /tmp/inv   # 导出四份清单
```

## 基线输出（审计时，commit `066ec71b736fc3755e29f58f733deadbdac3d1af`）

```
删除集条目            : 53
删除集源文件 / 源码行 : 408 / 176188
类 A（随模块消亡）    : 143 文件 / 63886 行
类 B（编译期断裂）    : 151 文件 / 146101 行
类 C 候选（可能漂移） : 605 文件
存活测试文件          : 756   全量测试文件: 899
```

**如果你跑出来的数字和上面不一样**，说明代码已经变了（上游合并、或删除已经开始）。
这不是错误——重新记录一份即可。文档里冻结的是**产生数字的方法**，不是数字本身。

## M0 时怎么用

```bash
cp docs/redesign/tools/deletion-set.txt script/deletion_set.txt
```

之后**删除动作本身也从这份清单派生**，让"计划删什么"和"实际删了什么"永远一致：

```bash
while read -r p; do
  case "$p" in ''|\#*) continue;; esac
  git rm -r --quiet "$p"
done < script/deletion_set.txt
```

详见 [07 §7.3.4](../07-测试与验证策略.md#s7-3-4)。
