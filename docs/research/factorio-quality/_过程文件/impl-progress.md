# impl-progress — Factorio quality in the planner (2026-11-21)

Spec: `../00-最终报告.md` §5. Branch `refactor`, not pushed.

## Step 0a — read docs and code (done)
- Read report, quality_lp.py, fable-review Resolution, PROGRESS.md, src/planner/{data,rules,lp,mod}.rs, converter.
- Fetched FactorioLab @50d8e977 (2026-11-21) `2x1` (SA 2.1.25), `2q1` (base+quality+recycler 2.1.25), `spa` (SA 2.0.77).
- Design: item ids `x@q` (q=1..4; normal keeps the bare id, so resources/weights stay valid); recipe `r@i`;
  base-level pruning (with recycling scope) before quality expansion; per-mode output vectors; recycling = recipe
  with flag (scrap-recycling is NOT reverse recycling: nobody crafts scrap); quality plans solved as LP.
- Next: converter (datasets), data.rs fields.

## Step 1+2 — data sets, quality expansion, oracle tests (done)
- Converter: `tools/convert_factoriolab.py factorio [2.0|2x1|2q1|spa]` → data/factorio{,-2x1,-2q1,-spa}.json;
  recipe `recycling` (reverse/self only; scrap recycling stays a production recipe), `quality_allowed`, `catalyst`;
  building `base_productivity`; module `by_quality` (qualityRecord); top-level `qualities`, `dataset`.
- Rust: data.rs (fields, `for_set`, `item_name` with ★tier), new quality.rs (split/qid/qrow, recycling scope
  chain|all|list + recycle_allow, base-level pruning then `r@i` expansion, loadout modes kQ (N−k)P + plain),
  rules.rs (Mode.outputs, `rules::outputs` with catalyst fix, base/research productivity, module_quality tier),
  lp.rs (Unit ins/outs, index-based relevant(), one-pass balance rows, item recycling excluded in normal plans,
  quality plans solved as LP + warning, Plan.columns/rows/quality), mod.rs (settings).
- Tests (src/planner/quality_tests.rs) pass: gear 79.8786 (spa, 1e-5 rel), 2.1 values 78.40, self-recycle 2726.9,
  wiki yields EM 2.722332 / cryo 4.835199 / AM3 0.583017 / chem 0.341205 / foundry 1.608046 %, ratio 7.0855 & 5.03,
  LDS shuffle 1.000 (research 15) and 0.048 (research 5), scope test, catalyst (Kovarex), normal plans unchanged.
- Finding: script I's "116.05 sets" for 4Q0P assumes 4 P3 at the legendary stage; with no productivity there
  (recipes that refuse it) the same closed form gives 154.7238 (test uses that).
- Next: scale report, every-item check on 2x1, wasm API + UI.

## Step 2b — data fixes found by the every-item check (done)
- 2x1/2q1/spa: items only "made" by item recycling (yumako, wood …) now count as raw drops; plant recipes
  (yumako/jellynut/tree) run in a synthetic agricultural tower (100 kW, no slots); asteroid-field location
  recipes make chunks raw (weight 1). `every_game_solves_its_items` now covers 2x1/2q1/spa: all pass.
- Scale (native release, 2x1, Resources objective, LP): gear@4 chain 416 columns / 112 items 40 ms; all-recycling
  16 578 columns / 1 346 items 79.7 s; processing-unit@4 chain 2 805 cols 0.49 s, all 136.5 s. → "all" is not
  usable in the browser with microlp; chain stays default; quality plans are LP (integer machines off).
- Next: wasm API (done in src/wasm_api.rs: game_data_json(game, dataset)), UI.

## Step 3 — UI + browser checks (in progress)
- solver.ts (types, DATASETS, qsplit/withQ/itemName/isLiquid, loadSolver(game, dataset)), App.tsx (reload on
  dataset), Icon.tsx (quality pip on icons; pip alone without icon), index.css (.qpip tier colours), Sidebar.tsx
  (Game data version select, per-row quality select on targets/inputs, Quality section: module quality, quality /
  other slot modules, recycling scope chain|list|all, allow-list chips + picker), Results/FlowGraph (quality ids).
- Asteroid chunks weight 100 (weight 1/25 made the Raw objective mine asteroids for iron).
- WASM (planner-dist): legendary gear 30/min 416 cols 44 ms → 652.7 iron ore + 13 calcite (foundry casting with
  3Q1P + recycling); legendary processing unit 10/min 2857 cols 53 ms. Normal Factorio / DSP compute (110 bld,
  287.7 MW, 189 units) / Satisfactory RIP (78 MW, 120 ore) unchanged. Native regression vs 60a2427 on 10 plans:
  identical except rocket part with prod modules (coal liquefaction catalyst fix: coal 785.7 → 823.1).
- Next: WASM timing of "all" scope, commit UI, PROGRESS.md.
- "Everything" recycling scope in WASM (gear@4, 2x1): 16 689 columns / 1 351 items, 190.6 s (browser pane hidden,
  may be throttled) vs 79.7 s native → keep "Target chain" default; UI hint warns.
- cargo test: 37 pass (2 ignored: scale report, DSP compute report). tsc + vite build clean.

## Remaining (not in this pass)
- Report §5 step 6: shared-machine integer constraint for tier machines; module cost in CostWeights; machine
  quality (speed ×(1+0.3·level)). Step 8 beacons. Step 3 quality mining (mining as recipes).
- Planet/location filter: Space Age plans may use Gleba agriculture / bacteria freely under Raw (plants cost 0).
- UI for research productivity (setting `research_productivity` exists, used by the LDS test); switch to stop
  recycling at a tier ("legendary not recycled" is implicit: the LP never chooses it unless useful).
- Catalyst share in quality rolls is assumed to roll with the rest (report §7 存疑).
