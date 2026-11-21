# Compute (UPS) objective — implementation progress

## 2026-11-21 step 1: read
- Read 00-最终报告.md §4 (objective: n_r·(c_bld + aux_r) + c_gen·Σp; aux_r = sorter·k_r + belt 0.03 + station 0.1 + coater share; logistics per machine, NOT per flow — §4.1 explains why), model.json, lp.rs/rules.rs/mod.rs, Sidebar/Results/solver.ts.
- Plan: Objective::Compute; CostWeights gets sorter / per_machine / coater weights; Mode gets ports + sprayed; lp adds aux to wb; coefficients read from docs/research/dsp-ups/model.json via include_str; PlanSettings.generator + mixed_belts; Plan.compute summary + PlanRecipe.compute.
- Next: Rust implementation + tests.

## step 2: Rust done
- New src/planner/compute.rs (include_str! of docs/research/dsp-ups/model.json; ComputeModel; weights(generator); sorters_per_machine; machine_cost).
- mod.rs: Objective::Compute; CostWeights.sorter/per_machine/coater; PlanSettings.generator ("artificial_star") + mixed_belts; PlanRecipe.compute; Plan.compute: ComputeSummary (filled for every DSP plan, any objective, so presets compare).
- rules.rs: Mode.sprayed. lp.rs: wb = w.buildings·machine_cost(...)·scale; finish() fills compute parts.
- Tests: 4 new (model coefficients, hand-computed 120 ingot/min = 1+0.24+0.13+0.0056·1.44, top tier + mixed belts, per-recipe proliferator modes universe matrix + wind flips to fewer sprayed) + 1 ignored report test. cargo test all pass.
- 60/min universe matrix MILP 20 s: Compute and Buildings presets give the same plan (110 buildings, compute 189.4 units ≈ 95 µs/tick, gap ~14 % not proven).
- Next: frontend.

## step 3: frontend done, builds
- wasm_api.rs: compute_weights_json(generator). solver.ts: Objective 'compute', CostWeights.sorter/per_machine/coater, Generator + GENERATORS, Settings.generator/mixed_belts, ComputeSummary, PlanRecipe.compute.
- Sidebar.tsx: 算力/UPS preset only for DSP, generator select, mixed-belt checkbox, hint + link to enable Mk.III modes; CostEditor DSP rows. Results.tsx: compute units card + CPU µs/tick card (range), 算力 % column. urlState: compute → buildings for other games.
- wasm-pack, tsc -b, vite build OK.
- Next: browser check (planner-dist, port 4173).

## step 4: browser check + docs
- planner-dist (4173): DSP universe matrix 60/min, Compute vs Buildings identical (110 bld, 189 units, ≈95 µs 38–189); wind: Compute 127 bld / 311 units vs Buildings 332. Light + dark, 375 px: no page overflow; µ kept lower-case in uppercase card labels (.nocase); sidebar preset buttons narrowed to fit 5. Satisfactory shows no Compute button. No console errors.
- coater/per_machine rounded (0.1×0.2 float noise). PROGRESS.md entry added.
- Next: commit.
