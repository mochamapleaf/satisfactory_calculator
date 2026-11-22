"""Before/after summary of bench_prod runs against the report's HiGHS 60 s best solutions.
python -I impl_summary.py data_dir run1.jsonl run2.jsonl ...   (paths relative to data_dir)
"""
import json, os, statistics, sys

d = sys.argv[1]


def load(path):
    out = {}
    for line in open(os.path.join(d, path), encoding='utf-8'):
        if line.startswith('{'):
            r = json.loads(line)
            out[r['name']] = r
    return out


best = {}
for i in list(range(6)) + [9]:
    p = os.path.join(d, f'highs_part{i}.jsonl')
    if os.path.exists(p):
        for k, r in load(f'highs_part{i}.jsonl').items():
            best[k] = r['obj']
runs = [(os.path.basename(p).replace('.jsonl', ''), load(p)) for p in sys.argv[2:]]
names = sorted(next(iter(runs))[1])
feasible = [n for n in names if not n.startswith('fx_sci7')]


def has_plan(r):
    return r and r['status'] == 'Optimal'


print('| run | plans / 105 | proven in 4 s | no plan | median gap to HiGHS-60s best | mean | >1 % | median time to first plan | median solve ms (proven) |')
print('|---|---|---|---|---|---|---|---|---|')
for label, rs in runs:
    rows = [rs.get(n) for n in feasible]
    plans = [r for r in rows if has_plan(r)]
    proven = [r for r in plans if r['proven']]
    rel = []
    for n in best:
        r = rs.get(n)
        if has_plan(r):
            # the best known is HiGHS 60 s or any run's plan (some microlp plans beat HiGHS within tolerance)
            ref = min([best[n]] + [x[1][n]['obj'] for x in runs if has_plan(x[1].get(n))])
            rel.append((r['obj'] - ref) / abs(ref) * 100)
    first = [r.get('first_ms') for r in plans if r.get('first_ms') is not None]
    fm = f"{statistics.median(first):.0f} ms" if first else '= end of solve'
    print(f"| {label} | {len(plans)} | {len(proven)} | {len(rows) - len(plans)} | {statistics.median(rel):+.2f} % | {statistics.mean(rel):+.2f} % | {sum(x > 1 for x in rel)} | {fm} | {statistics.median([r['solve_ms'] for r in proven]):.0f} |")
print()
print('Per scenario (obj, ✔ = proven, ms):')
print('| scenario | ' + ' | '.join(l for l, _ in runs) + ' |')
print('|---|' + '---|' * len(runs))
for n in feasible:
    cells = []
    for _, rs in runs:
        r = rs.get(n)
        if not has_plan(r):
            cells.append('no plan')
        else:
            extra = f", first {r['first_ms']:.0f} ms" if r.get('first_ms') is not None else ''
            cells.append(f"{'✔ ' if r['proven'] else ''}{r['obj']:.6g} ({r['solve_ms']:.0f} ms{extra}, {r.get('power', 0):.0f} MW)")
    print(f"| {n} | " + ' | '.join(cells) + ' |')
