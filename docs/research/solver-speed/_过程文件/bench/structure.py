"""Recipe-graph structure for typical plans: cycles (SCC) and single-producer items.
python structure.py <repo data dir>"""
import json, sys, os
from collections import defaultdict

sys.setrecursionlimit(10000)
D = sys.argv[1]


def analyse(f, targets, alt=True, label=''):
    d = json.load(open(os.path.join(D, f), encoding='utf-8'))
    recs = [r for r in d['recipes'] if (alt or not r.get('alt')) and not r.get('recycling')]
    io = lambda r, k: [x[0] for x in r[k]]
    raw = set(x['item'] for x in d['resources'])
    name2id = {i['name']: i['id'] for i in d['items']}
    tg = [name2id.get(t, t) for t in targets]
    by_out = defaultdict(list)
    for r in recs:
        for o in io(r, 'out'):
            by_out[o].append(r)
    need = set(tg)
    q = list(tg)
    used = {}
    while q:
        it = q.pop()
        for r in by_out.get(it, []):
            if r['id'] in used:
                continue
            if r.get('building') == 'Desc_Converter_C' and any(o in raw for o in io(r, 'out')):
                continue
            used[r['id']] = r
            for i in io(r, 'in'):
                if i not in need:
                    need.add(i)
                    q.append(i)
    R = list(used.values())
    prod = defaultdict(list)
    cons = defaultdict(list)
    for r in R:
        for o in io(r, 'out'):
            prod[o].append(r['id'])
        for i in io(r, 'in'):
            cons[i].append(r['id'])
    g = {r['id']: set() for r in R}
    for r in R:
        for o in io(r, 'out'):
            for c in cons.get(o, []):
                g[r['id']].add(c)
    idx, low, st, on, sccs, c = {}, {}, [], set(), [], [0]

    def sc(v):
        idx[v] = low[v] = c[0]
        c[0] += 1
        st.append(v)
        on.add(v)
        for w in g[v]:
            if w not in idx:
                sc(w)
                low[v] = min(low[v], low[w])
            elif w in on:
                low[v] = min(low[v], idx[w])
        if low[v] == idx[v]:
            comp = []
            while True:
                w = st.pop()
                on.discard(w)
                comp.append(w)
                if w == v:
                    break
            sccs.append(comp)

    for v in g:
        if v not in idx:
            sc(v)
    cyc = sum(len(s) for s in sccs if len(s) > 1 or s[0] in g[s[0]])
    items = [i for i in need if i not in raw]
    single = [i for i in items if len(prod[i]) == 1]
    multi = [i for i in items if len(prod[i]) > 1]
    solo = [r for r in R if all(len(prod[o]) == 1 for o in io(r, 'out'))]
    # machines per recipe (tiers) for DSP / Factorio
    tiers = sum(max(1, len(r.get('producers') or [])) for r in R)
    print(f"| {label} | {len(R)} | {tiers} | {cyc} ({cyc*100//max(1,len(R))}%) | {max(len(s) for s in sccs)} | {len(items)} | {len(single)} | {len(multi)} | {len(solo)} |")


print('| 计划 | 配方数 | 配方×机器等级 | 在环里的配方 | 最大 SCC | 非原料物品 | 只有 1 个配方能产 | 多个配方能产 | 其全部产物都只有它能产的配方 |')
print('|---|---|---|---|---|---|---|---|---|')
P4 = ['Assembly Director System', 'Magnetic Field Generator', 'Thermal Propulsion Rocket', 'Nuclear Pasta']
analyse('game-data.json', ['Heavy Modular Frame'], False, 'Satisfactory HMF, 无替代')
analyse('game-data.json', ['Heavy Modular Frame'], True, 'Satisfactory HMF, 全替代')
analyse('game-data.json', P4, False, 'Satisfactory 阶段 4, 无替代')
analyse('game-data.json', P4, True, 'Satisfactory 阶段 4, 全替代')
analyse('dsp.json', ['universe-matrix'], True, 'DSP 宇宙矩阵')
analyse('factorio.json', ['automation-science-pack', 'logistic-science-pack', 'military-science-pack', 'chemical-science-pack', 'production-science-pack', 'utility-science-pack'], True, 'Factorio 6 种科技包')
