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
import type { GeoNode, Purity } from './model.ts'

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

export interface SaveExtractor { node: string; item: string; building: string }
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
}

// the parsed save, as far as it is read here (the parser's own types are much wider)
interface Ref { pathName?: string }
interface Prop { value?: unknown; values?: unknown[] }
interface SaveObject {
  typePath: string; instanceName: string
  properties?: Record<string, Prop>
  transform?: { translation?: { x: number; y: number } }
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

export function summarizeSave(save: ParsedSave, nodes: GeoNode[]): SaveSummary {
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
        if (n) extractors.push({ node: n.id, item: n.item, building })
        else unmatched++
        continue
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
  return {
    session: h.sessionName ?? '', saveVersion: h.saveVersion ?? 0, buildVersion: h.buildVersion ?? 0, playHours: (h.playDurationSeconds ?? 0) / 3600,
    tier, milestones, hub, extractors, unmatched, waterPumps, purity,
  }
}
