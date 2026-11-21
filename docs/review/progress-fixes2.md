# Fixes for review 2 (docs/review/fable-review-2.md)
- 2026-11-21 start. Plan: H1, then H2, M1-M5, L1-L9; verify per group; commit per group.
- H1 done: rules.rs tiered_modes + quality.rs loadout_modes keep b.base_productivity on every recipe; research and
  prod modules only where allowed. Test builtin_productivity_on_all_recipes (modes, 120-belt foundry plan = 40 plates,
  quality loadouts). 
- M5 done: recycling_scope_and_full_data split into recycling_scope_chain_full_data (fast), recycling_scopes (small
  iron subset: chain/all/list/recycle_allow semantics, all <= chain objective; replaces the always-true ore assert,
  part of L3) and #[ignore] all_scope_full_data. cargo test -q: 39 passed, 3 ignored, 42 s (was 256 s).
- H2 in progress: Rust max_columns guard + Status::TooLarge (test column_limit_reports_size; model_sizes ignored test: chain plans <= 2 878 cols, gear 'all' 16 689); solver.ts terminates/restarts the worker when superseded or cancelled, stale results dropped; App banner 'Large model, not solved yet' + Solve anyway / Cancel; persisted 'all' now lands on the banner. Note: objectives_on_complex_chains failed once (Turbo Motor Power alts, Infeasible) then passed 4x: timing-based MILP flake in debug.
- H2 done + browser-verified: persisted 'all' (this machine's localStorage) lands on 'Large model, not solved yet' (16 689 cols); Solve anyway -> timer + Cancel; Cancel stops the worker; switching to Target chain mid-solve terminates it and shows the chain plan in < 2 s.
- M1 done: lp.rs warns "This data set has no quality" when a tier is asked on data without tiers; solver.ts
  DATASETS[].quality + withDataset() strips @q (merging rows) on switch and in urlState.upgrade.
- L4 done: upgrade() resets an unknown Factorio data set id to 2.0 (and drops a saved max_columns).
- M2 done: Plan.building_counts; quality plans round up per (base recipe, mode) group; Results uses it. Test
  quality_building_count_shares_tiers.
- M4 done: compute summary per-entity terms use ceil(n) in LP mode (power as solved). Test compute_counts_whole_entities_in_lp.
- L1 done: model.json fractionator sorters 0, labs sorter_share 0.1 (stack of 10, estimate); compute.rs sorter_share. Test compute_sorter_share.
- M3: objective label 算力 (title has UPS), aside .seg wraps instead of overflowing. Browser check pending.
- L2 done: "id@0" excludes only the normal tier (quality.rs expand + Results exclude). Test exclude_single_tiers.
- L3 done: tests for built-in productivity, normal-plan numbers vs base data, 2.1 gear status/balance, quality-path
  catalyst, quality_allowed=false, module tier in normal modes, scopes; objectives_on_complex_chains retries a
  timing-only timeout with 20 s.
- L5 done: picker 'taken' is per quality; quality change merges into an existing row.
- L6 done: compute card one decimal below 100, "(excl. mining, collectors)" + tooltip.
- L7 done: DSP generator + mixed belts shown for every objective, with a hint outside Compute.
- L8 done: catalyst roll comment + UI hint; normal module modes label "★legendary" and hold item@q.
- L9 done: converter drops unconsumed resources (heat) and machines without recipes; all 5 data files regenerated
  (only buildings / heat removed, checked by semantic diff).
- Builds: cargo test 45 passed / 4 ignored (38-137 s, the spread is objectives_on_complex_chains MILP timeouts under
  parallel load); wasm-pack, tsc -b, vite build pass.
- Browser (planner-dist): legendary gear 10/min chain = 20 buildings (recyclers 2); foundry 120 belts = 40 plates +
  40 gears; switch 2x1 -> 2.0 with [gear@4 10, gear 5] -> one row "Iron gear wheel 15", plan solves; link with dataset
  "2.1" -> 2.0 and solves; normal + legendary gear both listable, quality change merges rows; Everything -> banner,
  Solve anyway/Cancel/supersede; DSP presets one row at 1024 px (padding 4px), no overflow at 375 px; DSP generator +
  belts shown under Buildings; light + dark OK. localStorage restored to the saved Factorio state (gear@4 30, 2x1,
  recycling all), which now opens on the banner.
