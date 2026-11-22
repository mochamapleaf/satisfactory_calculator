# Review 6 — Satisfactory geo Map tab (siting model in TypeScript, HiGHS worker, p sweep, map view)

Date 2026-11-22. Scope: `git diff a243811..36fd943` (8861758 model port and tests, 994a5ee sweep / worker / Map tab,
36fd943 live-test fixes). File:line references are to 36fd943. Spec: `docs/research/satisfactory-geo/00-最终报告.md`
§7–§8.5, `_过程文件/progress-g6.md`, geo part of `fable-review-3.md`.

## What was run

| Check | Result |
|---|---|
| `npm run test:geo` at 36fd943 | 25 pass, 226 s |
| `npm run test:geo` after the fixes | 31 pass (6 new), 227 s |
| `npx tsc -b`, `npx eslint` (changed files), `npx vite build` | clean; dist rebuilt from the fixed tree |
| Live browser, before the fixes | Playwright (Chromium headless) on `vite preview` :4174, build of 36fd943: 12 000 plate at Sm3_0, tab switch during a solve, supplied-input plan, 7-target late-game plan with all alternates, 390 px dark. Evidence quoted in the findings |
| Live browser, after the fixes | `docs/research/satisfactory-geo/_过程文件/scripts/live_test_review6.py`, screenshots in `_过程文件/shots-web/review6/`. List in the Resolution section |

Counts: **Critical 0 · High 2 · Medium 6 · Low 11.**

The model port itself holds: no difference from `tools/geo_solve.py` was found (see "Checked, no finding"). The
defects are in what the page does around the model: a number formatter that drops zeros, the tab leaving a dead
solve on screen, plans the model cannot site answered with solver jargon or nothing, and two statements in the
summary that can be false after a time-limited solve.

---

## High

### H1. `fmt(x, 0)` drops the trailing zeros of whole numbers: 57 040.3 OE is shown as "5704", 10 s as "1 s"
`react-frontend/src/solver.ts:500-501`, used with 0 decimals at `MapView.tsx:43` (`oe`: every cost), `:210`
(elapsed seconds), `:229` (hub cost), `:263`, `:270`, `:282`, `:293`, `:306`, `:356`.

`fmt` formats with `toFixed(d)` and then strips `0+$`. With `d = 0` there is no decimal point, so the zeros stripped
are the number's own: `fmt(9.7, 0)` = "1", `fmt(0.3, 0)` = "", `fmt(57040.3, 0)` = "5704". Values that are whole to
1e-6 take the other branch and are right, which is why the 12 000 plate case (99 386.19, 95 123.63) looked fine.

Seen live at 36fd943: the status line read `Solving F 1× · p ≤ 2:  s of ≤ 30 s` (empty seconds) and
`… 1 s of ≤ 30 s` at ten seconds. Any cost, flow ("rail pays from …/min", "Mined: …") or bound whose rounded value
ends in 0 and is not a whole number loses a factor of 10 or more — one value in ten.
The function is older than this range and the Plan tab has the same defect (`Results.tsx:66` fractional buildings:
119.7 reads "12"; `:86` solve time 10.3 ms reads "1 ms"), but the Map tab is the first view where almost every
number goes through it with 0 decimals. The fix below covers both.

### H2. Leaving the Map tab during a sweep kills the worker and leaves "Solving…" on screen for good
`MapView.tsx:71` (unmount terminates the worker) with `:38`, `:48-49` (run state kept in the module variable `last`).

`App.tsx:222` unmounts `MapView` on every tab change. The cleanup terminates the worker but the kept state still
has `solving` set. Back on the tab, `useState(last)` shows `Solving F 1× · p ≤ 2: … s of ≤ 30 s` with a Stop
button; no worker exists, and the watchdog returns early at `:83` (`!worker.current`), so nothing ever ends it.
Seen live: workers 2 → 1 on leaving, the status line still counting 5 s and 10 s later. The user waits for a solve
that is not running; only Stop followed by "Plan sites" recovers.

## Medium

### M1. A plan with supplied inputs cannot be sited: "aggregate LP: infeasible"
`MapView.tsx:174`, `:208`; `model.ts:384`; `sweep.ts:44-47`.

With "this plan's" recipes the model gets only the recipes the plan uses. An item the plan takes from Available
inputs has no recipe and no node, so nothing can provide it. Seen live (10 Motor/min with Rotor supplied): the page
shows the raw text `aggregate LP: infeasible` and "No plan found." The yellow note at `:208` says the recipes "may
not be enough", when in fact they never are.

### M2. A plan the map cannot supply runs the whole sweep and ends in "No plan found." with no reason
`sweep.ts:96-115`, `MapView.tsx:220`.

When p = 0 is infeasible (a resource locked at the tier — nitrogen below T8, oil below T5 —, node output too low,
a target above what all nodes give), every later point is infeasible too, since rail only moves items. The sweep
still builds and solves p = 2 … pmax and both F rows, and the page then says only "No plan found." Nothing names
the missing resource or the tier, and with rail off the infeasibility is found by the MILP rather than said up front.

### M3. One failed re-check aborts the whole sweep
`sweep.ts:80`, `:117`.

`check()` throws `CheckError` and the outer `try` turns it into the sweep's error: all later points are lost, and the
message is a JSON array (`["integrality","b|S0_0|S0_m1",2.0000014]`). The check's integrality tolerance (1e-6,
`model.ts:402`) equals HiGHS's own `mip_feasibility_tolerance`, and the balance tolerance is absolute (1e-5) on
flows of 10⁴ items/min, so a time-limited incumbent on a large plan can fail it by rounding alone. One doubtful
point should cost that point, not the sweep.

### M4. Below tier 6 the sweep solves the same model pmax + 2 times
`sweep.ts:96-115`, `model.ts:247`, `:327`.

Rail is off below T6 (`rail = false`: no hub variables, no `p` row), so p = 0, 2, …, pmax and F 0.5×, 2× are one and
the same MILP. At "Hubs up to 4" that is five identical solves, each up to the time limit (2.5 min instead of 30 s on
a large plan), and the page then shows a cost(p) chart and a hub-cost table that mean nothing at that tier.

### M5. "Rail saves 11.2 % against belts only" on a plan with no rail
`MapView.tsx:165`, `:249`, `:262`.

The saving is computed against the p = 0 row. On a plan where solves hit the time limit, the p ≤ 2 solve (warm
started from p = 0) often just finds a better plan that still opens no hub. Seen live on the late-game plan:
cards "0 train hubs · 0 km rail (0 cars)", and under them "p ≤ 2: time limit … Rail saves 11.2 % against belts only
(677 673 OE) … 0/min by rail". The 11.2 % is the gap between two belt-only incumbents, reported as the case for
trains.

### M6. Points that were skipped or found no plan are left out of "best" without a word
`MapView.tsx:161`, `:167-170`, `:94-109`.

`best` is the cheapest row with an objective. A point the watchdog skipped ("stuck"), or one that hit the time
limit without an incumbent, has none and silently drops out; the summary then reads "p ≤ 2: proven optimal" as if
the sweep were complete. The only trace is "not proven" in the hub-cost table. For F 0.5× / 2×, `pStar` starts its
reduce from the first row whatever its objective, so an unsolved row shows **best p\* = 0** — a result that was
never computed.

## Low

- **L1. After a watchdog restart the hub cost shown can be the 0.5× or 2× one.** `sweep.ts:59-69`. Each new worker
  sends `meta` from its first build; a sweep restarted with only the F rows left builds first at F 0.5×, so
  `meta.F` is halved: the table at `MapView.tsx:229` and the break-even notes (`:287`, `:292`) use the wrong F.
- **L2. `worker.onerror` of a replaced worker overwrites the new run.** `MapView.tsx:147`. No check that the
  failing worker is the live one, and `worker.current` is left pointing at the terminated worker.
- **L3. Set-up errors bypass the sweep's error path.** `sweep.ts:40-50`: `new Data`, the hub rule and the OE LPs
  run before the `try`; an exception there surfaces as "The solver worker failed: Uncaught Error: …".
  `oeCache` (`:36`, `:48-50`) never hits: every sweep runs in a new worker.
- **L4. The same recipe set in another order counts as "other inputs".** `MapView.tsx:57-59`. While the planner is
  still refining, `plan.recipes` is re-ordered; the note "result below is for other inputs" came and went during
  one live sweep without any change by the user.
- **L5. Chart tooltip of an unsolved point reads "NaN OE, 0 hubs".** `MapView.tsx:356`. Picking that point then
  says "No plan found." for the whole run.
- **L6. On a phone the map's own text is 5–7 px.** `map.css:13-14`: axis 120 and labels 150 map units of 7 600
  across 350 px.
- **L7. The legend omits the base star and the candidate-hub ring.** `MapView.tsx:416-427`.
- **L8 (left). Buildings without a build cost count 100 OE.** `model.ts:18-27`, `:254`, as in `geo_solve.py`.
  With the plan's recipes a late-game plan uses particle accelerators, converters, quantum encoders and nuclear
  buildings; their machine cost is understated, which leans towards spreading them. Needs the build costs from
  data1.0.json and a decision for the Python model too.
- **L9 (left). Large plans do not finish in the default time.** Late-game plan (7 targets, all alternates, 69
  recipes): 29 742 variables, 4 131 rows; every solve stops at 30 s, the sweep takes 181 s and ends 11–17 % from
  the bound. It works and says "time limit", but the hub question is not answered at that size. Flow variables for
  every item on every arc are the bulk; pruning items by reachability, or the per-site refinement of step G7, is
  the real fix.
- **L10 (left). "Fewest hubs on ties" uses 1e-6, the MIP gap is 1e-4.** `MapView.tsx:161`, same as
  `geo_solve.py:728`. A point with one more hub that is 0.005 % cheaper within the gap becomes "best".
- **L11 (left). The re-check tolerances are HiGHS's own.** `model.ts:402`, `:413`. After M3 a failure only drops
  the point; the tolerances were left as in Python so the two stay comparable.

## Checked, no finding

- **Port against Python.** All 13 cases of `python-ref.json`: same variables / integers / rows, objective within
  the gap, same hubs and cars. Read side by side with `geo_solve.py`: rates per minute (no 60/time factor, the H1
  of review 3), belt 780/min Mk.5 chosen by OE per capacity, pipe 600, stack table, 32 slots, platform feed two
  belts, dock 27.08 s, F = 2 stations + 2 000 = 3 490 OE, track 1 826 OE/km, car + 2 platforms 2 030 OE, detour
  1.3 on straight hub distance, node rate by tier and purity capped at one belt / pipe. Break-even table of
  report §8.5 reproduced by the test.
- **12 000 plate at Sm3_0**, live: 95 124 OE, rail saves 4.3 % against 99 386, hubs Sm1_2 + Sm3_0, 5 cars,
  5 760/min by rail, p\* 2 / 2 / 0, all proven — the Python numbers.
- **Envelope and warm start.** The envelope is the running minimum; the warm start is replaced only by a solution
  on the envelope; a restarted sweep carries the envelope in. A worse later incumbent cannot become "best".
- **HiGHS memory and workers.** Every model is disposed in `finally`; the geo worker is a separate worker with its
  own HiGHS instance and is terminated at the end, on Stop, on restart and (after H2) on leaving. It shares
  nothing with the planner's workers but the CPU.
- **Map.** Drawn from node coordinates only (rect, grid, circles, lines); no game map image. Light and dark use
  the page tokens. No horizontal overflow at 390 px.

---

## Resolution

Date 2026-11-22, same branch. Fixed: H1, H2, M1–M6, L1–L7. Left: L8–L11 (reasons above).

- **H1.** `fmt` moved to `src/fmt.ts` (re-exported by `solver.ts`); zeros are stripped only after a decimal point.
  Test `fmt keeps integer zeros…`.
- **H2.** The unmount cleanup marks the kept run as stopped; back on the tab it reads "Stopped; the points solved
  so far are kept." with "Solve again".
- **M1.** New `supplied` option of `build` / `aggregateGross` (web addition, off in the Python-equal cases): the
  plan's supplied inputs are available free at the base site up to the rate the plan uses; `check` and the summary
  know them. The page says so in a note. An input nothing provides is named before any solve:
  "Nothing on the map makes Rotor with this plan's recipes." (`unmakeable`). Test `supplied inputs…`.
- **M2.** An infeasible p = 0 ends the sweep with the shortfall from the location-free plan against all nodes at the
  tier (`rawShortfalls`): "The map cannot supply this plan at tier 8: Iron Ore 3000000/min needed, 72780/min from
  all nodes." Test `a plan the nodes cannot supply…`.
- **M3.** A `CheckError` drops that point (status "rejected", no plan) and the sweep goes on. Test
  `a solution that fails the re-check…`.
- **M4.** With rail off the sweep is one solve; the page shows "Trains are not unlocked at tier 4…" and no
  hub-cost table. Test `no rail at tier 4…`.
- **M5.** The belts-only reference is the cheapest solved plan without a hub at any p. A shown plan without hubs
  says "No train pays off in this plan"; a rail saving is stated only for a plan with hubs, and says when the
  reference is not proven.
- **M6.** A warning lists every point without a plan and says the best is among the solved ones only; `pStar` is
  "—" / "not solved" when nothing was solved.
- **L1** `meta.F` is always the 1× cost (test `meta.F is the 1× hub cost…`). **L2** `onerror` checks and clears the
  live worker. **L3** set-up runs inside the sweep's `try`; `oeCache` removed. **L4** recipe ids sorted.
  **L5** tooltip and "No plan at this point." **L6** larger map text under 800 px. **L7** legend has "candidate
  hub" and "base".

### Live re-test (Playwright, Chromium headless, `vite preview` :4174, production build of the fixed tree)

`live_test_review6.py`, 20 checks, all pass:

- **12 000 Iron Plate at Sm3_0, desktop light:** 95.1k OE, 2 hubs, 5 cars, 3.5 km rail; "Rail saves 4.3 % against the
  best belts-only plan found (99386 OE)"; cost parts with their zeros (Total 95124, Machines 24600); p\* 2 / 2 / 0
  proven; elapsed seconds always a number; legend entries; tapping a site opens its detail.
- **Tab switch during the sweep:** the geo worker is terminated on leaving (counted in the page); back on the tab:
  "Stopped…", two points kept, "Solve again", which then completes with 95.1k OE. **Stop button:** solving ends,
  all three workers made in that session were terminated.
- **Supplied Rotor, 10 Motor/min:** sited (3.9k OE), note shown, no warning.
- **Tier 4, 600 plate:** one point in 1.6 s, the tier hint, no hub table.
- **2 000 000 plate:** stops in about 1 s with the iron shortfall.
- **Late game** (Space Elevator parts 5–9 at 5/min, Supercomputer 5, Turbo Motor 5, all alternates) **at 390 px,
  dark:** a plan, cost and links; no overflow; status "time limit, within … % of the bound"; no rail saving claimed
  on a plan without hubs; sweep 181 s. 12 000 plate again at 390 px dark: same numbers, no overflow.
- No console or page errors in any case.

Not tested live: the watchdog's stuck-solve restart (it needs a hanging HiGHS; its sweep side — skip, envelope,
`meta.F` — is covered by node tests), and a real phone (390 px emulation only).
