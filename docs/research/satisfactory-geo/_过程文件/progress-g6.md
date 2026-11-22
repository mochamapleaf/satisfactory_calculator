# G6 progress: siting model + map view in the web app

Branch `worktree-agent-ac4b893018efffcdc` (reset to `refactor` a243811 at start; the worktree had been cut from master).

## 2026-11-22 step 0: plan
- Read: report §7-§9 (incl. §8.5), tools/geo_solve.py, geo_sites.py, geo_breakeven.py, geo_solve_test.py, impl-progress.md,
  fable-review-3 geo Resolution, PROGRESS.md, highsEngine.ts / worker.ts.
- Design: TypeScript port of geo_solve.py (`react-frontend/src/geo/model.ts`: node_rate, rtd, platform_cap, OE LP,
  recipe closure, build, aggregate_gross, check, summarize, haul_costs / threshold_belts, hub rule of geo_sites.py),
  solved by the same highs-js package (LP text) in its own worker (`geo/geoWorker.ts`); map tab `MapView.tsx`.
- Reference numbers: `tools/geo_reference_dump.py` runs the geo_solve_test cases + 12 000 plate at Sm3_0 + a mixed
  late-game target with highspy and writes `react-frontend/src/geo/python-ref.json`; `geo/geo.test.ts` (node) compares.
- Started: npm ci + wasm-pack build in the worktree; python reference dump (background).
- Next: model.ts.

## 2026-11-22 step 1: model port + tests (done)
- Files: `react-frontend/src/geo/model.ts` (port of geo_solve.py build/aggregate_gross/ore_equivalents/check/summarize,
  geo_breakeven haul_costs/threshold_belts, geo_sites hub rule), `geo/highsLp.ts` (highs-js 1.15.4 via arrays/CSC,
  same options as geo_solve.solve_lp; `threads` is not an option there, it is single-threaded),
  `tools/geo_reference_dump.py` -> `geo/python-ref.json` (13 cases, highspy, 274 s), `geo/geo.test.ts` (node --test).
- Tests: `npm run test:geo` -> 0 fail. All geo_solve_test.py cases ported (formulas, OE, tier supply, hand machine counts,
  two-site closed form, checker, p=0 == rail off, determinism, stress), plus break-even table (report §8.5) and
  hubsFor(S0_m1) == sites.json hubs. Every Python case: same vars/ints/rows, same objective within gap, same hubs and
  cars per line. 12 000 plate at Sm3_0: 95 123.63 (rail, Sm1_2+Sm3_0, 5 cars) vs 99 386.19 belts = -4.3 %, as in Python.
  late_mix (10 HMF + 5 computer + 10 motor + 20 encased beam at S0_m1): 56 609.42 both, 40 s in node WASM (Python 21 s).
- Next: sweep + worker + MapView.
