"""Tests for tools/geo_solve.py on the real data (G5). Run: python -I tools/geo_solve_test.py
Each solve is time-limited; geo_solve's watchdog prints [hb] lines and kills a stalled solve."""
import math
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import geo_solve as gs  # noqa: E402

T = {'Desc_ModularFrameHeavy_C': 10.0}
TL = 180.0
TIMES = []


def timed(label, **kw):
    res, m, idx, meta = gs.run(D, kw.pop('targets', T), label=label, time_limit=TL, **kw)
    TIMES.append((label, round(res['time'], 2), res['status']))
    return res, m, idx, meta


def test_formulas():
    # report section 7.2 checks: 1 km, t_acc 30 s -> RtD 2.903 min, one car at 2400/min feed -> 2027 items/min
    r = gs.rtd_min(1000, 33.333, 27.08, 30)
    assert abs(r - 2.903) < 1e-3, r
    assert abs(gs.platform_cap(2400, r, 27.08) - 2027) < 1, gs.platform_cap(2400, r, 27.08)
    # car limited (500/stack, RtD 427.08 s) equals the platform limit 2247.8 (wiki table)
    r2 = 427.08 / 60
    assert abs(32 * 500 / r2 - 2247.8) < 0.1 and abs(gs.platform_cap(2400, r2, 27.08) - 2247.8) < 0.1


def test_oe():
    oe = gs.ore_equivalents(D, ['Desc_IronPlate_C', 'Desc_SteelPipe_C', 'Desc_SteelPlate_C'])
    # report 7.2: plate 1.5 ore, steel pipe 3 ore, steel beam 8 ore (1.0 base recipes)
    assert abs(oe['Desc_IronPlate_C'] - 1.5) < 1e-6 and abs(oe['Desc_SteelPipe_C'] - 3) < 1e-6
    assert abs(oe['Desc_SteelPlate_C'] - 8) < 1e-6, oe


def test_conservation_and_checker():
    res, m, idx, meta = timed('conservation p free')
    assert res['status'] == 'Optimal'
    gs.check(D, m, idx, meta, res['x'])  # flow balance per site/item, demand, supply, capacities, integrality
    # the checker must catch a broken solution: drop all extraction at one used site
    bad = res['x'].copy()
    used = [j for (k, s), j in idx['e'].items() if bad[j] > 1]
    bad[used[0]] = 0.0
    try:
        gs.check(D, m, idx, meta, bad)
    except AssertionError as exc:
        assert exc.args[0][0] == 'balance', exc
    else:
        raise AssertionError('checker missed a balance violation')
    return res


def test_belt_only_equals_rail_disabled():
    a, *_ = timed('p=0 (rail vars present)', p=0, gap=1e-6)
    b, *_ = timed('rail disabled', rail=False, gap=1e-6)
    assert a['status'] == b['status'] == 'Optimal', (a['status'], b['status'])
    assert abs(a['obj'] - b['obj']) <= 1e-6 * abs(b['obj']) + 1e-6, (a['obj'], b['obj'])
    assert a['summary']['hubs'] == [] and a['summary']['rail_flow_items_per_min'] == 0
    assert a['n_vars'] > b['n_vars']
    return a, b


def test_rail_free_not_worse():
    a, *_ = timed('rail p free')
    b, *_ = timed('rail disabled (gap 1e-4)', rail=False)
    assert a['obj'] <= b['obj'] * (1 + 1e-4) + 1e-6, (a['obj'], b['obj'])


def test_deterministic():
    a, *_ = timed('determinism run 1', p=2)
    b, *_ = timed('determinism run 2', p=2)
    assert gs.digest(a['summary']) == gs.digest(b['summary']), 'solutions differ between identical solves'
    assert a['obj'] == b['obj']


def test_stress_far_demand_passes_checker():
    # larger, harder instance: 120 HMF/min demanded at a far hub (Sm3_0); hits the time limit (gap ~6 %), and the
    # incumbent must still pass the checker (lines only between open hubs, rail flow only with track and a train).
    # Not asserting that rail wins.
    far = 'Sm3_0'
    res, m, idx, meta = timed('stress 120 HMF at Sm3_0', targets={'Desc_ModularFrameHeavy_C': 120.0}, base=far)
    assert res['x'] is not None
    gs.check(D, m, idx, meta, res['x'])
    return res


def test_rail_chosen_when_belts_expensive():
    # exercises the rail constraints for real: with belts 20x dearer, 10 HMF/min at the far hub Sm3_0 is served by
    # train (optimal in ~40 s: hubs Sm2_m1 + Sm3_0, no belts); the checker re-verifies car and platform capacity
    res, m, idx, meta = timed('belts x20, 10 HMF at Sm3_0', base='Sm3_0', belt_mult=20.0)
    assert res['status'] == 'Optimal', res['status']
    gs.check(D, m, idx, meta, res['x'])
    s = res['summary']
    assert s['lines'] and s['rail_flow_items_per_min'] > 1, s['hubs']
    assert len(s['hubs']) >= 2 and all(ln['a'] in s['hubs'] and ln['b'] in s['hubs'] for ln in s['lines'])


if __name__ == '__main__':
    D = gs.Data()
    t0 = time.time()
    for name, fn in list(globals().items()):
        if name.startswith('test_') and callable(fn):
            t = time.time()
            fn()
            print(f'PASS {name} ({time.time() - t:.1f}s)', flush=True)
    print('solve times:')
    for row in TIMES:
        print('  ', row)
    print(f'all tests passed in {time.time() - t0:.0f}s')
