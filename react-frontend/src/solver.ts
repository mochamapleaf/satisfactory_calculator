// Typed wrapper around the Rust/WASM planner (src/wasm_api.rs).
import init, { compute_weights_json, game_data_json, solve_json } from './wasm-pkg/satisfactory_calculator.js'

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
/** Item name with its quality ("Iron gear wheel ★legendary"). */
export function itemName(data: GameData, id: string): string {
  const [base, q] = qsplit(id)
  const it = data.items.find(i => i.id === base)
  const n = dn(it, data.game, base)
  return q ? `${n} ★${(data.qualities?.[q] ?? QUALITY_NAMES[q]).toLowerCase()}` : n
}
export const isLiquid = (data: GameData, id: string) => data.items.find(i => i.id === qsplit(id)[0])?.liquid ?? false
export interface Beacon {
  id: string; name: string; effectivity: number; modules: number; power: number; profile: number[]
  by_quality?: { effectivity: number; power: number }[]
}
export interface Resource { item: string; limit: number | null; extractor: string; extractor_rate: number; extractor_power: number }
export interface GameData {
  game: Game; effects?: Effect[]
  /** Factorio: quality tier names (normal … legendary); absent = no quality in this data set. */
  qualities?: string[]; dataset?: string
  /** Space Age planets / space platform (absent = no planet filter). */
  locations?: { id: string; name: string }[]
  /** Researched productivity families: technology, recipes it boosts, productivity per level. */
  research?: { id: string; name: string; recipes: string[]; step: number }[]
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
export const GENERATORS: { id: Generator; name: string }[] = [
  { id: 'artificial_star', name: '人造恒星' },
  { id: 'mini_fusion', name: '微型聚变电站' },
  { id: 'thermal', name: '火力发电厂' },
  { id: 'solar', name: '太阳能板' },
  { id: 'wind', name: '风力涡轮机' },
]
/** Default scarcity weight of a raw resource (Iron Ore = 1), as in lp.rs. */
export const scarcity = (limit: number | null) => (limit && limit > 0 ? 92100 / limit : 0.001)
export interface ItemRate { item: string; rate: number }
export interface Settings {
  game: Game
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
  /** Factorio: objective cost per module held, by module quality (normal … legendary); [] = free. */
  module_costs: number[]
  /** Solver guard: models with more columns are not solved (0 = no limit). Set per request, not saved. */
  max_columns?: number
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
  item_values: ItemValue[]; flows: Flow[]; warnings: string[]
  compute?: ComputeSummary | null
  /** Model size: solver columns and item rows; quality = item@q expansion was used. */
  columns?: number; rows?: number; quality?: boolean
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
  // DSP Dark Fog machines are endgame drops: off unless the player enables them
  disabled_buildings: game === 'dsp' ? ['df-negentropy-smelter', 'df-recomposing-assembler', 'df-self-evolution-lab'] : [],
  effects: [],
  auto_somersloops: false,
  somersloop_budget: null,
  integer: true,
  shard_budget: null,
  cost: preset('resources'),
  time_limit_ms: 4000,
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
  module_costs: [],
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

/** Display name: Chinese for DSP when available. */
export function dn(x: { name: string; name_zh?: string | null } | undefined, game: Game, fallback = ''): string {
  if (!x) return fallback
  return game === 'dsp' && x.name_zh ? x.name_zh : x.name
}

export function solve(s: Settings): Plan {
  return JSON.parse(solve_json(JSON.stringify(s))) as Plan
}

// Solves in a Web Worker; a newer request supersedes older ones. WASM cannot be interrupted,
// so a request still running when a newer one (or a cancel) arrives terminates the worker and
// the next request starts a fresh one; the superseded promise resolves as 'cancelled'.
let worker: Worker | null = null
let seq = 0
let running: { id: number; resolve: (p: Plan) => void } | null = null
const cancelled = (): Plan => ({ ...emptyPlan(), status: 'cancelled' })
function stopWorker() {
  worker?.terminate()
  worker = null
  const r = running
  running = null
  r?.resolve(cancelled())
}
export function solveAsync(s: Settings): Promise<Plan> {
  if (running) stopWorker()
  if (!worker) {
    worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' })
    worker.onmessage = (e: MessageEvent<{ id: number; plan: string }>) => {
      if (running?.id !== e.data.id) return
      const r = running
      running = null
      r.resolve(JSON.parse(e.data.plan) as Plan)
    }
  }
  const id = ++seq
  return new Promise(resolve => {
    running = { id, resolve }
    worker!.postMessage({ id, settings: JSON.stringify(s) })
  })
}
/** Stop the running solve (if any); its promise resolves with status 'cancelled'. */
export function cancelSolve() {
  if (running) { seq++; stopWorker() }
}
/** Models above this many columns are not solved until the user asks (status 'too_large'). */
export const AUTO_SOLVE_COLUMNS = 6000
export const latestRequest = () => seq

/** Smallest belt / pipe tier that carries `rate`, e.g. "Mk.3" or "2× Mk.6". */
export function carrier(rate: number, liquid: boolean, data: GameData): string {
  const tiers = liquid ? data.pipes : data.belts
  const kind = liquid ? 'Pipe' : 'Belt'
  if (!tiers.length) return '—'
  const i = tiers.findIndex(t => rate <= t + 1e-9)
  if (i >= 0) return `${kind} Mk.${i + 1}`
  const max = tiers[tiers.length - 1]
  return `${Math.ceil(rate / max)}× ${kind} Mk.${tiers.length}`
}

export const fmt = (x: number, d = 2) =>
  Math.abs(x - Math.round(x)) < 1e-6 ? String(Math.round(x)) : x.toFixed(d).replace(/0+$/, '').replace(/\.$/, '')
