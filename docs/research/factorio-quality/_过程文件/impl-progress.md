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

## Step 4 — planet filter (done, 2026-11-21)
- Converter: FactorioLab `locations` on recipes and resources (union over a resource's mining recipes;
  asteroid chunks → space-platform), top-level `locations` (only when >1, i.e. Space Age); also emits
  `beacons` (profile, per-quality effectivity/MW) and `research` families (technology.recipeProductivity) for
  the next steps. Other data unchanged (checked: recipes/resources equal ignoring the new field; dsp.json same).
- Rust: `Recipe/Resource.locations`, `GameData.{locations,research,beacons}`, `GameData::located`;
  `PlanSettings.planets` (default ["nauvis"]); filter in `allowed_recipes` and raw-resource columns; on an
  infeasible plan a warning names the needed resources that are off-planet ("Tungsten ore (Vulcanus)").
  Supplies are never filtered (imports). Machines are not filtered (a biochamber may run on Nauvis).
- Test `planet_filter_space_age`: plastic 60/min on all planets = bioplastic from free plants; Nauvis only =
  coal liquefaction, no Gleba recipe, every raw on Nauvis; tungsten plate infeasible on Nauvis with hint,
  optimal with Vulcanus or with tungsten ore + calcite as supplies; no asteroid chunks without the platform.
  `every_game_solves_its_items` runs with all planets. Suite: 46 pass, 4 ignored.
- UI: Game data → Planets checkboxes (Space Age sets only) + hint. Browser (planner-dist, 2x1, plastic 60/min):
  Nauvis → coal liquefaction plan; +Gleba → bioplastic from agricultural towers, no raw input.
- Seen, not changed: heat exchanger runs without a heat input (heat dropped as an unused resource in step L9).
- Next: research productivity UI.

## Step 5 — research productivity UI (done)
- Data: `research` families from FactorioLab technology.recipeProductivity (2x1/spa: steel, LDS, plastic,
  rocket fuel, processing unit, scrap recycling, rocket part, asteroid crushing; base sets have none).
- UI: Sidebar "Research productivity" (Space Age sets): level per family → `research_productivity` for each
  of its recipes (level × 0.1); tooltip lists the recipes.
- Test `research_productivity_families`: 60 steel/min furnace-only = 300 plates; level 5 → 200; level 40 is
  capped (+300 %) → 75. Browser (2x1 steel 60/min, casting off): level 5 → 200 iron ore (was 300).
- Next: beacons.

## Step 6 — beacons (report §5 step 8, done)
- Data: `beacons` (effectivity 1.5, 2 slots, 480 kW, profile 1/√n table, per-quality effectivity/kW).
- Rust `rules.rs::beacon_options`: none + one option per count in `beacon_counts` when `beacon_module` is a
  speed/efficiency module: transmitted effect = n × effectivity(beacon_quality) × profile[n−1] × slots ×
  module effect (module_quality), speed modules' quality penalty included. Every Factorio mode (plain or
  module fill, and every quality loadout in `quality.rs`) × beacon option is a mode; only machines with
  module slots. Beacon MW and beacon buildings per machine (÷ `beacon_share`) go into the mode
  (`Mode.beacons`, power_base) and the building cost (`compute::machine_cost`, weight `beacon`); held beacon
  modules at their tier. Plan: `PlanRecipe.beacons`, "Beacon" row in building_counts (ceil of the total).
  Quality loadouts now hold `module@q` (tiered ids).
- Tests: `beacon_mode_values_by_hand` (2.0 circuit, AM3 4×P3 + 8 beacons×2 S3: speed 5.8025, ×1.4,
  974.82/min per machine, 7.64205 MW; plan = 1 machine + 8 beacons; sharing 2 halves beacon MW/count; no
  module / prod module → no beacon modes), `beacon_profile_and_quality` (12 beacons 5.1948; legendary beacon
  2.5 / 80 kW with legendary S3 = +625 %; penalty −7.5 % per normal beacon), `beacon_quality_penalty_in_loadouts`
  (2q1 gear 4×legendary Q3 + 1 beacon: q = 17.5 %, uncommon share 0.9 q; plan balances).
- UI: Modules section → beacon module select, beacons per machine (list, e.g. "4, 8, 12"), machines sharing
  a beacon, beacon quality. Browser (planner-dist, 2.0, circuit 974.82/min, P3 + 8×S3 beacons, LP): circuit
  1 machine 7.6 MW, cable 1.07 machines (2088.9/min = 240 × 5.8025 × 1.4 per machine), 17 beacons; furnaces
  stay without beacons (not worth it under Raw + building tie-break).
- Regression vs b34e62c (native release, 10 plans, `examples/regress.rs` kept in scratchpad): identical
  except (a) Satisfactory HMF Buildings MILP at a 3 s limit (same 30 buildings, other tie-break; time-limited
  under full CPU load from other jobs) and (b) 2x1 legendary gear on the new default Nauvis-only planets
  (no calcite → no foundry casting: 3095 ore vs 653). With all planets 2x1 processing unit@4 is identical.
- Note: the machine was at 100 % CPU from other sessions' jobs; vite builds took minutes, the hidden
  browser pane looked hung while the page was only slow.
- Next: module cost.

## Step 7 — module cost in the objective (report §5 step 6 part, done)
- `PlanSettings.module_costs` (per held module, by quality tier normal…legendary; shorter list repeats the
  last value; empty = free); `rules::module_cost` adds Σ held modules × cost (machine slots and beacon
  modules, Factorio module items only) to each building's objective coefficient under every objective.
- Tests: `module_cost_break_even` (60 gears/min from plates: prod saves 34.286 plates for 2.857 modules →
  break-even 12/module; cost 10 keeps P3 with objective 85.714 + 28.571, cost 15 drops modules → 120 plates),
  `module_cost_by_quality_tier` (gear@4 oracle 79.8786 with free modules; legendary-only price: objective =
  plates + 1000 × held modules, plates never below the oracle). Suite 52 pass, 4 ignored.
- UI: Modules section → "Charge for modules" (defaults 1/3/10/30/100 per tier, a rough guess, editable).
  Browser (2x1 Nauvis+Vulcanus, legendary gear 10/min, legendary modules): free → 20 buildings (10 AM3,
  8 foundries, 2 recyclers, as in review 2 M2); charged → 14 buildings, plain gear assembler, foundry gear
  casting with 4Q.

## Step 8 — integer machines shared across quality tiers (report §5 step 6): design only, not built
Why not built now: the change touches every per-column part of the MILP path in `lp.rs` (variables, warm
start `hint_of`, `restrict`, the re-weighting heuristic `scale`, `finish` rounding), and its speed could not
be measured reliably today (machine at 100 % CPU from other sessions; a 4 s MILP limit is already the binding
case for Satisfactory chains). Quality plans stay LP; `building_counts` already rounds up per shared group.
Design:
1. Group key g = (base recipe `split(r.id).0`, mode key). All columns of a group run on the same machine type
   and loadout (same `speed`, `power_base`), i.e. the same physical machines switching recipes (r@0 … r@3).
2. In `build_scaled`, when `quality_on && integer`: per column keep x_c continuous and drop its own n_c
   (or keep n_c continuous with cost 0 and x_c ≤ speed·n_c); add one integer N_g with the building weight
   (machine_cost + beacons + module cost) and the row Σ_{c∈g} x_c / speed ≤ N_g. Power stays per column
   (exponent 1, no overclock), so p_c ≥ P·x_c/speed is unchanged.
3. Warm start: N_g = ceil(Σ x_c/speed) from the LP (always feasible). Restricted MILP: keep groups whose
   columns the LP uses. `scale` re-weighting per group (ceil(N)/N).
4. `finish`: report per column the fractional share n_c = x_c/speed, and building_counts = Σ_g N_g (the
   current per-group ceil already gives this for LP plans; with the MILP it becomes the optimum, not a
   rounding).
5. Size: gear@4 chain 416 columns → ~100 groups (≈ 4× fewer integers than per-column); processing unit@4
   2 857 columns → ~700 groups. Expect gear in < 1 s; processing unit likely hits the time limit (return the
   best plan + gap as today). Gate: auto-enable only below ~300 groups, otherwise LP + rounding with a note.
6. Test (report): legendary gear, 4-slot all-quality loadout fixed, integer → "k normal machines + 1 shared"
   (normal tier vs. uncommon…epic sharing one machine), building count ≤ LP per-group ceil, balance holds.
