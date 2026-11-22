"""microlp 4 s baseline vs HiGHS native: python cmp_highs.py base.jsonl highs1.jsonl ..."""
import sys, json
base = {}
for line in open(sys.argv[1], encoding='utf-8'):
    if line.startswith('{'):
        r = json.loads(line)
        base[r['name']] = r
hs = {}
for f in sys.argv[2:]:
    for line in open(f, encoding='utf-8'):
        if line.startswith('{'):
            r = json.loads(line)
            hs[r['name']] = r
print('| 场景 | 列/整数/行 | microlp 4 s: gap, obj | HiGHS: 时间, gap, obj, 节点 | microlp 方案比 HiGHS 差 |')
print('|---|---|---|---|---|')
n_opt = 0
fast = 0
worse = []
for n, h in sorted(hs.items()):
    b = base[n]
    if b['status'] != 'Optimal':
        mb = '无方案'
        diff = float('nan')
    else:
        mb = f"{float(b['gap'])*100:.2f}%, {b['obj']:.6g}"
        diff = (b['obj'] - h['obj']) / abs(h['obj']) * 100
        worse.append(diff)
    opt = h['status'] == 'Optimal'
    n_opt += opt
    fast += opt and h['highs_s'] <= 4
    print(f"| {n} | {b['units']}/{b['int']}/{b['rows']} | {mb} | {'✔ ' if opt else '✘ '}{h['highs_s']:.2f} s, {h['gap']*100:.2f}%, {h['obj']:.6g}, {h['nodes']}n | {diff:+.3f}% |")
print()
print(f"HiGHS 证明最优: {n_opt}/{len(hs)}; 其中 ≤4 s: {fast}")
import statistics
print(f"microlp 4 s 方案相对 HiGHS 最好解: 中位 {statistics.median(worse):+.3f}%, 最大 {max(worse):+.3f}%, >0.1% 的有 {sum(1 for w in worse if w > 0.1)} 个, >1% 的有 {sum(1 for w in worse if w > 1)} 个")
