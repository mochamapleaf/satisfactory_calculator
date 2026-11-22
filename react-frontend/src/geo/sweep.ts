// p sweep of the siting model (geo_solve.sweep, report §7.2 "主火车站最优数量", step G6): solve with Σh ≤ p for
// p = 0, 2, 3, … pmax at F 1×, warm-starting each from the best solution so far (mapped by variable name), and keep
// the envelope cost(p) = min(this solve, cost(p−1)). p = 1 equals p = 0 (one hub cannot run a line and costs F), so
// it is copied, not solved. Then F 0.5× and 2× at p ≤ pmax give the sensitivity of p* (the number of hubs of the best
// plan). Each solve has a time limit; rows say whether they are proven optimal.
import {
  CheckError, Data, aggregateGross, build, check, hubsFor, materials, oreEquivalents, rawShortfalls, recipeClosure, summarize, unmakeable,
  type GameLite, type GameRecipe, type GeoNode, type HaulMeta, type LpSolve, type SitesDoc, type Summary,
} from './model.ts'

export interface SweepOptions {
  targets: Record<string, number>; base: string; tier: number; workKm: number; pmax: number; timeLimit: number
  /** recipe ids to use (the plan's), or null for the base-recipe closure as in geo_solve.py */
  recipeIds: string[] | null
  fmults?: number[]
  /** steps already done (`${fmult}|${p}`), when the page restarts a sweep after a stuck solve; and the envelope so far */
  skip?: string[]; envStart?: number | null
  /** supplied inputs of the plan (/min), taken free at the base site; only with the plan's recipes */
  supplied?: Record<string, number> | null
  /** display names for messages (item id -> name) */
  names?: Record<string, string>
}
export interface SweepRow {
  fmult: number; F: number; p: number; obj: number | null; bound: number | null; gap: number; status: string; time_s: number
  envelope: number | null; hubs: string[]; n_lines: number; rail_flow: number; belts: number; copied?: boolean
}
export interface SweepMeta extends HaulMeta {
  base: string; hubs: string[]; tier: number; belt_tier: number; pipe_cap: number; pipe_oe_km: number; station_oe: number
  n_vars: number; n_int: number; n_rows: number; recipes: string[]; items: string[]; rail: boolean
}
export type SweepEvent =
  | { type: 'meta'; meta: SweepMeta }
  | { type: 'solving'; label: string; fmult: number; p: number; limit: number }
  | { type: 'progress'; label: string; bound: number | null; best: number | null; nodes: number }
  | { type: 'row'; row: SweepRow; summary: Summary | null }
  | { type: 'done'; ms: number }
  | { type: 'error'; message: string }

export interface GeoInput { sitesDoc: SitesDoc; nodes: GeoNode[]; transport: unknown }

export function runSweep(game: GameLite, geo: GeoInput, solve: LpSolve, o: SweepOptions, emit: (e: SweepEvent) => void) {
  const t0 = performance.now()
  const nm = (k: string) => o.names?.[k] ?? k
  const per = (v: number) => (v >= 100 ? Math.round(v) : Math.round(v * 10) / 10)
  try {
    sweep(t0, nm, per)
  } catch (err) {
    emit({ type: 'error', message: String(err instanceof Error ? err.message : err) })
  }

  function sweep(t0: number, nm: (k: string) => string, per: (v: number) => number) {
  const docHubs = geo.sitesDoc.hubs.map(h => h.site)
  if (!geo.sitesDoc.sites.some(s => s.id === o.base)) throw new Error(`Unknown base site ${o.base}.`)
  const hubs = docHubs.includes(o.base) ? docHubs : hubsFor(geo.sitesDoc, o.base)
  const d = new Data(game, geo.sitesDoc, geo.nodes, geo.transport, hubs)
  let recipes: GameRecipe[] | null = null
  if (o.recipeIds) {
    const byId = new Map(game.recipes.map(r => [r.id, r]))
    recipes = [...new Set(o.recipeIds)].map(id => byId.get(id)).filter((r): r is GameRecipe => !!r).sort((a, b) => (a.id < b.id ? -1 : 1))
  }
  const supplied = o.recipeIds ? o.supplied ?? null : null
  // nothing provides an input: say which, before any solve (the LP would only answer "infeasible")
  const lost = unmakeable(d, recipes ?? recipeClosure(d, o.targets), o.targets, supplied)
  if (lost.length) throw new Error(`Nothing on the map makes ${lost.map(nm).join(', ')} with ${o.recipeIds ? "this plan's recipes" : 'the base recipes'}. ${o.recipeIds ? 'Choose “base recipes”, or change the plan.' : ''}`.trim())
  const oe = oreEquivalents(d, materials(), solve)
  const common = { base: o.base, tier: o.tier, workKm: o.workKm, oe, recipes, supplied }
  let env: number | null = o.envStart ?? null
  const skip = new Set(o.skip ?? [])
  let warm: Map<string, number> | null = null
  let metaSent = false
  const solveOne = (p: number, fmult: number, warmFrom: Map<string, number> | null) => {
    const label = `F ${fmult}× · p ≤ ${p}`
    const { m, idx, meta } = build(d, o.targets, solve, { ...common, p, fmult })
    if (!metaSent) {
      metaSent = true
      emit({
        type: 'meta', meta: {
          base: meta.base, hubs: d.hubs, tier: meta.tier, belt_tier: meta.belt_tier, belt_cap: meta.belt_cap, belt_oe_km: meta.belt_oe_km,
          pipe_cap: meta.pipe_cap, pipe_oe_km: meta.pipe_oe_km, v_mps: meta.v_mps, dock_s: meta.dock_s, acc_s: meta.acc_s, slots: meta.slots,
          feed: meta.feed, F: meta.F / fmult, // always the 1× hub cost, also when a restarted sweep begins at another F
          track_oe_km: meta.track_oe_km, loco_oe: meta.loco_oe, car_oe: meta.car_oe, platform_oe: meta.platform_oe,
          station_oe: meta.station_oe, n_vars: m.c.length, n_int: m.integ.filter(Boolean).length, n_rows: m.rows.length,
          recipes: meta.recipes, items: meta.items, rail: meta.rail,
        },
      })
    }
    let w: Float64Array | null = null
    if (warmFrom) {
      w = new Float64Array(m.c.length)
      m.names.forEach((nm, j) => { const v = warmFrom.get(nm); if (v !== undefined) w![j] = v })
    }
    emit({ type: 'solving', label, fmult, p, limit: o.timeLimit })
    let res = solve(m, { timeLimit: o.timeLimit, warm: w, progress: pr => emit({ type: 'progress', label, ...pr }) })
    let summary: Summary | null = null, named: Map<string, number> | null = null
    if (res.x) {
      // independent re-check: a solution that fails it is dropped (this point counts as not solved), the sweep goes on
      try { check(d, m, idx, meta, res.x, p) } catch (err) {
        if (!(err instanceof CheckError)) throw err
        res = { ...res, x: null, obj: null, status: 'rejected' }
      }
    }
    if (res.x) {
      summary = summarize(d, m, idx, meta, res.x)
      summary.cbar_binding = meta.cbar_binding ?? []
      named = new Map()
      m.names.forEach((nm, j) => { if (Math.abs(res.x![j]) > 1e-9) named!.set(nm, res.x![j]) })
    }
    return { res, meta, summary, named, idx }
  }
  const rowOf = (p: number, fmult: number, F: number, r: ReturnType<typeof solveOne>['res'], s: Summary | null, envelope: number | null): SweepRow => ({
    fmult, F, p, obj: r.obj, bound: r.bound, gap: r.gap, status: r.status, time_s: Math.round(r.time * 100) / 100, envelope,
    hubs: s?.hubs ?? [], n_lines: s?.lines.length ?? 0, rail_flow: s?.rail_flow_items_per_min ?? 0, belts: s?.belt_count ?? 0,
  })
  let lastRow: SweepRow | null = null
  let lastSummary: Summary | null = null
  {
    let best: { named: Map<string, number>; obj: number } | null = null
    for (let p = 0; p <= o.pmax; p++) {
      if (skip.has(`1|${p}`)) continue
      if (p === 1) {
        // a line needs two open hubs, so one hub only adds F: cost(1) = cost(0)
        if (lastRow) emit({ type: 'row', row: { ...lastRow, p: 1, copied: true }, summary: lastSummary })
        continue
      }
      const r = solveOne(p, 1, warm)
      const obj = r.res.obj
      if (!r.res.x && /infeasible/i.test(r.res.status)) {
        // more hubs cannot help (rail only moves items): stop here and say what is short
        const recs = r.meta.recipes.map(id => game.recipes.find(x => x.id === id)!)
        let short: [string, number, number][] = []
        try { short = rawShortfalls(d, aggregateGross(d, recs, r.idx.A, o.targets, r.meta.items, solve, supplied), o.tier, r.meta.belt_cap, r.meta.pipe_cap) } catch { /* no location-free plan either */ }
        throw new Error(short.length
          ? `The map cannot supply this plan at tier ${o.tier}: ${short.map(([k, q, h]) => `${nm(k)} ${per(q)}/min needed, ${per(h)}/min from all nodes`).join('; ')}.`
          : `No siting satisfies this plan at tier ${o.tier}: the nodes cannot supply it (a resource is locked at this tier, or its nodes are too small).`)
      }
      if (obj !== null) env = env === null ? obj : Math.min(env, obj)
      const row = rowOf(p, 1, r.meta.F, r.res, r.summary, env)
      lastRow = row; lastSummary = r.summary
      emit({ type: 'row', row, summary: r.summary })
      if (r.named && obj !== null && (warm === null || obj <= env! + 1e-9)) { warm = r.named; best = { named: r.named, obj } }
      // no rail at this tier: every p is the same model, one solve is the whole sweep
      if (!r.meta.rail) { emit({ type: 'done', ms: performance.now() - t0 }); return }
    }
    for (const fm of o.fmults ?? [0.5, 2]) {
      if (skip.has(`${fm}|${o.pmax}`)) continue
      const r = solveOne(o.pmax, fm, best?.named ?? null)
      emit({ type: 'row', row: rowOf(o.pmax, fm, r.meta.F, r.res, r.summary, r.res.obj), summary: r.summary })
    }
    emit({ type: 'done', ms: performance.now() - t0 })
  }
  }
}
