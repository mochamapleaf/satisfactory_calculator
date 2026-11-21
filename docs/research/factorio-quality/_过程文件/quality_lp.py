"""Validate the item@quality LP model for Factorio 2.0 quality upcycling.

Single-item loop: ingredient set I@q --craft--> (1+p) product P with a quality roll,
P@q (q < legendary) --recycle--> 0.25 I with a quality roll. Legendary P is the output.
Each (tier, #quality modules) pair is one LP column ("mode"), exactly how the planner
would model it. Solved with scipy/HiGHS.

Checks:
  A. scottmsul/FactorioQualityOptimizer README: legendary gears from normal plates,
     AM3, legendary T3 modules, 4 slots  -> 79.8786 plates per 0.5 legendary gear/s.
  B. wiki Tutorial:Quality_upcycling_math: EM plant, 5 legendary Q3 (31 %), +50 % base,
     recycler 24.8 %  -> 0.013 legendary per normal input (L^100 bottom row).
  C. closed form (forward substitution on the lower-triangular loop matrix) == LP.
  D. pure self-recycling (item recycles to itself) and the 2.0 asteroid "casino".

Run: python -I quality_lp.py
"""
import numpy as np
from scipy.optimize import linprog

T = 5  # normal, uncommon, rare, epic, legendary


def qmat(q):
    """M[i, j] = P(output tier j | input tier i) with total quality chance q."""
    M = np.zeros((T, T))
    for i in range(T):
        if i == T - 1:
            M[i, i] = 1.0
            continue
        M[i, i] = 1 - q
        for j in range(i + 1, T - 1):
            M[i, j] = q * 0.9 * 0.1 ** (j - i - 1)
        M[i, T - 1] = q * 0.1 ** (T - 2 - i)
    return M


def solve(slots, p0, q_mod, p_mod, q_rec=4 * 0.062, ret=0.25, craft_time=None,
          craft_speed=None, rec_time=None, rec_speed=None, objective="input", prod_cap=3.0):
    """Min normal ingredient sets per 1 legendary product (or machines if objective='machines')."""
    cols = []  # (kind, tier, k)
    for i in range(T):
        for k in range(slots + 1):
            if i == T - 1 and k > 0:
                continue  # quality modules useless at legendary
            cols.append(("craft", i, k))
    for i in range(T - 1):
        cols.append(("rec", i, 4))
    cols.append(("supply", 0, 0))
    n = len(cols)
    # rows: I@0..4, P@0..4
    A = np.zeros((2 * T, n))
    b = np.zeros(2 * T)
    Qr = qmat(q_rec)
    cost = np.zeros(n)
    for c, (kind, i, k) in enumerate(cols):
        if kind == "craft":
            p = min(p0 + (slots - k) * p_mod, prod_cap)
            Q = qmat(k * q_mod)
            A[i, c] -= 1.0
            for j in range(T):
                A[T + j, c] += (1 + p) * Q[i, j]
            if objective == "machines":
                speed = craft_speed * max(0.2, 1 - 0.15 * (slots - k) - 0.05 * k)
                cost[c] = craft_time / speed
        elif kind == "rec":
            A[T + i, c] -= 1.0
            for j in range(T):
                A[j, c] += ret * Qr[i, j]
            if objective == "machines":
                cost[c] = rec_time / (rec_speed * 0.8)
        else:
            A[0, c] += 1.0
            if objective == "input":
                cost[c] = 1.0
    b[2 * T - 1] = 1.0  # one legendary product per unit time; other rows balance to 0
    res = linprog(cost, A_eq=A, b_eq=b, bounds=[(0, None)] * n, method="highs")
    assert res.status == 0, res.message
    x = res.x
    used = {f"{kind}@{i} q{k}": round(v, 6) for (kind, i, k), v in zip(cols, x) if v > 1e-9}
    return res.fun, x[-1], used


def closed_form(slots, p0, q_mod, p_mod, ks, q_rec=0.248, ret=0.25):
    """Legendary per normal ingredient for fixed #quality modules ks[i] per tier (i<4).

    x_i = ingredient sets crafted at tier i (steady state, 1 normal set supplied).
    Product at tier j: y_j = sum_i x_i (1+p_i) Q_i[i,j]; recycled ingredient back at m:
    sum_{j<4} y_j ret Qr[j,m].  Lower triangular -> forward substitution:
      x_m = (delta_m0 + sum_{i<m} L[m,i] x_i) / (1 - L[m,m]).
    Legendary products: sum_i x_i (1+p_i) Q_i[i,4] + x_4 (1+p_4).
    """
    Qr = qmat(q_rec)
    Pm = []
    for i in range(T):
        k = ks[i] if i < T - 1 else 0
        p = min(p0 + (slots - k) * p_mod, 3.0)
        Pm.append(((1 + p) * qmat(k * q_mod))[i])
    L = np.zeros((T, T))  # L[m, i]: ingredient at m per ingredient crafted at i
    for i in range(T - 1):
        for j in range(T - 1):
            for m in range(T):
                L[m, i] += Pm[i][j] * ret * Qr[j, m]
    x = np.zeros(T)
    for m in range(T):
        x[m] = ((1.0 if m == 0 else 0.0) + L[m, :m] @ x[:m]) / (1 - L[m, m] if m < T - 1 else 1)
    leg = sum(x[i] * Pm[i][T - 1] for i in range(T - 1)) + x[T - 1] * Pm[T - 1][T - 1]
    return leg, x


def self_recycle(q, ret):
    """Item whose recycler returns itself: legendary per normal item (closed form).

    a = ret(1-q) stays, the step i->j moves ret*M[i,j].  Same forward substitution.
    """
    M = ret * qmat(q)
    x = np.zeros(T)
    for m in range(T - 1):
        x[m] = ((1.0 if m == 0 else 0.0) + M[:m, m] @ x[:m]) / (1 - M[m, m])
    return sum(x[i] * M[i, T - 1] for i in range(T - 1))


if __name__ == "__main__":
    Q3L, P3L = 0.062, 0.25  # legendary T3 quality / productivity module (2.0 values)

    # A. scottmsul gears: 2 plates -> 1 gear, AM3 (4 slots, no base prod).
    sets, _, used = solve(4, 0.0, Q3L, P3L)
    print(f"A. gears: {sets:.4f} normal ingredient sets per legendary gear"
          f" = {sets * 2 * 0.5:.4f} plates/s for 0.5 legendary gear/s (README 79.8786)")
    for k, v in used.items():
        print("     ", k, v)
    leg, _ = closed_form(4, 0.0, Q3L, P3L, [2, 2, 2, 2])
    print(f"   closed form 2Q/2P every tier: {1 / leg:.4f} sets per legendary")

    # B. wiki EM plant all quality.
    leg, x = closed_form(5, 0.5, Q3L, P3L, [5, 5, 5, 5])
    print(f"B. EM plant 5xQ3L every tier: {leg:.5f} legendary per normal INGREDIENT"
          f" (wiki's 0.013 uses the per-normal-PRODUCT metric, see F)")
    for slots, p0, name in [(3, 0, "chem plant"), (4, 0, "AM3"), (4, .5, "foundry"),
                            (5, .5, "EM plant"), (8, 0, "cryo plant")]:
        sets, _, used = solve(slots, p0, Q3L, P3L)
        print(f"   LP optimum {name:10s}: {1 / sets * 100:.6f} % legendary per normal ingredient,"
              f" {sets:.2f} inputs per legendary")

    # C. closed form vs LP for a fixed loadout.
    for ks in ([4, 4, 4, 4], [2, 2, 2, 2], [3, 2, 2, 2]):
        leg, _ = closed_form(4, 0.0, Q3L, P3L, ks)
        print(f"C. AM3 ks={ks}: closed form {1 / leg:.4f} sets/legendary")

    # D. self-recycling and asteroid casino (2.0).
    print(f"D. self-recycle 4xQ3L (24.8 %, 25 %): {self_recycle(0.248, 0.25):.6f} legendary per normal"
          f" -> {1 / self_recycle(0.248, 0.25):.1f} : 1")
    print(f"   asteroid reprocessing 2.0 (2xQ3L = 12.4 %, 80 % return):"
          f" {self_recycle(0.124, 0.8):.4f} legendary per normal chunk")

    # E. min machines for the gear loop at 0.5 legendary gear/s (legendary buildings).
    m, s, used = solve(4, 0.0, Q3L, P3L, craft_time=0.5, craft_speed=1.25 * 2.5,
                       rec_time=0.5 / 16, rec_speed=0.5 * 2.5, objective="machines")
    print(f"E. min machines (LP, fractional) for 0.5 gear/s: {m * 0.5:.3f} machines,"
          f" {s * 2 * 0.5:.2f} plates/s")
    f, _, used = solve(4, 0.0, Q3L, P3L)
    mach = 0.0
    for key, v in used.items():
        k = int(key.split(" q")[1])
        if key.startswith("craft"):
            mach += v * 0.5 / (1.25 * 2.5 * max(0.2, 1 - 0.15 * (4 - k) - 0.05 * k))
        elif key.startswith("rec"):
            mach += v * (0.5 / 16) / (0.5 * 2.5 * 0.8)
    print(f"   input-optimal plan for 0.5 gear/s: {mach * 0.5:.2f} machines, {f * 2 * 0.5:.2f} plates/s")

    # F. wiki yield table metric: legendary products per 1 normal PRODUCT entering the recycler
    #    (wiki L = A.R + C, L^100 bottom-left). Integer loadouts with machine mixing = LP;
    #    fractional module counts plugged into (1+p(k))Q(kq) = what the wiki table did.
    from scipy.optimize import minimize

    def wiki_metric(slots, p0, ks, leg_p=None):
        Qr = qmat(0.248)
        R = (0.25 * Qr).T.copy()
        R[4, 4] = 0
        A = np.zeros((T, T))
        for i in range(T):
            k = ks[i] if i < T - 1 else 0
            p = min(p0 + (slots - k) * P3L, 3.0) if (i < T - 1 or leg_p is None) else leg_p
            A[:, i] = (1 + p) * qmat(k * Q3L)[i]
        C = np.zeros((T, T)); C[4, 4] = 1
        return np.linalg.matrix_power(A @ R + C, 300)[4, 0]

    print(f"F. wiki EM all-quality incl. legendary stage (p=0.5): {wiki_metric(5, .5, [5] * 4, leg_p=.5):.6f} (wiki 0.013)")
    # LP with mixing: products per normal product = sum_m 0.25 Qr[0,m] * (legendary per ingredient at m).
    Qr0 = qmat(0.248)[0]
    for slots, p0, name, wiki in [(3, 0, "chem plant", .344061), (4, 0, "AM3", .586191),
                                  (4, .5, "foundry", 1.624266), (5, .5, "EM plant", 2.722332),
                                  (8, 0, "cryo plant", 4.835199)]:
        # LP: start with one normal product recycled -> supply ingredient@m = 0.25 Qr0[m]
        best_int = max(wiki_metric(slots, p0, list(ks)) for ks in
                       __import__("itertools").product(range(slots + 1), repeat=4))
        r = min((minimize(lambda k: -wiki_metric(slots, p0, k), [s0] * 4, bounds=[(0, slots)] * 4)
                 for s0 in (0.5, 1, 2)), key=lambda r: r.fun)
        print(f"   {name:10s}: integer loadouts {best_int * 100:.4f} %, fractional modules"
              f" {-r.fun * 100:.4f} %  (wiki {wiki} %)")

    # G. 7:1 ratio sensitivity: assembler 4 slots all quality; recycler fixed 24.8 % vs same modules.
    for q in (0.062, 0.05, 0.025):
        _, x1 = closed_form(4, 0, q, 0.25, [4] * 4)
        _, x2 = closed_form(4, 0, q, 0.25, [4] * 4, q_rec=4 * q)
        print(f"G. module {q * 100:.1f} %: normal:higher crafts = {x1[0] / x1[1:4].sum():.2f} (recycler 24.8 %),"
              f" {x2[0] / x2[1:4].sum():.2f} (recycler same modules)")
    print(f"   self-recycle with 2.1 values (q_r 25 %): {1 / self_recycle(0.25, 0.25):.1f} : 1;"
          f" asteroid 2.0: {1 / self_recycle(0.124, 0.8):.1f} : 1")

    # H. wiki metric as an LP that may mix every integer loadout within a tier (machine mixing),
    #    and the wiki's own fractional rows substituted into (1+p(k))Q(kq).
    import itertools

    def wiki_lp(slots, p0, q_mod=Q3L, p_mod=P3L, q_rec=0.248):
        Qr = qmat(q_rec)
        cols = [("craft", i, k) for i in range(T) for k in range(slots + 1) if not (i == T - 1 and k > 0)]
        cols += [("rec", j, 0) for j in range(T - 1)] + [("sink", 4, 0)]
        A = np.zeros((2 * T, len(cols)))
        for c, (kind, i, k) in enumerate(cols):
            if kind == "craft":
                p = min(p0 + (slots - k) * p_mod, 3.0)
                A[i, c] -= 1
                A[T:, c] += (1 + p) * qmat(k * q_mod)[i]
            elif kind == "rec":
                A[T + i, c] -= 1
                A[:T, c] += 0.25 * Qr[i]
            else:
                A[2 * T - 1, c] -= 1
        b = np.zeros(2 * T)
        b[:T] = -0.25 * Qr[0]          # one normal product recycled at the start
        cost = np.zeros(len(cols)); cost[-1] = -1
        res = linprog(cost, A_eq=A, b_eq=b, bounds=[(0, None)] * len(cols), method="highs")
        assert res.status == 0
        return -res.fun

    for slots, p0, name in [(3, 0, "chem plant"), (4, 0, "AM3"), (4, .5, "foundry"),
                            (5, .5, "EM plant"), (8, 0, "cryo plant")]:
        print(f"H. wiki metric, LP mixing integer loadouts, {name:10s}: {wiki_lp(slots, p0) * 100:.6f} %")
    print(f"   wiki fractional row AM3 (1.67,1.67,1.67,1.8): {wiki_metric(4, 0, [1.67, 1.67, 1.67, 1.8]) * 100:.6f} % (wiki 0.586191)")
    print(f"   wiki fractional row foundry (1.4,1.4,1.4,1.5): {wiki_metric(4, .5, [1.4, 1.4, 1.4, 1.5]) * 100:.6f} % (wiki 1.624266)")

    # I. normal : (uncommon..epic) craft ratio by loadout (legendary modules, recycler 24.8 %).
    for slots, p0, ks, name in [(4, 0, [0] * 4, "AM3 0Q4P"), (4, 0, [2] * 4, "AM3 2Q2P (input-optimal)"),
                                (4, 0, [4] * 4, "AM3 4Q0P"), (5, .5, [5] * 4, "EM plant 5Q")]:
        leg, x = closed_form(slots, p0, Q3L, P3L, ks)
        print(f"I. {name:26s}: ratio {x[0] / x[1:4].sum():.2f}, {1 / leg:.2f} sets per legendary;"
              f" tier work share {np.round(x[1:4] / x[0], 4)}")

    # J. LDS shuffle: cast 5 plastic -> (1+p) LDS in a foundry (4 slots, +50 % base, + research),
    #    recycle LDS -> 1.25 plastic (recycler 4xQ3L). Legendary LDS is recycled to legendary plastic.
    def lds(research, ks, q_rec=0.248):
        Qr = qmat(q_rec)
        rows = []
        for i in range(T - 1):
            k = ks[i]
            p = min(0.5 + research + (4 - k) * P3L, 3.0)
            rows.append((1 + p) / 5 * qmat(k * Q3L)[i])   # LDS per plastic cast at tier i
        x = np.zeros(T - 1)                                  # plastic cast at tier i per normal plastic
        for m in range(T - 1):
            inflow = (1.0 if m == 0 else 0.0) + sum(
                x[i] * sum(rows[i][j] * 1.25 * Qr[j, m] for j in range(T - 1)) for i in range(m))
            stay = sum(rows[m][j] * 1.25 * Qr[j, m] for j in range(T - 1))
            x[m] = inflow / (1 - stay)
        # legendary plastic: legendary LDS recycled (x1.25) + legendary plastic straight from the recycler
        leg_plastic = sum(x[i] * rows[i][T - 1] for i in range(T - 1)) * 1.25 \
            + sum(x[i] * rows[i][j] * 1.25 * Qr[j, T - 1] for i in range(T - 1) for j in range(T - 1))
        return leg_plastic, x.sum() / 5

    cost = lambda L: sum(1000 * 1.5 ** l for l in range(1, L + 1))
    for L, ks, name in [(15, [0] * 4, "foundry 4xP3L"), (25, [4] * 4, "foundry 4xQ3L")]:
        lp_, casts = lds(L * 0.1, ks)
        print(f"J. LDS research {L}: {name}: {lp_:.4f} legendary plastic per normal plastic,"
              f" {casts * 5:.1f} casts per 5 normal plastic, research cost {cost(L) / 1e6:.2f} M")
    for L in (5, 10, 14):
        best = max(itertools.product(range(5), repeat=4), key=lambda ks: lds(L * 0.1, ks)[0])
        print(f"   research {L}: best integer loadout {best}, {lds(L * 0.1, best)[0]:.3f} legendary plastic per plastic")
