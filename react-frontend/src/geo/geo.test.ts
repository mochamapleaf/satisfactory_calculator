// Tests of the web siting model (geo/model.ts on highs-js) against tools/geo_solve.py: the geo_solve_test.py cases
// as TypeScript, and every case of python-ref.json (written by tools/geo_reference_dump.py with highspy) solved again
// here with the same options; objectives must agree within the solvers' gaps and the models must have the same size.
// Run: node --test src/geo/geo.test.ts   (from react-frontend; prints one progress line per solve)
import { test } from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import highsLoader from 'highs'
import {
  Data, type BuildOptions, type GeoNode, type SitesDoc, build, check, CheckError, hubsFor, haulCosts, nodeRate, oreEquivalents,
  perMin, platformCap, rtdMin, summarize, thresholdBelts, W_RAW, materials, recipeClosure, type LpSolve, type Summary,
} from './model.ts'
import { highsSolver } from './highsLp.ts'
import { runSweep, type SweepMeta, type SweepOptions, type SweepRow } from './sweep.ts'

const R = (p: string) => JSON.parse(fs.readFileSync(new URL('../../../' + p, import.meta.url), 'utf8'))
const game = R('data/game-data.json'), sitesDoc: SitesDoc = R('data/satisfactory-geo/sites.json')
const nodes: GeoNode[] = R('data/satisfactory-geo/nodes.json').nodes, tr = R('data/satisfactory-geo/transport.json')
const ref = JSON.parse(fs.readFileSync(new URL('./python-ref.json', import.meta.url), 'utf8'))
const solve = highsSolver(await highsLoader())
const D = new Data(game, sitesDoc, nodes, tr)
const HMF = { Desc_ModularFrameHeavy_C: 10 }
const OE = oreEquivalents(D, materials(), solve)

function run(d: Data, targets: Record<string, number>, o: BuildOptions & { gap?: number; timeLimit?: number } = {}, label = '') {
  const t0 = performance.now()
  const { m, idx, meta } = build(d, targets, solve, { oe: d === D ? OE : null, ...o })
  const res = solve(m, { gap: o.gap ?? 1e-4, timeLimit: o.timeLimit ?? 180 })
  if (res.x) check(d, m, idx, meta, res.x, o.p ?? null)
  const s = res.x ? summarize(d, m, idx, meta, res.x) : null
  console.log(`  [solve] ${label} ${res.status} obj=${res.obj} gap=${res.gap.toExponential(2)} ${((performance.now() - t0) / 1000).toFixed(1)}s vars=${m.c.length}`)
  return { res, m, idx, meta, s }
}

test('formulas (hand values, report §2.2 and wiki throughput table)', () => {
  const r = rtdMin(1000, 33.333, 27.08, 30)
  assert.ok(Math.abs(r - 174.16 / 60) < 1e-3)
  assert.ok(Math.abs(platformCap(2400, r, 27.08) - 2026.8) < 0.2)
  assert.ok(Math.abs(platformCap(2400, 427.08 / 60, 27.08) - 2247.8) < 0.1)
})

test('ore equivalents equal the Python LP (plate 1.5, steel pipe 3, beam 8, …)', () => {
  for (const [k, v] of Object.entries(ref.oe as Record<string, number>)) assert.ok(Math.abs(OE[k] - v) < 1e-9, `${k} ${OE[k]} vs ${v}`)
  assert.ok(Math.abs(OE.Desc_IronPlate_C - 1.5) < 1e-9 && Math.abs(OE.Desc_SteelPipe_C - 3) < 1e-9 && Math.abs(OE.Desc_SteelPlate_C - 8) < 1e-9)
})

test('tier supply per node', () => {
  const pure = { type: 'node', item: 'Desc_OreIron_C', purity: 'pure' } as const
  assert.equal(nodeRate(pure, 8, 780, 600), 780)
  assert.equal(nodeRate(pure, 8, 1200, 600), 1200)
  assert.equal(nodeRate(pure, 6, 60, 600), 60)
  assert.equal(nodeRate({ type: 'node', item: 'Desc_OreIron_C', purity: 'impure' }, 2, 120, 300), 30)
  assert.equal(nodeRate({ type: 'node', item: 'Desc_LiquidOil_C', purity: 'pure' }, 5, 780, 300), 300)
  assert.equal(nodeRate({ type: 'well', item: 'Desc_NitrogenGas_C', purity: 'pure' }, 6, 780, 600), 0)
})

test('hub rule of geo_sites.py reproduces sites.json for its base', () => {
  assert.deepEqual(hubsFor(sitesDoc, 'S0_m1'), sitesDoc.hubs.map(h => h.site).sort())
})

test('machine counts (hand values): 10 HMF = 5 Manufacturers + 25 Assemblers', () => {
  const rec = new Map(D.recipes.map(r => [r.id, r]))
  const a = perMin(rec.get('Recipe_ModularFrameHeavy_C')!)
  assert.equal(a.get('Desc_ModularFrameHeavy_C'), 2)
  assert.equal(a.get('Desc_IronScrew_C'), -240)
  assert.equal(perMin(rec.get('Recipe_IronPlate_C')!).get('Desc_IronPlate_C'), 20)
  const { res, s, meta } = run(D, HMF, { rail: false }, 'hand values 10 HMF belt-only')
  assert.equal(res.status, 'Optimal')
  assert.ok(Math.abs(s!.machines_total.Recipe_ModularFrameHeavy_C - 5) < 1e-6)
  assert.ok(Math.abs(s!.machines_total.Recipe_ModularFrame_C - 25) < 1e-6)
  const mach = Object.entries(s!.machines_total).reduce((t, [r, n]) => t + n * meta.mach_oe[r], 0)
  assert.ok(Math.abs(mach - s!.cost_parts.x) < 1e-3)
})

function twoSite(dist: number, n = 12) {
  const ns: GeoNode[] = Array.from({ length: n }, (_, i) => ({ id: `N${i}`, item: 'Desc_OreIron_C', purity: 'pure', type: 'node', x: 0, y: 0, max_rate: 1200 }))
  const L = Math.round(dist * 1.3 * 10) / 10
  const site = (id: string, x: number, nn: string[]) => ({ id, kind: 'grid', x, y: 0, supply: {}, feed_dist_mean_m: 0, nodes: nn })
  const doc: SitesDoc = {
    params: { detour_factor: 1.3 }, sites: [site('A', 0, ns.map(x => x.id)), site('B', dist, [])],
    arcs: [{ a: 'A', b: 'B', length_m: L }], hubs: [{ site: 'B', reason: 'base' }, { site: 'A', reason: 'top_output' }],
  }
  return { d: new Data(game, doc, ns, tr), L }
}

test('two-site closed form (6.5 km route): 780/min by belt, 7800/min by rail with 21 cars', () => {
  const { d, L } = twoSite(5000)
  assert.equal(L, 6500)
  for (const [Q, mode, transport] of [[780, 'belt', 7583.333], [7800, 'rail', 62199]] as const) {
    const { res, meta, s } = run(d, { Desc_OreIron_C: Q }, { gap: 1e-9, timeLimit: 60 }, `two-site ${Q}`)
    assert.equal(res.status, 'Optimal')
    assert.ok(Math.abs(meta.F - 3490) < 1e-6 && Math.abs(meta.track_oe_km - 1826) < 1e-6)
    assert.ok(Math.abs(meta.belt_oe_km - 1166.667) < 1e-2)
    assert.ok(Math.abs(meta.car_oe + 2 * meta.platform_oe - 2030) < 1e-6)
    const [belt, rail, cars] = haulCosts(meta, Q, 100, L)
    assert.ok(Math.abs(Math.min(belt, rail) - transport) < 0.5)
    assert.ok(Math.abs(res.obj! - (Q * W_RAW + transport)) < 0.5)
    if (mode === 'rail') {
      assert.deepEqual(s!.hubs, ['A', 'B']); assert.equal(s!.belt_count, 0); assert.equal(cars, 21)
      assert.equal(s!.lines[0].dirs['A->B'].cars, 21)
    } else { assert.deepEqual(s!.hubs, []); assert.equal(s!.belt_count, 1) }
  }
})

test('break-even table (geo_breakeven.py, report §8.5): full Mk.5 belts above which rail pays', () => {
  const { meta } = build(D, HMF, solve, { rail: false, oe: OE })
  const want: Record<number, (number | null)[]> = { 100: [null, 31, 14, 8, 6, 5], 200: [null, 11, 7, 5, 4, 4], 500: [null, 11, 7, 4, 4, 3] }
  for (const [st, row] of Object.entries(want)) {
    const got = [1, 2, 3, 5, 7, 10].map(km => thresholdBelts(meta, +st, km * 1000))
    assert.deepEqual(got, row, `stack ${st}`)
  }
})

test('checker catches a broken solution', () => {
  const { res, m, idx, meta } = run(D, HMF, {}, 'conservation p free')
  assert.equal(res.status, 'Optimal')
  assert.ok(Math.abs(meta.check_obj! - res.obj!) < 1e-3 * Math.max(1, res.obj!))
  const bad = Float64Array.from(res.x!)
  const used = [...idx.e.values()].filter(j => bad[j] > 1)
  bad[used[0]] = 0
  assert.throws(() => check(D, m, idx, meta, bad), (e: unknown) => e instanceof CheckError && e.info[0] === 'balance')
})

test('p = 0 with rail variables equals rail disabled', () => {
  const a = run(D, HMF, { p: 0, gap: 1e-6 }, 'p=0'), b = run(D, HMF, { rail: false, gap: 1e-6 }, 'rail disabled')
  assert.equal(a.res.status, 'Optimal'); assert.equal(b.res.status, 'Optimal')
  assert.ok(Math.abs(a.res.obj! - b.res.obj!) <= 1e-6 * b.res.obj! + 1e-6)
  assert.deepEqual(a.s!.hubs, []); assert.equal(a.s!.rail_flow_items_per_min, 0)
  assert.ok(a.m.c.length > b.m.c.length)
})

test('deterministic: identical solves give identical summaries', () => {
  const a = run(D, HMF, { p: 2 }, 'determinism 1'), b = run(D, HMF, { p: 2 }, 'determinism 2')
  assert.equal(JSON.stringify(a.s), JSON.stringify(b.s))
  assert.equal(a.res.obj, b.res.obj)
})

test('stress: time-limited incumbent passes the checker (120 HMF at Sm3_0, 30 s)', () => {
  const { res } = run(D, { Desc_ModularFrameHeavy_C: 120 }, { base: 'Sm3_0', timeLimit: 30 }, 'stress 120 HMF at Sm3_0')
  assert.ok(res.x) // run() already called check()
})

// every Python reference case, same options; tier6_hmf hit the 180 s limit in Python, so only its checker is compared
const KW: Record<string, string> = { belt_mult: 'beltMult', base: 'base', p: 'p', rail: 'rail', fmult: 'fmult', tier: 'tier' }
for (const [name, c] of Object.entries(ref.cases as Record<string, Json>)) {
  test(`equals Python: ${name}`, () => {
    const o: BuildOptions & { gap?: number; timeLimit?: number } = {}
    for (const [k, v] of Object.entries(c.kw)) if (k === 'gap') o.gap = v as number; else (o as Json)[KW[k]] = v
    if (c.status !== 'Optimal') o.timeLimit = 30
    const d = name.startsWith('two_site') ? twoSite(5000).d : D
    const { res, m, s } = run(d, c.targets, o, `python case ${name}`)
    assert.equal(m.c.length, c.n_vars); assert.equal(m.integ.filter(Boolean).length, c.n_int); assert.equal(m.rows.length, c.n_rows)
    if (c.status !== 'Optimal') { assert.ok(res.x); return }
    assert.equal(res.status, 'Optimal')
    const tol = Math.max(res.gap, c.gap, 1e-6) * Math.abs(c.obj) + 1e-6
    assert.ok(Math.abs(res.obj! - c.obj) <= tol, `obj ${res.obj} vs Python ${c.obj} (tol ${tol})`)
    assert.deepEqual(s!.hubs, c.hubs, 'hubs')
    assert.deepEqual(s!.lines.map(l => ({ a: l.a, b: l.b, cars: Object.fromEntries(Object.entries(l.dirs).map(([k, v]) => [k, v.cars])) })), c.lines, 'lines and cars')
  })
}
test('p sweep equals the Python sweep p_sweep_plate12k.json (12 000 plate at Sm3_0), and resumes after a skipped step', () => {
  const py = JSON.parse(fs.readFileSync(new URL('../../../docs/research/satisfactory-geo/_过程文件/p_sweep_plate12k.json', import.meta.url), 'utf8'))
  const geo = { sitesDoc, nodes, transport: tr }
  const o = { targets: { Desc_IronPlate_C: 12000 }, base: 'Sm3_0', tier: 8, workKm: 0, pmax: 3, timeLimit: 60, recipeIds: null }
  const rows: SweepRow[] = []
  runSweep(game, geo, solve, o, e => { if (e.type === 'row') rows.push(e.row); if (e.type === 'error') throw new Error(e.message) })
  assert.deepEqual(rows.map(r => `${r.fmult}|${r.p}`), ['1|0', '1|1', '1|2', '1|3', '0.5|3', '2|3'])
  for (const r of rows) {
    const want = py.rows.find((x: Json) => x.fmult === r.fmult && x.p === r.p)
    assert.equal(r.status, 'Optimal')
    assert.ok(Math.abs(r.obj! - want.obj) <= 1e-4 * want.obj, `${r.fmult}|${r.p}: ${r.obj} vs ${want.obj}`)
    if (r.fmult === 1) assert.ok(Math.abs(r.envelope! - want.envelope) <= 1e-4 * want.envelope)
    assert.deepEqual(r.hubs, want.hubs)
  }
  assert.ok(rows[1].copied)
  // resume: steps 1|0, 1|1 done elsewhere, envelope carried in
  const rest: SweepRow[] = []
  runSweep(game, geo, solve, { ...o, fmults: [], skip: ['1|0', '1|1'], envStart: rows[0].obj }, e => { if (e.type === 'row') rest.push(e.row) })
  assert.deepEqual(rest.map(r => r.p), [2, 3])
  assert.ok(Math.abs(rest[1].envelope! - rows[3].envelope!) <= 1e-4 * rows[3].envelope!)
})
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Json = any

// ---------- review 6: what the Map tab adds around the Python model ----------
const GEO = { sitesDoc, nodes, transport: tr }
function sweepRun(o: Partial<SweepOptions> & { targets: Record<string, number> }, s: LpSolve = solve) {
  const rows: SweepRow[] = [], sums: (Summary | null)[] = []
  let error: string | null = null, done = false, meta: SweepMeta | null = null
  let best: Record<string, number> | null = null, bestObj: number | null = null
  runSweep(game, GEO, s, { base: 'S0_m1', tier: 8, workKm: 0, pmax: 2, timeLimit: 30, recipeIds: null, ...o }, e => {
    if (e.type === 'row') { rows.push(e.row); sums.push(e.summary) }
    if (e.type === 'error') error = e.message
    if (e.type === 'done') { done = true; best = e.best ?? null; bestObj = e.bestObj ?? null }
    if (e.type === 'meta') meta = e.meta
  })
  return { rows, sums, error: error as string | null, done, meta: meta as SweepMeta | null, best: best as Record<string, number> | null, bestObj: bestObj as number | null }
}

test('fmt keeps integer zeros at 0 decimals (9.7 s is "10", 57040.3 OE is "57040")', async () => {
  const { fmt } = await import('../fmt.ts')
  assert.equal(fmt(9.7, 0), '10'); assert.equal(fmt(57040.3, 0), '57040'); assert.equal(fmt(0.3, 0), '0')
  assert.equal(fmt(1.5, 1), '1.5'); assert.equal(fmt(2.0000001, 2), '2'); assert.equal(fmt(10.04, 1), '10')
})

test('supplied inputs are taken at the base; without them the sweep names what nothing makes', () => {
  const rec = recipeClosure(D, { Desc_Motor_C: 10 }).filter(r => !r.out.some(([k]) => k === 'Desc_Rotor_C')).map(r => r.id)
  const names = { Desc_Rotor_C: 'Rotor' }
  const bad = sweepRun({ targets: { Desc_Motor_C: 10 }, recipeIds: rec, names })
  assert.equal(bad.rows.length, 0)
  assert.match(bad.error!, /Nothing on the map makes Rotor with this plan's recipes/)
  const ok = sweepRun({ targets: { Desc_Motor_C: 10 }, recipeIds: rec, supplied: { Desc_Rotor_C: 20.00002 }, fmults: [] })
  assert.equal(ok.error, null); assert.ok(ok.done)
  assert.equal(ok.rows[0].status, 'Optimal')
  assert.ok(Math.abs(ok.sums[0]!.supplied!.Desc_Rotor_C - 20) < 1e-3) // 5 motor machines x 4 rotors/min ... 10 motors need 20 rotors
  // less than the plan needs: infeasible, reported, no further solves
  const short = sweepRun({ targets: { Desc_Motor_C: 10 }, recipeIds: rec, supplied: { Desc_Rotor_C: 5 } })
  assert.ok(short.error && short.rows.length === 0)
})

test('a plan the nodes cannot supply stops at once with the shortfall, not with "No plan found"', () => {
  const r = sweepRun({ targets: { Desc_IronPlate_C: 2e6 }, names: { Desc_OreIron_C: 'Iron Ore' } })
  assert.equal(r.rows.length, 0); assert.ok(!r.done)
  assert.match(r.error!, /cannot supply this plan at tier 8: Iron Ore 3000000\/min needed, \d+\/min from all nodes/)
})

test('no rail at tier 4: one solve is the whole sweep', () => {
  const r = sweepRun({ targets: { Desc_IronPlate_C: 600 }, tier: 4, pmax: 4 })
  assert.equal(r.error, null); assert.ok(r.done)
  assert.deepEqual(r.rows.map(x => `${x.fmult}|${x.p}`), ['1|0'])
  assert.equal(r.meta!.rail, false)
})

test('a solution that fails the re-check is dropped and the sweep goes on', () => {
  let n = 0
  const flaky: LpSolve = (m, o) => {
    const res = solve(m, o)
    if (m.integ.some(Boolean) && n++ === 0 && res.x) res.x.fill(0) // first MILP answer: nothing built, demand unmet
    return res
  }
  const r = sweepRun({ targets: { Desc_IronPlate_C: 600 } }, flaky)
  assert.equal(r.error, null); assert.ok(r.done)
  assert.deepEqual(r.rows.map(x => `${x.fmult}|${x.p}`), ['1|0', '1|1', '1|2', '0.5|2', '2|2'])
  assert.equal(r.rows[0].status, 'rejected'); assert.equal(r.rows[0].obj, null); assert.equal(r.sums[0], null)
  assert.equal(r.rows[2].status, 'Optimal'); assert.ok(r.rows[2].obj! > 0)
})

test('meta.F is the 1× hub cost also when a restarted sweep begins at F 0.5×', () => {
  const r = sweepRun({ targets: { Desc_IronPlate_C: 600 }, skip: ['1|0', '1|1', '1|2'] })
  assert.deepEqual(r.rows.map(x => `${x.fmult}|${x.p}`), ['0.5|2', '2|2'])
  assert.ok(Math.abs(r.meta!.F - 3490) < 1e-6)
  assert.ok(Math.abs(r.rows[0].F - 1745) < 1e-6)
})

test('tech tier limits the recipes (report G9): plastic is not made at tier 2, but is at tier 5', () => {
  // the recipe set a tier unlocks comes from the WASM (speedrun.rs recipes_up_to_tier); here: everything but
  // the plastic recipes at "tier 2", everything at "tier 5"
  const all = D.recipes.map(r => r.id)
  const t2 = all.filter(id => !/Plastic/.test(id))
  const low = sweepRun({ targets: { Desc_Plastic_C: 20 }, recipeIds: null, tierRecipes: t2, tier: 5, fmults: [] })
  assert.equal(low.rows.length, 0)
  assert.match(low.error!, /Nothing on the map makes .* only the recipes unlocked by then count/)
  const high = sweepRun({ targets: { Desc_Plastic_C: 20 }, recipeIds: null, tierRecipes: all, tier: 5, fmults: [] })
  assert.equal(high.error, null); assert.ok(high.done)
})

test('multi-period (G9): what an earlier period built is free; growing the same plan costs only the extra', () => {
  const small = sweepRun({ targets: { Desc_IronPlate_C: 60 }, tier: 4, fmults: [] })
  assert.equal(small.error, null); assert.ok(small.best && small.bestObj! > 0)
  // the same plan again on top of itself: nothing new to build
  const again = sweepRun({ targets: { Desc_IronPlate_C: 60 }, tier: 4, fmults: [], existing: small.best })
  assert.ok(again.bestObj! <= 1e-6 * small.bestObj!, `${again.bestObj} vs ${small.bestObj}`)
  // growing to 120/min later costs less than building 120/min from scratch
  const fresh = sweepRun({ targets: { Desc_IronPlate_C: 120 }, tier: 6, fmults: [] })
  const grow = sweepRun({ targets: { Desc_IronPlate_C: 120 }, tier: 6, fmults: [], existing: small.best })
  assert.ok(grow.bestObj! < fresh.bestObj!, `${grow.bestObj} vs ${fresh.bestObj}`)
})

test('world variants (G10): all pure = pure rate × nodes; a node switched off supplies nothing', async () => {
  const { editNodes } = await import('./model.ts')
  const nodes = GEO.nodes
  const total = (ns: GeoNode[], item: string, tier: number) => {
    const d = new Data(game, GEO.sitesDoc, ns, GEO.transport, D.hubs)
    let q = 0
    for (const s of d.sid) q += d.supply(s, tier, 780, 600).get(item) ?? 0
    return q
  }
  const iron = nodes.filter(n => n.item === 'Desc_OreIron_C' && n.type === 'node')
  const pure = editNodes(nodes, 'pure')
  const want = iron.length * nodeRate({ type: 'node', item: 'Desc_OreIron_C', purity: 'pure' }, 8, 780, 600)
  assert.ok(Math.abs(total(pure, 'Desc_OreIron_C', 8) - want) < 1e-6, `${total(pure, 'Desc_OreIron_C', 8)} vs ${want}`)
  assert.ok(total(pure, 'Desc_OreIron_C', 8) >= total(nodes, 'Desc_OreIron_C', 8))
  const off = editNodes(nodes, 'game', { [iron[0].id]: 'off' })
  const one = nodeRate(iron[0], 8, 780, 600)
  assert.ok(Math.abs(total(nodes, 'Desc_OreIron_C', 8) - total(off, 'Desc_OreIron_C', 8) - one) < 1e-6)
})
