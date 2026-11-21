"""G1: convert the Satisfactory world resource node dump to data/satisfactory-geo/nodes.json and validate it.

Source: rockfactory/satisfactory-logistics @bb997b42 src/recipes/WorldResourceNodes.json (MIT code; coordinates are
Coffee Stain game data), stored unchanged at data/satisfactory-geo/raw/WorldResourceNodes.json.
Usage: python -I tools/geo_nodes.py        (converts, then checks; exits non-zero on any failed assert)
"""
import collections
import json
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW = os.path.join(ROOT, 'data', 'satisfactory-geo', 'raw', 'WorldResourceNodes.json')
OUT = os.path.join(ROOT, 'data', 'satisfactory-geo', 'nodes.json')
GAME = os.path.join(ROOT, 'data', 'game-data.json')
SOURCE = {
    'repo': 'rockfactory/satisfactory-logistics',
    'commit': 'bb997b42',
    'path': 'src/recipes/WorldResourceNodes.json',
    'url': 'https://raw.githubusercontent.com/rockfactory/satisfactory-logistics/bb997b42/src/recipes/WorldResourceNodes.json',
    'licence': 'MIT (code and compilation); node coordinates are Coffee Stain Studios game data',
}
# Max extraction per node at 250 % clock: Miner Mk.3 (300/600/1200 per minute), oil extractor (150/300/600),
# resource well satellite with pressurizer at 250 % (75/150/300).
MK3 = {'impure': 300, 'normal': 600, 'pure': 1200}
OIL = {'impure': 150, 'normal': 300, 'pure': 600}
WELL = {'impure': 75, 'normal': 150, 'pure': 300}
TYPE = {'node': 'node', 'frackingSatellite': 'well', 'frackingCore': 'well_core',
        'geyser': 'geyser', 'deposit': 'deposit'}


def max_rate(rec):
    t, item, p = rec['nodeType'], rec['resource'], rec['purity']
    if t == 'node':
        return (OIL if item == 'Desc_LiquidOil_C' else MK3)[p]
    if t == 'frackingSatellite':
        return WELL[p]
    return 0  # core (a well's centre, no output of its own), geyser (power, not an item), hand-mined deposit


def convert():
    raw = json.load(open(RAW, encoding='utf8'))
    nodes = []
    for r in raw:
        nodes.append({
            'id': r['id'], 'item': r['resource'], 'purity': r['purity'], 'type': TYPE[r['nodeType']],
            'x': round(r['x'] / 100, 2), 'y': round(r['y'] / 100, 2), 'z': round(r['z'] / 100, 2),
            'max_rate': max_rate(r),
        })
    nodes.sort(key=lambda n: (n['item'], n['type'], n['x'], n['y'], n['id']))
    doc = {
        'source': SOURCE,
        'units': {'x/y/z': 'metres (game cm / 100)', 'max_rate': 'items or m3 per minute at 250 % clock; 0 = no output'},
        'types': {'node': 'solid or oil node', 'well': 'resource well satellite (carries the output)',
                  'well_core': 'resource well centre', 'geyser': 'geothermal geyser (item GeothermalEnergy)',
                  'deposit': 'hand-mined deposit'},
        'nodes': nodes,
    }
    with open(OUT, 'w', encoding='utf8', newline='\n') as f:
        json.dump(doc, f, ensure_ascii=False, separators=(',', ':'))
        f.write('\n')
    return doc


def check(doc):
    nodes = doc['nodes']
    game = json.load(open(GAME, encoding='utf8'))
    items = {i['id'] for i in game['items']}
    limits = {r['item']: r['limit'] for r in game['resources']}
    assert len(nodes) == 626, len(nodes)
    assert len({n['id'] for n in nodes}) == 626
    cnt = collections.Counter(n['type'] for n in nodes)
    assert cnt == {'node': 459, 'well': 118, 'well_core': 17, 'geyser': 31, 'deposit': 1}, cnt
    for n in nodes:
        assert n['purity'] in MK3
        if n['type'] != 'geyser':
            assert n['item'] in items, n['item']  # ids match data/game-data.json
    tot = collections.Counter()
    for n in nodes:
        tot[n['item']] += n['max_rate']
    for item, lim in limits.items():
        if lim is None:
            continue  # water: no world limit in game-data (open water); wells are checked below
        assert tot[item] == lim, (item, tot[item], lim)
    # 10 solids and oil: node count by purity (wiki Resource_node, iron 39/42/46)
    iron = collections.Counter(n['purity'] for n in nodes if n['item'] == 'Desc_OreIron_C' and n['type'] == 'node')
    assert (iron['impure'], iron['normal'], iron['pure']) == (39, 42, 46), iron
    wells = sum({'impure': 30, 'normal': 60, 'pure': 120}[n['purity']]
                for n in nodes if n['item'] == 'Desc_Water_C' and n['type'] == 'well')
    assert wells * 2.5 == 13125, wells
    xs = [n['x'] for n in nodes]
    ys = [n['y'] for n in nodes]
    assert -3300 < min(xs) and max(xs) < 4300 and -3800 < min(ys) and max(ys) < 3800
    return cnt, {k: v for k, v in tot.items() if limits.get(k)}


if __name__ == '__main__':
    d = convert()
    c, t = check(d)
    print('nodes.json:', len(d['nodes']), dict(c))
    print('world limits match game-data.json:', len(t), 'resources; water well max 13125 m3/min')
