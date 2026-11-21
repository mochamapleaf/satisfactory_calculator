"""10k universe matrix/min: machines / compute cost / flow / power under proliferator schemes.
Each item: first non-alt recipe; top-tier building; multi-output recipes charged fully to the
requested product. Costs: machine 1.0, lab 1.5, sorter X per port (in+out). Optional proliferator
consumption (Mk.III, 60 sprays, one spray per input item) fed back into the chain."""
import json, sys, itertools
d = json.load(open(sys.argv[1] if len(sys.argv) > 1 else 'data/dsp.json', encoding='utf-8'))
SORTER = float(sys.argv[2]) if len(sys.argv) > 2 else 0.12
PWR = float(sys.argv[3]) if len(sys.argv) > 3 else 0.0  # cost units per MW of generation
TARGET = 10000.0
B = {b['id']: b for b in d['buildings']}
raw = {i['id'] for i in d['items'] if i['raw']}
rec = {}
for r in d['recipes']:
    if r['alt']:
        continue
    for it, _ in r['out']:
        rec.setdefault(it, r)
TIER = {}  # building-id -> allowed (tier-A run restricts producers)
def top(r):
    ps = r['producers'] or [r['building']]
    if TIER:
        ps = [p for p in ps if p in TIER] or ps
    return max(ps, key=lambda p: B[p]['speed'])
LAB = {'matrix-lab', 'df-self-evolution-lab'}
def modes(r, scheme):
    allowed = r['productivity_allowed']
    m = {'none': [('none', 1, 1)], 'speed': [('speed', 2, 1)],
         'prod': [('prod', 1, 1.25)] if allowed else [('speed', 2, 1)],
         'mix': [('none', 1, 1), ('speed', 2, 1)] + ([('prod', 1, 1.25)] if allowed else [])}[scheme]
    return m
def unit_info(it, mode, prolif):
    r = rec[it]; b = top(r); sp = B[b]['speed']
    out = dict(r['out'])[it]
    name, smul, omul = mode
    mach = 1.0 / (out * sp * smul * omul)
    ins = {i: q / (out * omul) for i, q in r['in']}
    if prolif and name != 'none':
        ins['proliferator-3'] = ins.get('proliferator-3', 0) + sum(q for i, q in r['in']) / (out * omul) / 60
    c = (1.5 if b in LAB else 1.0) + SORTER * (len(r['in']) + len(r['out']))
    pw = B[b]['power'] * (2.5 if name != 'none' else 1.0)
    return b, mach, ins, c, pw
def solve(scheme, prolif):
    cost = {it: 0.0 for it in rec}
    choice = {}
    for _ in range(60):
        for it in rec:
            best = None
            for md in modes(rec[it], scheme):
                b, mach, ins, c, pw = unit_info(it, md, prolif)
                v = mach * (c + PWR * unit_info(it, md, prolif)[4]) + sum(q * cost.get(i, 0.0) for i, q in ins.items() if i not in raw)
                if best is None or v < best[0]:
                    best = (v, md)
            cost[it] = best[0]; choice[it] = best[1]
    dem = {}
    for _ in range(300):
        new = {'universe-matrix': TARGET}
        for it, q in dem.items():
            if it in raw: continue
            _, _, ins, _, _ = unit_info(it, choice[it], prolif)
            for i, a in ins.items():
                new[i] = new.get(i, 0) + a * q
        dem = new
    M = C = P = F = 0.0; labs = 0.0; per = {}
    for it, q in dem.items():
        if it in raw: F += q; continue
        b, mach, ins, c, pw = unit_info(it, choice[it], prolif)
        n = mach * q; M += n; C += n * (c + PWR * pw); P += n * pw; F += q
        if b in LAB: labs += n
        per[it] = (b, choice[it][0], n)
    rawd = {i: q for i, q in dem.items() if i in raw}
    return M, C, P, F, labs, per, rawd
if __name__ == '__main__':
    for prolif in (False, True):
        print('--- proliferator consumption counted:', prolif, ' sorter', SORTER)
        for sch in ('none', 'speed', 'prod', 'mix'):
            M, C, P, F, labs, per, rawd = solve(sch, prolif)
            print(f"{sch:6s} machines {M:8.0f} labs {labs:6.0f} cost {C:8.0f} power {P/1000:6.1f} GW flow {F/1e6:5.2f}M/min photon {rawd.get('critical-photon',0):6.0f}/min prolif3 {per.get('proliferator-3',(0,0,0))[2]:5.0f} mach")
    M, C, P, F, labs, per, rawd = solve('mix', True)
    from collections import Counter
    print('mix modes count by machines:', Counter({k: 0 for k in ()}), {m: round(sum(v[2] for v in per.values() if v[1]==m)) for m in ('none','speed','prod')})
    TIER.update(dict.fromkeys(['assembling-machine-3','plane-smelter','quantum-chemical-plant','matrix-lab','oil-refinery','miniature-particle-collider','fractionator']))
    for sch in ('none',):
        M2, C2, P2, F2, labs2, per2, _ = solve(sch, False)
        print(f'tier A (Mk.III/plane/quantum/matrix-lab), no prolif: machines {M2:.0f} labs {labs2:.0f} cost {C2:.0f} power {P2/1000:.1f} GW')
    TIER.clear()
    print('mix choice:', {k: v[1] for k, v in sorted(per.items(), key=lambda kv: -kv[1][2])[:25]})
