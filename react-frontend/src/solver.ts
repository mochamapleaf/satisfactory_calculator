// Typed wrapper around the Rust/WASM planner (src/wasm_api.rs).
import init, { game_data_json, solve_json } from './wasm-pkg/satisfactory_calculator.js'

export interface Item { id: string; name: string; liquid: boolean; sink: number; raw: boolean }
export interface Recipe {
  id: string; name: string; alt: boolean; building: string; time: number
  in: [string, number][]; out: [string, number][]; power?: number
}
export interface Building { id: string; name: string; power: number; power_exp: number; somersloop_slots: number }
export interface Resource { item: string; limit: number | null; extractor: string; extractor_rate: number; extractor_power: number }
export interface GameData {
  version: string; source: string; items: Item[]; recipes: Recipe[]; buildings: Building[]
  resources: Resource[]; belts: number[]; pipes: number[]
}

export type Objective = 'resources' | 'buildings' | 'power'
export interface ItemRate { item: string; rate: number }
export interface Settings {
  targets: ItemRate[]
  supplies: ItemRate[]
  objective: Objective
  allow_all_alternates: boolean
  enabled_alternates: string[]
  disabled_recipes: string[]
  resource_limits: Record<string, number | null>
  ignore_limits: boolean
  allow_ore_conversion: boolean
  clock_percent: number
  somersloops: Record<string, number>
}

export interface RateLine { item: string; name: string; rate: number }
export interface PlanRecipe {
  id: string; name: string; alt: boolean; building: string; machines: number; machines_ceil: number
  power: number; inputs: RateLine[]; outputs: RateLine[]; somersloops: number
}
export interface Flow { from: string; to: string; item: string; name: string; rate: number }
export interface Plan {
  status: 'empty' | 'optimal' | 'infeasible' | 'unbounded' | 'error'
  objective: number; targets: RateLine[]; recipes: PlanRecipe[]; raw: RateLine[]; supplied: RateLine[]
  surplus: RateLine[]; production_power: number; extraction_power: number; buildings: number
  flows: Flow[]; warnings: string[]
}

export const defaultSettings = (): Settings => ({
  targets: [{ item: 'Desc_IronPlateReinforced_C', rate: 10 }],
  supplies: [],
  objective: 'resources',
  allow_all_alternates: false,
  enabled_alternates: [],
  disabled_recipes: [],
  resource_limits: {},
  ignore_limits: false,
  allow_ore_conversion: false,
  clock_percent: 100,
  somersloops: {},
})

let ready: Promise<GameData> | null = null
export function loadSolver(): Promise<GameData> {
  ready ??= init().then(() => JSON.parse(game_data_json()) as GameData)
  return ready
}

export function solve(s: Settings): Plan {
  return JSON.parse(solve_json(JSON.stringify(s))) as Plan
}

/** Smallest belt / pipe tier that carries `rate`, e.g. "Mk.3" or "2× Mk.6". */
export function carrier(rate: number, liquid: boolean, data: GameData): string {
  const tiers = liquid ? data.pipes : data.belts
  const kind = liquid ? 'Pipe' : 'Belt'
  const i = tiers.findIndex(t => rate <= t + 1e-9)
  if (i >= 0) return `${kind} Mk.${i + 1}`
  const max = tiers[tiers.length - 1]
  return `${Math.ceil(rate / max)}× ${kind} Mk.${tiers.length}`
}

export const fmt = (x: number, d = 2) =>
  Math.abs(x - Math.round(x)) < 1e-6 ? String(Math.round(x)) : x.toFixed(d).replace(/0+$/, '').replace(/\.$/, '')
