"""Compare variants: python compare.py base.jsonl var1.jsonl var2.jsonl ...  (markdown table)"""
import sys, json, os


def load(f):
    out = {}
    for line in open(f, encoding='utf-8'):
        if line.startswith('{'):
            r = json.loads(line)
            out[r['name']] = r
    return out


files = sys.argv[1:]
data = [(os.path.basename(f).replace('.jsonl', '').replace('var_', ''), load(f)) for f in files]
names = [n for n in data[1][1]] if len(data) > 1 else list(data[0][1])
names = [n for n in names if all(n in d for _, d in data[1:])]
print('| 场景 | ' + ' | '.join(lbl for lbl, _ in data) + ' |')
print('|---|' + '---|' * len(data))
for n in names:
    cells = []
    for lbl, d in data:
        r = d.get(n)
        if r is None:
            cells.append('–')
            continue
        if r['status'] != 'Optimal':
            cells.append(f"无方案 ({r['units']}列)")
            continue
        g = float(r['gap']) * 100
        if r['proven']:
            cells.append(f"✔ {r['t_end']:.0f} ms, {r['nodes']}n, obj {r['obj']:.4g}, {r['units']}列")
        else:
            cells.append(f"gap {g:.2f}%, {r['nodes']}n, obj {r['obj']:.5g}, {r['units']}列")
    print(f"| {n} | " + ' | '.join(cells) + ' |')
