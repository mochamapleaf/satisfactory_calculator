// DSP galaxy tab (星区): seed import, highest cube rate, total, infinite thresholds, multi-system LP, Dark Fog.
// Model: src/planner/dsp_galaxy.rs; spec docs/research/dsp-seed/00-最终报告.md §4–§5, §9.
// Its settings are its own (stored per browser): changing them never re-solves the main plan.
// All text is Chinese (Xinyu 2026-11-23: 「你是不是应该全中文？」).
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
interface RecipeUse { recipe: string; mode: string; building: string; machines_per_rate: number }
interface TotalReport {
  status: string; cubes: number; cubes_no_util: number; levels_done: number; stars: number; hub: string | null; warpers: number
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
  total_levels: number; total_scope: 'all' | 'ticked'; total_warpers: boolean; total_alts: boolean; stock_quotas: boolean
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
const MULTS: [number, string][] = [[0.1, '0.1× 极少'], [0.5, '0.5×'], [0.8, '0.8×'], [1, '1×'], [1.5, '1.5×'], [2, '2×'], [3, '3×'], [5, '5×'], [8, '8×'], [100, '无限']]
/** BWIKI 黑雾掉落 unlock levels (same table as dsp_galaxy.rs `drop_unlock_level`). */
const DROPS: [string, number][] = [
  ['processor', 9], ['graphene', 9], ['carbon-nanotube', 12], ['df-dark-fog-matrix', 12], ['optical-grating-crystal', 15], ['particle-container', 15],
  ['casimir-crystal', 15], ['df-matter-recombinator', 15], ['strange-matter', 18], ['df-silicon-based-neuron', 18], ['antimatter', 21], ['quantum-chip', 21],
  ['graviton-lens', 21], ['space-warper', 21], ['df-negentropy-singularity', 21], ['unipolar-magnet', 24], ['antimatter-fuel-rod', 24], ['df-core-element', 24],
]
const TYPE_ZH: Record<string, string> = { MainSeqStar: '主序星', GiantStar: '巨星', WhiteDwarf: '白矮星', NeutronStar: '中子星', BlackHole: '黑洞' }
const KIND_ZH: Record<string, string> = { Ocean: '海洋', Desert: '沙漠', Ice: '冰原', Gas: '气态巨星', Vocano: '火山', Volcano: '火山' }
const SOURCE_ZH: Record<string, string> = { vein: '矿脉', oil: '油井', gas: '气巨', 'vein+gas': '矿脉 + 气巨', dyson: '戴森球' }
const RARE = new Set(['fire-ice', 'kimberlite-ore', 'fractal-silicon', 'organic-crystal', 'optical-grating-crystal', 'spiniform-stalagmite-crystal', 'unipolar-magnet'])
const STATUS_ZH: Record<string, string> = { unbounded: '无上限', infeasible: '无解' }

/** Thousands-separated number with at most `d` decimals. */
const num = (x: number, d = 0) => (Number.isFinite(x) ? (Number(x.toFixed(d)) + 0).toLocaleString('en-US', { maximumFractionDigits: d, minimumFractionDigits: 0 }) : '–')
/** Compact in Chinese units: 1.23 亿, 45.6 万. */
const compact = (x: number) => (Math.abs(x) >= 1e8 ? `${fmt(x / 1e8, 2)} 亿` : Math.abs(x) >= 1e4 ? `${fmt(x / 1e4, Math.abs(x) >= 1e6 ? 0 : 1)} 万` : num(x, 1))
const lv = (l: number | null | undefined) => (l === null || l === undefined ? '—' : `${l} 级`)
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
      const out = JSON.stringify({ error: `星区计算线程出错：${e.message || '加载失败'}` })
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

const Stale = ({ what }: { what: string }) => <p className="warn stale" role="status"><b>结果已过期</b>：设置改了，按「{what}」重新计算。</p>

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
  const recipeName = (id: string) => { const r = data.recipes.find(x => x.id === id); return r ? (r.name_zh ?? r.name) : id }
  const gen = useRef(0)

  // the recipe side comes from the sidebar (alternates, proliferator, buildings); targets are set by the model
  const plan = useMemo(() => ({ ...settings, game: 'dsp', integer: false }), [settings])
  // "every recipe": all alternates and both Mk.III modes, so the LP picks what gives the most cubes
  const bestPlan = useMemo(() => (o.total_alts
    ? { ...plan, allow_all_alternates: true, effects: [...new Set([...plan.effects, 'proliferator-3-speed', 'proliferator-3-products'])] }
    : plan), [plan, o.total_alts])
  const darkFog = o.dark_fog

  const importSeed = (s: SeedSettings) => {
    const t = ++gen.current
    setBusy('正在生成星区…')
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
    setBusy('正在求最高白糖速率…')
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
    stock_quotas: o.stock_quotas, remaining: o.remaining / 100, systems, oil: o.oil, building_cap: o.building_cap, photon_gw: o.photon_gw,
    vessel_capacity: o.vessel_capacity, warpers_per_trip: o.warpers_per_trip, dark_fog: darkFog,
  }), [o, plan, systems, darkFog])
  const totPayload = useMemo(() => JSON.stringify({
    seed: o.seed, plan: bestPlan, levels_done: o.total_levels, remaining: o.remaining / 100, stock_override: {},
    systems: scopeSystems, warpers: o.total_warpers, vessel_capacity: o.vessel_capacity, warpers_per_trip: o.warpers_per_trip,
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }), [o.seed, bestPlan, o.total_levels, o.remaining, JSON.stringify(scopeSystems), o.total_warpers, o.vessel_capacity, o.warpers_per_trip])
  const runTotal = () => {
    const t = gen.current, sent = totPayload
    setBusy('正在求总产量…')
    void call<TotalReport>('total', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setTot(r); setTotKey(sent) })
  }
  const totStale = !!tot && !tot.error && totKey !== totPayload
  const [thKey, setThKey] = useState('')
  const [gpKey, setGpKey] = useState('')
  const runThresholds = () => {
    const t = gen.current, sent = thPayload
    setBusy('正在算无限阈值…')
    void call<ThresholdReport>('thresholds', JSON.parse(sent)).then(r => { if (t !== gen.current) return; setBusy(''); setTh(r); setThKey(sent) })
  }
  const runPlan = () => {
    const t = gen.current, sent = gpPayload
    setBusy('正在求多星系规划…')
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
        星区规划：输入种子生成整个星区（生成器 <a href="https://github.com/DoubleUTH/DSP-Seed-Finder" target="_blank" rel="noreferrer">DoubleUTH/DSP-Seed-Finder</a>，有修改，{' '}
        <a href={`${import.meta.env.BASE_URL}licenses/DSP-Seed-Finder-LICENSE.txt`} target="_blank" rel="noreferrer">Apache-2.0</a>，{' '}
        <a href={`${import.meta.env.BASE_URL}licenses/DSP-Seed-Finder-NOTICE.txt`} target="_blank" rel="noreferrer">改动说明</a>），
        然后算这个星区最多能做多少白糖（宇宙矩阵）。
      </p>

      {/* ---- seed import */}
      <h3>种子</h3>
      <div className="mbsettings">
        <label>种子<NumField value={seedDraft.seed} lo={0} hi={99999999} integer onCommit={v => setSeedDraft(x => ({ ...x, seed: v ?? x.seed }))} style={{ width: 110 }} /></label>
        <label>恒星数<NumField value={seedDraft.stars} lo={32} hi={64} integer onCommit={v => setSeedDraft(x => ({ ...x, stars: v ?? x.stars }))} /></label>
        <label>资源倍率
          <select value={seedDraft.resource_multiplier} onChange={e => setSeedDraft(x => ({ ...x, resource_multiplier: Number(e.target.value) }))}>
            {MULTS.map(([v, t]) => <option key={v} value={v}>{t}</option>)}
          </select>
        </label>
        <button onClick={() => importSeed(seedDraft)} disabled={busyNow}>导入种子</button>
        <details>
          <summary>黑雾生成设置（巢穴数）</summary>
          <div className="mbsettings">
            <label>初始殖民<NumField step={0.25} value={seedDraft.hive_initial_colonize} lo={0} hi={2} onCommit={v => setSeedDraft(x => ({ ...x, hive_initial_colonize: v ?? 1 }))} /></label>
            <label>最大密度<NumField step={0.25} value={seedDraft.hive_max_density} lo={0} hi={3} onCommit={v => setSeedDraft(x => ({ ...x, hive_max_density: v ?? 1 }))} /></label>
          </div>
        </details>
      </div>
      <p className="hint">
        网页和桌面版生成的星区完全一样{g && !g.error ? <>（指纹 <code>{g.fingerprint}</code>）</> : null}。和游戏只做过抽查：黑雾更新前玩家贴出的数据对得上
        （四个种子的恒星光度和距离；一个种子的母星铁矿和原油，煤 830 万对这里 859 万）。重要的储量请先在自己的存档里看一眼。
        {seedDirty && <> <b>上面的输入和已导入的星区不同</b>（种子 {o.seed.seed}，{o.seed.stars} 颗星，{o.seed.resource_multiplier}×）：请按「导入种子」。</>}
      </p>
      {busy && <p className="solving" role="status">{busy}</p>}
      {g?.error && <p className="warn">{g.error}</p>}

      {/* ---- highest cube rate (the headline) */}
      {g && !g.error && <>
        <h3>最高白糖速率</h3>
        <p className="hint">
          矿物利用每升一级，采矿消耗 ×0.94、采矿速度 +10%。研究消耗按等级平方涨，矿的等效储量按指数涨，所以跑到某一级之后矿就<b>永远用不完</b>
          （笑笑 BV1QF24YDE7H 03:20 起）。从那以后白糖每分钟能做多少，只看<b>速率</b>：每个矿点每分钟能采多少（随等级变快）、油井和气巨的速率、
          以及戴森球能给多少光子：每颗恒星的戴森壳满球功率（按光度和最大半径算），再加上戴森云（太阳帆数量不限，但做帆的石墨烯等原料也要按速率算）。海洋（水、硫酸）抽多少都有。机器、太阳帆、物流塔都不限。
        </p>
        <div className="mbsettings">
          <label>矿物利用等级（正在研究的一级）<NumField value={o.rate_level} lo={1} hi={10000} integer onCommit={v => set({ rate_level: v ?? 81 })} /></label>
          <label>每个矿点几台大型采矿机<NumField step={1} value={o.miners_per_node} lo={0.1} hi={10} onCommit={v => set({ miners_per_node: v ?? 1 })} /></label>
          <label>采矿机速度 %<NumField step={10} value={o.miner_speed} lo={100} hi={300} onCommit={v => set({ miner_speed: v ?? 100 })} /></label>
          <label className="check"><input type="checkbox" checked={o.dyson} onChange={e => set({ dyson: e.target.checked })} /> 光子受戴森球满球功率限制</label>
          {o.dyson && <label>戴森壳建成 %<NumField step={10} value={o.dyson_share} lo={0} hi={100} onCommit={v => set({ dyson_share: v ?? 100 })} /></label>}
          {o.dyson && <label className="check"><input type="checkbox" checked={o.swarm} onChange={e => set({ swarm: e.target.checked })} /> 加戴森云（太阳帆不限，帆的原料算进速率）</label>}
          <button onClick={runRate} disabled={busyNow || (o.total_scope === 'ticked' && systems.length === 0)}>计算最高速率</button>
        </div>
        <CommonSettings o={o} set={set} g={g} systems={systems} prolif={prolif} />
        {rr?.error && <p className="warn">{rr.error}</p>}
        {rrStale && <Stale what="计算最高速率" />}
        {rr && !rr.error && <div className={rrStale ? 'staleresult' : undefined}><RateView r={rr} name={name} recipeName={recipeName} miners={o.miners_per_node} /></div>}
      </>}

      {g && !g.error && <GalaxyTables g={g} name={name} systems={systems} toggle={toggleSystem} open={open} setOpen={setOpen}
        resetSystems={() => set({ systems: null })} />}

      {/* ---- galaxy total */}
      {g && !g.error && <>
        <h3>总量：储量一共能做多少白糖</h3>
        <p className="hint">
          另一个问题：不升矿物利用（或只升到某一级）时，整个星区的有限矿脉一共能变成多少白糖。机器和时间不限，所以海洋、气巨、油井（不采到底线以下）和光子都算无限，
          只有矿脉会用完。每多升一级矿物利用，总量 ×1/0.94（+6.4%），等级无限时总量也无限，这就是上面「最高白糖速率」的前提。
          爬科技时消耗的矿还没算进去。
        </p>
        <div className="mbsettings">
          <label>已完成的矿物利用级数<NumField value={o.total_levels} lo={0} hi={500} integer onCommit={v => set({ total_levels: v ?? 0 })} /></label>
          <label>剩余储量 %<NumField value={o.remaining} lo={0} hi={100} onCommit={v => set({ remaining: v ?? 100 })} /></label>
          <button onClick={runTotal} disabled={busyNow || (o.total_scope === 'ticked' && systems.length === 0)}>计算总量</button>
        </div>
        {tot?.error && <p className="warn">{tot.error}</p>}
        {totStale && <Stale what="计算总量" />}
        {tot && !tot.error && <div className={totStale ? 'staleresult' : undefined}><TotalView t={tot} name={name} level={o.level} recipeName={recipeName} /></div>}
      </>}

      {/* ---- thresholds */}
      {g && !g.error && <>
        <h3>无限阈值：每种矿从几级起用不完</h3>
        <p className="hint">
          假设从 L 级起所有白糖都拿去研究矿物利用，一种资源按「每个白糖用 a 个」一直用下去也用不完的条件是 a·ρ·S(L) ≤ B，
          S(L) = Σ<sub>n≥L</sub> 4000(n−5)·0.94<sup>n−1</sup>（S(6) = 815 449）。研究站喷 Mk.III 增产剂时 ρ = 0.8（哈希 ×1.25，存疑）。
          B 是剩余储量，原油只算每个油井 2500 底线以上的部分。笑笑会在放开等级之后再多等约五级当缓冲（L80 → L85）。
        </p>
        <div className="mbsettings">
          <label>白糖/分钟（换算成小时）<NumField value={o.rate} lo={1} hi={1e7} onCommit={v => set({ rate: v ?? 750 })} /></label>
          <label>剩余储量 %<NumField value={o.remaining} lo={0} hi={100} onCommit={v => set({ remaining: v ?? 100 })} /></label>
          <label>配额等级 L<NumField value={o.level} lo={1} hi={2000} integer onCommit={v => set({ level: v ?? 81 })} /></label>
          <button onClick={runThresholds} disabled={busyNow}>计算阈值</button>
        </div>
        {th?.error && <p className="warn">{th.error}</p>}
        {thStale && <Stale what="计算阈值" />}
        {th && !th.error && <div className={thStale ? 'staleresult' : undefined}><ThresholdTable th={th} name={name} gas={new Set(g.gas_totals.map(x => x.item))} /></div>}
      </>}

      {/* ---- Dark Fog */}
      {g && !g.error && <DarkFogBox g={g} df={darkFog} set={df => set({ dark_fog: df })} name={name} />}

      {/* ---- multi-system LP */}
      {g && !g.error && <>
        <h3>多星系规划：哪个星系做什么、怎么运</h3>
        <p className="hint">
          对上面勾选的 {systems.length} 个星系做一个线性规划（一个星系的所有行星合在一起算；笑笑在满强度黑雾下每个星系只开一颗行星）：每个星系的产量、
          星际运输（物流塔数按距离，按笑笑「7 光年内 4 条四堆叠蓝带、12 光年 3 条」拟合；每趟耗翘曲器）、
          有限储量按配额 m·ρ·S(L) ≤ B·z 分到各星系、原油和气巨按 L 级速率（×{fmt(1 + 0.1 * (o.level - 1), 1)}）、有海洋的星系才有水和硫酸。
          发电厂没有建模；光子不设上限时不限。
        </p>
        <div className="mbsettings">
          <label>目标
            <select value={o.goal} onChange={e => set({ goal: e.target.value as Opts['goal'] })}>
              <option value="target">给定白糖/分钟，建筑最少</option>
              <option value="max">白糖/分钟最大（需要建筑或功率上限）</option>
            </select>
          </label>
          {o.goal === 'target' && <label>目标白糖/分钟<NumField value={o.target} lo={0} hi={1e7} onCommit={v => set({ target: v ?? 1000 })} /></label>}
          {o.goal === 'target' && <label>最少化
            <select value={o.objective} onChange={e => set({ objective: e.target.value as Opts['objective'] })}>
              <option value="buildings">建筑数</option>
              <option value="photons">光子，其次建筑数</option>
            </select>
          </label>}
          <label>等级 L<NumField value={o.level} lo={1} hi={2000} integer onCommit={v => set({ level: v ?? 81 })} /></label>
          <label>建筑上限<NumField placeholder="不限" nullable value={o.building_cap} lo={0} hi={1e8} onCommit={v => set({ building_cap: v })} /></label>
          <label>戴森球 GW<NumField placeholder="不限" nullable value={o.photon_gw} lo={0} hi={1e6} onCommit={v => set({ photon_gw: v })} /></label>
          <button onClick={runPlan} disabled={busyNow || systems.length === 0}>规划星区</button>
          <details>
            <summary>更多：配额、不拿去研究的白糖</summary>
            <div className="mbsettings">
              <label className="check"><input type="checkbox" checked={o.stock_quotas} onChange={e => set({ stock_quotas: e.target.checked })} /> 储量配额（关掉 = 只看速率）</label>
              <label>φ 不研究的白糖 %<NumField value={o.phi} lo={0} hi={95} onCommit={v => set({ phi: v ?? 0 })} /></label>
            </div>
          </details>
        </div>
        {gp?.error && <p className="warn">{gp.error}</p>}
        {gpStale && <Stale what="规划星区" />}
        {gp && !gp.error && <div className={gpStale ? 'staleresult' : undefined}><PlanView gp={gp} name={name} starName={starName} recipeName={recipeName} /></div>}
      </>}
    </div>
  )
}

/** Settings shared by the rate, total and plan: which stars, recipes, oil, research ρ, logistics. */
function CommonSettings({ o, set, g, systems, prolif }: { o: Opts; set: (p: Partial<Opts>) => void; g: Galaxy; systems: number[]; prolif: string[] }) {
  return (
    <details>
      <summary>通用设置：哪些星、配方、原油、研究、运输</summary>
      <div className="mbsettings">
        <label>算哪些星
          <select value={o.total_scope} onChange={e => set({ total_scope: e.target.value as Opts['total_scope'] })}>
            <option value="all">全部 {g.stars.length} 颗</option>
            <option value="ticked">下表勾选的 {systems.length} 个星系</option>
          </select>
        </label>
        <label className="check"><input type="checkbox" checked={o.total_alts} onChange={e => set({ total_alts: e.target.checked })} /> 所有配方 + Mk.III 增产剂（只在能多出白糖时才用）</label>
        {!o.total_alts && <span className="small muted">用侧栏的配方：{prolif.length ? `增产剂 ${prolif.join(' + ')}` : '不喷增产剂'}</span>}
        <label>原油速率
          <select value={o.oil} onChange={e => set({ oil: e.target.value as Opts['oil'] })}>
            <option value="initial">油井初始速率（上限）</option>
            <option value="floor">采到底线：每个油井 0.1/秒</option>
          </select>
        </label>
        <label>研究 ρ
          <select value={o.rho} onChange={e => set({ rho: Number(e.target.value) })}>
            <option value={1}>1（研究站不喷）</option>
            <option value={0.8}>0.8（喷 Mk.III）</option>
          </select>
        </label>
        <label className="check"><input type="checkbox" checked={o.total_warpers} onChange={e => set({ total_warpers: e.target.checked })} /> 星际运输耗翘曲器（关掉：慢速飞，不耗东西）</label>
        <label>每船装载<NumField value={o.vessel_capacity} lo={1} hi={10000} onCommit={v => set({ vessel_capacity: v ?? 400 })} /></label>
        <label>每趟翘曲器<NumField step={1} value={o.warpers_per_trip} lo={0} hi={4} onCommit={v => set({ warpers_per_trip: v ?? 2 })} /></label>
      </div>
    </details>
  )
}

function RateView({ r, name, recipeName, miners }: { r: RateReport; name: (id: string) => string; recipeName: (id: string) => string; miners: number }) {
  if (r.status !== 'optimal') {
    return <div className="warn"><b>{STATUS_ZH[r.status] ?? r.status}</b>{r.warnings.map((w, i) => <p key={i}>{w}</p>)}</div>
  }
  const limits = r.uses.filter(u => u.binding)
  const dysonBound = limits.some(u => u.item === 'critical-photon')
  const label = (u: RateUse) => (u.item === 'critical-photon' ? '戴森球（光子）' : `${name(u.item)}（${SOURCE_ZH[u.source] ?? u.source}）`)
  const ok = r.infinite_level !== null && r.level >= r.infinite_level
  return <>
    <div className="cards">
      <div className="card"><span className="big">{num(r.cubes_per_min, 0)}<span className="unit">/分钟</span></span><span className="muted">最高白糖（{r.level} 级，采矿 ×{fmt(r.speed, 1)}）</span></div>
      <div className="card"><span className="big">{gw(r.dyson_used_gw + r.swarm_gw)}</span><span className="muted">用掉的戴森球功率{r.dyson_gw > 0 ? `（戴森壳 ${gw(r.dyson_used_gw)} / 满 ${gw(r.dyson_gw)}${r.swarm_gw > 0 ? `，戴森云 ${gw(r.swarm_gw)}` : ''}）` : ''}</span></div>
      {r.ceiling_shells > 0 && <div className="card"><span className="big">{num(r.ceiling_shells, 0)}<span className="unit">/分钟</span></span><span className="muted">极限：矿物利用无限高时，戴森壳能撑的白糖</span></div>}
      <div className="card"><span className="big">{lv(r.infinite_level)}</span><span className="muted">从这级起所用矿脉都用不完</span></div>
      {r.dyson_gw > 0 && <div className="card"><span className="big">{lv(r.dyson_level)}</span><span className="muted">从这级起戴森壳用满</span></div>}
    </div>
    <p>
      <b>卡住它的是：</b>{limits.length ? limits.map(label).join('、') : '—'}。
      {dysonBound
        ? <> 戴森壳已经用满：再升矿物利用，白糖只能靠戴森云（多发太阳帆）和更快的开采慢慢涨。</>
        : <> 这些都是速率上限：矿物利用每升一级，采矿、油井和气巨都快 10%（按基础速率），白糖跟着涨；戴森壳{r.dyson_level ? `约到 ${r.dyson_level} 级才用满` : '还远没用满'}。</>}
      {r.swarm_gw > 0 && r.swarm_star && <> 戴森云放在光度最高的 {r.swarm_star}：每分钟发 {num(r.sails_per_min, 0)} 张帆。</>}
    </p>
    {r.ceiling_shells > 0 && <p className="small">
      矿物利用越高，开采越快，白糖越多；但戴森壳的功率是固定的，所以总有一个极限：等级无限高、开采不再卡时，这个星区的戴森壳最多撑 <b>{num(r.ceiling_shells, 0)}</b> 白糖/分钟
      （工厂用电 {fmt(r.power_mw_per_rate, 1)} MW / 每白糖每分钟，也从戴森球来）。{r.swarm_gw > 0 ? '再加戴森云还能更多。' : ''}
    </p>}
    {!ok && r.infinite_level !== null && <p className="warn">在 {r.level} 级时这个配方组合还没到无限：要从 {r.infinite_level} 级起才永远用不完（之前按这个速率会把储量用掉）。</p>}
    {r.infinite_level === null && <p className="warn">有一种用到的矿脉无论升到几级都会用完，见下表「从几级起无限」。</p>}
    {r.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <h4>随等级的变化</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th className="num">矿物利用等级</th><th className="num">采矿速度</th><th className="num">最高白糖/分钟</th><th>限制</th></tr></thead>
        <tbody>
          {r.curve.map(p => (
            <tr key={p.level} className={p.level === r.level ? 'sel' : undefined}>
              <td className="num">{p.level}{p.level === r.level ? ' ←' : ''}</td><td className="num">×{fmt(1 + 0.1 * (p.level - 1), 1)}</td>
              <td className="num">{num(p.cubes_per_min, 0)}</td><td className="small">{p.dyson_bound ? '开采速率 + 戴森壳已满' : '开采速率'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>各资源的速率上限（{r.stars} 颗星）</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>资源</th><th>来源</th><th className="num">上限/分钟</th><th className="num">用了</th><th className="num">每个白糖</th><th className="num">每多 1000/分钟 多几个白糖</th><th className="num">从几级起无限</th></tr></thead>
        <tbody>
          {r.uses.filter(u => u.cap > 0 || u.used > 0).map(u => (
            <tr key={u.item}>
              <td><Icon id={u.item} size={18} />{name(u.item)}{RARE.has(u.item) ? <span className="tag">珍奇</span> : null}</td>
              <td className="small">{SOURCE_ZH[u.source] ?? u.source}</td>
              <td className="num">{compact(u.cap)}</td>
              <td className="num">{u.binding ? <span className="st-warn">用满，卡住</span> : `${fmt(100 * u.used / Math.max(u.cap, 1e-12), 0)} %`}</td>
              <td className="num">{fmt(u.per_cube, 3)}</td>
              <td className="num">{u.binding ? fmt(1000 * u.marginal, 1) : '—'}</td>
              <td className="num">{u.source === 'vein' || u.source === 'oil' || u.source === 'vein+gas' ? (u.used > 0 ? lv(u.infinite_from) : '—') : '一直'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
    <p className="small muted">
      矿脉上限 = 矿点数 × 60/分钟（大型采矿机 100%）× {miners} 台/矿点 × 采矿速度。游戏里采矿机范围可以重叠，每台都按重叠的矿点全速采（dsp-wiki），所以这一项其实取决于你摆多少台；
      默认每个矿点算 1 台。气巨按每颗 40 个轨道采集器、扣掉 30 MW 自耗。原油按油井初始速率（矿物利用高了以后油井几乎不再衰减）；想保守就在通用设置里改成底线 0.1/秒。
      生产建筑的用电（{gw(r.power_gw)}）也算在戴森球上；采矿机、抽水站和物流塔的用电没算。无限的来源：{r.unlimited.map(name).join('、') || '—'}。{r.hub ? `运输以 ${r.hub} 为中心，翘曲器 ${fmt(r.warpers_per_min, 1)}/分钟。` : ''}
      {' '}{fmt(r.solve_ms, 0)} ms。
    </p>
    {r.dyson.length > 0 && <>
      <h4>戴森壳满球功率（前 12 颗）</h4>
      <div className="tablewrap">
        <table className="mbtable">
          <thead><tr><th>恒星</th><th className="num">光度</th><th className="num">满球功率</th></tr></thead>
          <tbody>
            {r.dyson.slice(0, 12).map(d => <tr key={d.index}><td>{d.name}</td><td className="num">{fmt(d.luminosity, 3)}</td><td className="num">{gw(d.max_gw)}</td></tr>)}
          </tbody>
        </table>
      </div>
      <p className="small muted">
        满球 = 10 层壳，从最大轨道半径起每层往里 1000 米，每层壳面铺满、60 个节点（每层：壳面点 52·r²/10⁴ × 15 kW，框架 1.22·r + 节点 1800 个结构点 × 96 kW，再 × 光度；
        按 B 站 wiki「戴森球发电量」的游戏内拟合，误差约 ±12%）。太阳帆 36 kW × 光度，寿命按满科技 9000 秒。射线接收站每 1.2 GJ 出 1 个光子（120 MW → 6 个/分钟），
        引力透镜只是让一台接收站收得更多，不改变每个光子要的能量；接收效率按满科技 100% 计（游戏里效率随「射线传输效率」科技提高，没满时要打折）。
        没算的：每个星系能摆多少射线接收站（行星表面有限，大巨星的几十 TW 一个星系的行星未必收得下）、行星轨道挡住的壳层。
      </p>
    </>}
    <h4>配方：每 1 白糖/分钟要几台机器</h4>
    <p className="small">{r.recipes.slice(0, 60).map(x => `${recipeName(x.recipe)}${x.mode ? `［${x.mode}］` : ''} ×${fmt(x.machines_per_rate, 2)}`).join(' · ')}{r.recipes.length > 60 ? ` … 还有 ${r.recipes.length - 60} 个` : ''}</p>
    <h4>每个白糖的原料</h4>
    <p className="small">{r.raw_per_cube.map(x => `${name(x.item)} ${fmt(x.rate, 2)}`).join(' · ')}</p>
  </>
}

function GalaxyTables({ g, name, systems, toggle, open, setOpen, resetSystems }: {
  g: Galaxy; name: (id: string) => string; systems: number[]; toggle: (i: number) => void; open: number | null; setOpen: (i: number | null) => void; resetSystems: () => void
}) {
  const on = new Set(systems)
  const mag = g.totals.find(t => t.item === 'unipolar-magnet')?.stock ?? 0
  return <>
    <h3>星区资源</h3>
    <div className="cards">
      <div className="card"><span className="big">{g.stars.length}</span><span className="muted">恒星</span></div>
      <div className="card"><span className="big">{g.stars.reduce((a, s) => a + s.planets.length, 0)}</span><span className="muted">行星</span></div>
      <div className="card"><span className="big">{g.gas_giants}</span><span className="muted">气态巨星</span></div>
      <div className="card"><span className="big">{fmt(g.oil_rate, 1)}<span className="unit">/秒</span></span><span className="muted">原油，{g.seeps} 个油井</span></div>
      <div className="card"><span className="big">{compact(mag)}</span><span className="muted">单极磁石</span></div>
      <div className="card"><span className="big">{fmt(g.generate_ms / 1000, 2)}<span className="unit"> 秒</span></span><span className="muted">生成用时</span></div>
    </div>
    <h4>全星区储量{g.rare_tier ? '（稀有档：原油 ×0.5、气体 ×0.8）' : ''}</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>资源</th><th className="num">储量</th><th className="num">行星数</th></tr></thead>
        <tbody>
          {g.totals.map(t => (
            <tr key={t.item}><td><Icon id={t.item} size={18} />{name(t.item)}{RARE.has(t.item) ? <span className="tag">珍奇</span> : null}</td>
              <td className="num">{compact(t.stock)}</td><td className="num">{t.planets}</td></tr>
          ))}
          {g.gas_totals.map(t => (
            <tr key={`gas-${t.item}`}><td><Icon id={t.item} size={18} />{name(t.item)} <span className="muted">气巨</span></td>
              <td className="num">基础 {fmt(t.base, 2)}/秒</td><td className="num">—</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>星系 · 勾选 = 参与规划（{systems.length}）<button className="tabbtn" onClick={resetSystems}>自动挑选</button></h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>规划</th><th>恒星</th><th className="num">光年</th><th className="num">光度</th><th className="num">巢穴</th><th>资源</th></tr></thead>
        <tbody>
          {g.stars.map(s => {
            const items = new Set<string>()
            s.planets.forEach(p => { p.veins.forEach(v => items.add(v.item)); p.gases.forEach(x => items.add(x.item)); if (p.ocean) items.add(p.ocean) })
            const rare = [...items].filter(i => RARE.has(i) || i === 'sulfuric-acid')
            return [
              <tr key={s.index} className="clickable" onClick={() => setOpen(open === s.index ? null : s.index)}>
                <td onClick={e => e.stopPropagation()}><input type="checkbox" aria-label={`规划 ${s.name}`} checked={on.has(s.index)} onChange={() => toggle(s.index)} /></td>
                <td><span className="caret">{open === s.index ? '▾' : '▸'}</span>{s.name} <span className="muted">{TYPE_ZH[s.star_type] ?? s.star_type} {s.spectr}</span></td>
                <td className="num">{fmt(s.distance, 1)}</td><td className="num">{fmt(s.luminosity, 2)}</td>
                <td className="num">{s.initial_hives}/{s.max_hives}</td>
                <td className="small">{rare.map(i => <Icon key={i} id={i} size={16} />)}{rare.length === 0 ? <span className="muted">只有普通资源</span> : null}</td>
              </tr>,
              open === s.index && <tr key={`${s.index}-p`}><td colSpan={6} className="small">
                {s.planets.map(p => (
                  <div key={p.index}><b>P{p.index}</b> {KIND_ZH[p.kind] ?? p.kind} · {p.theme}{p.ocean ? ` · ${name(p.ocean)}海` : ''}
                    {p.gases.length > 0 && <> · 气体 {p.gases.map(x => `${name(x.item)} ${fmt(x.base, 3)}`).join('、')}</>}
                    {p.veins.length > 0 && <> · {p.veins.map(v => v.item === 'crude-oil' ? `${name(v.item)} ${fmt(v.amount * 4e-5, 2)}/秒（${v.nodes} 井）` : `${name(v.item)} ${compact(v.amount)}（${v.nodes} 点）`).join('、')}</>}
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
      所有配额同时满足（线性规划，建筑最少，1 白糖/分钟）：从 <b>{lv(th.feasible_from[0])}</b>（ρ = 1）/ <b>{lv(th.feasible_from[1])}</b>（ρ = 0.8）起可行。
      {th.feasible_from[0] !== null && th.raw_at_feasible.length > 0 && <> 在 {lv(th.feasible_from[0])} 时每个白糖用原油 {fmt(th.raw_at_feasible.find(r => r.item === 'crude-oil')?.rate ?? 0, 2)}、
        单极磁石 {fmt(th.raw_at_feasible.find(r => r.item === 'unipolar-magnet')?.rate ?? 0, 2)}（用到配额为止）。</>}
    </p>
    {th.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <div className="tablewrap">
      <table className="mbtable threshold">
        <thead>
          <tr><th rowSpan={2}>资源</th><th className="num" rowSpan={2}>储量 B</th><th className="num" rowSpan={2}>每个白糖 a</th>
            <th colSpan={3}>ρ = 1</th><th colSpan={3}>ρ = 0.8</th><th className="num" rowSpan={2}>{th.level} 级配额/白糖</th></tr>
          <tr><th className="num">放开</th><th className="num">白糖</th><th className="num">小时</th><th className="num">放开</th><th className="num">白糖</th><th className="num">小时</th></tr>
        </thead>
        <tbody>
          {rows.map(r => (
            <tr key={r.item}>
              <td><Icon id={r.item} size={18} />{name(r.item)}{gas.has(r.item) ? <span className="tag" title="气巨也产，速率永远不会用完；这一行只算矿脉储量">+ 气巨</span> : null}</td>
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
    <p className="hint">放开 = S(L) 允许按每个白糖 a 不受限使用的第一级；白糖 = 研究到这一级要花的 2000(L−6)(L−5)·ρ 个。在那之前线性规划仍会用它，但不超过配额 B/(ρ·S(L))；最好的做法是「先留着，晚点放开」（报告 §4.3）。</p>
  </>
}

function DarkFogBox({ g, df, set, name }: { g: Galaxy; df: DarkFog; set: (d: DarkFog) => void; name: (id: string) => string }) {
  const [item, setItem] = useState(DROPS[15][0])
  return (
    <details className="dfbox" open={df.enabled}>
      <summary>黑雾掉落当作来源 {df.enabled ? `· 开，${df.bases.reduce((a, b) => a + b[1], 0)} 个基地` : '· 关'}</summary>
      <p className="hint">
        掉落速率没有公开数据（报告 §5.4）：填你在游戏里实测的每个地面基地每分钟掉落，以及测的时候正在研究的矿物利用等级（L = 正在研究的一级，已完成 L − 1 级）。
        普通物品按 (1 + 0.04 (L − 1)) 放大，黑雾专属物品不放大。物品要到对应的黑雾等级才掉（BWIKI 黑雾掉落）。用在无限阈值表和多星系规划里。
      </p>
      <div className="mbsettings">
        <label className="row"><input type="checkbox" checked={df.enabled} onChange={e => set({ ...df, enabled: e.target.checked })} /> 使用掉落</label>
        <label>黑雾等级<NumField value={df.level} lo={0} hi={30} integer onCommit={v => set({ ...df, level: v ?? 24 })} /></label>
        <label>测量时的等级 L<NumField value={df.measured_at_level} lo={1} hi={2000} integer onCommit={v => set({ ...df, measured_at_level: v ?? 1 })} /></label>
      </div>
      <h4>刷的基地</h4>
      <ul className="plain">
        {df.bases.map(([s, n], k) => (
          <li key={k} className="row small">
            <select value={s} onChange={e => set({ ...df, bases: df.bases.map((b, j) => j === k ? [Number(e.target.value), b[1]] : b) })}>
              {g.stars.map(st => <option key={st.index} value={st.index}>{st.name}（巢穴 {st.initial_hives}/{st.max_hives}）</option>)}
            </select>
            <NumField value={n} lo={0} hi={1000} aria-label="基地数" onCommit={v => set({ ...df, bases: df.bases.map((b, j) => j === k ? [b[0], v ?? 0] : b) })} />
            <button className="x" aria-label="删除" onClick={() => set({ ...df, bases: df.bases.filter((_, j) => j !== k) })}>×</button>
          </li>
        ))}
      </ul>
      <button className="tabbtn" onClick={() => set({ ...df, bases: [...df.bases, [0, 1]] })}>加基地</button>
      <h4>每个基地每分钟掉落（实测）</h4>
      <ul className="plain">
        {df.drops.map((d, k) => (
          <li key={k} className="row small"><Icon id={d.item} size={18} />{name(d.item)} <span className="muted">黑雾 {DROPS.find(x => x[0] === d.item)?.[1] ?? 0} 级起</span>
            <NumField step={0.1} value={d.rate} lo={0} hi={1e6} aria-label={`${name(d.item)} 每基地每分钟`} onCommit={v => set({ ...df, drops: df.drops.map((x, j) => j === k ? { ...x, rate: v ?? 0 } : x) })} />
            <button className="x" aria-label="删除" onClick={() => set({ ...df, drops: df.drops.filter((_, j) => j !== k) })}>×</button>
          </li>
        ))}
      </ul>
      <span className="row small">
        <select value={item} onChange={e => setItem(e.target.value)} aria-label="掉落物品">
          {DROPS.filter(([i]) => !df.drops.some(d => d.item === i)).map(([i, l]) => <option key={i} value={i}>{name(i)}（{l} 级起）</option>)}
        </select>
        <button className="tabbtn" onClick={() => { if (!df.drops.some(d => d.item === item)) set({ ...df, drops: [...df.drops, { item, rate: 1 }] }) }}>加掉落</button>
      </span>
    </details>
  )
}

function PlanView({ gp, name, starName, recipeName }: { gp: GalaxyPlan; name: (id: string) => string; starName: (i: number) => string; recipeName: (id: string) => string }) {
  const [sel, setSel] = useState<number | null>(null)
  if (gp.status !== 'optimal') {
    return <div className="warn"><b>{STATUS_ZH[gp.status] ?? '没有方案'}</b>{gp.warnings.map((w, i) => <p key={i}>{w}</p>)}</div>
  }
  return <>
    <div className="cards">
      <div className="card"><span className="big">{num(gp.z, 0)}<span className="unit">/分钟</span></span><span className="muted">白糖</span></div>
      <div className="card"><span className="big">{num(gp.buildings, 0)}</span><span className="muted">建筑</span></div>
      <div className="card"><span className="big">{fmt(gp.receiver_gw, 1)}<span className="unit"> GW</span></span><span className="muted">射线接收，每白糖 {fmt(gp.z > 0 ? gp.photons_per_min / gp.z : 0, 2)} 光子</span></div>
      <div className="card"><span className="big">{fmt(gp.warpers_per_min, 1)}<span className="unit">/分钟</span></span><span className="muted">翘曲器</span></div>
      <div className="card"><span className="big">{num(gp.power_mw, 0)}<span className="unit"> MW</span></span><span className="muted">生产耗电</span></div>
    </div>
    <p className="small muted">
      {gp.level} 级：采矿 ×{fmt(gp.speed, 1)}，S(L) = {num(gp.s_level, 0)}。建筑：生产 {num(gp.production_buildings, 0)}，采矿机 {num(gp.miners, 0)}，
      抽水站 {num(gp.pumps, 0)}，原油萃取站 {num(gp.oil_extractors, 0)}，轨道采集器 {num(gp.collectors, 0)}，射线接收站 {num(gp.receivers, 0)}，物流塔 {num(gp.stations, 1)}。
      线性规划 {num(gp.columns, 0)} 列 × {num(gp.rows, 0)} 行，{fmt(gp.solve_ms / 1000, 2)} 秒。
    </p>
    {gp.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <h4>每个白糖用的有限资源（珍奇用到配额 = 卡住）</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>资源</th><th className="num">每白糖开采</th><th className="num">每白糖配额</th><th>用量</th></tr></thead>
        <tbody>
          {gp.rares.filter(r => r.quota_per_cube > 0 || r.per_cube > 0).map(r => (
            <tr key={r.item}><td><Icon id={r.item} size={18} />{name(r.item)}</td><td className="num">{fmt(r.per_cube, 3)}</td><td className="num">{fmt(r.quota_per_cube, 3)}</td>
              <td className="small">{r.binding ? <span className="st-warn">到配额</span> : <span className="muted">{fmt(100 * r.per_cube / Math.max(r.quota_per_cube, 1e-12), 0)} %</span>}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>各星系（点开看配方和开采）</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>星系</th><th className="num">光年</th><th className="num">白糖/分钟</th><th className="num">生产建筑</th><th className="num">采矿机</th><th className="num">采集器</th><th className="num">物流塔</th></tr></thead>
        <tbody>
          {gp.systems.map(s => [
            <tr key={s.star} className="clickable" onClick={() => setSel(sel === s.star ? null : s.star)}>
              <td><span className="caret">{sel === s.star ? '▾' : '▸'}</span>{s.name}</td><td className="num">{fmt(s.distance, 1)}</td><td className="num">{num(s.cubes, 0)}</td>
              <td className="num">{num(s.production_buildings, 0)}</td><td className="num">{num(s.miners, 0)}</td><td className="num">{num(s.collectors, 0)}</td><td className="num">{fmt(s.stations, 1)}</td>
            </tr>,
            sel === s.star && <tr key={`${s.star}-d`}><td colSpan={7} className="small">
              {s.extraction.length > 0 && <div><b>开采</b>：{s.extraction.map(e => `${name(e.item.replace(' (gas)', ''))}${e.item.endsWith('(gas)') ? '（气巨）' : ''} ${num(e.rate, 1)}/分钟${e.cap !== null ? `，上限 ${num(e.cap, 0)} ${e.limit}` : ''}`).join(' · ')}</div>}
              {s.drops.length > 0 && <div><b>黑雾掉落</b>：{s.drops.map(d => `${name(d.item)} ${num(d.rate, 1)}/分钟`).join(' · ')}</div>}
              <div><b>配方</b>：{s.recipes.length === 0 ? '无' : s.recipes.slice(0, 40).map(r => `${recipeName(r.recipe)}${r.mode ? `［${r.mode}］` : ''} ×${num(r.machines, 1)}`).join(' · ')}{s.recipes.length > 40 ? ` … 还有 ${s.recipes.length - 40} 个` : ''}</div>
            </td></tr>,
          ])}
        </tbody>
      </table>
    </div>
    <h4>星际运输（{gp.shipments.length} 条，按 物量×光年 从大到小）</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>物品</th><th>从 → 到</th><th className="num">/分钟</th><th className="num">光年</th></tr></thead>
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
    return <div className="warn"><b>{STATUS_ZH[t.status] ?? '没有结果'}</b>{t.warnings.map((w, i) => <p key={i}>{w}</p>)}</div>
  }
  const binding = t.finite.filter(u => u.binding)
  return <>
    <div className="cards">
      <div className="card"><span className="big">{compact(t.cubes)}</span><span className="muted">白糖总数{t.levels_done > 0 ? `（已完成 ${t.levels_done} 级矿物利用）` : '（不升矿物利用）'}</span></div>
      {level > 1 && <div className="card"><span className="big">{compact(t.cubes_no_util / Math.pow(0.94, level - 1))}</span><span className="muted">完成 {level - 1} 级时（×{fmt(1 / Math.pow(0.94, level - 1), 0)}）</span></div>}
      <div className="card"><span className="big">{binding.length}</span><span className="muted">卡住：{binding.map(u => name(u.item)).join('、') || '—'}</span></div>
      {t.hub && <div className="card"><span className="big">{compact(t.warpers)}</span><span className="muted">翘曲器（中心 {t.hub}）</span></div>}
    </div>
    <p className="small muted">{t.stars} 颗星。这里算作无限的：{t.unlimited.map(name).join('、')}。{fmt(t.solve_ms, 0)} ms。</p>
    {t.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
    <h4>有限储量</h4>
    <div className="tablewrap">
      <table className="mbtable">
        <thead><tr><th>资源</th><th className="num">可采</th><th className="num">用了</th><th className="num">用量</th><th className="num">每个白糖</th><th className="num">每多 1000 多几个白糖</th></tr></thead>
        <tbody>
          {t.finite.map(u => (
            <tr key={u.item}><td><Icon id={u.item} size={18} />{name(u.item)}</td><td className="num">{compact(u.available)}</td><td className="num">{compact(u.used)}</td>
              <td className="num">{u.binding ? <span className="st-warn">用完，卡住</span> : `${fmt(100 * u.used / Math.max(u.available, 1e-12), 0)} %`}</td>
              <td className="num">{fmt(u.per_cube, 3)}</td><td className="num">{u.binding ? fmt(1000 * u.marginal, 2) : '—'}</td></tr>
          ))}
        </tbody>
      </table>
    </div>
    <h4>配方：每 1 白糖/分钟要几台机器</h4>
    <p className="small">{t.recipes.slice(0, 60).map(r => `${recipeName(r.recipe)}${r.mode ? `［${r.mode}］` : ''} ×${fmt(r.machines_per_rate, 2)}`).join(' · ')}{t.recipes.length > 60 ? ` … 还有 ${t.recipes.length - 60} 个` : ''}</p>
    <h4>每个白糖的原料</h4>
    <p className="small">{t.raw_per_cube.map(r => `${name(r.item)} ${fmt(r.rate, 2)}`).join(' · ')} <span className="muted">（无限的原料按总投入计）</span></p>
  </>
}
