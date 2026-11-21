"""Closed-form rail-vs-belt break-even for one point-to-point haul, using geo_solve's own cost coefficients.

One haul of Q items/min (stack size sigma) over a route of L metres (the same L for belts and track):
  belts  ceil(Q / cap) * L_km * belt_oe_km
  rail   2 F (two hubs) + L_km * track_oe_km + loco + n_cars * (car + 2 platforms),
         n_cars = ceil(max(Q / sigma / (slots / RtD), Q / platform_cap(feed, RtD)))
Extraction and machines are the same for both modes and cancel. work_km adds player work per km of each belt and
of each of the two tracks (geo_solve.build's knob).

Usage:
  python -I tools/geo_breakeven.py                  # table + plot into docs/research/satisfactory-geo/_过程文件
"""
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import geo_solve as gs  # noqa: E402


def haul_costs(meta, Q, stack, L_m):
    """(belt OE, rail OE, cars) for one haul; meta from geo_solve.build."""
    km = L_m / 1000.0
    belt = math.ceil(Q / meta['belt_cap'] - 1e-9) * km * meta['belt_oe_km']
    rtd = gs.rtd_min(L_m, meta['v_mps'], meta['dock_s'], meta['acc_s'])
    per_car = min(meta['slots'] / rtd * stack, gs.platform_cap(meta['feed'], rtd, meta['dock_s']))
    cars = math.ceil(Q / per_car - 1e-9)
    rail = 2 * meta['F'] + km * meta['track_oe_km'] + meta['loco_oe'] + cars * (meta['car_oe'] + 2 * meta['platform_oe'])
    return belt, rail, cars


def threshold_belts(meta, stack, L_m, nmax=60):
    """Smallest number of full belts n for which rail <= belts at Q = n * cap and for every larger n up to nmax."""
    ok = [haul_costs(meta, n * meta['belt_cap'], stack, L_m)[1] <= haul_costs(meta, n * meta['belt_cap'], stack, L_m)[0]
          for n in range(1, nmax + 1)]
    for i in range(len(ok)):
        if all(ok[i:]):
            return i + 1
    return None


def table(d, works=(0.0, 250.0, 1000.0), stacks=(100, 200, 500), dists_km=(1, 2, 3, 5, 7, 10), fmults=(1.0,)):
    rows = []
    for w in works:
        for fm in fmults:
            _, _, meta = gs.build(d, {'Desc_ModularFrameHeavy_C': 10.0}, rail=False, work_km=w, fmult=fm)
            meta['v_mps'] = gs.tv(d.tr['train']['speed_mps'])
            for st in stacks:
                for dk in dists_km:
                    n = threshold_belts(meta, st, dk * 1000.0)
                    rows.append({'work_km': w, 'fmult': fm, 'stack': st, 'route_km': dk, 'belts': n,
                                 'items_per_min': n * meta['belt_cap'] if n else None,
                                 'belt_cap': meta['belt_cap']})
    return rows, meta


def plot(rows, png):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig, (a1, a2) = plt.subplots(1, 2, figsize=(10, 4.2), dpi=130, sharey=True)
    sel = lambda **kw: [r for r in rows if all(r[k] == v for k, v in kw.items()) and r['belts']]
    for st, col, ls, mk in ((100, '#2a7fbf', '-', 'o'), (200, '#222222', '-', 's'), (500, '#c0392b', '--', '^')):
        rs = sel(stack=st, work_km=0.0, fmult=1.0)
        a1.plot([r['route_km'] for r in rs], [r['items_per_min'] for r in rs], ls, marker=mk, ms=4, color=col,
                label=f'stack {st}')
    a1.set_title('default costs (F 1x, no work weight)', fontsize=9)
    for lab, kw, col, ls in (('F 0.5x', dict(work_km=0.0, fmult=0.5), '#2a7fbf', '--'),
                             ('F 1x', dict(work_km=0.0, fmult=1.0), '#222222', '-'),
                             ('F 2x', dict(work_km=0.0, fmult=2.0), '#c0392b', '--'),
                             ('work 250 OE/km', dict(work_km=250.0, fmult=1.0), '#27ae60', ':'),
                             ('work 1000 OE/km', dict(work_km=1000.0, fmult=1.0), '#8e44ad', ':')):
        rs = sel(stack=200, **kw)
        a2.plot([r['route_km'] for r in rs], [r['items_per_min'] for r in rs], ls, marker='o', ms=3, color=col,
                label=lab)
    a2.set_title('stack 200: sensitivity to F and player work', fontsize=9)
    for ax in (a1, a2):
        ax.set_xlabel('route length (km, after detour)')
        ax.set_yscale('log')
        ax.grid(alpha=0.3, which='both')
        ax.legend(fontsize=7)
    a1.set_ylabel('flow above which one rail line is cheaper (items/min)')
    fig.suptitle('rail vs Mk.5 belts (780/min), one haul; missing points = not cheaper within 60 belts', fontsize=9)
    fig.tight_layout()
    fig.savefig(png)
    plt.close(fig)


def main():
    d = gs.Data()
    rows, meta = table(d, fmults=(1.0, 0.5, 2.0))
    out = os.path.join(gs.OUTDIR, 'breakeven.json')
    with open(out, 'w', encoding='utf-8') as fh:
        json.dump({'note': 'smallest number of full Mk.5 belts (780/min) above which one rail line is cheaper',
                   'rows': rows}, fh, indent=1)
    plot(rows, os.path.join(gs.OUTDIR, 'breakeven.png'))
    for w in (0.0, 250.0, 1000.0):
        for fm in (1.0, 0.5, 2.0):
            print(f'work {w:g} OE/km, F {fm}x')
            for st in (100, 200, 500):
                print('  stack', st, ' '.join(f'{r["route_km"]}km:{r["belts"]}' for r in rows
                                             if r['work_km'] == w and r['stack'] == st and r['fmult'] == fm))


if __name__ == '__main__':
    main()
