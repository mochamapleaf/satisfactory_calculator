# Progress: review 4 (solver speed / timeouts / HiGHS / proliferator hint), range a6b189b..ed64b95

## step 1 — read the diff (done)
- Read: lp.rs (whole), wasm_api.rs, solver.ts, worker.ts, highsEngine.ts, App.tsx, TimeInput.tsx, rules.rs hint, Sidebar/Results diff.
- Baseline `cargo test -q` started (log in scratchpad cargo_test_base.log).
- Next: write probes for the suspected defects (HiGHS "proven" with a rejected plan, race plan choice, App stale-run
  handlers, hint with mixed flavours), live browser run on :4173, then docs/review/fable-review-4.md.

## step 2 — probes on the build of ed64b95 (done)
- Live (Playwright, :4173): HiGHS file blocked -> silent, Auto runs microlp twice; quality plan under Auto -> 2 workers;
  390 px -> tab bar 188.5 px while solving; time limit changed mid-solve -> status of the new solve shows the old limit, plan 716 -> 717.
- Rust: `NO_SHRINK` did not switch linear power off (reduction test did not test it); baseline 64 tests pass.

## step 3 — fixes (done, committed a910264)
- Files: src/planner/{lp.rs,rules.rs,solve_tests.rs,tests.rs}, react-frontend/src/{App.tsx,solver.ts,worker.ts,highsEngine.ts,index.css}.
- New tests: reductions_keep_the_optimum_with_modules_beacons_and_power, engine_proof_needs_a_plan_within_the_gap,
  somersloop_budget_without_a_plan_names_the_budget, dsp_proliferator_hint_ignores_modes_the_best_tier_does_not_offer.
- `cargo test -q`: 68 passed, 4 ignored. WASM rebuilt, `tsc -b` clean, `vite build` done (dist is the fixed tree; wasm-pkg and dist are not tracked).

## step 4 — live re-test (done)
- `docs/research/solver-speed/_过程文件/bench/live_test_review4.py <shots dir>`: 24 checks pass on the rebuilt preview.
  Screenshots: `docs/research/solver-speed/_过程文件/shots/review4/`.
- Worker trace (Worker wrapped in the page): Stop during HiGHS terminates the worker; Auto Stop -> lane 0 `stop`, lane 1 terminated; Keep solving -> `resume`.

## step 5 — review written (done)
- docs/review/fable-review-4.md: Critical 0, High 0, Medium 5, Low 10; all 15 fixed; two notes left as they are
  (3x retry saved in settings; `detier` misses one equal-cost lower-power display).
- Not tested live: L1, L5, L6, L7, M1 in the browser (triggers not producible), real phone, Firefox / Safari.
- Next: nothing open in this review. Not pushed (branch refactor).
