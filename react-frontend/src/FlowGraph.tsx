import { useMemo } from 'react'
import { Background, Controls, MarkerType, MiniMap, ReactFlow, type Edge, type Node, Position } from '@xyflow/react'
import dagre from '@dagrejs/dagre'
import { Icon } from './Icon'
import { carrier, dn, fmt, type GameData, type Plan } from './solver'

const W = 190
const H = 64

function nodeKind(id: string) {
  return id.split(':', 1)[0] as 'r' | 'raw' | 'sup' | 'out' | 'extra'
}

export default function FlowGraph({ data, plan }: { data: GameData; plan: Plan }) {
  const { nodes, edges } = useMemo(() => {
    const itemName = new Map(data.items.map(i => [i.id, dn(i, data.game)]))
    const bid = (name: string) => data.buildings.find(b => dn(b, data.game) === name || b.name === name)?.id
    const liquid = new Set(data.items.filter(i => i.liquid).map(i => i.id))
    const recipes = new Map(plan.recipes.map(r => [r.id, r]))
    const ids = new Set<string>()
    for (const f of plan.flows) { ids.add(f.from); ids.add(f.to) }

    const label = (id: string) => {
      const [kind, key] = [nodeKind(id), id.slice(id.indexOf(':') + 1)]
      if (kind === 'r') {
        const r = recipes.get(key)!
        return (
          <div>
            <div className="nodehead"><Icon id={r.outputs[0]?.item} size={24} /><Icon id={bid(r.building)} size={24} /><b>{fmt(r.machines)}× {r.building}</b></div>
            <div>{r.name}{r.alt ? ' (alt)' : ''}</div>
            <div className="muted">{fmt(r.power, 1)} MW{r.somersloops ? ` · ${r.somersloops} sloop` : ''}</div>
          </div>
        )
      }
      const name = itemName.get(key) ?? key
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
      const liq = liquid.has(f.item)
      return {
        id: `e${i}`,
        source: f.from,
        target: f.to,
        label: `${fmt(f.rate)} ${f.name}`,
        title: carrier(f.rate, liq, data),
        style: { strokeWidth: 1 + 5 * Math.sqrt(f.rate / maxRate), stroke: liq ? 'var(--liquid)' : 'var(--solid)' },
        labelStyle: { fontSize: 11 },
        labelBgStyle: { fill: 'var(--bg)' },
        markerEnd: { type: MarkerType.ArrowClosed },
      }
    })
    return { nodes, edges }
  }, [data, plan])

  if (plan.status !== 'optimal') return <p className="muted">Nothing to draw.</p>
  return (
    <div className="graph">
      {/* key: re-mount (and re-fit the view) when the set of nodes changes */}
      <ReactFlow key={nodes.map(n => n.id).join('|')} nodes={nodes} edges={edges} fitView minZoom={0.1} nodesDraggable proOptions={{ hideAttribution: true }}>
        <Background />
        <Controls />
        <MiniMap pannable zoomable />
      </ReactFlow>
      <p className="hint">Edges split each producer's output across consumers in proportion to demand. Blue = pipes. Drag nodes to tidy up.</p>
    </div>
  )
}
