// Typed wrapper around the Rust/WASM planner (src/wasm_api.rs).
import init, { game_data_json, solve_json } from './wasm-pkg/satisfactory_calculator.js'

export type Game = 'satisfactory' | 'dsp' | 'factorio'
export const GAMES: { id: Game; name: string }[] = [
  { id: 'satisfactory', name: 'Satisfactory' },
  { id: 'dsp', name: '戴森球计划' },
  { id: 'factorio', name: 'Factorio' },
]
export interface Item { id: string; name: string; name_zh?: string | null; liquid: boolean; sink: number; raw: boolean }
export interface Recipe {
  id: string; name: string; name_zh?: string | null; alt: boolean; building: string; producers?: string[]; time: number
  in: [string, number][]; out: [string, number][]; power?: number
}
export interface Building {
  id: string; name: string; name_zh?: string | null; power: number; power_exp: number; somersloop_slots: number
  speed?: number; modules?: number
}
export interface Effect { id: string; name: string; name_zh?: string | null; kind: string; speed: number; productivity: number; consumption: number; sprays?: number | null; item: string }
export interface Resource { item: string; limit: number | null; extractor: string; extractor_rate: number; extractor_power: number }
export interface GameData {
  game: Game; effects?: Effect[]
  version: string; source: string; items: Item[]; recipes: Recipe[]; buildings: Building[]
  resources: Resource[]; belts: number[]; pipes: number[]
}

export type Objective = 'resources' | 'buildings' | 'power' | 'custom'
export interface CostWeights {
  resources: number; buildings: number; power: number; shards: number; surplus: number
  resource_weights: Record<string, number>; building_weights: Record<string, number>
}
/** Same numbers as Objective::preset in src/planner/mod.rs. */
export function preset(o: Objective): CostWeights {
  const [resources, buildings, power, shards] =
    o === 'buildings' ? [0.001, 1, 0, 0.01] : o === 'power' ? [0.001, 1, 1, 0.01] : [1, 0.01, 0, 0.01]
  return { resources, buildings, power, shards, surplus: 0, resource_weights: {}, building_weights: {} }
}
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
}

export interface RateLine { item: string; name: string; rate: number }
export interface PlanRecipe {
  id: string; mode: string; mode_label: string; name: string; alt: boolean; building: string; machines: number; machines_ceil: number
  throughput: number; shards: number; groups: MachineGroup[]
  power: number; inputs: RateLine[]; outputs: RateLine[]; somersloops: number
}
export interface MachineGroup { count: number; clock_percent: number; shards: number }
export interface ItemValue { item: string; name: string; value: number }
export interface Flow { from: string; to: string; item: string; name: string; rate: number }
export interface Plan {
  status: 'empty' | 'optimal' | 'infeasible' | 'unbounded' | 'error'
  objective: number; targets: RateLine[]; recipes: PlanRecipe[]; raw: RateLine[]; supplied: RateLine[]
  surplus: RateLine[]; production_power: number; extraction_power: number; buildings: number
  shards: number; somersloops: number; proven_optimal: boolean; gap: number | null; lp_bound: number; solve_ms: number
  item_values: ItemValue[]; flows: Flow[]; warnings: string[]
}

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
})

let wasm: Promise<unknown> | null = null
const cache = new Map<Game, GameData>()
export async function loadSolver(game: Game): Promise<GameData> {
  wasm ??= init()
  await wasm
  if (!cache.has(game)) {
    const d = JSON.parse(game_data_json(game)) as GameData
    d.game ??= game // Satisfactory's JSON predates the field
    cache.set(game, d)
  }
  return cache.get(game)!
}

/** Display name: Chinese for DSP when available. */
export function dn(x: { name: string; name_zh?: string | null } | undefined, game: Game, fallback = ''): string {
  if (!x) return fallback
  return game === 'dsp' && x.name_zh ? x.name_zh : x.name
}

export function solve(s: Settings): Plan {
  return JSON.parse(solve_json(JSON.stringify(s))) as Plan
}

// Solves in a Web Worker; a newer request supersedes older ones.
let worker: Worker | null = null
let seq = 0
const pending = new Map<number, (p: Plan) => void>()
export function solveAsync(s: Settings): Promise<Plan> {
  if (!worker) {
    worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' })
    worker.onmessage = (e: MessageEvent<{ id: number; plan: string }>) => {
      const done = pending.get(e.data.id)
      pending.delete(e.data.id)
      done?.(JSON.parse(e.data.plan) as Plan)
    }
  }
  const id = ++seq
  return new Promise(resolve => {
    pending.set(id, resolve)
    worker!.postMessage({ id, settings: JSON.stringify(s) })
  })
}
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
