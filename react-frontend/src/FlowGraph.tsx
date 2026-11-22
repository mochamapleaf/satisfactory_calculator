import { useMemo } from 'react'
import { Background, Controls, MarkerType, MiniMap, ReactFlow, type Edge, type Node, Position } from '@xyflow/react'
import dagre from '@dagrejs/dagre'
import { Icon } from './Icon'
import { carrier, dn, fmt, isLiquid, itemName as qItemName, type GameData, type Plan, type PlanRecipe } from './solver'

const W = 190
const H = 64

function nodeKind(id: string) {
  return id.split(':', 1)[0] as 'r' | 'raw' | 'sup' | 'out' | 'extra'
}

export default function FlowGraph({ data, plan }: { data: GameData; plan: Plan }) {
  const { nodes, edges } = useMemo(() => {
    const bid = (name: string) => data.buildings.find(b => dn(b, data.game) === name || b.name === name)?.id
    // one node per recipe id: the machines and MW of all its modes (loadouts) together
    const recipes = new Map<string, PlanRecipe>()
    const modes = new Map<string, number>()
    for (const r of plan.recipes) {
      const o = recipes.get(r.id)
      recipes.set(r.id, o ? { ...o, machines: o.machines + r.machines, power: o.power + r.power } : r)
      modes.set(r.id, (modes.get(r.id) ?? 0) + 1)
    }
    const ids = new Set<string>()
    for (const f of plan.flows) { ids.add(f.from); ids.add(f.to) }

    const label = (id: string) => {
      const [kind, key] = [nodeKind(id), id.slice(id.indexOf(':') + 1)]
      if (kind === 'r') {
        const r = recipes.get(key)!
        return (
          <div>
            <div className="nodehead"><Icon id={r.outputs[0]?.item} size={24} /><Icon id={bid(r.building)} size={24} /><b>{fmt(r.machines)}× {r.building}</b></div>
            {plan.quality && !!r.group_machines && modes.get(key) === 1 && r.group_machines - r.machines > 1e-6 && <div className="muted" title="The quality tiers of this recipe share these machines, which switch recipes">share of {fmt(r.group_machines, 0)} machines</div>}
            <div>{r.name}{r.alt ? ' (alt)' : ''}</div>
            <div className="muted">{fmt(r.power, 1)} MW{r.somersloops ? ` · ${r.somersloops} sloop` : ''}</div>
          </div>
        )
      }
      const name = qItemName(data, key)
      const line = (list: { item: string; rate: number }[]) => list.find(l => l.item === key)?.rate ?? 0
      const rate = kind === 'raw' ? line(plan.raw) : kind === 'sup' ? line(plan.supplied)
        : kind === 'out' ? line(plan.targets) : line(plan.surplus)
      const what = { raw: 'Raw', sup: 'Input', out: 'Output', extra: 'Byproduct', r: '' }[kind]
      return <div><div className="nodehead"><Icon id={key} size={24} /><b>{name}</b></div><div>{what} · {fmt(rate)}/min</div></div>
    }

    const g = new dagre.graphlib.Graph()
    g.setGraph({ rankdir: 'LR', nodesep: 24, ranksep: 110 })
    g.setDefaultEdgeLabel(() => ({}))
    for (const id of ids) g.setNode(id, { width: W, height: H })
    for (const f of plan.flows) g.setEdge(f.from, f.to)
    dagre.layout(g)

    const nodes: Node[] = [...ids].map(id => {
      const p = g.node(id)
      return {
        id,
        position: { x: p.x - W / 2, y: p.y - H / 2 },
        data: { label: label(id) },
        className: `fnode fnode-${nodeKind(id)}`,
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
        style: { width: W },
      }
    })
    const maxRate = Math.max(1, ...plan.flows.map(f => f.rate))
    const edges: Edge[] = plan.flows.map((f, i) => {
      const liq = isLiquid(data, f.item)
      return {
        id: `e${i}`,
        source: f.from,
        target: f.to,
        label: `${fmt(f.rate)} ${f.name}`,
        title: carrier(f.rate, liq, data),
        style: { strokeWidth: 1 + 5 * Math.sqrt(f.rate / maxRate), stroke: liq ? 'var(--liquid)' : 'var(--solid)' },
        labelStyle: { fontSize: 11, fontVariantNumeric: 'tabular-nums' },
        labelBgStyle: { fill: 'var(--bg)' },
        markerEnd: { type: MarkerType.ArrowClosed },
      }
    })
    return { nodes, edges }
  }, [data, plan])

  if (plan.status !== 'optimal') return <p className="muted">Nothing to draw.</p>
  return (
    <>
      <p className="hint">Each producer's output is split across its consumers in proportion to demand. Grey edges are belts, blue edges are pipes. Drag nodes to tidy up; scroll to zoom.</p>
      <div className="graph">
        {/* key: re-mount (and re-fit the view) when the set of nodes changes */}
        <ReactFlow key={nodes.map(n => n.id).join('|')} nodes={nodes} edges={edges} fitView minZoom={0.1} nodesDraggable colorMode="system" proOptions={{ hideAttribution: true }}>
          <Background />
          <Controls />
          <MiniMap pannable zoomable />
        </ReactFlow>
      </div>
    </>
  )
}
