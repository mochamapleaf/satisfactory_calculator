# Review 4 — solver timeouts, model shrink, streamed solve, HiGHS engine, proliferator hint

Date 2026-11-22. Scope: `git diff a6b189b..ed64b95` without the research data dumps: 058f17c (timeout banner,
Solve again / Cancel, `TimeInput`), 169d65a (DSP proliferator tier hint), 52083d0 / d492606 (dominated columns,
linear power, plan always in hand, sliced solve streamed from the worker), 680f0d2 (HiGHS-WASM engine, Auto =
microlp and HiGHS in two workers, plan validation). File:line references are to ed64b95.

Complaint this work answers (Xinyu): solves 「老是超时，也没有按钮重新计算或者调整超时时间」.

## What was run

| Check | Result |
|---|---|
| `cargo test -q` at ed64b95 | 64 passed, 4 ignored |
| `cargo test -q` after the fixes | 68 passed, 4 ignored, 87 s |
| `npx tsc -b`, `npx vite build`, `wasm-pack build --release` | clean; dist rebuilt from the fixed tree |
| Live browser, before the fixes | Playwright (Chromium headless) on `vite preview` :4173, build of ed64b95: HiGHS file blocked (HiGHS and Auto), quality plan under Auto, 390 px width, settings changed during a solve. Evidence quoted in the findings |
| Live browser, after the fixes | 24 checks, all pass: `docs/research/solver-speed/_过程文件/bench/live_test_review4.py`, screenshots in `shots/review4/`. List in the Resolution section |
| Worker message trace | `Worker` wrapped in the page (constructor, `postMessage`, `terminate`, every update): Stop during HiGHS, Stop and Keep solving under Auto |
| New Rust tests written for this review | 4 (below); two of them fail on ed64b95 by construction (M1, M5) |

Counts: **Critical 0 · High 0 · Medium 5 · Low 10.**

No case was found where the dominated-column or linear-power reduction changes the optimum (see "Checked, no
finding"). The defects are in what the page does around the solve: a proof taken on trust, updates of an old solve
reaching the new one, wasted or silent fallbacks, and the status line on a phone.

---

## Medium

### M1. An engine's "optimal" marks the plan in hand as proven optimal, even when the engine's own solution was refused
`src/planner/lp.rs:1214` (`offer_values`), with `react-frontend/src/worker.ts:41` and `solver.ts:357` (`better`).

`offer_values(values, bound, proven)` first checks the engine's values (`rows_hold`, budgets). When they are refused,
or HiGHS returns no solution vector, `best` is still the older plan (for example the rounded LP plan), and
`if proven && self.best.is_some()` sets `proven = true`. The plan then reads "optimal" with whatever gap it has.
Under Auto this is worse: `better()` prefers a proven plan over a cheaper unproven one, and a proof ends both lanes,
so the microlp lane's cheaper plan is thrown away for the dearer one labelled optimal.

Failure scenario: HiGHS ends `optimal` at 145.0 buildings; its vector fails the 1e-6 row check (or
`getSolution` throws); the lane's plan in hand is the rounded plan at 156.7. The page shows 156.7 "optimal", no
banner, no Keep solving. Not reproduced in the browser (HiGHS's vectors passed the check in every run); reproduced
at the API: test `engine_proof_needs_a_plan_within_the_gap` (refused vector, bound halfway to the LP bound,
`proven = true`) ends proven on ed64b95.

Fix: prove only when the plan in hand is within the gap of the engine's bound; otherwise take the bound and stay
unproven.

### M2. Updates of a superseded solve still reach the page
`react-frontend/src/App.tsx:67-86, 87-103`.

After a settings change the effect waits 150 ms before `solveStream` replaces the old solve. During that time the
old solve's handler still runs: it writes `progress`, `plan`, `planKey` and, if the old solve ends in that window,
`setSolving(false)`.

Observed (build of ed64b95, time limit changed from 1.5 s to 8 s during a solve): the status of the new solve read
"Solving of ≤ 1.5 s" in all three runs. By the same path, an old solve that finishes inside the window clears
`solving` for good: the new solve then runs with no status and no Stop button, and "Solve again" is offered while
it runs. Also `extra` (the added time of a restarted Keep solving) was cleared only when the timer fired, so
settings changed in the window inherited both the extra time and the "keep the older plan if cheaper" rule, which
compares plans of two different models.

Fix: a token per solve request; handlers of older requests return at once. `extra` is consumed when the request is
made.

### M3. On a phone the solving status is squeezed into a column and the tab bar grows to 188 px
`react-frontend/src/index.css:191-206` (`.tabs` is a no-wrap flex row; `.solving` is a shrinking item).

Measured at 390 px on ed64b95: tab bar 51 px idle, 188.5 px while solving (the status "Solving 1 s of ≤ 3 s · best
plan so far, within 1.8 % of the bound" breaks into one or two words per line). The plan below jumps by 137 px when
a solve starts and ends, on every solve. In the banner, "Keep solving (+3 s)" and "Use fractional buildings" wrap
inside their buttons (three lines each) because `.bigmodel .row` does not wrap.

Fix: the tab bar wraps; on narrow screens the status takes the line below the tabs, two lines at most, beside Stop;
banner rows wrap and buttons keep their text on one line. After: 112 px while solving, status 45 px.

### M4. The proliferator tier hint blames the tier when the mode was the only choice
`src/planner/rules.rs:375-398`.

The hint counts every recipe sprayed with a lower tier than the best one enabled. With Mk.III enabled for speed
only and Mk.I for extra products, a recipe on Mk.I extra products could not have used Mk.III in that mode. The plan
still says "sprayed with Proliferator Mk.I although Mk.III is enabled … a lower-tier spray needs far fewer upstream
buildings … untick the lower-tier proliferator modes", which is the wrong cause and the wrong advice (unticking Mk.I
removes extra products altogether). Reproduced: 60 universe matrix/min, Buildings, effects
`proliferator-3-speed` + `proliferator-1-products` uses Mk.I extra products and showed the hint.

Also the closing claim "in tests this costs about 0.5–1 % more" rests on one case (0.6 %, with the test asserting
< 1 %).

Fix: count a recipe only when the best tier is enabled in the same mode; state the one measured case.

### M5. The reduction test did not test linear power, and its two LP cases compared a model with itself
`src/planner/lp.rs:246` (`linear_power` ignores `NO_SHRINK`), `:1446` (`if integer && shrink()`),
`src/planner/solve_tests.rs:82-96`.

`model_reductions_keep_the_optimum` is the evidence behind "exact" in the report and in impl-results. `NO_SHRINK`
switched off only `drop_dominated`; `linear_power` was on in both arms, so no test compared the model with power
variables against the one without. The two fractional cases ("linear power on 2439 / 875 columns") set
`integer = false`, where dominated columns are not dropped either: both arms built the identical model, and the
assertion could not fail.

Fix: `linear_power` honours `NO_SHRINK`. The existing 9 MILP cases then really compare against the power-variable
model and pass. New test `reductions_keep_the_optimum_with_modules_beacons_and_power`: 10 more cases on the settings
the reductions act on (Factorio speed / efficiency / productivity modules, beacons 1–8, module cost, Power,
Buildings, Resources; DSP all tiers with proliferator modes under Power, Buildings, Compute; power-only and
resources-only weights). LP bounds agree to 1e-6 in all 10; the 7 that both arms prove agree in objective.
The claim holds; it is now checked.

---

## Low

### L1. Auto picks the proven plan over a cheaper one, and shows a cheaper one as unproven after a proof
`react-frontend/src/solver.ts:357-358, 369-375`. `better()` returns `b` when `b` is proven and `a` is not, whatever
the objectives (up to the 1e-4 gap dearer). In the other direction, when microlp proves and HiGHS's unproven plan is
cheaper by less than the gap, the cheaper plan is shown with `proven_optimal: false`: the banner says "not proven
optimal" and offers Keep solving on a solve both lanes consider finished. Fix: the cheaper plan wins; a proof by
either lane marks the shown plan proven.

### L2. Fractional quality plans and "too large" answers run in both workers
`solver.ts:400-404`. Auto races whenever `integer` is set. A quality plan is solved with fractional machines inside
`solve_start`, in both lanes, and the page waits for both. Observed: 2 workers for `electronic-circuit@4`. An
approved 16 000-column recycling plan runs its minutes-long LP twice, with twice the memory. Fix: no race for
quality plans; a result that needs no search ends the race at the first lane.

### L3. The last step's plan is dropped on Stop
`solver.ts:312, 340-347`. `cancel()` clears `running`, so the update of the step the worker was in (up to 150 ms,
possibly a better plan) is ignored, and the worker has already recorded it as sent. The page shows an older plan
than the worker holds until the next improvement. Fix: the stopped lane still delivers that one plan.

### L4. A failed HiGHS load is silent, and under Auto the second worker repeats microlp
`worker.ts:27, 90`. On a load failure `useHighs = false` makes `engineInfo()` drop `highs_status`, so the page never
learns of it. The lane then runs the full microlp search, which under Auto is the same deterministic search as lane
0. Observed with the HiGHS file blocked: Auto ends at the microlp plan (716) with two workers busy for 4 s and no
word about HiGHS. Fix: the status is kept and shown in the banner; Auto's HiGHS lane ends instead of repeating.

### L5. An exception inside HiGHS discards the plan
`worker.ts:56` (`catch → failed(err)`). An abort or out-of-memory in `runHighs` posts
`{status: 'error'}` as the final plan: with the HiGHS engine the plan in hand is replaced by an error. Not
reproduced (no way found to make HiGHS throw in the browser). Fix: fall back to microlp with the plan kept, drop the
instance so the next solve loads a new one.

### L6. Error plans are incomplete, and a worker that trapped is reused
`worker.ts:26`, `wasm_api.rs:45, 87`, `solver.ts:299-317`. An error plan has only `status` and `warnings`; the
Item flow and Machine graph tabs read `plan.recipes` / `plan.flows` and would throw. After a WASM trap the
module's `RefCell` stays borrowed, and an idle worker is reused for the next solve, so every later solve fails
until the page is reloaded. The worker also has no `onerror`. Not reproduced. Fix: plans are completed from
`emptyPlan()`; an error ends the worker; `onerror` reports.

### L7. A stop that crosses the start of HiGHS leaves it running
`solver.ts:340-347`. `cancel()` looks at `busy`, which is set when the "HiGHS is running" message arrives. A Stop
(or a proof in the other lane) just before that posts `stop` to a worker that is about to block for the rest of the
time limit. Fix: a "busy" message from a stopped solve ends that worker.

### L8. HiGHS's absolute gap can end a search above the 1e-4 relative gap
`highsEngine.ts:51`. Only `mip_rel_gap` is set; HiGHS's default `mip_abs_gap` is 1e-6. DSP objectives are about 0.01
per building, custom weights can be smaller, so a plan can be called optimal at a larger relative gap than
`MIP_GAP`. With M1 fixed the plan would then stay unproven instead; set `mip_abs_gap` near zero so HiGHS goes on.

### L9. "No whole-building plan fits (check the shard budget)" for a Somersloop budget; "No feasible plan" while still searching
`lp.rs:1162, 1278`. Forced Somersloops beyond the budget (Reinforced Iron Plate at 4 Somersloops, budget 1) end with
advice about shards. Until the search ends, the streamed plan has status `infeasible`, which the page shows as "No
feasible plan" although it is still looking. Fix: the message names both budgets; the page says "No whole-building
plan yet; still searching…".

### L10. A new time limit throws the plan in hand away
`App.tsx:43, 87-103`. The time limit is part of the settings key, so typing a new limit in the banner restarts the
search and the first plan of the new run replaces the one on screen. Observed: 716 buildings on screen, limit
changed, 717 shown. For the complaint this change answers, raising the limit should never make the plan worse.
Fix: a search restarted for the same model (only time limit or engine differ) keeps the cheaper plan on screen.

### Left as they are
- "Try again with 3×" writes the tripled limit into the saved settings (4 → 12 → 36 → 108 → 300 s). It is what the
  commit says it does; the limit is visible and editable in the banner and the sidebar.
- `detier` does not recover every equal-cost, lower-power display: Factorio Buildings with module cost showed
  1.04 MW with the reductions and 0.95 MW without, at the same objective (8.1525). Power has no weight there; the
  objective is identical.

## Checked, no finding

- **Dominated columns** (`lp.rs:556-616`): same recipe, same inputs and products per unit of work, `speed ≥`,
  building weight `≤` (with sorter, coater, beacon and module cost), power `≤` when weighted, held items `≤`, same
  overclocking and exponent. Moving work and buildings to the dominating column keeps every row: clock, shards
  (fewer needed), budgets (`held ≤`), power (the segment maximum is n·P·g(x / n·speed), increasing in the clock).
  Satisfactory: every mode overclocks and Somersloop levels differ in products, so nothing is dropped and neither
  reduction applies; shards, clock and Somersloop budgets are untouched. DSP: tiers differ in power, so they are
  dropped only when power has no weight; proliferator modes differ in inputs. Factorio: quality plans are fractional
  and skip the reduction; burner tiers differ in fuel; a module or beacon loadout is never dropped for a faster one
  that holds more.
- **Linear power** (`lp.rs:246, 297-332`): at exponent 1 and clock ≤ 1 every segment is p ≥ P·x/speed + beacons·n,
  p is only in the objective, and no building or recipe has negative power in the six data files (checked by script). Now tested (M5).
- **Plan validation** (`rows_hold`): values are rounded to whole buildings and shards, then every bound, row and
  budget is checked at 1e-6 relative; the objective is recomputed from the model, not taken from the engine.
- **Workers**: at most two exist; a new solve, a proof, or Stop during HiGHS terminates the busy one (trace:
  `terminate` at the click). Playwright's worker count stays at 1 after terminating a blocked worker; the trace is
  the evidence, not the count.
- **Budget fallback / warm start** (d492606): the fallback plan is kept but not used as warm start; `extend` resumes
  the same tree.

---

## Resolution

Fixed in a910264 (code and tests); this file, the live test and screenshots in the following commit.

| Finding | Status | Change | Verified by |
|---|---|---|---|
| M1 | fixed | `offer_values`: proven only when the plan's gap to the engine bound ≤ 2·MIP_GAP | Rust test `engine_proof_needs_a_plan_within_the_gap`; `external_engine_round_trip` still passes |
| M2 | fixed | `App.tsx`: token per request (`live`), `extra` consumed at the request | live: 5 change offsets, status always the new solve; "status never shows the old limit" |
| M3 | fixed | `index.css`: wrapping tab bar, phone rule for the status, wrapping banner rows | live at 390 and 1024 px, screenshots `w390-solving.png`, `w390-final.png`, `w1024-solving.png` |
| M4 | fixed | `rules.rs`: same-mode condition; hint states the measured case | Rust test `dsp_proliferator_hint_ignores_modes_the_best_tier_does_not_offer`; the 169d65a test still passes |
| M5 | fixed | `linear_power` honours `NO_SHRINK`; 10-case test added | `cargo test`: both reduction tests pass |
| L1 | fixed | `better` by objective, `asProven` | not reproducible live; code only, no unit test (the frontend has no test runner) |
| L2 | fixed | no race for quality plans; a direct result ends the race | live: quality plan under Auto, 1 worker |
| L3 | fixed | `Lane.late` delivers the stopped step's plan | live: Stop keeps a plan not worse; the trace shows the stopped step's update arriving after `stop` |
| L4 | fixed | status kept; banner sentence; Auto's HiGHS lane ends | live with the HiGHS file blocked, HiGHS and Auto; screenshot `highsfail-auto.png` |
| L5 | fixed | `highsSearch` catches, falls back, resets the instance | code only: no way found to make HiGHS throw |
| L6 | fixed | plans completed in `Lane`; error ends the worker; `onerror` | code only |
| L7 | fixed | late "busy" message ends the worker | code only (a window of a few ms) |
| L8 | fixed | `mip_abs_gap: 1e-10` | live: HiGHS and Auto runs end as before (DSP Buildings proven at 211) |
| L9 | fixed | message; "still searching" line | Rust test `somersloop_budget_without_a_plan_names_the_budget`; the page line is code only (the case ends in 10 ms) |
| L10 | fixed | same-model restart keeps the cheaper plan | live: limit typed into the banner, plan stays at 713 through the restart; HiGHS restart after Stop likewise |

Flows tested live after the fixes (Chromium, `vite preview` of the rebuilt dist, 24 checks pass):
first plan within 1 s and plans only improving (Satisfactory, 7 targets, all alternates, Auto); end-of-time banner
with Keep solving and the time field; Keep solving; Stop (plan kept, "stopped"); Keep solving after Stop; a new time
limit typed into the banner; Solve again; the four tabs during a solve with no page error; settings changed at five
moments near the end of a solve; HiGHS file blocked under HiGHS and under Auto; HiGHS engine, Stop during HiGHS and
Keep solving (restart); Factorio quality plan under Auto; DSP Buildings proven under Auto; Somersloop budget 20
under Auto; 390 px and 1024 px widths while solving and at the banner.

Not tested live: L1, L5, L6, L7 and the M1 browser path (their triggers could not be produced in the browser), the
"Time limit reached, no plan" banner (no case found that still ends without a plan), a real phone (emulated width
only), Firefox / Safari.
