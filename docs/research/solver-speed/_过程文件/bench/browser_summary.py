"""Summary of browser_bench runs: python -X utf8 browser_summary.py data_dir run1.jsonl run2.jsonl ..."""
import json, os, statistics, sys

d = sys.argv[1]


def load(path):
    return {r['name']: r for r in (json.loads(l) for l in open(os.path.join(d, path), encoding='utf-8') if l.startswith('{'))}


best = {}
for i in list(range(6)) + [9]:
    p = os.path.join(d, f'highs_part{i}.jsonl')
    if os.path.exists(p):
        best.update({k: r['obj'] for k, r in load(f'highs_part{i}.jsonl').items()})
runs = [(os.path.basename(p)[:-6], load(p)) for p in sys.argv[2:]]
ok = lambda r: r is not None and r['status'] == 'optimal' and r['obj'] != 0
names = sorted(set().union(*[set(r) for _, r in runs]))
ref = {n: min([best.get(n, float('inf'))] + [rs[n]['obj'] for _, rs in runs if ok(rs.get(n))]) for n in names}
print('| run | plans | proven in 4 s | median gap to best known | mean | >1 % | first plan median / max (ms) | HiGHS load (ms) |')
print('|---|---|---|---|---|---|---|---|')
for label, rs in runs:
    rows = [rs[n] for n in names if n in rs]
    plans = [r for r in rows if ok(r)]
    rel = [(r['obj'] - ref[r['name']]) / abs(ref[r['name']]) * 100 for r in plans if r['name'] in best]
    first = [r['first_ms'] for r in plans if r['first_ms'] is not None]
    loads = sorted({r.get('highs_load_ms') for r in rows if r.get('highs_load_ms')})
    print(f"| {label} | {len(plans)}/{len(rows)} | {sum(r['proven'] for r in plans)} | {statistics.median(rel):+.2f} % | {statistics.mean(rel):+.2f} % | {sum(x > 1 for x in rel)} | {statistics.median(first):.0f} / {max(first):.0f} | {', '.join(map(str, loads)) or '–'} |")
if len(runs) == 2:
    (la, a), (lb, b) = runs
    better = worse = 0
    lines = []
    for n in names:
        x, y = a.get(n), b.get(n)
        if not (ok(x) and ok(y)):
            lines.append(f'| {n} | {x and x["status"]} | {y and y["status"]} |')
            continue
        r = (y['obj'] - x['obj']) / abs(x['obj']) * 100
        better += r < -0.01
        worse += r > 0.01
        if abs(r) > 0.01 or x['proven'] != y['proven']:
            f = lambda q: f"{q['obj']:.6g}{' ✔' if q['proven'] else ''} ({q['end_ms']} ms)"
            lines.append(f'| {n} | {f(x)} | {f(y)} {r:+.2f} % |')
    print(f'\n{lb} vs {la}: better {better}, worse {worse} (|Δ| > 0.01 %)\n')
    print(f'| scenario | {la} | {lb} |\n|---|---|---|')
    print('\n'.join(lines))
