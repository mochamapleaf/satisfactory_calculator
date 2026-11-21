"""Toy geographic MILP on the real 1.0 node map (HiGHS via scipy.milp) to measure size/solve time.
Cells = 1 km grid cells holding nodes. Simplified chains (real 1.0 base-recipe ratios):
 iron ore -> plate (3:2, plate stack 200), copper ore -> wire (1:2, 500), limestone -> concrete (3:1, 500),
 caterium ore -> ingot (3:1, 100), quartz -> crystal (5:3, 200), iron ore + coal -> steel ingot (1+1:1, 100).
Transport: Mk6 belts between neighbour cells (integer belts), trains between opened hubs (binary track,
integer cars, capacity 32 stacks per round trip). Demand at one base cell.
usage: python -I geo_toy.py nodes.json [time_limit_s]
"""
import json, sys, math, time, collections, os
import numpy as np
from scipy.optimize import milp, LinearConstraint, Bounds
from scipy.sparse import lil_matrix

recs = json.load(open(sys.argv[1], encoding='utf8'))
TL = float(sys.argv[2]) if len(sys.argv) > 2 else 30
CELL = 1000.0
MK3 = {'impure': 300, 'normal': 600, 'pure': 1200}
RAWMAP = {'OreIron': 'iron', 'OreCopper': 'copper', 'Stone': 'lime', 'Coal': 'coal', 'OreGold': 'cat', 'RawQuartz': 'quartz'}
sup = collections.defaultdict(lambda: collections.Counter()); pos = collections.defaultdict(list)
for r in recs:
    if r['nodeType'] != 'node': continue
    k = RAWMAP.get(r['resource'][5:-2])
    c = (math.floor(r['x'] / 100 / CELL), math.floor(r['y'] / 100 / CELL))
    pos[c].append((r['x'] / 100, r['y'] / 100))
    if k: sup[c][k] += MK3[r['purity']]
cells = sorted(pos); C = len(cells)
xy = {c: (np.mean([p[0] for p in pos[c]]), np.mean([p[1] for p in pos[c]])) for c in cells}
def dist(a, b): return math.hypot(xy[a][0] - xy[b][0], xy[a][1] - xy[b][1]) * 1.3  # detour factor (assumption)
# items: name -> stack
STACK = {'iron': 100, 'copper': 100, 'lime': 100, 'coal': 100, 'cat': 100, 'quartz': 100,
         'plate': 200, 'wire': 500, 'concrete': 500, 'cating': 100, 'crystal': 200, 'steel': 100}
K = list(STACK)
# recipes: inputs, outputs per machine-minute (one "line" machine at 100 %), machine cost weight
REC = {'plate': ({'iron': 30}, {'plate': 20}), 'wire': ({'copper': 30}, {'wire': 60}),
       'concrete': ({'lime': 45}, {'concrete': 15}), 'cating': ({'cat': 45}, {'cating': 15}),
       'crystal': ({'quartz': 37.5}, {'crystal': 22.5}), 'steel': ({'iron': 45, 'coal': 45}, {'steel': 45})}
base = max(cells, key=lambda c: -math.hypot(xy[c][0] - 600, xy[c][1]))  # cell nearest map middle
if os.environ.get('POORBASE'):  # resource-poorest cell instead
    base = min(cells, key=lambda c: sum(sup[c].values()))
DSC = float(os.environ.get('DEMSCALE', '1'))
DEM = {k: v * DSC for k, v in {'plate': 2000, 'wire': 3000, 'concrete': 1000, 'cating': 400, 'crystal': 500, 'steel': 1500}.items()}
W_BELT_KM, W_TRACK_KM, W_CAR, W_HUB, W_MACH, W_RAW = 10.0, 15.0, 5.0, 60.0, 1.0, 0.001
V = 120 / 3.6; DOCK = 27.08; ACC = 30.0  # s; ACC per stop is an assumption

var = []; lb = []; ub = []; cost = []; integ = []
def nv(c, l=0, u=np.inf, i=0):
    var.append(None); lb.append(l); ub.append(u); cost.append(c); integ.append(i); return len(var) - 1
ext = {(c, k): nv(W_RAW, 0, sup[c][k]) for c in cells for k in STACK if sup[c][k] > 0}
import os
MI = 0 if os.environ.get('RELAX_MACH') else 1
mach = {(c, r): nv(W_MACH, 0, 500, MI) for c in cells for r in REC}
belt_arcs = [(a, b) for a in cells for b in cells if a != b and max(abs(a[0]-b[0]), abs(a[1]-b[1])) == 1]
belts = {}; flow = {}
for (a, b) in belt_arcs:
    belts[(a, b)] = nv(W_BELT_KM * dist(a, b) / 1000, 0, 50, 1)  # one variable per direction
    for k in K: flow[('b', a, b, k)] = nv(0)
hub = {c: nv(W_HUB, 0, 1, 1) for c in cells}
pairs = [(a, b) for i, a in enumerate(cells) for b in cells[i+1:]]
track = {p: nv(W_TRACK_KM * dist(*p) / 1000, 0, 1, 1) for p in pairs}
cars = {p: nv(W_CAR, 0, 64, 1) for p in pairs}
for (a, b) in pairs:
    for k in K: flow[('t', a, b, k)] = nv(0); flow[('t', b, a, k)] = nv(0)
n = len(var); rows = []; rlo = []; rhi = []
A = lil_matrix((0, n))
def row(coefs, lo, hi):
    rows.append(coefs); rlo.append(lo); rhi.append(hi)
inc = collections.defaultdict(list); outg = collections.defaultdict(list)
for key, j in flow.items():
    _, a, b, k = key; outg[(a, k)].append(j); inc[(b, k)].append(j)
for c in cells:
    for k in K:
        co = {}
        if (c, k) in ext: co[ext[(c, k)]] = 1
        for r, (ins, outs) in REC.items():
            co[mach[(c, r)]] = co.get(mach[(c, r)], 0) + outs.get(k, 0) - ins.get(k, 0)
        for j in inc[(c, k)]: co[j] = co.get(j, 0) + 1
        for j in outg[(c, k)]: co[j] = co.get(j, 0) - 1
        d = DEM.get(k, 0) if c == base else 0
        row(co, d, np.inf)  # balance (surplus allowed)
for (a, b), jb in belts.items():   # directed arc a->b, own belt count
    co = {flow[('b', a, b, k)]: 1 for k in K}; co[jb] = -1200; row(co, -np.inf, 0)
for (a, b) in pairs:
    rtd = (2 * dist(a, b) / V + 2 * DOCK + 2 * ACC) / 60  # min
    capstk = 32 / rtd  # stacks/min per car per direction (car runs loaded both ways)
    for (u, w) in ((a, b), (b, a)):
        co = {flow[('t', u, w, k)]: 1 / STACK[k] for k in K}; co[cars[(a, b)]] = -capstk; row(co, -np.inf, 0)
    row({cars[(a, b)]: 1, track[(a, b)]: -64}, -np.inf, 0)
    row({track[(a, b)]: 1, hub[a]: -1}, -np.inf, 0); row({track[(a, b)]: 1, hub[b]: -1}, -np.inf, 0)
PFIX = int(sys.argv[3]) if len(sys.argv) > 3 else -1
if PFIX >= 0: row({hub[c]: 1 for c in cells}, 0, PFIX)  # at most p hubs
A = lil_matrix((len(rows), n))
for i, co in enumerate(rows):
    for j, v in co.items(): A[i, j] = v
t0 = time.time()
res = milp(np.array(cost), constraints=LinearConstraint(A.tocsr(), rlo, rhi), integrality=np.array(integ),
           bounds=Bounds(lb, ub), options={'time_limit': TL, 'mip_rel_gap': 1e-3, 'disp': False})
dt = time.time() - t0
print(f'cells={C} vars={n} int={sum(integ)} rows={len(rows)} base={base} p_fixed={PFIX}')
print(f'status={res.status} {res.message[:60]} obj={res.fun if res.x is not None else None} bound={getattr(res, "mip_dual_bound", None)} time={dt:.1f}s gap={getattr(res, "mip_gap", None)}')
if res.x is not None:
    x = res.x
    hubs = [c for c in cells if x[hub[c]] > 0.5]; nb = sum(round(x[j]) for j in belts.values())
    nt = sum(1 for p in pairs if x[track[p]] > 0.5); nc = sum(round(x[cars[p]]) for p in pairs)
    print(f'hubs={len(hubs)} belts={nb} tracks={nt} cars={nc}')
    for r in REC:
        where = {c: round(x[mach[(c, r)]]) for c in cells if x[mach[(c, r)]] > 0.5}
        print(f'  {r}: {sum(where.values())} machines at {len(where)} cells, base share {where.get(base, 0)}')
    print('  raw shipped by train (items/min):', round(sum(x[j] for key, j in flow.items() if key[0] == 't' and key[3] in RAWMAP.values())),
          ' products by train:', round(sum(x[j] for key, j in flow.items() if key[0] == 't' and key[3] not in RAWMAP.values())))
