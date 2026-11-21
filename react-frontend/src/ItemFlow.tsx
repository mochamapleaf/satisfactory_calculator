import { useMemo, useState } from 'react'
import { Icon } from './Icon'
import { fmt, type GameData, type Plan } from './solver'

// Sankey of item flow: sources (mined / supplied) → items → recipes → items → … → outputs.
// Every item is a node whose size is its total rate, so "how much of X, and where does it go"
// reads directly off the chart. Recipe loops (an item feeding back into its own chain) are
// found with a DFS; those back edges are drawn as thin dashed return lines under the chart,
// labelled with their rate. A recipe that uses and makes the same item (DSP X-ray cracking,
// Kovarex) is shown with its net amount; the part it recycles internally is noted on the recipe.

type Kind = 'item' | 'recipe' | 'src' | 'sink'
interface N { id: string; kind: Kind; label: string; sub: string; icon?: string; value: number; drawn: number; layer: number; y: number; h: number; order: number }
interface L { from: string; to: string; item: string; rate: number; back: boolean; sy: number; ty: number; w: number }

const NODE_W = 12
const COL = 190
const GAP = 14
const PAD = 30

function hue(s: string) {
  let h = 0
  for (const c of s) h = (h * 31 + c.charCodeAt(0)) >>> 0
  return h % 360
}

function build(plan: Plan, names: Map<string, string>, scaleMax: number) {
  const nodes = new Map<string, N>()
  const links: L[] = []
  const node = (id: string, kind: Kind, label: string, sub = '', icon?: string) => {
    if (!nodes.has(id)) nodes.set(id, { id, kind, label, sub, icon, value: 0, drawn: 0, layer: 0, y: 0, h: 0, order: 0 })
    return id
  }
  const item = (it: string) => node('i:' + it, 'item', names.get(it) ?? it, '', it)
  const link = (from: string, to: string, it: string, rate: number) => {
    if (rate > 1e-6) links.push({ from, to, item: it, rate, back: false, sy: 0, ty: 0, w: 0 })
  }
  // one node per recipe (modes merged), exact rates from the plan
  const byRecipe = new Map<string, { name: string; building: string; machines: number; ins: Map<string, number>; outs: Map<string, number> }>()
  for (const r of plan.recipes) {
    const e = byRecipe.get(r.id) ?? { name: r.name, building: r.building, machines: 0, ins: new Map(), outs: new Map() }
    e.machines += r.machines
    for (const l of r.inputs) e.ins.set(l.item, (e.ins.get(l.item) ?? 0) + l.rate)
    for (const l of r.outputs) e.outs.set(l.item, (e.outs.get(l.item) ?? 0) + l.rate)
    byRecipe.set(r.id, e)
  }
  for (const [id, e] of byRecipe) {
    const cyc = netSelfLoops(e.ins, e.outs)
    const note = cyc.map(c => `↻ ${fmt(c.rate)} ${names.get(c.item) ?? c.item}`).join(', ')
    const rid = node('r:' + id, 'recipe', e.name, `${fmt(e.machines)}× ${e.building}`, [...e.outs.keys()][0])
    if (note) nodes.get(rid)!.label += ` · ${note}`
    for (const [it, q] of e.ins) link(item(it), rid, it, q)
    for (const [it, q] of e.outs) link(rid, item(it), it, q)
  }
  for (const l of plan.raw) link(node('raw:' + l.item, 'src', 'mined', '', undefined), item(l.item), l.item, l.rate)
  for (const l of plan.supplied) link(node('sup:' + l.item, 'src', 'supplied', '', undefined), item(l.item), l.item, l.rate)
  for (const l of plan.targets) link(item(l.item), node('out:' + l.item, 'sink', 'output', '', undefined), l.item, l.rate)
  for (const l of plan.surplus) link(item(l.item), node('extra:' + l.item, 'sink', 'byproduct', '', undefined), l.item, l.rate)

  // --- loops: DFS from sources, an edge into a node on the stack is a back edge
  const out = new Map<string, L[]>()
  const inc = new Map<string, L[]>()
  for (const l of links) {
    if (!out.has(l.from)) out.set(l.from, [])
    if (!inc.has(l.to)) inc.set(l.to, [])
    out.get(l.from)!.push(l)
    inc.get(l.to)!.push(l)
  }
  const state = new Map<string, 1 | 2>() // 1 on stack, 2 done
  const roots = [...nodes.values()].sort((a, b) => (inc.has(a.id) ? 1 : 0) - (inc.has(b.id) ? 1 : 0))
  for (const r of roots) {
    if (state.has(r.id)) continue
    const stack: [string, number][] = [[r.id, 0]]
    state.set(r.id, 1)
    while (stack.length) {
      const top = stack[stack.length - 1]
      const es = out.get(top[0]) ?? []
      if (top[1] >= es.length) { state.set(top[0], 2); stack.pop(); continue }
      const e = es[top[1]++]
      const s = state.get(e.to)
      if (s === 1) e.back = true
      else if (!s) { state.set(e.to, 1); stack.push([e.to, 0]) }
    }
  }

  // --- layers: longest path over forward edges; sinks go to the last column
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
  const maxLayer = Math.max(0, ...[...nodes.values()].map(n => n.layer))
  for (const n of nodes.values()) if (n.kind === 'sink') n.layer = maxLayer
  // sources sit just left of their item
  for (const n of nodes.values()) {
    if (n.kind !== 'src') continue
    const t = nodes.get((out.get(n.id) ?? [])[0]?.to ?? '')
    if (t) n.layer = Math.max(0, t.layer - 1)
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

  // --- vertical order: barycentre sweeps
  const layers: N[][] = Array.from({ length: maxLayer + 1 }, () => [])
  for (const n of nodes.values()) layers[n.layer].push(n)
  layers.forEach(l => l.forEach((n, i) => (n.order = i)))
  const bary = (n: N, dir: 'in' | 'out') => {
    const es = (dir === 'in' ? inc.get(n.id) : out.get(n.id)) ?? []
    let s = 0, w = 0
    for (const e of es) {
      if (e.back) continue
      const o = nodes.get(dir === 'in' ? e.from : e.to)!
      s += o.order * e.rate; w += e.rate
    }
    return w ? s / w : n.order
  }
  for (let it = 0; it < 6; it++) {
    const seq = it % 2 ? [...layers].reverse() : layers
    for (const l of seq) {
      const b = new Map(l.map(n => [n.id, bary(n, it % 2 ? 'out' : 'in')]))
      l.sort((a, c) => b.get(a.id)! - b.get(c.id)!)
      l.forEach((n, i) => (n.order = i))
    }
  }

  // --- sizes: one pixel scale for the whole chart
  const biggest = Math.max(1, ...layers.map(l => l.reduce((s, n) => s + n.drawn, 0)))
  const crowd = Math.max(...layers.map(l => l.length))
  const k = Math.min(scaleMax, Math.max(140, 34 * crowd)) / biggest
  let height = 0
  for (const l of layers) {
    let y = PAD
    for (const n of l) {
      n.h = Math.max(3, n.drawn * k)
      n.y = y
      y += n.h + GAP
    }
    height = Math.max(height, y)
  }
  // link attachment points, sorted so bands do not cross at the node
  for (const n of nodes.values()) {
    let y = n.y
    for (const l of [...(out.get(n.id) ?? [])].sort((a, b) => nodes.get(a.to)!.y - nodes.get(b.to)!.y)) {
      if (l.back) { l.w = 2; l.sy = n.y + n.h - 1; continue }
      l.w = Math.max(1, l.rate * k); l.sy = y + l.w / 2; y += l.rate * k
    }
    y = n.y
    for (const l of [...(inc.get(n.id) ?? [])].sort((a, b) => nodes.get(a.from)!.y - nodes.get(b.from)!.y)) {
      if (l.back) { l.ty = n.y + n.h - 1; continue }
      l.ty = y + Math.max(1, l.rate * k) / 2; y += l.rate * k
    }
  }
  return { nodes: [...nodes.values()], links, height, width: (maxLayer + 1) * COL + 160, inc, out }
}

export default function ItemFlow({ plan }: { data?: GameData; plan: Plan }) {
  const [focus, setFocus] = useState<string | null>(null)
  const names = useMemo(() => {
    const m = new Map<string, string>()
    for (const r of plan.recipes) for (const l of [...r.inputs, ...r.outputs]) m.set(l.item, l.name)
    for (const l of [...plan.raw, ...plan.supplied, ...plan.targets, ...plan.surplus]) m.set(l.item, l.name)
    return m
  }, [plan])
  const g = useMemo(() => build(plan, names, 520), [plan, names])
  const balance = useMemo(() => itemBalance(plan, names), [plan, names])
  if (plan.status !== 'optimal') return <p className="muted">Nothing to draw.</p>

  const x = (n: N) => 20 + n.layer * COL
  const backs = g.links.filter(l => l.back)
  const H = g.height + (backs.length ? 30 + backs.length * 18 : 0)
  const near = (l: L) => !focus || l.from === focus || l.to === focus
  const pathOf = (l: L, i: number) => {
    const s = g.nodes.find(n => n.id === l.from)!, t = g.nodes.find(n => n.id === l.to)!
    const x0 = x(s) + NODE_W, x1 = x(t)
    if (!l.back) {
      const m = (x0 + x1) / 2
      return `M${x0},${l.sy} C${m},${l.sy} ${m},${l.ty} ${x1},${l.ty}`
    }
    const by = g.height + 20 + i * 18
    return `M${x0},${l.sy} C${x0 + 30},${l.sy} ${x0 + 30},${by} ${x0},${by} L${x1},${by} C${x1 - 30},${by} ${x1 - 30},${l.ty} ${x1},${l.ty}`
  }
  const nodeName = (id: string) => {
    const n = g.nodes.find(m => m.id === id)
    return n ? (n.kind === 'recipe' ? n.label : n.kind === 'item' ? n.label : n.label) : id
  }
  return (
    <div className="results">
      <p className="hint">
        Band width is items per minute, and each item bar is as tall as its total rate. Hover a bar to follow it.
        {backs.length > 0 && <> {backs.length === 1 ? 'One loop edge' : `${backs.length} loop edges`} (an item feeding back into its own chain) {backs.length === 1 ? 'is' : 'are'} drawn as thin dashed lines below the chart.</>}
        {' '}↻ on a recipe marks what it makes and reuses itself; only the net amount is drawn.
      </p>
      <div className="sankeywrap">
        <svg width={g.width} height={H} className="sankey" onMouseLeave={() => setFocus(null)}>
          {g.links.map((l, i) => (
            <path key={i} d={pathOf(l, backs.indexOf(l))} fill="none"
              strokeWidth={l.w}
              style={{ stroke: `hsl(${hue(l.item)} var(--sk-sat) var(--sk-link-l))`, strokeOpacity: near(l) ? (l.back ? 'calc(var(--sk-link-op) + 0.2)' : 'var(--sk-link-op)') : 0.06 }}
              strokeDasharray={l.back ? '6 4' : undefined}>
              <title>{`${fmt(l.rate)}/min ${names.get(l.item) ?? l.item}: ${nodeName(l.from)} → ${nodeName(l.to)}${l.back ? ' (loop)' : ''}`}</title>
            </path>
          ))}
          {backs.map((l, i) => {
            const s = g.nodes.find(n => n.id === l.from)!, t = g.nodes.find(n => n.id === l.to)!
            return (
              <text key={'b' + i} className="backlabel" x={(Math.min(x(s), x(t)) + Math.max(x(s), x(t))) / 2 + NODE_W}
                y={g.height + 16 + i * 18} textAnchor="middle" opacity={near(l) ? 1 : 0.3}>
                ↺ {fmt(l.rate)}/min {names.get(l.item) ?? l.item}
              </text>
            )
          })}
          {g.nodes.map(n => (
            <g key={n.id} onMouseEnter={() => setFocus(n.id)} className={`snode snode-${n.kind}`}>
              <rect x={x(n)} y={n.y} width={NODE_W} height={n.h} rx={2}
                style={{ fill: n.kind === 'item' ? `hsl(${hue(n.icon ?? '')} var(--sk-sat) var(--sk-node-l))` : n.kind === 'recipe' ? 'var(--fg)' : 'var(--muted)' }} />
              <foreignObject x={x(n) + NODE_W + 4} y={n.y + n.h / 2 - 11} width={COL - NODE_W - 10} height={34}>
                <div className={`slabel ${n.kind}`}>
                  {n.kind !== 'src' && n.kind !== 'sink' && <Icon id={n.icon} size={18} />}
                  <span>
                    {n.kind === 'item' ? <><b>{n.label}</b> {fmt(n.value)}/min</>
                      : n.kind === 'recipe' ? <>{n.sub}<br /><span className="muted">{n.label}</span></>
                      : <span className="muted">{n.label} {fmt(n.value)}</span>}
                  </span>
                </div>
              </foreignObject>
              <title>{n.kind === 'recipe' ? `${n.sub} — ${n.label}` : `${n.label}: ${fmt(n.value)}/min`}</title>
            </g>
          ))}
        </svg>
      </div>

      <h3>Where each item goes</h3>
      <div className="tablewrap">
        <table className="balance">
          <thead><tr><th>Item</th><th className="num">Total /min</th><th>Made by</th><th>Used by</th></tr></thead>
          <tbody>
            {balance.map(b => (
              <tr key={b.item}>
                <td><Icon id={b.item} />{b.name}{b.loop && <span className="tag" title="also consumed upstream of its own producer">loop</span>}</td>
                <td className="num">{fmt(b.total)}</td>
                <td className="small">{b.made.map(m => <div key={m.who}>{fmt(m.rate)} · {m.who}</div>)}</td>
                <td className="small">{b.used.map(m => <div key={m.who}>{fmt(m.rate)} · {m.who} <span className="muted">({fmt((100 * m.rate) / b.total, 0)}%)</span></div>)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

function itemBalance(plan: Plan, names: Map<string, string>) {
  const made = new Map<string, Map<string, number>>()
  const used = new Map<string, Map<string, number>>()
  const add = (m: Map<string, Map<string, number>>, it: string, who: string, q: number) => {
    if (q <= 1e-6) return
    if (!m.has(it)) m.set(it, new Map())
    m.get(it)!.set(who, (m.get(it)!.get(who) ?? 0) + q)
  }
  for (const r of plan.recipes) {
    const who = `${r.name} (${r.building})`
    const ins = new Map<string, number>(), outs = new Map<string, number>()
    for (const l of r.inputs) ins.set(l.item, (ins.get(l.item) ?? 0) + l.rate)
    for (const l of r.outputs) outs.set(l.item, (outs.get(l.item) ?? 0) + l.rate)
    netSelfLoops(ins, outs)
    for (const [it, q] of outs) add(made, it, who, q)
    for (const [it, q] of ins) add(used, it, who, q)
  }
  for (const l of plan.raw) add(made, l.item, 'mined', l.rate)
  for (const l of plan.supplied) add(made, l.item, 'supplied', l.rate)
  for (const l of plan.targets) add(used, l.item, 'output', l.rate)
  for (const l of plan.surplus) add(used, l.item, 'byproduct', l.rate)
  // an item is in a loop when a recipe both makes it (directly or via its chain) and uses it;
  // cheap check: some recipe that uses it also appears among the producers of one of its inputs' makers
  const recipeOf = new Map(plan.recipes.map(r => [`${r.name} (${r.building})`, r]))
  const reach = (from: string, target: string) => {
    const seen = new Set<string>(); const st = [from]
    while (st.length) {
      const w = st.pop()!
      if (seen.has(w)) continue
      seen.add(w)
      const r = recipeOf.get(w)
      if (!r) continue
      for (const o of r.outputs) {
        if (o.item === target) return true
        for (const u of used.get(o.item)?.keys() ?? []) st.push(u)
      }
    }
    return false
  }
  const items = [...new Set([...made.keys(), ...used.keys()])]
  return items.map(it => {
    const m = [...(made.get(it) ?? [])].map(([who, rate]) => ({ who, rate })).sort((a, b) => b.rate - a.rate)
    const u = [...(used.get(it) ?? [])].map(([who, rate]) => ({ who, rate })).sort((a, b) => b.rate - a.rate)
    const total = Math.max(m.reduce((s, x) => s + x.rate, 0), u.reduce((s, x) => s + x.rate, 0))
    const loop = u.some(x => recipeOf.has(x.who) && reach(x.who, it))
    return { item: it, name: names.get(it) ?? it, total, made: m, used: u, loop }
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
