import { useMemo, useState } from 'react'
import { defaultSettings, type GameData, type ItemRate, type Objective, type Settings } from './solver'

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
  const byName = useMemo(() => new Map(data.items.map(i => [i.name.toLowerCase(), i.id])), [data])
  const name = (id: string) => data.items.find(i => i.id === id)?.name ?? id
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
          <span className="grow">{name(t.item)}</span>
          <input type="number" min={0} step="any" value={t.rate}
            onChange={e => onChange(list.map((x, j) => j === i ? { ...x, rate: Math.max(0, +e.target.value) } : x))} />
          <span className="muted">/min</span>
          <button className="x" title="remove" onClick={() => onChange(list.filter((_, j) => j !== i))}>×</button>
        </div>
      ))}
      <input className="wide" list={producibleOnly ? 'items-prod' : 'items-all'} placeholder="+ add item…"
        value={text} onChange={e => add(e.target.value)} />
      <datalist id={producibleOnly ? 'items-prod' : 'items-all'}>
        {options.map(i => <option key={i.id} value={i.name} />)}
      </datalist>
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
  const itemName = (id: string) => data.items.find(i => i.id === id)?.name ?? id
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
          {(['resources', 'buildings', 'power'] as Objective[]).map(o => (
            <button key={o} className={s.objective === o ? 'on' : ''} onClick={() => set({ objective: o })}>
              {o === 'resources' ? 'Raw resources' : o === 'buildings' ? 'Buildings' : 'Power'}
            </button>
          ))}
        </div>
        <p className="hint">
          {s.objective === 'resources' && 'Each raw resource is weighted by 1 / world maximum, so scarce ores cost more.'}
          {s.objective === 'buildings' && 'Fewest production buildings; raw resources break ties.'}
          {s.objective === 'power' && 'Least power: production + estimated extraction (Miner Mk.3 / extractors at 100%).'}
        </p>
      </section>

      <section>
        <h2>Clock speed</h2>
        <div className="row">
          <input type="range" min={1} max={250} value={s.clock_percent} className="grow"
            onChange={e => set({ clock_percent: +e.target.value })} />
          <input type="number" min={1} max={250} value={s.clock_percent}
            onChange={e => set({ clock_percent: Math.min(250, Math.max(1, +e.target.value)) })} />
          <span className="muted">%</span>
        </div>
        <p className="hint">Applied to every production building. Power ∝ clock<sup>1.32</sup>. Somersloops are set per recipe in the plan table.</p>
      </section>

      <section>
        <h2>Alternate recipes</h2>
        <label className="row">
          <input type="checkbox" checked={s.allow_all_alternates}
            onChange={e => set({ allow_all_alternates: e.target.checked, enabled_alternates: [] })} />
          <span>Allow all {data.recipes.filter(r => r.alt).length} alternates</span>
        </label>
        <label className="row">
          <input type="checkbox" checked={s.allow_ore_conversion} onChange={e => set({ allow_ore_conversion: e.target.checked })} />
          <span>Converter ore conversion (e.g. Bauxite from Copper)</span>
        </label>
        <details>
          <summary>Choose individually ({s.allow_all_alternates ? 'all' : s.enabled_alternates.length} on)</summary>
          <input className="wide" placeholder="filter…" value={altFilter} onChange={e => setAltFilter(e.target.value)} />
          <div className="scroll">
            {alts.map(r => (
              <label className="row small" key={r.id}>
                <input type="checkbox" checked={altEnabled(r.id)} onChange={() => toggleAlt(r.id)} />
                <span>{r.name}</span>
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

      <section>
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
      </section>

      <section>
        <button className="wide" onClick={() => setSettings(() => defaultSettings())}>Reset everything</button>
        <p className="hint">The URL holds the whole plan; copy it to share.</p>
      </section>
    </aside>
  )
}
