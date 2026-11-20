import { carrier, fmt, type GameData, type MachineGroup, type Plan, type Settings } from './solver'

const groupText = (g: MachineGroup) =>
  `${fmt(g.count)} × ${fmt(g.clock_percent, 2)}%${g.shards ? ` (${g.shards} shard${g.shards > 1 ? 's' : ''})` : ''}`

interface Props {
  data: GameData
  plan: Plan
  settings: Settings
  setSettings: (f: (s: Settings) => Settings) => void
}

export default function Results({ data, plan, settings: s, setSettings }: Props) {
  if (plan.status === 'empty') return <p className="muted">Add a target item on the left.</p>
  if (plan.status !== 'optimal') {
    return (
      <div className="warn">
        <b>{plan.status === 'infeasible' ? 'No feasible plan' : plan.status}</b>
        {plan.warnings.map(w => <p key={w}>{w}</p>)}
      </div>
    )
  }
  const liquid = (id: string) => data.items.find(i => i.id === id)?.liquid ?? false
  const recipeById = new Map(data.recipes.map(r => [r.id, r]))
  const slots = (rid: string) => {
    const b = recipeById.get(rid)?.building
    return data.buildings.find(x => x.id === b)?.somersloop_slots ?? 0
  }
  const setSloops = (rid: string, n: number) =>
    setSettings(x => {
      const somersloops = { ...x.somersloops }
      // n < 0: let the solver choose (auto mode); 0 is a fixed "none" only in auto mode
      if (n > 0 || (n === 0 && x.auto_somersloops)) somersloops[rid] = n
      else delete somersloops[rid]
      return { ...x, somersloops }
    })
  const exclude = (rid: string) => setSettings(x => ({ ...x, disabled_recipes: [...x.disabled_recipes, rid] }))
  const totalSloops = plan.somersloops

  // group buildings by type
  const byBuilding = new Map<string, number>()
  for (const r of plan.recipes) byBuilding.set(r.building, (byBuilding.get(r.building) ?? 0) + (s.integer ? r.machines : r.machines_ceil))

  const limit = (item: string) => {
    if (s.ignore_limits) return null
    if (item in s.resource_limits) return s.resource_limits[item]
    return data.resources.find(r => r.item === item)?.limit ?? null
  }

  return (
    <div className="results">
      <div className="cards">
        <div className="card"><div className="big">{fmt(plan.buildings, 0)}</div><div className="muted">production buildings</div></div>
        <div className="card"><div className="big">{fmt(plan.production_power, 1)} MW</div><div className="muted">production power</div></div>
        <div className="card"><div className="big">{fmt(plan.extraction_power, 1)} MW</div><div className="muted">extraction (estimate)</div></div>
        <div className="card"><div className="big">{fmt(plan.production_power + plan.extraction_power, 1)} MW</div><div className="muted">total</div></div>
        {plan.shards > 0 && <div className="card"><div className="big">{fmt(plan.shards, 0)}</div><div className="muted">power shards</div></div>}
        {totalSloops > 0 && <div className="card"><div className="big">{fmt(totalSloops, 0)}</div><div className="muted">Somersloops</div></div>}
        <div className="card" title={`cost ${fmt(plan.objective, 3)} · continuous bound ${fmt(plan.lp_bound, 3)}`}>
          <div className="big">{!s.integer ? 'LP' : plan.proven_optimal ? 'optimal' : (plan.gap ?? 1) < 0.0005 ? '≈ optimal' : `≤ ${fmt(100 * (plan.gap ?? 0), 1)}% gap`}</div>
          <div className="muted">{s.integer ? 'whole buildings' : 'fractional buildings'} · {fmt(plan.solve_ms, 0)} ms</div>
        </div>
      </div>
      {plan.warnings.map(w => <p className="warn" key={w}>{w}</p>)}

      <h3>Buildings</h3>
      <p>{[...byBuilding].map(([b, n]) => `${n}× ${b}`).join(' · ')}</p>

      <h3>Recipes</h3>
      <div className="tablewrap">
        <table>
          <thead>
            <tr><th>Recipe</th><th>Building</th><th className="num">Buildings</th><th>Clock</th><th>{data.game === 'satisfactory' ? 'Somersloops' : ''}</th><th className="num">MW</th><th>In (/min)</th><th>Out (/min)</th><th></th></tr>
          </thead>
          <tbody>
            {plan.recipes.map(r => (
              <tr key={r.id + '#' + r.mode}>
                <td>{r.name}{r.alt && <span className="tag">alt</span>}{r.mode_label && <div className="muted small">{r.mode_label}</div>}</td>
                <td>{r.building}</td>
                <td className="num" title={`work = ${fmt(r.throughput, 4)} machines at 100 %`}>
                  {s.integer ? fmt(r.machines) : <>{fmt(r.machines)} <span className="muted">→ {r.machines_ceil}</span></>}
                </td>
                <td className="small">{r.groups.map((g, i) => <div key={i}>{groupText(g)}</div>)}</td>
                <td>
                  {data.game === 'satisfactory' && slots(r.id) > 0
                    ? <select value={r.id in s.somersloops ? r.somersloops : 'auto'} onChange={e => setSloops(r.id, e.target.value === 'auto' ? -1 : +e.target.value)}>
                        {s.auto_somersloops && <option value="auto">auto ({r.somersloops})</option>}
                        {Array.from({ length: slots(r.id) + 1 }, (_, i) => <option key={i} value={i}>{i}/{slots(r.id)}</option>)}
                      </select>
                    : <span className="muted">—</span>}
                </td>
                <td className="num">{fmt(r.power, 1)}</td>
                <td className="small">{r.inputs.map(l => <div key={l.item}>{fmt(l.rate)} {l.name}</div>)}</td>
                <td className="small">{r.outputs.map(l => <div key={l.item}>{fmt(l.rate)} {l.name}</div>)}</td>
                <td><button className="x" title="Exclude this recipe and re-solve" onClick={() => exclude(r.id)}>×</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="cols">
        <div>
          <h3>Raw resources</h3>
          <table>
            <thead><tr><th>Resource</th><th className="num">/min</th><th className="num">of world max</th><th>Carrier</th></tr></thead>
            <tbody>
              {plan.raw.map(l => {
                const lim = limit(l.item)
                return (
                  <tr key={l.item}>
                    <td>{l.name}</td>
                    <td className="num">{fmt(l.rate)}</td>
                    <td className="num">{lim ? `${fmt((100 * l.rate) / lim, 1)}%` : '—'}</td>
                    <td className="small">{carrier(l.rate, liquid(l.item), data)}</td>
                  </tr>
                )
              })}
              {plan.supplied.map(l => (
                <tr key={'s' + l.item}><td>{l.name} <span className="tag">input</span></td><td className="num">{fmt(l.rate)}</td><td /><td className="small">{carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
            </tbody>
          </table>
        </div>
        <div>
          <h3>Outputs</h3>
          <table>
            <thead><tr><th>Item</th><th className="num">/min</th><th>Carrier</th></tr></thead>
            <tbody>
              {plan.targets.map(l => (
                <tr key={l.item}><td>{l.name}</td><td className="num">{fmt(l.rate)}</td><td className="small">{carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
              {plan.surplus.map(l => (
                <tr key={'x' + l.item}><td>{l.name} <span className="tag">byproduct</span></td><td className="num">{fmt(l.rate)}</td><td className="small">{carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
