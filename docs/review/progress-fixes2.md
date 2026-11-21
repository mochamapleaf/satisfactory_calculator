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
