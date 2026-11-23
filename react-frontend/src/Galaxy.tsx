// DSP galaxy tab (星区): seed import, infinite-threshold button, multi-system LP, Dark Fog sources.
// Model: src/planner/dsp_galaxy.rs; spec docs/research/dsp-seed/00-最终报告.md §4–§5, §9.
// Its settings are its own (stored per browser): changing them never re-solves the main plan.
import { useEffect, useMemo, useRef, useState } from 'react'
import { Icon } from './Icon'
import { fmt, itemName, type GameData, type ItemRate, type Settings } from './solver'

// ---- types mirroring dsp_galaxy.rs
interface SeedSettings { seed: number; stars: number; resource_multiplier: number; hive_initial_colonize: number; hive_max_density: number }
interface Vein { item: string; amount: number; nodes: number }
interface GasRate { item: string; base: number }
interface PlanetInfo { index: number; kind: string; theme: string; ocean: string | null; gases: GasRate[]; veins: Vein[]; seeps: number; oil_rate: number }
interface StarInfo {
  index: number; name: string; star_type: string; spectr: string; luminosity: number; position: number[]; distance: number
  dyson_radius: number; initial_hives: number; max_hives: number; planets: PlanetInfo[]
}
interface Galaxy {
  settings: SeedSettings; rare_tier: boolean; infinite: boolean; stars: StarInfo[]; totals: { item: string; stock: number; planets: number }[]
  gas_giants: number; gas_totals: GasRate[]; oil_rate: number; seeps: number; generate_ms: number; fingerprint: string; auto_systems: number[]; error?: string
}
interface DarkFog { enabled: boolean; level: number; bases: [number, number][]; drops: ItemRate[]; measured_at_level: number }
interface ThresholdRow { item: string; stock: number; per_cube: number; level: (number | null)[]; cubes: (number | null)[]; hours: (number | null)[]; quota_at_level: number }
interface ThresholdReport { rows: ThresholdRow[]; feasible_from: (number | null)[]; raw_at_feasible: ItemRate[]; raw_at_level: ItemRate[]; level: number; s_level: number; warnings: string[]; error?: string }
interface SystemResult {
  star: number; name: string; distance: number; cubes: number; production_buildings: number; miners: number; pumps: number; oil_extractors: number; collectors: number
  receivers: number; stations: number; power_mw: number; recipes: { recipe: string; mode: string; building: string; machines: number }[]
  extraction: { item: string; rate: number; cap: number | null; limit: string }[]; drops: ItemRate[]
}
interface GalaxyPlan {
  status: string; z: number; level: number; speed: number; s_level: number; buildings: number; production_buildings: number; miners: number; pumps: number; oil_extractors: number
  collectors: number; receivers: number; stations: number; photons_per_min: number; receiver_gw: number; warpers_per_min: number; power_mw: number
  systems: SystemResult[]; shipments: { item: string; from: number; to: number; rate: number; ly: number }[]
  rares: { item: string; per_cube: number; quota_per_cube: number; binding: boolean }[]; raw_per_cube: ItemRate[]
  columns: number; rows: number; solve_ms: number; warnings: string[]; error?: string
}
interface TotalReport {
  status: string; cubes: number; cubes_no_util: number; levels_done: number; stars: number; hub: string | null; warpers: number
  finite: { item: string; available: number; used: number; per_cube: number; binding: boolean; marginal: number }[]
  unlimited: string[]; raw_per_cube: ItemRate[]; recipes: { recipe: string; mode: string; building: string; machines_per_rate: number }[]
  solve_ms: number; warnings: string[]; error?: string
}
interface Opts {
  seed: SeedSettings
  total_levels: number; total_scope: 'all' | 'ticked'; total_warpers: boolean; total_alts: boolean; stock_quotas: boolean
  rate: number; remaining: number; level: number
  goal: 'target' | 'max'; target: number; objective: 'buildings' | 'photons'; rho: number; phi: number
  oil: 'initial' | 'floor'; building_cap: number | null; photon_gw: number | null; vessel_capacity: number; warpers_per_trip: number
  systems: number[] | null
  dark_fog: DarkFog
}

const DEFAULTS: Opts = {
  seed: { seed: 12345678, stars: 32, resource_multiplier: 0.1, hive_initial_colonize: 1, hive_max_density: 1 },
  rate: 750, remaining: 100, level: 81,
  total_levels: 0, total_scope: 'all', total_warpers: false, total_alts: true, stock_quotas: true,
  goal: 'target', target: 1000, objective: 'buildings', rho: 1, phi: 0,
  oil: 'initial', building_cap: null, photon_gw: null, vessel_capacity: 400, warpers_per_trip: 2,
  systems: null,
  dark_fog: { enabled: false, level: 24, bases: [], drops: [], measured_at_level: 81 },
}
const KEY = 'dspGalaxy.v1'
function loadOpts(): Opts {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? 'null') as Partial<Opts> | null
    if (raw && typeof raw === 'object') {
      const o = { ...DEFAULTS, ...raw, seed: { ...DEFAULTS.seed, ...raw.seed }, dark_fog: { ...DEFAULTS.dark_fog, ...raw.dark_fog } }
      if (!MULTS.some(([v]) => v === o.seed.resource_multiplier)) o.seed.resource_multiplier = 1
      return o
    }
  } catch { /* private mode or bad JSON: defaults */ }
  return DEFAULTS
}

// the game's ten settings (the same list as DSP-Seed-Finder web/src/util.ts; there is no 10×)
const MULTS: [number, string][] = [[0.1, '0.1× 极少'], [0.5, '0.5×'], [0.8, '0.8×'], [1, '1×'], [1.5, '1.5×'], [2, '2×'], [3, '3×'], [5, '5×'], [8, '8×'], [100, '无限 infinite']]
/** BWIKI 黑雾掉落 unlock levels (same table as dsp_galaxy.rs `drop_unlock_level`). */
const DROPS: [string, number][] = [
  ['processor', 9], ['graphene', 9], ['carbon-nanotube', 12], ['df-dark-fog-matrix', 12], ['optical-grating-crystal', 15], ['particle-container', 15],
  ['casimir-crystal', 15], ['df-matter-recombinator', 15], ['strange-matter', 18], ['df-silicon-based-neuron', 18], ['antimatter', 21], ['quantum-chip', 21],
  ['graviton-lens', 21], ['space-warper', 21], ['df-negentropy-singularity', 21], ['unipolar-magnet', 24], ['antimatter-fuel-rod', 24], ['df-core-element', 24],
]
const TYPE_ZH: Record<string, string> = { MainSeqStar: '主序星', GiantStar: '巨星', WhiteDwarf: '白矮星', NeutronStar: '中子星', BlackHole: '黑洞' }
const RARE = new Set(['fire-ice', 'kimberlite-ore', 'fractal-silicon', 'organic-crystal', 'optical-grating-crystal', 'spiniform-stalagmite-crystal', 'unipolar-magnet'])

/** Thousands-separated number with at most `d` decimals. */
const num = (x: number, d = 0) => (Number.isFinite(x) ? (Number(x.toFixed(d)) + 0).toLocaleString('en-US', { maximumFractionDigits: d, minimumFractionDigits: 0 }) : '–')
/** Compact: 1.23 M, 456 k. */
const compact = (x: number) => (Math.abs(x) >= 1e9 ? `${fmt(x / 1e9, 2)} G` : Math.abs(x) >= 1e6 ? `${fmt(x / 1e6, 2)} M` : Math.abs(x) >= 1e4 ? `${fmt(x / 1e3, 0)} k` : num(x, 1))
const lv = (l: number | null | undefined) => (l === null || l === undefined ? '—' : `L${l}`)

/** Number field: typing is free; the value is clamped and applied on Enter or when the field is left
 *  (clamping on every keystroke made "48" untypeable in a 32–64 field: "4" became 32). An empty
 *  `nullable` field means "none". */
function NumField({ value, onCommit, lo, hi, integer, nullable, ...rest }: {
  value: number | null; onCommit: (v: number | null) => void; lo: number; hi: number; integer?: boolean; nullable?: boolean
} & Omit<React.InputHTMLAttributes<HTMLInputElement>, 'value' | 'onChange' | 'type'>) {
  const show = (v: number | null) => (v === null ? '' : String(v))
  const [draft, setDraft] = useState(show(value))
  useEffect(() => setDraft(show(value)), [value])
  const commit = () => {
    if (draft.trim() === '' && nullable) { if (value !== null) onCommit(null); return }
    const x = Number(draft)
    if (draft.trim() === '' || !Number.isFinite(x)) { setDraft(show(value)); return }
    const v = Math.min(hi, Math.max(lo, integer ? Math.round(x) : x))
    setDraft(show(v))
    if (v !== value) onCommit(v)
  }
  return <input type="number" min={lo} max={hi} {...rest} value={draft} onChange={e => setDraft(e.target.value)} onBlur={commit}
    onKeyDown={e => { if (e.key === 'Enter') commit() }} />
}

// one worker for all calls; answers matched by id
let worker: Worker | null = null
let nextId = 1
const waiting = new Map<number, (s: string) => void>()
function call<T>(fn: 'galaxy' | 'thresholds' | 'plan' | 'total', payload: unknown): Promise<T> {
  if (!worker) {
    worker = new Worker(new URL('./galaxyWorker.ts', import.meta.url), { type: 'module' })
    worker.onmessage = (e: MessageEvent<{ id: number; out: string }>) => {
      waiting.get(e.data.id)?.(e.data.out)
      waiting.delete(e.data.id)
    }
    // a worker that fails to load or dies answers nothing: end every pending call with an error and
    // start a fresh worker on the next call, so the tab never stays on "Solving…"
    worker.onerror = e => {
      const out = JSON.stringify({ error: `the galaxy worker failed: ${e.message || 'could not load'}` })
      waiting.forEach(res => res(out))
      waiting.clear()
      worker?.terminate()
      worker = null
    }
  }
  const id = nextId++
  return new Promise<T>(res => {
    waiting.set(id, out => res(JSON.parse(out) as T))
    worker!.postMessage({ id, fn, json: JSON.stringify(payload) })
  })
}

interface Props { data: GameData; settings: Settings }

export default function GalaxyTab({ data, settings }: Props) {
  const [o, setO] = useState<Opts>(loadOpts)
  useEffect(() => { try { localStorage.setItem(KEY, JSON.stringify(o)) } catch { /* ignore */ } }, [o])
  const set = (p: Partial<Opts>) => setO(x => ({ ...x, ...p }))
  const [seedDraft, setSeedDraft] = useState<SeedSettings>(o.seed)
  const [g, setG] = useState<Galaxy | null>(null)
  const [busy, setBusy] = useState('')
  const [th, setTh] = useState<ThresholdReport | null>(null)
  const [gp, setGp] = useState<GalaxyPlan | null>(null)
  const [tot, setTot] = useState<TotalReport | null>(null)
  const [totKey, setTotKey] = useState('')
  const [open, setOpen] = useState<number | null>(null)
  const name = (id: string) => itemName(data, id)
  const gen = useRef(0)

  // the recipe side comes from the sidebar (alternates, proliferator, buildings); targets are set by the model
  const plan = useMemo(() => ({ ...settings, game: 'dsp', integer: false }), [settings])
  const darkFog = o.dark_fog

  const importSeed = (s: SeedSettings) => {
    const t = ++gen.current
    setBusy('Generating the galaxy…')
    setTh(null); setGp(null); setTot(null)
    void call<Galaxy>('galaxy', s).then(r => {
      if (t !== gen.current) return
      setBusy('')
      setG(r)
      if (!r.error) set({ seed: s, systems: o.seed.seed === s.seed && o.seed.stars === s.stars ? o.systems : null })
    })
  }
  // the stored seed is imported on first view
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { importSeed(o.seed) }, [])

  const systems = useMemo(() => (g && !g.error ? (o.systems ?? g.auto_systems) : []), [g, o.systems])
  const toggleSystem = (i: number) => {
    const cur = new Set(systems)
    if (cur.has(i)) cur.delete(i); else cur.add(i)
    set({ systems: [...cur].sort((a, b) => a - b) })
  }

  // what each button sends; a result is shown as out of date when this no longer equals what it was computed from
  const thPayload = useMemo(() => JSON.stringify({
    seed: o.seed, plan, rate: o.rate, remaining: o.remaining / 100, stock_override: {}, dark_fog: darkFog, level: Math.max(6, o.level),
  }), [o.seed, plan, o.rate, o.remaining, darkFog, o.level])
  const gpPayload = useMemo(() => JSON.stringify({
    seed: o.seed, plan, goal: o.goal, target: o.target, objective: o.objective, level: o.level, rho: o.rho, phi: o.phi / 100,
    stock_quotas: o.stock_quotas, remaining: o.remaining / 100, systems, oil: o.oil, building_cap: o.building_cap, photon_gw: o.photon_gw,
    vessel_capacity: o.vessel_capacity, warpers_per_trip: o.warpers_per_trip, dark_fog: darkFog,
  }), [o, plan, systems, darkFog])
  const totPayload = useMemo(() => JSON.stringify({
    // "every recipe": all alternates and both Mk.III modes, so the LP picks what gives the most cubes
    seed: o.seed, plan: o.total_alts ? { ...plan, allow_all_alternates: true, effects: [...new Set([...plan.effects, 'proliferator-3-speed', 'proliferator-3-products'])] } : plan,
    levels_done: o.total_levels, remaining: o.remaining / 100, stock_override: {},
    systems: o.total_scope === 'all' ? [] : systems, warpers: o.total_warpers, vessel_capacity: o.vessel_capacity, warpers_per_trip: o.warpers_per_trip,
  }), [o.seed, plan, o.total_alts, o.total_levels, o.remaining, o.total_scope, o.total_scope === 'all' ? null : systems, o.total_warpers, o.vessel_capacity, o.warpers_per_trip])
  const runTotal = () => {
    const t = gen.current, sent = totPayload
    setBusy('Solving the galaxy total…')
    void call<TotalReport>('total', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setTot(r); setTotKey(sent) })
  }
  const totStale = !!tot && !tot.error && totKey !== totPayload
  const [thKey, setThKey] = useState('')
  const [gpKey, setGpKey] = useState('')
  const runThresholds = () => {
    const t = gen.current, sent = thPayload
    setBusy('Computing thresholds…')
    void call<ThresholdReport>('thresholds', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setTh(r); setThKey(sent) })
  }
  const runPlan = () => {
    const t = gen.current, sent = gpPayload
    setBusy('Solving the galaxy LP…')
    void call<GalaxyPlan>('plan', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setGp(r); setGpKey(sent) })
  }
  const thStale = !!th && !th.error && thKey !== thPayload
  const gpStale = !!gp && !gp.error && gpKey !== gpPayload

  const starName = (i: number) => g?.stars[i]?.name ?? `#${i}`
  const seedDirty = JSON.stringify(seedDraft) !== JSON.stringify(o.seed)
  const prolif = settings.effects.find(e => e.startsWith('proliferator'))

  return (
    <div className="mixedbelt galaxy">
      <p className="hint">
        星区规划 Galaxy planning: generate the galaxy of a seed (generator: <a href="https://github.com/DoubleUTH/DSP-Seed-Finder" target="_blank" rel="noreferrer">DoubleUTH/DSP-Seed-Finder</a>, modified,{' '}
        <a href={`${import.meta.env.BASE_URL}licenses/DSP-Seed-Finder-LICENSE.txt`} target="_blank" rel="noreferrer">Apache-2.0</a>,{' '}
        <a href={`${import.meta.env.BASE_URL}licenses/DSP-Seed-Finder-NOTICE.txt`} target="_blank" rel="noreferrer">changes</a>),
        find when each finite resource can be used forever (矿物利用 0.94<sup>L</sup>), and split production over star systems.
        Recipe options come from the sidebar: {settings.allow_all_alternates ? 'all alternates' : `${settings.enabled_alternates.length} alternates`},
        {prolif ? ` proliferator ${settings.effects.filter(e => e.startsWith('proliferator')).join(' + ')}` : ' no proliferator'}
        {!settings.allow_all_alternates && <> (the report's numbers use all alternates)</>}.
      </p>

      {/* ---- seed import */}
      <h3>种子 Seed</h3>
      <div className="mbsettings">
        <label>Seed<NumField value={seedDraft.seed} lo={0} hi={99999999} integer onCommit={v => setSeedDraft(x => ({ ...x, seed: v ?? x.seed }))} style={{ width: 110 }} /></label>
        <label>Stars<NumField value={seedDraft.stars} lo={32} hi={64} integer onCommit={v => setSeedDraft(x => ({ ...x, stars: v ?? x.stars }))} /></label>
        <label>Resources
          <select value={seedDraft.resource_multiplier} onChange={e => setSeedDraft(x => ({ ...x, resource_multiplier: Number(e.target.value) }))}>
            {MULTS.map(([v, t]) => <option key={v} value={v}>{t}</option>)}
          </select>
        </label>
        <button onClick={() => importSeed(seedDraft)} disabled={!!busy}>Import seed</button>
        <details>
          <summary>Dark Fog generation settings (hive counts)</summary>
          <div className="mbsettings">
            <label>Initial colonize<NumField step={0.25} value={seedDraft.hive_initial_colonize} lo={0} hi={2} onCommit={v => setSeedDraft(x => ({ ...x, hive_initial_colonize: v ?? 1 }))} /></label>
            <label>Max density<NumField step={0.25} value={seedDraft.hive_max_density} lo={0} hi={3} onCommit={v => setSeedDraft(x => ({ ...x, hive_max_density: v ?? 1 }))} /></label>
          </div>
        </details>
      </div>
      <p className="hint">
        The browser and the desktop build give the same galaxy{g && !g.error ? <> (fingerprint <code>{g.fingerprint}</code>)</> : null}. Against the game it is spot-checked only:
        values players posted before the Dark Fog update match (star luminosity and distance for four seeds; home-planet iron and oil for one, its coal 8.3 M vs 8.59 M here).
        Check a planet of your own save before relying on a stock.
        {seedDirty && <> <b>The fields above differ from the imported galaxy</b> (seed {o.seed.seed}, {o.seed.stars} stars, {o.seed.resource_multiplier}×): press Import seed.</>}
      </p>
      {busy && <p className="solving" role="status">{busy}</p>}
      {g?.error && <p className="warn">{g.error}</p>}
      {g && !g.error && <GalaxyTables g={g} name={name} systems={systems} toggle={toggleSystem} open={open} setOpen={setOpen}
        resetSystems={() => set({ systems: null })} />}

      {/* ---- galaxy total */}
      {g && !g.error && <>
        <h3>理想总产量 Galaxy total</h3>
        <p className="hint">
          How many universe matrices the galaxy's finite veins make at best, with no quota and no rate: machines, stations, sails and time are unlimited,
          so oceans, gas giants, oil seeps (never mined below their floor) and Dyson photons are unlimited too, and only the veins run out.
          One LP picks the recipes (sidebar alternates and proliferator) that turn the stock into the most cubes. Ore spent on research before then is not counted yet.
        </p>
        <div className="mbsettings">
          <label>Vein utilisation levels done<NumField value={o.total_levels} lo={0} hi={500} integer onCommit={v => set({ total_levels: v ?? 0 })} /></label>
          <label>Stock of
            <select value={o.total_scope} onChange={e => set({ total_scope: e.target.value as Opts['total_scope'] })}>
              <option value="all">every star ({g.stars.length})</option>
              <option value="ticked">the ticked systems ({systems.length})</option>
            </select>
          </label>
          <label className="check"><input type="checkbox" checked={o.total_alts} onChange={e => set({ total_alts: e.target.checked })} /> every recipe and Mk.III proliferator (used only where they give more)</label>
          <label className="check"><input type="checkbox" checked={o.total_warpers} onChange={e => set({ total_warpers: e.target.checked })} /> ship with warpers (faster; off: vessels fly slowly, no cost)</label>
          <label>Stock remaining %<NumField value={o.remaining} lo={0} hi={100} onCommit={v => set({ remaining: v ?? 100 })} /></label>
          <button onClick={runTotal} disabled={!!busy || (o.total_scope === 'ticked' && systems.length === 0)}>Compute total</button>
        </div>
        {tot?.error && <p className="warn">{tot.error}</p>}
        {totStale && <p className="warn stale" role="status"><b>Out of date</b> — settings changed since this total was computed. Press Compute total.</p>}
        {tot && !tot.error && <div className={totStale ? 'staleresult' : undefined}><TotalView t={tot} name={name} level={o.level}
          recipeName={id => { const r = data.recipes.find(x => x.id === id); return r ? (r.name_zh ?? r.name) : id }} /></div>}
      </>}

      {/* ---- thresholds */}
      {g && !g.error && <>
        <h3>无限阈值 Infinite threshold</h3>
        <p className="hint">
          Level L at which a resource can be used at its unconstrained rate forever, if every cube from then on goes into vein utilisation:
          a·ρ·S(L) ≤ B, S(L) = Σ<sub>n≥L</sub> 4000(n−5)·0.94<sup>n−1</sup> (S(6) = 815 449). ρ = 0.8 if research labs are sprayed with Mk.III (×1.25 hashes, 存疑).
          B is the remaining stock: oil counts seep points above the 2500 floor. 笑笑 waits about five levels past the release level as a buffer (L80 → L85).
        </p>
        <div className="mbsettings">
          <label>Cubes/min (for hours)<NumField value={o.rate} lo={1} hi={1e7} onCommit={v => set({ rate: v ?? 750 })} /></label>
          <label>Stock remaining %<NumField value={o.remaining} lo={0} hi={100} onCommit={v => set({ remaining: v ?? 100 })} /></label>
          <label>Quota level L<NumField value={o.level} lo={1} hi={2000} integer onCommit={v => set({ level: v ?? 81 })} /></label>
          <button onClick={runThresholds} disabled={!!busy}>Compute thresholds</button>
        </div>
        {th?.error && <p className="warn">{th.error}</p>}
        {thStale && <p className="warn stale" role="status"><b>Out of date</b> — settings changed since these thresholds were computed. Press Compute thresholds.</p>}
        {th && !th.error && <div className={thStale ? 'staleresult' : undefined}><ThresholdTable th={th} name={name} gas={new Set(g.gas_totals.map(x => x.item))} /></div>}
      </>}

      {/* ---- Dark Fog */}
      {g && !g.error && <DarkFogBox g={g} df={darkFog} set={df => set({ dark_fog: df })} name={name} />}

      {/* ---- multi-system LP */}
      {g && !g.error && <>
        <h3>多星系规划 Multi-system plan</h3>
        <p className="hint">
          One LP over the {systems.length} systems ticked above (all planets of a system are pooled; 笑笑 opens one planet per system under a full-strength Dark Fog): production per system,
          interstellar shipping (stations ∝ distance, fitted to 笑笑's 4 four-stacked blue belts per tower at 7 ly and 3 at 12 ly; warpers per trip),
          finite stocks as quotas m·ρ·S(L) ≤ B·z per system, oil and gas giants at level-L rates (×{fmt(1 + 0.1 * (o.level - 1), 1)}), water and acid where the system has that ocean.
          Power plants are not modelled; photons are free unless capped.
        </p>
        <div className="mbsettings">
          <label>Goal
            <select value={o.goal} onChange={e => set({ goal: e.target.value as Opts['goal'] })}>
              <option value="target">Target z, fewest buildings</option>
              <option value="max">Maximum z (终局最大白糖)</option>
            </select>
          </label>
          {o.goal === 'target' && <label>Target cubes/min<NumField value={o.target} lo={0} hi={1e7} onCommit={v => set({ target: v ?? 1000 })} /></label>}
          {o.goal === 'target' && <label>Minimise
            <select value={o.objective} onChange={e => set({ objective: e.target.value as Opts['objective'] })}>
              <option value="buildings">buildings</option>
              <option value="photons">photons, then buildings</option>
            </select>
          </label>}
          <label>Level L<NumField value={o.level} lo={1} hi={2000} integer onCommit={v => set({ level: v ?? 81 })} /></label>
          <label>ρ research
            <select value={o.rho} onChange={e => set({ rho: Number(e.target.value) })}>
              <option value={1}>1 (labs not sprayed)</option>
              <option value={0.8}>0.8 (Mk.III sprayed)</option>
            </select>
          </label>
          <label>Building cap<NumField placeholder="none" nullable value={o.building_cap} lo={0} hi={1e8} onCommit={v => set({ building_cap: v })} /></label>
          <label>Dyson GW<NumField placeholder="none" nullable value={o.photon_gw} lo={0} hi={1e6} onCommit={v => set({ photon_gw: v })} /></label>
          <button onClick={runPlan} disabled={!!busy || systems.length === 0}>Plan galaxy</button>
          <details>
            <summary>More: oil, quotas, cubes not researched, logistics</summary>
            <div className="mbsettings">
              <label>Oil rate
                <select value={o.oil} onChange={e => set({ oil: e.target.value as Opts['oil'] })}>
                  <option value="initial">initial seeps (upper bound)</option>
                  <option value="floor">depleted floor 0.1/s per seep</option>
                </select>
              </label>
              <label className="check"><input type="checkbox" checked={o.stock_quotas} onChange={e => set({ stock_quotas: e.target.checked })} /> stock quotas (off: rates only)</label>
              <label>φ cubes elsewhere %<NumField value={o.phi} lo={0} hi={95} onCommit={v => set({ phi: v ?? 0 })} /></label>
              <label>Vessel capacity<NumField value={o.vessel_capacity} lo={1} hi={10000} onCommit={v => set({ vessel_capacity: v ?? 400 })} /></label>
              <label>Warpers per trip<NumField step={1} value={o.warpers_per_trip} lo={0} hi={4} onCommit={v => set({ warpers_per_trip: v ?? 2 })} /></label>
            </div>
          </details>
        </div>
        {gp?.error && <p className="warn">{gp.error}</p>}
        {gpStale && <p className="warn stale" role="status"><b>Out of date</b> — settings changed since this plan was solved. Press Plan galaxy.</p>}
        {gp && !gp.error && <div className={gpStale ? 'staleresult' : undefined}><PlanView gp={gp} name={name} starName={starName} recipeName={id => { const r = data.recipes.find(x => x.id === id); return r ? (r.name_zh ?? r.name) : id }} /></div>}
      </>}
    </div>
  )
}

function GalaxyTables({ g, name, systems, toggle, open, setOpen, resetSystems }: {
  g: Galaxy; name: (id: string) => string; systems: number[]; toggle: (i: number) => void; open: number | null; setOpen: (i: number | null) => void; resetSystems: () => void
}) {
  const on = new Set(systems)
  const mag = g.totals.find(t => t.item === 'unipolar-magnet')?.stock ?? 0
  return <>
    <div className="cards">
      <div className="card"><span className="big">{g.stars.length}</span><span className="muted">Stars</span></div>
      <div className="card"><span className="big">{g.stars.reduce((a, s) => a + s.planets.length, 0)}</span><span className="muted">Planets</span></div>
      <div className="card"><span className="big">{g.gas_giants}</span><span className="muted">Gas giants</span></div>
      <div className="card"><span className="big">{fmt(g.oil_rate, 1)}<span className="unit">/s</span></span><span className="muted">Oil, {g.seeps} seeps</span></div>
      <div className="card"><span className="big">{compact(mag)}</span><span className="muted">Unipolar magnets</span></div>
      <div className="card"><span className="big">{fmt(g.generate_ms / 1000, 2)}<span className="unit"> s</span></span><span className="muted">Generated</span></div>
    </div>
    <h4>Galaxy stock {g.rare_tier ? '(rare tier: oil ×0.5, gas ×0.8)' : ''}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>Resource</th><th className="num">Stock</th><th className="num">Planets</th></tr></thead>
        <tbody>
          {g.totals.map(t => (
            <tr key={t.item}><td><Icon id={t.item} size={18} />{name(t.item)}{RARE.has(t.item) ? <span className="tag">珍奇</span> : null}</td>
              <td className="num">{compact(t.stock)}</td><td className="num">{t.planets}</td></tr>
          ))}
          {g.gas_totals.map(t => (
            <tr key={`gas-${t.item}`}><td><Icon id={t.item} size={18} />{name(t.item)} <span className="muted">gas giants</span></td>
              <td className="num">{fmt(t.base, 2)}/s base</td><td className="num">—</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>Star systems · ticked = planned ({systems.length}) <button className="tabbtn" onClick={resetSystems}>Automatic pick</button></h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>Plan</th><th>Star</th><th className="num">ly</th><th className="num">Lum.</th><th className="num">Hives</th><th>Resources</th></tr></thead>
        <tbody>
          {g.stars.map(s => {
            const items = new Set<string>()
            s.planets.forEach(p => { p.veins.forEach(v => items.add(v.item)); p.gases.forEach(x => items.add(x.item)); if (p.ocean) items.add(p.ocean) })
            const rare = [...items].filter(i => RARE.has(i) || i === 'sulfuric-acid')
            return [
              <tr key={s.index} className="clickable" onClick={() => setOpen(open === s.index ? null : s.index)}>
                <td onClick={e => e.stopPropagation()}><input type="checkbox" aria-label={`Plan ${s.name}`} checked={on.has(s.index)} onChange={() => toggle(s.index)} /></td>
                <td><span className="caret">{open === s.index ? '▾' : '▸'}</span>{s.name} <span className="muted">{TYPE_ZH[s.star_type] ?? s.star_type} {s.spectr}</span></td>
                <td className="num">{fmt(s.distance, 1)}</td><td className="num">{fmt(s.luminosity, 2)}</td>
                <td className="num">{s.initial_hives}/{s.max_hives}</td>
                <td className="small">{rare.map(i => <Icon key={i} id={i} size={16} />)}{rare.length === 0 ? <span className="muted">common only</span> : null}</td>
              </tr>,
              open === s.index && <tr key={`${s.index}-p`}><td colSpan={6} className="small">
                {s.planets.map(p => (
                  <div key={p.index}><b>P{p.index}</b> {p.kind} · {p.theme}{p.ocean ? ` · ${name(p.ocean)} ocean` : ''}
                    {p.gases.length > 0 && <> · gas {p.gases.map(x => `${name(x.item)} ${fmt(x.base, 3)}`).join(', ')}</>}
                    {p.veins.length > 0 && <> · {p.veins.map(v => v.item === 'crude-oil' ? `${name(v.item)} ${fmt(v.amount * 4e-5, 2)}/s (${v.nodes})` : `${name(v.item)} ${compact(v.amount)}`).join(', ')}</>}
                  </div>
                ))}
              </td></tr>,
            ]
          })}
        </tbody>
      </table>
    </div>
  </>
}

function ThresholdTable({ th, name, gas }: { th: ThresholdReport; name: (id: string) => string; gas: Set<string> }) {
  const rows = th.rows.filter(r => r.stock > 0 || r.per_cube > 0)
  return <>
    <p>
      All quotas at once (LP, fewest buildings, 1 cube/min): feasible from <b>{lv(th.feasible_from[0])}</b> (ρ = 1) / <b>{lv(th.feasible_from[1])}</b> (ρ = 0.8).
      {th.feasible_from[0] !== null && th.raw_at_feasible.length > 0 && <> At {lv(th.feasible_from[0])} it uses crude oil {fmt(th.raw_at_feasible.find(r => r.item === 'crude-oil')?.rate ?? 0, 2)}/cube
        and unipolar magnets {fmt(th.raw_at_feasible.find(r => r.item === 'unipolar-magnet')?.rate ?? 0, 2)}/cube (up to their quotas).</>}
    </p>
    {th.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <div className="tablewrap">
      <table className="mbtable threshold">
        <thead>
          <tr><th rowSpan={2}>Resource</th><th className="num" rowSpan={2}>Stock B</th><th className="num" rowSpan={2}>Per cube a</th>
            <th colSpan={3}>ρ = 1</th><th colSpan={3}>ρ = 0.8</th><th className="num" rowSpan={2}>Quota/cube at L{th.level}</th></tr>
          <tr><th className="num">Release</th><th className="num">Cubes</th><th className="num">Hours</th><th className="num">Release</th><th className="num">Cubes</th><th className="num">Hours</th></tr>
        </thead>
        <tbody>
          {rows.map(r => (
            <tr key={r.item}>
              <td><Icon id={r.item} size={18} />{name(r.item)}{gas.has(r.item) ? <span className="tag" title="Gas giants also give it at a rate that never runs out; this row counts the vein stock only">+ gas giants</span> : null}</td>
              <td className="num">{compact(r.stock)}</td>
              <td className="num">{fmt(r.per_cube, 3)}</td>
              {[0, 1].map(j => [
                <td key={`l${j}`} className="num"><b>{lv(r.level[j])}</b></td>,
                <td key={`c${j}`} className="num">{r.cubes[j] === null ? '—' : compact(r.cubes[j]!)}</td>,
                <td key={`h${j}`} className="num">{r.hours[j] === null ? '—' : num(r.hours[j]!, 0)}</td>,
              ])}
              <td className="num">{fmt(r.quota_at_level, 3)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
    <p className="hint">Release = first level whose S(L) allows the unconstrained use a per cube; cubes = 2000(L−6)(L−5)·ρ to get there. Before it the LP still uses the resource up to its quota B/(ρ·S(L)); the best schedule is threshold-shaped (report §4.3: save it, release it late).</p>
  </>
}

function DarkFogBox({ g, df, set, name }: { g: Galaxy; df: DarkFog; set: (d: DarkFog) => void; name: (id: string) => string }) {
  const [item, setItem] = useState(DROPS[15][0])
  return (
    <details className="dfbox" open={df.enabled}>
      <summary>黑雾 Dark Fog drops as sources {df.enabled ? `· on, ${df.bases.reduce((a, b) => a + b[1], 0)} bases` : '· off'}</summary>
      <p className="hint">
        No drop rates are published (report §5.4): enter what you measure in game per farmed ground base per minute, and the level of vein utilisation
        you measured at (L = the level being researched, as in the plan: L − 1 levels done). Normal items scale with (1 + 0.04 (L − 1)); df-* items do not. Items drop only from their Dark Fog level (BWIKI 黑雾掉落).
        Used by the threshold table (drops lower the mined amount per cube at the cubes/min given) and by the plan (a source at the base's system).
      </p>
      <div className="mbsettings">
        <label className="row"><input type="checkbox" checked={df.enabled} onChange={e => set({ ...df, enabled: e.target.checked })} /> Use drops</label>
        <label>Dark Fog level<NumField value={df.level} lo={0} hi={30} integer onCommit={v => set({ ...df, level: v ?? 24 })} /></label>
        <label>Measured at L<NumField value={df.measured_at_level} lo={1} hi={2000} integer onCommit={v => set({ ...df, measured_at_level: v ?? 1 })} /></label>
      </div>
      <h4>Farmed bases</h4>
      <ul className="plain">
        {df.bases.map(([s, n], k) => (
          <li key={k} className="row small">
            <select value={s} onChange={e => set({ ...df, bases: df.bases.map((b, j) => j === k ? [Number(e.target.value), b[1]] : b) })}>
              {g.stars.map(st => <option key={st.index} value={st.index}>{st.name} ({st.initial_hives}/{st.max_hives} hives)</option>)}
            </select>
            <NumField value={n} lo={0} hi={1000} aria-label="bases" onCommit={v => set({ ...df, bases: df.bases.map((b, j) => j === k ? [b[0], v ?? 0] : b) })} />
            <button className="x" aria-label="Remove" onClick={() => set({ ...df, bases: df.bases.filter((_, j) => j !== k) })}>×</button>
          </li>
        ))}
      </ul>
      <button className="tabbtn" onClick={() => set({ ...df, bases: [...df.bases, [0, 1]] })}>Add bases</button>
      <h4>Drops per base per minute (measured)</h4>
      <ul className="plain">
        {df.drops.map((d, k) => (
          <li key={k} className="row small"><Icon id={d.item} size={18} />{name(d.item)} <span className="muted">from Lv{DROPS.find(x => x[0] === d.item)?.[1] ?? 0}</span>
            <NumField step={0.1} value={d.rate} lo={0} hi={1e6} aria-label={`${name(d.item)} per base per minute`} onCommit={v => set({ ...df, drops: df.drops.map((x, j) => j === k ? { ...x, rate: v ?? 0 } : x) })} />
            <button className="x" aria-label="Remove" onClick={() => set({ ...df, drops: df.drops.filter((_, j) => j !== k) })}>×</button>
          </li>
        ))}
      </ul>
      <span className="row small">
        <select value={item} onChange={e => setItem(e.target.value)} aria-label="Drop item">
          {DROPS.filter(([i]) => !df.drops.some(d => d.item === i)).map(([i, l]) => <option key={i} value={i}>{name(i)} (Lv{l}+)</option>)}
        </select>
        <button className="tabbtn" onClick={() => { if (!df.drops.some(d => d.item === item)) set({ ...df, drops: [...df.drops, { item, rate: 1 }] }) }}>Add drop</button>
      </span>
    </details>
  )
}

function PlanView({ gp, name, starName, recipeName }: { gp: GalaxyPlan; name: (id: string) => string; starName: (i: number) => string; recipeName: (id: string) => string }) {
  const [sel, setSel] = useState<number | null>(null)
  if (gp.status !== 'optimal') {
    return <div className="warn"><b>{gp.status === 'unbounded' ? 'Unbounded' : 'No plan'}</b>{gp.warnings.map((w, i) => <p key={i}>{w}</p>)}</div>
  }
  return <>
    <div className="cards">
      <div className="card"><span className="big">{num(gp.z, 0)}<span className="unit">/min</span></span><span className="muted">Universe matrix</span></div>
      <div className="card"><span className="big">{num(gp.buildings, 0)}</span><span className="muted">Buildings</span></div>
      <div className="card"><span className="big">{fmt(gp.receiver_gw, 1)}<span className="unit"> GW</span></span><span className="muted">Ray receivers, {fmt(gp.z > 0 ? gp.photons_per_min / gp.z : 0, 2)} photon/cube</span></div>
      <div className="card"><span className="big">{fmt(gp.warpers_per_min, 1)}<span className="unit">/min</span></span><span className="muted">Warpers</span></div>
      <div className="card"><span className="big">{num(gp.power_mw, 0)}<span className="unit"> MW</span></span><span className="muted">Production power</span></div>
    </div>
    <p className="small muted">
      L{gp.level}: speed ×{fmt(gp.speed, 1)}, S(L) = {num(gp.s_level, 0)}. Buildings: production {num(gp.production_buildings, 0)}, miners {num(gp.miners, 0)},
      pumps {num(gp.pumps, 0)}, oil extractors {num(gp.oil_extractors, 0)}, orbital collectors {num(gp.collectors, 0)}, ray receivers {num(gp.receivers, 0)}, stations {num(gp.stations, 1)}.
      LP {num(gp.columns, 0)} columns × {num(gp.rows, 0)} rows, {fmt(gp.solve_ms / 1000, 2)} s.
    </p>
    {gp.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <h4>Finite resources per cube (珍奇 at quota = binding)</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>Resource</th><th className="num">Mined/cube</th><th className="num">Quota/cube</th><th>Use</th></tr></thead>
        <tbody>
          {gp.rares.filter(r => r.quota_per_cube > 0 || r.per_cube > 0).map(r => (
            <tr key={r.item}><td><Icon id={r.item} size={18} />{name(r.item)}</td><td className="num">{fmt(r.per_cube, 3)}</td><td className="num">{fmt(r.quota_per_cube, 3)}</td>
              <td className="small">{r.binding ? <span className="st-warn">at quota</span> : <span className="muted">{fmt(100 * r.per_cube / Math.max(r.quota_per_cube, 1e-12), 0)} %</span>}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>Systems (click for recipes and extraction)</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>System</th><th className="num">ly</th><th className="num">Cubes/min</th><th className="num">Production</th><th className="num">Miners</th><th className="num">Collectors</th><th className="num">Stations</th></tr></thead>
        <tbody>
          {gp.systems.map(s => [
            <tr key={s.star} className="clickable" onClick={() => setSel(sel === s.star ? null : s.star)}>
              <td><span className="caret">{sel === s.star ? '▾' : '▸'}</span>{s.name}</td><td className="num">{fmt(s.distance, 1)}</td><td className="num">{num(s.cubes, 0)}</td>
              <td className="num">{num(s.production_buildings, 0)}</td><td className="num">{num(s.miners, 0)}</td><td className="num">{num(s.collectors, 0)}</td><td className="num">{fmt(s.stations, 1)}</td>
            </tr>,
            sel === s.star && <tr key={`${s.star}-d`}><td colSpan={7} className="small">
              {s.extraction.length > 0 && <div><b>Extraction</b>: {s.extraction.map(e => `${name(e.item.replace(' (gas)', ''))}${e.item.endsWith('(gas)') ? ' (gas)' : ''} ${num(e.rate, 1)}/min${e.cap !== null ? ` of ${num(e.cap, 0)} ${e.limit}` : ''}`).join(' · ')}</div>}
              {s.drops.length > 0 && <div><b>Dark Fog drops</b>: {s.drops.map(d => `${name(d.item)} ${num(d.rate, 1)}/min`).join(' · ')}</div>}
              <div><b>Recipes</b>: {s.recipes.length === 0 ? 'none' : s.recipes.slice(0, 40).map(r => `${recipeName(r.recipe)}${r.mode ? ` [${r.mode}]` : ''} ×${num(r.machines, 1)}`).join(' · ')}{s.recipes.length > 40 ? ` … +${s.recipes.length - 40}` : ''}</div>
            </td></tr>,
          ])}
        </tbody>
      </table>
    </div>
    <h4>Interstellar shipments ({gp.shipments.length}, largest item·ly first)</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>Item</th><th>From → to</th><th className="num">/min</th><th className="num">ly</th></tr></thead>
        <tbody>
          {gp.shipments.slice(0, 30).map((s, i) => (
            <tr key={i}><td><Icon id={s.item} size={18} />{name(s.item)}</td><td>{starName(s.from)} → {starName(s.to)}</td><td className="num">{num(s.rate, 1)}</td><td className="num">{fmt(s.ly, 1)}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
  </>
}

function TotalView({ t, name, recipeName, level }: { t: TotalReport; name: (id: string) => string; recipeName: (id: string) => string; level: number }) {
  if (t.status !== 'optimal') {
    return <div className="warn"><b>{t.status === 'unbounded' ? 'Unlimited' : 'No result'}</b>{t.warnings.map((w, i) => <p key={i}>{w}</p>)}</div>
  }
  const binding = t.finite.filter(u => u.binding)
  return <>
    <div className="cards">
      <div className="card"><span className="big">{compact(t.cubes)}</span><span className="muted">Universe matrices in total{t.levels_done > 0 ? `, ${t.levels_done} vein utilisation levels done` : ', no vein utilisation'}</span></div>
      {level > 1 && <div className="card"><span className="big">{compact(t.cubes_no_util / Math.pow(0.94, level - 1))}</span><span className="muted">with L{level - 1} vein utilisation done (×{fmt(1 / Math.pow(0.94, level - 1), 0)})</span></div>}
      <div className="card"><span className="big">{binding.length}</span><span className="muted">Limiting: {binding.map(u => name(u.item)).join(', ') || '—'}</span></div>
      {t.hub && <div className="card"><span className="big">{compact(t.warpers)}</span><span className="muted">Warpers (hub {t.hub})</span></div>}
    </div>
    <p className="small muted">
      {t.stars} stars. Each level of vein utilisation multiplies the total by 1/0.94 (+6.4 %); with unlimited levels it has no limit, which is what the
      infinite-threshold section below is about (research cubes and their ore are not counted here yet). Unlimited here: {t.unlimited.map(name).join(', ')}. {fmt(t.solve_ms, 0)} ms.
    </p>
    {t.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <h4>Finite stock</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>Resource</th><th className="num">Available</th><th className="num">Used</th><th className="num">Use</th><th className="num">Per cube</th><th className="num">+cubes per 1000 more</th></tr></thead>
        <tbody>
          {t.finite.map(u => (
            <tr key={u.item}><td><Icon id={u.item} size={18} />{name(u.item)}</td><td className="num">{compact(u.available)}</td><td className="num">{compact(u.used)}</td>
              <td className="num">{u.binding ? <span className="st-warn">all, limiting</span> : `${fmt(100 * u.used / Math.max(u.available, 1e-12), 0)} %`}</td>
              <td className="num">{fmt(u.per_cube, 3)}</td><td className="num">{u.binding ? fmt(1000 * u.marginal, 2) : '—'}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>Recipes, machines per 1 cube/min</h4>
    <p className="small muted">The mix that gives the most cubes, scaled to a rate: build this many machines per cube/min you want.</p>
    <p className="small">{t.recipes.slice(0, 60).map(r => `${recipeName(r.recipe)}${r.mode ? ` [${r.mode}]` : ''} ×${fmt(r.machines_per_rate, 2)}`).join(' · ')}{t.recipes.length > 60 ? ` … +${t.recipes.length - 60}` : ''}</p>
    <h4>Raw per cube</h4>
    <p className="small">{t.raw_per_cube.map(r => `${name(r.item)} ${fmt(r.rate, 2)}`).join(' · ')} <span className="muted">(unlimited items: gross intake)</span></p>
  </>
}
