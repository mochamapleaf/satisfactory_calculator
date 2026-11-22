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
class Lane {
  worker: Worker | null = null
  running: { id: number; onUpdate: OnUpdate; gotPlan: boolean; busy: boolean } | null = null
  idle: number | null = null
  private open() {
    this.worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' })
    this.worker.onmessage = (e: MessageEvent<{ id: number; update: string }>) => {
      const r = this.running
      if (r?.id !== e.data.id) return
      const u = JSON.parse(e.data.update) as SolveUpdate
      if (u.plan) r.gotPlan = true
      r.busy = !!u.engine_busy
      if (u.done) { this.running = null; this.idle = u.gone ? null : r.id }
      r.onUpdate(u)
    }
  }
  stop() {
    this.worker?.terminate()
    this.worker = null
    this.running = null
    this.idle = null
  }
  start(id: number, s: Settings, onUpdate: OnUpdate) {
    if (this.running) this.stop()
    if (!this.worker) this.open()
    this.running = { id, onUpdate, gotPlan: false, busy: false }
    this.idle = null
    this.worker!.postMessage({ type: 'solve', id, settings: JSON.stringify(s) })
  }
  keep(id: number, ms: number, onUpdate: OnUpdate): boolean {
    if (this.running || this.idle !== id || !this.worker) return false
    this.running = { id, onUpdate, gotPlan: true, busy: false }
    this.idle = null
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
    } else this.stop()
  }
}

// lane 0: the requested engine (microlp for Auto); lane 1: HiGHS racing it under Auto
const lanes = [new Lane(), new Lane()]
let seq = 0

const better = (a: Plan | null, b: Plan | null) =>
  !a || a.status !== 'optimal' ? b : !b || b.status !== 'optimal' ? a : (b.proven_optimal && !a.proven_optimal) || b.objective < a.objective - 1e-9 * Math.max(1, Math.abs(a.objective)) ? b : a

/** Auto: microlp and HiGHS solve the same plan in two workers; the page sees the better plan at
 *  every moment, so Auto is never worse than microlp alone. A proof by either ends both. */
let raceBest: (Plan | null)[] = [null, null]
function race(onUpdate: OnUpdate, keep = false) {
  const last: (SolveUpdate | null)[] = [null, null]
  let shown: Plan | null = null
  // "Keep solving": a lane whose search is gone still holds its best plan
  const best: (Plan | null)[] = keep ? raceBest : [null, null]
  raceBest = best
  return (k: number) => (u: SolveUpdate) => {
    last[k] = { ...u, plan: null }
    if (u.plan) best[k] = u.plan
    const proven = last.some(x => x?.proven) || best.some(p => p?.proven_optimal)
    const done = proven || last.every(x => x?.done)
    if (proven) for (const l of lanes) l.cancel()
    const pick = better(best[0], best[1])
    const plan = pick !== shown || done ? pick : null
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
      engine_busy: false,
      gone: last.every(x => x?.gone) || undefined,
    })
  }
}

let raceIds: [number, number] | null = null

/** Start a solve; `onUpdate` gets the streamed updates. Returns the solve id. */
export function solveStream(s: Settings, onUpdate: OnUpdate): number {
  const id = ++seq
  if (s.integer && (s.engine ?? 'auto') === 'auto') {
    const h = race(onUpdate)
    lanes[0].start(id, { ...s, engine: 'microlp' }, h(0))
    lanes[1].start(id, { ...s, engine: 'highs' }, h(1))
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
      h(a ? 1 : 0)({ plan: null, done: true })
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
  const kind = liquid ? 'Pipe' : 'Belt'
  if (!tiers.length) return '—'
  const i = tiers.findIndex(t => rate <= t + 1e-9)
  if (i >= 0) return `${kind} Mk.${i + 1}`
  const max = tiers[tiers.length - 1]
  return `${Math.ceil(rate / max)}× ${kind} Mk.${tiers.length}`
}

export const fmt = (x: number, d = 2) =>
  Math.abs(x - Math.round(x)) < 1e-6 ? String(Math.round(x)) : x.toFixed(d).replace(/0+$/, '').replace(/\.$/, '')
