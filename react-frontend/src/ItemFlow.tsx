import { useMemo, useState } from 'react'
import { Icon } from './Icon'
import { dn, fmt, itemName, type GameData, type Plan } from './solver'
import { t } from './i18n'

// Sankey of item flow: sources (mined / supplied) → items → recipes → items → … → outputs.
// Every item is a node whose size is the flow through it, so "how much of X, and where does it go"
// reads directly off the chart. Recipe loops (an item feeding back into its own chain) are broken
// at their smallest-rate edge, which is the recycle; those back edges are drawn as thin dashed return
// lines under the chart, labelled with their rate. Links that span several columns are routed
// through invisible dummy nodes so they pass around, not through, the nodes in between.
// A recipe that uses and makes the same item (DSP X-ray cracking, Kovarex) is shown with its net
// amount; the part it recycles internally is noted on the recipe (↻).

type Kind = 'item' | 'recipe' | 'src' | 'sink' | 'dummy'
interface N {
  id: string; kind: Kind; label: string; sub: string; building: string; note: string; icon?: string
  value: number; drawn: number; layer: number; y: number; h: number; order: number
}
/** One drawn piece of a link between adjacent columns (a link spanning k columns has k segments). */
interface Seg { from: string; to: string; rate: number; link: L; sy: number; ty: number; w: number }
interface L { from: string; to: string; item: string; rate: number; back: boolean; sy: number; ty: number; w: number; segs: Seg[] }

const NODE_W = 12
const COL = 190
const GAP = 14
const DUMMY_GAP = 6
const PAD = 30

function hue(s: string) {
  let h = 0
  for (const c of s) h = (h * 31 + c.charCodeAt(0)) >>> 0
  return h % 360
}

interface RecipeFlow {
  id: string; name: string; building: string; who: string; machines: number
  ins: Map<string, number>; outs: Map<string, number>; cyc: { item: string; rate: number }[]; main?: string
}

/** One entry per recipe id (modes merged), with self-loops netted. Shared by the chart and the table. */
function recipeFlows(plan: Plan, rn: (id: string, name: string) => string, bn: (name: string) => string): RecipeFlow[] {
  const by = new Map<string, RecipeFlow>()
  for (const r of plan.recipes) {
    const e = by.get(r.id) ?? {
      id: r.id, name: rn(r.id, r.name), building: bn(r.building), who: `${rn(r.id, r.name)} (${bn(r.building)})`, machines: 0,
      ins: new Map(), outs: new Map(), cyc: [], main: r.outputs[0]?.item,
    }
    e.machines += r.machines
    for (const l of r.inputs) e.ins.set(l.item, (e.ins.get(l.item) ?? 0) + l.rate)
    for (const l of r.outputs) e.outs.set(l.item, (e.outs.get(l.item) ?? 0) + l.rate)
    by.set(r.id, e)
  }
  for (const e of by.values()) e.cyc = netSelfLoops(e.ins, e.outs)
  return [...by.values()]
}

/** Mark back edges until the forward graph is acyclic: each cycle is broken at its smallest-rate edge. */
function markBackEdges(nodes: Map<string, N>, links: L[]) {
  const out = new Map<string, L[]>()
  const hasIn = new Set<string>()
  for (const l of links) {
    if (!out.has(l.from)) out.set(l.from, [])
    out.get(l.from)!.push(l)
    hasIn.add(l.to)
  }
  for (const es of out.values()) es.sort((a, b) => b.rate - a.rate)
  const roots = [...nodes.keys()].sort((a, b) => (hasIn.has(a) ? 1 : 0) - (hasIn.has(b) ? 1 : 0))
  const findCycle = (): L[] | null => {
    const state = new Map<string, 1 | 2>() // 1 on stack, 2 done
    for (const r of roots) {
      if (state.has(r)) continue
      const stack: [string, number][] = [[r, 0]]
      const via: L[] = [] // via[k] is the edge that entered stack[k + 1]
      const pos = new Map<string, number>([[r, 0]])
      state.set(r, 1)
      while (stack.length) {
        const top = stack[stack.length - 1]
        const es = out.get(top[0]) ?? []
        if (top[1] >= es.length) { state.set(top[0], 2); pos.delete(top[0]); stack.pop(); via.pop(); continue }
        const e = es[top[1]++]
        if (e.back) continue
        const s = state.get(e.to)
        if (s === 1) return [...via.slice(pos.get(e.to)!), e]
        if (!s) { state.set(e.to, 1); pos.set(e.to, stack.length); stack.push([e.to, 0]); via.push(e) }
      }
    }
    return null
  }
  for (let guard = 0; guard <= links.length; guard++) {
    const cyc = findCycle()
    if (!cyc) return
    // ties go to the edge that closed the cycle
    let m = cyc[cyc.length - 1]
    for (const e of cyc) if (e.rate < m.rate - 1e-9) m = e
    m.back = true
  }
}

function build(plan: Plan, names: Map<string, string>, flows: RecipeFlow[], scaleMax: number) {
  const nodes = new Map<string, N>()
  const links: L[] = []
  const node = (id: string, kind: Kind, label: string, sub = '', icon?: string, building = '') => {
    if (!nodes.has(id)) nodes.set(id, { id, kind, label, sub, building, note: '', icon, value: 0, drawn: 0, layer: 0, y: 0, h: 0, order: 0 })
    return id
  }
  const item = (it: string) => node('i:' + it, 'item', names.get(it) ?? it, '', it)
  const link = (from: string, to: string, it: string, rate: number) => {
    if (rate > 1e-6) links.push({ from, to, item: it, rate, back: false, sy: 0, ty: 0, w: 0, segs: [] })
  }
  for (const e of flows) {
    const rid = node('r:' + e.id, 'recipe', e.name, `${fmt(e.machines)}× ${e.building}`, e.main, e.building)
    nodes.get(rid)!.note = e.cyc.map(c => `↻ ${fmt(c.rate)} ${names.get(c.item) ?? c.item}`).join(', ')
    for (const [it, q] of e.ins) link(item(it), rid, it, q)
    for (const [it, q] of e.outs) link(rid, item(it), it, q)
  }
  for (const l of plan.raw) link(node('raw:' + l.item, 'src', t('mined', '开采')), item(l.item), l.item, l.rate)
  for (const l of plan.supplied) link(node('sup:' + l.item, 'src', t('supplied', '外部供应')), item(l.item), l.item, l.rate)
  for (const l of plan.targets) link(item(l.item), node('out:' + l.item, 'sink', t('output', '产出')), l.item, l.rate)
  for (const l of plan.surplus) link(item(l.item), node('extra:' + l.item, 'sink', t('byproduct', '副产物')), l.item, l.rate)

  markBackEdges(nodes, links)
  const out = new Map<string, L[]>()
  const inc = new Map<string, L[]>()
  for (const l of links) {
    if (!out.has(l.from)) out.set(l.from, [])
    if (!inc.has(l.to)) inc.set(l.to, [])
    out.get(l.from)!.push(l)
    inc.get(l.to)!.push(l)
  }

  // --- layers: longest path over forward edges; sources just left of their item, sinks just right
  const indeg = new Map<string, number>()
  for (const n of nodes.values()) indeg.set(n.id, 0)
  for (const l of links) if (!l.back) indeg.set(l.to, indeg.get(l.to)! + 1)
  const queue = [...nodes.values()].filter(n => indeg.get(n.id) === 0).map(n => n.id)
  while (queue.length) {
    const id = queue.shift()!
    for (const l of out.get(id) ?? []) {
      if (l.back) continue
      const t = nodes.get(l.to)!
      t.layer = Math.max(t.layer, nodes.get(id)!.layer + 1)
      indeg.set(l.to, indeg.get(l.to)! - 1)
      if (indeg.get(l.to) === 0) queue.push(l.to)
    }
  }
  for (const n of nodes.values()) {
    if (n.kind === 'src') {
      const t = nodes.get((out.get(n.id) ?? [])[0]?.to ?? '')
      if (t) n.layer = Math.max(0, t.layer - 1)
    } else if (n.kind === 'sink') {
      const f = nodes.get((inc.get(n.id) ?? [])[0]?.from ?? '')
      if (f) n.layer = f.layer + 1
    }
  }
  for (const n of nodes.values()) {
    const i = (inc.get(n.id) ?? []).reduce((s, l) => s + l.rate, 0)
    const o = (out.get(n.id) ?? []).reduce((s, l) => s + l.rate, 0)
    n.value = Math.max(i, o)
    // drawn height counts forward bands only; loop edges are thin lines at the node's bottom edge
    const fi = (inc.get(n.id) ?? []).reduce((s, l) => s + (l.back ? 0 : l.rate), 0)
    const fo = (out.get(n.id) ?? []).reduce((s, l) => s + (l.back ? 0 : l.rate), 0)
    n.drawn = Math.max(fi, fo)
  }

  // --- dummy nodes: a forward link spanning several columns gets one per column it skips
  const sOut = new Map<string, Seg[]>()
  const sIn = new Map<string, Seg[]>()
  const seg = (l: L, from: string, to: string) => {
    const s: Seg = { from, to, rate: l.rate, link: l, sy: 0, ty: 0, w: 0 }
    l.segs.push(s)
    if (!sOut.has(from)) sOut.set(from, [])
    if (!sIn.has(to)) sIn.set(to, [])
    sOut.get(from)!.push(s)
    sIn.get(to)!.push(s)
  }
  let dn = 0
  for (const l of links) {
    if (l.back) continue
    const a = nodes.get(l.from)!, b = nodes.get(l.to)!
    let prev = a.id
    for (let layer = a.layer + 1; layer < b.layer; layer++) {
      const id = 'd:' + dn++
      nodes.set(id, { id, kind: 'dummy', label: '', sub: '', building: '', note: '', value: l.rate, drawn: l.rate, layer, y: 0, h: 0, order: 0 })
      seg(l, prev, id)
      prev = id
    }
    seg(l, prev, b.id)
  }
  const maxLayer = Math.max(0, ...[...nodes.values()].map(n => n.layer))

  // --- vertical order: barycentre sweeps over segments (dummies included)
  const layers: N[][] = Array.from({ length: maxLayer + 1 }, () => [])
  for (const n of nodes.values()) layers[n.layer].push(n)
  layers.forEach(l => l.forEach((n, i) => (n.order = i)))
  const bary = (n: N, dir: 'in' | 'out') => {
    const es = (dir === 'in' ? sIn.get(n.id) : sOut.get(n.id)) ?? []
    let s = 0, w = 0
    for (const e of es) {
      const o = nodes.get(dir === 'in' ? e.from : e.to)!
      s += o.order * e.rate; w += e.rate
    }
    return w ? s / w : n.order
  }
  for (let it = 0; it < 8; it++) {
    const seq = it % 2 ? [...layers].reverse() : layers
    for (const l of seq) {
      const b = new Map(l.map(n => [n.id, bary(n, it % 2 ? 'out' : 'in')]))
      l.sort((a, c) => b.get(a.id)! - b.get(c.id)!)
      l.forEach((n, i) => (n.order = i))
    }
  }

  // --- sizes: one pixel scale for the whole chart; each real node gets room for its label
  const labelH = (n: N) => n.kind === 'dummy' ? 0 : n.kind === 'recipe' ? (n.note ? 42 : 30) : n.kind === 'item' ? 22 : 16
  const biggest = Math.max(1, ...layers.map(l => l.reduce((s, n) => s + n.drawn, 0)))
  const crowd = Math.max(...layers.map(l => l.filter(n => n.kind !== 'dummy').length))
  const k = Math.min(scaleMax, Math.max(140, 34 * crowd)) / biggest
  let height = 0
  for (const l of layers) {
    let y = PAD
    for (const n of l) {
      n.h = n.kind === 'dummy' ? Math.max(1, n.drawn * k) : Math.max(3, n.drawn * k)
      const slot = Math.max(n.h, labelH(n))
      n.y = y + (slot - n.h) / 2
      y += slot + (n.kind === 'dummy' ? DUMMY_GAP : GAP)
    }
    height = Math.max(height, y)
  }
  // attachment points, sorted so bands do not cross at the node
  for (const n of nodes.values()) {
    let y = n.y
    for (const s of [...(sOut.get(n.id) ?? [])].sort((a, b) => nodes.get(a.to)!.y - nodes.get(b.to)!.y)) {
      s.w = Math.max(1, s.rate * k); s.sy = y + s.w / 2; y += s.rate * k
    }
    y = n.y
    for (const s of [...(sIn.get(n.id) ?? [])].sort((a, b) => nodes.get(a.from)!.y - nodes.get(b.from)!.y)) {
      s.ty = y + Math.max(1, s.rate * k) / 2; y += s.rate * k
    }
  }
  for (const l of links) {
    if (l.back) {
      l.w = 2; l.sy = nodes.get(l.from)!.y + nodes.get(l.from)!.h - 1; l.ty = nodes.get(l.to)!.y + nodes.get(l.to)!.h - 1
    } else l.w = l.segs[0].w
  }
  const real = [...nodes.values()].filter(n => n.kind !== 'dummy')
  return { nodes: real, all: nodes, links, height, width: (maxLayer + 1) * COL + 160 }
}

export default function ItemFlow({ data, plan }: { data?: GameData; plan: Plan }) {
  const [focus, setFocus] = useState<string | null>(null)
  const names = useMemo(() => {
    const m = new Map<string, string>()
    const nm = (l: { item: string; name: string }) => (data ? itemName(data, l.item) : l.name)
    for (const r of plan.recipes) for (const l of [...r.inputs, ...r.outputs]) m.set(l.item, nm(l))
    for (const l of [...plan.raw, ...plan.supplied, ...plan.targets, ...plan.surplus]) m.set(l.item, nm(l))
    return m
  }, [plan, data])
  const flows = useMemo(() => recipeFlows(
    plan,
    (id, name) => (data ? dn(data.recipes.find(r => r.id === id), data.game, name) : name),
    name => (data ? dn(data.buildings.find(b => b.name === name || b.id === name), data.game, name) : name),
  ), [plan, data])
  const g = useMemo(() => build(plan, names, flows, 520), [plan, names, flows])
  const balance = useMemo(() => {
    // "loop" in the table means the same thing as in the chart: a dashed return edge carries the item,
    // or a recipe recycles it internally (↻)
    const loops = new Map<string, string>()
    for (const f of flows) for (const c of f.cyc) loops.set(c.item, t(`recycled inside ${f.name} (↻ on the recipe)`, `在 ${f.name} 内部循环利用 (配方上的 ↻)`))
    for (const l of g.links) if (l.back) loops.set(l.item, t('part of it returns upstream to the chain that makes it (dashed line in the chart)', '一部分回流到上游产线 (图中虚线)'))
    return itemBalance(plan, names, flows, loops)
  }, [plan, names, flows, g])
  if (plan.status !== 'optimal') return <p className="muted">{t('Nothing to draw.', '没有可画的内容。')}</p>

  const nx = (id: string) => 20 + g.all.get(id)!.layer * COL
  const backs = g.links.filter(l => l.back)
  const noted = g.nodes.some(n => n.note)
  const H = g.height + (backs.length ? 30 + backs.length * 18 : 0)
  const near = (l: L) => !focus || l.from === focus || l.to === focus
  const pathOf = (l: L, i: number) => {
    if (!l.back) {
      let d = ''
      l.segs.forEach((s, j) => {
        const x0 = nx(s.from) + NODE_W, x1 = nx(s.to), m = (x0 + x1) / 2
        d += `${j ? ' L' : 'M'}${x0},${s.sy} C${m},${s.sy} ${m},${s.ty} ${x1},${s.ty}`
        if (j < l.segs.length - 1) d += ` L${x1 + NODE_W},${s.ty}`
      })
      return d
    }
    const x0 = nx(l.from) + NODE_W, x1 = nx(l.to)
    const by = g.height + 20 + i * 18
    return `M${x0},${l.sy} C${x0 + 30},${l.sy} ${x0 + 30},${by} ${x0},${by} L${x1},${by} C${x1 - 30},${by} ${x1 - 30},${l.ty} ${x1},${l.ty}`
  }
  const nodeName = (id: string) => {
    const n = g.all.get(id)
    if (!n) return id
    return n.kind === 'recipe' ? `${n.label} (${n.building})` : n.label
  }
  return (
    <div className="results">
      <p className="hint">
        {t('Band width is items per minute; a bar is as tall as the flow passing through it. Hover a bar to follow it.', '带宽表示每分钟物品数;柱子的高度等于流经的流量。鼠标悬停在柱子上可追踪流向。')}
        {backs.length === 1 && <> {t('One loop edge (an item feeding back into its own chain) is drawn as a thin dashed line below the chart.', '一条回环边 (物品回流到自己的产线) 以细虚线画在图下方。')}</>}
        {backs.length > 1 && <> {t(`${backs.length} loop edges (an item feeding back into its own chain) are drawn as thin dashed lines below the chart.`, `${backs.length} 条回环边 (物品回流到自己的产线) 以细虚线画在图下方。`)}</>}
        {noted && <> {t('↻ on a recipe marks what it makes and reuses itself; only the net amount is drawn.', '配方上的 ↻ 表示它自产自用的部分;图中只画净量。')}</>}
      </p>
      <div className="sankeywrap">
        <svg width={g.width} height={H} className="sankey" onMouseLeave={() => setFocus(null)}>
          {g.links.map((l, i) => (
            <path key={i} d={pathOf(l, backs.indexOf(l))} fill="none"
              strokeWidth={l.w}
              style={{ stroke: `hsl(${hue(l.item)} var(--sk-sat) var(--sk-link-l))`, strokeOpacity: near(l) ? (l.back ? 'calc(var(--sk-link-op) + 0.2)' : 'var(--sk-link-op)') : 0.06 }}
              strokeDasharray={l.back ? '6 4' : undefined}>
              <title>{`${fmt(l.rate)}/min ${names.get(l.item) ?? l.item}: ${nodeName(l.from)} → ${nodeName(l.to)}${l.back ? t(' (loop)', ' (回环)') : ''}`}</title>
            </path>
          ))}
          {backs.map((l, i) => {
            const a = nx(l.from), b = nx(l.to)
            return (
              <text key={'b' + i} className="backlabel" x={(Math.min(a, b) + Math.max(a, b)) / 2 + NODE_W}
                y={g.height + 16 + i * 18} textAnchor="middle" opacity={near(l) ? 1 : 0.3}>
                ↺ {fmt(l.rate)}/min {names.get(l.item) ?? l.item}
              </text>
            )
          })}
          {g.nodes.map(n => (
            <g key={n.id} onMouseEnter={() => setFocus(n.id)} className={`snode snode-${n.kind}`}>
              <rect x={nx(n.id)} y={n.y} width={NODE_W} height={n.h} rx={2}
                style={{ fill: n.kind === 'item' ? `hsl(${hue(n.icon ?? '')} var(--sk-sat) var(--sk-node-l))` : n.kind === 'recipe' ? 'var(--fg)' : 'var(--muted)' }} />
              <foreignObject x={nx(n.id) + NODE_W + 4} y={n.y + n.h / 2 - 22} width={COL - NODE_W - 10} height={44}>
                <div className={`slabel ${n.kind}`}>
                  {n.kind !== 'src' && n.kind !== 'sink' && <Icon id={n.icon} size={18} />}
                  <span className="slabeltext">
                    {n.kind === 'item' ? <><b>{n.label}</b> {fmt(n.value)}/min</>
                      : n.kind === 'recipe' ? <>{n.sub}<br /><span className="muted">{n.label}</span>
                        {n.note && <><br /><span className="loopnote">{n.note}</span></>}</>
                      : <span className="muted">{n.label} {fmt(n.value)}</span>}
                  </span>
                </div>
              </foreignObject>
              <title>{n.kind === 'recipe' ? `${n.sub} — ${n.label}${n.note ? `\n${n.note}` : ''}` : `${n.label}: ${fmt(n.value)}/min`}</title>
            </g>
          ))}
        </svg>
      </div>

      <h3>{t('Where each item goes', '各物品的去向')}</h3>
      <div className="tablewrap">
        <table className="balance">
          <thead><tr><th>{t('Item', '物品')}</th><th className="num">{t('Total /min', '合计 /min')}</th><th>{t('Made by', '产自')}</th><th>{t('Used by', '用于')}</th></tr></thead>
          <tbody>
            {balance.map(b => (
              <tr key={b.item}>
                <td><Icon id={b.item} />{b.name}{b.loop && <span className="tag" title={b.loop}>{t('loop', '回环')}</span>}</td>
                <td className="num">{fmt(b.total)}</td>
                <td className="small">{b.made.map(m => <div key={m.key}>{fmt(m.rate)} · {m.who}</div>)}</td>
                <td className="small">{b.used.map(m => <div key={m.key}>{fmt(m.rate)} · {m.who} <span className="muted">({fmt((100 * m.rate) / b.total, 0)}%)</span></div>)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

function itemBalance(plan: Plan, names: Map<string, string>, flows: RecipeFlow[], loops: Map<string, string>) {
  type Row = { key: string; who: string; rate: number }
  const made = new Map<string, Map<string, Row>>()
  const used = new Map<string, Map<string, Row>>()
  const add = (m: Map<string, Map<string, Row>>, it: string, key: string, who: string, q: number) => {
    if (q <= 1e-6) return
    if (!m.has(it)) m.set(it, new Map())
    const e = m.get(it)!.get(key)
    if (e) e.rate += q
    else m.get(it)!.set(key, { key, who, rate: q })
  }
  for (const f of flows) {
    for (const [it, q] of f.outs) add(made, it, f.id, f.who, q)
    for (const [it, q] of f.ins) add(used, it, f.id, f.who, q)
  }
  for (const l of plan.raw) add(made, l.item, 'mined', t('mined', '开采'), l.rate)
  for (const l of plan.supplied) add(made, l.item, 'supplied', t('supplied', '外部供应'), l.rate)
  for (const l of plan.targets) add(used, l.item, 'output', t('output', '产出'), l.rate)
  for (const l of plan.surplus) add(used, l.item, 'byproduct', t('byproduct', '副产物'), l.rate)
  const items = [...new Set([...made.keys(), ...used.keys()])]
  return items.map(it => {
    const m = [...(made.get(it)?.values() ?? [])].sort((a, b) => b.rate - a.rate)
    const u = [...(used.get(it)?.values() ?? [])].sort((a, b) => b.rate - a.rate)
    const total = Math.max(m.reduce((s, x) => s + x.rate, 0), u.reduce((s, x) => s + x.rate, 0))
    return { item: it, name: names.get(it) ?? it, total, made: m, used: u, loop: loops.get(it) }
  }).sort((a, b) => b.total - a.total)
}

/** Cancel an item a recipe both consumes and produces; returns what it recycles internally. */
function netSelfLoops(ins: Map<string, number>, outs: Map<string, number>) {
  const cyc: { item: string; rate: number }[] = []
  for (const [it, q] of ins) {
    const o = outs.get(it)
    if (o === undefined) continue
    const c = Math.min(q, o)
    cyc.push({ item: it, rate: c })
    if (q - c > 1e-9) ins.set(it, q - c); else ins.delete(it)
    if (o - c > 1e-9) outs.set(it, o - c); else outs.delete(it)
  }
  return cyc
}
