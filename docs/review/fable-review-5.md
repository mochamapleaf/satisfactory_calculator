# Review 5 — DSP mixed-belt (混带) planner

Date 2026-11-22. Scope: commits 315483a and f20054a: `src/planner/mixed_belt.rs`, `mixed_belt_tests.rs`,
`react-frontend/src/MixedBelt.tsx`, the WASM binding `mixed_belt_json`, `solver.ts` / `urlState.ts` / `App.tsx`
changes and the CSS. Spec: `docs/research/dsp-mixed-belt/00-最终报告.md` with `_过程文件/proto_mixed_belt.py`,
`fable-review.md`, `progress-impl.md`. File:line references are to f20054a.

## What was run

| Check | Result |
|---|---|
| `cargo test -q mixed_belt` at f20054a | 13 passed |
| `cargo test -q` after the fixes | 88 passed, 4 ignored, 92 s (mixed belt: 20, 7 of them new) |
| Probe tests at f20054a (Rust, removed again) | his five lines, Mk.II lines of 1–40 machines, every DSP recipe × mode × 6 belt settings × 6 machine counts (118 152 layouts), bad settings JSON, slow recipes, a 3 000 000-machine line |
| Live browser before the fixes | Playwright (Chromium headless) on `vite preview` :4173, build of f20054a: fractional R, Mk.II 27 machines, a universe-matrix plan (33 layouts). Evidence quoted in the findings |
| `wasm-pack build --release`, `npx tsc -b`, `npx vite build` | clean; dist rebuilt from the fixed tree |
| Live browser after the fixes | the original `live_test_mixed_belt.py` 26/26; new `docs/research/dsp-mixed-belt/_过程文件/live_test_review5.py` 31/31, screenshots in `shots/review5/`. List in the Resolution section |

Counts: **Critical 0 · High 1 · Medium 5 · Low 12.**

The model's arithmetic is right: the two-input formula, the even positions, the ladder groups and the simulator
reproduce the report's numbers, and the fuzz found no NaN and no panic. The defects are around it: one typed
number could blank the page and break the solve, the return-flow rule was checked after the choice instead of
being part of it, and long lines were unbounded. One number in the report was wrong, not the code.

## The two questions asked

**The tie-break added to reproduce his turbine returns 4.5 / 12 — principled or overfit?** Mixed.
`refine` (`mixed_belt.rs:572-611`) ordered equally good layouts by (1) return flows within the control unit's
shares and a known unit, (2) equal minima, (3) smaller total return.
- (1) is not a tie-break. It is the report's feasibility condition 4 (§2.3) and step 6. Used only among ties, it
  left lines failing that have a passing layout (M1).
- (2) is his stated preference [BV1fXkxYDEbW 34:05].
- (3) is MINE, and was labelled so in the code. It is not fitted to the turbine: the test accepts either mirror
  image (the planner shows coil first at σ 2/5; he built motor first at σ 3/5, the same line with the two equal
  ingredients swapped). It was not disclosed in the page.
After the fix: (1) is a feasibility condition, (2) and (3) stay as tie-breaks, and the page says the order is
the tool's own.

**Processors with two rows: report §2.3 (b) says interval 9 reaches 1.5 / 1.5, the implementer says 1 / 2 — who is
right?** The implementer. Each belt position uses a whole 1 + 1 stacks/s. Moving the first refill by one position
moves both minima by 1, and a σ step moves them by 6, so the minima are whole numbers. S = 3 cannot be split into
1.5 / 1.5; the best is 1 / 2. The report's 1.5 was S/2, never searched. The same holds for the report's other
reading (speed-up, one row, 1 + 1 per machine). Report corrected (M2).

---

## High

### H1. One fractional or negative number in a belt setting blanks the page, is saved, and breaks the solve
`src/planner/mixed_belt.rs:56-91` (integer fields `row_len`, `unit_max`, `unit_cap`, `ladder_short`, `ladder_cap`,
`hint_machines`, override `k`, `period` as `u32` / `u8`), `src/planner/mod.rs:181` (`belt_layout` inside
`PlanSettings`), `react-frontend/src/MixedBelt.tsx:296-309` (inputs write any number), `:266, 315`
(`rep.recipes.length` on an error object), `react-frontend/src/urlState.ts:46-57` (finite numbers pass),
`solver.ts:422` (the same settings JSON goes to the solve).

Typing 7.5 into 每排台数 R makes `PlanSettings` fail to parse ("invalid type: floating point `7.5`, expected u32").
`mixed_belt_json` then returns `{"error": …}` with no `recipes`, and the tab reads `rep.recipes.length`.

Observed (build of f20054a, Mk.II proliferator plan, tab open, R = 7.5): the page goes blank, page error "Cannot
read properties of undefined (reading 'length')". The value is already in the URL and localStorage. Since the
solver parses the same object, every later solve fails too, and the tab that holds the field says "Solve a plan
first" without its settings bar. Same for −1 in 短组, 2.5 in 且 > 台, a large `k`, or `"margin": null` in a link.

Fix: read `belt_layout` leniently on the Rust side, clean it on the page side, never index an error object, keep
the settings bar without a plan.

---

## Medium

### M1. The return-flow rule is checked after the layout is chosen, so lines fail that have a passing layout
`mixed_belt.rs:461-503` (`choose_period`: steady state only), `:572-611` (`refine`: only among layouts with the
same best minimum), `:693-700` (the check, after the choice).

Report §2.3 lists four feasibility conditions; the fourth is the return-flow check of §2.4, and step 4 takes the
largest *feasible* period. The code took the largest period whose steady state is ≥ m, then the layout with the
highest minimum, and only then looked at the returns of the actual line.

Failure scenario: Mk.II proliferator (3 / 1.5 stacks/s), 27 machines. Chosen: P = 6, k = 3, minima 4.5 / 4.5;
the line ends 3 machines into a period and returns Mk.I at 22.5 > its share 18 → 不通过. P = 6 with k = 4
(minima 3 / 6, both ≥ m) returns 7.5 / 9 and passes. For this recipe half of all line lengths failed (every count
with remainder 3, 4 or 5 mod 6). In the universe-matrix plan 4 of 20 scheme-A rows read 不通过 (Mk.III
proliferator 142, carbon nanotubes 252, quantum chips 52 "超出已知结构", titanium crystal 103); in the probe
13 540 of 36 432 scheme-A layouts failed.

Fix: make the return flow part of the choice: within a period, then smaller periods.

### M2. Report: processors at interval 9 do not reach 1.5 / 1.5
`docs/research/dsp-mixed-belt/00-最终报告.md` §1.3 (line 77), §2.1 (line 197), §2.3 (b) (line 275); also carried in
`_过程文件/fable-review.md` (S2, 旁证).

See "The two questions". Consequence beyond the number: §2.1 named the processor line as a second case where he
accepted a minimum of 1.5. It is not: interval 9 gives 1, and he only said it would do; he built interval 7
(4 / 5). The default m = 1.5 now rests on one built line (Mk.III proliferator). The code was already right
(`mixed_belt_tests.rs:119-131`).

### M3. On a green or yellow mixed belt the control unit shown cannot be built as described
`mixed_belt.rs:386` (shares = fifths of C for any belt), `:30, 506-524` (return limit fixed at 12), `:690`
(limiter text "green belt (2/5)").

Fifths come from a green (12/30) or yellow (6/30) limiter lane on a blue belt, and the return limit is one green
belt. With 带 = 绿带 the planner still shows σ 2/5 with "green belt (2/5)" as limiter (a green lane on a green belt
limits nothing) and judges returns against 12, the whole belt. Rows read 通过.

Fix: flag it. A model of control units for other belts would be new design work without a source (left open).

### M4. A long line is laid out machine by machine with no bound
`mixed_belt.rs:310-329` (`run_two`), `:572-611` (one full run per tied layout), `:721-733` (first-fit over every
refill, quadratic), `:769-786` and `:206-265` (one event and one trace point per machine, all serialised),
`:1183-1184` (`machines * n_out` in `u32`), `MixedBelt.tsx:108` (one SVG rect per machine).

A target typed with too many zeros gives a recipe millions of machines on one control unit. Measured at f20054a
(debug build): 3 000 000 machines took 40 s in `design_two` alone, before the trace. With 4 × 10⁹ machines the
sorter totals overflow `u32` (wraps in the WASM build). A 12 000-machine line means 12 000 rects per diagram.

Fix: a line limit with a clear message, an equivalent short line for the search and the diagram, saturating
totals.

### M5. While a new solve runs, the tab lays out the old plan with the new settings
`react-frontend/src/App.tsx:223`, `MixedBelt.tsx:265-267`.

The tab called `mixedBelt(settings, plan)` with the current settings and whatever plan is in hand. After a
settings change the plan is the earlier one until the new solve delivers (or for good, if the solve is stopped).
Modes are rebuilt from the new settings, so a recipe whose proliferator was just switched off reads "this recipe's
machine / proliferator mode is no longer enabled: solve again", and nothing says the rows belong to an earlier plan.

Fix: lay the plan out with the settings it was solved for, and say when it is not the current plan.

---

## Low

### L1. "Recommended" names a layout the tab never showed
`mixed_belt.rs:630` (taken before `refine`), `:681`. Turbines, 60 machines: auto shows coil first, k = 1; after
fixing σ by hand the hint read "推荐 … 先补 电动机, k = 9".

### L2. Spray coaters counted for one control unit
`mixed_belt.rs:1193`: `feed_points + 2 * units`, where `feed_points` is one unit's refills. With a unit cap the
count is too low.

### L3. Repeat unit ignores the second row
`mixed_belt.rs:474`: lcm(P, R). A belt that feeds two rows passes 2R machines per run, so the layout repeats every
lcm(P, 2R). Same result at his R = 15; different for even R (P = 6, R = 16: 48 by the code, 96 in fact, over the
4R cap).

### L4. No Reset when nothing is recommended; period list too short
`MixedBelt.tsx:209` (Reset only with a recommendation), `:194` (24 multiples: a slow recipe's recommended P = 576
is not in its own list).

### L5. Odd positions typed by hand with two rows are laid out silently
`mixed_belt.rs:381-383, 655`: an odd k, or a period with odd intervals, falls back to single-row positions without
a word.

### L6. Sequence text and diagram do not use the model's values
`MixedBelt.tsx:52` writes "> 12 → 双绿带单元" from its own comparison, also when the model says "超出已知结构";
`:160` draws the margin from the raw setting while Rust replaces an invalid one by 1.5.

### L7. Copy gives no feedback and does nothing without the clipboard API
`MixedBelt.tsx:166`: `navigator.clipboard?.writeText` is undefined on plain http (the NAS by address).

### L8. Trace minima start from the wrong flows
`mixed_belt.rs:208`: `mins` starts at the returns arriving at the start. A ladder ring starts empty, so its
minima were always 0. Not shown in the page.

### L9. Machine count rounded to nearest in integer mode
`mixed_belt.rs:1014`: `machines.round()`. A plan with 2.4 machines under `integer: true` (a link, a stopped solve)
is laid out as 2; 0.3 disappears.

### L10. Search cost grows with P², ladder loop runs to any cap
`mixed_belt.rs:389` simulates k + 2P machines per (σ, first, k): c = 0.005 stacks/s took 4.7 s (debug).
`:854` loops from `ladder_cap` down; the setting accepted 10¹².

### L11. Output on pile sorters is not checked against the pile sorter's rate
`mixed_belt.rs:1166`: the input side has the check, the output side falls back to "pile" unconditionally.

### L12. A split line may have more places than machines
`mixed_belt.rs:674-680`: 101 machines with a cap of 30 become 4 units of 26. Not stated.

---

## Checked, no finding

- **Two-input formula.** `closed_sum` and the simulation agree on all five of his lines and on over 3 000 further
  layouts (new test compares the closed-form minima with `run_two`).
- **Even positions on two rows.** k and both intervals step by 2; Mk.III proliferator gives 1.5 with k 16 / 18.
- **Period choice** 6 / 32 / 30 / 20 and 24 without a row length. Still fitted (report says so).
- **Ladder groups.** Motor 10, particle container 24 at 9.75 (also uncapped), titanium alloy 12 / 13; surplus
  only above 12.
- **Ingredients.** Taken from the game data: proliferator is not a third ingredient; DSP has no fuel inputs in
  these recipes. All 135 recipes with ≥ 2 ingredients lay out: 2 ingredients → A (44 recipes), 3 / 4 / 5 / 6 → B (42 / 44 / 4 / 1), one ingredient → listed as single. X-ray cracking (hydrogen in and out) is scheme A on its inputs.
  Multi-output recipes get one output sorter per product.
- **Numbers.** No NaN / null in any of 118 152 probe layouts (yellow belt at stack 1 up to blue at 4).
  Zero machines are skipped.
- **390 px, dark theme.** No sideways page scroll; the diagram scrolls in its box.
- **Fitted parameters** are named as fitted in the page header and footer.

## Left open

- M3: control units for green / yellow mixed belts are only flagged, not modelled.
- Search below the largest steady-state period stops after 16 candidates (bounded work); a line that needs a
  smaller period than that reads 不通过 with the machines to add.
- Manual overrides are kept per recipe and mode across plans (as designed).
- `_过程文件/fable-review.md` still carries the 1.5 / 1.5 claim; it is an intermediate file and was not edited.
- P5, in-game calibration of m, ρ, the unit choice and restart time: 「需要 Xinyu」.

---

## Resolution

All 18 findings are fixed except M3, which is flagged in the page instead of modelled.

| # | Fix | Regression test |
|---|---|---|
| H1 | `BeltLayoutSettings::from_value` + `#[serde(deserialize_with = "mixed_belt::lenient")]`: each field rounded and clamped, anything else default. `cleanBeltLayout` (solver.ts) on every write and on load. `mixedBelt()` always returns the lists. Settings bar shown without a plan | `belt_layout_settings_are_read_leniently`; live H1 ×8 |
| M1 | `pick_layout`: feasible = steady state ≥ m, line ≥ m, returns pass; then highest minimum; then the tie-breaks. `search_period`: from the largest steady-state period downwards. Candidates table says "return flow on this line"; a note says why a smaller P was taken | `return_flow_is_part_of_the_choice`; live M1 ×5 |
| M2 | Report §1.3, §2.1, §2.3 (b) corrected (1 / 2 at interval 9, 3 / 3 at 8, 4 / 5 at 7; m rests on one built line) | `processor_intervals_in_whole_positions`; live M2 ×2 |
| M3 | Warning on every scheme-A row when the belt is not blue; report §2.8 | `review5_smaller_fixes`; live M3 |
| M4 | `LINE_MAX` 20 000 machines per unit with a message naming the unit cap; `equivalent_line` (whole periods left out, same start, tail, minima, returns) for the search and the diagram (`trace_omitted`, said under the diagram); saturating totals | `long_lines_are_bounded`, `closed_form_minima_and_equivalent_line_match_the_simulation`; live M4 ×3 |
| M5 | App passes the settings the plan was solved for and `current`; the tab says "Earlier plan" | live M5 ×2 |
| L1 | `recommended` is the result of the same search with no manual choice | `return_flow_is_part_of_the_choice`; live |
| L2 | `units × (refills + 2)` | `long_lines_are_bounded` |
| L3 | lcm(P, rows·R); label and report updated | `review5_smaller_fixes` |
| L4 | Reset whenever a manual value is set; 60 multiples plus the recommended and chosen period | live |
| L5 | Warning with [BV1fXkxYDEbW 77:08] | `review5_smaller_fixes`; live |
| L6 | Unit text from `two.unit`; margin from the report | — (page text) |
| L7 | "Copied" / fallback copy | live |
| L8 | Minima after machines and control units only | `review5_smaller_fixes` |
| L9 | Rounded up within 1e-6 | `review5_smaller_fixes` |
| L10 | Closed-form `steady_mins` (simulation only for an approximated ratio); ladder loop starts at C/Σc; cap ≤ 1000 | `long_lines_are_bounded` |
| L11 | Failure note when the output exceeds the pile sorter | — |
| L12 | The split note states places and machines | — |

Measured after the fixes: Mk.II lines of 1–120 machines all pass; universe-matrix plan 0 of 21 scheme-A rows
fail (4 of 20 before); the fuzz test reports 373 of 3 008 blue-belt scheme-A layouts failing, all for a stated
reason (machine over the belt, no period ≥ m, pile sorter too slow, or a return that no layout avoids).

Flows tested live after the fixes (`live_test_review5.py`, 31 checks, all pass):
- typing 7.5 / −3 / 1e12 / 0.4 / 2.5 into the integer settings, reload, and a link carrying bad belt settings;
- Mk.II proliferator at 27, 29 and 30 machines (period, returns, note, candidates table);
- processors with two rows: auto P 32 (3 / 3), manual P 36 (1 / 2, flagged), Reset;
- a green mixed belt; a 10⁹/min target (failure message, then a unit cap of 300); a 12 000-machine line
  (597 rects, omission stated);
- universe-matrix plan; target changed during a 12 s solve ("Earlier plan" note, rows unchanged, note gone after);
- turbine: recommendation text after fixing σ, Reset, Copy (clipboard read back);
- his Mk.III line with two rows (P 30, 2.25 / 1.5 as his k = 18), odd k by hand;
- an infeasible plan (settings bar stays); 390 px dark for the long line and a manual layout.

Not tested live: L11 and L12 (covered by reading the code; no plan in the test set triggers them), and nothing
in the game itself.
