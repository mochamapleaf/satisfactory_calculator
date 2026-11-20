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

## 2026-11-20 — Integer (MILP) solver and custom cost function
Feedback: "LP then ceil" is not a real discrete solution; wants an exact whole-building
optimum and a user-defined cost function.
- Solver switched to `microlp` 0.8 (pure Rust MILP, branch-and-bound, LP duals, time limits
  that work in WASM). `src/planner/lp.rs` model per recipe: work x, buildings n (int),
  shards s (int), power p. Exact clock/shard rules (x ≤ n + 0.5 s, x ≤ c_max n, s ≤ 3n).
  Power = perspective of a piecewise-linear c^1.32 (linear, exact at breakpoints); reported
  power is recomputed exactly from the chosen clocks (`machine_groups`).
- Cost function `CostWeights`: resources (per-item weights, default scarcity Iron Ore = 1),
  buildings (per-building-type weights), MW, shards, byproducts. Presets raw / buildings /
  power, or custom. Optional shard budget, max clock (replaces the fixed global clock).
- LP relaxation first: lower bound, item shadow prices (Item values tab), warm start.
  Primal heuristic: re-weighted LPs (ceil(n)/n building cost) collect a recipe
  neighbourhood → restricted MILP → warm-starts the full MILP. Time limit (default 4 s)
  returns the best plan with its gap.
- Benchmark (release, 7.3/min): most cases optimal or within 0.01 %; Buildings objective
  with all alternates on big chains stays 10–25 % from the bound at 4 s (HMF: 21 buildings,
  best seen 20).
- UI: solving in a Web Worker; cost-function editor; whole-building table with clock groups
  and shards; solve status card (optimal / gap, ms); Item values tab.
- 16 tests pass. Preview: private claude.ai artifact 9EkeeHce4wpgBA3CtXg9QU (v2).

## 2026-11-20 — Multi-game direction (Satisfactory + DSP + Factorio)
- Core refactor: recipes run in per-game *modes* (`src/planner/rules.rs`); Satisfactory
  Somersloop levels are modes with automatic placement under a budget (c3aa51f).
- Survey of existing calculators: `docs/research/calculators-survey.md`. Key points:
  FactorioLab (MIT) covers 14 games incl. all three with GLPK LP; its per-game data JSON is
  the fastest data source. No other tool does whole-building MILP. DSP 量化计算器
  (dsp-calc, MulanPSL-2) and 混带 tool dspqc (GPL-3) define the DSP-specific calculations.
- Next: critic pass on the survey; DSP + Factorio data from FactorioLab; DSP proliferator
  and Factorio module/beacon modes; transcribe 笑笑meow videos (needs Bilibili sign-in).
- Survey critic pass done (review in `docs/research/_过程文件/`); fixes applied to the survey.
  Design-relevant: Factorio module values per quality must come from data (game rounds
  down); beacons use the data's `profile` table; GaiusJ/DSPCalculator have no licence (no
  code reuse); SatisfactoryTools images not reusable. **FactorioLab ships Satisfactory 1.2
  data — ours is 1.0: check whether 1.2 changed production numbers.**
- DSP + Factorio data and modes landed (efd043b); UI game switcher in progress.
