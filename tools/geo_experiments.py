"""Rail-vs-belt experiments on the real map (G7 revision). Every comparison solves the same instance with rail
allowed and rail disabled; a result counts as conclusive only if both are proven optimal or the difference exceeds
the larger optimality gap.

Usage (one group per process; each writes _过程文件/exp_<group>.json after every solve):
  python -I tools/geo_experiments.py volume      # Iron Plate at Sm3_0: 1500 .. 24000 /min (6 volumes)
  python -I tools/geo_experiments.py work        # 12000 plate at Sm3_0, work 0 / 250 / 1000 OE per km
  python -I tools/geo_experiments.py base_only   # 10 HMF at Sm3_0 with machines only at Sm3_0
  python -I tools/geo_experiments.py hubs        # L9: 12000 plate at Sm3_0 with hubs picked from a belt-only solve
  python -I tools/geo_experiments.py default     # 10 HMF at S0_m1 (the default instance)
Solves are time-limited (--time-limit, default 300 s) and geo_solve's watchdog prints [hb] lines.
"""
import argparse
import copy
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import geo_solve as gs  # noqa: E402

TRANSPORT = ('b', 'bp', 'h', 'u', 'z', 'c', 'cf')
PLATE = 'Desc_IronPlate_C'
HMF = 'Desc_ModularFrameHeavy_C'


def one(d, label, targets, tl, **kw):
    res, m, idx, meta = gs.run(d, targets, label=label, time_limit=tl, **kw)
    s = res.get('summary', {})
    cp = s.get('cost_parts', {})
    return {'label': label, 'obj': res['obj'], 'bound': res['bound'], 'gap': res['gap'], 'status': res['status'],
            'time_s': round(res['time'], 1), 'transport': round(sum(cp.get(k, 0.0) for k in TRANSPORT), 2),
            'cost_parts': cp, 'hubs': s.get('hubs'), 'belts': s.get('belt_count'),
            'rail_flow': s.get('rail_flow_items_per_min'),
            'cars': sum(dd['cars'] for ln in s.get('lines', []) for dd in ln['dirs'].values()),
            'lines': [(ln['a'], ln['b']) for ln in s.get('lines', [])], 'cbar_binding': s.get('cbar_binding'),
            'machines_total': s.get('machines_total')}


def compare(d, name, targets, tl, **kw):
    r = one(d, f'{name} rail', targets, tl, **kw)
    b = one(d, f'{name} belt-only', targets, tl, rail=False, **kw)
    both_opt = r['status'] == b['status'] == 'Optimal'
    diff = b['obj'] - r['obj']
    gapabs = max(r['gap'] * abs(r['obj']), b['gap'] * abs(b['obj']))
    tol = gapabs + 1e-6 * abs(b['obj'])
    verdict = ('rail cheaper' if diff > tol and (r['rail_flow'] or 0) > 0 else
               'belt cheaper (time-limited incumbent)' if diff < -tol else 'same (no rail used)')
    kw = {k: sorted(v) if isinstance(v, set) else v for k, v in kw.items()}
    return {'name': name, 'targets': targets, 'kw': kw, 'rail': r, 'belt': b, 'saving': round(diff, 2),
            'saving_pct': round(100 * diff / b['obj'], 2), 'conclusive': both_opt or abs(diff) > gapabs,
            'verdict': verdict}


def hubs_from_belt_solve(d, targets, base, k=12, **kw):
    """L9: base + demand site + the K//3 sites with the largest belt outflow in a belt-only solve, the rest filled by
    farthest-point sampling (same rule as geo_sites.py otherwise)."""
    res, m, idx, meta = gs.run(d, targets, label='belt-only for hub choice', time_limit=kw.pop('tl', 300),
                               rail=False, base=base, **kw)
    out = {s: 0.0 for s in d.sid}
    for (it, sa, sb), j in idx['f'].items():
        out[sa] += res['x'][j]
    chosen = [d.base] + ([base] if base != d.base else [])
    want = len(chosen) + k // 3
    for s in sorted(out, key=lambda s: (-out[s], s)):
        if len(chosen) >= want:
            break
        if s not in chosen and out[s] > 1e-6:
            chosen.append(s)
    xy = {s['id']: (s['x'], s['y']) for s in d.sites}
    while len(chosen) < k:
        far = max((s for s in d.sid if s not in chosen),
                  key=lambda s: (min(math.dist(xy[s], xy[c]) for c in chosen), s))
        chosen.append(far)
    doc = copy.deepcopy(d.sites_doc)
    doc['hubs'] = [{'site': s, 'reason': 'base' if s == d.base else 'belt_flow'} for s in chosen]
    return gs.Data(sites_doc=doc), chosen, {s: round(v, 1) for s, v in out.items() if v > 1e-6}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('group')
    ap.add_argument('--time-limit', type=float, default=300.0)
    a = ap.parse_args()
    d = gs.Data()
    tl = a.time_limit
    out_path = os.path.join(gs.OUTDIR, f'exp_{a.group}.json')
    rows = []

    def save(extra=None):
        with open(out_path, 'w', encoding='utf-8') as fh:
            json.dump({'group': a.group, 'time_limit_s': tl, 'rows': rows, **(extra or {})}, fh, indent=1)

    if a.group == 'volume':
        for q in (1500, 3000, 6000, 9000, 12000, 24000):
            rows.append(compare(d, f'{q} plate at Sm3_0', {PLATE: float(q)}, tl, base='Sm3_0')); save()
    elif a.group == 'work':
        for w in (250.0, 1000.0):
            rows.append(compare(d, f'12000 plate at Sm3_0, work {w:g}', {PLATE: 12000.0}, tl, base='Sm3_0',
                                work_km=w)); save()
    elif a.group == 'base_only':
        rows.append(compare(d, '10 HMF at Sm3_0, machines only there', {HMF: 10.0}, tl, base='Sm3_0',
                            machines_at={'Sm3_0'})); save()
        rows.append(compare(d, '10 HMF at Sm3_0, machines anywhere', {HMF: 10.0}, tl, base='Sm3_0')); save()
    elif a.group == 'hubs':
        d2, chosen, flow = hubs_from_belt_solve(d, {PLATE: 12000.0}, 'Sm3_0', tl=tl)
        extra = {'hubs_sites_json': d.hubs, 'hubs_from_belt_solve': sorted(chosen), 'belt_outflow': flow}
        save(extra)
        r = one(d2, '12000 plate at Sm3_0, belt-flow hubs, rail', {PLATE: 12000.0}, tl, base='Sm3_0')
        rows.append({'name': '12000 plate at Sm3_0, belt-flow hubs', 'rail': r}); save(extra)
    elif a.group == 'default':
        rows.append(compare(d, '10 HMF at S0_m1', {HMF: 10.0}, tl)); save()
    else:
        sys.exit(f'unknown group {a.group}')
    for r in rows:
        print(json.dumps({k: r[k] for k in ('name', 'saving', 'saving_pct', 'conclusive', 'verdict') if k in r}))


if __name__ == '__main__':
    main()
