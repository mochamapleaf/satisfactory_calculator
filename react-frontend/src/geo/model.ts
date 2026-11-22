// Siting MILP for the Satisfactory map view: a line-by-line port of tools/geo_solve.py (G5/R1, report
// docs/research/satisfactory-geo/00-最终报告.md §7.1–7.2, §8.5), plus geo_breakeven.haul_costs and the hub rule
// of geo_sites.py. The solver is passed in (LpSolve), so the browser worker and the node test share this file.
//
// Model (machines continuous): x[r,s] machines of recipe r at site s; e[k,s] raw extraction (<= site supply at the
// tier); f[k,a] flow on directed belt/pipe arcs, b[a] / bp[a] integer belts / Mk.2 pipes; rail only between the K
// candidate hubs, one private double track per hub pair: h[s] hub open, u[l] track, z[l,d] a train runs, c / cf cars,
// g[k,l,d] rail flow. car: Σ g/stack ≤ 32/RtD·c; platform: Σ g ≤ feed·(1 − dock/RtD)·c; c + cf ≤ C̄·z; z ≤ u ≤ h;
// Σ h ≤ p. Balance per site and item ≥ demand (free surplus). Objective: one-time build cost in raw-ore equivalents.
// Departures from the report's model are the ones geo_solve.py lists (all lean against rail).

export const W_MACH = 100.0
export const W_RAW = 1.0
export const W_WORK_HUB = 2000.0
export const UNITS_PER_KM = 500
type Bom = Record<string, number>
const PIPE_BOM: Bom[] = [{ Desc_CopperSheet_C: 1 }, { Desc_CopperSheet_C: 2, Desc_Plastic_C: 1 }]
const BUILDING_COST: Record<string, Bom> = {
  Desc_ConstructorMk1_C: { Desc_IronPlateReinforced_C: 2, Desc_Cable_C: 8 },
  Desc_SmelterMk1_C: { Desc_IronRod_C: 5, Desc_Wire_C: 8 },
  Desc_AssemblerMk1_C: { Desc_IronPlateReinforced_C: 8, Desc_Rotor_C: 4, Desc_Cable_C: 10 },
  Desc_FoundryMk1_C: { Desc_ModularFrame_C: 10, Desc_Rotor_C: 10, Desc_Cement_C: 20 },
  Desc_ManufacturerMk1_C: { Desc_Motor_C: 10, Desc_ModularFrame_C: 20, Desc_Plastic_C: 50, Desc_Cable_C: 50 },
  Desc_OilRefinery_C: { Desc_Motor_C: 10, Desc_SteelPlateReinforced_C: 10, Desc_SteelPipe_C: 30, Desc_CopperSheet_C: 20 },
  Desc_Packager_C: { Desc_SteelPlate_C: 20, Desc_Rubber_C: 10, Desc_Plastic_C: 10 },
  Desc_Blender_C: { Desc_Computer_C: 10, Desc_ModularFrameHeavy_C: 10, Desc_Motor_C: 20, Desc_AluminumCasing_C: 50 },
}
const EXCLUDED_BUILDINGS = new Set(['Desc_Converter_C', 'Desc_QuantumEncoder_C', 'Desc_HadronCollider_C'])
export const BELT_MATERIAL = ['Desc_IronPlate_C', 'Desc_IronPlateReinforced_C', 'Desc_SteelPlate_C', 'Desc_SteelPlateReinforced_C', 'Desc_AluminumPlate_C']
const LOCO: Bom = { Desc_ModularFrame_C: 5, Desc_Motor_C: 10, Desc_SteelPipe_C: 20, Desc_Rubber_C: 20, Desc_Wire_C: 50 }
const CAR: Bom = { Desc_ModularFrame_C: 5, Desc_SteelPipe_C: 10, Desc_SteelPlate_C: 10 }
const STATION: Bom = { Desc_SteelPlateReinforced_C: 10, Desc_Plastic_C: 50, Desc_Cement_C: 50, Desc_Wire_C: 200 }
const PLATFORM: Bom = { Desc_Motor_C: 5, Desc_SteelPlateReinforced_C: 10, Desc_Plastic_C: 25, Desc_Cement_C: 50, Desc_Wire_C: 100 }
// stack sizes: SatisfactoryTools data1.0.json @2192bd15 (geo_solve.py STACK_TABLE); default 100
const STACK_TABLE: Record<string, number> = {
  Desc_IronPlate_C: 200, Desc_IronRod_C: 200, Desc_IronScrew_C: 500, Desc_Wire_C: 500, Desc_Cable_C: 200, Desc_Cement_C: 500,
  Desc_IronPlateReinforced_C: 100, Desc_ModularFrame_C: 50, Desc_ModularFrameHeavy_C: 50, Desc_SteelPipe_C: 200,
  Desc_SteelPlate_C: 200, Desc_SteelPlateReinforced_C: 100, Desc_IronIngot_C: 100, Desc_SteelIngot_C: 100,
  Desc_CopperIngot_C: 100, Desc_CopperSheet_C: 200, Desc_Rotor_C: 100, Desc_Stator_C: 100, Desc_Motor_C: 50,
  Desc_Plastic_C: 200, Desc_Rubber_C: 200, Desc_QuartzCrystal_C: 200, Desc_Silica_C: 200, Desc_GoldIngot_C: 100,
  Desc_HighSpeedWire_C: 500, Desc_CircuitBoard_C: 200, Desc_Computer_C: 50,
}
export const stackOf = (k: string) => STACK_TABLE[k] ?? 100

// ---------- data ----------
export interface GameItem { id: string; liquid: boolean; name?: string }
export interface GameRecipe { id: string; alt: boolean; building: string; in: [string, number][]; out: [string, number][]; name?: string }
export interface GameLite { items: GameItem[]; recipes: GameRecipe[]; resources: { item: string }[] }
export interface GeoNode { id: string; item: string; purity: 'impure' | 'normal' | 'pure'; type: string; x: number; y: number; max_rate: number }
export interface Site { id: string; kind: string; cell?: number[]; x: number; y: number; feed_dist_mean_m: number; nodes: string[]; supply?: Record<string, number> }
export interface SitesDoc {
  params: { detour_factor: number; [k: string]: unknown }
  sites: Site[]; arcs: { a: string; b: string; length_m: number }[]; hubs: { site: string; reason: string }[]
}
// transport.json leaf: {value} or {v1_0: {value}, v1_2: {value}} (1.0 to match game-data.json)
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Json = any
export const tv = (node: Json) => ('value' in node ? node.value : node.v1_0.value)

export const cmp = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0)

export class Data {
  game: GameLite; sitesDoc: SitesDoc; nodes: Map<string, GeoNode>; tr: Json
  items: Map<string, GameItem>; raw: Set<string>; sites: Site[]; sid: string[]; site: Map<string, Site>
  hubs: string[]; base: string; arcs: [string, string, number][]; detour: number; recipes: GameRecipe[]
  constructor(game: GameLite, sitesDoc: SitesDoc, nodes: GeoNode[], transport: Json, hubs?: string[]) {
    this.game = game
    this.sitesDoc = sitesDoc
    this.nodes = new Map(nodes.map(n => [n.id, n]))
    this.tr = transport
    this.items = new Map(game.items.map(i => [i.id, i]))
    this.raw = new Set(game.resources.map(r => r.item))
    this.sites = sitesDoc.sites
    this.sid = this.sites.map(s => s.id)
    this.site = new Map(this.sites.map(s => [s.id, s]))
    this.hubs = (hubs ?? sitesDoc.hubs.map(h => h.site)).slice().sort(cmp)
    this.base = sitesDoc.hubs.find(h => h.reason === 'base')!.site
    this.arcs = []
    for (const a of sitesDoc.arcs) {
      this.arcs.push([a.a, a.b, a.length_m])
      this.arcs.push([a.b, a.a, a.length_m])
    }
    this.detour = sitesDoc.params.detour_factor
    this.recipes = game.recipes
  }
  liquid(k: string) { return !!this.items.get(k)?.liquid }
  stack(k: string) { return stackOf(k) }
  supply(s: string, tier: number, beltCap: number, pipeCap: number): Map<string, number> {
    const out = new Map<string, number>()
    for (const nid of this.site.get(s)!.nodes) {
      const n = this.nodes.get(nid)!
      const r = nodeRate(n, tier, beltCap, pipeCap)
      if (r > 0) out.set(n.item, (out.get(n.item) ?? 0) + r)
    }
    return out
  }
}

/** Net items/min of one machine at 100 % (game-data.json rates are already per minute). */
export function perMin(r: GameRecipe): Map<string, number> {
  const a = new Map<string, number>()
  for (const [k, q] of r.in) a.set(k, (a.get(k) ?? 0) - q)
  for (const [k, q] of r.out) a.set(k, (a.get(k) ?? 0) + q)
  return a
}

const MK1_BASE = { impure: 30, normal: 60, pure: 120 }
const OIL_BASE = { impure: 60, normal: 120, pure: 240 }
const WELL_BASE = { impure: 30, normal: 60, pure: 120 }
const MINER_TIERS: [number, number][] = [[8, 4.0], [4, 2.0], [0, 1.0]]
const OIL_UNLOCK = 5, WELL_UNLOCK = 8, SHARD_TIER = 3

/** Items (m³) per minute one node delivers at `tier`, capped by its one output belt / pipe. */
export function nodeRate(n: Pick<GeoNode, 'type' | 'item' | 'purity'>, tier: number, beltCap: number, pipeCap: number): number {
  const clock = tier >= SHARD_TIER ? 2.5 : 1.0
  if (n.type === 'node' && n.item === 'Desc_LiquidOil_C') return tier >= OIL_UNLOCK ? Math.min(OIL_BASE[n.purity] * clock, pipeCap) : 0
  if (n.type === 'node') {
    const mk = MINER_TIERS.find(([t]) => tier >= t)![1]
    return Math.min(MK1_BASE[n.purity] * mk * clock, beltCap)
  }
  if (n.type === 'well') return tier >= WELL_UNLOCK ? Math.min(WELL_BASE[n.purity] * clock, pipeCap) : 0
  return 0
}

/** Round trip in minutes: travel both ways plus, at each end, the dock lock and the accel/brake loss. */
export const rtdMin = (L: number, v: number, dock: number, acc: number) => (2 * L / v + 2 * (dock + acc)) / 60
/** Items/min one car can be loaded with at a platform fed at `feed` (ports locked `dock` s per stop). */
export const platformCap = (feed: number, rtd: number, dock: number) => feed * (1 - (dock / 60) / rtd)

// ---------- LP container and solver interface ----------
export class Model {
  c: number[] = []; lb: number[] = []; ub: number[] = []; integ: boolean[] = []; names: string[] = []
  rows: Map<number, number>[] = []; lo: number[] = []; hi: number[] = []; rnames: string[] = []
  var(name: string, cost = 0, lb = 0, ub = Infinity, integer = false) {
    this.c.push(cost); this.lb.push(lb); this.ub.push(ub); this.integ.push(integer); this.names.push(name)
    return this.c.length - 1
  }
  row(name: string, coefs: Map<number, number>, lo = -Infinity, hi = Infinity) {
    this.rows.push(coefs); this.lo.push(lo); this.hi.push(hi); this.rnames.push(name)
    return this.rows.length - 1
  }
}
export interface LpResult { x: Float64Array | null; obj: number | null; bound: number | null; gap: number; status: string; time: number }
export interface LpOptions { timeLimit?: number; gap?: number; warm?: Float64Array | null; progress?: (info: { bound: number | null; best: number | null; nodes: number }) => void }
export type LpSolve = (m: Model, opts: LpOptions) => LpResult

// ---------- OE, recipe closure, model ----------
const recipeKeys = (r: GameRecipe) => [...r.in, ...r.out].map(([k]) => k)

/** Min raw items (water 0) for one unit of each item, 1.0 base recipes. One LP per item. */
export function oreEquivalents(d: Data, items: string[], solve: LpSolve): Record<string, number> {
  const recs = d.recipes.filter(r => !r.alt && !EXCLUDED_BUILDINGS.has(r.building))
  const allk = [...new Set([...recs.flatMap(recipeKeys), ...d.raw])].sort(cmp)
  const kix = new Map(allk.map((k, i) => [k, i]))
  const out: Record<string, number> = {}
  for (const item of items) {
    const m = new Model()
    const rows = allk.map(() => new Map<number, number>())
    recs.forEach(r => {
      const j = m.var(r.id, 0)
      for (const [k, q] of perMin(r)) rows[kix.get(k)!].set(j, q)
    })
    for (const k of [...d.raw].sort(cmp)) {
      const j = m.var(k, k === 'Desc_Water_C' ? 0 : 1)
      rows[kix.get(k)!].set(j, 1)
    }
    allk.forEach((k, i) => m.row(k, rows[i], k === item ? 1 : 0))
    const res = solve(m, {})
    if (!kix.has(item) || !res.x || !(res.obj! > 0)) throw new Error(`OE of ${item}: ${res.status}`)
    out[item] = res.obj!
  }
  return out
}

export function recipeClosure(d: Data, targets: Record<string, number>): GameRecipe[] {
  const need = Object.keys(targets), seen = new Set<string>(), chosen: GameRecipe[] = []
  while (need.length) {
    const k = need.pop()!
    if (seen.has(k)) continue
    seen.add(k)
    if (d.raw.has(k)) continue
    const prod = d.recipes.filter(r => !r.alt && r.out.some(([o]) => o === k) && !r.out.some(([o]) => d.raw.has(o)))
    const good0 = prod.filter(r => !EXCLUDED_BUILDINGS.has(r.building))
    const good = good0.length ? good0 : prod
    if (!good.length) throw new Error(`no recipe makes ${k}`)
    for (const r of good) {
      if (!chosen.includes(r)) {
        chosen.push(r)
        need.push(...r.in.map(([i]) => i))
      }
    }
  }
  return chosen.sort((a, b) => cmp(a.id, b.id))
}

export interface BuildOptions {
  tier?: number; rail?: boolean; p?: number | null; fmult?: number; base?: string; accS?: number; waterAnywhere?: boolean
  beltTier?: number | null; oe?: Record<string, number> | null; beltMult?: number; workKm?: number
  machinesAt?: Set<string> | null; cbarMult?: number
  /** Web addition: recipes to use instead of the base-recipe closure (the current plan's recipes). */
  recipes?: GameRecipe[] | null
  /** Web addition: items the plan takes as supplied inputs, available free at the base site up to this rate (/min). */
  supplied?: Record<string, number> | null
}
export interface Line { a: string; b: string; length_m: number; rtd_min: number; cbar: number; cbar_fluid: number }
export interface Meta {
  base: string; targets: Record<string, number>; tier: number; belt_tier: number; belt_cap: number; belt_oe_km: number
  pipe_cap: number; pipe_oe_km: number; mach_oe: Record<string, number>; track_oe_km: number; loco_oe: number
  car_oe: number; platform_oe: number; station_oe: number; F: number; fmult: number; rail: boolean; p: number | null
  acc_s: number; work_km: number; cbar_mult: number; recipes: string[]; items: string[]; oe: Record<string, number>
  lines: Line[]; water_anywhere: boolean; slots: number; feed: number; dock_s: number; fluid_car: number; v_mps: number
  supply: Map<string, number>; check_obj?: number; cbar_binding?: string[]
}
type K2 = string // `${a}|${b}`
type K3 = string // `${k}|${a}|${b}`
export interface Index {
  x: Map<K2, number>; e: Map<K2, number>; f: Map<K3, number>; b: Map<K2, number>; bp: Map<K2, number>; h: Map<string, number>
  u: Map<K2, number>; z: Map<K2, number>; c: Map<K2, number>; cf: Map<K2, number>; g: Map<K3, number>
  A: Map<string, Map<string, number>>; solids: string[]; fluids: string[]
  /** supplied input k taken at the base (web addition; empty in the Python model) */
  sup: Map<string, number>
}
const k2 = (a: string, b: string) => `${a}|${b}`
const k3 = (k: string, a: string, b: string) => `${k}|${a}|${b}`

export function materials(): string[] {
  return [...new Set([...BELT_MATERIAL, ...Object.keys(LOCO), ...Object.keys(CAR), ...Object.keys(STATION), ...Object.keys(PLATFORM),
    'Desc_SteelPipe_C', 'Desc_SteelPlate_C', ...PIPE_BOM.flatMap(Object.keys), ...Object.values(BUILDING_COST).flatMap(Object.keys)])].sort(cmp)
}

export function build(d: Data, targets: Record<string, number>, solve: LpSolve, o: BuildOptions = {}): { m: Model; idx: Index; meta: Meta } {
  const tier = o.tier ?? 8, p = o.p ?? null, fmult = o.fmult ?? 1, base = o.base ?? d.base, accS = o.accS ?? 30
  const waterAnywhere = o.waterAnywhere ?? true, beltMult = o.beltMult ?? 1, workKm = o.workKm ?? 0, cbarMult = o.cbarMult ?? 2
  const recs = o.recipes ?? recipeClosure(d, targets)
  const A = new Map(recs.map(r => [r.id, perMin(r)]))
  const items = [...new Set([...[...A.values()].flatMap(a => [...a.keys()]), ...Object.keys(targets)])].sort(cmp)
  const solids = items.filter(k => !d.liquid(k)), fluids = items.filter(k => d.liquid(k))
  const tr = d.tr
  const beltCaps: number[] = tv(tr.belts.tiers), beltUnlock: number[] = tv(tr.belts.unlock_tier)
  const oe = o.oe ?? oreEquivalents(d, materials(), solve)
  let beltTier = o.beltTier ?? null
  if (beltTier === null) { // cheapest OE per unit capacity among unlocked tiers (Mk.6 not costed)
    const cand = [0, 1, 2, 3, 4].filter(i => beltUnlock[i] <= tier)
    const key = (i: number) => UNITS_PER_KM * oe[BELT_MATERIAL[i]] / beltCaps[i]
    beltTier = cand.reduce((best, i) => (key(i) < key(best) || (key(i) === key(best) && i > best) ? i : best))
  }
  const cap = beltCaps[beltTier]
  const beltOeKm = beltMult * UNITS_PER_KM * oe[BELT_MATERIAL[beltTier]] + workKm
  const pipeIx = tv(tr.pipes.unlock_tier)[1] <= tier ? 1 : 0
  const pipeCap: number = tv(tr.pipes.tiers)[pipeIx]
  const rail = (o.rail ?? true) && tv(tr.train.unlock_tier) <= tier
  const km83 = tv(tr.rail.track_cost_per_km)
  const trackOeKm = tv(tr.rail.double_track_factor) * (km83.steel_pipe * oe.Desc_SteelPipe_C + km83.steel_beam * oe.Desc_SteelPlate_C + workKm)
  const costOf = (bom: Bom) => Object.entries(bom).reduce((s, [k, q]) => s + q * oe[k], 0)
  const locoOe = costOf(LOCO), carOe = costOf(CAR), platOe = costOf(PLATFORM), stationOe = costOf(STATION)
  const pipeOeKm = UNITS_PER_KM * costOf(PIPE_BOM[pipeIx]) + workKm
  const machOe: Record<string, number> = {}
  for (const r of recs) machOe[r.id] = BUILDING_COST[r.building] ? costOf(BUILDING_COST[r.building]) : W_MACH
  const F = fmult * (2 * stationOe + W_WORK_HUB)
  const v: number = tv(tr.train.speed_mps), dock: number = tv(tr.train.platform_dock_lock_s)
  const slots: number = tv(tr.train.car_slots), fluidCar: number = tv(tr.train.car_fluid_m3)
  const feed = 2 * cap
  const m = new Model()
  const x = new Map<K2, number>()
  for (const s of d.sid) for (const r of recs) x.set(k2(r.id, s), m.var(`x|${r.id}|${s}`, machOe[r.id], 0, !o.machinesAt || o.machinesAt.has(s) ? Infinity : 0))
  const e = new Map<K2, number>(), supply = new Map<string, number>()
  for (const s of d.sid) {
    const st = d.site.get(s)!
    const siteSup = d.supply(s, tier, cap, pipeCap)
    for (const k of items) {
      if (!d.raw.has(k)) continue
      let sup = siteSup.get(k) ?? 0
      if (k === 'Desc_Water_C' && waterAnywhere) sup = Infinity
      if (sup <= 0) continue
      const fdKm = st.feed_dist_mean_m / 1000
      const feedc = fdKm * (d.liquid(k) ? pipeOeKm / pipeCap : beltOeKm / cap)
      e.set(k2(k, s), m.var(`e|${k}|${s}`, W_RAW + feedc, 0, sup))
      supply.set(k2(k, s), sup)
    }
  }
  const supVar = new Map<string, number>()
  for (const [k, q] of Object.entries(o.supplied ?? {}).sort((a_, b_) => cmp(a_[0], b_[0]))) {
    if (q > 0 && items.includes(k)) supVar.set(k, m.var(`s|${k}|${base}`, 0, 0, q))
  }
  const f = new Map<K3, number>(), b = new Map<K2, number>(), bp = new Map<K2, number>()
  for (const [sa, sb, L] of d.arcs) {
    const km = L / 1000
    if (solids.length) b.set(k2(sa, sb), m.var(`b|${sa}|${sb}`, km * beltOeKm, 0, 50, true))
    if (fluids.length) bp.set(k2(sa, sb), m.var(`bp|${sa}|${sb}`, km * pipeOeKm, 0, 50, true))
    for (const k of items) f.set(k3(k, sa, sb), m.var(`f|${k}|${sa}|${sb}`))
  }
  const hvar = new Map<string, number>(), u = new Map<K2, number>(), z = new Map<K2, number>()
  const cs = new Map<K2, number>(), cfl = new Map<K2, number>(), g = new Map<K3, number>(), lines: Line[] = []
  const M = (pairs: [number, number][]) => new Map(pairs)
  if (rail) {
    const gross = aggregateGross(d, recs, A, targets, items, solve, o.supplied ?? null)
    let stacksUb = 0, fluidUb = 0, itemsUb = 0
    for (const [k, q] of gross) {
      if (d.liquid(k)) fluidUb += q
      else { stacksUb += q / d.stack(k); itemsUb += q }
    }
    for (const s of d.hubs) hvar.set(s, m.var(`h|${s}`, F, 0, 1, true))
    d.hubs.forEach((sa, i) => {
      for (const sb of d.hubs.slice(i + 1)) {
        const A_ = d.site.get(sa)!, B_ = d.site.get(sb)!
        const L = Math.hypot(A_.x - B_.x, A_.y - B_.y) * d.detour
        const rtd = rtdMin(L, v, dock, accS)
        const cbar = Math.ceil(cbarMult * Math.max(stacksUb * rtd / slots, itemsUb / platformCap(feed, rtd, dock))) + 1
        const cbarF = fluids.length ? Math.ceil(cbarMult * fluidUb * rtd / fluidCar) + 1 : 0
        lines.push({ a: sa, b: sb, length_m: round(L, 1), rtd_min: round(rtd, 4), cbar, cbar_fluid: cbarF })
        const l = k2(sa, sb)
        u.set(l, m.var(`u|${sa}|${sb}`, L / 1000 * trackOeKm, 0, 1, true))
        m.row(`uha|${sa}|${sb}`, M([[u.get(l)!, 1], [hvar.get(sa)!, -1]]), -Infinity, 0)
        m.row(`uhb|${sa}|${sb}`, M([[u.get(l)!, 1], [hvar.get(sb)!, -1]]), -Infinity, 0)
        for (const [o_, t] of [[sa, sb], [sb, sa]]) {
          const key = k2(o_, t)
          z.set(key, m.var(`z|${o_}|${t}`, locoOe, 0, 1, true))
          m.row(`zu|${o_}|${t}`, M([[z.get(key)!, 1], [u.get(l)!, -1]]), -Infinity, 0)
          const link = M([[z.get(key)!, -(cbar + cbarF)]])
          if (solids.length) { cs.set(key, m.var(`c|${o_}|${t}`, carOe + 2 * platOe, 0, cbar, true)); link.set(cs.get(key)!, 1) }
          if (fluids.length) { cfl.set(key, m.var(`cf|${o_}|${t}`, carOe + 2 * platOe, 0, cbarF, true)); link.set(cfl.get(key)!, 1) }
          m.row(`cz|${o_}|${t}`, link, -Infinity, 0)
          for (const k of items) g.set(k3(k, o_, t), m.var(`g|${k}|${o_}|${t}`))
          if (solids.length) {
            m.row(`car|${o_}|${t}`, M([...solids.map(k => [g.get(k3(k, o_, t))!, 1 / d.stack(k)] as [number, number]), [cs.get(key)!, -slots / rtd]]), -Infinity, 0)
            m.row(`plat|${o_}|${t}`, M([...solids.map(k => [g.get(k3(k, o_, t))!, 1] as [number, number]), [cs.get(key)!, -platformCap(feed, rtd, dock)]]), -Infinity, 0)
          }
          if (fluids.length) {
            m.row(`fcar|${o_}|${t}`, M([...fluids.map(k => [g.get(k3(k, o_, t))!, 1] as [number, number]), [cfl.get(key)!, -fluidCar / rtd]]), -Infinity, 0)
            m.row(`fplat|${o_}|${t}`, M([...fluids.map(k => [g.get(k3(k, o_, t))!, 1] as [number, number]), [cfl.get(key)!, -platformCap(1200, rtd, dock)]]), -Infinity, 0)
          }
        }
      }
    })
    if (p !== null) m.row('p', M(d.hubs.map(s => [hvar.get(s)!, 1])), -Infinity, p)
  }
  for (const [sa, sb] of d.arcs) {
    if (solids.length) m.row(`belt|${sa}|${sb}`, M([...solids.map(k => [f.get(k3(k, sa, sb))!, 1] as [number, number]), [b.get(k2(sa, sb))!, -cap]]), -Infinity, 0)
    if (fluids.length) m.row(`pipe|${sa}|${sb}`, M([...fluids.map(k => [f.get(k3(k, sa, sb))!, 1] as [number, number]), [bp.get(k2(sa, sb))!, -pipeCap]]), -Infinity, 0)
  }
  // balance
  const inc = new Map<string, number[]>(), outg = new Map<string, number[]>()
  const push = (mp: Map<string, number[]>, key: string, j: number) => { const l = mp.get(key); if (l) l.push(j); else mp.set(key, [j]) }
  for (const mp of [f, g]) {
    for (const [key, j] of mp) {
      const [k, sa, sb] = key.split('|')
      push(outg, k2(k, sa), j); push(inc, k2(k, sb), j)
    }
  }
  for (const s of d.sid) {
    for (const k of items) {
      const co = new Map<number, number>()
      const add = (j: number, q: number) => co.set(j, (co.get(j) ?? 0) + q)
      for (const r of recs) {
        const q = A.get(r.id)!.get(k) ?? 0
        if (q) add(x.get(k2(r.id, s))!, q)
      }
      const ej = e.get(k2(k, s))
      if (ej !== undefined) add(ej, 1)
      if (s === base && supVar.has(k)) add(supVar.get(k)!, 1)
      for (const j of inc.get(k2(k, s)) ?? []) add(j, 1)
      for (const j of outg.get(k2(k, s)) ?? []) add(j, -1)
      const dem = s === base ? targets[k] ?? 0 : 0
      if (!co.size && dem === 0) continue
      m.row(`bal|${k}|${s}`, co, dem)
    }
  }
  const meta: Meta = {
    base, targets, tier, belt_tier: beltTier + 1, belt_cap: cap, belt_oe_km: beltOeKm, pipe_cap: pipeCap, pipe_oe_km: pipeOeKm,
    mach_oe: machOe, track_oe_km: trackOeKm, loco_oe: locoOe, car_oe: carOe, platform_oe: platOe, station_oe: stationOe, F, fmult,
    rail, p, acc_s: accS, work_km: workKm, cbar_mult: cbarMult, recipes: recs.map(r => r.id), items, oe,
    lines, water_anywhere: waterAnywhere, slots, feed, dock_s: dock, fluid_car: fluidCar, v_mps: v, supply,
  }
  const idx: Index = { x, e, f, b, bp, h: hvar, u, z, c: cs, cf: cfl, g, A, solids, fluids, sup: supVar }
  return { m, idx, meta }
}

/** Location-free LP: min machines + raw; gross production per item (upper bound for flows). */
export function aggregateGross(d: Data, recs: GameRecipe[], A: Map<string, Map<string, number>>, targets: Record<string, number>,
  items: string[], solve: LpSolve, supplied: Record<string, number> | null = null): Map<string, number> {
  const m = new Model()
  const x = new Map(recs.map(r => [r.id, m.var(r.id, W_MACH)]))
  const e = new Map<string, number>()
  for (const k of items) if (d.raw.has(k)) e.set(k, m.var(k, W_RAW))
  for (const k of items) {
    const co = new Map<number, number>()
    for (const [r, j] of x) { const q = A.get(r)!.get(k); if (q) co.set(j, q) }
    const ej = e.get(k)
    if (ej !== undefined) co.set(ej, 1)
    const sq = supplied?.[k] ?? 0
    if (sq > 0) co.set(m.var(`s|${k}`, 0, 0, sq), 1)
    m.row(k, co, targets[k] ?? 0)
  }
  const res = solve(m, {})
  if (!res.x) throw new Error(`aggregate LP: ${res.status}`)
  const gross = new Map<string, number>()
  for (const [r, j] of x) for (const [k, q] of A.get(r)!) if (q > 0) gross.set(k, (gross.get(k) ?? 0) + q * res.x[j])
  for (const [k, j] of e) gross.set(k, (gross.get(k) ?? 0) + res.x[j])
  return gross
}

export class CheckError extends Error {
  info: unknown[]
  constructor(info: unknown[]) { super(JSON.stringify(info)); this.info = info }
}
const req = (cond: boolean, ...info: unknown[]) => { if (!cond) throw new CheckError(info) }

/** Independent feasibility check of a solution vector (geo_solve.check); throws CheckError on the first violation. */
export function check(d: Data, m: Model, idx: Index, meta: Meta, xs: ArrayLike<number>, p: number | null = null, tol = 1e-5) {
  const X = (j: number) => xs[j]
  for (let j = 0; j < m.c.length; j++) {
    req(xs[j] >= m.lb[j] - tol && xs[j] <= m.ub[j] + tol, m.names[j], xs[j])
    if (m.integ[j]) req(Math.abs(xs[j] - Math.round(xs[j])) <= 1e-6, 'integrality', m.names[j], xs[j])
  }
  const { A } = idx, base = meta.base, targets = meta.targets
  const net = new Map<string, number>()
  const add = (key: string, q: number) => net.set(key, (net.get(key) ?? 0) + q)
  for (const [key, j] of idx.x) { const [r, s] = key.split('|'); for (const [k, q] of A.get(r)!) add(k2(k, s), q * X(j)) }
  for (const [key, j] of idx.e) add(key, X(j))
  for (const [k, j] of idx.sup) add(k2(k, base), X(j))
  for (const mp of [idx.f, idx.g]) for (const [key, j] of mp) { const [k, sa, sb] = key.split('|'); add(k2(k, sa), -X(j)); add(k2(k, sb), X(j)) }
  for (const s of d.sid) for (const k of meta.items) {
    const need = s === base ? targets[k] ?? 0 : 0
    const nv = net.get(k2(k, s)) ?? 0
    req(nv >= need - tol * Math.max(1, need) - 1e-6, 'balance', k, s, nv, need)
  }
  for (const [k, q] of Object.entries(targets)) req((net.get(k2(k, base)) ?? 0) >= q - 1e-6, 'demand', k)
  for (const [key, j] of idx.e) {
    const [k, s] = key.split('|')
    const nSup = d.supply(s, meta.tier, meta.belt_cap, meta.pipe_cap).get(k) ?? 0
    const sup = k === 'Desc_Water_C' && meta.water_anywhere ? Infinity : nSup
    req(X(j) <= sup + tol, 'supply', k, s, X(j), sup)
  }
  const cap = meta.belt_cap
  for (const [a, j] of idx.b) {
    const [sa, sb] = a.split('|')
    const tot = idx.solids.reduce((t, k) => t + X(idx.f.get(k3(k, sa, sb))!), 0)
    req(tot <= cap * Math.round(X(j)) + 1e-4, 'belt cap', sa, sb, tot, X(j))
  }
  for (const [a, j] of idx.bp) {
    const [sa, sb] = a.split('|')
    const tot = idx.fluids.reduce((t, k) => t + X(idx.f.get(k3(k, sa, sb))!), 0)
    req(tot <= meta.pipe_cap * Math.round(X(j)) + 1e-4, 'pipe cap', sa, sb, tot, X(j))
  }
  // objective recomputed from physical quantities and the cost table (not from m.c)
  let obj = 0
  for (const [key, j] of idx.x) obj += meta.mach_oe[key.split('|')[0]] * X(j)
  const km = new Map(d.arcs.map(([sa, sb, L]) => [k2(sa, sb), L / 1000]))
  for (const [key, j] of idx.e) {
    const [k, s] = key.split('|')
    const per = d.liquid(k) ? meta.pipe_oe_km / meta.pipe_cap : meta.belt_oe_km / cap
    obj += X(j) * (W_RAW + d.site.get(s)!.feed_dist_mean_m / 1000 * per)
  }
  for (const [a, j] of idx.b) obj += Math.round(X(j)) * km.get(a)! * meta.belt_oe_km
  for (const [a, j] of idx.bp) obj += Math.round(X(j)) * km.get(a)! * meta.pipe_oe_km
  for (const j of idx.h.values()) if (X(j) > 0.5) obj += meta.F
  const lens = new Map(meta.lines.map(ln => [k2(ln.a, ln.b), ln.length_m]))
  for (const [l, j] of idx.u) if (X(j) > 0.5) obj += lens.get(l)! / 1000 * meta.track_oe_km
  for (const j of idx.z.values()) if (X(j) > 0.5) obj += meta.loco_oe
  for (const j of [...idx.c.values(), ...idx.cf.values()]) obj += Math.round(X(j)) * (meta.car_oe + 2 * meta.platform_oe)
  meta.check_obj = obj
  if (idx.h.size) {
    const opened = new Set([...idx.h].filter(([, j]) => X(j) > 0.5).map(([s]) => s))
    if (p !== null) req(opened.size <= p, 'p', opened.size, p)
    for (const [l, j] of idx.u) if (X(j) > 0.5) { const [sa, sb] = l.split('|'); req(opened.has(sa) && opened.has(sb), 'line without hubs', sa, sb) }
    for (const [key, j] of idx.z) {
      const [o_, t] = key.split('|')
      const l = idx.u.has(key) ? key : k2(t, o_)
      const used = meta.items.reduce((s, k) => s + X(idx.g.get(k3(k, o_, t))!), 0)
      if (used > 1e-6) req(X(idx.u.get(l)!) > 0.5 && X(j) > 0.5, 'rail flow without track/train', o_, t)
    }
    const rtd = new Map(meta.lines.map(ln => [k2(ln.a, ln.b), ln.rtd_min]))
    const rtdOf = (o_: string, t: string) => rtd.get(k2(o_, t)) ?? rtd.get(k2(t, o_))!
    for (const [key, j] of idx.c) {
      const [o_, t] = key.split('|'), r = rtdOf(o_, t)
      const stk = idx.solids.reduce((s, k) => s + X(idx.g.get(k3(k, o_, t))!) / d.stack(k), 0)
      const its = idx.solids.reduce((s, k) => s + X(idx.g.get(k3(k, o_, t))!), 0)
      const c = Math.round(X(j))
      req(stk <= meta.slots / r * c * (1 + 1e-3) + 1e-6, 'car capacity', o_, t, stk, c)
      req(its <= platformCap(meta.feed, r, meta.dock_s) * c * (1 + 1e-3) + 1e-6, 'platform', o_, t)
    }
    for (const [key, j] of idx.cf) {
      const [o_, t] = key.split('|'), r = rtdOf(o_, t)
      const vol = idx.fluids.reduce((s, k) => s + X(idx.g.get(k3(k, o_, t))!), 0)
      const c = Math.round(X(j))
      req(vol <= meta.fluid_car / r * c * (1 + 1e-3) + 1e-6, 'fluid car', o_, t, vol, c)
      req(vol <= platformCap(1200, r, meta.dock_s) * c * (1 + 1e-3) + 1e-6, 'fluid platform', o_, t)
    }
    const cb = new Map(meta.lines.map(ln => [k2(ln.a, ln.b), ln.cbar + ln.cbar_fluid]))
    meta.cbar_binding = [...idx.z.keys()].filter(key => {
      const [o_, t] = key.split('|')
      const n = (idx.c.has(key) ? Math.round(X(idx.c.get(key)!)) : 0) + (idx.cf.has(key) ? Math.round(X(idx.cf.get(key)!)) : 0)
      return n >= (cb.get(key) ?? cb.get(k2(t, o_))!)
    }).map(key => key.replace('|', '->')).sort(cmp)
  }
}

// ---------- solution summary ----------
export const round = (v: number, n: number) => { const f = 10 ** n; return Math.round(v * f) / f }
const r6 = (v: number) => round(v, 6)
export interface Dir { cars: number; fluid_cars: number; flow: Record<string, number> }
export interface Summary {
  cost_parts: Record<string, number>; hubs: string[]
  lines: { a: string; b: string; length_m: number; dirs: Record<string, Dir> }[]
  belts: { a: string; b: string; n: number; length_m: number; flow: Record<string, number> }[]
  pipes: { a: string; b: string; n: number; length_m: number; flow: Record<string, number> }[]
  machines: Record<string, Record<string, number>>; machines_total: Record<string, number>
  extraction: Record<string, Record<string, number>>; rail_flow_items_per_min: number; belt_count: number
  /** supplied inputs taken at the base, /min (web addition) */
  supplied?: Record<string, number>
  cbar_binding?: string[]
}
const sortedObj = <T>(entries: [string, T][]) => Object.fromEntries(entries.sort((a, b) => cmp(a[0], b[0])))

/** geo_solve.summarize, plus each belt/pipe/line's length (for the map's totals). */
export function summarize(d: Data, m: Model, idx: Index, meta: Meta, xs: ArrayLike<number>): Summary {
  const X = (j: number) => Number(xs[j])
  const parts = new Map<string, number>()
  m.names.forEach((nm, j) => { const k = nm.split('|')[0]; parts.set(k, (parts.get(k) ?? 0) + m.c[j] * xs[j]) })
  const cost_parts = sortedObj([...parts].map(([k, v]) => [k, r6(v)]))
  const hubs = [...idx.h].filter(([, j]) => X(j) > 0.5).map(([s]) => s).sort(cmp)
  const lens = new Map(meta.lines.map(ln => [k2(ln.a, ln.b), ln.length_m]))
  const arcLen = new Map(d.arcs.map(([sa, sb, L]) => [k2(sa, sb), L]))
  const lines: Summary['lines'] = []
  for (const [l, j] of [...idx.u].sort((a, b) => cmp(a[0], b[0]))) {
    if (X(j) <= 0.5) continue
    const [sa, sb] = l.split('|')
    const dirs: Record<string, Dir> = {}
    for (const [o_, t] of [[sa, sb], [sb, sa]]) {
      const key = k2(o_, t)
      const flow: Record<string, number> = {}
      for (const k of meta.items) { const q = X(idx.g.get(k3(k, o_, t))!); if (q > 1e-6) flow[k] = r6(q) }
      dirs[`${o_}->${t}`] = {
        cars: idx.c.has(key) ? Math.round(X(idx.c.get(key)!)) : 0, fluid_cars: idx.cf.has(key) ? Math.round(X(idx.cf.get(key)!)) : 0, flow,
      }
    }
    lines.push({ a: sa, b: sb, length_m: lens.get(l)!, dirs })
  }
  const arcsOf = (mp: Map<K2, number>, its: string[]) => {
    const out: Summary['belts'] = []
    for (const [a, j] of [...mp].sort((p_, q) => cmp(p_[0], q[0]))) {
      if (X(j) <= 0.5) continue
      const [sa, sb] = a.split('|')
      const flow: Record<string, number> = {}
      for (const k of its) { const q = X(idx.f.get(k3(k, sa, sb))!); if (q > 1e-6) flow[k] = r6(q) }
      out.push({ a: sa, b: sb, n: Math.round(X(j)), length_m: arcLen.get(a)!, flow })
    }
    return out
  }
  const belts = arcsOf(idx.b, idx.solids), pipes = arcsOf(idx.bp, idx.fluids)
  const siteRec = new Map<string, [string, number][]>()
  for (const [key, j] of [...idx.x].sort((a, b) => cmp(a[0], b[0]))) {
    if (X(j) <= 1e-6) continue
    const [r, s] = key.split('|')
    if (!siteRec.has(s)) siteRec.set(s, [])
    siteRec.get(s)!.push([r, r6(X(j))])
  }
  const machines = sortedObj([...siteRec].map(([s, l]) => [s, Object.fromEntries(l)]))
  const tot = new Map<string, number>()
  for (const l of siteRec.values()) for (const [r, n] of l) tot.set(r, (tot.get(r) ?? 0) + n)
  const machines_total = sortedObj([...tot].map(([r, n]) => [r, r6(n)]))
  const ext = new Map<string, [string, number][]>()
  for (const [key, j] of [...idx.e].sort((a, b) => cmp(a[0], b[0]))) {
    if (X(j) <= 1e-6) continue
    const [k, s] = key.split('|')
    if (!ext.has(s)) ext.set(s, [])
    ext.get(s)!.push([k, r6(X(j))])
  }
  const extraction = sortedObj([...ext].map(([s, l]) => [s, Object.fromEntries(l)]))
  let rail = 0
  for (const j of idx.g.values()) rail += X(j)
  return {
    cost_parts, hubs, lines, belts, pipes, machines, machines_total, extraction,
    rail_flow_items_per_min: r6(rail), belt_count: belts.reduce((s, b_) => s + b_.n, 0),
    ...(idx.sup.size ? { supplied: sortedObj([...idx.sup].filter(([, j]) => X(j) > 1e-6).map(([k, j]) => [k, r6(X(j))])) } : {}),
  }
}

// ---------- why a plan cannot be sited (web addition) ----------
/** Items the recipes need but nothing provides: not raw, not made by a recipe, not supplied. */
export function unmakeable(d: Data, recs: GameRecipe[], targets: Record<string, number>, supplied: Record<string, number> | null = null): string[] {
  const made = new Set(recs.flatMap(r => r.out.map(([k]) => k)))
  const need = new Set([...recs.flatMap(r => r.in.map(([k]) => k)), ...Object.keys(targets)])
  return [...need].filter(k => !d.raw.has(k) && !made.has(k) && !((supplied?.[k] ?? 0) > 0)).sort(cmp)
}
/** Raw items the location-free plan needs more of than all nodes of the map give at `tier`: [item, need, have]. */
export function rawShortfalls(d: Data, gross: Map<string, number>, tier: number, beltCap: number, pipeCap: number, waterAnywhere = true): [string, number, number][] {
  const have = new Map<string, number>()
  for (const s of d.sid) for (const [k, q] of d.supply(s, tier, beltCap, pipeCap)) have.set(k, (have.get(k) ?? 0) + q)
  const out: [string, number, number][] = []
  for (const [k, q] of gross) {
    if (!d.raw.has(k) || (k === 'Desc_Water_C' && waterAnywhere)) continue
    const h = have.get(k) ?? 0
    if (q > h * (1 + 1e-9) + 1e-6) out.push([k, q, h])
  }
  return out.sort((a, b) => cmp(a[0], b[0]))
}

// ---------- one haul, closed form (geo_breakeven.py) ----------
export type HaulMeta = Pick<Meta, 'belt_cap' | 'belt_oe_km' | 'v_mps' | 'dock_s' | 'acc_s' | 'slots' | 'feed' | 'F' | 'track_oe_km' | 'loco_oe' | 'car_oe' | 'platform_oe'>
/** [belt OE, rail OE, cars] of one point-to-point haul of Q items/min over L metres. */
export function haulCosts(meta: HaulMeta, Q: number, stack: number, L: number): [number, number, number] {
  const km = L / 1000
  const belt = Math.ceil(Q / meta.belt_cap - 1e-9) * km * meta.belt_oe_km
  const rtd = rtdMin(L, meta.v_mps, meta.dock_s, meta.acc_s)
  const perCar = Math.min(meta.slots / rtd * stack, platformCap(meta.feed, rtd, meta.dock_s))
  const cars = Math.ceil(Q / perCar - 1e-9)
  const rail = 2 * meta.F + km * meta.track_oe_km + meta.loco_oe + cars * (meta.car_oe + 2 * meta.platform_oe)
  return [belt, rail, cars]
}
/** Smallest number of full belts from which one rail line is cheaper (and stays so up to nmax), or null. */
export function thresholdBelts(meta: HaulMeta, stack: number, L: number, nmax = 60): number | null {
  const ok: boolean[] = []
  for (let n = 1; n <= nmax; n++) { const [bc, rc] = haulCosts(meta, n * meta.belt_cap, stack, L); ok.push(rc <= bc) }
  for (let i = 0; i < ok.length; i++) if (ok.slice(i).every(Boolean)) return i + 1
  return null
}

// ---------- candidate hubs for another base (geo_sites.py hub rule) ----------
const NOT_COUNTED = new Set(['Desc_Water_C', 'Desc_NitrogenGas_C', 'Desc_GeothermalEnergy_C'])
/** K candidate hubs with `base` required: base, the K//3 grid sites of largest raw output, then farthest-point sampling. */
export function hubsFor(doc: SitesDoc, base: string, k = 12): string[] {
  const grid = doc.sites.filter(s => s.kind === 'grid')
  const xy = new Map(doc.sites.map(s => [s.id, [s.x, s.y]]))
  const chosen = [base], reason = new Map([[base, 'base']])
  const out = (s: Site) => Object.entries(s.supply ?? {}).reduce((t, [i, v]) => t + (NOT_COUNTED.has(i) ? 0 : v), 0)
  for (const s of [...grid].sort((a, b) => out(b) - out(a) || cmp(a.id, b.id))) {
    if ([...reason.values()].filter(r => r === 'top_output').length >= Math.floor(k / 3)) break
    if (!reason.has(s.id)) { chosen.push(s.id); reason.set(s.id, 'top_output') }
  }
  while (chosen.length < k) {
    let best: string | null = null, bestD = -1
    for (const s of grid) {
      if (reason.has(s.id)) continue
      const [x, y] = xy.get(s.id)!
      const dmin = Math.min(...chosen.map(j => { const [x2, y2] = xy.get(j)!; return Math.hypot(x - x2, y - y2) }))
      if (dmin > bestD || (dmin === bestD && cmp(s.id, best!) > 0)) { best = s.id; bestD = dmin }
    }
    if (best === null) break
    chosen.push(best); reason.set(best, 'farthest_point')
  }
  return chosen.sort(cmp)
}
