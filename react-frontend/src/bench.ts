// Dev-only browser benchmark (bench.html): the report's scenarios (docs/research/solver-speed,
// bench.rs) solved through the real worker with a chosen engine. Driven by
// docs/research/solver-speed/_过程文件/bench/browser_bench.py via window.runBench.
import { solveStream, type Settings, type SolveUpdate } from './solver'

type S = Partial<Settings> & Record<string, unknown>
const T = (o: Record<string, number>) => Object.entries(o).map(([item, rate]) => ({ item, rate }))

function scenarios(): [string, S][] {
  const out: [string, S][] = []
  const objs: [string, string][] = [['res', 'resources'], ['bld', 'buildings'], ['pow', 'power']]
  const p4 = { 'Assembly Director System': 4, 'Magnetic Field Generator': 4, 'Thermal Propulsion Rocket': 2, 'Nuclear Pasta': 1 }
  const p5 = { 'Biochemical Sculptor': 2, 'Ballistic Warp Drive': 0.5, 'AI Expansion Server': 1, 'Nuclear Pasta': 1 }
  const sf: [string, Record<string, number>][] = [
    ['rip10', { 'Reinforced Iron Plate': 10 }], ['hmf7', { 'Heavy Modular Frame': 7.3 }],
    ['turbo_rip', { 'Reinforced Iron Plate': 10, 'Turbo Motor': 5 }], ['pasta', { 'Nuclear Pasta': 1 }], ['phase4', p4], ['phase5', p5],
  ]
  for (const [name, t] of sf) for (const alt of [false, true]) for (const [on, o] of objs)
    out.push([`sf_${name}_${alt ? 'alt' : 'std'}_${on}`, { game: 'satisfactory', targets: T(t), allow_all_alternates: alt, objective: o as Settings['objective'] }])
  for (const [on, o] of objs) {
    const s: S = { game: 'satisfactory', targets: T(p4), allow_all_alternates: true, objective: o as Settings['objective'] }
    out.push([`sf_phase4_oc250_${on}`, { ...s, max_clock_percent: 250 }])
    out.push([`sf_phase4_sloop20_${on}`, { ...s, auto_somersloops: true, somersloop_budget: 20 }])
  }
  const df = ['df-negentropy-smelter', 'df-recomposing-assembler', 'df-self-evolution-lab']
  const p3 = ['proliferator-3-speed', 'proliferator-3-products']
  const pall = ['proliferator-1-products', 'proliferator-2-products', 'proliferator-3-products', 'proliferator-1-speed', 'proliferator-2-speed', 'proliferator-3-speed']
  const dsp: [string, Record<string, number>][] = [
    ['um60', { 'universe-matrix': 60 }],
    ['multi', { 'universe-matrix': 60, 'small-carrier-rocket': 5, 'solar-sail': 60, 'antimatter-fuel-rod': 10 }],
  ]
  for (const [name, t] of dsp) for (const [en, eff] of [['plain', []], ['p3', p3], ['pall', pall]] as [string, string[]][])
    for (const [on, o] of [['res', 'resources'], ['bld', 'buildings'], ['cmp', 'compute']])
      out.push([`dsp_${name}_${en}_${on}`, { game: 'dsp', targets: T(t), objective: o as Settings['objective'], effects: eff, disabled_buildings: df }])
  const sci6 = { 'automation-science-pack': 60, 'logistic-science-pack': 60, 'military-science-pack': 60, 'chemical-science-pack': 60, 'production-science-pack': 60, 'utility-science-pack': 60 }
  const sci7 = { ...sci6, 'space-science-pack': 60 }
  const mods: [string, string[], string, number[]][] = [
    ['plain', [], '', [8]],
    ['mod', ['productivity-module-3', 'speed-module-3'], '', [8]],
    ['modall', ['productivity-module-3', 'speed-module-3', 'efficiency-module-3', 'productivity-module-2', 'speed-module-2'], '', [8]],
    ['mod_b8', ['productivity-module-3', 'speed-module-3'], 'speed-module-3', [8]],
    ['mod_b4', ['productivity-module-3', 'speed-module-3'], 'speed-module-3', [2, 4, 8, 12]],
  ]
  for (const [ds, dn] of [['', 'fa'], ['2x1', 'fx']]) for (const [name, t] of [['sci6', sci6], ['sci7', sci7]] as [string, Record<string, number>][])
    for (const [mn, eff, bm, bc] of mods) for (const [on, o] of objs) {
      if (dn === 'fx' && name === 'sci7') continue // Nauvis only: infeasible in the report too
      out.push([`${dn}_${name}_${mn}_${on}`, { game: 'factorio', dataset: ds, targets: T(t), objective: o as Settings['objective'], effects: eff, beacon_module: bm, beacon_counts: bc }])
    }
  return out
}

interface Row { name: string; engine: string; status: string; obj: number; proven: boolean; gap: number | null; first_ms: number | null; end_ms: number; highs_load_ms?: number; highs_status?: string; used?: string }

function runOne(name: string, s: S, engine: string, ms: number): Promise<Row> {
  return new Promise(resolve => {
    const t0 = performance.now()
    let first: number | null = null
    let info: SolveUpdate | null = null
    solveStream({ integer: true, ...s, engine, time_limit_ms: ms } as unknown as Settings, u => {
      if (u.plan && u.plan.status === 'optimal' && first === null) first = performance.now() - t0
      if (u.engine) info = u
      if (!u.done) return
      const p = u.plan
      resolve({
        name, engine, status: p?.status ?? '?', obj: p?.objective ?? 0, proven: !!p?.proven_optimal, gap: p?.gap ?? null,
        first_ms: first === null ? null : Math.round(first), end_ms: Math.round(performance.now() - t0),
        highs_load_ms: (info as SolveUpdate | null)?.highs_load_ms, highs_status: (info as SolveUpdate | null)?.highs_status, used: (info as SolveUpdate | null)?.engine,
      })
    })
  })
}

declare global { interface Window { runBench: (engine: string, ms: number, filter?: string) => Promise<Row[]>; benchNames: () => string[] } }
window.benchNames = () => scenarios().map(s => s[0])
window.runBench = async (engine, ms, filter = '') => {
  const rows: Row[] = []
  for (const [name, s] of scenarios()) {
    if (filter && !filter.split(',').some(f => name.includes(f))) continue
    const r = await runOne(name, s, engine, ms)
    rows.push(r)
    console.log('BENCH ' + JSON.stringify(r))
  }
  return rows
}
document.getElementById('out')!.textContent = 'ready'
