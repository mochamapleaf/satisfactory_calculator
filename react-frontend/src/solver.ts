import { lang, t } from './i18n'
// Typed wrapper around the Rust/WASM planner (src/wasm_api.rs).
import init, { compute_weights_json, game_data_json, mixed_belt_json, solve_json, tier_recipes_json } from './wasm-pkg/satisfactory_calculator.js'

export type Game = 'satisfactory' | 'dsp' | 'factorio'
export const GAMES: { id: Game; name: string }[] = [
  { id: 'satisfactory', name: 'Satisfactory' },
  { id: 'dsp', name: '戴森球计划' },
  { id: 'factorio', name: 'Factorio' },
]
export interface Item { id: string; name: string; name_zh?: string | null; liquid: boolean; sink: number; raw: boolean; group?: string; row?: number; order?: number }
export interface Recipe {
  id: string; name: string; name_zh?: string | null; alt: boolean; building: string; producers?: string[]; time: number
  in: [string, number][]; out: [string, number][]; power?: number
  /** Factorio item recycling (used only in quality plans). */
  recycling?: boolean
}
export interface Building {
  id: string; name: string; name_zh?: string | null; power: number; power_exp: number; somersloop_slots: number
  speed?: number; modules?: number
}
export interface EffectTier { speed: number; productivity: number; consumption: number; quality: number }
export interface Effect {
  id: string; name: string; name_zh?: string | null; kind: string; speed: number; productivity: number; consumption: number
  quality?: number; sprays?: number | null; item: string; by_quality?: EffectTier[]
}

/** Factorio data sets (data::FACTORIO_DATASETS). */
export const DATASETS: { id: string; name: string; quality: boolean; research: boolean }[] = [
  { id: '2.0', name: '2.0.77 base', quality: false, research: false },
  { id: '2x1', name: '2.1.25 Space Age', quality: true, research: true },
  { id: '2q1', name: '2.1.25 base + quality', quality: true, research: false },
  { id: 'spa', name: '2.0.77 Space Age', quality: true, research: true },
]
/** Settings for Factorio data set `id`: a set without quality turns quality targets and
 *  supplies back to normal (rows of the same item merge); a set without research technologies
 *  drops research levels (the solver also ignores recipes the set's research does not name). */
export function withDataset(s: Settings, id: string): Settings {
  const ds = DATASETS.find(d => d.id === id)
  if (!ds?.research && Object.keys(s.research_productivity ?? {}).length) s = { ...s, research_productivity: {} }
  if (ds?.quality) return { ...s, dataset: id }
  const plain = (l: ItemRate[]) => {
    const out: ItemRate[] = []
    for (const t of l) {
      const item = qsplit(t.item)[0]
      const same = out.find(x => x.item === item)
      if (same) same.rate += t.rate
      else out.push({ item, rate: t.rate })
    }
    return out
  }
  return { ...s, dataset: id, targets: plain(s.targets), supplies: plain(s.supplies) }
}
/** Quality tiers: "iron-gear-wheel@4" → ["iron-gear-wheel", 4]; normal keeps the bare id. */
export function qsplit(id: string): [string, number] {
  const m = /^(.+)@([1-4])$/.exec(id)
  return m ? [m[1], +m[2]] : [id, 0]
}
export const withQ = (id: string, q: number) => (q > 0 ? `${qsplit(id)[0]}@${q}` : qsplit(id)[0])
export const QUALITY_NAMES = ['normal', 'uncommon', 'rare', 'epic', 'legendary']
// official Chinese tier names (Factorio wiki Quality/zh)
const QUALITY_ZH = ['基础级', '精良级', '稀有级', '史诗级', '传说级']
/** Quality tier names in the interface language (the game's Chinese names). */
export function qualityNames(data: GameData): string[] {
  const n = data.qualities ?? []
  return lang() === 'zh' ? n.map((x, i) => QUALITY_ZH[i] ?? x) : n
}
/** Item name with its quality ("Iron gear wheel ★legendary"). */
export function itemName(data: GameData, id: string): string {
  // ore still in the ground, dug by quality-mining drills (data.rs VEIN)
  if (id.endsWith('#vein')) return `${itemName(data, id.slice(0, -5))}${t(' (in the ground)', '（地下矿脉）')}`
  const [base, q] = qsplit(id)
  const it = data.items.find(i => i.id === base)
  const n = dn(it, data.game, base)
  return q ? `${n} ★${(qualityNames(data)[q] ?? QUALITY_NAMES[q]).toLowerCase()}` : n
}
export const isLiquid = (data: GameData, id: string) => data.items.find(i => i.id === qsplit(id)[0])?.liquid ?? false
export interface Beacon {
  name_zh?: string | null
  id: string; name: string; effectivity: number; modules: number; power: number; profile: number[]
  by_quality?: { effectivity: number; power: number }[]
}
export interface Resource { item: string; limit: number | null; extractor: string; extractor_rate: number; extractor_power: number }
export interface GameData {
  game: Game; effects?: Effect[]
  /** Factorio: quality tier names (normal … legendary); absent = no quality in this data set. */
  qualities?: string[]; dataset?: string
  /** Space Age planets / space platform (absent = no planet filter). */
  locations?: { id: string; name: string; name_zh?: string | null }[]
  /** Researched productivity families: technology, recipes it boosts, productivity per level. */
  research?: { id: string; name: string; name_zh?: string | null; recipes: string[]; step: number }[]
  beacons?: Beacon[]
  version: string; source: string; items: Item[]; recipes: Recipe[]; buildings: Building[]
  resources: Resource[]; belts: number[]; pipes: number[]
}

export type Objective = 'resources' | 'buildings' | 'power' | 'compute' | 'custom'
export interface CostWeights {
  resources: number; buildings: number; power: number; shards: number; surplus: number
  resource_weights: Record<string, number>; building_weights: Record<string, number>
  /** DSP compute terms: per sorter port, per building (belts + stations), per sprayed building. */
  sorter: number; per_machine: number; coater: number
}
/** Same numbers as Objective::preset in src/planner/mod.rs; Compute comes from the WASM
 *  module (coefficients of docs/research/dsp-ups/model.json), so call it after loadSolver. */
export function preset(o: Objective, generator: Generator = 'artificial_star'): CostWeights {
  if (o === 'compute') return JSON.parse(compute_weights_json(generator)) as CostWeights
  const [resources, buildings, power, shards] =
    o === 'buildings' ? [0.001, 1, 0, 0.01] : o === 'power' ? [0.001, 1, 1, 0.01] : [1, 0.01, 0, 0.01]
  return { resources, buildings, power, shards, surplus: 0, resource_weights: {}, building_weights: {}, sorter: 0, per_machine: 0, coater: 0 }
}
/** Generators for the DSP Compute objective (keys of model.json generation.cost_per_mw). */
export type Generator = 'artificial_star' | 'mini_fusion' | 'thermal' | 'solar' | 'wind'
export const GENERATORS: { id: Generator; name: string; name_en: string }[] = [
  { id: 'artificial_star', name: '人造恒星', name_en: 'Artificial star' },
  { id: 'mini_fusion', name: '微型聚变电站', name_en: 'Mini fusion power plant' },
  { id: 'thermal', name: '火力发电厂', name_en: 'Thermal power plant' },
  { id: 'solar', name: '太阳能板', name_en: 'Solar panel' },
  { id: 'wind', name: '风力涡轮机', name_en: 'Wind turbine' },
]
/** Default scarcity weight of a raw resource (Iron Ore = 1), as in lp.rs. */
export const scarcity = (limit: number | null) => (limit && limit > 0 ? 92100 / limit : 0.001)
export interface ItemRate { item: string; rate: number }
export interface Settings {
  game: Game
  /** Satisfactory: build generators for the plan's own power (generators as recipes, memo step 8). */
  self_power: boolean
  /** Max-output mode: the targets are a ratio, scaled up to the resource limits (memo step 8). */
  max_output: boolean
  /** Factorio quality plans: drills with quality modules reroll ore (memo step 15). */
  quality_mining: boolean
  /** Satisfactory: highest clock per recipe id, percent (overrides max_clock_percent). */
  recipe_clocks: Record<string, number>
  targets: ItemRate[]
  supplies: ItemRate[]
  objective: Objective
  allow_all_alternates: boolean
  enabled_alternates: string[]
  disabled_recipes: string[]
  resource_limits: Record<string, number | null>
  ignore_limits: boolean
  allow_ore_conversion: boolean
  max_clock_percent: number
  somersloops: Record<string, number>
  disabled_buildings: string[]
  effects: string[]
  auto_somersloops: boolean
  somersloop_budget: number | null
  integer: boolean
  shard_budget: number | null
  cost: CostWeights
  time_limit_ms: number
  /** Whole-building MILP engine: microlp (built in) or HiGHS (loaded on first use); auto runs both
   *  in parallel workers and shows the better plan. */
  engine: Engine
  generator: Generator
  mixed_belts: boolean
  /** Factorio data set ('' = 2.0 base). */
  dataset: string
  /** Factorio: quality tier of all modules (0 normal … 4 legendary). */
  module_quality: number
  /** Factorio quality plans: module ids for quality / productivity slots ('' = none). */
  quality_module: string
  prod_module: string
  /** Quality plans: recycling scope. */
  recycling: 'chain' | 'all' | 'list'
  /** Items whose recycling is allowed in any scope. */
  recycle_allow: string[]
  research_productivity: Record<string, number>
  /** Space Age: planets whose recipes and resources may be used (default Nauvis). */
  planets: string[]
  /** Factorio beacons: module in each beacon ('' = none), counts per machine offered to the
   *  solver, beacon quality, machines sharing one beacon. */
  beacon_module: string
  beacon_counts: number[]
  beacon_quality: number
  beacon_share: number
  /** Factorio: quality tier of every crafting machine (0 normal … 4 legendary): crafting speed from the data. */
  machine_quality: number
  /** Factorio: objective cost per module held, by module quality (normal … legendary); [] = free. */
  module_costs: number[]
  /** Solver guard: models with more columns are not solved (0 = no limit). Set per request, not saved. */
  max_columns?: number
  /** DSP mixed-belt tab (src/planner/mixed_belt.rs). Not part of the solve. */
  belt_layout: BeltLayoutSettings
}

// ---- DSP mixed-belt (混带) layout, docs/research/dsp-mixed-belt/00-最终报告.md (src/planner/mixed_belt.rs)
export interface TwoOverride { period?: number | null; first?: number | null; k?: number | null; sigma5?: number | null }
export interface BeltLayoutSettings {
  /** 0 yellow, 1 green, 2 blue */
  belt: number
  stack: number
  /** machine rows fed by one belt (1 | 2) */
  rows: number
  row_len: number | null
  unit_max: number | null
  sorter_max: number
  pile_level: number
  margin: number
  ladder_rho: number
  ladder_short: number
  ladder_cap: number
  unit_cap: number | null
  hint_items: number
  hint_machines: number
  end_pile_a: number
  end_pile_b: number
  /** station feeding a line: interstellar ("ils") or planetary ("pls"); slots used (≤ 12) */
  station: 'ils' | 'pls'
  station_slots: number
  /** one ring through several stations on one control unit (else one module per station) */
  station_ring: boolean
  /** per "recipe|mode" */
  overrides: Record<string, TwoOverride>
}
export const defaultBeltLayout = (): BeltLayoutSettings => ({
  belt: 2, stack: 4, rows: 1, row_len: null, unit_max: null, sorter_max: 2, pile_level: 0, margin: 1.5,
  ladder_rho: 1 / 12, ladder_short: 12, ladder_cap: 24, unit_cap: null, hint_items: 12, hint_machines: 60,
  end_pile_a: 2, end_pile_b: 3, station: 'ils', station_slots: 12, station_ring: false, overrides: {},
})
/** Mixed-belt settings from anything (a link, saved settings, a typed number): every count a whole
 * number in its range, optional counts null, unknown fields dropped. Mirrors
 * `BeltLayoutSettings::from_value` in src/planner/mixed_belt.rs, which reads the same object. */
export function cleanBeltLayout(raw: unknown): BeltLayoutSettings {
  const d = defaultBeltLayout()
  const v = (raw && typeof raw === 'object' && !Array.isArray(raw) ? raw : {}) as Record<string, unknown>
  const num = (o: Record<string, unknown>, k: string) => (typeof o[k] === 'number' && Number.isFinite(o[k] as number) ? (o[k] as number) : null)
  const int = (k: keyof BeltLayoutSettings, lo: number, hi: number) => { const x = num(v, k); return x === null ? (d[k] as number) : Math.min(hi, Math.max(lo, Math.round(x))) }
  const opt = (o: Record<string, unknown>, k: string, hi: number) => { const x = num(o, k); return x === null || Math.round(x) < 1 ? null : Math.min(hi, Math.round(x)) }
  const real = (k: keyof BeltLayoutSettings, lo: number, hi: number) => { const x = num(v, k); return x === null || x < lo || x > hi ? (d[k] as number) : x }
  const overrides: Record<string, TwoOverride> = {}
  if (v.overrides && typeof v.overrides === 'object' && !Array.isArray(v.overrides)) {
    for (const [key, o] of Object.entries(v.overrides as Record<string, unknown>)) {
      if (!o || typeof o !== 'object') continue
      const r = o as Record<string, unknown>
      const ov: TwoOverride = {}
      const period = opt(r, 'period', 100000), k = opt(r, 'k', 1000000), first = num(r, 'first'), sigma5 = num(r, 'sigma5')
      if (period !== null) ov.period = period
      if (first === 0 || first === 1) ov.first = first
      if (k !== null) ov.k = k
      if (sigma5 !== null && [1, 2, 3, 4].includes(sigma5)) ov.sigma5 = sigma5
      if (Object.keys(ov).length) overrides[key] = ov
    }
  }
  return {
    belt: int('belt', 0, 2), stack: int('stack', 1, 4), rows: int('rows', 1, 2), row_len: opt(v, 'row_len', 100000),
    unit_max: opt(v, 'unit_max', 10000000), sorter_max: int('sorter_max', 0, 2), pile_level: int('pile_level', 0, 2),
    margin: real('margin', 0, 1000), ladder_rho: real('ladder_rho', 0, 10), ladder_short: int('ladder_short', 0, 1000000),
    ladder_cap: int('ladder_cap', 1, 1000), unit_cap: opt(v, 'unit_cap', 20000), hint_items: real('hint_items', 0, 1e9),
    hint_machines: int('hint_machines', 0, 10000000), end_pile_a: int('end_pile_a', 0, 1000), end_pile_b: int('end_pile_b', 0, 1000),
    station: v.station === 'pls' ? 'pls' : 'ils', station_slots: int('station_slots', 1, 12), station_ring: v.station_ring === true,
    overrides,
  }
}
export interface BeltNote { level: 'fail' | 'warn' | 'info'; text: string; cite: string }
export interface BeltRate { item: string; name: string; items: number; stacks: number }
export interface TracePoint { pos: number; kind: 'unit' | 'machine' | 'fill' | 'feed' | 'recover' | 'coater'; item: number | null; amount: number; flows: number[] }
export interface Trace { capacity: number; points: TracePoint[]; mins: number[]; returns: number[]; passes: number; converged: boolean; violations: string[] }
export interface Candidate { period: number; n: [number, number]; sum: number; unit: number; best: number | null; feasible: boolean; reason: string }
export interface TwoItemDesign {
  c: [number, number]; p: number; q: number; ratio_dev: number; period: number; n: [number, number]; sum: number
  sigma5: number; first: number; k: number; best: number; best_mins: [number, number]; line_mins: [number, number]
  candidates: Candidate[]; recommended: [number, number, number, number, number] | null; overridden: boolean
  feeds: { pos: number; item: number; amount: number }[]; returns: [number, number]; shares: [number, number]
  jump: number; limiter_lane: string; unit: 'standard' | 'double' | 'unknown'; complete_add: number
  tower_belts: [number, number]; units: number; machines_per_unit: number
}
export interface LadderDesign {
  group: number; groups: number; last_group: number; rho: number; total: number; c: number[]
  feeds: { item: number; need: number; rate: number; combo: string }[]; recover_sorters: number[]
}
export interface SorterPlan {
  input: string; input_items: number; end_pile: number; mixed: number; separate: number; lp_estimate: number
  outputs: { item: string; name: string; rate: number; sized_for: number; tier: string; full_at: number }[]
}
export interface RecipeLayout {
  key: string; id: string; mode: string; mode_label: string; name: string; building: string; machines: number
  inputs: BeltRate[]; outputs: BeltRate[]; scheme: 'A' | 'B' | 'none'; status: 'ok' | 'warn' | 'fail'; notes: BeltNote[]
  two: TwoItemDesign | null; ladder: LadderDesign | null; sorters: SorterPlan
  coaters: { coaters: number; sprays_per_s: number; proliferator_per_s: number; proliferator: string } | null
  trace: Trace
  /** machines left out of the trace (whole periods of a long line); 0 = the whole line */
  trace_omitted: number
  stations: StationPlan | null
}
// station module (src/planner/mixed_station.rs)
export interface StationPort { slot: number; dir: 'in' | 'out'; item: string; name: string; role: string; items: number; serves: number[] }
export interface StationPlan {
  kind: 'ils' | 'pls'; slots: number; lines: number; stations_total: number
  stations: { index: number; first: number; last: number; ports: StationPort[]; items: number; limit: number }[]
  refills: { after: number; item: string; name: string; amount: number; station: number; slots: number[] }[]
  machines: { n: number; station: number; pos: number; side: 'L' | 'R'; input: string; outputs: [number, string][] }[]
  sequence: string[]; notes: string[]
}
export interface MixedBeltReport { capacity: number; stack: number; margin: number; recipes: RecipeLayout[]; single: string[]; warnings: string[]; error?: string }
/** Mixed-belt layout of a solved plan (fast, runs on the page thread; no re-solve). */
export function mixedBelt(s: Settings, plan: Plan): MixedBeltReport {
  const empty: MixedBeltReport = { capacity: 30, stack: 4, margin: 1.5, recipes: [], single: [], warnings: [] }
  try {
    // an error comes back as { error } alone: the page still gets every list
    return { ...empty, ...(JSON.parse(mixed_belt_json(JSON.stringify(s), JSON.stringify(plan))) as Partial<MixedBeltReport>) }
  } catch (e) {
    return { ...empty, error: String(e) }
  }
}

export interface RateLine { item: string; name: string; rate: number }
export interface PlanRecipe {
  id: string; mode: string; mode_label: string; name: string; alt: boolean; building: string; machines: number; machines_ceil: number
  throughput: number; shards: number; groups: MachineGroup[]
  power: number; inputs: RateLine[]; outputs: RateLine[]; somersloops: number
  /** DSP: compute units per tick of this recipe (incl. its power). */
  compute: number
  /** Factorio: beacons charged to this recipe. */
  beacons?: number
  /** Factorio quality plans: whole machines of the row's shared group (tiers of one recipe on one
   *  machine type and loadout switch recipes); `machines` is this tier's share. */
  group_machines?: number
}
/** DSP compute (UPS) cost of a plan; 1 unit = one machine updated once per logic tick. */
export interface ComputeSummary {
  total: number; machines: number; sorters: number; logistics: number; coaters: number; generation: number
  generator: Generator; cost_per_mw: number; us_per_tick: number; us_low: number; us_high: number
}
export interface MachineGroup { count: number; clock_percent: number; shards: number }
export interface ItemValue { item: string; name: string; value: number }
export interface Flow { from: string; to: string; item: string; name: string; rate: number }
export interface Plan {
  /** too_large: over `max_columns`, not solved (columns / rows set); cancelled: stopped by the user or a newer request. */
  status: 'empty' | 'optimal' | 'infeasible' | 'unbounded' | 'error' | 'too_large' | 'cancelled'
  objective: number; targets: RateLine[]; recipes: PlanRecipe[]; raw: RateLine[]; supplied: RateLine[]
  surplus: RateLine[]; production_power: number; extraction_power: number; buildings: number
  /** Buildings to place per building name (sums to `buildings`). */
  building_counts?: { building: string; count: number }[]
  shards: number; somersloops: number; proven_optimal: boolean; gap: number | null; lp_bound: number; solve_ms: number
  /** The gap in objective weights of the cheapest building the plan uses (a better plan may also trade resources or power); null when the building rule is off. */
  gap_buildings?: number | null
  item_values: ItemValue[]; flows: Flow[]; warnings: string[]
  compute?: ComputeSummary | null
  /** Model size: solver columns and item rows; quality = item@q expansion was used. */
  columns?: number; rows?: number; quality?: boolean
  build_materials?: RateLine[]
}

export const emptyPlan = (): Plan => ({
  status: 'empty', objective: 0, targets: [], recipes: [], raw: [], supplied: [], surplus: [], production_power: 0,
  extraction_power: 0, buildings: 0, shards: 0, somersloops: 0, proven_optimal: false, gap: null, lp_bound: 0, solve_ms: 0,
  item_values: [], flows: [], warnings: [],
})

const DEFAULT_TARGET: Record<Game, ItemRate> = {
  satisfactory: { item: 'Desc_IronPlateReinforced_C', rate: 10 },
  dsp: { item: 'electromagnetic-matrix', rate: 60 },
  factorio: { item: 'electronic-circuit', rate: 60 },
}
export const defaultSettings = (game: Game = 'satisfactory'): Settings => ({
  game,
  targets: [DEFAULT_TARGET[game]],
  supplies: [],
  objective: 'resources',
  allow_all_alternates: false,
  enabled_alternates: [],
  disabled_recipes: [],
  resource_limits: {},
  ignore_limits: false,
  allow_ore_conversion: false,
  max_clock_percent: 100,
  somersloops: {},
  recipe_clocks: {},
  self_power: false,
  max_output: false,
  quality_mining: false,
  // DSP Dark Fog machines are endgame drops: off unless the player enables them
  disabled_buildings: game === 'dsp' ? ['df-negentropy-smelter', 'df-recomposing-assembler', 'df-self-evolution-lab'] : [],
  effects: [],
  auto_somersloops: false,
  somersloop_budget: null,
  integer: true,
  shard_budget: null,
  cost: preset('resources'),
  time_limit_ms: 4000,
  engine: 'auto',
  generator: 'artificial_star',
  mixed_belts: false,
  dataset: game === 'factorio' ? '2.0' : '',
  module_quality: 0,
  quality_module: 'quality-module-3',
  prod_module: 'productivity-module-3',
  recycling: 'chain',
  recycle_allow: [],
  research_productivity: {},
  planets: ['nauvis'],
  beacon_module: '',
  beacon_counts: [8],
  beacon_quality: 0,
  beacon_share: 1,
  machine_quality: 0,
  module_costs: [],
  belt_layout: defaultBeltLayout(),
})

let wasm: Promise<unknown> | null = null
const cache = new Map<string, GameData>()
export async function loadSolver(game: Game, dataset = ''): Promise<GameData> {
  wasm ??= init()
  await wasm
  const key = `${game}/${game === 'factorio' ? dataset : ''}`
  if (!cache.has(key)) {
    const d = JSON.parse(game_data_json(game, game === 'factorio' ? dataset : '')) as GameData
    d.game ??= game // Satisfactory's JSON predates the field
    d.dataset ??= game === 'factorio' ? '2.0' : ''
    cache.set(key, d)
  }
  return cache.get(key)!
}

/** Display name in the interface language: Chinese where the data has it (`tools/add_zh_names.py`). */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
export function dn(x: { name: string; name_zh?: string | null } | undefined, _game: Game, fallback = ''): string {
  if (!x) return fallback
  return lang() === 'zh' && x.name_zh ? x.name_zh : x.name
}

/** Satisfactory recipe ids unlocked by tech tier `tier` (memo step 14: the map's tier limits recipes too). */
export function tierRecipes(tier: number): string[] {
  return JSON.parse(tier_recipes_json(tier)) as string[]
}

export function solve(s: Settings): Plan {
  return JSON.parse(solve_json(JSON.stringify({ ...s, lang: lang() }))) as Plan
}

// Solves in a Web Worker and streams the best plan so far (whole buildings: the rounded LP plan at
// once, then every improvement, report solver-speed §6 step 3). A newer request supersedes older
// ones: WASM cannot be interrupted inside a step, so a busy worker is terminated and the next
// request starts a fresh one. A cancel after the first plan stops the worker between steps and
// keeps its search, so "Keep solving" can resume the same tree.
export type Engine = 'auto' | 'microlp' | 'highs'
export interface SolveUpdate {
  /** The best plan, when it changed (the final update of a solve always has it). */
  plan: Plan | null
  done: boolean
  elapsed_ms?: number
  objective?: number | null
  bound?: number
  gap?: number | null
  proven?: boolean
  /** heuristic | restricted | search | done */
  phase?: string
  nodes?: number
  deadline_ms?: number
  /** resume: the worker no longer holds this solve */
  gone?: boolean
  /** HiGHS is running (the worker cannot take messages until it returns) */
  engine_busy?: boolean
  /** engine that ran the search, HiGHS load time and final status */
  engine?: string
  highs_load_ms?: number
  highs_status?: string
}
type OnUpdate = (u: SolveUpdate) => void

/** One worker and the solve it runs (or holds, idle, for "Keep solving"). */
type Held = { id: number; onUpdate: OnUpdate; gotPlan: boolean; busy: boolean }
class Lane {
  worker: Worker | null = null
  running: Held | null = null
  idle: number | null = null
  /** the solve just stopped: its worker may still post the step it was in (review 4 L3) */
  private late: Held | null = null
  private open() {
    const w = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' })
    this.worker = w
    w.onmessage = (e: MessageEvent<{ id: number; update: string }>) => {
      if (this.worker !== w) return
      const u = JSON.parse(e.data.update) as SolveUpdate
      // error plans carry only status and warnings: complete them, so every tab can render
      if (u.plan) u.plan = { ...emptyPlan(), ...u.plan }
      const r = this.running
      if (r?.id !== e.data.id) {
        const l = this.late
        if (l?.id !== e.data.id) return
        // HiGHS started just before the stop reached the worker: it cannot be stopped, end it
        if (u.engine_busy) { this.stop(); return }
        // the step that was running when the solve was stopped: its plan is the newest
        if (u.plan?.status === 'optimal') l.onUpdate({ ...u, done: true })
        return
      }
      if (u.plan) r.gotPlan = true
      r.busy = !!u.engine_busy
      if (u.done) { this.running = null; this.idle = u.gone ? null : r.id }
      // a solver error (a WASM trap) leaves the module unusable: the next solve gets a new worker
      if (u.done && u.plan?.status === 'error') this.stop()
      r.onUpdate(u)
    }
    w.onerror = e => {
      if (this.worker !== w) return
      const r = this.running
      this.stop()
      r?.onUpdate({ done: true, plan: { ...emptyPlan(), status: 'error', warnings: [`The solver worker failed: ${e.message || 'it could not be loaded'}. Use "Solve again" to retry.`] } })
    }
  }
  stop() {
    this.worker?.terminate()
    this.worker = null
    this.running = null
    this.idle = null
    this.late = null
  }
  start(id: number, s: Settings, onUpdate: OnUpdate, race = false) {
    if (this.running) this.stop()
    if (!this.worker) this.open()
    this.running = { id, onUpdate, gotPlan: false, busy: false }
    this.idle = null
    this.late = null
    this.worker!.postMessage({ type: 'solve', id, settings: JSON.stringify(s), race })
  }
  keep(id: number, ms: number, onUpdate: OnUpdate): boolean {
    if (this.running || this.idle !== id || !this.worker) return false
    this.running = { id, onUpdate, gotPlan: true, busy: false }
    this.idle = null
    this.late = null
    this.worker.postMessage({ type: 'resume', id, ms })
    return true
  }
  /** After the first plan the worker keeps the search (see `keep`); before it, or while HiGHS
   *  blocks the worker, the worker is terminated. */
  cancel() {
    const r = this.running
    if (!r) return
    if (r.gotPlan && this.worker && !r.busy) {
      this.worker.postMessage({ type: 'stop', id: r.id })
      this.running = null
      this.idle = r.id
      this.late = r
    } else this.stop()
  }
}

// lane 0: the requested engine (microlp for Auto); lane 1: HiGHS racing it under Auto
const lanes = [new Lane(), new Lane()]
let seq = 0

/** The cheaper plan; at equal cost the proven one (both lanes solve the same model). */
export const better = (a: Plan | null, b: Plan | null) => {
  if (!a || a.status !== 'optimal') return b ?? a
  if (!b || b.status !== 'optimal') return a
  const tol = 1e-9 * Math.max(1, Math.abs(a.objective))
  if (b.objective < a.objective - tol) return b
  if (a.objective < b.objective - tol) return a
  return b.proven_optimal && !a.proven_optimal ? b : a
}
/** A proof by either lane covers the cheaper plan of the two: it is within the same gap. */
export const asProven = (p: Plan): Plan =>
  // the tolerance depends on the plan's own cheapest building: a plan more than one of its buildings
  // above its bound stays unproven (its own bound is no higher than the proof's, so this is safe)
  p.status !== 'optimal' || p.proven_optimal || (p.gap_buildings ?? 0) >= 0.99 ? p : { ...p, proven_optimal: true, gap: null, gap_buildings: null, warnings: p.warnings.filter(w => !w.startsWith('Time limit reached')) }

/** Auto: microlp and HiGHS solve the same plan in two workers; the page sees the better plan at
 *  every moment, so Auto is never worse than microlp alone. A proof by either ends both. */
let raceBest: (Plan | null)[] = [null, null]
function race(onUpdate: OnUpdate, keep = false) {
  const last: (SolveUpdate | null)[] = [null, null]
  let shown: Plan | null = null
  let over = false
  // "Keep solving": a lane whose search is gone still holds its best plan
  const best: (Plan | null)[] = keep ? raceBest : [null, null]
  raceBest = best
  return (k: number) => (u: SolveUpdate) => {
    last[k] = { ...u, plan: null }
    if (u.plan) best[k] = u.plan
    // a result without a search (too large, infeasible, an LP) is the same
    // in both lanes: the first one ends the solve
    const direct = u.done && u.phase === undefined && !u.gone && !!u.plan && u.plan.status !== 'error'
    const proven = last.some(x => x?.proven) || best.some(p => p?.proven_optimal)
    const done = direct || proven || last.every(x => x?.done)
    if ((proven || direct) && !over) { over = true; for (const l of lanes) l.cancel() }
    let pick = direct ? u.plan : better(best[0], best[1])
    if (pick && proven && !direct) pick = asProven(pick)
    const changed = !!pick && (!shown || pick.objective !== shown.objective || pick.proven_optimal !== shown.proven_optimal || pick.status !== shown.status)
    const plan = changed || done ? pick : null
    if (plan) shown = plan
    const bounds = last.map(x => x?.bound).filter((b): b is number => typeof b === 'number' && Number.isFinite(b))
    const one = last[k]!
    onUpdate({
      ...one,
      plan: plan ?? (done && !pick ? u.plan : null),
      done,
      proven,
      bound: bounds.length ? Math.max(...bounds) : undefined,
      elapsed_ms: Math.max(...last.map(x => x?.elapsed_ms ?? 0)),
      deadline_ms: Math.max(...last.map(x => x?.deadline_ms ?? 0)),
      engine: `race (${last.map(x => x?.engine ?? '–').join(' / ')})`,
      highs_status: last[1]?.highs_status,
      engine_busy: false,
      gone: last.every(x => x?.gone) || undefined,
    })
  }
}

let raceIds: [number, number] | null = null

/** Start a solve; `onUpdate` gets the streamed updates. Returns the solve id. */
export function solveStream(settings: Settings, onUpdate: OnUpdate): number {
  // names in the plan follow the interface language (data.rs `pick`)
  const s = { ...settings, lang: lang() }
  const id = ++seq
  if (s.integer && (s.engine ?? 'auto') === 'auto') {
    const h = race(onUpdate)
    lanes[0].start(id, { ...s, engine: 'microlp' }, h(0))
    lanes[1].start(id, { ...s, engine: 'highs' }, h(1), true)
    raceIds = [id, id]
  } else {
    lanes[1].cancel()
    raceIds = null
    lanes[0].start(id, s, onUpdate)
  }
  return id
}
/** One-shot solve (final plan only). */
export function solveAsync(s: Settings): Promise<Plan> {
  return new Promise(resolve => solveStream(s, u => { if (u.done) resolve(u.plan ?? { ...emptyPlan(), status: 'cancelled' }) }))
}
/** Resume solve `id` for `ms` more where it stopped. False when no worker holds it any more. */
export function keepSolving(id: number, ms: number, onUpdate: OnUpdate): boolean {
  if (raceIds && raceIds[0] === id) {
    const h = race(onUpdate, true)
    const a = lanes[0].keep(id, ms, h(0))
    const b = lanes[1].keep(id, ms, h(1))
    if (a !== b) {
      // only one search is left: it alone continues (the other counts as done)
      h(a ? 1 : 0)({ plan: null, done: true, phase: 'done' })
    }
    return a || b
  }
  return lanes[0].keep(id, ms, onUpdate)
}
/** Stop the running solve(s), if any. */
export function cancelSolve() {
  for (const l of lanes) l.cancel()
}
/** Models above this many columns are not solved until the user asks (status 'too_large'). */
export const AUTO_SOLVE_COLUMNS = 6000
/** Longest whole-building time limit the solver accepts (it clamps to 300 s). */
export const MAX_TIME_MS = 300_000
export const clampTime = (sec: number) => Math.round(Math.min(MAX_TIME_MS / 1000, Math.max(0.2, Number.isFinite(sec) ? sec : 4)) * 1000)

/** Smallest belt / pipe tier that carries `rate`, e.g. "Mk.3" or "2× Mk.6". */
export function carrier(rate: number, liquid: boolean, data: GameData): string {
  const tiers = liquid ? data.pipes : data.belts
  const kind = liquid ? t('Pipe', '管道') : t('Belt', '传送带')
  if (!tiers.length) return '—'
  const i = tiers.findIndex(t => rate <= t + 1e-9)
  if (i >= 0) return `${kind} Mk.${i + 1}`
  const max = tiers[tiers.length - 1]
  return `${Math.ceil(rate / max)}× ${kind} Mk.${tiers.length}`
}

export { fmt } from './fmt'
