// DSP galaxy tab (星区): seed import, highest cube rate, total, infinite thresholds, multi-system LP, Dark Fog.
// Model: src/planner/dsp_galaxy.rs; spec docs/research/dsp-seed/00-最终报告.md §4–§5, §9.
// Its settings are its own (stored per browser): changing them never re-solves the main plan.
// Text is bilingual: every string goes through tr() (= t() of ./i18n); the Chinese is the original wording (Xinyu 2026-11-23: 「你是不是应该全中文？」).
import { useEffect, useMemo, useRef, useState } from 'react'
import { Icon } from './Icon'
import { lang, t as tr } from './i18n'
import { dn, fmt, itemName, type GameData, type ItemRate, type Settings } from './solver'

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
  star: number; planet?: number | null; name: string; distance: number; cubes: number; production_buildings: number; miners: number; pumps: number; oil_extractors: number; collectors: number
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
interface RecipeUse { recipe: string; mode: string; building: string; machines_per_rate: number }
interface TotalReport {
  status: string; cubes: number; cubes_no_util: number; levels_done: number; research_cubes: number; research_need: number; infinite: boolean; best_levels_done: number; best_cubes: number; stars: number; hub: string | null; warpers: number
  finite: { item: string; available: number; used: number; per_cube: number; binding: boolean; marginal: number }[]
  unlimited: string[]; raw_per_cube: ItemRate[]; recipes: RecipeUse[]
  solve_ms: number; warnings: string[]; error?: string
}
interface RateUse { item: string; source: 'vein' | 'oil' | 'gas' | 'vein+gas' | 'dyson'; cap: number; used: number; per_cube: number; binding: boolean; marginal: number; infinite_from: number | null }
interface RateReport {
  status: string; cubes_per_min: number; level: number; speed: number; stars: number; hub: string | null
  dyson_gw: number; dyson_used_gw: number; dyson: { index: number; name: string; luminosity: number; max_gw: number }[]
  swarm_star: string | null; photons_per_sail: number; sails_per_min: number; swarm_gw: number
  uses: RateUse[]; unlimited: string[]; infinite_level: number | null; dyson_level: number | null
  curve: { level: number; cubes_per_min: number; dyson_bound: boolean }[]
  recipes: RecipeUse[]; raw_per_cube: ItemRate[]; warpers_per_min: number; machines: number; power_gw: number; power_mw_per_rate: number; ceiling_shells: number
  solve_ms: number; warnings: string[]; error?: string
}
interface Opts {
  seed: SeedSettings
  rate_level: number; miners_per_node: number; miner_speed: number; dyson: boolean; dyson_share: number; swarm: boolean
  total_levels: number; total_research: boolean; total_scope: 'all' | 'ticked'; total_warpers: boolean; total_alts: boolean; stock_quotas: boolean; one_planet: boolean
  rate: number; remaining: number; level: number
  goal: 'target' | 'max'; target: number; objective: 'buildings' | 'photons'; rho: number; phi: number
  oil: 'initial' | 'floor'; building_cap: number | null; photon_gw: number | null; vessel_capacity: number; warpers_per_trip: number
  systems: number[] | null
  dark_fog: DarkFog
}

const DEFAULTS: Opts = {
  seed: { seed: 12345678, stars: 32, resource_multiplier: 0.1, hive_initial_colonize: 1, hive_max_density: 1 },
  rate_level: 81, miners_per_node: 1, miner_speed: 100, dyson: true, dyson_share: 100, swarm: true,
  rate: 750, remaining: 100, level: 81,
  total_levels: 0, total_research: true, total_scope: 'all', total_warpers: false, total_alts: true, stock_quotas: true, one_planet: false,
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
      if (!MULTS.includes(o.seed.resource_multiplier)) o.seed.resource_multiplier = 1
      return o
    }
  } catch { /* private mode or bad JSON: defaults */ }
  return DEFAULTS
}

// the game's ten settings (the same list as DSP-Seed-Finder web/src/util.ts; there is no 10×)
const MULTS: number[] = [0.1, 0.5, 0.8, 1, 1.5, 2, 3, 5, 8, 100]
const multLabel = (v: number): string => (v === 0.1 ? tr('0.1× very low', '0.1× 极少') : v === 100 ? tr('Unlimited', '无限') : `${v}×`)
/** BWIKI 黑雾掉落 unlock levels (same table as dsp_galaxy.rs `drop_unlock_level`). */
const DROPS: [string, number][] = [
  ['processor', 9], ['graphene', 9], ['carbon-nanotube', 12], ['df-dark-fog-matrix', 12], ['optical-grating-crystal', 15], ['particle-container', 15],
  ['casimir-crystal', 15], ['df-matter-recombinator', 15], ['strange-matter', 18], ['df-silicon-based-neuron', 18], ['antimatter', 21], ['quantum-chip', 21],
  ['graviton-lens', 21], ['space-warper', 21], ['df-negentropy-singularity', 21], ['unipolar-magnet', 24], ['antimatter-fuel-rod', 24], ['df-core-element', 24],
]
const typeName = (k: string): string => ({
  MainSeqStar: tr('Main-sequence star', '主序星'), GiantStar: tr('Giant star', '巨星'), WhiteDwarf: tr('White dwarf', '白矮星'),
  NeutronStar: tr('Neutron star', '中子星'), BlackHole: tr('Black hole', '黑洞'),
} as Record<string, string>)[k] ?? k
const kindName = (k: string): string => ({
  Ocean: tr('Ocean', '海洋'), Desert: tr('Desert', '沙漠'), Ice: tr('Ice field', '冰原'), Gas: tr('Gas giant', '气态巨星'),
  Vocano: tr('Volcanic', '火山'), Volcano: tr('Volcanic', '火山'),
} as Record<string, string>)[k] ?? k
const sourceName = (k: string): string => ({
  vein: tr('veins', '矿脉'), oil: tr('oil wells', '油井'), gas: tr('gas giant', '气巨'), 'vein+gas': tr('veins + gas giant', '矿脉 + 气巨'), dyson: tr('Dyson sphere', '戴森球'),
} as Record<string, string>)[k] ?? k
const RARE = new Set(['fire-ice', 'kimberlite-ore', 'fractal-silicon', 'organic-crystal', 'optical-grating-crystal', 'spiniform-stalagmite-crystal', 'unipolar-magnet'])
const statusName = (k: string, d?: string): string => (k === 'unbounded' ? tr('Unbounded', '无上限') : k === 'infeasible' ? tr('Infeasible', '无解') : d ?? k)

/** Thousands-separated number with at most `d` decimals. */
const num = (x: number, d = 0) => (Number.isFinite(x) ? (Number(x.toFixed(d)) + 0).toLocaleString('en-US', { maximumFractionDigits: d, minimumFractionDigits: 0 }) : '–')
/** Compact: 亿 / 万 in Chinese, B / M / k in English. */
const compact = (x: number) => {
  const a = Math.abs(x)
  return tr(
    a >= 1e9 ? `${fmt(x / 1e9, 2)} B` : a >= 1e6 ? `${fmt(x / 1e6, a >= 1e8 ? 0 : 1)} M` : a >= 1e3 ? `${fmt(x / 1e3, a >= 1e5 ? 0 : 1)} k` : num(x, 1),
    a >= 1e8 ? `${fmt(x / 1e8, 2)} 亿` : a >= 1e4 ? `${fmt(x / 1e4, a >= 1e6 ? 0 : 1)} 万` : num(x, 1),
  )
}
const lv = (l: number | null | undefined) => (l === null || l === undefined ? '—' : tr(`Lv ${l}`, `${l} 级`))
const gw = (x: number) => (x >= 1000 ? `${fmt(x / 1000, 2)} TW` : `${fmt(x, x >= 100 ? 0 : 1)} GW`)

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
function call<T>(fn: 'galaxy' | 'thresholds' | 'plan' | 'total' | 'rate', payload: unknown): Promise<T> {
  if (!worker) {
    worker = new Worker(new URL('./galaxyWorker.ts', import.meta.url), { type: 'module' })
    worker.onmessage = (e: MessageEvent<{ id: number; out: string }>) => {
      waiting.get(e.data.id)?.(e.data.out)
      waiting.delete(e.data.id)
    }
    // a worker that fails to load or dies answers nothing: end every pending call with an error and
    // start a fresh worker on the next call, so the tab never stays on "计算中"
    worker.onerror = e => {
      const out = JSON.stringify({ error: tr(`Galaxy worker failed: ${e.message || 'could not load'}`, `星区计算线程出错：${e.message || '加载失败'}`) })
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

const Stale = ({ what }: { what: string }) => <p className="warn stale" role="status"><b>{tr('Result out of date', '结果已过期')}</b>{tr(`: the settings changed, press "${what}" to recompute.`, `：设置改了，按「${what}」重新计算。`)}</p>

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
  const [rr, setRr] = useState<RateReport | null>(null)
  const [totKey, setTotKey] = useState('')
  const [rrKey, setRrKey] = useState('')
  const [open, setOpen] = useState<number | null>(null)
  const name = (id: string) => itemName(data, id)
  const recipeName = (id: string) => { const r = data.recipes.find(x => x.id === id); return dn(r, data.game, id) }
  const gen = useRef(0)

  // the recipe side comes from the sidebar (alternates, proliferator, buildings); targets are set by the model
  const plan = useMemo(() => ({ ...settings, game: 'dsp', integer: false, lang: lang() }), [settings])
  // "every recipe": all alternates and both Mk.III modes, so the LP picks what gives the most cubes
  const bestPlan = useMemo(() => (o.total_alts
    ? { ...plan, allow_all_alternates: true, effects: [...new Set([...plan.effects, 'proliferator-3-speed', 'proliferator-3-products'])] }
    : plan), [plan, o.total_alts])
  const darkFog = o.dark_fog

  const importSeed = (s: SeedSettings) => {
    const t = ++gen.current
    setBusy(tr('Generating the galaxy…', '正在生成星区…'))
    setTh(null); setGp(null); setTot(null); setRr(null)
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
  const scopeSystems = o.total_scope === 'all' ? [] : systems

  // what each button sends; a result is shown as out of date when this no longer equals what it was computed from
  const rrPayload = useMemo(() => JSON.stringify({
    seed: o.seed, plan: bestPlan, level: o.rate_level, systems: scopeSystems, miners_per_node: o.miners_per_node, miner_speed: o.miner_speed / 100,
    oil: o.oil, dyson: o.dyson, dyson_share: o.dyson_share / 100, swarm: o.swarm, rho: o.rho, warpers: o.total_warpers, vessel_capacity: o.vessel_capacity, warpers_per_trip: o.warpers_per_trip,
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }), [o.seed, bestPlan, o.rate_level, JSON.stringify(scopeSystems), o.miners_per_node, o.miner_speed, o.oil, o.dyson, o.dyson_share, o.swarm, o.rho, o.total_warpers, o.vessel_capacity, o.warpers_per_trip])
  const runRate = () => {
    const t = gen.current, sent = rrPayload
    setBusy(tr('Finding the highest cube rate…', '正在求最高白糖速率…'))
    void call<RateReport>('rate', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setRr(r); setRrKey(sent) })
  }
  const rrStale = !!rr && !rr.error && rrKey !== rrPayload
  // the headline is computed as soon as the galaxy is there
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { if (g && !g.error && !rr && !busy) runRate() }, [g])

  const thPayload = useMemo(() => JSON.stringify({
    seed: o.seed, plan, rate: o.rate, remaining: o.remaining / 100, stock_override: {}, dark_fog: darkFog, level: Math.max(6, o.level),
  }), [o.seed, plan, o.rate, o.remaining, darkFog, o.level])
  const gpPayload = useMemo(() => JSON.stringify({
    seed: o.seed, plan, goal: o.goal, target: o.target, objective: o.objective, level: o.level, rho: o.rho, phi: o.phi / 100,
    stock_quotas: o.stock_quotas, one_planet: o.one_planet, remaining: o.remaining / 100, systems, oil: o.oil, building_cap: o.building_cap, photon_gw: o.photon_gw,
    vessel_capacity: o.vessel_capacity, warpers_per_trip: o.warpers_per_trip, dark_fog: darkFog,
  }), [o, plan, systems, darkFog])
  const totPayload = useMemo(() => JSON.stringify({
    seed: o.seed, plan: bestPlan, levels_done: o.total_levels, remaining: o.remaining / 100, stock_override: {},
    systems: scopeSystems, warpers: o.total_warpers, vessel_capacity: o.vessel_capacity, warpers_per_trip: o.warpers_per_trip,
    research: o.total_research, rho: o.rho,
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }), [o.seed, bestPlan, o.total_levels, o.total_research, o.rho, o.remaining, JSON.stringify(scopeSystems), o.total_warpers, o.vessel_capacity, o.warpers_per_trip])
  const runTotal = () => {
    const t = gen.current, sent = totPayload
    setBusy(tr('Computing the total…', '正在求总产量…'))
    void call<TotalReport>('total', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setTot(r); setTotKey(sent) })
  }
  const totStale = !!tot && !tot.error && totKey !== totPayload
  const [thKey, setThKey] = useState('')
  const [gpKey, setGpKey] = useState('')
  const runThresholds = () => {
    const t = gen.current, sent = thPayload
    setBusy(tr('Computing infinite thresholds…', '正在算无限阈值…'))
    void call<ThresholdReport>('thresholds', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setTh(r); setThKey(sent) })
  }
  const runPlan = () => {
    const t = gen.current, sent = gpPayload
    setBusy(tr('Solving the multi-system plan…', '正在求多星系规划…'))
    void call<GalaxyPlan>('plan', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setGp(r); setGpKey(sent) })
  }
  const thStale = !!th && !th.error && thKey !== thPayload
  const gpStale = !!gp && !gp.error && gpKey !== gpPayload

  const starName = (i: number) => g?.stars[i]?.name ?? `#${i}`
  const seedDirty = JSON.stringify(seedDraft) !== JSON.stringify(o.seed)
  const prolif = settings.effects.filter(e => e.startsWith('proliferator'))
  const busyNow = !!busy

  return (
    <div className="mixedbelt galaxy">
      <p className="hint">
        {tr('Galaxy planning: enter a seed to generate the whole galaxy (generator ', '星区规划：输入种子生成整个星区（生成器 ')}<a href="https://github.com/DoubleUTH/DSP-Seed-Finder" target="_blank" rel="noreferrer">DoubleUTH/DSP-Seed-Finder</a>{tr(', modified, ', '，有修改，')}{' '}
        <a href={`${import.meta.env.BASE_URL}licenses/DSP-Seed-Finder-LICENSE.txt`} target="_blank" rel="noreferrer">Apache-2.0</a>{tr(', ', '，')}{' '}
        <a href={`${import.meta.env.BASE_URL}licenses/DSP-Seed-Finder-NOTICE.txt`} target="_blank" rel="noreferrer">{tr('change notes', '改动说明')}</a>{tr('), then work out how many cubes (universe matrices) this galaxy can make at most.', '），然后算这个星区最多能做多少白糖（宇宙矩阵）。')}
      </p>

      {/* ---- seed import */}
      <h3>{tr('Seed', '种子')}</h3>
      <div className="mbsettings">
        <label>{tr('Seed', '种子')}<NumField value={seedDraft.seed} lo={0} hi={99999999} integer onCommit={v => setSeedDraft(x => ({ ...x, seed: v ?? x.seed }))} style={{ width: 110 }} /></label>
        <label>{tr('Stars', '恒星数')}<NumField value={seedDraft.stars} lo={32} hi={64} integer onCommit={v => setSeedDraft(x => ({ ...x, stars: v ?? x.stars }))} /></label>
        <label>{tr('Resource multiplier', '资源倍率')}
          <select value={seedDraft.resource_multiplier} onChange={e => setSeedDraft(x => ({ ...x, resource_multiplier: Number(e.target.value) }))}>
            {MULTS.map(v => <option key={v} value={v}>{multLabel(v)}</option>)}
          </select>
        </label>
        <button onClick={() => importSeed(seedDraft)} disabled={busyNow}>{tr('Import seed', '导入种子')}</button>
        <details>
          <summary>{tr('Dark Fog generation settings (hive count)', '黑雾生成设置（巢穴数）')}</summary>
          <div className="mbsettings">
            <label>{tr('Initial colonisation', '初始殖民')}<NumField step={0.25} value={seedDraft.hive_initial_colonize} lo={0} hi={2} onCommit={v => setSeedDraft(x => ({ ...x, hive_initial_colonize: v ?? 1 }))} /></label>
            <label>{tr('Max density', '最大密度')}<NumField step={0.25} value={seedDraft.hive_max_density} lo={0} hi={3} onCommit={v => setSeedDraft(x => ({ ...x, hive_max_density: v ?? 1 }))} /></label>
          </div>
        </details>
      </div>
      <p className="hint">
        {tr('The web and desktop versions generate exactly the same galaxy', '网页和桌面版生成的星区完全一样')}{g && !g.error ? <>{tr(' (fingerprint ', '（指纹 ')}<code>{g.fingerprint}</code>{tr(')', '）')}</> : null}{tr('. It has only been spot-checked against the game: data players posted before the Dark Fog update matches (star luminosity and distance for four seeds; home-star iron ore and crude oil for one seed, coal 8.30 million in game against 8.59 million here). Check important reserves in your own save first.', '。和游戏只做过抽查：黑雾更新前玩家贴出的数据对得上（四个种子的恒星光度和距离；一个种子的母星铁矿和原油，煤 830 万对这里 859 万）。重要的储量请先在自己的存档里看一眼。')}
        {seedDirty && <> <b>{tr('The inputs above differ from the imported galaxy', '上面的输入和已导入的星区不同')}</b>{tr(` (seed ${o.seed.seed}, ${o.seed.stars} stars, ${o.seed.resource_multiplier}×): press "Import seed".`, `（种子 ${o.seed.seed}，${o.seed.stars} 颗星，${o.seed.resource_multiplier}×）：请按「导入种子」。`)}</>}
      </p>
      {busy && <p className="solving" role="status">{busy}</p>}
      {g?.error && <p className="warn">{g.error}</p>}

      {/* ---- highest cube rate (the headline) */}
      {g && !g.error && <>
        <h3>{tr('Highest cube rate', '最高白糖速率')}</h3>
        <p className="hint">
          {tr('Each vein utilisation level multiplies mining consumption by ×0.94 and raises mining speed by +10%. Research cost grows with the square of the level and the effective ore stock grows exponentially, so beyond some level the ore ', '矿物利用每升一级，采矿消耗 ×0.94、采矿速度 +10%。研究消耗按等级平方涨，矿的等效储量按指数涨，所以跑到某一级之后矿就')}<b>{tr('never runs out', '永远用不完')}</b>
          {tr(' (笑笑 BV1QF24YDE7H from 03:20). From then on, how many cubes per minute can be made depends only on the ', '（笑笑 BV1QF24YDE7H 03:20 起）。从那以后白糖每分钟能做多少，只看')}<b>{tr('rate', '速率')}</b>{tr(": how much each vein node yields per minute (faster with level), the rates of oil wells and gas giants, and how many photons the Dyson spheres give: the full-shell power of each star's Dyson shell (from luminosity and maximum radius), plus the Dyson swarm (unlimited solar sails, but the graphene and other inputs for the sails also count against rates). Oceans give unlimited water and sulfuric acid. Machines, solar sails and logistics stations are unlimited.", '：每个矿点每分钟能采多少（随等级变快）、油井和气巨的速率、以及戴森球能给多少光子：每颗恒星的戴森壳满球功率（按光度和最大半径算），再加上戴森云（太阳帆数量不限，但做帆的石墨烯等原料也要按速率算）。海洋（水、硫酸）抽多少都有。机器、太阳帆、物流塔都不限。')}
        </p>
        <div className="mbsettings">
          <label>{tr('Vein utilisation level (the one being researched)', '矿物利用等级（正在研究的一级）')}<NumField value={o.rate_level} lo={1} hi={10000} integer onCommit={v => set({ rate_level: v ?? 81 })} /></label>
          <label>{tr('Large miners per vein node', '每个矿点几台大型采矿机')}<NumField step={1} value={o.miners_per_node} lo={0.1} hi={10} onCommit={v => set({ miners_per_node: v ?? 1 })} /></label>
          <label>{tr('Miner speed %', '采矿机速度 %')}<NumField step={10} value={o.miner_speed} lo={100} hi={300} onCommit={v => set({ miner_speed: v ?? 100 })} /></label>
          <label className="check"><input type="checkbox" checked={o.dyson} onChange={e => set({ dyson: e.target.checked })} /> {tr('Photons limited by full Dyson sphere power', '光子受戴森球满球功率限制')}</label>
          {o.dyson && <label>{tr('Dyson shell built %', '戴森壳建成 %')}<NumField step={10} value={o.dyson_share} lo={0} hi={100} onCommit={v => set({ dyson_share: v ?? 100 })} /></label>}
          {o.dyson && <label className="check"><input type="checkbox" checked={o.swarm} onChange={e => set({ swarm: e.target.checked })} /> {tr('Add a Dyson swarm (sails unlimited, sail inputs count against rates)', '加戴森云（太阳帆不限，帆的原料算进速率）')}</label>}
          <button onClick={runRate} disabled={busyNow || (o.total_scope === 'ticked' && systems.length === 0)}>{tr('Compute max rate', '计算最高速率')}</button>
        </div>
        <CommonSettings o={o} set={set} g={g} systems={systems} prolif={prolif} />
        {rr?.error && <p className="warn">{rr.error}</p>}
        {rrStale && <Stale what={tr('Compute max rate', '计算最高速率')} />}
        {rr && !rr.error && <div className={rrStale ? 'staleresult' : undefined}><RateView r={rr} name={name} recipeName={recipeName} miners={o.miners_per_node} /></div>}
      </>}

      {g && !g.error && <GalaxyTables g={g} name={name} systems={systems} toggle={toggleSystem} open={open} setOpen={setOpen}
        resetSystems={() => set({ systems: null })} />}

      {/* ---- galaxy total */}
      {g && !g.error && <>
        <h3>{tr('Total: how many cubes the reserves make in all', '总量：储量一共能做多少白糖')}</h3>
        <p className="hint">
          {tr('Another question: without upgrading vein utilisation (or only up to some level), how many cubes can the finite veins of the whole galaxy be turned into? Machines and time are unlimited, so oceans, gas giants, oil wells (not mined below the floor) and photons all count as unlimited; only veins run out. Each further vein utilisation level makes every later cube use 6% less ore; but researching level n costs 4000(n−5) cubes, which are also made from ore. Enter the levels already done and the remaining reserve; tick "Count the ore spent on research" and it works out how many more levels are worth researching from now (the cubes spent on research and the ore they use are both deducted). If, from now on, putting all cubes into research never runs the ore out, the total is infinite, which is the premise of "Highest cube rate" above. Only vein utilisation research is counted; levels 1–5, other technologies and the proliferator sprayed on research stations are not.', '另一个问题：不升矿物利用（或只升到某一级）时，整个星区的有限矿脉一共能变成多少白糖。机器和时间不限，所以海洋、气巨、油井（不采到底线以下）和光子都算无限，只有矿脉会用完。每多升一级矿物利用，之后每个白糖少用 6% 的矿；但研究第 n 级要花 4000(n−5) 个白糖，这些白糖也是用矿做的。填已经完成的级数，剩余储量填现在还剩多少；勾上「算上研究耗的矿」就会算：从现在起再研究几级最划算（研究花掉的白糖和它们用的矿都扣掉）。如果从现在起把白糖全拿去研究也永远用不完，总量就是无限的，这就是上面「最高白糖速率」的前提。只算矿物利用的研究，1–5 级和其他科技、给研究站喷的增产剂没算。')}
        </p>
        <div className="mbsettings">
          <label>{tr('Vein utilisation levels done', '已完成的矿物利用级数')}<NumField value={o.total_levels} lo={0} hi={500} integer onCommit={v => set({ total_levels: v ?? 0 })} /></label>
          <label className="check"><input type="checkbox" checked={o.total_research} onChange={e => set({ total_research: e.target.checked })} /> {tr('Count the ore spent on research (from level 6; research ρ is in the common settings)', '算上研究耗的矿（6 级起，研究 ρ 在通用设置里）')}</label>
          <label>{tr('Remaining reserve %', '剩余储量 %')}<NumField value={o.remaining} lo={0} hi={100} onCommit={v => set({ remaining: v ?? 100 })} /></label>
          <button onClick={runTotal} disabled={busyNow || (o.total_scope === 'ticked' && systems.length === 0)}>{tr('Compute total', '计算总量')}</button>
        </div>
        {tot?.error && <p className="warn">{tot.error}</p>}
        {totStale && <Stale what={tr('Compute total', '计算总量')} />}
        {tot && !tot.error && <div className={totStale ? 'staleresult' : undefined}><TotalView t={tot} name={name} level={o.level} recipeName={recipeName} research={o.total_research} /></div>}
      </>}

      {/* ---- thresholds */}
      {g && !g.error && <>
        <h3>{tr('Infinite thresholds: from which level each ore never runs out', '无限阈值：每种矿从几级起用不完')}</h3>
        <p className="hint">
          {tr('Assume that from level L on, all cubes go into researching vein utilisation. A resource used at a per cube forever never runs out if a·ρ·S(L) ≤ B, with S(L) = Σ', '假设从 L 级起所有白糖都拿去研究矿物利用，一种资源按「每个白糖用 a 个」一直用下去也用不完的条件是 a·ρ·S(L) ≤ B，S(L) = Σ')}<sub>n≥L</sub> 4000(n−5)·0.94<sup>n−1</sup>{tr(' (S(6) = 815 449). With Mk.III proliferator sprayed on research stations ρ = 0.8 (hash ×1.25, unconfirmed). B is the remaining reserve; for crude oil only the part above the 2500 floor of each well counts. 笑笑 waits about five more levels after unlocking as a buffer (L80 → L85).', '（S(6) = 815 449）。研究站喷 Mk.III 增产剂时 ρ = 0.8（哈希 ×1.25，存疑）。B 是剩余储量，原油只算每个油井 2500 底线以上的部分。笑笑会在放开等级之后再多等约五级当缓冲（L80 → L85）。')}
        </p>
        <div className="mbsettings">
          <label>{tr('Cubes/min (converted to hours)', '白糖/分钟（换算成小时）')}<NumField value={o.rate} lo={1} hi={1e7} onCommit={v => set({ rate: v ?? 750 })} /></label>
          <label>{tr('Remaining reserve %', '剩余储量 %')}<NumField value={o.remaining} lo={0} hi={100} onCommit={v => set({ remaining: v ?? 100 })} /></label>
          <label>{tr('Quota level L', '配额等级 L')}<NumField value={o.level} lo={1} hi={2000} integer onCommit={v => set({ level: v ?? 81 })} /></label>
          <button onClick={runThresholds} disabled={busyNow}>{tr('Compute thresholds', '计算阈值')}</button>
        </div>
        {th?.error && <p className="warn">{th.error}</p>}
        {thStale && <Stale what={tr('Compute thresholds', '计算阈值')} />}
        {th && !th.error && <div className={thStale ? 'staleresult' : undefined}><ThresholdTable th={th} name={name} gas={new Set(g.gas_totals.map(x => x.item))} /></div>}
      </>}

      {/* ---- Dark Fog */}
      {g && !g.error && <DarkFogBox g={g} df={darkFog} set={df => set({ dark_fog: df })} name={name} />}

      {/* ---- multi-system LP */}
      {g && !g.error && <>
        <h3>{tr('Multi-system plan: what each system makes and how it is shipped', '多星系规划：哪个星系做什么、怎么运')}</h3>
        <p className="hint">
          {tr(`A linear programme over the ${systems.length} systems ticked above (all planets of a system together; 笑笑 opens only one planet per system at full-strength Dark Fog, which the option “One planet per system” under More enforces): each system's output, interstellar transport (logistics station count by distance, fitted to 笑笑's "4 quad-stack blue belts within 7 ly, 3 at 12 ly"; each trip costs warpers), finite reserves split among the systems by the quota m·ρ·S(L) ≤ B·z, oil and gas giants at the level-L rate (×${fmt(1 + 0.1 * (o.level - 1), 1)}), water and sulfuric acid only in systems with an ocean. Power plants are not modelled; photons are unlimited when no cap is set.`, `对上面勾选的 ${systems.length} 个星系做一个线性规划（一个星系的所有行星合在一起算；笑笑在满强度黑雾下每个星系只开一颗行星，可在「更多」里勾选「每个星系只开一颗行星」）：每个星系的产量、星际运输（物流塔数按距离，按笑笑「7 光年内 4 条四堆叠蓝带、12 光年 3 条」拟合；每趟耗翘曲器）、有限储量按配额 m·ρ·S(L) ≤ B·z 分到各星系、原油和气巨按 L 级速率（×${fmt(1 + 0.1 * (o.level - 1), 1)}）、有海洋的星系才有水和硫酸。发电厂没有建模；光子不设上限时不限。`)}
        </p>
        <div className="mbsettings">
          <label>{tr('Goal', '目标')}
            <select value={o.goal} onChange={e => set({ goal: e.target.value as Opts['goal'] })}>
              <option value="target">{tr('Given cubes/min, fewest buildings', '给定白糖/分钟，建筑最少')}</option>
              <option value="max">{tr('Maximum cubes/min (needs a building or power cap)', '白糖/分钟最大（需要建筑或功率上限）')}</option>
            </select>
          </label>
          {o.goal === 'target' && <label>{tr('Target cubes/min', '目标白糖/分钟')}<NumField value={o.target} lo={0} hi={1e7} onCommit={v => set({ target: v ?? 1000 })} /></label>}
          {o.goal === 'target' && <label>{tr('Minimise', '最少化')}
            <select value={o.objective} onChange={e => set({ objective: e.target.value as Opts['objective'] })}>
              <option value="buildings">{tr('Buildings', '建筑数')}</option>
              <option value="photons">{tr('Photons, then buildings', '光子，其次建筑数')}</option>
            </select>
          </label>}
          <label>{tr('Level L', '等级 L')}<NumField value={o.level} lo={1} hi={2000} integer onCommit={v => set({ level: v ?? 81 })} /></label>
          <label>{tr('Building cap', '建筑上限')}<NumField placeholder={tr('unlimited', '不限')} nullable value={o.building_cap} lo={0} hi={1e8} onCommit={v => set({ building_cap: v })} /></label>
          <label>{tr('Dyson sphere GW', '戴森球 GW')}<NumField placeholder={tr('unlimited', '不限')} nullable value={o.photon_gw} lo={0} hi={1e6} onCommit={v => set({ photon_gw: v })} /></label>
          <button onClick={runPlan} disabled={busyNow || systems.length === 0}>{tr('Plan the galaxy', '规划星区')}</button>
          <details>
            <summary>{tr('More: quotas, cubes not spent on research', '更多：配额、不拿去研究的白糖')}</summary>
            <div className="mbsettings">
              <label className="check"><input type="checkbox" checked={o.stock_quotas} onChange={e => set({ stock_quotas: e.target.checked })} /> {tr('Reserve quotas (off = rates only)', '储量配额（关掉 = 只看速率）')}</label>
              <label className="check"><input type="checkbox" checked={o.one_planet} onChange={e => set({ one_planet: e.target.checked })} /> {tr('One planet per system (gas giants still collected)', '每个星系只开一颗行星（气巨照常采集）')}</label>
              <label>{tr('φ cubes not spent on research %', 'φ 不研究的白糖 %')}<NumField value={o.phi} lo={0} hi={95} onCommit={v => set({ phi: v ?? 0 })} /></label>
            </div>
          </details>
        </div>
        {gp?.error && <p className="warn">{gp.error}</p>}
        {gpStale && <Stale what={tr('Plan the galaxy', '规划星区')} />}
        {gp && !gp.error && <div className={gpStale ? 'staleresult' : undefined}><PlanView gp={gp} name={name} starName={starName} recipeName={recipeName} /></div>}
      </>}
    </div>
  )
}

/** Settings shared by the rate, total and plan: which stars, recipes, oil, research ρ, logistics. */
function CommonSettings({ o, set, g, systems, prolif }: { o: Opts; set: (p: Partial<Opts>) => void; g: Galaxy; systems: number[]; prolif: string[] }) {
  return (
    <details>
      <summary>{tr('Common settings: which stars, recipes, oil, research, transport', '通用设置：哪些星、配方、原油、研究、运输')}</summary>
      <div className="mbsettings">
        <label>{tr('Which stars', '算哪些星')}
          <select value={o.total_scope} onChange={e => set({ total_scope: e.target.value as Opts['total_scope'] })}>
            <option value="all">{tr(`All ${g.stars.length}`, `全部 ${g.stars.length} 颗`)}</option>
            <option value="ticked">{tr(`The ${systems.length} systems ticked in the table below`, `下表勾选的 ${systems.length} 个星系`)}</option>
          </select>
        </label>
        <label className="check"><input type="checkbox" checked={o.total_alts} onChange={e => set({ total_alts: e.target.checked })} /> {tr('All recipes + Mk.III proliferator (used only where it yields more cubes)', '所有配方 + Mk.III 增产剂（只在能多出白糖时才用）')}</label>
        {!o.total_alts && <span className="small muted">{tr(`Sidebar recipes: ${prolif.length ? `proliferator ${prolif.join(' + ')}` : 'no proliferator sprayed'}`, `用侧栏的配方：${prolif.length ? `增产剂 ${prolif.join(' + ')}` : '不喷增产剂'}`)}</span>}
        <label>{tr('Oil rate', '原油速率')}
          <select value={o.oil} onChange={e => set({ oil: e.target.value as Opts['oil'] })}>
            <option value="initial">{tr('Initial well rate (upper bound)', '油井初始速率（上限）')}</option>
            <option value="floor">{tr('Mined down to the floor: 0.1/s per well', '采到底线：每个油井 0.1/秒')}</option>
          </select>
        </label>
        <label>{tr('Research ρ', '研究 ρ')}
          <select value={o.rho} onChange={e => set({ rho: Number(e.target.value) })}>
            <option value={1}>{tr('1 (research stations not sprayed)', '1（研究站不喷）')}</option>
            <option value={0.8}>{tr('0.8 (sprayed with Mk.III)', '0.8（喷 Mk.III）')}</option>
          </select>
        </label>
        <label className="check"><input type="checkbox" checked={o.total_warpers} onChange={e => set({ total_warpers: e.target.checked })} /> {tr('Interstellar transport costs warpers (off: slow flight, costs nothing)', '星际运输耗翘曲器（关掉：慢速飞，不耗东西）')}</label>
        <label>{tr('Load per ship', '每船装载')}<NumField value={o.vessel_capacity} lo={1} hi={10000} onCommit={v => set({ vessel_capacity: v ?? 400 })} /></label>
        <label>{tr('Warpers per trip', '每趟翘曲器')}<NumField step={1} value={o.warpers_per_trip} lo={0} hi={4} onCommit={v => set({ warpers_per_trip: v ?? 2 })} /></label>
      </div>
    </details>
  )
}

function RateView({ r, name, recipeName, miners }: { r: RateReport; name: (id: string) => string; recipeName: (id: string) => string; miners: number }) {
  if (r.status !== 'optimal') {
    return <div className="warn"><b>{statusName(r.status)}</b>{r.warnings.map((w, i) => <p key={i}>{w}</p>)}</div>
  }
  const limits = r.uses.filter(u => u.binding)
  const dysonBound = limits.some(u => u.item === 'critical-photon')
  const label = (u: RateUse) => (u.item === 'critical-photon' ? tr('Dyson sphere (photons)', '戴森球（光子）') : `${name(u.item)}${tr(' (', '（')}${sourceName(u.source)}${tr(')', '）')}`)
  const ok = r.infinite_level !== null && r.level >= r.infinite_level
  return <>
    <div className="cards">
      <div className="card"><span className="big">{num(r.cubes_per_min, 0)}<span className="unit">{tr('/min', '/分钟')}</span></span><span className="muted">{tr(`Highest cubes (level ${r.level}, mining ×${fmt(r.speed, 1)})`, `最高白糖（${r.level} 级，采矿 ×${fmt(r.speed, 1)}）`)}</span></div>
      <div className="card"><span className="big">{gw(r.dyson_used_gw + r.swarm_gw)}</span><span className="muted">{tr('Dyson sphere power used', '用掉的戴森球功率')}{r.dyson_gw > 0 ? tr(` (Dyson shell ${gw(r.dyson_used_gw)} / full ${gw(r.dyson_gw)}${r.swarm_gw > 0 ? `, Dyson swarm ${gw(r.swarm_gw)}` : ''})`, `（戴森壳 ${gw(r.dyson_used_gw)} / 满 ${gw(r.dyson_gw)}${r.swarm_gw > 0 ? `，戴森云 ${gw(r.swarm_gw)}` : ''}）`) : ''}</span></div>
      {r.ceiling_shells > 0 && <div className="card"><span className="big">{num(r.ceiling_shells, 0)}<span className="unit">{tr('/min', '/分钟')}</span></span><span className="muted">{tr('Ceiling: the cubes the Dyson shells can sustain with unlimited vein utilisation', '极限：矿物利用无限高时，戴森壳能撑的白糖')}</span></div>}
      <div className="card"><span className="big">{lv(r.infinite_level)}</span><span className="muted">{tr('From this level the veins in use never run out', '从这级起所用矿脉都用不完')}</span></div>
      {r.dyson_gw > 0 && <div className="card"><span className="big">{lv(r.dyson_level)}</span><span className="muted">{tr('From this level the Dyson shells are fully used', '从这级起戴森壳用满')}</span></div>}
    </div>
    <p>
      <b>{tr('What holds it back: ', '卡住它的是：')}</b>{limits.length ? limits.map(label).join(tr(', ', '、')) : '—'}{tr('.', '。')}
      {dysonBound
        ? <> {tr('The Dyson shell is already fully used: more vein utilisation only raises cubes slowly, through the Dyson swarm (more solar sails) and faster mining.', '戴森壳已经用满：再升矿物利用，白糖只能靠戴森云（多发太阳帆）和更快的开采慢慢涨。')}</>
        : <> {tr(`These are all rate limits: each vein utilisation level makes mining, oil wells and gas giants 10% faster (of the base rate) and cubes rise with it; the Dyson shell ${r.dyson_level ? `is fully used at about level ${r.dyson_level}` : 'is nowhere near fully used'}.`, `这些都是速率上限：矿物利用每升一级，采矿、油井和气巨都快 10%（按基础速率），白糖跟着涨；戴森壳${r.dyson_level ? `约到 ${r.dyson_level} 级才用满` : '还远没用满'}。`)}</>}
      {r.swarm_gw > 0 && r.swarm_star && <> {tr(`The Dyson swarm sits at the most luminous star, ${r.swarm_star}: ${num(r.sails_per_min, 0)} sails per minute.`, `戴森云放在光度最高的 ${r.swarm_star}：每分钟发 ${num(r.sails_per_min, 0)} 张帆。`)}</>}
    </p>
    {r.ceiling_shells > 0 && <p className="small">
      {tr("The higher the vein utilisation, the faster the mining and the more cubes; but the Dyson shell power is fixed, so there is a limit: with infinite level and mining no longer a bottleneck, this galaxy's Dyson shells sustain at most ", '矿物利用越高，开采越快，白糖越多；但戴森壳的功率是固定的，所以总有一个极限：等级无限高、开采不再卡时，这个星区的戴森壳最多撑 ')}<b>{num(r.ceiling_shells, 0)}</b>{tr(` cubes/min (factory power ${fmt(r.power_mw_per_rate, 1)} MW per cube/min also comes from the Dyson sphere). `, ` 白糖/分钟（工厂用电 ${fmt(r.power_mw_per_rate, 1)} MW / 每白糖每分钟，也从戴森球来）。`)}{r.swarm_gw > 0 ? tr('Adding a Dyson swarm gives more.', '再加戴森云还能更多。') : ''}
    </p>}
    {!ok && r.infinite_level !== null && <p className="warn">{tr(`At level ${r.level} this recipe set is not yet infinite: only from level ${r.infinite_level} do the ores last forever (before that, this rate would use up the reserves).`, `在 ${r.level} 级时这个配方组合还没到无限：要从 ${r.infinite_level} 级起才永远用不完（之前按这个速率会把储量用掉）。`)}</p>}
    {r.infinite_level === null && <p className="warn">{tr('Some vein in use runs out at any level; see "From which level infinite" in the table below.', '有一种用到的矿脉无论升到几级都会用完，见下表「从几级起无限」。')}</p>}
    {r.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <h4>{tr('Change with level', '随等级的变化')}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th className="num">{tr('Vein utilisation level', '矿物利用等级')}</th><th className="num">{tr('Mining speed', '采矿速度')}</th><th className="num">{tr('Max cubes/min', '最高白糖/分钟')}</th><th>{tr('Limit', '限制')}</th></tr></thead>
        <tbody>
          {r.curve.map(p => (
            <tr key={p.level} className={p.level === r.level ? 'sel' : undefined}>
              <td className="num">{p.level}{p.level === r.level ? ' ←' : ''}</td><td className="num">×{fmt(1 + 0.1 * (p.level - 1), 1)}</td>
              <td className="num">{num(p.cubes_per_min, 0)}</td><td className="small">{p.dyson_bound ? tr('Mining rate + Dyson shell full', '开采速率 + 戴森壳已满') : tr('Mining rate', '开采速率')}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>{tr(`Rate limits per resource (${r.stars} stars)`, `各资源的速率上限（${r.stars} 颗星）`)}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>{tr('Resource', '资源')}</th><th>{tr('Source', '来源')}</th><th className="num">{tr('Limit/min', '上限/分钟')}</th><th className="num">{tr('Used', '用了')}</th><th className="num">{tr('Per cube', '每个白糖')}</th><th className="num">{tr('Extra cubes per +1000/min', '每多 1000/分钟 多几个白糖')}</th><th className="num">{tr('From which level infinite', '从几级起无限')}</th></tr></thead>
        <tbody>
          {r.uses.filter(u => u.cap > 0 || u.used > 0).map(u => (
            <tr key={u.item}>
              <td><Icon id={u.item} size={18} />{name(u.item)}{RARE.has(u.item) ? <span className="tag">{tr('rare', '珍奇')}</span> : null}</td>
              <td className="small">{sourceName(u.source)}</td>
              <td className="num">{compact(u.cap)}</td>
              <td className="num">{u.binding ? <span className="st-warn">{tr('full, binding', '用满，卡住')}</span> : `${fmt(100 * u.used / Math.max(u.cap, 1e-12), 0)} %`}</td>
              <td className="num">{fmt(u.per_cube, 3)}</td>
              <td className="num">{u.binding ? fmt(1000 * u.marginal, 1) : '—'}</td>
              <td className="num">{u.source === 'vein' || u.source === 'oil' || u.source === 'vein+gas' ? (u.used > 0 ? lv(u.infinite_from) : '—') : tr('always', '一直')}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
    <p className="small muted">
      {tr(`Vein limit = nodes × 60/min (large miner at 100%) × ${miners} miners/node × mining speed. In the game miner ranges can overlap and each miner works at full speed on every overlapped node (dsp-wiki), so this really depends on how many you place; by default each node counts as 1 miner. Gas giants: 40 orbital collectors each, minus 30 MW self-use. Oil at the initial well rate (wells barely decay once vein utilisation is high); to be conservative, set the floor 0.1/s in the common settings. The power of production buildings (${gw(r.power_gw)}) is also drawn from the Dyson sphere; the power of miners, water pumps and logistics stations is not counted. Unlimited sources: ${r.unlimited.map(name).join(', ') || '—'}. ${r.hub ? `Transport is centred on ${r.hub}, warpers ${fmt(r.warpers_per_min, 1)}/min.` : ''}`, `矿脉上限 = 矿点数 × 60/分钟（大型采矿机 100%）× ${miners} 台/矿点 × 采矿速度。游戏里采矿机范围可以重叠，每台都按重叠的矿点全速采（dsp-wiki），所以这一项其实取决于你摆多少台；默认每个矿点算 1 台。气巨按每颗 40 个轨道采集器、扣掉 30 MW 自耗。原油按油井初始速率（矿物利用高了以后油井几乎不再衰减）；想保守就在通用设置里改成底线 0.1/秒。生产建筑的用电（${gw(r.power_gw)}）也算在戴森球上；采矿机、抽水站和物流塔的用电没算。无限的来源：${r.unlimited.map(name).join('、') || '—'}。${r.hub ? `运输以 ${r.hub} 为中心，翘曲器 ${fmt(r.warpers_per_min, 1)}/分钟。` : ''}`)}
      {' '}{fmt(r.solve_ms, 0)} ms{tr('.', '。')}
    </p>
    {r.dyson.length > 0 && <>
      <h4>{tr('Full-sphere Dyson shell power (top 12 stars)', '戴森壳满球功率（前 12 颗）')}</h4>
      <div className="tablewrap">
        <table className="mbtable">
          <thead><tr><th>{tr('Star', '恒星')}</th><th className="num">{tr('Luminosity', '光度')}</th><th className="num">{tr('Full-sphere power', '满球功率')}</th></tr></thead>
          <tbody>
            {r.dyson.slice(0, 12).map(d => <tr key={d.index}><td>{d.name}</td><td className="num">{fmt(d.luminosity, 3)}</td><td className="num">{gw(d.max_gw)}</td></tr>)}
          </tbody>
        </table>
      </div>
      <p className="small muted">
        {tr("Full sphere = 10 shell layers, each 1000 m further in from the maximum orbit radius, every layer fully covered, 60 nodes (per layer: shell surface points 52·r²/10⁴ × 15 kW, frame 1.22·r + nodes 1800 structure points × 96 kW, then × luminosity; an in-game fit to the Bilibili wiki 'Dyson sphere power output', error about ±12%). A solar sail is 36 kW × luminosity, lifetime 9000 s with all upgrades. A ray receiver yields 1 photon per 1.2 GJ (120 MW → 6/min); a graviton lens only lets one receiver receive more and does not change the energy per photon; reception efficiency is taken as 100% with all upgrades (in game it rises with the 'Ray transmission efficiency' technology; below that it must be discounted). Not counted: how many ray receivers each system can hold (planet surfaces are limited; the tens of TW of a big giant may not fit on one system's planets) and shell layers blocked by planet orbits.", '满球 = 10 层壳，从最大轨道半径起每层往里 1000 米，每层壳面铺满、60 个节点（每层：壳面点 52·r²/10⁴ × 15 kW，框架 1.22·r + 节点 1800 个结构点 × 96 kW，再 × 光度；按 B 站 wiki「戴森球发电量」的游戏内拟合，误差约 ±12%）。太阳帆 36 kW × 光度，寿命按满科技 9000 秒。射线接收站每 1.2 GJ 出 1 个光子（120 MW → 6 个/分钟），引力透镜只是让一台接收站收得更多，不改变每个光子要的能量；接收效率按满科技 100% 计（游戏里效率随「射线传输效率」科技提高，没满时要打折）。没算的：每个星系能摆多少射线接收站（行星表面有限，大巨星的几十 TW 一个星系的行星未必收得下）、行星轨道挡住的壳层。')}
      </p>
    </>}
    <h4>{tr('Recipes: machines per 1 cube/min', '配方：每 1 白糖/分钟要几台机器')}</h4>
    <p className="small">{r.recipes.slice(0, 60).map(x => `${recipeName(x.recipe)}${x.mode ? tr(` [${x.mode}]`, `［${x.mode}］`) : ''} ×${fmt(x.machines_per_rate, 2)}`).join(' · ')}{r.recipes.length > 60 ? tr(` … ${r.recipes.length - 60} more`, ` … 还有 ${r.recipes.length - 60} 个`) : ''}</p>
    <h4>{tr('Raw inputs per cube', '每个白糖的原料')}</h4>
    <p className="small">{r.raw_per_cube.map(x => `${name(x.item)} ${fmt(x.rate, 2)}`).join(' · ')}</p>
  </>
}

function GalaxyTables({ g, name, systems, toggle, open, setOpen, resetSystems }: {
  g: Galaxy; name: (id: string) => string; systems: number[]; toggle: (i: number) => void; open: number | null; setOpen: (i: number | null) => void; resetSystems: () => void
}) {
  const on = new Set(systems)
  const mag = g.totals.find(t => t.item === 'unipolar-magnet')?.stock ?? 0
  return <>
    <h3>{tr('Galaxy resources', '星区资源')}</h3>
    <div className="cards">
      <div className="card"><span className="big">{g.stars.length}</span><span className="muted">{tr('Stars', '恒星')}</span></div>
      <div className="card"><span className="big">{g.stars.reduce((a, s) => a + s.planets.length, 0)}</span><span className="muted">{tr('Planets', '行星')}</span></div>
      <div className="card"><span className="big">{g.gas_giants}</span><span className="muted">{tr('Gas giants', '气态巨星')}</span></div>
      <div className="card"><span className="big">{fmt(g.oil_rate, 1)}<span className="unit">{tr('/s', '/秒')}</span></span><span className="muted">{tr(`Crude oil, ${g.seeps} wells`, `原油，${g.seeps} 个油井`)}</span></div>
      <div className="card"><span className="big">{compact(mag)}</span><span className="muted">{tr('Unipolar magnets', '单极磁石')}</span></div>
      <div className="card"><span className="big">{fmt(g.generate_ms / 1000, 2)}<span className="unit">{tr(' s', ' 秒')}</span></span><span className="muted">{tr('Generation time', '生成用时')}</span></div>
    </div>
    <h4>{tr('Galaxy-wide reserves', '全星区储量')}{g.rare_tier ? tr(' (rare tier: oil ×0.5, gases ×0.8)', '（稀有档：原油 ×0.5、气体 ×0.8）') : ''}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>{tr('Resource', '资源')}</th><th className="num">{tr('Reserve', '储量')}</th><th className="num">{tr('Planets', '行星数')}</th></tr></thead>
        <tbody>
          {g.totals.map(t => (
            <tr key={t.item}><td><Icon id={t.item} size={18} />{name(t.item)}{RARE.has(t.item) ? <span className="tag">{tr('rare', '珍奇')}</span> : null}</td>
              <td className="num">{compact(t.stock)}</td><td className="num">{t.planets}</td></tr>
          ))}
          {g.gas_totals.map(t => (
            <tr key={`gas-${t.item}`}><td><Icon id={t.item} size={18} />{name(t.item)} <span className="muted">{tr('gas giant', '气巨')}</span></td>
              <td className="num">{tr('base', '基础')} {fmt(t.base, 2)}{tr('/s', '/秒')}</td><td className="num">—</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>{tr(`Systems · tick = in the plan (${systems.length})`, `星系 · 勾选 = 参与规划（${systems.length}）`)}<button className="tabbtn" onClick={resetSystems}>{tr('Auto-pick', '自动挑选')}</button></h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>{tr('Plan', '规划')}</th><th>{tr('Star', '恒星')}</th><th className="num">{tr('ly', '光年')}</th><th className="num">{tr('Luminosity', '光度')}</th><th className="num">{tr('Hives', '巢穴')}</th><th>{tr('Resources', '资源')}</th></tr></thead>
        <tbody>
          {g.stars.map(s => {
            const items = new Set<string>()
            s.planets.forEach(p => { p.veins.forEach(v => items.add(v.item)); p.gases.forEach(x => items.add(x.item)); if (p.ocean) items.add(p.ocean) })
            const rare = [...items].filter(i => RARE.has(i) || i === 'sulfuric-acid')
            return [
              <tr key={s.index} className="clickable" onClick={() => setOpen(open === s.index ? null : s.index)}>
                <td onClick={e => e.stopPropagation()}><input type="checkbox" aria-label={tr(`Plan ${s.name}`, `规划 ${s.name}`)} checked={on.has(s.index)} onChange={() => toggle(s.index)} /></td>
                <td><span className="caret">{open === s.index ? '▾' : '▸'}</span>{s.name} <span className="muted">{typeName(s.star_type)} {s.spectr}</span></td>
                <td className="num">{fmt(s.distance, 1)}</td><td className="num">{fmt(s.luminosity, 2)}</td>
                <td className="num">{s.initial_hives}/{s.max_hives}</td>
                <td className="small">{rare.map(i => <Icon key={i} id={i} size={16} />)}{rare.length === 0 ? <span className="muted">{tr('only common resources', '只有普通资源')}</span> : null}</td>
              </tr>,
              open === s.index && <tr key={`${s.index}-p`}><td colSpan={6} className="small">
                {s.planets.map(p => (
                  <div key={p.index}><b>P{p.index}</b> {kindName(p.kind)} · {p.theme}{p.ocean ? tr(` · ${name(p.ocean)} ocean`, ` · ${name(p.ocean)}海`) : ''}
                    {p.gases.length > 0 && <> · {tr('gas', '气体')} {p.gases.map(x => `${name(x.item)} ${fmt(x.base, 3)}`).join(tr(', ', '、'))}</>}
                    {p.veins.length > 0 && <> · {p.veins.map(v => v.item === 'crude-oil' ? tr(`${name(v.item)} ${fmt(v.amount * 4e-5, 2)}/s (${v.nodes} wells)`, `${name(v.item)} ${fmt(v.amount * 4e-5, 2)}/秒（${v.nodes} 井）`) : tr(`${name(v.item)} ${compact(v.amount)} (${v.nodes} nodes)`, `${name(v.item)} ${compact(v.amount)}（${v.nodes} 点）`)).join(tr(', ', '、'))}</>}
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
      {tr('All quotas satisfied at once (linear programme, fewest buildings, 1 cube/min): feasible from ', '所有配额同时满足（线性规划，建筑最少，1 白糖/分钟）：从 ')}<b>{lv(th.feasible_from[0])}</b>{tr(' (ρ = 1) / ', '（ρ = 1）/ ')}<b>{lv(th.feasible_from[1])}</b>{tr(' (ρ = 0.8).', '（ρ = 0.8）起可行。')}
      {th.feasible_from[0] !== null && th.raw_at_feasible.length > 0 && <> {tr(`At ${lv(th.feasible_from[0])} each cube uses ${fmt(th.raw_at_feasible.find(r => r.item === 'crude-oil')?.rate ?? 0, 2)} crude oil and ${fmt(th.raw_at_feasible.find(r => r.item === 'unipolar-magnet')?.rate ?? 0, 2)} unipolar magnets (up to the quota).`, `在 ${lv(th.feasible_from[0])} 时每个白糖用原油 ${fmt(th.raw_at_feasible.find(r => r.item === 'crude-oil')?.rate ?? 0, 2)}、单极磁石 ${fmt(th.raw_at_feasible.find(r => r.item === 'unipolar-magnet')?.rate ?? 0, 2)}（用到配额为止）。`)}</>}
    </p>
    {th.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <div className="tablewrap">
      <table className="mbtable threshold">
        <thead>
          <tr><th rowSpan={2}>{tr('Resource', '资源')}</th><th className="num" rowSpan={2}>{tr('Reserve B', '储量 B')}</th><th className="num" rowSpan={2}>{tr('Per cube a', '每个白糖 a')}</th>
            <th colSpan={3}>ρ = 1</th><th colSpan={3}>ρ = 0.8</th><th className="num" rowSpan={2}>{tr(`Quota per cube at level ${th.level}`, `${th.level} 级配额/白糖`)}</th></tr>
          <tr><th className="num">{tr('Unlock', '放开')}</th><th className="num">{tr('Cubes', '白糖')}</th><th className="num">{tr('Hours', '小时')}</th><th className="num">{tr('Unlock', '放开')}</th><th className="num">{tr('Cubes', '白糖')}</th><th className="num">{tr('Hours', '小时')}</th></tr>
        </thead>
        <tbody>
          {rows.map(r => (
            <tr key={r.item}>
              <td><Icon id={r.item} size={18} />{name(r.item)}{gas.has(r.item) ? <span className="tag" title={tr('Gas giants also produce it, so the rate never runs out; this row counts only vein reserves', '气巨也产，速率永远不会用完；这一行只算矿脉储量')}>{tr('+ gas giant', '+ 气巨')}</span> : null}</td>
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
    <p className="hint">{tr('Unlock = the first level from which S(L) lets a cube use a without limit; cubes = the 2000(L−6)(L−5)·ρ cubes it costs to research up to that level. Before that the linear programme still uses it, but not beyond the quota B/(ρ·S(L)); the best approach is "keep it for now, unlock later" (report §4.3).', '放开 = S(L) 允许按每个白糖 a 不受限使用的第一级；白糖 = 研究到这一级要花的 2000(L−6)(L−5)·ρ 个。在那之前线性规划仍会用它，但不超过配额 B/(ρ·S(L))；最好的做法是「先留着，晚点放开」（报告 §4.3）。')}</p>
  </>
}

function DarkFogBox({ g, df, set, name }: { g: Galaxy; df: DarkFog; set: (d: DarkFog) => void; name: (id: string) => string }) {
  const [item, setItem] = useState(DROPS[15][0])
  return (
    <details className="dfbox" open={df.enabled}>
      <summary>{tr('Dark Fog drops as a source', '黑雾掉落当作来源')} {df.enabled ? tr(`· on, ${df.bases.reduce((a, b) => a + b[1], 0)} bases`, `· 开，${df.bases.reduce((a, b) => a + b[1], 0)} 个基地`) : tr('· off', '· 关')}</summary>
      <p className="hint">
        {tr('There is no public data on drop rates (report §5.4): enter what you measured in game as drops per ground base per minute, and the vein utilisation level being researched when you measured (L = the level being researched, L − 1 levels done). Ordinary items are scaled by (1 + 0.04 (L − 1)); Dark Fog-only items are not. An item only drops from the matching Dark Fog level (BWIKI Dark Fog drops). Used in the infinite-threshold table and the multi-system plan.', '掉落速率没有公开数据（报告 §5.4）：填你在游戏里实测的每个地面基地每分钟掉落，以及测的时候正在研究的矿物利用等级（L = 正在研究的一级，已完成 L − 1 级）。普通物品按 (1 + 0.04 (L − 1)) 放大，黑雾专属物品不放大。物品要到对应的黑雾等级才掉（BWIKI 黑雾掉落）。用在无限阈值表和多星系规划里。')}
      </p>
      <div className="mbsettings">
        <label className="row"><input type="checkbox" checked={df.enabled} onChange={e => set({ ...df, enabled: e.target.checked })} /> {tr('Use drops', '使用掉落')}</label>
        <label>{tr('Dark Fog level', '黑雾等级')}<NumField value={df.level} lo={0} hi={30} integer onCommit={v => set({ ...df, level: v ?? 24 })} /></label>
        <label>{tr('Level L when measured', '测量时的等级 L')}<NumField value={df.measured_at_level} lo={1} hi={2000} integer onCommit={v => set({ ...df, measured_at_level: v ?? 1 })} /></label>
      </div>
      <h4>{tr('Farmed bases', '刷的基地')}</h4>
      <ul className="plain">
        {df.bases.map(([s, n], k) => (
          <li key={k} className="row small">
            <select value={s} onChange={e => set({ ...df, bases: df.bases.map((b, j) => j === k ? [Number(e.target.value), b[1]] : b) })}>
              {g.stars.map(st => <option key={st.index} value={st.index}>{st.name}{tr(` (hives ${st.initial_hives}/${st.max_hives})`, `（巢穴 ${st.initial_hives}/${st.max_hives}）`)}</option>)}
            </select>
            <NumField value={n} lo={0} hi={1000} aria-label={tr('base count', '基地数')} onCommit={v => set({ ...df, bases: df.bases.map((b, j) => j === k ? [b[0], v ?? 0] : b) })} />
            <button className="x" aria-label={tr('Delete', '删除')} onClick={() => set({ ...df, bases: df.bases.filter((_, j) => j !== k) })}>×</button>
          </li>
        ))}
      </ul>
      <button className="tabbtn" onClick={() => set({ ...df, bases: [...df.bases, [0, 1]] })}>{tr('Add base', '加基地')}</button>
      <h4>{tr('Drops per base per minute (measured)', '每个基地每分钟掉落（实测）')}</h4>
      <ul className="plain">
        {df.drops.map((d, k) => (
          <li key={k} className="row small"><Icon id={d.item} size={18} />{name(d.item)} <span className="muted">{tr(`from Dark Fog level ${DROPS.find(x => x[0] === d.item)?.[1] ?? 0}`, `黑雾 ${DROPS.find(x => x[0] === d.item)?.[1] ?? 0} 级起`)}</span>
            <NumField step={0.1} value={d.rate} lo={0} hi={1e6} aria-label={tr(`${name(d.item)} per base per minute`, `${name(d.item)} 每基地每分钟`)} onCommit={v => set({ ...df, drops: df.drops.map((x, j) => j === k ? { ...x, rate: v ?? 0 } : x) })} />
            <button className="x" aria-label={tr('Delete', '删除')} onClick={() => set({ ...df, drops: df.drops.filter((_, j) => j !== k) })}>×</button>
          </li>
        ))}
      </ul>
      <span className="row small">
        <select value={item} onChange={e => setItem(e.target.value)} aria-label={tr('Dropped item', '掉落物品')}>
          {DROPS.filter(([i]) => !df.drops.some(d => d.item === i)).map(([i, l]) => <option key={i} value={i}>{name(i)}{tr(` (from level ${l})`, `（${l} 级起）`)}</option>)}
        </select>
        <button className="tabbtn" onClick={() => { if (!df.drops.some(d => d.item === item)) set({ ...df, drops: [...df.drops, { item, rate: 1 }] }) }}>{tr('Add drop', '加掉落')}</button>
      </span>
    </details>
  )
}

function PlanView({ gp, name, starName, recipeName }: { gp: GalaxyPlan; name: (id: string) => string; starName: (i: number) => string; recipeName: (id: string) => string }) {
  const [sel, setSel] = useState<number | null>(null)
  if (gp.status !== 'optimal') {
    return <div className="warn"><b>{statusName(gp.status, tr('No plan', '没有方案'))}</b>{gp.warnings.map((w, i) => <p key={i}>{w}</p>)}</div>
  }
  return <>
    <div className="cards">
      <div className="card"><span className="big">{num(gp.z, 0)}<span className="unit">{tr('/min', '/分钟')}</span></span><span className="muted">{tr('Cubes', '白糖')}</span></div>
      <div className="card"><span className="big">{num(gp.buildings, 0)}</span><span className="muted">{tr('Buildings', '建筑')}</span></div>
      <div className="card"><span className="big">{fmt(gp.receiver_gw, 1)}<span className="unit"> GW</span></span><span className="muted">{tr(`Ray reception, ${fmt(gp.z > 0 ? gp.photons_per_min / gp.z : 0, 2)} photons per cube`, `射线接收，每白糖 ${fmt(gp.z > 0 ? gp.photons_per_min / gp.z : 0, 2)} 光子`)}</span></div>
      <div className="card"><span className="big">{fmt(gp.warpers_per_min, 1)}<span className="unit">{tr('/min', '/分钟')}</span></span><span className="muted">{tr('Warpers', '翘曲器')}</span></div>
      <div className="card"><span className="big">{num(gp.power_mw, 0)}<span className="unit"> MW</span></span><span className="muted">{tr('Production power', '生产耗电')}</span></div>
    </div>
    <p className="small muted">
      {tr(`Level ${gp.level}: mining ×${fmt(gp.speed, 1)}, S(L) = ${num(gp.s_level, 0)}. Buildings: production ${num(gp.production_buildings, 0)}, miners ${num(gp.miners, 0)}, water pumps ${num(gp.pumps, 0)}, oil extractors ${num(gp.oil_extractors, 0)}, orbital collectors ${num(gp.collectors, 0)}, ray receivers ${num(gp.receivers, 0)}, logistics stations ${num(gp.stations, 1)}. Linear programme ${num(gp.columns, 0)} columns × ${num(gp.rows, 0)} rows, ${fmt(gp.solve_ms / 1000, 2)} s.`, `${gp.level} 级：采矿 ×${fmt(gp.speed, 1)}，S(L) = ${num(gp.s_level, 0)}。建筑：生产 ${num(gp.production_buildings, 0)}，采矿机 ${num(gp.miners, 0)}，抽水站 ${num(gp.pumps, 0)}，原油萃取站 ${num(gp.oil_extractors, 0)}，轨道采集器 ${num(gp.collectors, 0)}，射线接收站 ${num(gp.receivers, 0)}，物流塔 ${num(gp.stations, 1)}。线性规划 ${num(gp.columns, 0)} 列 × ${num(gp.rows, 0)} 行，${fmt(gp.solve_ms / 1000, 2)} 秒。`)}
    </p>
    {gp.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <h4>{tr('Finite resources per cube (rare ones at their quota = binding)', '每个白糖用的有限资源（珍奇用到配额 = 卡住）')}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>{tr('Resource', '资源')}</th><th className="num">{tr('Mined per cube', '每白糖开采')}</th><th className="num">{tr('Quota per cube', '每白糖配额')}</th><th>{tr('Usage', '用量')}</th></tr></thead>
        <tbody>
          {gp.rares.filter(r => r.quota_per_cube > 0 || r.per_cube > 0).map(r => (
            <tr key={r.item}><td><Icon id={r.item} size={18} />{name(r.item)}</td><td className="num">{fmt(r.per_cube, 3)}</td><td className="num">{fmt(r.quota_per_cube, 3)}</td>
              <td className="small">{r.binding ? <span className="st-warn">{tr('at quota', '到配额')}</span> : <span className="muted">{fmt(100 * r.per_cube / Math.max(r.quota_per_cube, 1e-12), 0)} %</span>}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>{tr('Systems (click to see recipes and mining)', '各星系（点开看配方和开采）')}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>{tr('System', '星系')}</th><th className="num">{tr('ly', '光年')}</th><th className="num">{tr('Cubes/min', '白糖/分钟')}</th><th className="num">{tr('Production buildings', '生产建筑')}</th><th className="num">{tr('Miners', '采矿机')}</th><th className="num">{tr('Collectors', '采集器')}</th><th className="num">{tr('Logistics stations', '物流塔')}</th></tr></thead>
        <tbody>
          {gp.systems.map(s => [
            <tr key={s.star} className="clickable" onClick={() => setSel(sel === s.star ? null : s.star)}>
              <td><span className="caret">{sel === s.star ? '▾' : '▸'}</span>{s.name}{s.planet !== null && s.planet !== undefined ? <span className="muted"> · P{s.planet}</span> : null}</td><td className="num">{fmt(s.distance, 1)}</td><td className="num">{num(s.cubes, 0)}</td>
              <td className="num">{num(s.production_buildings, 0)}</td><td className="num">{num(s.miners, 0)}</td><td className="num">{num(s.collectors, 0)}</td><td className="num">{fmt(s.stations, 1)}</td>
            </tr>,
            sel === s.star && <tr key={`${s.star}-d`}><td colSpan={7} className="small">
              {s.extraction.length > 0 && <div><b>{tr('Mining', '开采')}</b>{tr(': ', '：')}{s.extraction.map(e => `${name(e.item.replace(' (gas)', ''))}${e.item.endsWith('(gas)') ? tr(' (gas giant)', '（气巨）') : ''} ${num(e.rate, 1)}${tr('/min', '/分钟')}${e.cap !== null ? tr(`, limit ${num(e.cap, 0)} ${e.limit}`, `，上限 ${num(e.cap, 0)}（${e.limit === 'rate' ? '速率' : e.limit === 'quota' ? '配额' : e.limit}）`) : ''}`).join(' · ')}</div>}
              {s.drops.length > 0 && <div><b>{tr('Dark Fog drops', '黑雾掉落')}</b>{tr(': ', '：')}{s.drops.map(d => `${name(d.item)} ${num(d.rate, 1)}${tr('/min', '/分钟')}`).join(' · ')}</div>}
              <div><b>{tr('Recipes', '配方')}</b>{tr(': ', '：')}{s.recipes.length === 0 ? tr('none', '无') : s.recipes.slice(0, 40).map(r => `${recipeName(r.recipe)}${r.mode ? tr(` [${r.mode}]`, `［${r.mode}］`) : ''} ×${num(r.machines, 1)}`).join(' · ')}{s.recipes.length > 40 ? tr(` … ${s.recipes.length - 40} more`, ` … 还有 ${s.recipes.length - 40} 个`) : ''}</div>
            </td></tr>,
          ])}
        </tbody>
      </table>
    </div>
    <h4>{tr(`Interstellar transport (${gp.shipments.length} routes, by quantity × ly, largest first)`, `星际运输（${gp.shipments.length} 条，按 物量×光年 从大到小）`)}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>{tr('Item', '物品')}</th><th>{tr('From → To', '从 → 到')}</th><th className="num">{tr('/min', '/分钟')}</th><th className="num">{tr('ly', '光年')}</th></tr></thead>
        <tbody>
          {gp.shipments.slice(0, 30).map((s, i) => (
            <tr key={i}><td><Icon id={s.item} size={18} />{name(s.item)}</td><td>{starName(s.from)} → {starName(s.to)}</td><td className="num">{num(s.rate, 1)}</td><td className="num">{fmt(s.ly, 1)}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
  </>
}

function TotalView({ t, name, recipeName, level, research }: { t: TotalReport; name: (id: string) => string; recipeName: (id: string) => string; level: number; research: boolean }) {
  if (t.status !== 'optimal') {
    return <div className="warn"><b>{statusName(t.status, tr('No result', '没有结果'))}</b>{t.warnings.map((w, i) => <p key={i}>{w}</p>)}</div>
  }
  const binding = t.finite.filter(u => u.binding)
  return <>
    <div className="cards">
      <div className="card"><span className="big">{compact(t.cubes)}</span><span className="muted">{tr('Total cubes', '白糖总数')}{t.levels_done > 0 ? tr(` (${t.levels_done} levels done, no further research)`, `（已完成 ${t.levels_done} 级，不再研究）`) : tr(' (no vein utilisation upgrades)', '（不升矿物利用）')}</span></div>
      {research && (t.infinite
        ? <div className="card"><span className="big">{tr('Infinite', '无限')}</span><span className="muted">{tr('Researching vein utilisation from now on, the ore never runs out', '从现在起一直研究矿物利用，矿永远用不完')}</span></div>
        : <div className="card"><span className="big">{compact(t.best_cubes)}</span><span className="muted">{t.best_levels_done > t.levels_done ? tr(`Best: research up to level ${t.best_levels_done} (costs ${compact(t.research_cubes)} cubes), then produce at full tilt`, `最多：再研究到 ${t.best_levels_done} 级（花 ${compact(t.research_cubes)} 白糖）后全力生产`) : tr('Best: no more research, just produce', '最多：不再研究，直接生产')}</span></div>)}
      {!research && level > 1 && <div className="card"><span className="big">{compact(t.cubes_no_util / Math.pow(0.94, level - 1))}</span><span className="muted">{tr(`With ${level - 1} levels done (×${fmt(1 / Math.pow(0.94, level - 1), 0)}, ore spent on research not counted)`, `完成 ${level - 1} 级时（×${fmt(1 / Math.pow(0.94, level - 1), 0)}，不算研究耗的矿）`)}</span></div>}
      <div className="card"><span className="big">{binding.length}</span><span className="muted">{tr('Binding: ', '卡住：')}{binding.map(u => name(u.item)).join(tr(', ', '、')) || '—'}</span></div>
      {t.hub && <div className="card"><span className="big">{compact(t.warpers)}</span><span className="muted">{tr(`Warpers (hub ${t.hub})`, `翘曲器（中心 ${t.hub}）`)}</span></div>}
    </div>
    {research && <p className="small">
      {t.infinite
        ? <>{tr(`The remaining ore converts to ${compact(t.cubes_no_util)} cubes at the non-upgraded consumption; if vein utilisation is researched continuously from now on, only ${compact(t.research_need)} cubes are needed in total after conversion, which is less, so as long as you keep researching the ore never runs out and the total has no limit (笑笑: infinite cubes from finite ore).`, `剩下的矿按不升级的消耗折算能做 ${compact(t.cubes_no_util)} 个白糖；从现在起把矿物利用一直研究下去，折算后一共只要 ${compact(t.research_need)} 个，比它少，所以只要一直研究，矿就永远用不完，总量没有上限（笑笑：用有限的矿造出无限的白糖）。`)}</>
        : <>{tr(`The remaining ore (equivalent to ${compact(t.cubes_no_util)} cubes) is not enough to keep researching forever (${compact(t.research_need)} needed): research too little and each cube uses a lot of ore; research too much and the ore all goes into research. ${t.best_levels_done > t.levels_done ? `Best: research up to level ${t.best_levels_done}, then use everything for production, ${compact(t.best_cubes)} cubes in all.` : 'Best: no more research, use everything for production.'}`, `剩下的矿（折算 ${compact(t.cubes_no_util)} 个白糖）不够一直研究下去（要 ${compact(t.research_need)} 个）：研究得太少，每个白糖用矿多；研究得太多，矿都花在研究上。${t.best_levels_done > t.levels_done ? `最好再研究到 ${t.best_levels_done} 级，之后全部用来生产，一共 ${compact(t.best_cubes)} 个白糖。` : '最好不再研究，直接全部用来生产。'}`)}</>}
    </p>}
    <p className="small muted">{tr(`${t.stars} stars. Counted as unlimited here: ${t.unlimited.map(name).join(', ')}. ${fmt(t.solve_ms, 0)} ms.`, `${t.stars} 颗星。这里算作无限的：${t.unlimited.map(name).join('、')}。${fmt(t.solve_ms, 0)} ms。`)}</p>
    {t.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <h4>{tr('Finite reserves', '有限储量')}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>{tr('Resource', '资源')}</th><th className="num">{tr('Available', '可采')}</th><th className="num">{tr('Used', '用了')}</th><th className="num">{tr('Usage', '用量')}</th><th className="num">{tr('Per cube', '每个白糖')}</th><th className="num">{tr('Extra cubes per +1000', '每多 1000 多几个白糖')}</th></tr></thead>
        <tbody>
          {t.finite.map(u => (
            <tr key={u.item}><td><Icon id={u.item} size={18} />{name(u.item)}</td><td className="num">{compact(u.available)}</td><td className="num">{compact(u.used)}</td>
              <td className="num">{u.binding ? <span className="st-warn">{tr('used up, binding', '用完，卡住')}</span> : `${fmt(100 * u.used / Math.max(u.available, 1e-12), 0)} %`}</td>
              <td className="num">{fmt(u.per_cube, 3)}</td><td className="num">{u.binding ? fmt(1000 * u.marginal, 2) : '—'}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>{tr('Recipes: machines per 1 cube/min', '配方：每 1 白糖/分钟要几台机器')}</h4>
    <p className="small">{t.recipes.slice(0, 60).map(r => `${recipeName(r.recipe)}${r.mode ? tr(` [${r.mode}]`, `［${r.mode}］`) : ''} ×${fmt(r.machines_per_rate, 2)}`).join(' · ')}{t.recipes.length > 60 ? tr(` … ${t.recipes.length - 60} more`, ` … 还有 ${t.recipes.length - 60} 个`) : ''}</p>
    <h4>{tr('Raw inputs per cube', '每个白糖的原料')}</h4>
    <p className="small">{t.raw_per_cube.map(r => `${name(r.item)} ${fmt(r.rate, 2)}`).join(' · ')} <span className="muted">{tr('(unlimited inputs are counted as total input)', '（无限的原料按总投入计）')}</span></p>
  </>
}
