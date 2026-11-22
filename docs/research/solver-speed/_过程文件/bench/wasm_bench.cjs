// node wasm_bench.cjs <pkg dir>   -- times solve_json (WASM) on fixed scenarios; prints plan.solve_ms
const path = require('path')
const pkg = require(path.resolve(process.argv[2], 'satisfactory_calculator.js'))
const T = (o) => Object.entries(o).map(([item, rate]) => ({ item, rate }))
const sci6 = { 'automation-science-pack': 60, 'logistic-science-pack': 60, 'military-science-pack': 60, 'chemical-science-pack': 60, 'production-science-pack': 60, 'utility-science-pack': 60 }
const df = ['df-negentropy-smelter', 'df-recomposing-assembler', 'df-self-evolution-lab']
const pall = ['proliferator-1-products', 'proliferator-2-products', 'proliferator-3-products', 'proliferator-1-speed', 'proliferator-2-speed', 'proliferator-3-speed']
const cases = {
  // proof-complete MILPs (same node count native / wasm -> pure speed ratio)
  sf_rip10_alt_bld: { game: 'satisfactory', targets: T({ 'Reinforced Iron Plate': 10 }), allow_all_alternates: true, objective: 'buildings' },
  sf_rip10_alt_pow: { game: 'satisfactory', targets: T({ 'Reinforced Iron Plate': 10 }), allow_all_alternates: true, objective: 'power' },
  sf_hmf7_alt_pow: { game: 'satisfactory', targets: T({ 'Heavy Modular Frame': 7.3 }), allow_all_alternates: true, objective: 'power' },
  // LP only (integer off)
  lp_fx_sci6_mod_b4_pow: { game: 'factorio', dataset: '2x1', targets: T(sci6), objective: 'power', integer: false, effects: ['productivity-module-3', 'speed-module-3'], beacon_module: 'speed-module-3', beacon_counts: [2, 4, 8, 12] },
  lp_dsp_multi_pall_cmp: { game: 'dsp', targets: T({ 'universe-matrix': 60, 'small-carrier-rocket': 5, 'solar-sail': 60, 'antimatter-fuel-rod': 10 }), objective: 'compute', integer: false, effects: pall, disabled_buildings: df },
  // time-limited MILP: nodes solved in 4 s cannot be read from the plan; gap is
  dsp_um60_plain_bld_4s: { game: 'dsp', targets: T({ 'universe-matrix': 60 }), objective: 'buildings', disabled_buildings: df },
}
for (const [name, s] of Object.entries(cases)) {
  const times = []
  let plan
  for (let i = 0; i < 3; i++) {
    plan = JSON.parse(pkg.solve_json(JSON.stringify(s)))
    times.push(plan.solve_ms)
  }
  times.sort((a, b) => a - b)
  console.log(JSON.stringify({ name, wasm_ms: +times[0].toFixed(1), median_ms: +times[1].toFixed(1), status: plan.status, objective: plan.objective, proven: plan.proven_optimal, gap: plan.gap, columns: plan.columns }))
}
