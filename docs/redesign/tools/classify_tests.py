#!/usr/bin/env python3
"""term4u —— 删除集的规模统计与测试三分类。

产出 02 §7.7 与 07 §7.3 引用的全部数字，使它们可被任何人重新复现。

用法（在仓库根目录）：
    python3 docs/redesign/tools/classify_tests.py            # 打印汇总
    python3 docs/redesign/tools/classify_tests.py --detail   # 附每条路径的明细
    python3 docs/redesign/tools/classify_tests.py --out DIR  # 把 class-a/b/c 清单写到 DIR

删除集定义在同目录的 deletion-set.txt，与 03 §1 的模块处置总表一一对应。
M0 执行时应把它复制为 script/deletion_set.txt，让"计划删什么"与"实际删了什么"同源。
"""
import os, re, sys, subprocess
from collections import Counter

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
DELSET = os.path.join(os.path.dirname(__file__), 'deletion-set.txt')

# 存活测试文件若引用下列符号 / 模块路径，即判为类 B（编译期断裂）。
CLASS_B_PATTERNS = [
    r'crate::server::server_api', r'crate::server::telemetry', r'crate::server::cloud_objects',
    r'crate::server::experiments', r'crate::server::sync_queue', r'crate::server::voice_transcriber',
    r'crate::server::iap_identity_minter', r'crate::server::graphql',
    r'crate::auth\b', r'crate::crash_reporting', r'crate::autoupdate', r'crate::changelog_model',
    r'crate::ai::ambient_agents', r'crate::ai::cloud_environments', r'crate::ai::artifacts',
    r'crate::ai::agent_sdk', r'crate::ai::agent::', r'crate::ai::agent_management',
    r'crate::ai::predict', r'crate::ai::get_relevant_files', r'crate::ai::remote_agent_context',
    r'crate::ai::restored_conversations', r'crate::ai::agent_conversations_model',
    r'crate::ai::request_usage_model', r'crate::ai::cloud_agent_config',
    r'crate::ai::cloud_agent_settings', r'crate::ai::connected_self_hosted_workers',
    r'crate::ai::artifact_download', r'crate::ai::voice\b',
    r'crate::ai_assistant', r'crate::drive\b', r'crate::cloud_object',
    r'crate::terminal::shared_session', r'crate::remote_server', r'crate::experiments\b',
    r'crate::settings::cloud_preferences_syncer',
    r'\bServerApiProvider\b', r'\bAuthManager\b', r'\bAuthStateProvider\b',
    r'\bwarp_server_client\b', r'\bwarp_server_auth\b', r'\bwarp_multi_agent_client\b',
    r'\bcloud_object_client\b', r'\bcloud_object_models\b', r'\bcloud_objects\b',
    r'\bcomputer_use\b', r'\bremote_server\b', r'\bfirebase\b', r'\bvoice_input\b',
]

def nlines(p):
    with open(p, errors='ignore') as fh:
        return sum(1 for _ in fh)

def main():
    targets = [l.strip() for l in open(DELSET) if l.strip() and not l.startswith('#')]
    missing = [t for t in targets if not os.path.exists(os.path.join(ROOT, t))]
    if missing:
        print('!! 删除集里有不存在的路径（上游改过结构？）:', missing)

    def in_del(p):
        for t in targets:
            if p == t or p.startswith(t + '/'):
                return True
            if t.endswith('.rs') and p == t[:-3] + '_tests.rs':
                return True
        return False

    rows, tot = [], [0, 0, 0, 0]      # srcFiles, srcLines, testFiles, testLines
    for t in targets:
        p = os.path.join(ROOT, t)
        sf = tf = sl = tl = 0
        if os.path.isdir(p):
            for r, _, fs in os.walk(p):
                for f in fs:
                    if not f.endswith('.rs'):
                        continue
                    n = nlines(os.path.join(r, f))
                    if f.endswith('_tests.rs'):
                        tf += 1; tl += n
                    else:
                        sf += 1; sl += n
        elif os.path.isfile(p):
            sf, sl = 1, nlines(p)
            tp = p[:-3] + '_tests.rs'
            if os.path.exists(tp):
                tf, tl = 1, nlines(tp)
        rows.append((t, sf, sl, tf, tl))
        for i, v in enumerate((sf, sl, tf, tl)):
            tot[i] += v

    tests = subprocess.check_output(
        ["bash", "-lc", f"cd {ROOT} && find app crates -name '*_tests.rs' -type f | sort"],
        text=True).split()
    class_a = [p for p in tests if in_del(p)]
    surv = [p for p in tests if not in_del(p)]
    rx = re.compile('|'.join(CLASS_B_PATTERNS))
    class_b = [p for p in surv
               if rx.search(open(os.path.join(ROOT, p), errors='ignore').read())]
    class_c = [p for p in surv if p not in set(class_b)]
    la = sum(nlines(os.path.join(ROOT, p)) for p in class_a)
    lb = sum(nlines(os.path.join(ROOT, p)) for p in class_b)

    print(f"删除集条目            : {len(targets)}")
    print(f"删除集源文件 / 源码行 : {tot[0]} / {tot[1]}")
    print(f"类 A（随模块消亡）    : {len(class_a)} 文件 / {la} 行")
    print(f"类 B（编译期断裂）    : {len(class_b)} 文件 / {lb} 行")
    print(f"类 C 候选（可能漂移） : {len(class_c)} 文件")
    print(f"存活测试文件          : {len(surv)}   全量测试文件: {len(tests)}")
    print()
    print("类 B 热点：")
    c = Counter('/'.join(p.split('/')[:4]) if len(p.split('/')) > 4
                else '/'.join(p.split('/')[:-1]) for p in class_b)
    for k, v in c.most_common(10):
        print(f"  {v:4d}  {k}")

    if '--detail' in sys.argv:
        print()
        print(f"{'path':<48}{'srcF':>6}{'srcL':>9}{'tstF':>6}{'tstL':>9}")
        for t, a, b, cc, d in sorted(rows, key=lambda r: -(r[2] + r[4])):
            print(f"{t:<48}{a:>6}{b:>9}{cc:>6}{d:>9}")

    if '--out' in sys.argv:
        out = sys.argv[sys.argv.index('--out') + 1]
        os.makedirs(out, exist_ok=True)
        for name, lst in (('class-a', class_a), ('class-b', class_b),
                          ('class-c-candidates', class_c), ('deleted-tests', class_a)):
            with open(os.path.join(out, name + '.txt'), 'w') as fh:
                fh.write('\n'.join(lst) + '\n')
        print(f"\n已写入 {out}/{{class-a,class-b,class-c-candidates,deleted-tests}}.txt")

if __name__ == '__main__':
    main()
