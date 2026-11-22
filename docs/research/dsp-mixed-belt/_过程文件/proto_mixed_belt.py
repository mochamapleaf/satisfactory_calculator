"""Prototype of the 2-item direct-feed mixed-belt model (report §2.3), checked against 笑笑's numbers.

Units: stacks/s (叠/s). C = belt capacity in stacks/s (Mk.III = 30). c[i] = per-machine use of item i.
Run: python -I proto_mixed_belt.py
"""
from fractions import Fraction as F


def simulate(C, c, start, first, intervals, n_machines):
    """start: flows after the control unit; first = (item, k) first refill after k machines;
    intervals: dict item -> machines between the previous refill (of the other item) and this one.
    Returns (min flow per item after any machine, list of (pos, item, amount) refills, end flows)."""
    f = dict(start)
    mins = {i: f[i] for i in f}
    feeds = []
    nxt_item, nxt_at = first
    other = {a: b for a, b in zip(c, list(c)[1:] + list(c)[:1])}
    for m in range(1, n_machines + 1):
        for i in f:
            f[i] -= c[i]
            mins[i] = min(mins[i], f[i])
        if m == nxt_at:
            free = C - sum(f.values())
            f[nxt_item] += free
            feeds.append((m, nxt_item, free))
            nxt_item = other[nxt_item]
            nxt_at = m + intervals[nxt_item]
    return mins, feeds, f


def case(name, C, c, start, first, intervals, n, expect):
    mins, feeds, end = simulate(C, c, start, first, intervals, n)
    print(f"{name}: mins={ {k: float(v) for k, v in mins.items()} } expect {expect}")
    print("   feeds:", [(p, i, float(a)) for p, i, a in feeds][:6], "... end", {k: float(v) for k, v in end.items()})
    return mins


if __name__ == "__main__":
    # ep19 Mk.II proliferator: recomposing x3, speedup x2 -> 18 items/s = 4.5 stacks/s (Y 3, D 1.5)
    # control unit 3/5 Y : 2/5 D; refill Y after 3, then D after 2, Y after 4, D after 2 ...
    m = case("ep19 绿剂", F(30), {"Y": F(3), "D": F(3, 2)}, {"Y": F(18), "D": F(12)}, ("Y", 3), {"Y": 4, "D": 2}, 30,
             "Y 4.5 / D 4.5 [BV1fXkxYDEbW 28:52-34:37]")
    assert m["Y"] == F(9, 2) and m["D"] == F(9, 2)
    # ep19 carbon nanotube: quantum chem plant x2, speedup x2 -> 1 stack/s (G 0.75, T 0.25); control 4:1
    # cycle: T refill 8 machines after G refill, G refill 24 machines after T refill; mins 2 / 2 [62:49]
    # first refill: G after 8 machines ("摇匀后等8台" [67:08]) reproduces mins 2/2
    m = case("ep19 碳管", F(30), {"G": F(3, 4), "T": F(1, 4)}, {"G": F(24), "T": F(6)}, ("G", 8), {"T": 8, "G": 24}, 72,
             "G 2 / T 2 [BV1fXkxYDEbW 62:49]")
    assert m["G"] == 2 and m["T"] == 2

    # closed form (report §2.3): steady-state sum of the two minima S = C - P (cA^2 + cA cB + cB^2)/(cA + cB)
    def S(C, P, a, b):
        return C - P * (a * a + a * b + b * b) / (a + b)
    assert S(F(30), 6, F(3), F(3, 2)) == 9            # 4.5 + 4.5
    assert S(F(30), 32, F(3, 4), F(1, 4)) == 4        # 2 + 2
    assert S(F(30), 30, F(3, 4), F(3, 8)) == F(15, 4)  # 1.875 * 2
    assert S(F(30), 20, F(3, 4), F(3, 4)) == F(15, 2)  # 3.75 * 2
    assert S(F(30), 9, F(3), F(3, 2)) == F(-3, 2)     # ep19 61:34 9-cycle; ep20 01:44 corrects the minima to 0 and -1.5
    # ep19 Mk.III proliferator: extra products, recomposing x3 -> 4.5 items/s = 1.125 stacks/s (P 0.75, N 0.375)
    # control 18:12 ; first P refill after k ; then N after 10, P after 20 ...  k=16 -> P 1.5 N 2.25 ; 17 equal ; 18 chosen
    for k, exp, want in [(16, "P 1.5 / N 2.25", (F(3, 2), F(9, 4))), (17, "equal 1.875", (F(15, 8), F(15, 8))),
                         (18, "P 2.25 / N 1.5", (F(9, 4), F(3, 2)))]:
        m = case(f"ep19 蓝剂 k={k}", F(30), {"P": F(3, 4), "N": F(3, 8)}, {"P": F(18), "N": F(12)}, ("P", k), {"P": 20, "N": 10},
                 k + 60, exp + " [BV1fXkxYDEbW 76:44]")
        assert (m["P"], m["N"]) == want
    # ep20 electromagnetic turbine (绿马达): extra products, x3 -> 6 items/s = 1.5 stacks/s (M 0.75, L 0.75); control 3:2
    # motor refill after 1, coil after 10, motor after 10 -> cycle 20 [BV1iQrKYoEoy 26:46]
    m = case("ep20 绿马达", F(30), {"M": F(3, 4), "L": F(3, 4)}, {"M": F(18), "L": F(12)}, ("M", 1), {"L": 10, "M": 10}, 61,
             "equal mins (not stated)")
    assert m["M"] == m["L"] == F(15, 4)

    # largest P with S >= 2m (necessary condition only; feasibility needs the (sigma, k) search)
    def max_p(a, b, p, q, m=F(3, 2), C=F(30)):
        best = None
        for t in range(1, 60):
            P = t * (p + q)
            if S(C, P, a, b) >= 2 * m and P * max(a, b) <= C:
                best = P
        return best
    assert [max_p(F(3), F(3, 2), 1, 2), max_p(F(3, 4), F(1, 4), 3, 1), max_p(F(3, 4), F(3, 8), 2, 1),
            max_p(F(3, 4), F(3, 4), 1, 1)] == [6, 32, 30, 24]

    # ---- return flows at the control unit (report §2.4). These numbers were NOT inputs: the model
    # computes them from the layouts above and they are compared with what 笑笑 reads off his sheet.
    def flows_at(C, c, start, first, intervals, n):
        return simulate(C, c, start, first, intervals, n)[2]
    # carbon tubes: the unit sits 16 machines after a Ti refill; he reads G 8 / T 6 there [BV1fXkxYDEbW 67:28-67:39].
    # In the layout above the Ti refills are at 16, 48, ... so "16 after" is machine 32 (= one full period).
    e = flows_at(F(30), {"G": F(3, 4), "T": F(1, 4)}, {"G": F(24), "T": F(6)}, ("G", 8), {"T": 8, "G": 24}, 32)
    assert (e["G"], e["T"]) == (8, 6)          # T returns at exactly 1/5 belt: the Ti feed of the unit idles [70:19]
    assert e["G"] + e["T"] > 12                # more than one Mk.II belt -> he needs the two-belt unit [67:42]
    # turbines, 60 machines = 3 full periods: the limited item comes back at exactly its 2/5 share [BV1iQrKYoEoy 28:05]
    e = flows_at(F(30), {"M": F(3, 4), "L": F(3, 4)}, {"M": F(18), "L": F(12)}, ("M", 1), {"L": 10, "M": 10}, 60)
    assert (e["M"], e["L"]) == (F(9, 2), 12)
    # Mk.II proliferator, 30 machines = 5 full periods: D returns at exactly 12, Y at 13.5; total 25.5 > 12,
    # i.e. over one Mk.II belt -- he suspects this himself at [BV1fXkxYDEbW 79:46].
    e = flows_at(F(30), {"Y": F(3), "D": F(3, 2)}, {"Y": F(18), "D": F(12)}, ("Y", 3), {"Y": 4, "D": 2}, 30)
    assert (e["Y"], e["D"]) == (F(27, 2), 12)

    # ---- the (sigma, first item, k) search of report §2.3, which the report describes but he never ran.
    def best_for(a, b, P, C=F(30), step=1):
        """Best achievable min(min_A, min_B) over the whole line for period P. step=2: one belt feeds two
        rows, so refill positions exist only every 2 machines."""
        nA, nB = P * a / (a + b), P * b / (a + b)
        if nA.denominator != 1 or nA % step or nB % step:
            return None
        best = None
        for s5 in (1, 2, 3, 4):                      # share of A after the control unit, in fifths
            for first in ("A", "B"):
                for k in range(step, P + 1, step):
                    mins, _, _ = simulate(C, {"A": a, "B": b}, {"A": C * s5 / 5, "B": C * (5 - s5) / 5},
                                          (first, k), {"A": int(nA), "B": int(nB)}, k + 2 * P)
                    v = min(mins.values())
                    if best is None or v > best[0]:
                        best = (v, s5, first, k)
        return best
    half = lambda a, b, P: S(F(30), P, a, b) / 2
    # the optimum of the search is S/2 in all four of his layouts (he also aimed at equal minima)
    for a, b, P in [(F(3), F(3, 2), 6), (F(3, 4), F(1, 4), 32), (F(3, 4), F(3, 8), 30), (F(3, 4), F(3, 4), 20)]:
        assert best_for(a, b, P)[0] == half(a, b, P), (a, b, P)
    # Mk.III proliferator with one belt feeding two rows: only even positions exist, so 17 is out and the
    # best is 1.5 (k = 16 or 18) -- exactly his "没有一个精确十七的位置" [BV1fXkxYDEbW 77:08]
    assert best_for(F(3, 4), F(3, 8), 30, step=2)[0] == F(3, 2)
    # S >= 2m is necessary, not sufficient: with two rows per belt the Mk.III line has S/2 = 1.875 but
    # can only reach 1.5 (asserted above). Single-row turbine P = 24 does reach its S/2 = 1.5:
    assert best_for(F(3, 4), F(3, 4), 24)[0] == F(3, 2)

    # ---- period choice (report §2.3 step 4): largest feasible P; with a row length R the repeat unit
    # lcm(P, R) must stay <= unit_max (MINE default 4R: he accepted 60 and rejected 120 with R = 15).
    from math import lcm
    def choose_p(a, b, p, q, m=F(3, 2), R=None, unit_max=None, C=F(30)):
        pick = None
        for t in range(1, 80):
            P = t * (p + q)
            if P * max(a, b) > C or S(C, P, a, b) < 2 * m:
                continue
            if R and lcm(P, R) > (unit_max or 4 * R):
                continue
            bf = best_for(a, b, P, C)
            if bf and bf[0] >= m:
                pick = P
        return pick
    assert choose_p(F(3), F(3, 2), 1, 2, R=15) == 6           # Mk.II proliferator
    assert choose_p(F(3, 4), F(1, 4), 3, 1) == 32             # carbon tubes (row length not stated)
    assert choose_p(F(3, 4), F(3, 8), 2, 1, R=15) == 30       # Mk.III proliferator
    assert choose_p(F(3, 4), F(3, 4), 1, 1, R=15) == 20       # turbines (24 would need a 120-machine unit)

    # ---- a case that was not used to set anything: processors [BV1iQrKYoEoy 56:00-56:40].
    # 2 boards + 2 components per 3 s, x3 building, extra-products mode -> 4 items/s = 1 stack/s per machine,
    # one belt feeding two rows -> 1 + 1 stacks/s per belt position. He says: refill every 7 positions,
    # "at every 10 it is exactly zero", 7/8/9 all fine.  (Reading "per position = a pair" is an assumption;
    # the single-row speed-up reading gives the same numbers, single-row extra-products does not.)
    assert S(F(30), 20, F(1), F(1)) == 0 and S(F(30), 18, F(1), F(1)) == 3 and S(F(30), 14, F(1), F(1)) == 9

    # ---- ladder groups (report §2.5): feeds = one rate-limited belt (0/6/12/30) + up to 2 sorters (stacks/s).
    # Evidence: feed == demand runs at N = 10 (motors) and N = 12 (titanium alloy); at N = 24 feed == demand
    # fails and feed = demand x 13/12 runs; N = 26 at feed == demand fails. So: surplus rho = 1/12 for N > 12
    # (the only surplus he showed working; anything in (0, 1/12) is untested). MINE, to be calibrated.
    sorters = [F(0), F(3, 2), F(3, 4), F(1, 2), F(3), F(1), F(6), F(2)]
    Q = sorted({bt + s1 + s2 for bt in (F(0), F(6), F(12), F(30)) for s1 in sorters for s2 in sorters})
    def ladder(c, C=F(30), rho=F(1, 12), cap=40, belts_only=False):
        q = [x for x in Q if not belts_only or x in (F(6), F(12), F(30))]
        for N in range(cap, 0, -1):
            need = [N * ci * (1 + rho if N > 12 else 1) for ci in c]
            feeds = [min((x for x in q if x >= d), default=None) for d in need]
            if None not in feeds and sum(feeds) <= C:
                return N, feeds
    assert ladder([F(3, 2), F(3, 4), F(3, 4)])[0] == 10                      # electric motor
    assert ladder([F(3, 8)] * 3) == (24, [F(39, 4)] * 3)                     # particle container: 24 at 9.75 each, no cap needed
    assert ladder([F(1), F(1, 2), F(1, 2)], belts_only=True) == (12, [12, 6, 6])   # titanium alloy as he built it
    print("titanium alloy, belt + sorters:", ladder([F(1), F(1, 2), F(1, 2)]))
    assert ladder([F(1), F(1, 2), F(1, 2)])[0] == 13
    print("all asserts passed")
