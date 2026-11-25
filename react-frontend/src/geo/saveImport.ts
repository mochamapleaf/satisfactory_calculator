// Save import for the map (report G10, memo step 18): what a Satisfactory .sav tells the siting model. The save is
// parsed by @etothepii/satisfactory-file-parser (MIT) in saveWorker.ts; this file only reads the parsed objects, so
// it runs in node tests without the parser.
//   - tech tier: the highest tier among the purchased HUB milestones (tiers from data/game-data.json progression;
//     the file numbers are not the tiers: Schematic_3-2 is a tier-2 milestone). MINE: the map then allows everything
//     up to that tier; milestones of that tier not bought and alternates bought are not read
//   - base: the HUB (Build_TradingPost) position
//   - extractors: miners, oil pumps, resource-well extractors and pressurizers, geothermal generators on map nodes
//     (mExtractableResource), matched by node id (1.0 saves) or else by the extractor's own position (it stands on
//     its node) against map nodes of the matching kind within 10 m
//   - purity: only when a node object carries a purity property (modded / randomised worlds) that differs from the
//     map; vanilla saves keep purity in the game files, not in the save
//   - built (memo step 19, with `sites`): the factory as the multi-period plan's `existing` amounts, by grid site
//     (the 1 km cell a building stands in, or the nearest site for an empty cell): production machines `x|recipe|site`
//     (clock-weighted), mining `e|item|site` (each extractor's rate from its node purity, miner Mk and clock), belts
//     `b|a|b` and pipes `bp|a|b` as the capacity of each segment that crosses from one site to a neighbouring one, and
//     open hubs `h|site` where a candidate hub site has a train station. MINE: tracks, trains and cars are not
//     credited; pipes count both ways (their flow direction is not in the save); somersloops are ignored
import { MK1_BASE, OIL_BASE, WELL_BASE, type GeoNode, type Purity, type SitesDoc } from './model.ts'

/** HUB milestone → tech tier, from data/game-data.json `progression.milestones` (geo.test.ts checks they agree). */
export const MILESTONE_TIER: Record<string, number> = {
  'Schematic_1-1': 1, 'Schematic_1-2': 1, 'Schematic_1-3': 1, 'Schematic_2-1': 2, 'Schematic_2-2': 2, 'Schematic_2-3': 2, 'Schematic_2-5': 2,
  'Schematic_3-2': 2, 'Schematic_3-1': 3, 'Schematic_3-3': 3, 'Schematic_3-4': 3, 'Schematic_4-2': 3, 'Schematic_4-1': 4, 'Schematic_4-3': 4,
  'Schematic_4-4': 4, 'Schematic_4-5': 4, 'Schematic_5-3': 4, 'Schematic_5-1': 5, 'Schematic_5-4': 5, 'Schematic_5-5': 5, 'Schematic_6-1': 5,
  'Schematic_6-2': 5, 'Schematic_5-2': 6, 'Schematic_6-3': 6, 'Schematic_6-5': 6, 'Schematic_6-6': 6, 'Schematic_6-7': 6, 'Schematic_7-1': 7,
  'Schematic_7-2': 7, 'Schematic_7-3': 7, 'Schematic_7-5': 7, 'Schematic_8-3': 7, 'Schematic_7-4': 8, 'Schematic_8-1': 8, 'Schematic_8-2': 8,
  'Schematic_8-4': 8, 'Schematic_8-5': 8, 'Schematic_9-1': 9, 'Schematic_9-2': 9, 'Schematic_9-3': 9, 'Schematic_9-4': 9, 'Schematic_9-5': 9,
}

/** Extractor building → the map node type it stands on. */
const NODE_TYPE: Record<string, string> = {
  MinerMk1: 'node', MinerMk2: 'node', MinerMk3: 'node', OilPump: 'node',
  FrackingExtractor: 'well', FrackingSmasher: 'well_core', GeneratorGeoThermal: 'geyser',
}
/** Buildings that stand on a node but do not mine it (the Plan on free nodes button leaves them alone). */
export const NOT_MINING = new Set(['GeneratorGeoThermal', 'FrackingSmasher'])

export interface SaveExtractor { node: string; item: string; building: string; clock: number }
export interface SaveSummary {
  session: string; saveVersion: number; buildVersion: number; playHours: number
  /** highest tier of a purchased milestone (0: none) and how many milestones */
  tier: number; milestones: number
  /** HUB position in map metres (x east, y south), null when the save has no HUB */
  hub: { x: number; y: number } | null
  extractors: SaveExtractor[]
  /** extractors whose node is not on the map (water pumps on lakes are counted separately) */
  unmatched: number; waterPumps: number
  /** node id → purity the save itself stores, where it differs from the map (modded or randomised worlds) */
  purity: Record<string, Purity>
  /** what stands, as the multi-period plan's `existing` amounts (only with `sites`) */
  built: SaveBuilt | null
}
export interface SaveBuilt {
  existing: Record<string, number>
  machines: number; recipes: number; miningRate: number
  /** belt / pipe segments crossing between neighbouring sites, and crossings between sites that are not neighbours */
  beltCrossings: number; pipeCrossings: number; skippedCrossings: number
  stations: number; hubs: string[]
  /** machines whose recipe the game data does not have (mods) */
  unknownRecipes: number
}

// the parsed save, as far as it is read here (the parser's own types are much wider)
interface Ref { pathName?: string }
interface Prop { value?: unknown; values?: unknown[] }
interface V3 { x: number; y: number; z?: number }
interface SaveObject {
  typePath: string; instanceName: string
  properties?: Record<string, Prop>
  transform?: { translation?: V3; rotation?: { x: number; y: number; z: number; w: number } }
}
export interface ParsedSave {
  header?: { sessionName?: string; saveVersion?: number; buildVersion?: number; playDurationSeconds?: number }
  levels: Record<string, { objects: SaveObject[] }> | { objects: SaveObject[] }[]
}

const short = (path: string) => path.split('.').pop() ?? path
// EResourcePurity: RP_Inpure (sic) 0, RP_Normal 1, RP_Pure 2
const purityOf = (v: unknown): Purity | null => {
  const raw = (v as { value?: unknown })?.value ?? v
  if (typeof raw === 'number') return raw === 0 ? 'impure' : raw === 1 ? 'normal' : raw === 2 ? 'pure' : null
  const s = String(raw).toLowerCase()
  return /inpure|impure/.test(s) ? 'impure' : /pure/.test(s) ? 'pure' : /normal/.test(s) ? 'normal' : null
}

const BELT_CAP = [60, 120, 270, 480, 780, 1200] // transport.json belts.tiers, Mk.1..Mk.6
const PIPE_CAP = [300, 600] // transport.json pipes.tiers
const MINER_MULT: Record<string, number> = { MinerMk1: 1, MinerMk2: 2, MinerMk3: 4 }
const clockOf = (o: SaveObject) => { const c = Number(o.properties?.mCurrentPotential?.value ?? 1); return Number.isFinite(c) && c > 0 ? c : 1 }
/** world position of a spline point (local, cm) in map metres: rotate by the quaternion, then translate */
function splineEnd(o: SaveObject, last: boolean): { x: number; y: number } | null {
  const pts = o.properties?.mSplineData?.values as { properties?: { Location?: { value?: V3 } } }[] | undefined
  const t = o.transform?.translation
  if (!pts?.length || !t) return null
  const v = pts[last ? pts.length - 1 : 0].properties?.Location?.value ?? { x: 0, y: 0, z: 0 }
  const q = o.transform?.rotation ?? { x: 0, y: 0, z: 0, w: 1 }
  const vz = v.z ?? 0
  // v' = v + 2w(q x v) + 2 q x (q x v)
  const cx = q.y * vz - q.z * v.y, cy = q.z * v.x - q.x * vz, cz = q.x * v.y - q.y * v.x
  const dx = q.y * cz - q.z * cy, dy = q.z * cx - q.x * cz
  return { x: (t.x + v.x + 2 * q.w * cx + 2 * dx) / 100, y: (t.y + v.y + 2 * q.w * cy + 2 * dy) / 100 }
}

export function summarizeSave(save: ParsedSave, nodes: GeoNode[], sites: SitesDoc | null = null, recipeIds: Set<string> | null = null): SaveSummary {
  const byId = new Map(nodes.map(n => [n.id, n]))
  const nearest = (x: number, y: number, type: string | undefined): GeoNode | null => {
    let best: GeoNode | null = null, d = 10
    for (const n of nodes) {
      if (type && n.type !== type) continue
      const e = Math.hypot(n.x - x, n.y - y)
      if (e < d) { d = e; best = n }
    }
    return best
  }
  const extractors: SaveExtractor[] = []
  let unmatched = 0, waterPumps = 0, tier = 0, milestones = 0
  let hub: { x: number; y: number } | null = null
  const purity: Record<string, Purity> = {}
  const machines: { recipe: string; x: number; y: number; clock: number }[] = []
  const segments: { cap: number; pipe: boolean; chain: string; a: { x: number; y: number }; b: { x: number; y: number } }[] = []
  const stations: { x: number; y: number }[] = []
  // one pass, no copy of the object list (a big save's parse is already ~85× the file size in memory)
  for (const level of Object.values(save.levels)) {
    for (const o of level.objects ?? []) {
      const props = o.properties
      const ref = props?.mExtractableResource?.value as Ref | undefined
      if (ref?.pathName) {
        const name = short(ref.pathName)
        if (/WaterVolume/i.test(name)) { waterPumps++; continue }
        const building = short(o.typePath).replace(/_C$/, '').replace(/^Build_/, '')
        const t = o.transform?.translation
        const n = byId.get(name) ?? (t ? nearest(t.x / 100, t.y / 100, NODE_TYPE[building]) : null)
        if (n) extractors.push({ node: n.id, item: n.item, building, clock: clockOf(o) })
        else unmatched++
        continue
      }
      if (sites) {
        const rec = (props?.mCurrentRecipe?.value as Ref | undefined)?.pathName
        const t = o.transform?.translation
        if (rec && t) { machines.push({ recipe: short(rec), x: t.x / 100, y: t.y / 100, clock: clockOf(o) }); continue }
        const belt = /Build_ConveyorBeltMk(\d)_C$/.exec(o.typePath)
        const pipe = /Build_Pipeline(MK2)?(_NoIndicator)?_C$/.exec(o.typePath)
        if (belt || pipe) {
          const a = splineEnd(o, false), b = splineEnd(o, true)
          const chain = (props?.mConveyorChainActor?.value as Ref | undefined)?.pathName || o.instanceName
          if (a && b) segments.push({ cap: belt ? BELT_CAP[Number(belt[1]) - 1] ?? 0 : PIPE_CAP[pipe![1] ? 1 : 0], pipe: !!pipe, chain, a, b })
          continue
        }
        if (/Build_TrainDockingStation(Liquid)?_C$|Build_TrainStation_C$/.test(o.typePath) && t) { stations.push({ x: t.x / 100, y: t.y / 100 }); continue }
      }
      if (/SchematicManager/.test(o.typePath)) {
        for (const v of (props?.mPurchasedSchematics?.values ?? []) as Ref[]) {
          const id = short(v.pathName ?? '').replace(/_C$/, '')
          if (id in MILESTONE_TIER) { milestones++; tier = Math.max(tier, MILESTONE_TIER[id]) }
        }
        continue
      }
      if (/Build_TradingPost/.test(o.typePath) && o.transform?.translation) {
        hub = { x: o.transform.translation.x / 100, y: o.transform.translation.y / 100 }
        continue
      }
      if (/ResourceNode|FrackingSatellite|FrackingCore/.test(o.typePath) && props) {
        const key = 'mPurity' in props ? 'mPurity' : Object.keys(props).find(k => /purity/i.test(k))
        const pv = key ? purityOf(props[key].value) : null
        if (!pv) continue
        const t = o.transform?.translation
        const n = byId.get(short(o.instanceName)) ?? (t ? nearest(t.x / 100, t.y / 100, undefined) : null)
        if (n && n.purity !== pv) purity[n.id] = pv
      }
    }
  }
  const h = save.header ?? {}
  const built = sites ? buildExisting(sites, nodes, extractors, purity, machines, segments, stations, recipeIds) : null
  return {
    session: h.sessionName ?? '', saveVersion: h.saveVersion ?? 0, buildVersion: h.buildVersion ?? 0, playHours: (h.playDurationSeconds ?? 0) / 3600,
    tier, milestones, hub, extractors, unmatched, waterPumps, purity, built,
  }
}

function buildExisting(sites: SitesDoc, nodes: GeoNode[], extractors: SaveExtractor[], purity: Record<string, Purity>,
  machines: { recipe: string; x: number; y: number; clock: number }[],
  segments: { cap: number; pipe: boolean; chain: string; a: { x: number; y: number }; b: { x: number; y: number } }[],
  stations: { x: number; y: number }[], recipeIds: Set<string> | null): SaveBuilt {
  const grid = sites.sites.filter(s => s.kind === 'grid')
  const byCell = new Map(grid.filter(s => s.cell).map(s => [`${s.cell![0]},${s.cell![1]}`, s.id]))
  const siteOf = (x: number, y: number): string => byCell.get(`${Math.floor(x / 1000)},${Math.floor(y / 1000)}`)
    ?? grid.reduce((b, s) => (Math.hypot(s.x - x, s.y - y) < Math.hypot(b.x - x, b.y - y) ? s : b)).id
  const nodeSite = new Map<string, string>()
  for (const s of grid) for (const n of s.nodes) nodeSite.set(n, s.id)
  const nodeById = new Map(nodes.map(n => [n.id, n]))
  const arcLen = new Map(sites.arcs.flatMap(a => [[`${a.a}|${a.b}`, a.length_m], [`${a.b}|${a.a}`, a.length_m]] as [string, number][]))
  const ex: Record<string, number> = {}
  const add = (k: string, v: number) => { ex[k] = (ex[k] ?? 0) + v }
  let nMach = 0, unknownRecipes = 0
  const recs = new Set<string>()
  for (const m of machines) {
    if (recipeIds && !recipeIds.has(m.recipe)) { unknownRecipes++; continue }
    add(`x|${m.recipe}|${siteOf(m.x, m.y)}`, m.clock); nMach++; recs.add(m.recipe)
  }
  let miningRate = 0
  for (const e of extractors) {
    const n = nodeById.get(e.node), s = nodeSite.get(e.node)
    if (!n || !s) continue
    const pur = purity[n.id] ?? n.purity
    // a miner's output is capped by its one output belt (Mk.6, 1200/min), as in nodeRate
    const rate = e.building in MINER_MULT ? Math.min(MK1_BASE[pur] * MINER_MULT[e.building] * e.clock, BELT_CAP[5])
      : e.building === 'OilPump' ? OIL_BASE[pur] * e.clock
        : e.building === 'FrackingExtractor' ? WELL_BASE[pur] * e.clock : 0
    if (rate > 0) { add(`e|${n.item}|${s}`, rate); miningRate += rate }
  }
  // a belt that runs along a cell edge crosses it back and forth: per conveyor chain (1.0 saves; a U8 segment and a
  // pipe segment are their own chain) only the net crossings count, and a chain is credited once per arc and
  // direction. A model arc hauls between site centroids (~1.2 km), so a chain is credited only in proportion to its
  // length: cap × min(1, chain length / arc length). A 50 m belt inside one factory that straddles a cell line adds
  // little (review H1: most crossings were such). MINE: U8 trunk belts (no chain actor) are under-credited
  const chainLen = new Map<string, number>()
  for (const g of segments) chainLen.set(g.chain, (chainLen.get(g.chain) ?? 0) + Math.hypot(g.b.x - g.a.x, g.b.y - g.a.y))
  let beltCrossings = 0, pipeCrossings = 0, skippedCrossings = 0
  const net = new Map<string, { cap: number; pipe: boolean; n: number }>() // chain|sa|sb → net crossings sa→sb
  for (const g of segments) {
    const sa = siteOf(g.a.x, g.a.y), sb = siteOf(g.b.x, g.b.y)
    if (sa === sb || !(g.cap > 0)) continue
    if (!arcLen.has(`${sa}|${sb}`)) { skippedCrossings++; continue }
    const fwd = sa < sb, key = `${g.chain}|${fwd ? sa : sb}|${fwd ? sb : sa}`
    const r = net.get(key) ?? { cap: g.cap, pipe: g.pipe, n: 0 }
    r.cap = Math.min(r.cap, g.cap) // a chain carries what its slowest segment carries
    r.n += fwd ? 1 : -1
    net.set(key, r)
  }
  for (const [key, r] of net) {
    if (r.n === 0) continue
    const parts = key.split('|'), hi = parts.pop()!, lo = parts.pop()!, chain = parts.join('|')
    const [sa, sb] = r.n > 0 ? [lo, hi] : [hi, lo]
    const cap = r.cap * Math.min(1, (chainLen.get(chain) ?? 0) / arcLen.get(`${sa}|${sb}`)!)
    if (!(cap > 1e-6)) continue
    // MINE: a pipe's flow direction is not in the save, so it counts both ways
    if (r.pipe) { add(`bp|${sa}|${sb}`, cap); add(`bp|${sb}|${sa}`, cap); pipeCrossings++ }
    else { add(`b|${sa}|${sb}`, cap); beltCrossings++ }
  }
  const hubSet = new Set(sites.hubs.map(h => h.site))
  const hubs = [...new Set(stations.map(p => siteOf(p.x, p.y)).filter(s => hubSet.has(s)))].sort()
  for (const s of hubs) ex[`h|${s}`] = 1
  return { existing: ex, machines: nMach, recipes: recs.size, miningRate, beltCrossings, pipeCrossings, skippedCrossings, stations: stations.length, hubs, unknownRecipes }
}
