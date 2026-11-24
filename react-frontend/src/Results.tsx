import { Icon, RemoveMark } from './Icon'
import { carrier, dn, fmt, isLiquid, qsplit, type GameData, type MachineGroup, type Plan, type PlanRecipe, type RateLine, type Settings } from './solver'

/** CPU time per tick: µs, or ms from 1000 µs. */
const fmtUs = (us: number) => (us >= 1000 ? `${fmt(us / 1000, 1)} ms` : `${fmt(us, us < 10 ? 1 : 0)} µs`)

const groupText = (g: MachineGroup) =>
  `${fmt(g.count)} × ${fmt(g.clock_percent, 2)}%${g.shards ? ` (${g.shards} shard${g.shards > 1 ? 's' : ''})` : ''}`

interface Props {
  data: GameData
  plan: Plan
  settings: Settings
  setSettings: (f: (s: Settings) => Settings) => void
}

export default function Results({ data, plan, settings: s, setSettings }: Props) {
  if (plan.status === 'empty') return <p className="muted">Add a target item to start a plan.</p>
  if (plan.status !== 'optimal') {
    return (
      <div className="warn">
        <b>{plan.status === 'infeasible' ? 'No feasible plan' : plan.status}</b>
        {plan.warnings.map(w => <p key={w}>{w}</p>)}
      </div>
    )
  }
  const liquid = (id: string) => isLiquid(data, id)
  const recipeById = new Map(data.recipes.map(r => [r.id, r]))
  const bid = (name: string) => data.buildings.find(b => dn(b, data.game) === name || b.name === name)?.id
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
  // quality plans: the normal-tier row excludes only that tier ("id@0"); the bare id would drop every tier
  const exclude = (rid: string) => {
    const key = plan.quality && qsplit(rid)[1] === 0 ? `${rid}@0` : rid
    setSettings(x => ({ ...x, disabled_recipes: [...x.disabled_recipes, key] }))
  }
  const totalSloops = plan.somersloops
  const whole = s.integer
  // quality plans: one row per shared group (base recipe × machine × loadout); the tiers it runs
  // switch recipes on the group's machines, listed as a tier split
  const rowGroups: PlanRecipe[][] = []
  {
    const at = new Map<string, PlanRecipe[]>()
    for (const r of plan.recipes) {
      const k = plan.quality ? `${qsplit(r.id)[0]}#${r.mode}` : `${r.id}#${r.mode}`
      const g = at.get(k)
      if (g) g.push(r)
      else { const n = [r]; at.set(k, n); rowGroups.push(n) }
    }
    for (const g of rowGroups) g.sort((a, b) => qsplit(a.id)[1] - qsplit(b.id)[1])
  }
  const tierName = (q: number) => (data.qualities?.[q] ?? ['normal', 'uncommon', 'rare', 'epic', 'legendary'][q] ?? '?').toLowerCase()
  const sumLines = (ls: RateLine[]) => {
    const m = new Map<string, RateLine>()
    for (const l of ls) { const o = m.get(l.item); if (o) o.rate += l.rate; else m.set(l.item, { ...l }) }
    return [...m.values()]
  }

  // group buildings by type
  // (quality plans: tiers of a recipe share machines, so the solver rounds up per base recipe and loadout)
  const byBuilding = new Map<string, number>()
  if (plan.building_counts?.length) for (const b of plan.building_counts) byBuilding.set(b.building, b.count)
  else for (const r of plan.recipes) byBuilding.set(r.building, (byBuilding.get(r.building) ?? 0) + (whole ? r.machines : r.machines_ceil))

  const limit = (item: string) => {
    if (s.ignore_limits) return null
    if (item in s.resource_limits) return s.resource_limits[item]
    return data.resources.find(r => r.item === item)?.limit ?? null
  }

  const sharedRow = (g: PlanRecipe[]) => {
              const r = g[0]
              const share = g.reduce((a, x) => a + x.machines, 0)
              const n = r.group_machines ?? Math.ceil(share - 1e-6)
              const base = r.name.replace(/ ★\S+$/, '')
              return (
                <tr key={qsplit(r.id)[0] + '#' + r.mode} className="shared">
                  <td><Icon id={qsplit(r.outputs[0]?.item ?? '')[0]} />{base}{r.alt && <span className="tag">alt</span>}<span className="tag" title="These quality tiers run on the same machines, which switch recipes">{g.length} tiers</span>{r.mode_label && <div className="muted small">{r.mode_label}</div>}</td>
                  <td><Icon id={bid(r.building)} />{r.building}</td>
                  <td className="num" title={`shared by ${g.length} tiers: ${fmt(share, 4)} machines of work`}>
                    <b>{fmt(n, whole ? 0 : 2)}</b>{!whole && <span className="muted"> ({fmt(share)})</span>}
                    {g.map(x => <div key={x.id} className="muted small">{tierName(qsplit(x.id)[1])} {fmt(x.machines)}</div>)}
                  </td>
                  <td className="small">{n > 0 ? `${fmt((100 * share) / n, 1)}% busy` : ''}</td>
                  <td><span className="muted">—</span></td>
                  <td className="num">{fmt(g.reduce((a, x) => a + x.power, 0), 1)}</td>
                  {plan.compute && <td className="num">{fmt((100 * g.reduce((a, x) => a + x.compute, 0)) / (plan.compute.total || 1), 1)}%</td>}
                  <td className="small">{sumLines(g.flatMap(x => x.inputs)).map(l => <div key={l.item}><Icon id={l.item} size={16} />{fmt(l.rate)} {l.name}</div>)}</td>
                  <td className="small">{sumLines(g.flatMap(x => x.outputs)).map(l => <div key={l.item}><Icon id={l.item} size={16} />{fmt(l.rate)} {l.name}</div>)}</td>
                  <td><button className="x" title="Exclude these tiers of the recipe and re-solve" aria-label={`Exclude ${base}`} onClick={() => setSettings(x => ({ ...x, disabled_recipes: [...x.disabled_recipes, ...g.map(y => qsplit(y.id)[1] === 0 ? `${y.id}@0` : y.id)] }))}><RemoveMark /></button></td>
                </tr>
              )
  }

  const generated = plan.recipes.reduce((a, r) => a + r.outputs.filter(o => o.item === 'Electricity').reduce((x, o) => x + o.rate, 0), 0)
  return (
    <div className="results">
      <div className="cards">
        <div className="card"><div className="big">{fmt(plan.buildings, 0)}</div><div className="muted">production buildings</div></div>
        <div className="card"><div className="big">{fmt(plan.production_power, 1)}<span className="unit">MW</span></div><div className="muted">production power</div></div>
        <div className="card"><div className="big">{fmt(plan.extraction_power, 1)}<span className="unit">MW</span></div><div className="muted">extraction, est.</div></div>
        <div className="card"><div className="big">{fmt(plan.production_power + plan.extraction_power, 1)}<span className="unit">MW</span></div><div className="muted">total power</div></div>
        {generated > 0 && <div className="card" title="Generators in this plan (fuel → Electricity). Self-powered plans cover their own production + extraction draw."><div className="big">{fmt(generated, 1)}<span className="unit">MW</span></div><div className="muted">generated</div></div>}
        {plan.shards > 0 && <div className="card"><div className="big">{fmt(plan.shards, 0)}</div><div className="muted">power shards</div></div>}
        {totalSloops > 0 && <div className="card"><div className="big">{fmt(totalSloops, 0)}</div><div className="muted">Somersloops</div></div>}
        {plan.compute && (
          <div className="card" title={`Whole buildings, working or idle; mining machines, orbital collectors and ray receivers are not counted. machines ${fmt(plan.compute.machines, 1)} · sorters ${fmt(plan.compute.sorters, 1)} · belts + stations ${fmt(plan.compute.logistics, 1)} · coaters ${fmt(plan.compute.coaters, 1)} · generation ${fmt(plan.compute.generation, 1)} (${fmt(plan.compute.cost_per_mw, 4)}/MW)`}>
            <div className="big">{fmt(plan.compute.total, plan.compute.total < 100 ? 1 : 0)}</div><div className="muted">compute units / tick <span className="nocase">(excl. mining, collectors)</span></div>
          </div>
        )}
        {plan.compute && (
          <div className="card" title="CPU time summed over threads; wall clock ≈ this ÷ effective threads. 60 UPS leaves 16 667 µs per tick. Order-of-magnitude estimate (report §2.1).">
            <div className="big">≈ {fmtUs(plan.compute.us_per_tick)}</div>
            <div className="muted">CPU / tick <span className="nocase">({fmtUs(plan.compute.us_low)}–{fmtUs(plan.compute.us_high)})</span></div>
          </div>
        )}
        <div className="card" title={`cost ${fmt(plan.objective, 3)} · continuous bound ${fmt(plan.lp_bound, 3)}${whole && plan.proven_optimal ? ' · optimal: within 0.01 % of the bound, and when buildings are a main cost (Buildings, Power, Compute) also less than one of its cheapest buildings above it' : ''}${whole && !plan.proven_optimal ? ` · within ${fmt(100 * (plan.gap ?? 0), 3)} % of the bound${plan.gap_buildings != null ? `, about ${fmt(plan.gap_buildings, 1)}× the weight of its cheapest building (a better plan may also trade resources or power)` : ''}` : ''}`}>
          <div className="big">{!whole ? 'LP' : plan.proven_optimal ? 'optimal' : plan.gap_buildings != null ? (plan.gap_buildings < 0.99 ? '≈ optimal' : `gap ≈ ${fmt(plan.gap_buildings, plan.gap_buildings < 10 ? 1 : 0)} bldg`) : (plan.gap ?? 1) < 0.0005 ? '≈ optimal' : `≤ ${fmt(100 * (plan.gap ?? 0), 1)}% gap`}</div>
          <div className="muted" title={plan.columns ? `${plan.columns} solver columns (recipe × machine × modules), ${plan.rows} item rows` : undefined}>
            {whole ? 'whole buildings' : 'fractional buildings'} · {fmt(plan.solve_ms, 0)} ms
          </div>

        </div>
      </div>
      {/* the time-limit warning is shown above the tabs with retry controls */}
      {plan.warnings.filter(w => !(s.integer && w.startsWith('Time limit reached'))).map(w => <p className="warn" key={w}>{w}</p>)}

      <h3>Buildings</h3>
      <ul className="chips">
        {[...byBuilding].map(([b, n]) => <li key={b}><Icon id={bid(b)} size={18} /><b>{n}×</b> {b}</li>)}
      </ul>

      <h3>Recipes</h3>
      <div className="tablewrap">
        <table>
          <thead>
            <tr><th>Recipe</th><th>Building</th><th className="num">Buildings</th><th>Clock</th><th>{data.game === 'satisfactory' ? 'Somersloops' : ''}</th><th className="num">MW</th>{plan.compute && <th className="num" title="Share of the plan's compute (units per tick incl. power)">算力</th>}<th>In (/min)</th><th>Out (/min)</th><th></th></tr>
          </thead>
          <tbody>
            {rowGroups.map(g => g.length > 1 ? sharedRow(g) : [g[0]].map(r => (
              <tr key={r.id + '#' + r.mode}>
                <td><Icon id={r.outputs[0]?.item} />{r.name}{r.alt && <span className="tag">alt</span>}{r.mode_label && <div className="muted small">{r.mode_label}</div>}</td>
                <td><Icon id={bid(r.building)} />{r.building}</td>
                <td className="num" title={`work = ${fmt(r.throughput, 4)} machines at 100 %`}>
                  {plan.quality && r.group_machines
                    ? (whole ? fmt(r.group_machines, 0) : <>{fmt(r.machines)} <span className="muted">→ {r.group_machines}</span></>)
                    : whole ? fmt(r.machines) : <>{fmt(r.machines)} <span className="muted">→ {r.machines_ceil}</span></>}
                </td>
                <td className="small">{plan.quality && r.group_machines
                  ? `${fmt((100 * r.machines) / r.group_machines, 1)}% busy`
                  : r.groups.map((g, i) => <div key={i}>{groupText(g)}</div>)}</td>
                <td>
                  {data.game === 'satisfactory' && slots(r.id) > 0
                    ? <select value={r.id in s.somersloops ? r.somersloops : 'auto'} onChange={e => setSloops(r.id, e.target.value === 'auto' ? -1 : +e.target.value)}>
                        {s.auto_somersloops && <option value="auto">auto ({r.somersloops})</option>}
                        {Array.from({ length: slots(r.id) + 1 }, (_, i) => <option key={i} value={i}>{i}/{slots(r.id)}</option>)}
                      </select>
                    : <span className="muted">—</span>}
                </td>
                <td className="num">{fmt(r.power, 1)}</td>
                {plan.compute && <td className="num" title={`${fmt(r.compute, 2)} units / tick`}>{fmt((100 * r.compute) / (plan.compute.total || 1), 1)}%</td>}
                <td className="small">{r.inputs.map(l => <div key={l.item}><Icon id={l.item} size={16} />{fmt(l.rate)} {l.name}</div>)}</td>
                <td className="small">{r.outputs.map(l => <div key={l.item}><Icon id={l.item} size={16} />{fmt(l.rate)} {l.name}</div>)}</td>
                <td><button className="x" title="Exclude this recipe and re-solve" aria-label={`Exclude ${r.name}`} onClick={() => exclude(r.id)}><RemoveMark /></button></td>
              </tr>
            ))[0])}
          </tbody>
        </table>
      </div>

      <div className="cols">
        <div>
          <h3>Raw resources</h3>
          <div className="tablewrap"><table>
            <thead><tr><th>Resource</th><th className="num">/min</th><th className="num">of world max</th><th>Carrier</th></tr></thead>
            <tbody>
              {plan.raw.map(l => {
                const lim = limit(l.item)
                return (
                  <tr key={l.item}>
                    <td><Icon id={l.item} />{l.name}</td>
                    <td className="num">{fmt(l.rate)}</td>
                    <td className="num">{lim ? `${fmt((100 * l.rate) / lim, 1)}%` : '—'}</td>
                    <td className="small">{l.item === 'Electricity' ? 'power lines' : carrier(l.rate, liquid(l.item), data)}</td>
                  </tr>
                )
              })}
              {plan.supplied.map(l => (
                <tr key={'s' + l.item}><td><Icon id={l.item} />{l.name} <span className="tag">input</span></td><td className="num">{fmt(l.rate)}</td><td /><td className="small">{l.item === 'Electricity' ? 'power lines' : carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
            </tbody>
          </table></div>
        </div>
        <div>
          <h3>Outputs</h3>
          <div className="tablewrap"><table>
            <thead><tr><th>Item</th><th className="num">/min</th><th>Carrier</th></tr></thead>
            <tbody>
              {plan.targets.map(l => (
                <tr key={l.item}><td><Icon id={l.item} />{l.name}</td><td className="num">{fmt(l.rate)}</td><td className="small">{l.item === 'Electricity' ? 'power lines' : carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
              {plan.surplus.map(l => (
                <tr key={'x' + l.item}><td><Icon id={l.item} />{l.name} <span className="tag">byproduct</span></td><td className="num">{fmt(l.rate)}</td><td className="small">{l.item === 'Electricity' ? 'power lines' : carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
            </tbody>
          </table></div>
        </div>
      </div>
    </div>
  )
}
