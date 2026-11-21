"""Tests for tools/geo_solve.py on the real data (G5). Run: python -I tools/geo_solve_test.py
Each solve is time-limited; geo_solve's watchdog prints [hb] lines and kills a stalled solve."""
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import geo_solve as gs  # noqa: E402
import geo_breakeven as gb  # noqa: E402

T = {'Desc_ModularFrameHeavy_C': 10.0}
TL = 180.0
TIMES = []


def timed(label, **kw):
    res, m, idx, meta = gs.run(D, kw.pop('targets', T), label=label, time_limit=kw.pop('time_limit', TL), **kw)
    TIMES.append((label, round(res['time'], 2), res['status']))
    return res, m, idx, meta


def test_formulas():
    # hand values: 1 km at 33.333 m/s = 30.0 s each way; + 2 x (27.08 + 30) s = 174.16 s = 2.9027 min
    r = gs.rtd_min(1000, 33.333, 27.08, 30)
    assert abs(r - 174.16 / 60) < 1e-3, r
    # 2400 x (1 - 27.08 / 174.16) = 2026.8 (report section 2.2)
    assert abs(gs.platform_cap(2400, r, 27.08) - 2026.8) < 0.2, gs.platform_cap(2400, r, 27.08)
    # wiki Tutorial:Train_throughput, stack 500, RtD 427.08 s: 2247.8 items/min (platform-limited)
    assert abs(gs.platform_cap(2400, 427.08 / 60, 27.08) - 2247.8) < 0.1


def test_oe():
    oe = gs.ore_equivalents(D, ['Desc_IronPlate_C', 'Desc_SteelPipe_C', 'Desc_SteelPlate_C'])
    # report 7.2: plate 1.5 ore, steel pipe 3 ore, steel beam 8 ore (1.0 base recipes)
    assert abs(oe['Desc_IronPlate_C'] - 1.5) < 1e-6 and abs(oe['Desc_SteelPipe_C'] - 3) < 1e-6
    assert abs(oe['Desc_SteelPlate_C'] - 8) < 1e-6, oe


def test_machine_counts_hand_values():
    # game-data.json rates are per minute. 10 HMF/min = 5 Manufacturers (2/min each); 50 Modular Frame/min =
    # 25 Assemblers (2/min); 12 000 Iron Plate/min = 600 Constructors (20/min) and 18 000 ingot/min = 600 Smelters.
    rec = {r['id']: r for r in D.recipes}
    a = gs.per_min(rec['Recipe_ModularFrameHeavy_C'])
    assert a['Desc_ModularFrameHeavy_C'] == 2.0 and a['Desc_IronScrew_C'] == -240.0, a
    assert gs.per_min(rec['Recipe_IronPlate_C'])['Desc_IronPlate_C'] == 20.0
    assert gs.per_min(rec['Recipe_IngotIron_C'])['Desc_IronIngot_C'] == 30.0
    res, *_ = timed('hand values 10 HMF belt-only', rail=False)
    mt = res['summary']['machines_total']
    assert abs(mt['Recipe_ModularFrameHeavy_C'] - 5.0) < 1e-6, mt
    assert abs(mt['Recipe_ModularFrame_C'] - 25.0) < 1e-6, mt
    # machine cost part = sum of counts x building OE (Manufacturer etc.), hand-assembled from the summary
    mach = sum(n * res_meta_mach(r) for r, n in mt.items())
    assert abs(mach - res['summary']['cost_parts']['x']) < 1e-3, (mach, res['summary']['cost_parts'])
    recs = gs.recipe_closure(D, {'Desc_IronPlate_C': 12000.0})
    A = {r['id']: gs.per_min(r) for r in recs}
    items = sorted({k for v in A.values() for k in v})
    m = gs.Model()
    x = {r: m.var(r, 1.0) for r in A}
    e = {k: m.var(k, 0.0) for k in items if k in D.raw}
    for k in items:
        co = {x[r]: A[r][k] for r in A if A[r].get(k)}
        if k in e:
            co[e[k]] = 1
        m.row(k, co, lo=12000.0 if k == 'Desc_IronPlate_C' else 0.0)
    sol = gs.solve_lp(m.c, m.rows, m.lo, m.hi, m.lb, m.ub)
    n = {r: sol['x'][j] for r, j in x.items()}
    assert abs(n['Recipe_IronPlate_C'] - 600) < 1e-6 and abs(n['Recipe_IngotIron_C'] - 600) < 1e-6, n


_MACH = {}


def res_meta_mach(r):
    if not _MACH:
        _, _, meta = gs.build(D, T, rail=False)
        _MACH.update(meta['mach_oe'])
    return _MACH[r]


def test_tier_supply():
    # a pure iron node at tier 8 gives 1200/min (Mk.3 at 250 %) but leaves on one Mk.5 belt: 780
    pure = {'type': 'node', 'item': 'Desc_OreIron_C', 'purity': 'pure'}
    assert gs.node_rate(pure, 8, 780, 600) == 780
    assert gs.node_rate(pure, 8, 1200, 600) == 1200
    assert gs.node_rate(pure, 6, 60, 600) == 60          # tier 6: Mk.2 miner 600/min, Mk.1 belt 60/min
    assert gs.node_rate({'type': 'node', 'item': 'Desc_OreIron_C', 'purity': 'impure'}, 2, 120, 300) == 30
    assert gs.node_rate({'type': 'node', 'item': 'Desc_LiquidOil_C', 'purity': 'pure'}, 5, 780, 300) == 300
    assert gs.node_rate({'type': 'well', 'item': 'Desc_NitrogenGas_C', 'purity': 'pure'}, 6, 780, 600) == 0


def two_site_data(dist_m, n_nodes=12):
    """Synthetic map: iron at A (n pure nodes), demand at B, one belt arc and one rail line of dist_m x 1.3."""
    nodes = [{'id': f'N{i}', 'item': 'Desc_OreIron_C', 'purity': 'pure', 'type': 'node', 'x': 0, 'y': 0, 'z': 0,
              'max_rate': 1200} for i in range(n_nodes)]
    L = round(dist_m * 1.3, 1)

    def site(i, x, nn):
        return {'id': i, 'kind': 'grid', 'x': x, 'y': 0.0, 'supply': {}, 'feed_dist_mean_m': 0.0, 'nodes': nn}
    doc = {'params': {'detour_factor': 1.3},
           'sites': [site('A', 0.0, [n['id'] for n in nodes]), site('B', dist_m, [])],
           'arcs': [{'a': 'A', 'b': 'B', 'length_m': L}],
           'hubs': [{'site': 'B', 'reason': 'base'}, {'site': 'A', 'reason': 'top_output'}]}
    return gs.Data(sites_doc=doc, nodes=nodes), L


def test_two_site_closed_form():
    # closed-form optimum on a two-site map, 5 km straight = 6.5 km route, ore (stack 100), tier 8.
    # Hand numbers: Mk.5 belt 500 x OE(alclad 2.3333) = 1166.67 OE/km; double track 2 x 83 x (3 + 8) = 1826 OE/km;
    # hub F = 2 x 745 + 2000 = 3490; loco 720; car 230 + 2 platforms x 900 = 2030.
    # RtD = (2 x 6500 / 33.333 + 2 x 57.08) / 60 = 8.4027 min -> one car carries 32 x 100 / 8.4027 = 380.8/min.
    #  780/min: belts 1 x 6.5 x 1166.67 = 7583.3 ; rail 6980 + 11869 + 720 + 3 x 2030 = 25659   -> belt
    # 7800/min: belts 10 x 7583.3 = 75833 ; rail 6980 + 11869 + 720 + 21 x 2030 = 62199          -> rail
    d2, L = two_site_data(5000.0)
    assert L == 6500.0
    for Q, mode, transport in ((780.0, 'belt', 7583.333), (7800.0, 'rail', 62199.0)):
        res, m, idx, meta = gs.run(d2, {'Desc_OreIron_C': Q}, label=f'two-site {Q:g}', time_limit=60, gap=1e-9)
        assert res['status'] == 'Optimal', res['status']
        assert abs(meta['F'] - 3490) < 1e-6 and abs(meta['track_oe_km'] - 1826) < 1e-6
        assert abs(meta['belt_oe_km'] - 1166.667) < 1e-2
        assert abs(meta['car_oe'] + 2 * meta['platform_oe'] - 2030) < 1e-6
        belt, rail, cars = gb.haul_costs(meta, Q, 100, L)
        assert abs(min(belt, rail) - transport) < 0.5, (belt, rail, cars)
        assert abs(res['obj'] - (Q * gs.W_RAW + transport)) < 0.5, (res['obj'], Q, transport)
        s = res['summary']
        if mode == 'rail':
            assert s['hubs'] == ['A', 'B'] and s['belt_count'] == 0 and cars == 21, s
            assert s['lines'][0]['dirs']['A->B']['cars'] == 21
        else:
            assert s['hubs'] == [] and s['belt_count'] == 1, s


def test_fluid_pipes():
    # a fluid target exercises pipes and the pipe checks: 1500 m3/min crude oil at the base S0_m1 (600 local)
    res, m, idx, meta = timed('fluid: 1500 crude oil at S0_m1', targets={'Desc_LiquidOil_C': 1500.0})
    assert res['status'] == 'Optimal', res['status']
    s = res['summary']
    piped = sum(fl for p_ in s['pipes'] for fl in p_['flow'].values())
    assert piped >= 900 - 1e-6 or s['rail_flow_items_per_min'] > 0, s
    assert all(sum(p_['flow'].values()) <= p_['n'] * meta['pipe_cap'] + 1e-6 for p_ in s['pipes'])


def test_conservation_and_checker():
    res, m, idx, meta = timed('conservation p free')
    assert res['status'] == 'Optimal'
    gs.check(D, m, idx, meta, res['x'])  # flow balance per site/item, demand, supply, capacities, integrality
    assert abs(meta['check_obj'] - res['obj']) < 1e-3 * max(1.0, abs(res['obj'])), (meta['check_obj'], res['obj'])
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
    # only meaningful when both solves are proven optimal (a time-limited incumbent can be worse with rail)
    a, *_ = timed('rail p free')
    b, *_ = timed('rail disabled (gap 1e-4)', rail=False)
    assert a['status'] == b['status'] == 'Optimal', (a['status'], b['status'])
    assert a['obj'] <= b['obj'] * (1 + 1e-4) + 1e-6, (a['obj'], b['obj'])


def test_deterministic():
    a, *_ = timed('determinism run 1', p=2)
    b, *_ = timed('determinism run 2', p=2)
    assert gs.digest(a['summary']) == gs.digest(b['summary']), 'solutions differ between identical solves'
    assert a['obj'] == b['obj']


def test_stress_far_demand_passes_checker():
    # larger, harder instance: 120 HMF/min demanded at a far hub (Sm3_0); may hit the time limit, and the incumbent
    # must still pass the checker (lines only between open hubs, rail flow only with track and a train).
    res, m, idx, meta = timed('stress 120 HMF at Sm3_0', targets={'Desc_ModularFrameHeavy_C': 120.0}, base='Sm3_0')
    assert res['x'] is not None
    gs.check(D, m, idx, meta, res['x'])
    return res


def test_rail_chosen_when_belts_expensive():
    # exercises the rail constraints on the real map: with belts 20x dearer, 10 HMF/min at the far hub Sm3_0 is
    # served by train; the checker re-verifies car and platform capacity. p = 2 (optimal in about 60 s; p free has a
    # three-hub optimum that needs more than the time limit to prove)
    res, m, idx, meta = timed('belts x20, 10 HMF at Sm3_0, p=2', base='Sm3_0', belt_mult=20.0, p=2)
    assert res['status'] == 'Optimal', res['status']
    gs.check(D, m, idx, meta, res['x'])
    s = res['summary']
    assert s['lines'] and s['rail_flow_items_per_min'] > 1, s['hubs']
    assert len(s['hubs']) >= 2 and all(ln['a'] in s['hubs'] and ln['b'] in s['hubs'] for ln in s['lines'])


if __name__ == '__main__':
    D = gs.Data()
    t0 = time.time()
    only = sys.argv[1:]
    for name, fn in list(globals().items()):
        if name.startswith('test_') and callable(fn) and (not only or name in only):
            t = time.time()
            fn()
            print(f'PASS {name} ({time.time() - t:.1f}s)', flush=True)
    print('solve times:')
    for row in TIMES:
        print('  ', row)
    print(f'all tests passed in {time.time() - t0:.0f}s')
