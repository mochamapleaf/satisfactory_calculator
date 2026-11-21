import { useMemo, useState } from 'react'
import { Icon, RemoveMark } from './Icon'
import { defaultSettings, dn, fmt, preset, scarcity, type CostWeights, type GameData, type ItemRate, type Objective, type Settings } from './solver'

interface Props {
  data: GameData
  settings: Settings
  setSettings: (f: (s: Settings) => Settings) => void
}

function ItemRates({ data, list, onChange, producibleOnly }: {
  data: GameData; list: ItemRate[]; onChange: (l: ItemRate[]) => void; producibleOnly?: boolean
}) {
  const produced = useMemo(() => new Set(data.recipes.flatMap(r => r.out.map(o => o[0]))), [data])
  const options = data.items.filter(i => !producibleOnly || produced.has(i.id) || i.raw)
  const [text, setText] = useState('')
  const byName = useMemo(() => new Map(data.items.flatMap(i => [[dn(i, data.game).toLowerCase(), i.id], [i.name.toLowerCase(), i.id]] as [string, string][])), [data])
  const name = (id: string) => dn(data.items.find(i => i.id === id), data.game, id)
  const add = (v: string) => {
    const id = byName.get(v.trim().toLowerCase())
    if (id && !list.some(t => t.item === id)) {
      onChange([...list, { item: id, rate: 10 }])
      setText('')
    } else setText(v)
  }
  return (
    <div>
      {list.map((t, i) => (
        <div className="row" key={t.item}>
          <span className="grow"><Icon id={t.item} />{name(t.item)}</span>
          <input type="number" min={0} step="any" value={t.rate}
            onChange={e => onChange(list.map((x, j) => j === i ? { ...x, rate: Math.max(0, +e.target.value) } : x))} />
          <span className="muted">/min</span>
          <button className="x" title={`Remove ${name(t.item)}`} aria-label={`Remove ${name(t.item)}`} onClick={() => onChange(list.filter((_, j) => j !== i))}><RemoveMark /></button>
        </div>
      ))}
      <input className="wide" list={producibleOnly ? 'items-prod' : 'items-all'} placeholder="Add an item…"
        value={text} onChange={e => add(e.target.value)} />
      <datalist id={producibleOnly ? 'items-prod' : 'items-all'}>
        {options.map(i => <option key={i.id} value={dn(i, data.game)} />)}
      </datalist>
    </div>
  )
}

function Num({ value, onChange, placeholder }: { value: number | undefined; onChange: (v: number | undefined) => void; placeholder?: string }) {
  return <input type="number" min={0} step="any" value={value ?? ''} placeholder={placeholder}
    onChange={e => onChange(e.target.value === '' ? undefined : Math.max(0, +e.target.value))} />
}

function CostEditor({ data, cost, onChange }: { data: GameData; cost: CostWeights; onChange: (c: CostWeights) => void }) {
  const set = (patch: Partial<CostWeights>) => onChange({ ...cost, ...patch })
  const terms: [keyof CostWeights, string, string][] = [
    ['resources', 'Raw resources', '× Σ weight · unit/min'],
    ['buildings', 'Buildings', '× Σ weight · building'],
    ['power', 'Power', 'per MW'],
    ['shards', 'Power shards', 'per shard'],
    ['surplus', 'Byproducts', 'per unit/min dumped'],
  ]
  const perMap = (key: 'resource_weights' | 'building_weights', id: string, v: number | undefined) => {
    const m = { ...cost[key] }
    if (v === undefined) delete m[id]
    else m[id] = v
    set({ [key]: m } as Partial<CostWeights>)
  }
  const usedBuildings = data.buildings.filter(b => data.recipes.some(r => r.building === b.id || r.producers?.includes(b.id)))
  return (
    <div className="costedit">
      {terms.map(([k, label, unit]) => (
        <div className="row small" key={k}>
          <span className="grow">{label} <span className="muted">{unit}</span></span>
          <Num value={cost[k] as number} onChange={v => set({ [k]: v ?? 0 } as Partial<CostWeights>)} />
        </div>
      ))}
      <details>
        <summary>Weight per raw resource</summary>
        {data.resources.map(r => (
          <div className="row small" key={r.item}>
            <span className="grow"><Icon id={r.item} size={16} />{dn(data.items.find(i => i.id === r.item), data.game, r.item)}</span>
            <Num value={cost.resource_weights[r.item]} placeholder={fmt((r as { weight?: number }).weight ?? scarcity(r.limit), 3)}
              onChange={v => perMap('resource_weights', r.item, v)} />
          </div>
        ))}
        <p className="hint">Blank = scarcity default (92 100 / world maximum).</p>
      </details>
      <details>
        <summary>Weight per building type</summary>
        {usedBuildings.map(b => (
          <div className="row small" key={b.id}>
            <span className="grow">{dn(b, data.game)}</span>
            <Num value={cost.building_weights[b.id]} placeholder="1" onChange={v => perMap('building_weights', b.id, v)} />
          </div>
        ))}
        <p className="hint">E.g. make Manufacturers cost 4 to prefer simpler buildings.</p>
      </details>
      <a href="#" onClick={e => { e.preventDefault(); onChange(preset('resources')) }}>Reset weights</a>
    </div>
  )
}

export default function Sidebar({ data, settings: s, setSettings }: Props) {
  const set = (patch: Partial<Settings>) => setSettings(x => ({ ...x, ...patch }))
  const [altFilter, setAltFilter] = useState('')
  const alts = data.recipes.filter(r => r.alt && r.name.toLowerCase().includes(altFilter.toLowerCase()))
  const altEnabled = (id: string) => s.allow_all_alternates || s.enabled_alternates.includes(id)
  const toggleAlt = (id: string) => {
    if (s.allow_all_alternates) {
      // switch to explicit list with everything but this one
      set({ allow_all_alternates: false, enabled_alternates: data.recipes.filter(r => r.alt && r.id !== id).map(r => r.id) })
    } else {
      set({ enabled_alternates: s.enabled_alternates.includes(id) ? s.enabled_alternates.filter(x => x !== id) : [...s.enabled_alternates, id] })
    }
  }
  const itemName = (id: string) => dn(data.items.find(i => i.id === id), data.game, id)
  const sf = data.game === 'satisfactory'
  // buildings that compete for some recipe (machine tiers)
  const tiers = data.buildings.filter(b => data.recipes.some(r => (r.producers?.length ?? 0) > 1 && r.producers!.includes(b.id)))
  const toggle = (list: string[], id: string) => (list.includes(id) ? list.filter(x => x !== id) : [...list, id])
  const limitOf = (item: string, dflt: number | null) => (item in s.resource_limits ? s.resource_limits[item] : dflt)

  return (
    <aside>
      <section>
        <h2>Targets</h2>
        <ItemRates data={data} list={s.targets} onChange={targets => set({ targets })} producibleOnly />
      </section>

      <section>
        <h2>Optimise for</h2>
        <div className="seg">
          {(['resources', 'buildings', 'power', 'custom'] as Objective[]).map(o => (
            <button key={o} className={s.objective === o ? 'on' : ''}
              onClick={() => set(o === 'custom' && s.objective !== 'custom'
                ? { objective: o, cost: { ...preset(s.objective), resource_weights: s.cost.resource_weights, building_weights: s.cost.building_weights } }
                : { objective: o })}>
              {o === 'resources' ? 'Raw' : o === 'buildings' ? 'Buildings' : o === 'power' ? 'Power' : 'Custom'}
            </button>
          ))}
        </div>
        <p className="hint">
          {s.objective === 'resources' && (sf ? 'Raw resources weighted by scarcity (Iron Ore = 1, uranium ≈ 44); buildings break ties.' : 'Raw resources (ores 1, water 0.01, oil 0.2, drops 10); buildings break ties.')}
          {s.objective === 'buildings' && 'Fewest whole buildings; raw resources break ties.'}
          {s.objective === 'power' && 'MW (production + extraction) + 1 per building, so it underclocks where that pays.'}
          {s.objective === 'custom' && 'Minimise the weighted sum below. Edit any weight; the plan re-solves.'}
        </p>
        {s.objective === 'custom' && <CostEditor data={data} cost={s.cost} onChange={cost => set({ cost })} />}
      </section>

      <section>
        <h2>Solver</h2>
        <label className="row">
          <input type="checkbox" checked={s.integer} onChange={e => set({ integer: e.target.checked })} />
          <span>Whole buildings and shards (integer program)</span>
        </label>
        {s.integer && (
          <div className="row small">
            <span className="grow">Time limit</span>
            <input type="number" min={0.2} max={120} step={0.5} value={s.time_limit_ms / 1000}
              onChange={e => set({ time_limit_ms: Math.round(Math.min(120, Math.max(0.2, +e.target.value)) * 1000) })} />
            <span className="muted">s</span>
          </div>
        )}
        <p className="hint">
          {s.integer
            ? 'Branch-and-bound over whole buildings; each building can be underclocked freely. On timeout the best plan found is shown with its gap to the lower bound.'
            : 'Continuous LP: fractional buildings (the old "solve, then round up" view).'}
        </p>
        {sf && <>
        <div className="row">
          <span>Max clock</span>
          <input type="range" min={1} max={250} value={s.max_clock_percent} className="grow" aria-label="Max clock"
            onChange={e => set({ max_clock_percent: +e.target.value })} />
          <input type="number" min={1} max={250} value={s.max_clock_percent}
            onChange={e => set({ max_clock_percent: Math.min(250, Math.max(1, +e.target.value)) })} />
          <span className="muted">%</span>
        </div>
        {s.max_clock_percent > 100 && (
          <div className="row small">
            <span className="grow">Power shards available</span>
            <input type="number" min={0} placeholder="∞" value={s.shard_budget ?? ''}
              onChange={e => set({ shard_budget: e.target.value === '' ? null : Math.max(0, Math.round(+e.target.value)) })} />
          </div>
        )}
        <p className="hint">Above 100 % a building needs one shard per started 50 %. Power ∝ clock<sup>1.32</sup>.</p>
        <label className="row">
          <input type="checkbox" checked={s.auto_somersloops} onChange={e => set({ auto_somersloops: e.target.checked })} />
          <span>Place Somersloops automatically</span>
        </label>
        <div className="row small">
          <span className="grow">Somersloops available</span>
          <input type="number" min={0} placeholder="∞" value={s.somersloop_budget ?? ''}
            onChange={e => set({ somersloop_budget: e.target.value === '' ? null : Math.max(0, Math.round(+e.target.value)) })} />
        </div>
        <p className="hint">Each doubles a building's output at 4× its power. Fix a recipe's count in the plan table; the rest are placed within the budget.</p>
        </>}
      </section>

      {!sf && (
        <section>
          <h2>Machines</h2>
          <p className="hint">Tiers the solver may use; it picks per recipe by the cost function.</p>
          <div className="scroll">
            {tiers.map(b => (
              <label className="row small" key={b.id}>
                <input type="checkbox" checked={!s.disabled_buildings.includes(b.id)}
                  onChange={() => set({ disabled_buildings: toggle(s.disabled_buildings, b.id) })} />
                <span className="grow"><Icon id={b.id} />{dn(b, data.game)}</span>
                <span className="muted">×{b.speed ?? 1}</span>
              </label>
            ))}
          </div>
        </section>
      )}

      {!sf && (data.effects?.length ?? 0) > 0 && (
        <section>
          <h2>{data.game === 'dsp' ? 'Proliferator' : 'Modules'}</h2>
          <p className="hint">
            {data.game === 'dsp'
              ? 'Enabled modes become options per recipe; spray use (one per input item) is added as an input, so the proliferator chain is planned too. Add it under Available inputs to treat it as free.'
              : 'Each enabled module fills all slots of a machine (mixed loadouts and beacons: later). Productivity is capped at +300 % and only on allowed recipes.'}
          </p>
          {data.effects!.map(e => (
            <label className="row small" key={e.id}>
              <input type="checkbox" checked={s.effects.includes(e.id)} onChange={() => set({ effects: toggle(s.effects, e.id) })} />
              <span className="grow"><Icon id={e.item || e.id} />{dn(e, data.game)}</span>
              <span className="muted">
                {e.productivity ? `+${fmt(100 * e.productivity)}% out ` : ''}{e.speed ? `${e.speed > 0 ? '+' : ''}${fmt(100 * e.speed)}% speed ` : ''}
              </span>
            </label>
          ))}
        </section>
      )}

      <section>
        <h2>Alternate recipes</h2>
        <label className="row">
          <input type="checkbox" checked={s.allow_all_alternates}
            onChange={e => set({ allow_all_alternates: e.target.checked, enabled_alternates: [] })} />
          <span>Allow all {data.recipes.filter(r => r.alt).length} alternates</span>
        </label>
        {sf && <label className="row">
          <input type="checkbox" checked={s.allow_ore_conversion} onChange={e => set({ allow_ore_conversion: e.target.checked })} />
          <span>Converter ore conversion (e.g. Bauxite from Copper)</span>
        </label>}
        <details>
          <summary>Choose individually ({s.allow_all_alternates ? 'all' : s.enabled_alternates.length} on)</summary>
          <input className="wide" placeholder="Filter by name" value={altFilter} onChange={e => setAltFilter(e.target.value)} />
          <div className="scroll">
            {alts.map(r => (
              <label className="row small" key={r.id}>
                <input type="checkbox" checked={altEnabled(r.id)} onChange={() => toggleAlt(r.id)} />
                <span>{dn(r, data.game)}</span>
              </label>
            ))}
          </div>
        </details>
        {s.disabled_recipes.length > 0 && (
          <p className="hint">
            {s.disabled_recipes.length} recipe(s) excluded in the plan table.{' '}
            <a href="#" onClick={e => { e.preventDefault(); set({ disabled_recipes: [] }) }}>Re-enable all</a>
          </p>
        )}
      </section>

      <section>
        <h2>Available inputs</h2>
        <p className="hint">Items you already produce elsewhere (free, up to the rate).</p>
        <ItemRates data={data} list={s.supplies} onChange={supplies => set({ supplies })} />
      </section>

      {sf && <section>
        <h2>Resource limits</h2>
        <label className="row">
          <input type="checkbox" checked={s.ignore_limits} onChange={e => set({ ignore_limits: e.target.checked })} />
          <span>Ignore limits</span>
        </label>
        <details>
          <summary>World maximum per minute (editable)</summary>
          {data.resources.map(r => {
            const v = limitOf(r.item, r.limit)
            return (
              <div className="row small" key={r.item}>
                <span className="grow">{itemName(r.item)}</span>
                <input type="number" min={0} placeholder="∞" value={v ?? ''}
                  onChange={e => set({ resource_limits: { ...s.resource_limits, [r.item]: e.target.value === '' ? null : +e.target.value } })} />
              </div>
            )
          })}
          <a href="#" onClick={e => { e.preventDefault(); set({ resource_limits: {} }) }}>Reset to world maxima</a>
        </details>
      </section>}

      <section>
        <button className="wide" onClick={() => setSettings(() => defaultSettings(s.game))}>Reset all settings for this game</button>
        <p className="hint">The page address holds the whole plan. Copy it to share or bookmark this plan.</p>
      </section>
    </aside>
  )
}
