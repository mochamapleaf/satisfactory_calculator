import json,sys,collections,math
recs=json.load(open(sys.argv[1],encoding='utf8'))
MK3={'impure':300,'normal':600,'pure':1200}   # Mk3 @250 % (Mk.6 belt not limiting for pure: 1200)
WELL={'impure':75,'normal':150,'pure':300}     # well extractor @250 %
OIL={'impure':150,'normal':300,'pure':600}  # oil extractor 60/120/240 @100 % x2.5
lim=collections.Counter()
for r in recs:
    t=r['nodeType'];res=r['resource'][5:-2];p=r['purity']
    if t=='node': lim[res]+= (OIL if res=='LiquidOil' else MK3)[p]
    elif t=='frackingSatellite': lim[res]+=WELL[p]
print('world limits from dataset:',dict(lim))
pts=[(r['x']/100,r['y']/100,r) for r in recs if r['nodeType'] in('node','frackingSatellite','geyser')]
def clusters(th):
    n=len(pts);par=list(range(n))
    def f(i):
        while par[i]!=i: par[i]=par[par[i]];i=par[i]
        return i
    for i in range(n):
        for j in range(i+1,n):
            if math.hypot(pts[i][0]-pts[j][0],pts[i][1]-pts[j][1])<th: par[f(i)]=f(j)
    g=collections.defaultdict(list)
    for i in range(n): g[f(i)].append(i)
    return g
for th in (150,300,500,800):
    g=clusters(th); sizes=sorted((len(v) for v in g.values()),reverse=True)
    print(f'th={th} m: {len(g)} clusters, biggest {sizes[:8]}, singletons {sizes.count(1)}')
# water: wells by distance from map centre of node bbox
xs=[p[0] for p in pts];ys=[p[1] for p in pts];cx=(min(xs)+max(xs))/2;cy=(min(ys)+max(ys))/2
print('bbox m',min(xs),max(xs),min(ys),max(ys),'centre',cx,cy)
w=[(round(math.hypot(r['x']/100-cx,r['y']/100-cy)),r['purity']) for r in recs if r['nodeType']=='frackingCore' and 'Water' in r['resource']]
print('water well cores dist from bbox centre (m):',sorted(w))
for cell in (500,750,1000):
    cells=collections.Counter((math.floor(p[0]/cell),math.floor(p[1]/cell)) for p in pts)
    print(f'grid {cell} m: {len(cells)} non-empty cells; nodes/cell max {max(cells.values())}')
# resource types present per 1 km cell
cells=collections.defaultdict(set)
for x,y,r in pts: cells[(math.floor(x/1000),math.floor(y/1000))].add(r['resource'][5:-2])
print('1 km cells with water well:',sum('Water' in s for s in cells.values()),'of',len(cells))
