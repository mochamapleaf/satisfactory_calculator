# Progress log — Satisfactory planner (2026-11 restart)

memo project: `satisfactory-calculator` (e3ea2a723535). Branch: `refactor`.

## 2026-11-19 · Step 1 — inventory / merge
- `_from-mac-2026-10-29/` holds only empty `react-frontend/` and `satisfactory-calculator-react/` dirs:
  nothing newer than the G: tree. `G:\code\_review\git-sync-conflicts-20261025\satisfactory_calculator`
  is empty too. Nothing to merge.
- Uncommitted 2025-08 work (Kiro React/WASM migration: `wasm_*`, `data_management.rs`, React scaffolds)
  committed as-is in d035712 so it isn't lost.
- Both React folders were Vite templates; `react-frontend/` is the one wired to wasm-pack and is kept.
  `satisfactory-calculator-react/` is a duplicate template (left in place, unused).
- Kiro modules + Yew front end + trunk `index.html` moved to `legacy/` (kept, not compiled).
- Toolchain: repo pins Rust 1.91.0 via `rust-toolchain.toml` (wasm-pack needs ≥1.88; global stays 1.85).

## 2026-11-19 · Step 2 — recipe data
- `data/game-data.json` from SatisfactoryTools data1.0.json (Docs.json-derived); see `data/SOURCE.md`.

## 2026-11-19 · Step 3 — solver
- `src/planner/matrix.rs`: readme incidence-matrix method (Aᵀx=b with input rows, Gaussian elimination).
- `src/planner/lp.rs`: LP generalisation (minilp): machines per recipe, raw extraction bounded by world
  limits, user supplies, free surplus for byproducts; objectives resources (1/limit weights) / buildings /
  power; alternates toggle; global clock (power ∝ clock^1.321928); Somersloops per recipe.
- `src/planner/flows.rs`: proportional producer→consumer flows for the graph view.
- Tests (`cargo test`, 13 pass): RIP chain = readme numbers (24 smelters, 720 ore) on both solvers; LP ==
  square matrix; two screw recipes → matrix error; plastic byproduct (90 oil, 30 HOR surplus); 250 % clock
  power; Somersloop ×2 output ×4 power; limits (Converter route when iron capped, infeasible without SAM);
  supplies; alternates never worse; HMF/Turbo Motor/Aluminum/FMF/Nuclear Pasta × 3 objectives × alts balance.
- `wasm-pack build --release --target web --out-dir react-frontend/src/wasm-pkg` works (368 KB wasm).
- Not done: shadow prices / duals (minilp doesn't expose them), power generators as recipes.
