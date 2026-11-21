# Code review 2: DSP compute/UPS preset and Factorio quality, `e1aa030..HEAD` on `refactor`

Reviewed 2026-11-21. Scope: `src/planner/{compute,quality,quality_tests,lp,rules,mod,data,tests}.rs`, `src/wasm_api.rs`,
`tools/convert_factoriolab.py`, `data/factorio*.json`, `data/dsp.json`, `react-frontend/src`. Specs:
`docs/research/dsp-ups/00-最终报告.md` §4 + `model.json`; `docs/research/factorio-quality/00-最终报告.md`.
No source files were edited.

**Checks run**
- `cargo test -q`: 37 passed, 0 failed, 2 ignored, 256 s.
- `npx tsc -b`, `npx vite build`: pass.
- Regression, old (`e1aa030`) vs HEAD, two clones with the same dump example: 1505 plans (every produced item of
  Satisfactory 428, DSP 472, Factorio 2.0 605; LP with Raw / Buildings / Power, modules or proliferator on, plus a
  sample of whole-building MILPs). 15 differ, all Factorio uranium chains with productivity modules (Kovarex catalyst
  fix: U-235 30 buildings instead of 8, etc.) plus one equal-objective tie (electric engine unit MILP, 14.63 vs
  14.73 MW). Satisfactory and DSP are identical.
- `data/factorio.json` vs `e1aa030`: only `base_productivity: 0`, `dataset`, version/source text and `catalyst` on
  Kovarex and the two coal-liquefaction recipes. `data/dsp.json`: only `base_productivity: 0` and source text.
- Oracle numbers recomputed independently (numpy closed form, not `quality_lp.py`): gear 79.8786 (2Q2P ×4 tiers),
  2.1 values 78.396, self-recycling 2726.91, no-productivity loop 154.7238 with ratio 7.0855, 2Q2P ratio 5.031.
  All match the tests, so the replaced 116.05 → 154.7238 in `oracle_normal_to_higher_ratio` is correct.
- Hand check of compute lines (600 universe matrix/min): AM3 products mode = 1 + 3×0.12 + 0.13 + 0.02 per building
  + 0.0056/MW; matrix lab 1.5 + 7×0.12 + 0.15. Every DSP recipe building has a `model.json` cost.
- Converted `spa` / `2x1` / `2q1` against raw: 279 / 280 / 199 recycling recipes, all recycler-only, none allow
  productivity; `scrap-recycling` stays a normal recipe; module `by_quality` equals `qualityRecord`; 7 / 7 / 3
  catalyst recipes scaled per minute like the outputs.
- Browser (`planner-dist`, 1024 px): DSP Compute preset, Factorio 2x1 legendary gear, data set switch, recycling
  scope switch. localStorage was backed up and restored.

**Checked and found fine**
- Objective = spec §4.1: `n·(c_bld + 0.12·k + 0.13 + 0.02·[sprayed]) + c_gen·MW`, aux added to the building weight,
  not to `scale`; non-Compute objectives get aux = 0, so other games are untouched.
- `qrow` (stay 1−q, 0.9q, 0.09q, 0.009q, overflow on legendary; legendary row absorbing), module values per
  quality from data, recycler 25 % and no productivity, +300 % cap over built-in + research + modules, catalyst
  formula `c + (a−c)(1+p)`, fluids without tiers, fluid-only recipes at tier 0 with quality modules.
- No free-lunch loop in the default scope on 2x1 (processing unit, quantum processor, productivity module 3,
  holmium plate, asteroid chunk, iron ore at legendary all need raw input in plausible amounts; legendary pentapod
  egg needs only water because Gleba farming has no raw cost, which PROGRESS already lists as open).
- Raw tie-break under Compute (0.001 per item/min, ~3 % of the objective): with wind it moves the plan by 0.014 %
  of compute (2781.79 vs 2781.41); not worth a finding.

Counts: High 2, Medium 5, Low 9.

---

## High

### H1. Built-in machine productivity is dropped on recipes that disallow productivity modules
`src/planner/rules.rs:85`, `src/planner/quality.rs:229-233`. Confidence 85 %.

Both places compute productivity as `if r.productivity_allowed { base + research + modules } else { 0 }`. The
recipe flag only forbids productivity *modules*; the foundry / EM plant / biochamber +50 % applies to every recipe
the machine runs. The spec says so itself (§3.3: belts in the foundry get the built-in +50 % "虽然配方本身禁产能插件",
which is the whole reason for scottmsul's 47.93 detour).

Failing scenario (run): 2x1, target 120 transport belt/min, assemblers disabled. Plan: foundry, 60 plates + 60
gears → 120 belts. In game that input makes 180. 45 recipes in 2x1 are affected: every module, beacon, solar
panel, accumulator, electric pole (EM plant), belts / undergrounds / splitters / pipes / big mining drill (foundry),
fish breeding (biochamber). Normal Space Age plans overbuild these by 50 %, and quality plans cannot find the
foundry detour and overstate the cost of legendary modules.

Fix: `let base_p = b.base_productivity + if r.productivity_allowed { research } else { 0.0 }` in `tiered_modes`,
same split in `loadout_modes` (modules only when allowed, cap on the sum). Add the spec's §5 step 2 test
(EM plant empty mode `out_mult == 1.5`) and one for a productivity-disallowed recipe.

### H2. "Everything" recycling scope: one click starts a ~3 minute solve that cannot be cancelled, and it is persisted
`react-frontend/src/Sidebar.tsx:215`, `react-frontend/src/solver.ts` (`solveAsync`), `react-frontend/src/worker.ts`,
`src/planner/lp.rs` (LP step has no time limit; `time_limit_ms` only bounds branch-and-bound). Confidence 95 %.

Failing scenario (run in the browser): 2x1, legendary gear, click "Everything", then click "Target chain" and
change the rate. Nine seconds later the page still shows "Solving" and the old plan for the old rate; there is no
cancel button. The single worker is busy with the 16 578-column LP (191 s in WASM per PROGRESS) and every later
request queues behind it. When it finishes, its stale plan is shown first (`setPlan(p)` is unconditional in
`App.tsx`). The setting is saved to localStorage and the URL hash, so reloading or opening a shared link starts the
same solve again. The Factorio state saved at `localhost:4173` on this machine right now is
`recycling: "all"`, gear@4, so opening the planner on Factorio currently lands in that solve.

Fix, any two of: (a) terminate and recreate the worker when a newer request supersedes a running one; (b) do not
auto-solve above a column threshold (expansion size is known before the LP: return a "N columns, press Solve"
status); (c) do not persist `recycling: "all"`, or reset it to `chain` on load; (d) drop stale results
(`if (id === latestRequest()) setPlan(p)`).

---

## Medium

### M1. A quality target survives a switch to a data set without quality: shown as legendary, planned as normal
`react-frontend/src/Sidebar.tsx:258` (data set select), `src/planner/lp.rs:557-562`. Confidence 95 %.

Failing scenario (run): 2x1, target Iron gear wheel legendary, switch "Factorio version" to 2.0.77 base. The target
row still reads "Iron gear wheel ★legendary", the quality select is gone (so it cannot be reset), the plan makes 10
normal gears from 20 ore, and there is no warning. Same through the API: `iron-gear-wheel@4` on base data is
silently planned as normal.

Fix: strip `@q` from targets and supplies when the new data set has no `qualities` (and drop items it lacks), and
push a warning in `solve` when a key carries a tier but `quality_on` is false.

### M2. Quality plans still count whole buildings per tier column, the overcount the warning says was avoided
`src/planner/lp.rs:534` (`plan.buildings` sums `machines_ceil`), `react-frontend/src/Results.tsx:49`. Confidence 90 %.

Failing scenario (run): 2x1 legendary gear, 10/min, Raw preset. The card says 118 production buildings; the columns
sum to 110.7. The epic and legendary gear columns need 0.215 and 0.048 assemblers and are counted as 1 each, the
four recycler tiers as 16 + 3 + 1 + 1. The plan's own warning says the tiers share machines. The error grows with
chain length (each recipe adds up to four near-empty columns).

Fix: for `plan.quality`, ceil per (building, loadout) group or per base recipe instead of per column, in both
`plan.buildings` and the Buildings table.

### M3. DSP objective row overflows the sidebar
`react-frontend/src/index.css:186`. Confidence 90 % (measured at 1024 px, default fonts on this machine).

With five presets the row needs 308 px in a 270 px container: "Custom" ends at x = 333 while the sidebar's client
width is 320, so the button is cut and the sidebar gains a horizontal scrollbar (`scrollWidth` 333). The CSS comment
says the padding change makes them fit; it does not with the CJK label.

Fix: `flex-wrap: wrap` on `aside .seg`, or a shorter label ("UPS"), or a select for the preset.

### M4. Compute card understates per-entity cost when "whole buildings" is off
`src/planner/lp.rs:445-506` (uses fractional `n`). Confidence 80 %.

The model charges per entity, working or idle (spec §4.2). In LP mode the summary multiplies by fractional `n`
while the same page shows `→ ceil` building counts. Scenario: 60 processor/min, fractional: every recipe that
needs 0.3 of a machine contributes 0.3 × 1.5 units instead of 1.5. For small plans the card is low by tens of
percent; presets compared in LP mode are compared on a cost nobody can build.

Fix: compute the summary from `machines_ceil` (keep power as is), or label the card "fractional" in LP mode.

### M5. `recycling_scope_and_full_data` takes about four minutes in a debug test run
`src/planner/quality_tests.rs:301-305`. Confidence 95 %.

`cargo test -q` went to 256 s; this test solves the 16 578-column "all" scope. `build.sh` runs `cargo test` before
every production build.

Fix: mark the "all" half `#[ignore]` (or run it on the small `2q1` set), keep the chain-scope asserts in the fast
test.

---

## Low

### L1. Sorters are charged on buildings that have none
`src/planner/compute.rs:108-119`. Confidence 70 %. Fractionators sit on the belt (no sorters) but get 2 × 0.12;
stacked matrix labs share one set of sorters per stack but each lab is charged all ports (universe matrix: 7 ports
= 0.84 per lab, more than half its 1.5). The spec's `k_r = ports` has no exception, so this is a spec gap as much
as a code one. Fix: `sorters: 0` for `fractionator`, and a per-building divisor (labs per stack) in `model.json`.

### L2. Excluding the normal-tier row of a recipe removes all its tiers
`src/planner/lp.rs:53` + `src/planner/quality.rs:157`. Confidence 85 %. The × button stores the row's id. For
`iron-gear-wheel@2` only that tier goes; for the normal row the id is the base id, `allowed_recipes` drops the
recipe and no tier is expanded. There is no way to exclude only normal-tier crafting or recycling. Fix: store
`id@0` for normal rows in quality plans and check it in `expand`.

### L3. Test gaps and weak asserts
`src/planner/quality_tests.rs`, `src/planner/tests.rs`. Confidence 90 %.
- No test of built-in / researched productivity or the cap in the normal (non-quality) modes; the spec asks for
  "EM plant empty mode out_mult = 1.5" (would have caught H1's sibling case).
- `normal_plans_unchanged` only asserts `!p.quality` and no recycling recipe; it compares no numbers.
- `gear_with_2_1_values` does not assert the status; `catalyst_is_not_multiplied` checks `rules::outputs` only,
  not the quality path (`loadout_modes` has its own copy of the formula).
- Nothing covers `quality_allowed = false`, the `list` scope, `module_quality` in normal module modes, or a
  disabled tier recipe.
- `recycling_scope_and_full_data`'s `ore <= 79.8786` is always true on full data (foundry casting), so it does not
  test the scope.

### L4. An unknown data set id never solves
`react-frontend/src/App.tsx:40`. Confidence 85 %. A link with `dataset: "2.1"` (typo, or an id removed later) loads
the base data (`dataset` "2.0"), the guard `data.dataset !== settings.dataset` stays true and the page shows no
plan and no error. Fix: in `upgrade`, reset a data set id not in `DATASETS` to "2.0".

### L5. The same item cannot be a target at two qualities
`react-frontend/src/Sidebar.tsx:42`. Confidence 90 %. `taken` is keyed by base id, so with legendary gears in the
list the picker disables gears; normal + legendary of one item (a common mall request) needs a hand-edited link.
Fix: key `taken` by full id when the data has qualities.

### L6. Compute card shows "1 compute units" for 1.38, and does not say what it leaves out
`react-frontend/src/Results.tsx:68`. Confidence 90 %. `fmt(total, 0)` rounds small plans to an integer while the µs
card next to it shows 0.7. A target that is a raw resource (60 deuterium/min) shows 0 units: mining machines,
orbital collectors and ray receivers are outside the model (spec §4.2) but the card and hint do not say so.
Fix: one decimal below 100; add "excludes mining and collectors" to the card title.

### L7. "Mixed input belts" is hidden but still applied under Custom
`react-frontend/src/Sidebar.tsx` (compute block shown only for `objective === 'compute'`), `lp.rs:218`.
Confidence 85 %. Switch Compute → Custom with the box ticked: the sorter weight row is editable, the input ports
are still merged, and the control that says so is gone. The generator select has the same problem for the summary
card. Fix: show both controls for DSP whenever `cost.sorter > 0` or always.

### L8. Catalyst output takes the quality roll; module quality is invisible in normal module modes
`src/planner/quality.rs:238-249`, `src/planner/rules.rs:103-141`. Confidence 60 %. (a) The spec lists "does the
catalyst part roll quality" as open (§1.1, §7); the code rolls the whole amount (pentapod egg: 2 eggs out, 1
catalyst). If the game keeps the catalyst at input quality, legendary egg yields are overstated. Worth a comment
and a line in the UI hint until checked in game. (b) With `module_quality = 4` the normal module modes use
legendary values but the label still reads "Productivity module 3 ×4" and the held item is the normal module id.

### L9. `heat` is a raw resource nobody consumes (2x1, 2q1)
`tools/convert_factoriolab.py:109-122`. Confidence 80 %. The converter turns the input-less heat recipe into a
resource with weight 1; no recipe takes it, so it only adds a meaningless entry to the item pickers. The heating
tower also arrives as a building with `base_productivity: 1.5` and no recipes. Fix: skip `heat` and machines
without recipes.

---

## Resolution (2026-11-21)

All 16 findings fixed on `refactor` in b6ba1a9, 204e5b8, 2fd85e8, 0600b32, f310e55; log in `progress-fixes2.md`.

| # | Fix |
|---|-----|
| H1 | `rules.rs` / `quality.rs`: built-in machine productivity on every recipe; research and productivity modules only where allowed, capped at +300 %. Test `builtin_productivity_on_all_recipes` (EM plant 1.7 with research, foundry belts 1.5, 120 belts = 40 plates + 40 gears, quality loadouts). |
| H2 | `PlanSettings.max_columns` → `Status::TooLarge` with the model size, returned before the LP. The front end auto-solves up to 6000 columns (chain-scope quality plans are ≤ 2 900; "Everything" is 16 689) and otherwise shows the size with *Solve anyway* / *Use Target chain*, also for saved settings and links, so the persisted `recycling: "all"` on this machine now opens on that banner. A running solve shows elapsed time and *Cancel*; cancel or any newer request terminates and recreates the worker; superseded results are dropped. "all" stays persisted (it needs a click). |
| M1 | Switching to a data set without quality turns quality rows normal (merging duplicates); the solver warns when a tier is asked on data without tiers. |
| M2 | `Plan.building_counts`: quality plans round up per (base recipe, machine, loadout); the card and Buildings table use it (legendary gear 10/min: 20 buildings, 2 recyclers). |
| M3 | Label "算力" (tooltip says UPS), segment padding 4 px, `flex-wrap` as fallback: one row at 1024 px, no overflow at 375 px. |
| M4 | Compute summary charges whole entities (ceil n) in LP mode; power as solved. The LP objective itself stays fractional. |
| M5 | Full-data "all" test `#[ignore]` (`all_scope_full_data`); scope semantics covered on a small subset (`recycling_scopes`). Suite 40–60 s typical (was 256 s); slower runs come from MILP time limits under parallel load. |
| L1 | `model.json`: fractionator `sorters: 0`, labs `sorter_share: 0.1` (stack of 10, estimate). |
| L2 | Normal-tier rows of quality plans exclude `id@0`, which only removes that tier. |
| L3 | Added: numbers in `normal_plans_unchanged`, status/balance in `gear_with_2_1_values`, quality-path catalyst, `quality_allowed = false`, module tier in normal modes, `list` scope, tier exclusion; the always-true ore assert is gone. `objectives_on_complex_chains` retries a timing-only timeout. |
| L4 | Unknown data set ids in links fall back to 2.0. |
| L5 | `taken` is per quality: normal and legendary of one item can both be targets; changing a row to a listed quality merges rows. |
| L6 | One decimal below 100; card and tooltip say mining / collectors are excluded. |
| L7 | DSP generator and mixed-belt controls show under every objective, with a hint outside Compute. |
| L8 | (a) Comment + UI hint that catalyst output rolls quality (unchecked in game). (b) Normal module modes label "★legendary" and hold `module@q`. |
| L9 | Converter drops resources nothing consumes (`heat`) and machines without recipes; data regenerated. |

Not changed: the catalyst roll itself (needs an in-game check), the lab stack height (estimate, kept in `model.json`).
