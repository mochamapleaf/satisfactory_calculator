"""Variants vs base and HiGHS best: python cmp_v2.py outdir var1 var2 ..."""
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
best = load('highs_part*.jsonl')
H4 = load('highs4s_part*.jsonl')
H2 = load('highs2s_part*.jsonl')
vs = [(v, load(v + '.jsonl')) for v in sys.argv[2:]]
names = sorted(best)


def obj(r):
    return r['obj'] if r and r.get('status') == 'Optimal' and r['obj'] != 0 else None


print('| 场景 | HiGHS 60 s 最好 | microlp 基线 | ' + ' | '.join(v for v, _ in vs) + ' | HiGHS 2 s | HiGHS 4 s |')
print('|---|---|---|' + '---|' * (len(vs) + 2))
agg = {k: [] for k in ['base'] + [v for v, _ in vs] + ['h2', 'h4']}
cnt = {k: {'proven': 0, 'none': 0} for k in agg}
for n in names:
    ref = best[n]['obj']
    cells = []
    for key, r in [('base', base.get(n))] + [(v, dd.get(n)) for v, dd in vs]:
        o = obj(r)
        if o is None:
            cells.append('无方案')
            cnt[key]['none'] += 1
            continue
        rel = (o - ref) / abs(ref) * 100
        agg[key].append(rel)
        cnt[key]['proven'] += bool(r['proven'])
        cells.append(f"{'✔' if r['proven'] else ''}{rel:+.2f}%" + (f" ({r['t_end']:.0f} ms)" if r['proven'] else ''))
    for key, hh in (('h2', H2), ('h4', H4)):
        h = hh.get(n)
        if h is None or h['obj'] > 1e6 * max(1.0, abs(ref)) or h['obj'] > 1e29:
            cells.append('无解' if (h is None or h['obj'] > 1e29) else '极差')
            cnt[key]['none'] += 1
            continue
        rel = (h['obj'] - ref) / abs(ref) * 100
        agg[key].append(rel)
        opt = h['status'] == 'Optimal'
        cnt[key]['proven'] += opt
        cells.append(f"{'✔' if opt else ''}{rel:+.2f}%")
    print(f"| {n} | {ref:.6g} | " + ' | '.join(cells) + ' |')
print()
print('相对 HiGHS 60 s 最好解的差距(越小越好):')
for k, v in agg.items():
    v2 = sorted(v)
    print(f"- {k}: 有方案 {len(v)}/{len(names)}, 证明最优 {cnt[k]['proven']}, 中位 {statistics.median(v):+.2f}%, 平均 {statistics.mean([min(x, 50) for x in v]):+.2f}%, 最差 {max(v):+.1f}%, ≤0.1% 的 {sum(1 for x in v if x <= 0.1)} 个, >1% 的 {sum(1 for x in v if x > 1)} 个")
