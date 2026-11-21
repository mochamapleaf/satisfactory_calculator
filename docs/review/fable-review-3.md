# Review 3 — Factorio planets / research / beacons / module cost, and the Satisfactory siting prototype

Date 2026-11-21. Scope: `git diff b34e62c..b6c29d5` (Factorio, part A) and `tools/geo_*`, `data/satisfactory-geo`,
`docs/research/satisfactory-geo/_过程文件/impl-progress.md` (part B). No source file was edited.

## What was run

| Check | Result |
|---|---|
| `cargo test -q` | 52 passed, 4 ignored, 87 s |
| `npx tsc -b` | clean |
| `npx vite build` | built in 48 s (dist rebuilt from the working tree) |
| `python -I tools/geo_solve_test.py` | all 8 pass, 304 s; the 120 HMF stress case stops at the 180 s limit with a 6.4 % gap |
| Hand checks | scratch crate calling `wasm_api::solve_json` (30 scenarios, numbers below) |
| UI | 8 states rendered in headless Chrome (scratch profile) against the rebuilt dist. The built-in browser pane was hidden and my own tab never got past "Loading solver…", so it was not used; the saved planner settings on `localhost:4173` were backed up and restored unchanged |
| Geo experiments | 12 extra solves (tier, base, target, rail on/off), logs in the scratch folder |
| Sources opened | wiki.factorio.com/Beacon, FactorioLab `src/state/adjustment.ts` (main), satisfactory.wiki.gg/wiki/Conveyor_Belts, local copy of SatisfactoryTools `data1.0.json` |

Counts: **Critical 0 · High 4 · Medium 5 · Low 9.**

---

## High

### H1. Geo: `per_min()` converts rates that are already per minute — machine counts and machine cost are wrong by 60/time
`tools/geo_solve.py:132-139` (used at :212 and :266). Confidence 95 %.

`data/game-data.json` stores recipe `in` / `out` per minute (Iron Plate: 30 ingot → 20 plate, `time` 6). `per_min`
multiplies them by `60 / time` again. One "machine" in the model therefore does 60/time times the real work:
×2 for Heavy Modular Frame (30 s), ×10 for plates and screws (6 s), ×30 for the smelter (2 s), ×1 only for 60 s recipes.

Failing scenario: the committed `solution_best.json` (10 HMF/min) lists 2.5 Manufacturers for HMF (real: 5),
1.67 Smelters for 2 400 ore/min (real: 80), 5.25 Constructors for 2 100 screws/min (real: 52.5). Total 48.6 machines;
the real plan needs several hundred. The `x` cost part (8 181 OE of 16 710) and every objective quoted in
impl-progress are affected. Flows, ore equivalents and the rail/belt trade-off are not (the error scales a whole
recipe column), which is why `test_oe` and the balance checker pass.

Fix: drop the `60 / r['time']` factor (`a[k] -= q`, `a[k] += q`). Add a test with a hand value: 10 HMF/min → 5
Manufacturers and 25 Modular Frame assemblers; 12 000 plates/min → 600 Constructors + 600 Smelters. Rerun the sweep
and replace the numbers in impl-progress.

### H2. Geo: "rail never beats belts" is false under the model's own default costs, and vacuous for the instances tested
`docs/research/satisfactory-geo/_过程文件/impl-progress.md` (G7 entry); `tools/geo_solve.py` defaults. Confidence 90 %.

Counterexample, default costs, tier 8, both solves proven optimal (gap 0):

| 12 000 Iron Plate/min at `Sm3_0` | objective (OE) | transport |
|---|---|---|
| rail allowed | **70 947.7** — hubs `Sm1_2` + `Sm3_0`, 5 cars, 5 792 items/min by rail, 9 belts | 40 590 |
| rail disabled | 72 397.5 — 23 belts | 42 003 |

Rail wins by 2 % in total and 3.4 % in transport. The break-even from the script's own coefficients (belt 1 167 OE/km
per 780/min; double track 1 826 OE/km; two hubs 6 980; locomotive 720; car + 2 platforms 2 030):

| flow, distance | ore (stack 100) rail/belt | stack 200 | stack 500 |
|---|---|---|---|
| 3 900/min, 5 km | 1.20 | 0.93 | 0.79 |
| 7 800/min, 5 km | 0.88 | 0.60 | 0.50 |
| 7 800/min, 2 km | 1.36 | 1.01 | 1.01 |
| 780/min, any | 2.6 – 5.7 | | |

So rail pays from about five belts over five kilometres. The tested instances never get there, for reasons that are
modelling choices rather than findings:

- **Target.** 10 HMF/min needs 2 400 iron, 900 coal, 900 limestone per minute. The base cell `S0_m1` holds 7 200 iron
  and 1 800 limestone; coal is one cell away. The optimum carries 780 items/min over 1.2 km: 1 407 OE of 16 710.
  There is no transport to choose a mode for. A Computer target (30/min) behaves the same.
- **Free dispersal.** Machines are continuous and a production site has no fixed cost, no power hookup and no
  minimum size, so the model always processes at the ore and ships the compressed product. Players centralise
  because every extra site costs work.
- **Unproven on the larger runs.** "60/120/200 HMF: no hub opened" comes from incumbents with 4–6 % gaps; the rail
  saving in the counterexample is 2 %. At a 100 s limit, 40 HMF at `Sm3_0` on tier 6 returned 127 604 with rail
  allowed and 127 341 with rail disabled: the larger feasible set gave the worse answer, so these runs cannot rank
  the two.
- **Cost metric.** Only hubs carry a work term (`W_WORK_HUB` 2 000 of F = 3 490). Belts carry none per kilometre,
  although five parallel belts over 5 km are far more building work than one track. Parity for 3 900 ore/min over
  5 km needs only about 240 OE of work per belt-km.
- **Not the cause:** tier (tier 6 picks Mk.1 belts, 750 OE/km per 60/min, and still opens no hub because the flows
  stay short), F (0.5×–2× changes nothing when nothing is shipped), water (HMF and Computer chains need none).

Fix: replace the sentence with the break-even statement and the counterexample; add instances that need transport
(a low-compression product at a resource-poor site, or machines restricted to the base); add a per-km work weight for
belts and track and sweep it next to F; report a rail-vs-belt result only when both solves are proven optimal or the
difference exceeds the gap.

### H3. Scrap recycling productivity research has no effect
`src/planner/rules.rs:168`, `src/planner/quality.rs:239`; UI hint `react-frontend/src/Sidebar.tsx:497`. Confidence 90 %.

Research is added only `if r.productivity_allowed`. `scrap-recycling` has `productivity_allowed: false` (the recycler
refuses productivity modules), so the family the UI offers does nothing.

Failing scenario (spa, planets Fulgora, 60 holmium ore/min): 6 000 scrap/min and 40 recyclers with and without
`research_productivity {"scrap-recycling": 0.5}`; expected 4 000 scrap and 26.7 recyclers. The UI shows level 5,
"+50 %".

In the game the technology changes the recipe's productivity whatever modules the machine accepts. FactorioLab adds
the per-recipe research value unconditionally (`adjustment.ts`); the module ban is a separate check.

Fix: `let base_p = b.base_productivity + research;` in both places (the converter only lists recipes a technology
names, so no other recipe gains anything). Correct the hint ("Recipes that refuse productivity modules get none")
and add the scenario above to `research_productivity_families`.

### H4. Heat and fuel: heat exchangers, boilers and burner machines run on nothing (known issue, confirmed and wider)
`tools/convert_factoriolab.py:58` (power = `usage` only), `:115` (`burn` recipes skipped), `:182-186` (`heat` dropped).
Confidence 90 %. Present before b34e62c; the data diff of this range only adds `locations`, `beacons`, `research`.

| Data set | Heat exchanger | Boiler | Other |
|---|---|---|---|
| 2.0, spa | 10 MW booked as **electricity** (FactorioLab `usage: 10000`, no type) | 2.0: 1.8 MW electricity; spa: 0 MW | stone/steel furnace, biochamber 0 MW, no fuel or nutrients |
| 2x1, 2q1 | **0 MW and no heat** (`machine.heat: 10000` is ignored; the `heat` item was dropped) | 0 MW, no fuel | steam engine −1.8 MW, steam turbine −5.82 MW, fusion generator −50 MW exist as buildings |

Failing scenario: target 6 185.567 steam/min. 2x1 and 2q1: one heat exchanger, 618.6 water, 0 MW, objective 1.006.
Base 2.0: the same plan with 10 MW of electricity. Coal liquefaction and any steam target get free energy in the 2.1
sets. The generators cannot be exploited today only because `lp.rs:241` creates a power variable for
`power_base > 0` alone; the 165 °C turbine recipe also ignores its `usageFactor` 0.309.

Fix, in the converter:
1. Machines with `heat`: add `["heat", heat_kW × 60]` (kJ/min at speed 1) to the inputs of the recipes they run. For
   the older 2.0/spa files special-case `heat-exchanger` the same way and set its electric power to 0.
2. Keep `heat` as an item and turn `nuclear-reactor-reactor` / `heating-tower-reactor` into real recipes with a fuel
   input: `burner_kW × 60 / (fuel MJ × 1000)` per minute for a default fuel (`defaults.fuelRank`), the spent cell as
   output for the reactor (this also gives `depleted-uranium-fuel-cell` a source; it is a weight-10 raw resource now),
   heating tower output × 2.5 from its `baseEffect`.
3. Same fuel input for boilers and other `burner` machines, or at least a documented exclusion.
4. Until generators are modelled, drop buildings with negative power.
Test: 6 185.567 steam/min on 2x1 must consume 600 000 kJ/min of heat = 0.25 reactor (no neighbour bonus).

---

## Medium

### M1. Research productivity set in a Space Age data set keeps applying in sets that have no research UI
`react-frontend/src/solver.ts:36-49` (`withDataset`), `src/planner/rules.rs:152`. Confidence 90 %.

Scenario: on spa set Steel plate productivity to level 10, switch the data set to "2.1.25 base + quality". The
Research section disappears, yet 60 steel/min costs 150 iron plate instead of 300 (checked in the solver and in the
rendered page). The base game has no such technology. The value also travels in shared links.

Fix: in `withDataset` (and `upgrade`) keep only the recipe ids named by the target set's `research`; or have the
solver ignore ids outside `data.research`.

### M2. Planet filter looks at recipe locations only: machine locations are dropped, and a recipe on another planet gives no hint
`tools/convert_factoriolab.py:147, 167` (recipe `locations` only), `src/planner/lp.rs:669-689`. Confidence 85 %.

- FactorioLab also tags machines: crusher `space-platform`, captive biter spawner `nauvis`. The converter ignores
  these. Scenarios: Nauvis only + supplied carbonic chunks → the plan crushes asteroids on Nauvis; planets = Gleba,
  target biter egg → 60/min as a raw resource with `locations: []`.
- Target Metallurgic science pack (or Space science pack) with the default Nauvis: status infeasible, 0 columns,
  only "targets exceed resource limits or need a disabled recipe". The planet hint covers resources, not recipes.
- The hint lists every off-planet resource in the closure, not the blocking ones: an infeasible Nauvis steel plan
  names calcite, lava, scrap and three asteroid chunks.

Fix: intersect each recipe's and resource's locations with those of its producers in the converter; in the
infeasible branch also name targets (and inputs) all of whose recipes were removed by the planet filter, with the
planets they need; list only resources whose items have no remaining source.

### M3. Geo: site supply and belt feed ignore the tier
`tools/geo_solve.py:312-319`, `data/satisfactory-geo/sites.json` (`max_rate` at 250 %). Confidence 75 %.

Supply is the Mk.3-miner 250 % rate at every tier, and the feed belt is costed linearly. At tier 8 a pure node
(1 200/min) cannot leave on a Mk.5 belt (780); at tier 6 the model extracts 2 240 iron/min from one cell through
Mk.1 belts (60/min each, 5.6 OE per item/min of feed). Tier comparisons therefore overstate local supply, which
again shortens hauls. Fix: cap each node at `min(miner rate at tier, belt capacity at tier)` when building `supply`,
or make supply a function of `tier` in `build()`.

### M4. Geo tests do not pin the numbers that matter
`tools/geo_solve_test.py`. Confidence 85 %.

- No test compares an objective, a machine count or a cost coefficient with a hand value, so H1 passed unnoticed.
- The only target is HMF, whose chain has no fluid: pipes, fluid cars, `fplat`/`fcar` rows and `water_anywhere` are
  never exercised. A Computer run has four fluids and builds 0 pipes.
- `check()` reuses the model's own `A`, variable index and `meta` (belt capacity, RtD); it does not verify pipe
  capacity, fluid cars, platform caps for fluids or the objective. It is a consistency check, not an independent one.
- `test_rail_free_not_worse` holds only because both solves reach the same optimum; under a time limit the property
  fails (see H2).
- `test_formulas` evaluates the same formula on both sides.

Fix: a two-site, one-item instance with a closed-form optimum (belt cost and rail cost computed by hand, rail chosen
above the break-even flow, belt below); a fluid target (Plastic or Fuel) with pipe and fluid-car assertions; assert
cost parts, not only feasibility.

### M5. Geo: implementation departs from the report's model in ways that all lean against rail
`tools/geo_solve.py:262-407` against `00-最终报告.md` §7.1–7.2. Confidence 80 %.

| Report | Implementation |
|---|---|
| track segment graph shared by lines, 2–3 candidate paths | one private double track per hub pair (`:347`); a three-hub network pays three full tracks |
| belts 1 200/min (Mk.6) | one tier chosen by OE per capacity: Mk.5, alclad sheet at 2.33 OE with water free |
| platform feed 2 400/min | `feed = 2 × cap` of the chosen belt = 1 560 (`:298`) |
| platforms per hub ≤ P̄·h | not enforced |
| power balance, generators as recipes | absent |
| extraction per node with its own feed distance | per site, mean feed distance |
| `C̄` from demand | from the gross of one LP optimum (`:332-343`), not a valid bound when recipes compete |

The docstring lists the first and the power item; the rest is unstated. Fix: state all deviations in the docstring
and impl-progress, and implement shared track before drawing mode conclusions for more than two hubs.

---

## Low

- **L1. A malformed link blanks the app and can stick.** `Sidebar.tsx:284`, `urlState.ts:9-25`. A hash with
  `"beacon_counts":"8"` throws `beacon_counts.join is not a function` (blank page, confirmed). `saveSettings` runs on
  mount before the sidebar renders, so the bad value is also written to localStorage. Fix: in `upgrade()` coerce
  `planets`, `beacon_counts`, `module_costs` to arrays of the right type and `research_productivity` to an object,
  else fall back to defaults. Confidence 80 % (the localStorage part was not reproduced).
- **L2. Research values are not clamped.** `rules.rs:152`, `quality.rs:198`. `{"steel-plate": -2}` gives
  `out_mult = −1`, an infeasible plan and the unrelated planet hint. Fix: `research.clamp(0.0, MAX_PRODUCTIVITY)`,
  non-finite → 0. Confidence 90 %.
- **L3. Beacon power follows machine utilisation.** `lp.rs:461-464`. `power_base` includes the beacons, and the plan
  multiplies it by the clock: 4 AM3 at 95.4 % with 8 shared beacons report 13.59 MW instead of 13.77 (beacons draw
  480 kW each all the time). Fix: keep beacon MW out of the clock factor. Confidence 85 %.
- **L4. Beacon counts are not checked against geometry or model size.** `rules.rs:107`. Any count up to 64 is
  offered; each count multiplies the columns (quality plans: tiers × loadouts × counts), which reaches the
  6 000-column prompt quickly. Fix: cap the list length (say 4) and note the usual maxima (8 in rows, 12 around a
  3×3 machine) in the hint. Confidence 70 %.
- **L5. DSP mode order changed.** `rules.rs:169-205`. Proliferator modes are now pushed before the plain mode (the
  "plain first" comment is stale). Nothing indexes the list, but the LP column order differs from b34e62c, so
  degenerate ties may resolve differently. The 10-plan regression in PROGRESS covers the common cases. Confidence 60 %.
- **L6. Research UI.** `Sidebar.tsx:499-508`. `max={300}` is not enforced (level 1 000 000 is stored; the solver
  caps at +300 %), and the level shown is read from the first recipe of the family only. Confidence 80 %.
- **L7. Module cost units.** `mod.rs:172-175`, `Sidebar.tsx:248`. A one-time module is added to a per-minute
  objective; the defaults 1/3/10/30/100 are a guess, as the hint says. Works as documented (71.546 = 71.429 ore +
  0.034 buildings + 0.083 modules by hand). Suggest deriving defaults from the module's own raw cost over a payback
  time. Confidence 70 %.
- **L8. Geo `W_RAW` and the OE metric.** `geo_solve.py:55-57`. `W_RAW = 1` per item/min is the right order for a
  Mk.2 miner (about 730 OE for 600/min) but is flat across purity and tier; OE counts every raw item as 1 and water
  as 0, so alclad sheet (bauxite, coal, copper, silica chain) costs less than a steel beam. State this next to the
  results. Confidence 75 %.
- **L9. Stale hub rule.** `geo_sites.py` picks hubs by raw output as a proxy; PROGRESS already lists "hub choice by
  a belt-only solve" as not done. In the counterexample the winning hub `Sm1_2` is a `top_output` hub, so the proxy
  mattered. Confidence 60 %.

---

## Verified correct

Beacons (`rules.rs:92-119`, `quality.rs:226-242`), by hand through the solver:
- 8 beacons × 2 speed module 3, normal: 8 × 1.5 × 0.3535 × 2 × 0.5 = +424.2 % speed; AM3 gear wheels
  150 → 786.4/min; 0.375 × (1 + 16 × 1.5 × 0.3535 × 0.7) = 2.602 MW; with sharing 8: +0.48 MW and 1 beacon per
  machine. Plan: n = 0.07631, 3.0820 MW per machine.
- 12 beacons + 4 productivity module 3: speed 1.25 × 5.5948, ×1.4 output, n = 0.05107, 4.3599 MW per machine.
- Efficiency beacons hit the 20 % consumption floor (0.0798 MW per machine at sharing 100).
- Quality penalty: 4 quality module 3 + 1 beacon of speed module 1: q = 0.10 − 0.03 = 0.07, uncommon/normal =
  0.063/0.93; legendary modules and legendary beacon: speed 1.25 × 3.3 = 4.125, q = 0.25 − 0.05 = 0.20,
  1.3925 MW, 89.1 uncommon gears/min per machine; module cost 6 × 100 per machine.
- Productivity and quality modules are refused as beacon modules; machines without slots get no beacon mode
  (the wiki says the same); share < 1 is treated as 1.
- Data: effectivity 1.5/1.7/1.9/2.1/2.5 and 480/400/320/240/80 kW match the wiki; the truncated profile
  (0.7071, 0.5773, …) is the game's own table; module penalties do not scale with quality.

Research and cap: steel at level 35 with productivity modules gives ×4 exactly (75 iron plate per 60 steel);
FactorioLab uses the same cap; the eight families and 0.1 step match the data.

Planets: resource locations (water Nauvis/Gleba, coal Nauvis/Vulcanus, crude Nauvis/Aquilo, stone Nauvis/Gleba,
heavy oil Fulgora, lava/calcite/tungsten/acid Vulcanus, scrap Fulgora, lithium brine/fluorine/ammoniacal Aquilo,
chunks on the platform) and the 39 tagged recipes are right; empty planet list and old links without the new
fields behave sensibly.

Regression: data files differ from b34e62c only by `locations`, `beacons`, `research`; Satisfactory and DSP solve
paths are untouched apart from L5.

Geo inputs: building, belt, pipe, rail, station, platform, car and locomotive costs and the stack sizes match
SatisfactoryTools `data1.0.json`; belts cost about 0.5 material per metre (wiki); OE values (plate 1.5, pipe 3,
beam 8, HMF 420) are right.

## Suggested order of fixes
H1 (one line + test) → H3 (one line in two files + test) → H2 (rewrite the conclusion, add the work weight and a
transport-heavy instance) → M1 → H4 (converter) → M2 → M4/M3/M5 → Low.

---

## Resolution (Factorio / solver / UI items, 2026-11-21)

Geo items (H1, H2, M3, M4, M5, L8, L9) are handled separately. Commits 993b339, 6e6779b, d1e318e; progress
log `docs/review/progress-fixes3.md`.

| Item | Status | Change and check |
|---|---|---|
| H3 | fixed | `rules::research_bonus` applies research whatever `productivity_allowed` says (tiered and quality loadouts); hint corrected. Test `scrap_recycling_research` (spa and 2x1, Fulgora, 60 holmium ore): 6 000 scrap / 40 recyclers → 4 000 / 26.67 at +50 %. Rendered page shows the same. |
| H4 | fixed in the converter, as proposed, with one change | Fuel sits on the building (`fuel` {item, rate/min, result}), not in recipe inputs, because burner furnaces share recipes with the electric furnace; the solver adds fuel per unit of work (rate × consumption / speed) and spent cells as extra products. Electric power only in `power` (2.0 boiler 1.8 MW and 2.0/spa heat exchanger 10 MW are gone). Heat is an item in MJ (no quality); `nuclear-reactor-reactor` / `heating-tower-reactor` are recipes (synthesised for 2.0.77; the 2.0.77 tower gets the 2.1 values, 16 MW of fuel at 250 %); no neighbour bonus. Depleted cells now come from reactors (no longer raw). Biochamber nutrients, captive spawner bioflux, fusion reactor cells included. Generators dropped. Test `heat_and_fuel`: 6 185.567 steam/min on 2x1, 2q1, spa = 600 MJ/min = 0.25 reactor = 0.075 cells/min; heating tower 0.25 = 60 coal/min; 2.0 boiler 6 000 steam = 45 coal; stone furnace 60 plates = 4.32 coal; biochamber 15 → 3 nutrients/min with 4 × efficiency 3. dsp.json unchanged. |
| M1 | fixed | Solver ignores research on recipes the data set's research does not name; `withDataset` drops research levels for sets without research. Tests in `research_productivity_families`; switching spa → 2q1 in the page clears them. |
| M2 | fixed | Machine `locations` (crusher: space platform, captive spawner: Nauvis) filter producers; the infeasible hint lists only blocking items (unobtainable leaves reached from the targets), with the planets from resources, recipes and machines. Metallurgic science pack on Nauvis → "Metallurgic science pack (Vulcanus)". Test `planet_filter_machines_and_hint`. |
| L1 | fixed | `urlState.sanitize` drops fields of the wrong type before defaults; `beacon_counts:"8"` renders. |
| L2 | fixed | Research clamped to 0 … +300 %, non-finite → 0. |
| L3 | fixed | `Mode.beacon_power` is charged per building, outside the clock factor (LP row, warm start, report). Test `beacon_power_and_mode_order`. |
| L4 | fixed | At most 4 beacon counts (smallest kept), hint notes 8 in rows / 12 around a 3×3 machine. |
| L5 | fixed | Modes back in b34e62c order (plain, then boosts in `effects` order). |
| L6 | fixed | Level input clamped to 0 … 300; the level shown is the highest of the family's recipes. |
| L7 | kept | Defaults stay; deriving them needs a payback time only the player can choose. The hint now explains one-time cost vs per-minute objective. |

Regression against b6c29d5 (20 plans): Satisfactory and DSP plans identical; Factorio plans change only
through fuel, heat and beacon power (steel furnaces give way to electric furnaces, steam needs coal or
reactor heat, F7 beacon power 3.92 → 4.27 MW). `cargo test -q` 56 passed / 4 ignored; wasm, `tsc -b`,
`vite build` pass.


---

## Resolution (geo)

Date 2026-11-21, branch `refactor`. Files: `tools/geo_solve.py`, `tools/geo_solve_test.py`, new `tools/geo_breakeven.py`
and `tools/geo_experiments.py`, `docs/research/satisfactory-geo/00-最终报告.md` (new §8.5, summary item 8, §11),
`_过程文件/impl-progress.md` (R1 entries), regenerated sweeps, plots and `exp_*.json` / `breakeven.*`.

- **H1 fixed.** `per_min()` no longer multiplies by 60/time. Test `test_machine_counts_hand_values`: 10 HMF/min ->
  5 Manufacturers, 25 Modular Frame assemblers (solved), 12 000 plate/min -> 600 Constructors + 600 Smelters; the
  machine cost part is reassembled from counts x building OE. Default instance now 35 417.31 OE (was 16 710.03).
- **H2 fixed.** The "rail never beats belts" sentence is replaced in impl-progress and the report (§0 item 8, §8.5)
  by the break-even statement and real-map results, all with both solves proven optimal:
  12 000 plate/min at Sm3_0: rail 95 123.6 vs belts 99 386.2 (-4.3 %; hubs Sm1_2 + Sm3_0, 3.5 km, 5 cars,
  5 760/min by rail); 24 000: -7.9 %; 1 500-9 000: belts only. Closed-form thresholds (`geo_breakeven.py`): rail pays
  from about 5 000-6 000 items/min (stack >= 200) on 2-3 km+ routes, 6 000-11 000 ore/min at 3-5 km, not under ~2 km.
  Per-km work weight `work_km` added and swept (250 / 1 000 OE/km: -7.9 % / -15.9 % at 12 000 plate) next to F
  (p sweep `_plate12k`: p* = 2 at F 0.5x/1x, 0 at 2x). `geo_experiments.py` labels a comparison conclusive only when
  both solves are optimal or the difference exceeds the gap; `test_rail_free_not_worse` now requires both optimal.
  Transport-needing instances added: volume series at Sm3_0 and `machines_at` (machines only at the demand site).
- **M3 fixed.** Supply per node from `node_rate(node, tier, belt_cap, pipe_cap)`: miner Mk.1/2/3 at T0/T4/T8,
  oil T5, wells T8, 250 % from T3, capped at one belt/pipe of the tier (pure node at T8: 780). `test_tier_supply`.
- **M4 fixed.** Hand-value tests (machine counts, RtD/platform literals), `test_two_site_closed_form` (synthetic
  two-site map; 780/min -> one belt 7 583.3 OE, 7 800/min -> rail 62 199 OE with 21 cars; objective asserted),
  `test_fluid_pipes` (1 500 m3/min oil at S0_m1, pipe capacity). `check()` now also verifies pipe capacity, fluid
  cars and fluid platforms, and recomputes the objective from physical quantities and the cost table.
- **M5 addressed.** All departures listed in the `geo_solve.py` docstring and report §8.5 (private track per pair,
  one belt tier, 2-belt platform feed, no platform cap / power, per-site feed distance, heuristic C-bar). C-bar is now
  2x the LP-derived need +1 and `check()` reports any direction that reaches it (none did). Shared track is not
  implemented; the mode conclusion rests on the two-hub 12 000-plate result, the three-hub case is flagged as
  conservative.
- **L8 addressed.** W_RAW / OE caveats (flat across purity and tier, water 0 makes alclad belts cheap) stated next
  to the results in report §8.5.
- **L9 tested.** Hubs re-picked from belt-only flow (`geo_experiments.py hubs`) drop the Sm1_2-Sm3_0 pair and fall
  back to belts (99 386.2), so the sites.json rule is kept; hub choice is listed as open in report §11.
- Tests: `python -I tools/geo_solve_test.py` -> 12 tests pass (303 s; the 120 HMF stress case stops at its 180 s limit with a 3.4 % gap and passes the checker). Commit 0805af3.
