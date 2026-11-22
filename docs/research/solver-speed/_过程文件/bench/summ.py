"""Summarise bench jsonl: python summ.py file [only_unproven]"""
import sys, json
rows = []
cur = None
for line in open(sys.argv[1], encoding='utf-8'):
    if line.startswith('{'):
        cur = json.loads(line)
        cur['traj'] = []
        rows.append(cur)
    elif line.strip().startswith('traj') and cur is not None:
        _, t, inc, b, n = line.split()
        cur['traj'].append((float(t), float(inc), float(b), int(n)))
only = len(sys.argv) > 2
print(f"{'name':28} {'units':>5} {'int':>5} {'rows':>5} {'t_lp':>6} {'t_sub':>6} {'t_end':>7} {'nodes':>7} {'ceil%':>7} {'sub%':>7} {'gap%':>7} {'lpgap%':>7} prov t_best")
for r in rows:
    if only and r['proven']:
        continue
    obj = r['obj']
    if r['status'] != 'Optimal' or obj == 0:
        print(f"{r['name']:28} {r['units']:5} {r['int']:5} {r['rows']:5} status={r['status']}")
        continue
    ceil = (r['ceil_obj'] - obj) / abs(obj) * 100
    sub = float(r['sub_obj'])
    subp = (sub - obj) / abs(obj) * 100 if sub == sub else float('nan')
    gap = float(r['gap']) * 100
    lpgap = (obj - r['lp_obj']) / abs(obj) * 100
    # time at which the final incumbent value was first seen
    tb = ''
    for (t, inc, b, n) in r['traj']:
        if inc == inc and abs(inc - obj) <= 1e-9 * abs(obj):
            tb = f"{t:.0f}"
            break
    print(f"{r['name']:28} {r['units']:5} {r['int']:5} {r['rows']:5} {r['t_lp']:6.1f} {r['t_sub']:6.0f} {r['t_end']:7.0f} {r['nodes']:7} {ceil:7.3f} {subp:7.3f} {gap:7.3f} {lpgap:7.3f} {'Y' if r['proven'] else 'N':>4} {tb}")
n = len(rows)
np_ = sum(1 for r in rows if not r['proven'])
print(f"total {n}, unproven {np_}")
