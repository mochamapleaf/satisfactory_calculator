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

## 2026-11-19 · Steps 4–6 — UI, graph view, deploy pipeline
- `react-frontend/` rewritten (React 19 + Vite 7, @xyflow/react + dagre): targets with rates, objective
  (resources / buildings / power), global clock, alternates (all / per recipe), Converter ore conversion
  toggle (off by default — scarcity weighting otherwise picks ~250 MW converters), available inputs,
  editable resource limits, per-recipe Somersloops and "exclude recipe" in the plan table, building counts,
  production + extraction power, belt/pipe tier per raw/output line, flow graph (LR dagre layout,
  pipes blue, edge width ∝ rate). Settings persist in the URL hash (shareable) and localStorage.
- Checked in the browser (dev + `vite preview` of the production build): 10 RIP/min → 4/3/2/3/2 machines,
  120 ore, 78 MW; RIP + Turbo Motor with all alternates solves instantly, graph renders (44 nodes, 81 edges).
- `.github/workflows/build-site.yml` replaced (trunk/Yew → cargo test + wasm-pack + Vite → Pages
  artifact v3 / deploy-pages v4). **Not pushed** — the decision desk must be told before pushing to GitHub.
  Repo Settings → Pages must be set to "GitHub Actions" for the deploy job.
- Open follow-ups: shadow prices (needs a solver with duals, e.g. replace minilp or compute via
  sensitivity), power generators as recipes (fuel → MW), per-recipe clock speeds, item icons.
