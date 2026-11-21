"""G3: candidate sites and candidate hubs for the geographic planner (report 00-最终报告.md sections 5.3 and 7.1).

Reads data/satisfactory-geo/nodes.json, writes data/satisfactory-geo/sites.json and a plot
docs/research/satisfactory-geo/_过程文件/sites.png (nodes, sites, hubs; no game map art).

Sites
  * grid site: a non-empty 1 km grid cell (node, well satellite, well core or geyser inside); its centre is the mean
    of its nodes. Hand-mined deposits are ignored.
  * transit site: an empty cell added so that all grid sites are connected through 8-neighbour belt arcs (components
    are joined along the shortest cell path, deterministically).
  * Each node feeds its own site; feed distance = straight distance to the site centre x detour factor (1.3).
Hubs (at most K, default 12), selection rule from the report:
  1. required sites: the base site (nearest grid site to --base x,y; default the centre of the node bounding box);
  2. the K // 3 sites with the largest raw output (sum of max_rate of solid items and oil; water, nitrogen and
     geysers are not counted), standing in for "solve a belt-only model and take the sites with the largest flow"
     (that needs the solver, so this is a proxy);
  3. the rest by farthest-point sampling over grid sites, starting from the chosen ones.
Only grid sites can be hubs. Ties break on site id, so the output is deterministic.
Usage: python -I tools/geo_sites.py [--k 12] [--cell 1000] [--base X,Y] [--no-plot]
"""
import argparse
import collections
import json
import math
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NODES = os.path.join(ROOT, 'data', 'satisfactory-geo', 'nodes.json')
OUT = os.path.join(ROOT, 'data', 'satisfactory-geo', 'sites.json')
PNG = os.path.join(ROOT, 'docs', 'research', 'satisfactory-geo', '_过程文件', 'sites.png')
DETOUR = 1.3
NOT_COUNTED = {'Desc_Water_C', 'Desc_NitrogenGas_C', 'Desc_GeothermalEnergy_C'}


def cell_of(x, y, size):
    return (math.floor(x / size), math.floor(y / size))


def neighbours(c):
    return [(c[0] + dx, c[1] + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if (dx, dy) != (0, 0)]


def components(cells):
    cells = set(cells)
    seen, comps = set(), []
    for c in sorted(cells):
        if c in seen:
            continue
        comp, stack = [], [c]
        seen.add(c)
        while stack:
            u = stack.pop()
            comp.append(u)
            for v in neighbours(u):
                if v in cells and v not in seen:
                    seen.add(v)
                    stack.append(v)
        comps.append(sorted(comp))
    return comps


def add_transit(grid_cells):
    """Add empty cells until all grid cells form one 8-connected set. Deterministic."""
    cells = set(grid_cells)
    transit = []
    while True:
        comps = components(cells)
        if len(comps) == 1:
            return sorted(transit)
        a = comps[0]  # component of the smallest cell
        best = min(((max(abs(p[0] - q[0]), abs(p[1] - q[1])), p, q) for p in a for comp in comps[1:] for q in comp))
        _, p, q = best
        while max(abs(p[0] - q[0]), abs(p[1] - q[1])) > 1:
            p = (p[0] + (q[0] > p[0]) - (q[0] < p[0]), p[1] + (q[1] > p[1]) - (q[1] < p[1]))
            cells.add(p)
            transit.append(p)


def build(nodes, size, k, base_xy):
    pts = [n for n in nodes if n['type'] != 'deposit']
    by_cell = collections.defaultdict(list)
    for n in pts:
        by_cell[cell_of(n['x'], n['y'], size)].append(n)
    grid_cells = sorted(by_cell)
    transit_cells = add_transit(grid_cells)
    sites = []
    for c in grid_cells + transit_cells:
        members = by_cell.get(c, [])
        if members:
            x = sum(n['x'] for n in members) / len(members)
            y = sum(n['y'] for n in members) / len(members)
        else:
            x, y = (c[0] + 0.5) * size, (c[1] + 0.5) * size
        sites.append({'id': f'S{c[0]}_{c[1]}'.replace('-', 'm'), 'kind': 'grid' if members else 'transit', 'cell': list(c),
                      'x': round(x, 1), 'y': round(y, 1), 'members': members})
    sid = {tuple(s['cell']): s for s in sites}
    out_sites = []
    for s in sites:
        supply = collections.Counter()
        feed = []
        for n in s['members']:
            if n['max_rate']:
                supply[n['item']] += n['max_rate']
            d = math.hypot(n['x'] - s['x'], n['y'] - s['y']) * DETOUR
            feed.append((n['id'], round(d, 1)))
        out_sites.append({
            'id': s['id'], 'kind': s['kind'], 'cell': s['cell'], 'x': s['x'], 'y': s['y'],
            'n_nodes': len(s['members']), 'supply': dict(sorted(supply.items())),
            'feed_dist_max_m': max((d for _, d in feed), default=0.0),
            'feed_dist_mean_m': round(sum(d for _, d in feed) / len(feed), 1) if feed else 0.0,
            'nodes': sorted(n['id'] for n in s['members']),
        })
    # 8-neighbour belt arcs between sites (undirected pairs, length = centre distance x detour)
    arcs = []
    for s in sites:
        for nb in neighbours(tuple(s['cell'])):
            t = sid.get(nb)
            if t and s['id'] < t['id']:
                arcs.append({'a': s['id'], 'b': t['id'],
                             'length_m': round(math.hypot(s['x'] - t['x'], s['y'] - t['y']) * DETOUR, 1)})
    arcs.sort(key=lambda a: (a['a'], a['b']))
    # hubs
    grid = [s for s in out_sites if s['kind'] == 'grid']
    xy = {s['id']: (s['x'], s['y']) for s in out_sites}
    base = min(grid, key=lambda s: (math.hypot(s['x'] - base_xy[0], s['y'] - base_xy[1]), s['id']))['id']
    chosen = [base]
    reason = {base: 'base'}
    out = lambda s: sum(v for i, v in s['supply'].items() if i not in NOT_COUNTED)
    for s in sorted(grid, key=lambda s: (-out(s), s['id'])):
        if len([r for r in reason.values() if r == 'top_output']) >= k // 3:
            break
        if s['id'] not in reason:
            chosen.append(s['id'])
            reason[s['id']] = 'top_output'
    while len(chosen) < k:
        nxt = max((s['id'] for s in grid if s['id'] not in reason),
                   key=lambda i: (min(math.hypot(xy[i][0] - xy[j][0], xy[i][1] - xy[j][1]) for j in chosen), i))
        chosen.append(nxt)
        reason[nxt] = 'farthest_point'
    hubs = [{'site': i, 'reason': reason[i]} for i in sorted(chosen)]
    return out_sites, arcs, hubs, grid_cells, transit_cells


def check(doc, size):
    sites = {s['id']: s for s in doc['sites']}
    grid = [s for s in doc['sites'] if s['kind'] == 'grid']
    assert len(grid) == 39, len(grid)                                   # report 5.3: 1 km grid -> 39 cells
    assert max(s['n_nodes'] for s in grid) == 37 or max(s['n_nodes'] for s in grid) <= 40
    assert len(doc['hubs']) <= doc['params']['k'] <= 12
    assert len({h['site'] for h in doc['hubs']}) == len(doc['hubs'])
    assert all(sites[h['site']]['kind'] == 'grid' for h in doc['hubs'])
    # every non-deposit node is assigned to exactly one site
    nodes = json.load(open(NODES, encoding='utf8'))['nodes']
    assigned = [i for s in doc['sites'] for i in s['nodes']]
    assert len(assigned) == len(set(assigned)) == sum(1 for n in nodes if n['type'] != 'deposit')
    # any two grid sites are connected by a belt path
    adj = collections.defaultdict(set)
    for a in doc['arcs']:
        adj[a['a']].add(a['b'])
        adj[a['b']].add(a['a'])
        assert a['length_m'] > 0
    start = grid[0]['id']
    seen, stack = {start}, [start]
    while stack:
        u = stack.pop()
        for v in adj[u]:
            if v not in seen:
                seen.add(v)
                stack.append(v)
    assert all(s['id'] in seen for s in grid), 'grid sites not belt-connected'
    # supply totals are conserved: the sum over sites equals the per-item total of nodes
    tot = collections.Counter()
    for n in nodes:
        if n['type'] != 'deposit':
            tot[n['item']] += n['max_rate']
    got = collections.Counter()
    for s in doc['sites']:
        got.update(s['supply'])
    assert {k: v for k, v in tot.items() if v} == dict(got)
    assert all(s['feed_dist_max_m'] <= size * math.sqrt(2) * DETOUR for s in grid)  # 1 km cell: <= 1.4 km x detour


def plot(doc, nodes, size):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from matplotlib.lines import Line2D
    fig, ax = plt.subplots(figsize=(10, 7), dpi=110)
    colors = {'Desc_OreIron_C': '#8c564b', 'Desc_OreCopper_C': '#d9822b', 'Desc_Stone_C': '#999999', 'Desc_Coal_C': '#222222',
              'Desc_OreGold_C': '#e8c600', 'Desc_RawQuartz_C': '#d070c0', 'Desc_Sulfur_C': '#c8d82c', 'Desc_OreBauxite_C': '#c04040',
              'Desc_OreUranium_C': '#2ca02c', 'Desc_SAM_C': '#6a3d9a', 'Desc_LiquidOil_C': '#000000', 'Desc_Water_C': '#1f77b4',
              'Desc_NitrogenGas_C': '#17becf', 'Desc_GeothermalEnergy_C': '#ff7f0e'}
    xs = [s['cell'][0] for s in doc['sites']]
    ys = [s['cell'][1] for s in doc['sites']]
    for s in doc['sites']:
        cx, cy = s['cell']
        ax.add_patch(plt.Rectangle((cx * size, cy * size), size, size, fill=s['kind'] == 'transit',
                                   fc='#eeeeee', ec='#bbbbbb', lw=0.8))
    for a in doc['arcs']:
        pa = next(s for s in doc['sites'] if s['id'] == a['a'])
        pb = next(s for s in doc['sites'] if s['id'] == a['b'])
        ax.plot([pa['x'], pb['x']], [pa['y'], pb['y']], color='#cccccc', lw=0.5, zorder=1)
    for n in nodes:
        if n['type'] == 'deposit':
            continue
        ax.scatter(n['x'], n['y'], s=9, c=colors.get(n['item'], '#555'), zorder=3, linewidths=0)
    gx = [s['x'] for s in doc['sites'] if s['kind'] == 'grid']
    gy = [s['y'] for s in doc['sites'] if s['kind'] == 'grid']
    ax.scatter(gx, gy, marker='s', s=34, facecolors='none', edgecolors='#333', zorder=4, label='site')
    tx = [s['x'] for s in doc['sites'] if s['kind'] == 'transit']
    ty = [s['y'] for s in doc['sites'] if s['kind'] == 'transit']
    ax.scatter(tx, ty, marker='x', s=30, c='#888', zorder=4)
    hub = {h['site'] for h in doc['hubs']}
    hx = [s['x'] for s in doc['sites'] if s['id'] in hub]
    hy = [s['y'] for s in doc['sites'] if s['id'] in hub]
    ax.scatter(hx, hy, marker='*', s=260, c='#d62728', edgecolors='k', linewidths=0.6, zorder=5)
    handles = [Line2D([], [], marker='s', ls='', mfc='none', mec='#333', label='grid site'),
               Line2D([], [], marker='x', ls='', c='#888', label='transit site'),
               Line2D([], [], marker='*', ls='', c='#d62728', mec='k', ms=13, label=f'candidate hub ({len(hub)})')]
    seen_items = sorted({n['item'] for n in nodes if n['type'] != 'deposit'})
    for i in seen_items:
        handles.append(Line2D([], [], marker='o', ls='', c=colors.get(i, '#555'), ms=5, label=i[5:-2]))
    ax.legend(handles=handles, fontsize=7, loc='upper left', bbox_to_anchor=(1.01, 1.0), framealpha=0.9)
    ax.set_aspect('equal')
    ax.set_xlim(min(xs) * size - 300, (max(xs) + 1) * size + 300)
    ax.set_ylim(min(ys) * size - 300, (max(ys) + 1) * size + 300)
    ax.set_xlabel('x (m)')
    ax.set_ylabel('y (m)')
    g = len(gx)
    ax.set_title(f'Satisfactory nodes, {g} grid sites + {len(tx)} transit, {len(hub)} candidate hubs ({size:g} m cells)')
    fig.tight_layout()
    os.makedirs(os.path.dirname(PNG), exist_ok=True)
    fig.savefig(PNG)
    plt.close(fig)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--k', type=int, default=12)
    ap.add_argument('--cell', type=float, default=1000.0)
    ap.add_argument('--base', default=None, help='x,y in metres of the base / demand point')
    ap.add_argument('--no-plot', action='store_true')
    ap.add_argument('--no-write', action='store_true')
    a = ap.parse_args()
    assert 1 <= a.k <= 12
    doc_in = json.load(open(NODES, encoding='utf8'))
    nodes = doc_in['nodes']
    solid = [n for n in nodes if n['type'] != 'deposit']
    if a.base:
        bx, by = (float(v) for v in a.base.split(','))
    else:
        bx = (min(n['x'] for n in solid) + max(n['x'] for n in solid)) / 2
        by = (min(n['y'] for n in solid) + max(n['y'] for n in solid)) / 2
    sites, arcs, hubs, grid_cells, transit_cells = build(nodes, a.cell, a.k, (bx, by))
    doc = {
        'params': {'cell_m': a.cell, 'k': a.k, 'detour_factor': DETOUR, 'base_xy': [round(bx, 1), round(by, 1)],
                   'hub_rule': 'base site; top K//3 grid sites by raw output (proxy for belt-only flow); rest by farthest-point sampling'},
        'source': 'data/satisfactory-geo/nodes.json',
        'counts': {'grid_sites': len(grid_cells), 'transit_sites': len(transit_cells), 'arcs': len(arcs), 'hubs': len(hubs)},
        'sites': sites, 'arcs': arcs, 'hubs': hubs,
    }
    if a.cell == 1000.0:
        check(doc, a.cell)
    if not a.no_write:
        with open(OUT, 'w', encoding='utf8', newline='\n') as f:
            json.dump(doc, f, ensure_ascii=False, separators=(',', ':'))
            f.write('\n')
    if not a.no_plot:
        plot(doc, nodes, a.cell)
    print('sites:', doc['counts'], 'hubs:', [h['site'] for h in hubs])


if __name__ == '__main__':
    main()
