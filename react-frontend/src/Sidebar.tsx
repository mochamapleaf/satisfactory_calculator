import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Icon, QualityPip, RemoveMark } from './Icon'
import ItemPicker from './ItemPicker'
import { DATASETS, defaultSettings, withDataset, dn, fmt, GENERATORS, itemName as qItemName, preset, qsplit, scarcity, withQ, type CostWeights, type Generator, type GameData, type ItemRate, type Objective, type Settings } from './solver'

interface Props {
  data: GameData
  settings: Settings
  setSettings: (f: (s: Settings) => Settings) => void
}

/** Quality tier select (Factorio data sets with quality). */
function QualitySelect({ data, value, onChange, label }: { data: GameData; value: number; onChange: (q: number) => void; label: string }) {
  const names = data.qualities ?? []
  return (
    <span className="qselwrap"><QualityPip q={value} />
      <select className="qsel" value={value} aria-label={label} title={label} onChange={e => onChange(+e.target.value)}>
        {names.map((n, q) => <option key={q} value={q}>{n}</option>)}
      </select>
    </span>
  )
}

function ItemRates({ data, list, onChange, producibleOnly, quality }: {
  data: GameData; list: ItemRate[]; onChange: (l: ItemRate[]) => void; producibleOnly?: boolean
  /** show a quality select per row (Factorio data with quality) */
  quality?: boolean
}) {
  const produced = useMemo(() => new Set(data.recipes.flatMap(r => r.out.map(o => o[0]))), [data])
  const options = useMemo(() => data.items.filter(i => !producibleOnly || produced.has(i.id) || i.raw), [data, produced, producibleOnly])
  // which picker is open: a row index (replace that item) or 'add'
  const [open, setOpen] = useState<number | 'add' | null>(null)
  const scope = useRef<HTMLDivElement>(null)
  // after a pick or Esc, focus goes back to the button that opened the picker (it may have been re-rendered)
  const refocus = useRef<number | 'add' | null>(null)
  useEffect(() => {
    if (refocus.current === null) return
    scope.current?.querySelector<HTMLElement>(`[data-opener="${refocus.current}"]`)?.focus()
    refocus.current = null
  })
  // Picker ids are base items; a row keeps its quality when its item changes. An item is taken
  // at the quality the pick would use (new rows are normal), so with quality the same item can
  // be listed at several qualities (normal + legendary gears).
  const liquid = (id: string) => data.items.find(i => i.id === qsplit(id)[0])?.liquid ?? false
  const atQ = (id: string, q: number) => (liquid(id) ? qsplit(id)[0] : withQ(id, q))
  const pickQ = open === null || open === 'add' ? 0 : qsplit(list[open]?.item ?? '')[1]
  const listed = new Set(list.map(t => t.item))
  const taken = new Set(list.map(t => qsplit(t.item)[0]).filter(b => listed.has(atQ(b, pickQ))))
  const name = (id: string) => qItemName(data, id)
  const pick = (id: string) => {
    // items already in the list are disabled in the picker, so a pick never duplicates or drops a row
    if (open === 'add') {
      if (!taken.has(id)) onChange([...list, { item: id, rate: 10 }])
    } else if (open !== null && (!taken.has(id) || qsplit(list[open].item)[0] === id)) {
      onChange(list.map((t, j) => (j === open ? { ...t, item: atQ(id, qsplit(t.item)[1]) } : t)))
    }
    refocus.current = open
    setOpen(null)
  }
  const close = useCallback((how: 'esc' | 'away') => {
    setOpen(o => { if (how === 'esc') refocus.current = o; return null })
  }, [])
  const picker = (current?: string) => (
    <ItemPicker data={data} items={options} current={current} taken={taken} scope={scope} onPick={pick} onClose={close} />
  )
  return (
    <div ref={scope}>
      {list.map((t, i) => (
        <div key={t.item}>
          <div className="row">
            <button type="button" className="swap grow" title={`Change ${name(t.item)}`} data-opener={i}
              aria-expanded={open === i} onClick={() => setOpen(open === i ? null : i)}>
              <Icon id={t.item} />{name(t.item)}
            </button>
            <input type="number" min={0} step="any" value={t.rate} aria-label={`${name(t.item)} per minute`}
              onChange={e => onChange(list.map((x, j) => j === i ? { ...x, rate: Math.max(0, +e.target.value) } : x))} />
            <span className="muted">/min</span>
            <button className="x" title={`Remove ${name(t.item)}`} aria-label={`Remove ${name(t.item)}`} onClick={() => onChange(list.filter((_, j) => j !== i))}><RemoveMark /></button>
          </div>
          {quality && !liquid(t.item) && (
            <div className="row small qrow">
              <span className="grow muted">Quality</span>
              <QualitySelect data={data} value={qsplit(t.item)[1]} label={`Quality of ${name(t.item)}`}
                onChange={q => {
                  // a quality that is already listed merges into that row
                  const item = withQ(t.item, q)
                  const k = list.findIndex((x, j) => j !== i && x.item === item)
                  onChange(k < 0 ? list.map((x, j) => j === i ? { ...x, item } : x)
                    : list.flatMap((x, j) => j === i ? [] : j === k ? [{ ...x, rate: x.rate + t.rate }] : [x]))
                }} />
            </div>
          )}
          {open === i && picker(qsplit(t.item)[0])}
        </div>
      ))}
      <button type="button" className="addpick" aria-expanded={open === 'add'} data-opener="add"
        onClick={() => setOpen(open === 'add' ? null : 'add')}>+ Add an item</button>
      {open === 'add' && picker()}
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
    ...(data.game === 'dsp' ? [
      ['sorter', 'Sorters', 'per port of each building'],
      ['per_machine', 'Belts + stations', 'per building'],
      ['coater', 'Spray coaters', 'per sprayed building'],
    ] as [keyof CostWeights, string, string][] : []),
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

/** Items whose recycling is allowed beyond the scope (chips + item picker). */
function AllowList({ data, list, onChange }: { data: GameData; list: string[]; onChange: (l: string[]) => void }) {
  const [open, setOpen] = useState(false)
  const scope = useRef<HTMLDivElement>(null)
  const recyclable = useMemo(() => {
    const ids = new Set(data.recipes.filter(r => r.recycling).map(r => r.in[0]?.[0]))
    return data.items.filter(i => ids.has(i.id))
  }, [data])
  return (
    <div ref={scope}>
      {list.length > 0 && <ul className="chips">
        {list.map(id => (
          <li key={id}><Icon id={id} size={16} />{qItemName(data, id)}
            <button className="x" aria-label={`Stop recycling ${qItemName(data, id)}`} title="Remove" onClick={() => onChange(list.filter(x => x !== id))}><RemoveMark /></button>
          </li>
        ))}
      </ul>}
      <button type="button" className="addpick" aria-expanded={open} onClick={() => setOpen(!open)}>+ Allow recycling of an item</button>
      {open && <ItemPicker data={data} items={recyclable} taken={new Set(list)} scope={scope}
        onPick={id => { if (!list.includes(id)) onChange([...list, id]); setOpen(false) }} onClose={() => setOpen(false)} />}
    </div>
  )
}

function QualitySettings({ data, s, set }: { data: GameData; s: Settings; set: (p: Partial<Settings>) => void }) {
  const mods = data.effects ?? []
  const qmods = mods.filter(e => (e.by_quality?.[0]?.quality ?? e.quality ?? 0) > 0)
  const pmods = mods.filter(e => e.kind === 'module' && e.productivity > 0)
  const tier = (id: string) => mods.find(e => e.id === id)?.by_quality?.[s.module_quality]
  const qv = tier(s.quality_module)?.quality ?? 0
  const pv = tier(s.prod_module)?.productivity ?? 0
  const active = s.targets.some(t => qsplit(t.item)[1] > 0) || s.supplies.some(t => qsplit(t.item)[1] > 0)
  return (
    <section>
      <h2>Quality</h2>
      <p className="hint">
        {active
          ? 'Every recipe on the chain runs per input quality; the solver picks quality / productivity modules per machine and where to recycle. Solved as an LP (fractional machines).'
          : 'Pick a quality above normal on a target to plan an upcycling loop.'}
      </p>
      <div className="row small">
        <span className="grow">Module quality</span>
        <QualitySelect data={data} value={s.module_quality} label="Quality of all modules" onChange={module_quality => set({ module_quality })} />
      </div>
      <div className="row small">
        <span className="grow">Quality slots</span>
        <select value={s.quality_module} aria-label="Quality module" onChange={e => set({ quality_module: e.target.value })}>
          <option value="">none</option>
          {qmods.map(e => <option key={e.id} value={e.id}>{dn(e, data.game)}</option>)}
        </select>
        <span className="muted">{qv ? `+${fmt(100 * qv, 2)}%` : ''}</span>
      </div>
      <div className="row small">
        <span className="grow">Other slots</span>
        <select value={s.prod_module} aria-label="Productivity module" onChange={e => set({ prod_module: e.target.value })}>
          <option value="">empty</option>
          {pmods.map(e => <option key={e.id} value={e.id}>{dn(e, data.game)}</option>)}
        </select>
        <span className="muted">{pv ? `+${fmt(100 * pv, 2)}%` : ''}</span>
      </div>
      <div className="row small"><span className="grow">Recycle</span></div>
      <div className="seg" role="group" aria-label="Recycling scope">
        {([['chain', 'Target chain'], ['list', 'Only listed'], ['all', 'Everything']] as const).map(([id, label]) => (
          <button key={id} className={s.recycling === id ? 'on' : ''} aria-pressed={s.recycling === id} onClick={() => set({ recycling: id })}>{label}</button>
        ))}
      </div>
      <p className="hint">
        {s.recycling === 'chain' && 'Recyclers may take the targets and the items on their ingredient chain, plus the items listed below.'}
        {s.recycling === 'list' && 'Recyclers only take the items listed below.'}
        {s.recycling === 'all' && 'Every recycling recipe: finds detours through other items, but the model grows to ~16 000 columns and takes minutes, so the planner asks before solving it (and you can cancel).'}
      </p>
      <AllowList data={data} list={s.recycle_allow} onChange={recycle_allow => set({ recycle_allow })} />
      <p className="hint">Quality mining is not modelled yet: ores come in normal quality. Inputs can carry a quality (set it on the row). Catalyst recipes (Kovarex, pentapod eggs) roll quality on their whole output, catalyst included; not yet checked in game.</p>
    </section>
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
  const itemName = (id: string) => qItemName(data, id)
  const sf = data.game === 'satisfactory'
  const hasQuality = data.game === 'factorio' && (data.qualities?.length ?? 0) > 0
  const dsp = data.game === 'dsp'
  // buildings that compete for some recipe (machine tiers)
  const tiers = data.buildings.filter(b => data.recipes.some(r => (r.producers?.length ?? 0) > 1 && r.producers!.includes(b.id)))
  const toggle = (list: string[], id: string) => (list.includes(id) ? list.filter(x => x !== id) : [...list, id])
  const limitOf = (item: string, dflt: number | null) => (item in s.resource_limits ? s.resource_limits[item] : dflt)

  return (
    <aside>
      {data.game === 'factorio' && (
        <section>
          <h2>Game data</h2>
          <div className="row small">
            <span className="grow">Factorio version</span>
            <select value={s.dataset || '2.0'} aria-label="Factorio data set" onChange={e => { const id = e.target.value; setSettings(x => withDataset(x, id)) }}>
              {DATASETS.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
            </select>
          </div>
          {!hasQuality && <p className="hint">Quality (legendary items, recyclers) needs a 2.1.25 or Space Age data set.</p>}
          {(data.locations?.length ?? 0) > 1 && <>
            <div className="row small"><span className="grow">Planets</span></div>
            <div className="chips" role="group" aria-label="Planets">
              {data.locations!.map(l => (
                <label key={l.id} className="chip">
                  <input type="checkbox" checked={s.planets.includes(l.id)}
                    onChange={() => set({ planets: toggle(s.planets, l.id) })} />{l.name}
                </label>
              ))}
            </div>
            <p className="hint">Recipes and resources of other planets are off (Gleba plants, Vulcanus calcite, asteroid chunks on the space platform). Items from elsewhere can be added under Available inputs.</p>
          </>}
        </section>
      )}

      <section>
        <h2>Targets</h2>
        <ItemRates data={data} list={s.targets} onChange={targets => set({ targets })} producibleOnly quality={hasQuality} />
      </section>

      {hasQuality && <QualitySettings data={data} s={s} set={set} />}

      <section>
        <h2>Optimise for</h2>
        <div className="seg">
          {(['resources', 'buildings', 'power', ...(dsp ? ['compute'] : []), 'custom'] as Objective[]).map(o => (
            <button key={o} className={s.objective === o ? 'on' : ''} title={o === 'compute' ? '算力 / UPS: minimise compute per logic tick' : undefined}
              onClick={() => set(o === 'custom' && s.objective !== 'custom'
                ? { objective: o, cost: s.objective === 'compute' ? preset('compute', s.generator)
                    : { ...preset(s.objective), resource_weights: s.cost.resource_weights, building_weights: s.cost.building_weights } }
                : { objective: o })}>
              {o === 'resources' ? 'Raw' : o === 'buildings' ? 'Buildings' : o === 'power' ? 'Power' : o === 'compute' ? '算力' : 'Custom'}
            </button>
          ))}
        </div>
        <p className="hint">
          {s.objective === 'resources' && (sf ? 'Raw resources weighted by scarcity (Iron Ore = 1, uranium ≈ 44); buildings break ties.' : `Raw resources (ores 1, water 0.01, oil 0.2, drops 10${data.game === 'factorio' && hasQuality ? ', asteroid chunks 100' : ''}); buildings break ties.`)}
          {s.objective === 'buildings' && 'Fewest whole buildings; raw resources break ties.'}
          {s.objective === 'power' && 'MW (production + extraction) + 1 per building, so it underclocks where that pays.'}
          {s.objective === 'compute' && 'CPU per logic tick: every building costs the same whether working or idle, plus its sorters, belts, stations, coaters and the generators for its power. The solver picks tier and proliferator mode per recipe.'}
          {s.objective === 'custom' && 'Minimise the weighted sum below. Edit any weight; the plan re-solves.'}
        </p>
        {dsp && <>
          <div className="row small">
            <span className="grow">Power from</span>
            <select value={s.generator} onChange={e => set({ generator: e.target.value as Generator })} aria-label="Generator">
              {GENERATORS.map(g => <option key={g.id} value={g.id}>{g.name}</option>)}
            </select>
          </div>
          <label className="row small">
            <input type="checkbox" checked={s.mixed_belts} onChange={e => set({ mixed_belts: e.target.checked })} />
            <span>Mixed input belts (one input sorter per building)</span>
          </label>
          {s.objective !== 'compute' && (
            <p className="hint">Generator and belts set how the compute card counts this plan{s.objective === 'custom' && s.cost.sorter > 0 ? ' and, through the sorter weight, the plan itself' : ''}.</p>
          )}
          {s.objective === 'compute' && !s.effects.some(e => e.startsWith('proliferator')) && (
            <p className="hint">No proliferator mode is enabled, so every recipe runs unsprayed.{' '}
              <a href="#" onClick={e => { e.preventDefault(); set({ effects: [...s.effects, 'proliferator-3-speed', 'proliferator-3-products'] }) }}>Enable Mk.III (both modes)</a>
            </p>
          )}
          {s.objective === 'compute' && <p className="hint">Coefficients: dsp-ups research model (machine = 1, sorter 0.12, belts + station 0.13 per building; estimates, low confidence). Mining machines, orbital collectors and ray receivers are not counted.</p>}
        </>}
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
              : `Each enabled module fills all slots of a machine${hasQuality ? ' (at the module quality set under Quality; quality plans mix quality and productivity modules per machine instead)' : ''}. The foundry / EM plant / biochamber built-in +50 % applies to every recipe they run; productivity modules and research only where the recipe allows them; the total is capped at +300 %. Beacons: later.`}
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
        <ItemRates data={data} list={s.supplies} onChange={supplies => set({ supplies })} quality={hasQuality} />

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
