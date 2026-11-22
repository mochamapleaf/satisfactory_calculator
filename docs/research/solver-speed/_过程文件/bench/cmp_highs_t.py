"""microlp 4 s vs HiGHS at 2/4/10/60 s limits. python cmp_highs_t.py outdir"""
import sys, json, glob, os, statistics
d = sys.argv[1]


def load(pattern):
    out = {}
    for f in glob.glob(os.path.join(d, pattern)):
        for line in open(f, encoding='utf-8'):
            if line.startswith('{'):
                r = json.loads(line)
                out[r['name']] = r
    return out


base = load('base4s.jsonl')
H = {T: load(f'highs{T}s_part*.jsonl') for T in (2, 4, 10)}
H[60] = load('highs_part*.jsonl')
names = sorted(H[60])
print('| 场景 | microlp 4 s obj (gap) | HiGHS 2 s | HiGHS 4 s | HiGHS 10 s | HiGHS 60 s |')
print('|---|---|---|---|---|---|')
stats = {T: {'opt': 0, 'better': 0, 'worse': 0, 'none': 0, 'rel': []} for T in H}
for n in names:
    b = base[n]
    bo = b['obj'] if b['status'] == 'Optimal' else None
    cells = []
    for T in (2, 4, 10, 60):
        h = H[T].get(n)
        if h is None:
            cells.append('–')
            continue
        ok = h['obj'] < 1e29 and h['status'] in ('Optimal', 'Time limit reached')
        if h['obj'] > 1e29:
            cells.append('无解')
            stats[T]['none'] += 1
            continue
        opt = h['status'] == 'Optimal'
        stats[T]['opt'] += opt
        if bo is not None:
            rel = (h['obj'] - bo) / abs(bo) * 100
            stats[T]['rel'].append(rel)
            if rel < -0.01:
                stats[T]['better'] += 1
            elif rel > 0.01:
                stats[T]['worse'] += 1
            cells.append(f"{'✔' if opt else ''}{h['highs_s']:.1f}s {h['obj']:.6g} ({rel:+.2f}%, gap {h['gap']*100:.2f}%)")
        else:
            cells.append(f"{'✔' if opt else ''}{h['highs_s']:.1f}s {h['obj']:.6g} (gap {h['gap']*100:.2f}%)")
    mb = f"{bo:.6g} ({float(b['gap'])*100:.2f}%)" if bo is not None else '无方案'
    print(f"| {n} | {mb} | " + ' | '.join(cells) + ' |')
print()
for T in (2, 4, 10, 60):
    s = stats[T]
    print(f"HiGHS {T} s: 共 {len(H[T])}; 证明最优 {s['opt']}; 无解 {s['none']}; 比 microlp 4 s 好(>0.01%) {s['better']}, 差 {s['worse']}; 相对差中位 {statistics.median(s['rel']):+.3f}%, 最好 {min(s['rel']):+.2f}%, 最差 {max(s['rel']):+.2f}%")
