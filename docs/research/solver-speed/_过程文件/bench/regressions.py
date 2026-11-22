"""List scenarios where run B is worse than run A by more than a threshold (default 0.1 %).
python -X utf8 regressions.py A.jsonl B.jsonl [pct]   (bench_prod/bench_stream or browser_bench rows)
"""
import json, sys

a, b = [{r['name']: r for r in map(json.loads, open(p, encoding='utf-8')) } for p in sys.argv[1:3]]
pct = float(sys.argv[3]) if len(sys.argv) > 3 else 0.1
ok = lambda r: r is not None and r['status'].lower() == 'optimal' and r['obj'] != 0
worse = []
for n, x in a.items():
    y = b.get(n)
    if not ok(x):
        continue
    if not ok(y):
        worse.append((n, x['obj'], None, float('inf')))
        continue
    r = (y['obj'] - x['obj']) / abs(x['obj']) * 100
    if r > pct:
        worse.append((n, x['obj'], y['obj'], r))
print(f'{len(worse)} scenario(s) worse by > {pct} %')
for n, x, y, r in worse:
    print(f'| {n} | {x:.6g} | {y if y is None else f"{y:.6g}"} | {r:+.2f} % |')
