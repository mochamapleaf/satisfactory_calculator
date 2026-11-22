"""Known optimum for `integer_shared_known_optimum` (quality_tests.rs), independent of lp.rs.

Widget loop (part -> widget 1:1 in AM3, widget recycles to 0.25 part), Space Age 2.0 legendary
Q3 (6.2 %, -5 % speed) / P3 (+25 %, -15 % speed), 1 legendary widget / min. Cost: 1 per normal
part + 2 per building. Shared machines (report §2.1, §5 step 6): one integer N per
(recipe, loadout) over the input tiers it runs, sum_i x_i / speed <= N.
Run: python -I int_known_optimum.py   (scipy >= 1.9, HiGHS MILP)
"""
import numpy as np
from scipy.optimize import milp, LinearConstraint, Bounds

QV, QS = 0.062, -0.05
PV, PS = 0.25, -0.15
RATE, WB, TARGET = 60.0, 2.0, 1.0


def qrow(q, i):
    row = np.zeros(5)
    if i >= 4:
        row[4] = 1.0
        return row
    row[i] = 1 - q
    for j in range(i + 1, 4):
        row[j] = q * 0.9 * 0.1 ** (j - i - 1)
    row[4] = q * 0.1 ** (3 - i)
    return row


cols = []  # (group, speed, {item: coef per unit of work})
for i in range(5):
    loads = [(0, 0), (0, 4)] + ([(k, 4 - k) for k in range(1, 5)] if i < 4 else [])
    for kq, kp in loads:
        speed = 1.25 * max(0.2, 1 + kq * QS + kp * PS)
        p = kp * PV
        net = {('part', i): -RATE}
        for j, sh in enumerate(qrow(kq * QV, i)):
            if sh > 1e-12:
                net[('widget', j)] = net.get(('widget', j), 0) + RATE * (1 + p) * sh
        cols.append((('craft', kq, kp), speed, net))
for j in range(5):
    for kq in ([0] + list(range(1, 5)) if j < 4 else [0]):
        speed = 0.5 * max(0.2, 1 + kq * QS)
        net = {('widget', j): -RATE}
        for m, sh in enumerate(qrow(kq * QV, j)):
            if sh > 1e-12:
                net[('part', m)] = net.get(('part', m), 0) + RATE * 0.25 * sh
        cols.append((('recycle', kq), speed, net))

groups = sorted({c[0] for c in cols}, key=str)
items = sorted({k for c in cols for k in c[2]})
nx, ng = len(cols), len(groups)
# variables: x (work) | N (group machines) | raw normal part | surplus per item
nv = nx + ng + 1 + len(items)
c = np.zeros(nv)
c[nx:nx + ng] = WB
c[nx + ng] = 1.0
A, lo, hi = [], [], []
for it in items:
    row = np.zeros(nv)
    for k, (_, _, net) in enumerate(cols):
        row[k] = net.get(it, 0.0)
    if it == ('part', 0):
        row[nx + ng] = 1.0
    row[nx + ng + 1 + items.index(it)] = -1.0
    b = TARGET if it == ('widget', 4) else 0.0
    A.append(row); lo.append(b); hi.append(b)
for g, gname in enumerate(groups):
    row = np.zeros(nv)
    for k, (gk, speed, _) in enumerate(cols):
        if gk == gname:
            row[k] = 1.0 / speed
    row[nx + g] = -1.0
    A.append(row); lo.append(-np.inf); hi.append(0.0)
integrality = np.zeros(nv)
integrality[nx:nx + ng] = 1
res = milp(c, constraints=LinearConstraint(np.array(A), lo, hi), integrality=integrality,
           bounds=Bounds(0, np.inf), options={"mip_rel_gap": 1e-9})
print(res.status, res.message)
print(f"optimum {res.fun:.9f}  parts {res.x[nx + ng]:.6f}  buildings {res.x[nx:nx + ng].sum():.0f}")
for g, gname in enumerate(groups):
    if res.x[nx + g] > 0.5:
        print("  ", gname, round(res.x[nx + g]))
lp = milp(c, constraints=LinearConstraint(np.array(A), lo, hi), bounds=Bounds(0, np.inf))
print(f"LP {lp.fun:.9f}")
