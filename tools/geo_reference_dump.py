"""G6: reference results of tools/geo_solve.py for the web port (react-frontend/src/geo/geo.test.ts).

Runs the geo_solve_test.py cases plus the report's 12 000 plate/min example and writes their numbers to
react-frontend/src/geo/python-ref.json; the TypeScript test solves the same models with highs-js and compares.
Each solve is time-limited; geo_solve's watchdog prints [hb] lines and kills a stalled solve.
Usage: python -I tools/geo_reference_dump.py
"""
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import geo_solve as gs  # noqa: E402
import geo_solve_test as gt  # noqa: E402

OUT = os.path.join(gs.ROOT, 'react-frontend', 'src', 'geo', 'python-ref.json')
HMF = {'Desc_ModularFrameHeavy_C': 10.0}
PLATE = {'Desc_IronPlate_C': 12000.0}
LATE = {'Desc_ModularFrameHeavy_C': 10.0, 'Desc_Computer_C': 5.0, 'Desc_Motor_C': 10.0, 'Desc_SteelPlateReinforced_C': 20.0}


def pack(res, meta):
    s = res.get('summary') or {}
    return {'status': res['status'], 'obj': res['obj'], 'bound': res['bound'], 'gap': res['gap'],
            'time_s': round(res['time'], 2), 'n_vars': res['n_vars'], 'n_int': res['n_int'], 'n_rows': res['n_rows'],
            'hubs': s.get('hubs'), 'belt_count': s.get('belt_count'), 'rail_flow': s.get('rail_flow_items_per_min'),
            'lines': [{'a': ln['a'], 'b': ln['b'], 'cars': {k: v['cars'] for k, v in ln['dirs'].items()}}
                      for ln in s.get('lines', [])],
            'cost_parts': s.get('cost_parts'), 'machines_total': s.get('machines_total'),
            'meta': {k: meta[k] for k in ('belt_tier', 'belt_cap', 'belt_oe_km', 'pipe_oe_km', 'track_oe_km', 'F',
                                          'loco_oe', 'car_oe', 'platform_oe', 'station_oe')}}


def main():
    d = gs.Data()
    gt.D = d
    out = {'generated': time.strftime('%Y-%m-%d %H:%M'), 'highspy': True, 'cases': {}}
    t0 = time.time()
    cases = [
        ('hmf_belt_only', d, HMF, dict(rail=False)),
        ('hmf_p_free', d, HMF, dict()),
        ('hmf_p0', d, HMF, dict(p=0, gap=1e-6)),
        ('hmf_no_rail_gap6', d, HMF, dict(rail=False, gap=1e-6)),
        ('fluid_oil_1500', d, {'Desc_LiquidOil_C': 1500.0}, dict()),
        ('plate12k_Sm3_0_rail', d, PLATE, dict(base='Sm3_0')),
        ('plate12k_Sm3_0_belts', d, PLATE, dict(base='Sm3_0', rail=False)),
        ('plate12k_Sm3_0_p2_f2', d, PLATE, dict(base='Sm3_0', p=2, fmult=2.0)),
        ('belts20_hmf_Sm3_0_p2', d, HMF, dict(base='Sm3_0', belt_mult=20.0, p=2)),
        ('tier6_hmf', d, HMF, dict(tier=6)),
        ('late_mix', d, LATE, dict(time_limit=120.0)),
    ]
    d2, _ = gt.two_site_data(5000.0)
    for Q in (780.0, 7800.0):
        cases.append((f'two_site_{Q:g}', d2, {'Desc_OreIron_C': Q}, dict(gap=1e-9)))
    for name, dd, tg, kw in cases:
        tl = kw.pop('time_limit', 180.0)
        res, m, idx, meta = gs.run(dd, tg, label=name, time_limit=tl, **kw)
        out['cases'][name] = {'targets': tg, 'kw': kw, **pack(res, meta)}
        with open(OUT, 'w', encoding='utf-8') as fh:  # checkpoint after every case
            json.dump(out, fh, indent=1, sort_keys=True)
    out['oe'] = gs.ore_equivalents(d, ['Desc_IronPlate_C', 'Desc_SteelPipe_C', 'Desc_SteelPlate_C',
                                       'Desc_AluminumPlate_C', 'Desc_Motor_C', 'Desc_Plastic_C'])
    with open(OUT, 'w', encoding='utf-8') as fh:
        json.dump(out, fh, indent=1, sort_keys=True)
    print(f'reference written to {OUT} in {time.time() - t0:.0f}s', flush=True)


if __name__ == '__main__':
    main()
