"""G5: single-period siting MILP for the geographic planner on the real data (report 00-最终报告.md sections 7.1-7.2, 8.4).

Inputs: data/satisfactory-geo/sites.json (grid sites, belt arcs, 12 candidate hubs), data/satisfactory-geo/transport.json
(belt, train and platform parameters), data/game-data.json (1.0 recipes and items).

Model (machines continuous, as the report's siting layer prescribes):
  * x[r,s] >= 0      machines (100 % clock) of recipe r at site s
  * e[k,s] >= 0      raw item k extracted at site s, <= the site's summed node max_rate (sites.json supply)
  * f[k,a] >= 0      flow of item k on directed belt/pipe arc a (both directions of every sites.json arc)
  * b[a], bp[a] in Z+ belts (solids, one tier) and Mk.2 pipes (fluids) per directed arc
  * rail, only between the K candidate hubs, one direct line per hub pair (no shared track segments yet):
      h[s] in {0,1} hub open; u[l] in {0,1} track built; z[l,d] in {0,1} a train runs in direction d;
      c[l,d], cf[l,d] in Z+ solid / fluid cars of that train; g[k,l,d] >= 0 rail flow.
    car capacity  sum_k g/stack_k <= 32/RtD * c            (stacks per minute; return trip empty)
    platform      sum_k g <= feed * (1 - 0.4513/RtD) * c   (feed = two belts of the chosen tier; ports locked 27.08 s)
    fluid         sum_k g <= fluid_car/RtD * cf ,  sum_k g <= 1200 * (1 - 0.4513/RtD) * cf
    linking       c + cf <= Cbar_l * z ;  z <= u ;  u <= h[a], u <= h[b] ;  sum_s h[s] <= p
    RtD = 2 L / v + 2 (dock 27.08 s + accel loss), L = straight distance x detour 1.3.
  * balance  sum_r A[r,k] x[r,s] + e[k,s] + inflow - outflow >= demand[k,s]   (surplus = by-products, free disposal)
Objective, one-time build cost in raw-ore equivalents (OE; one OE = one raw item, water 0), OE of each building
material from a min-raw LP over the 1.0 base recipes in game-data.json:
  machines at their build cost (data1.0.json) + extraction W_RAW per item/min (assumption) plus a feed belt over the
  site's mean feed distance + belts len_km * 500 * OE(belt material) (wiki: about 0.5 per metre) + pipes likewise
  + track len_km * 2 (double track) * 83 * (OE(steel pipe) + OE(steel beam)) + locomotive per running direction
  + car + 2 freight platforms per car + hub fixed cost F = 2 stations + W_WORK_HUB (assumption) per open hub.
Not modelled (report section 7.2 lists them): shared track segments and candidate paths, block headway, pump head,
belt sorting losses, power balance, drones and trucks.

Usage:
  python -I tools/geo_solve.py                         # default target 10 HMF/min, p free, F 1x
  python -I tools/geo_solve.py --target Desc_ModularFrameHeavy_C=10 --p 3 --no-rail --tier 8 --fmult 2
  python -I tools/geo_solve.py --sweep                 # G7: p = 0..12 at F 0.5x/1x/2x, plots and JSON
Each solve is time-limited (--time-limit, default 120 s); a watchdog thread prints "[hb]" lines every 30 s and
kills the process (exit 3, status STALLED) when a solve runs 60 s past its limit.
"""
import argparse
import collections
import hashlib
import json
import math
import os
import sys
import threading
import time

import numpy as np
import highspy
from scipy.sparse import csr_matrix

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GEO = os.path.join(ROOT, 'data', 'satisfactory-geo')
OUTDIR = os.path.join(ROOT, 'docs', 'research', 'satisfactory-geo', '_过程文件')
GAME = os.path.join(ROOT, 'data', 'game-data.json')

W_MACH = 100.0       # OE per machine for buildings without a cost entry below (assumption)
W_RAW = 1.0          # OE per item/min extracted (miner / extractor share; assumption)
W_WORK_HUB = 2000.0  # OE-equivalent player work per hub (station yard, signals; assumption)
# Build costs from greeny/SatisfactoryTools data1.0.json @2192bd15 (fetched 2026-11-21); belts and pipes cost one
# BOM per unit; the wiki's Conveyor_Belts page says about 0.5 units per metre, so 500 units per km (assumed for pipes).
UNITS_PER_KM = 500
PIPE_BOM = [{'Desc_CopperSheet_C': 1}, {'Desc_CopperSheet_C': 2, 'Desc_Plastic_C': 1}]  # Mk.1, Mk.2
BUILDING_COST = {
    'Desc_ConstructorMk1_C': {'Desc_IronPlateReinforced_C': 2, 'Desc_Cable_C': 8},
    'Desc_SmelterMk1_C': {'Desc_IronRod_C': 5, 'Desc_Wire_C': 8},
    'Desc_AssemblerMk1_C': {'Desc_IronPlateReinforced_C': 8, 'Desc_Rotor_C': 4, 'Desc_Cable_C': 10},
    'Desc_FoundryMk1_C': {'Desc_ModularFrame_C': 10, 'Desc_Rotor_C': 10, 'Desc_Cement_C': 20},
    'Desc_ManufacturerMk1_C': {'Desc_Motor_C': 10, 'Desc_ModularFrame_C': 20, 'Desc_Plastic_C': 50, 'Desc_Cable_C': 50},
    'Desc_OilRefinery_C': {'Desc_Motor_C': 10, 'Desc_SteelPlateReinforced_C': 10, 'Desc_SteelPipe_C': 30,
                           'Desc_CopperSheet_C': 20},
    'Desc_Packager_C': {'Desc_SteelPlate_C': 20, 'Desc_Rubber_C': 10, 'Desc_Plastic_C': 10},
    'Desc_Blender_C': {'Desc_Computer_C': 10, 'Desc_ModularFrameHeavy_C': 10, 'Desc_Motor_C': 20,
                       'Desc_AluminumCasing_C': 50},
}
EXCLUDED_BUILDINGS = {'Desc_Converter_C', 'Desc_QuantumEncoder_C', 'Desc_HadronCollider_C'}
BELT_MATERIAL = ['Desc_IronPlate_C', 'Desc_IronPlateReinforced_C', 'Desc_SteelPlate_C', 'Desc_SteelPlateReinforced_C',
                 'Desc_AluminumPlate_C']  # Mk.1..Mk.5 (wiki Conveyor_Belts); Mk.6 Ficsite+Time Crystal not costed
LOCO = {'Desc_ModularFrame_C': 5, 'Desc_Motor_C': 10, 'Desc_SteelPipe_C': 20, 'Desc_Rubber_C': 20, 'Desc_Wire_C': 50}
CAR = {'Desc_ModularFrame_C': 5, 'Desc_SteelPipe_C': 10, 'Desc_SteelPlate_C': 10}
STATION = {'Desc_SteelPlateReinforced_C': 10, 'Desc_Plastic_C': 50, 'Desc_Cement_C': 50, 'Desc_Wire_C': 200}
PLATFORM = {'Desc_Motor_C': 5, 'Desc_SteelPlateReinforced_C': 10, 'Desc_Plastic_C': 25, 'Desc_Cement_C': 50,
            'Desc_Wire_C': 100}
STACK = {}  # filled from a built-in table below (game-data.json has no stack sizes)
# stack sizes: SatisfactoryTools data1.0.json @2192bd15 stackSize (checked 2026-11-21); raw ores 100; default 100
STACK_TABLE = {'Desc_IronPlate_C': 200, 'Desc_IronRod_C': 200, 'Desc_IronScrew_C': 500, 'Desc_Wire_C': 500,
               'Desc_Cable_C': 200, 'Desc_Cement_C': 500, 'Desc_IronPlateReinforced_C': 100,
               'Desc_ModularFrame_C': 50, 'Desc_ModularFrameHeavy_C': 50, 'Desc_SteelPipe_C': 200,
               'Desc_SteelPlate_C': 200, 'Desc_SteelPlateReinforced_C': 100, 'Desc_IronIngot_C': 100,
               'Desc_SteelIngot_C': 100, 'Desc_CopperIngot_C': 100, 'Desc_CopperSheet_C': 200,
               'Desc_Rotor_C': 100, 'Desc_Stator_C': 100, 'Desc_Motor_C': 50, 'Desc_Plastic_C': 200,
               'Desc_Rubber_C': 200, 'Desc_QuartzCrystal_C': 200, 'Desc_Silica_C': 200, 'Desc_GoldIngot_C': 100,
               'Desc_HighSpeedWire_C': 500, 'Desc_CircuitBoard_C': 200, 'Desc_Computer_C': 50}


def load(path):
    with open(path, encoding='utf-8') as fh:
        return json.load(fh)


def tv(node):
    """transport.json leaf value (handles v1_0 / v1_2 variants: 1.0 to match game-data.json)."""
    if 'value' in node:
        return node['value']
    return node['v1_0']['value']


class Data:
    def __init__(self):
        self.game = load(GAME)
        self.sites_doc = load(os.path.join(GEO, 'sites.json'))
        self.tr = load(os.path.join(GEO, 'transport.json'))
        self.items = {i['id']: i for i in self.game['items']}
        self.raw = {r['item'] for r in self.game['resources']}
        self.sites = self.sites_doc['sites']
        self.sid = [s['id'] for s in self.sites]
        self.site = {s['id']: s for s in self.sites}
        self.hubs = sorted(h['site'] for h in self.sites_doc['hubs'])
        self.base = next(h['site'] for h in self.sites_doc['hubs'] if h['reason'] == 'base')
        self.arcs = []  # directed
        for a in self.sites_doc['arcs']:
            self.arcs.append((a['a'], a['b'], a['length_m']))
            self.arcs.append((a['b'], a['a'], a['length_m']))
        self.detour = self.sites_doc['params']['detour_factor']
        self.recipes = [r for r in self.game['recipes']]

    def liquid(self, k):
        return self.items[k]['liquid']

    def stack(self, k):
        return STACK_TABLE.get(k, 100)


def per_min(r):
    m = 60.0 / r['time']
    a = collections.defaultdict(float)
    for k, q in r['in']:
        a[k] -= q * m
    for k, q in r['out']:
        a[k] += q * m
    return dict(a)


def rtd_min(L_m, v_mps, dock_s, acc_s):
    """Round trip in minutes: travel both ways plus, at each end, the dock lock and the accel/brake loss."""
    return (2 * L_m / v_mps + 2 * (dock_s + acc_s)) / 60.0


def platform_cap(feed, rtd, dock_s):
    """Items/min one car can be loaded with at a platform fed at `feed` (ports locked dock_s per stop)."""
    return feed * (1 - (dock_s / 60.0) / rtd)


def solve_lp(c, A_rows, lo, hi, lb, ub, integ=None, time_limit=None, warm=None, gap=1e-4, threads=1):
    """Thin HiGHS wrapper. A_rows: list of {col: coef}. Returns dict with x, obj, bound, gap, status, time."""
    n = len(c)
    data, ri, ci = [], [], []
    for i, row in enumerate(A_rows):
        for j in sorted(row):
            ri.append(i); ci.append(j); data.append(row[j])
    A = csr_matrix((data, (ri, ci)), shape=(len(A_rows), n)).tocsc()
    h = highspy.Highs()
    h.setOptionValue('output_flag', False)
    h.setOptionValue('random_seed', 0)
    h.setOptionValue('threads', threads)
    h.setOptionValue('mip_rel_gap', gap)
    if time_limit:
        h.setOptionValue('time_limit', float(time_limit))
    lp = highspy.HighsLp()
    lp.num_col_ = n
    lp.num_row_ = len(A_rows)
    lp.col_cost_ = np.asarray(c, dtype=float)
    lp.col_lower_ = np.asarray(lb, dtype=float)
    lp.col_upper_ = np.asarray([highspy.kHighsInf if u == math.inf else u for u in ub], dtype=float)
    lp.row_lower_ = np.asarray([-highspy.kHighsInf if v == -math.inf else v for v in lo], dtype=float)
    lp.row_upper_ = np.asarray([highspy.kHighsInf if v == math.inf else v for v in hi], dtype=float)
    lp.a_matrix_.format_ = highspy.MatrixFormat.kColwise
    lp.a_matrix_.start_ = A.indptr
    lp.a_matrix_.index_ = A.indices
    lp.a_matrix_.value_ = A.data
    lp.a_matrix_.num_col_ = n
    lp.a_matrix_.num_row_ = len(A_rows)
    if integ is not None and any(integ):
        lp.integrality_ = [highspy.HighsVarType.kInteger if t else highspy.HighsVarType.kContinuous for t in integ]
    h.passModel(lp)
    if warm is not None:
        sol = highspy.HighsSolution()
        sol.col_value = list(map(float, warm))
        sol.value_valid = True
        h.setSolution(sol)
    t0 = time.time()
    h.run()
    dt = time.time() - t0
    st = h.modelStatusToString(h.getModelStatus())
    info = h.getInfo()
    x = np.array(h.getSolution().col_value) if info.primal_solution_status >= 2 else None
    mip = integ is not None and any(integ)
    return {'x': x, 'obj': info.objective_function_value if x is not None else None,
            'bound': info.mip_dual_bound if mip else info.objective_function_value,
            'gap': info.mip_gap if mip else 0.0, 'status': st, 'time': dt}


def ore_equivalents(d, items):
    """Min raw items (water weight 0) to make one unit of each item, 1.0 base recipes. One LP per item."""
    recs = [r for r in d.recipes if not r['alt'] and r['building'] not in EXCLUDED_BUILDINGS]
    allk = sorted({k for r in recs for k, _ in r['in'] + r['out']} | d.raw)
    kix = {k: i for i, k in enumerate(allk)}
    out = {}
    for item in items:
        c, lb, ub = [], [], []
        rows = [dict() for _ in allk]
        for j, r in enumerate(recs):
            c.append(0.0); lb.append(0.0); ub.append(math.inf)
            for k, q in per_min(r).items():
                rows[kix[k]][j] = q
        for k in sorted(d.raw):
            j = len(c)
            c.append(0.0 if k == 'Desc_Water_C' else 1.0); lb.append(0.0); ub.append(math.inf)
            rows[kix[k]][j] = 1.0
        lo = [1.0 if k == item else 0.0 for k in allk]
        res = solve_lp(c, rows, lo, [math.inf] * len(allk), lb, ub)
        assert item in kix and res['x'] is not None and res['obj'] > 0, (item, res['status'])
        out[item] = res['obj']
    return out


def recipe_closure(d, targets):
    need, seen, chosen = list(targets), set(), []
    while need:
        k = need.pop()
        if k in seen:
            continue
        seen.add(k)
        if k in d.raw:
            continue
        prod = [r for r in d.recipes if not r['alt'] and any(o == k for o, _ in r['out'])
                and not any(o in d.raw for o, _ in r['out'])]
        good = [r for r in prod if r['building'] not in EXCLUDED_BUILDINGS] or prod
        assert good, f'no recipe makes {k}'
        for r in good:
            if r not in chosen:
                chosen.append(r)
                need.extend(i for i, _ in r['in'])
    return sorted(chosen, key=lambda r: r['id'])


class Model:
    """Column/row builder; keeps names so solutions can be read back."""

    def __init__(self):
        self.c, self.lb, self.ub, self.integ, self.names = [], [], [], [], []
        self.rows, self.lo, self.hi, self.rnames = [], [], [], []

    def var(self, name, cost=0.0, lb=0.0, ub=math.inf, integer=False):
        self.c.append(cost); self.lb.append(lb); self.ub.append(ub); self.integ.append(1 if integer else 0)
        self.names.append(name)
        return len(self.c) - 1

    def row(self, name, coefs, lo=-math.inf, hi=math.inf):
        self.rows.append(coefs); self.lo.append(lo); self.hi.append(hi); self.rnames.append(name)
        return len(self.rows) - 1


def build(d, targets, tier=8, rail=True, p=None, fmult=1.0, base=None, acc_s=30.0, water_anywhere=True,
          belt_tier=None, oe=None, belt_mult=1.0):
    base = base or d.base
    recs = recipe_closure(d, targets)
    A = {r['id']: per_min(r) for r in recs}
    items = sorted({k for a in A.values() for k in a} | set(targets))
    solids = [k for k in items if not d.liquid(k)]
    fluids = [k for k in items if d.liquid(k)]
    tr = d.tr
    belt_caps = tv(tr['belts']['tiers'])
    belt_unlock = tv(tr['belts']['unlock_tier'])
    mats = sorted(set(BELT_MATERIAL) | set(LOCO) | set(CAR) | set(STATION) | set(PLATFORM)
                  | {'Desc_SteelPipe_C', 'Desc_SteelPlate_C'} | {k for b_ in PIPE_BOM for k in b_}
                  | {k for b_ in BUILDING_COST.values() for k in b_})
    oe = oe or ore_equivalents(d, mats)
    if belt_tier is None:  # cheapest OE per unit capacity among unlocked tiers (Mk.6 not costed)
        cand = [i for i in range(5) if belt_unlock[i] <= tier]
        belt_tier = min(cand, key=lambda i: (UNITS_PER_KM * oe[BELT_MATERIAL[i]] / belt_caps[i], -i))
    cap = belt_caps[belt_tier]
    belt_oe_km = belt_mult * UNITS_PER_KM * oe[BELT_MATERIAL[belt_tier]]  # belt_mult: test / sensitivity knob
    pipe_ix = 1 if tv(tr['pipes']['unlock_tier'])[1] <= tier else 0
    pipe_cap = tv(tr['pipes']['tiers'])[pipe_ix]
    rail = rail and tv(tr['train']['unlock_tier']) <= tier
    km83 = tv(tr['rail']['track_cost_per_km'])
    track_oe_km = tv(tr['rail']['double_track_factor']) * (km83['steel_pipe'] * oe['Desc_SteelPipe_C']
                                                          + km83['steel_beam'] * oe['Desc_SteelPlate_C'])
    cost_of = lambda bom: sum(q * oe[k] for k, q in bom.items())
    loco_oe, car_oe, plat_oe, station_oe = cost_of(LOCO), cost_of(CAR), cost_of(PLATFORM), cost_of(STATION)
    pipe_oe_km = UNITS_PER_KM * cost_of(PIPE_BOM[pipe_ix])
    mach_oe = {r['id']: cost_of(BUILDING_COST[r['building']]) if r['building'] in BUILDING_COST else W_MACH
               for r in recs}
    F = fmult * (2 * station_oe + W_WORK_HUB)
    v = tv(tr['train']['speed_mps'])
    dock = tv(tr['train']['platform_dock_lock_s'])
    slots = tv(tr['train']['car_slots'])
    fluid_car = tv(tr['train']['car_fluid_m3'])
    feed = 2 * cap
    m = Model()
    meta = {'base': base, 'targets': targets, 'tier': tier, 'belt_tier': belt_tier + 1, 'belt_cap': cap,
            'belt_oe_km': belt_oe_km, 'pipe_cap': pipe_cap, 'pipe_oe_km': pipe_oe_km, 'mach_oe': mach_oe, 'track_oe_km': track_oe_km, 'loco_oe': loco_oe,
            'car_oe': car_oe, 'platform_oe': plat_oe, 'station_oe': station_oe, 'F': F, 'fmult': fmult,
            'rail': rail, 'p': p, 'acc_s': acc_s, 'recipes': [r['id'] for r in recs], 'items': items,
            'oe': dict(sorted(oe.items()))}  # unrounded: the sweep reuses it as cost input
    x = {(r['id'], s): m.var(f'x|{r["id"]}|{s}', mach_oe[r['id']]) for s in d.sid for r in recs}
    e = {}
    for s in d.sid:
        st = d.site[s]
        for k in items:
            if k not in d.raw:
                continue
            sup = st['supply'].get(k, 0)
            if k == 'Desc_Water_C' and water_anywhere:
                sup = math.inf
            if sup <= 0:
                continue
            fd_km = st['feed_dist_mean_m'] / 1000.0
            feedc = fd_km * (pipe_oe_km / pipe_cap if d.liquid(k) else belt_oe_km / cap)
            e[(k, s)] = m.var(f'e|{k}|{s}', W_RAW + feedc, 0, sup)
    f, b, bp = {}, {}, {}
    for (sa, sb, L) in d.arcs:
        km = L / 1000.0
        if solids:
            b[(sa, sb)] = m.var(f'b|{sa}|{sb}', km * belt_oe_km, 0, 50, True)
        if fluids:
            bp[(sa, sb)] = m.var(f'bp|{sa}|{sb}', km * pipe_oe_km, 0, 50, True)
        for k in items:
            f[(k, sa, sb)] = m.var(f'f|{k}|{sa}|{sb}')
    # aggregate bound on stacks moved (for tight car bounds): gross production of all items, plain LP
    hvar, u, z, cs, cfl, g, lines = {}, {}, {}, {}, {}, {}, []
    if rail:
        gross = aggregate_gross(d, recs, A, targets, items)
        stacks_ub = sum(q / d.stack(k) for k, q in gross.items() if not d.liquid(k))
        fluid_ub = sum(q for k, q in gross.items() if d.liquid(k))
        for s in d.hubs:
            hvar[s] = m.var(f'h|{s}', F, 0, 1, True)
        for i, sa in enumerate(d.hubs):
            for sb in d.hubs[i + 1:]:
                A_, B_ = d.site[sa], d.site[sb]
                L = math.hypot(A_['x'] - B_['x'], A_['y'] - B_['y']) * d.detour
                rtd = rtd_min(L, v, dock, acc_s)
                cbar = math.ceil(stacks_ub * rtd / slots) + 1
                cbar_f = math.ceil(fluid_ub * rtd / fluid_car) + 1 if fluids else 0
                l = (sa, sb)
                lines.append({'a': sa, 'b': sb, 'length_m': round(L, 1), 'rtd_min': round(rtd, 4),
                              'cbar': cbar, 'cbar_fluid': cbar_f})
                u[l] = m.var(f'u|{sa}|{sb}', L / 1000.0 * track_oe_km, 0, 1, True)
                m.row(f'uha|{sa}|{sb}', {u[l]: 1, hvar[sa]: -1}, hi=0)
                m.row(f'uhb|{sa}|{sb}', {u[l]: 1, hvar[sb]: -1}, hi=0)
                for (o, t) in ((sa, sb), (sb, sa)):
                    key = (o, t)
                    z[key] = m.var(f'z|{o}|{t}', loco_oe, 0, 1, True)
                    m.row(f'zu|{o}|{t}', {z[key]: 1, u[l]: -1}, hi=0)
                    link = {z[key]: -(cbar + cbar_f)}
                    if solids:
                        cs[key] = m.var(f'c|{o}|{t}', car_oe + 2 * plat_oe, 0, cbar, True)
                        link[cs[key]] = 1
                    if fluids:
                        cfl[key] = m.var(f'cf|{o}|{t}', car_oe + 2 * plat_oe, 0, cbar_f, True)
                        link[cfl[key]] = 1
                    m.row(f'cz|{o}|{t}', link, hi=0)
                    for k in items:
                        g[(k, o, t)] = m.var(f'g|{k}|{o}|{t}')
                    if solids:
                        m.row(f'car|{o}|{t}', {**{g[(k, o, t)]: 1.0 / d.stack(k) for k in solids},
                                               cs[key]: -slots / rtd}, hi=0)
                        m.row(f'plat|{o}|{t}', {**{g[(k, o, t)]: 1.0 for k in solids},
                                                cs[key]: -platform_cap(feed, rtd, dock)}, hi=0)
                    if fluids:
                        m.row(f'fcar|{o}|{t}', {**{g[(k, o, t)]: 1.0 for k in fluids},
                                                cfl[key]: -fluid_car / rtd}, hi=0)
                        m.row(f'fplat|{o}|{t}', {**{g[(k, o, t)]: 1.0 for k in fluids},
                                                 cfl[key]: -platform_cap(1200, rtd, dock)}, hi=0)
        if p is not None:
            m.row('p', {hvar[s]: 1 for s in d.hubs}, hi=p)
    # belt / pipe capacity
    for (sa, sb, _) in d.arcs:
        if solids:
            m.row(f'belt|{sa}|{sb}', {**{f[(k, sa, sb)]: 1 for k in solids}, b[(sa, sb)]: -cap}, hi=0)
        if fluids:
            m.row(f'pipe|{sa}|{sb}', {**{f[(k, sa, sb)]: 1 for k in fluids}, bp[(sa, sb)]: -pipe_cap}, hi=0)
    # balance
    inc, outg = collections.defaultdict(list), collections.defaultdict(list)
    for (k, sa, sb), j in list(f.items()) + list(g.items()):
        outg[(k, sa)].append(j); inc[(k, sb)].append(j)
    for s in d.sid:
        for k in items:
            co = collections.defaultdict(float)
            for r in recs:
                q = A[r['id']].get(k, 0.0)
                if q:
                    co[x[(r['id'], s)]] += q
            if (k, s) in e:
                co[e[(k, s)]] += 1
            for j in inc[(k, s)]:
                co[j] += 1
            for j in outg[(k, s)]:
                co[j] -= 1
            dem = targets.get(k, 0.0) if s == base else 0.0
            if not co and dem == 0:
                continue
            m.row(f'bal|{k}|{s}', dict(co), lo=dem)
    meta['lines'] = lines
    meta.update({'slots': slots, 'feed': feed, 'dock_s': dock, 'fluid_car': fluid_car})
    idx = {'x': x, 'e': e, 'f': f, 'b': b, 'bp': bp, 'h': hvar, 'u': u, 'z': z, 'c': cs, 'cf': cfl, 'g': g,
           'A': A, 'solids': solids, 'fluids': fluids}
    return m, idx, meta


def aggregate_gross(d, recs, A, targets, items):
    """Location-free LP: min machines + raw; returns gross production per item (upper bound for flows)."""
    m = Model()
    x = {r['id']: m.var(r['id'], W_MACH) for r in recs}
    e = {k: m.var(k, W_RAW) for k in items if k in d.raw}
    for k in items:
        co = {x[r]: A[r][k] for r in x if A[r].get(k)}
        if k in e:
            co[e[k]] = 1
        m.row(k, co, lo=targets.get(k, 0.0))
    res = solve_lp(m.c, m.rows, m.lo, m.hi, m.lb, m.ub)
    assert res['x'] is not None, res['status']
    gross = collections.defaultdict(float)
    for r, j in x.items():
        for k, q in A[r].items():
            if q > 0:
                gross[k] += q * res['x'][j]
    for k, j in e.items():
        gross[k] += res['x'][j]
    return gross


def check(d, m, idx, meta, xs, p=None, tol=1e-5):
    """Independent feasibility check of a solution vector; raises AssertionError with the first violation."""
    x = lambda j: xs[j]
    for j, (lo, hi, t) in enumerate(zip(m.lb, m.ub, m.integ)):
        assert xs[j] >= lo - tol and xs[j] <= hi + tol, (m.names[j], xs[j])
        if t:
            assert abs(xs[j] - round(xs[j])) <= 1e-6, ('integrality', m.names[j], xs[j])
    A, base, targets = idx['A'], meta['base'], meta['targets']
    # flow conservation per site and item, recomputed from the named variables
    net = collections.defaultdict(float)
    for (r, s), j in idx['x'].items():
        for k, q in A[r].items():
            net[(k, s)] += q * x(j)
    for (k, s), j in idx['e'].items():
        net[(k, s)] += x(j)
    for key in ('f', 'g'):
        for (k, sa, sb), j in idx[key].items():
            net[(k, sa)] -= x(j); net[(k, sb)] += x(j)
    for s in d.sid:
        for k in meta['items']:
            need = targets.get(k, 0.0) if s == base else 0.0
            assert net[(k, s)] >= need - tol * max(1.0, need) - 1e-6, ('balance', k, s, net[(k, s)], need)
    for k, q in targets.items():
        assert net[(k, base)] >= q - 1e-6, ('demand', k)
    for (k, s), j in idx['e'].items():
        sup = d.site[s]['supply'].get(k, math.inf if k == 'Desc_Water_C' else 0)
        assert x(j) <= sup + tol, ('supply', k, s)
    cap = meta['belt_cap']
    for (sa, sb), j in idx['b'].items():
        tot = sum(x(idx['f'][(k, sa, sb)]) for k in idx['solids'])
        assert tot <= cap * round(x(j)) + 1e-4, ('belt cap', sa, sb, tot, x(j))
    if idx['h']:
        opened = {s for s, j in idx['h'].items() if x(j) > 0.5}
        if p is not None:
            assert len(opened) <= p, ('p', len(opened), p)
        for (sa, sb), j in idx['u'].items():
            if x(j) > 0.5:
                assert sa in opened and sb in opened, ('line without hubs', sa, sb)
        for (o, t), j in idx['z'].items():
            l = (o, t) if (o, t) in idx['u'] else (t, o)
            used = sum(x(idx['g'][(k, o, t)]) for k in meta['items'])
            if used > 1e-6:
                assert x(idx['u'][l]) > 0.5 and x(j) > 0.5, ('rail flow without track/train', o, t)
        rtd = {(ln['a'], ln['b']): ln['rtd_min'] for ln in meta['lines']}
        for (o, t), j in idx['c'].items():
            r = rtd.get((o, t)) or rtd[(t, o)]
            stk = sum(x(idx['g'][(k, o, t)]) / d.stack(k) for k in idx['solids'])
            its = sum(x(idx['g'][(k, o, t)]) for k in idx['solids'])
            c = round(x(j))
            assert stk <= meta['slots'] / r * c * (1 + 1e-3) + 1e-6, ('car capacity', o, t, stk, c)
            assert its <= platform_cap(meta['feed'], r, meta['dock_s']) * c * (1 + 1e-3) + 1e-6, ('platform', o, t)


def summarize(d, m, idx, meta, xs):
    x = lambda j: float(xs[j])
    r6 = lambda v_: round(v_, 6)
    parts = collections.defaultdict(float)
    for j, nm in enumerate(m.names):
        parts[nm.split('|')[0]] += m.c[j] * xs[j]
    out = {'cost_parts': {k: r6(v_) for k, v_ in sorted(parts.items())}}
    out['hubs'] = sorted(s for s, j in idx['h'].items() if x(j) > 0.5)
    out['lines'] = []
    for (sa, sb), j in sorted(idx['u'].items()):
        if x(j) > 0.5:
            dirs = {}
            for (o, t) in ((sa, sb), (sb, sa)):
                fl = {k: r6(x(idx['g'][(k, o, t)])) for k in meta['items'] if x(idx['g'][(k, o, t)]) > 1e-6}
                dirs[f'{o}->{t}'] = {'cars': round(x(idx['c'][(o, t)])) if (o, t) in idx['c'] else 0,
                                     'fluid_cars': round(x(idx['cf'][(o, t)])) if (o, t) in idx['cf'] else 0,
                                     'flow': fl}
            out['lines'].append({'a': sa, 'b': sb, 'dirs': dirs})
    out['belts'] = []
    for (sa, sb), j in sorted(idx['b'].items()):
        if x(j) > 0.5:
            fl = {k: r6(x(idx['f'][(k, sa, sb)])) for k in idx['solids'] if x(idx['f'][(k, sa, sb)]) > 1e-6}
            out['belts'].append({'a': sa, 'b': sb, 'n': round(x(j)), 'flow': fl})
    out['pipes'] = []
    for (sa, sb), j in sorted(idx['bp'].items()):
        if x(j) > 0.5:
            fl = {k: r6(x(idx['f'][(k, sa, sb)])) for k in idx['fluids'] if x(idx['f'][(k, sa, sb)]) > 1e-6}
            out['pipes'].append({'a': sa, 'b': sb, 'n': round(x(j)), 'flow': fl})
    site_rec = collections.defaultdict(dict)
    for (r, s), j in sorted(idx['x'].items()):
        if x(j) > 1e-6:
            site_rec[s][r] = r6(x(j))
    out['machines'] = dict(sorted(site_rec.items()))
    ext = collections.defaultdict(dict)
    for (k, s), j in sorted(idx['e'].items()):
        if x(j) > 1e-6:
            ext[s][k] = r6(x(j))
    out['extraction'] = dict(sorted(ext.items()))
    out['rail_flow_items_per_min'] = r6(sum(x(j) for j in idx['g'].values()))
    out['belt_count'] = sum(b_['n'] for b_ in out['belts'])
    return out


class Watchdog:
    """Prints [hb] lines while a solve runs; exits the process (STALLED) if it overruns its limit by `grace` s."""

    def __init__(self, status_path=None, every=30.0, grace=60.0):
        self.every, self.grace, self.status_path = every, grace, status_path
        self.label, self.t0, self.limit = None, None, None
        self.lock = threading.Lock()
        threading.Thread(target=self._run, daemon=True).start()

    def start(self, label, limit):
        with self.lock:
            self.label, self.t0, self.limit = label, time.time(), limit

    def stop(self):
        with self.lock:
            self.label = None

    def _run(self):
        while True:
            time.sleep(self.every)
            with self.lock:
                if self.label is None:
                    continue
                el = time.time() - self.t0
                print(f'[hb] {time.strftime("%H:%M:%S")} solving {self.label} {el:.0f}s', flush=True)
                if self.limit and el > self.limit + self.grace:
                    msg = f'STALLED {self.label} after {el:.0f}s (limit {self.limit}s)'
                    print(msg, flush=True)
                    if self.status_path:
                        with open(self.status_path, 'w', encoding='utf-8') as fh:
                            fh.write(msg + '\n')
                    os._exit(3)


WD = None


def run(d, targets, label='solve', time_limit=120.0, gap=1e-4, warm=None, oe=None, **kw):
    global WD
    if WD is None:
        WD = Watchdog()
    t0 = time.time()
    m, idx, meta = build(d, targets, oe=oe, **kw)
    tb = time.time() - t0
    w = None
    if warm is not None:  # map a previous solution by variable name (models of one sweep share names)
        pos = {nm: j for j, nm in enumerate(m.names)}
        w = np.zeros(len(m.c))
        for nm, val in warm.items():
            if nm in pos:
                w[pos[nm]] = val
    WD.start(label, time_limit)
    res = solve_lp(m.c, m.rows, m.lo, m.hi, m.lb, m.ub, m.integ, time_limit=time_limit, warm=w, gap=gap)
    WD.stop()
    res['build_s'] = tb
    res['n_vars'] = len(m.c)
    res['n_int'] = int(sum(m.integ))
    res['n_rows'] = len(m.rows)
    if res['x'] is not None:
        check(d, m, idx, meta, res['x'], p=kw.get('p'))
        res['summary'] = summarize(d, m, idx, meta, res['x'])
        res['named'] = {nm: float(v_) for nm, v_ in zip(m.names, res['x']) if abs(v_) > 1e-9}
    print(f'{label}: status={res["status"]} obj={res["obj"]} bound={res["bound"]} gap={res["gap"]:.2e} '
          f'time={res["time"]:.1f}s vars={res["n_vars"]} int={res["n_int"]} rows={res["n_rows"]}', flush=True)
    return res, m, idx, meta


def parse_targets(specs):
    t = {}
    for s in specs or ['Desc_ModularFrameHeavy_C=10']:
        k, q = s.split('=')
        t[k] = float(q)
    return t


def digest(obj):
    return hashlib.md5(json.dumps(obj, sort_keys=True).encode()).hexdigest()


def sweep(d, targets, tier, time_limit, out_json, fmults=(1.0, 0.5, 2.0), pmax=12, base=None, belt_mult=1.0):
    mats_oe = None
    rows = []
    best_sol = None
    t_all = time.time()
    _, _, meta0 = build(d, targets, tier=tier, rail=False, base=base, belt_mult=belt_mult)
    mats_oe = {k: v_ for k, v_ in meta0['oe'].items()}
    for fm in fmults:
        env, warm = math.inf, None
        for p in range(0, pmax + 1):
            res, m, idx, meta = run(d, targets, label=f'F{fm}x p={p}', time_limit=time_limit, warm=warm,
                                    oe=mats_oe, tier=tier, rail=True, p=p, fmult=fm, base=base,
                                    belt_mult=belt_mult)
            obj = res['obj'] if res['obj'] is not None else math.inf
            env = min(env, obj)
            s = res.get('summary', {})
            rows.append({'fmult': fm, 'F': meta['F'], 'p': p, 'obj': obj, 'bound': res['bound'], 'gap': res['gap'],
                         'status': res['status'], 'time_s': round(res['time'], 2), 'envelope': env,
                         'hubs': s.get('hubs'), 'n_lines': len(s.get('lines', [])),
                         'rail_flow': s.get('rail_flow_items_per_min'), 'belts': s.get('belt_count')})
            if res['obj'] is not None and (warm is None or obj <= env + 1e-9):
                warm = res['named']
            if fm == 1.0 and (best_sol is None or obj < best_sol['obj'] - 1e-6):
                best_sol = {'obj': obj, 'p': p, 'summary': s, 'meta': {k: v_ for k, v_ in meta.items() if k != 'lines'}}
            with open(out_json, 'w', encoding='utf-8') as fh:  # checkpoint after every solve
                json.dump({'targets': targets, 'tier': tier, 'base': meta['base'], 'rows': rows}, fh, indent=1)
    print(f'sweep total {time.time() - t_all:.0f}s', flush=True)
    return rows, best_sol


def plot_sweep(rows, png, what=''):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig, ax = plt.subplots(figsize=(7, 4.2), dpi=130)
    for fm, col in ((0.5, '#2a7fbf'), (1.0, '#222222'), (2.0, '#c0392b')):
        rs = [r for r in rows if r['fmult'] == fm]
        if not rs:
            continue
        ps = [r['p'] for r in rs]
        env = [r['envelope'] for r in rs]
        ax.plot(ps, env, '-o', color=col, ms=4, label=f'F {fm}x (F={rs[0]["F"]:.0f} OE)')
        lo = [min(r['bound'], r['envelope']) for r in rs]
        ax.fill_between(ps, lo, env, color=col, alpha=0.12, lw=0)
        for r in rs:
            if r['status'] != 'Optimal':
                ax.plot(r['p'], r['envelope'], 'x', color=col, ms=7)
    vals = [r['envelope'] for r in rows if math.isfinite(r['envelope'])]
    lo_all = min(min(r['bound'], r['envelope']) for r in rows)
    mid, span = (max(vals) + lo_all) / 2, max(max(vals) - lo_all, 0.02 * max(vals))  # at least +-1 % visible
    ax.set_ylim(mid - 0.6 * span, mid + 0.6 * span)
    ax.ticklabel_format(axis='y', useOffset=False, style='plain')
    ax.set_xlabel('max open hubs p')
    ax.set_ylabel('total build cost, ore equivalents (running min)')
    ax.set_title(f'{what}\ncost(p) envelope; band = HiGHS lower bound, x = not proven optimal', fontsize=9)
    ax.grid(alpha=0.3)
    ax.legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(png)
    plt.close(fig)


def plot_map(d, sol, png, title, sol_base=None):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    nodes = load(os.path.join(GEO, 'nodes.json'))['nodes']
    fig, ax = plt.subplots(figsize=(8, 8), dpi=130)
    used = {k for ex in sol['extraction'].values() for k in ex}
    cmap = plt.get_cmap('tab20')
    kinds = sorted({n['item'] for n in nodes})
    for i, k in enumerate(kinds):
        pts = [n for n in nodes if n['item'] == k]
        ax.scatter([n['x'] for n in pts], [-n['y'] for n in pts], s=6 if k in used else 2,
                   color=cmap(i % 20), alpha=0.8 if k in used else 0.25,
                   label=k.replace('Desc_', '').replace('_C', '') if k in used else None)
    xy = {s['id']: (s['x'], -s['y']) for s in d.sites}
    ax.scatter([v_[0] for v_ in xy.values()], [v_[1] for v_ in xy.values()], s=10, c='#999', marker='s')
    for a in d.sites_doc['arcs']:
        pa, pb = xy[a['a']], xy[a['b']]
        ax.plot([pa[0], pb[0]], [pa[1], pb[1]], color='#ddd', lw=0.5, zorder=0)
    for bl in sol['belts']:
        pa, pb = xy[bl['a']], xy[bl['b']]
        ax.annotate('', xy=pb, xytext=pa, arrowprops=dict(arrowstyle='->', color='#1f6f3f',
                                                            lw=0.8 + 1.2 * bl['n'], alpha=0.8))
    for ln in sol['lines']:
        pa, pb = xy[ln['a']], xy[ln['b']]
        ax.plot([pa[0], pb[0]], [pa[1], pb[1]], color='#b03060', lw=3, ls='--')
    for s in d.hubs:
        ax.scatter(*xy[s], s=90, facecolors='none', edgecolors='#b03060' if s in sol['hubs'] else '#bbb', lw=1.5)
    for s, recs in sol['machines'].items():
        n = sum(recs.values())
        ax.scatter(*xy[s], s=20 + 6 * n, c='#f39c12', edgecolors='k', lw=0.5, zorder=5)
        ax.annotate(f'{n:.1f}', xy[s], fontsize=6, xytext=(4, 4), textcoords='offset points')
    ax.scatter(*xy[sol_base], s=200, marker='*', c='gold', edgecolors='k', zorder=6)
    ax.set_aspect('equal')
    ax.set_title(title, fontsize=9)
    ax.set_xlabel('x (m)')
    ax.set_ylabel('-y (m), north up')
    ax.legend(fontsize=6, loc='lower left', markerscale=2)
    fig.tight_layout()
    fig.savefig(png)
    plt.close(fig)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--target', action='append', help='ITEM=rate per min at the base site; repeatable')
    ap.add_argument('--tier', type=int, default=8, help='milestone tier (8 = phase 4: Mk.5 belts, rail)')
    ap.add_argument('--p', type=int, default=None)
    ap.add_argument('--no-rail', action='store_true')
    ap.add_argument('--fmult', type=float, default=1.0)
    ap.add_argument('--time-limit', type=float, default=120.0)
    ap.add_argument('--sweep', action='store_true')
    ap.add_argument('--out', default=None, help='solution JSON path')
    ap.add_argument('--base', default=None, help='demand site id (default: the base hub in sites.json)')
    ap.add_argument('--tag', default='', help='suffix for sweep output files')
    ap.add_argument('--pmax', type=int, default=12)
    ap.add_argument('--replot', action='store_true', help='redraw p_sweep{tag}.png from p_sweep{tag}.json')
    ap.add_argument('--belt-mult', type=float, default=1.0, help='multiply belt cost (sensitivity / illustration)')
    a = ap.parse_args()
    d = Data()
    targets = parse_targets(a.target)
    if a.replot:
        doc = load(os.path.join(OUTDIR, f'p_sweep{a.tag}.json'))
        what = ', '.join(f'{q:g}/min {k.replace("Desc_", "").replace("_C", "")}' for k, q in doc['targets'].items())
        plot_sweep(doc['rows'], os.path.join(OUTDIR, f'p_sweep{a.tag}.png'), f'{what} at {doc["base"]}'
                   + (f', belts x{a.belt_mult:g}' if a.belt_mult != 1 else ''))
        return
    if a.sweep:
        tag = a.tag
        out_json = os.path.join(OUTDIR, f'p_sweep{tag}.json')
        rows, best = sweep(d, targets, a.tier, a.time_limit, out_json, base=a.base, pmax=a.pmax,
                           belt_mult=a.belt_mult)
        plot_sweep(rows, os.path.join(OUTDIR, f'p_sweep{tag}.png'),
                   ', '.join(f'{q:g}/min {k.replace("Desc_", "").replace("_C", "")}' for k, q in targets.items())
                   + f' at {best["meta"]["base"]}' + (f', belts x{a.belt_mult:g}' if a.belt_mult != 1 else ''))
        with open(os.path.join(OUTDIR, f'solution_best{tag}.json'), 'w', encoding='utf-8') as fh:
            json.dump(best, fh, indent=1, sort_keys=True)
        plot_map(d, best['summary'], os.path.join(OUTDIR, f'solution_map{tag}.png'), sol_base=best['meta']['base'],
                 title=
                 f'best solution F 1x, p={best["p"]}, cost {best["obj"]:.0f} OE; '
                 f'{len(best["summary"]["hubs"])} hubs, {len(best["summary"]["lines"])} rail lines')
        for r in rows:
            print(json.dumps({k: r[k] for k in ('fmult', 'p', 'obj', 'envelope', 'gap', 'time_s', 'hubs')}))
        return
    res, m, idx, meta = run(d, targets, time_limit=a.time_limit, tier=a.tier, rail=not a.no_rail, p=a.p,
                            fmult=a.fmult, base=a.base, belt_mult=a.belt_mult)
    if res['x'] is None:
        sys.exit(f'no solution: {res["status"]}')
    s = res['summary']
    print(json.dumps({'meta': {k: meta[k] for k in ('base', 'belt_tier', 'belt_cap', 'belt_oe_km', 'track_oe_km',
                                                  'F', 'car_oe', 'platform_oe', 'loco_oe')},
                      'cost_parts': s['cost_parts'], 'hubs': s['hubs'], 'lines': len(s['lines']),
                      'belts': s['belt_count'], 'rail_flow': s['rail_flow_items_per_min'],
                      'digest': digest(s)}, indent=1))
    if a.out:
        with open(a.out, 'w', encoding='utf-8') as fh:
            json.dump({'obj': res['obj'], 'summary': s, 'meta': {k: v_ for k, v_ in meta.items()}}, fh, indent=1,
                      sort_keys=True)


if __name__ == '__main__':
    main()
