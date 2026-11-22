# Implementation progress: solver-speed §6 (steps 0–4, no HiGHS)

Plan: `../00-最终报告.md` §6 + review `00-最终报告.review.md` (S1: hint rejection cause unknown, keep best plan in hand).

## 2026-11-21 step B0 — baseline
- Done: `bench/bench_prod.rs` (public `plan()` API, the report's 120 scenarios). Baseline = `git archive HEAD` (c4c6194, src == 058f17c solver)
  copied to scratchpad `before/`, `cargo build --release --example bench_prod`, run `ms=4000` serially under a 600 s stall watchdog.
- Output: `data/impl_before4s.jsonl` (copied when finished).
- Next: model shrink (linear power, dominated columns) in `src/planner/lp.rs`.

## step 0+2+3 Rust — best plan in hand, model shrink, stepped solve (built, tests pass)
- `src/planner/lp.rs`: `linear_power` (no p var / segment rows when power_exp = 1, no overclock), `drop_dominated`
  (+ `detier` display tie-break: same buildings, same weight -> lowest-MW tier), `Run` state machine
  (`start` -> rounded LP plan in hand; `step(slice)`: reweight LPs (each rounded plan offered), restricted MILP,
  full MILP; first MILP call node_limit 0 / no time limit so the hint is checked; integer-only hints;
  every MILP incumbent offered; budget check; `budget_fallback` for Somersloop / shard budgets; `extend` = keep solving).
  `finish` now takes a `Point`. `solve()` = start + step(None) loop.
- `src/planner/solve_tests.rs` (6 tests: reductions keep optimum on 9 MILP + 2 LP scenarios, DSP Buildings provable,
  Somersloop / shard budget plan, 2439-col model at 200 ms keeps plan, stepped solve monotone + extend). `cargo test -q`: 63 pass.
- `src/wasm_api.rs`: `solve_start` / `solve_step(ms)` / `solve_extend(ms)` (thread_local Run).
- Frontend: `worker.ts` step loop (150 ms slices, stop/resume), `solver.ts` `solveStream` / `keepSolving` / `cancelSolve`,
  `App.tsx` streams plans, "Stop" keeps best plan, note banner "Best plan found … within x %" + "Keep solving".
- Next: native after-bench (running), wasm rebuild s vs 3, browser tests.

## step 1 (opt-level) + native after-bench
- Native after (opt s, `data/impl_after_native_s.jsonl`): 105/105 plans (before 104), 45 proven (before 43); DSP plain Buildings 2x proven in 7–8 ms.
  Somersloop Buildings: first fallback (floor caps) 194 -> greedy caps (`budget_fallback`: floor + leftover budget to largest fractions,
  best of greedy/floor/zero) 156.7 (HiGHS 60 s best 145). Somersloop Resources 5443 -> 5725 (deterministic, B&B path; noted).
- opt-level 3 vs s (Node 26, `data/impl_wasm_opt_s_vs_3.txt`): 1.04–1.40x faster (mostly 1.2–1.4); .wasm 1 876 002 -> 2 068 757 B
  (+193 KB; gzip +64 KB, brotli +43 KB). Report's +1.2 MB did not reproduce. Set `opt-level = 3`.
- WASM rebuilt (opt 3), `npx tsc -b && npx vite build` ok. Running: native stepped bench opt 3 -> `data/impl_after4s.jsonl`.
- Scope addition (coordinator, Xinyu 「加」 2026-11-22): HiGHS-WASM engine after 1–4 are committed. highs@1.15.4 (MIT, wasm 3.53 MB / gzip 1.21 MB)
  probed in scratchpad: persistent model, setSolution (MIP start), mipImprovingSolution / mipLogging callbacks. Draft engine in scratchpad `highsEngine.ts`.
- Next: live browser test (`bench/live_test.py`), commit 1–4, then HiGHS.

## step 3 live test + results (done)
- Native after (opt 3, stepped): `data/impl_after4s.jsonl`; summary `impl-results.md`, full table `impl_table_full.md`.
- Live browser (`bench/live_test.py`, Chromium headless, preview :4173): hard SF plan first plan 0.35 s and improving, Stop keeps plan,
  Keep solving resumes, Somersloop budget plan (20/20), DSP compute. Screenshots `shots/`. Fixed after the first run: gap in the card / status
  now follows bound updates; elapsed counter reset at solve start.
- `cargo test -q`: 63 pass. Next: commit, then HiGHS.

## Committed 52083d0 (code) + a017580 (bench/results). HiGHS engine (in progress, uncommitted)
- Rust: `Model` records bounds/rows; `Run::model_lp` (CPLEX LP text), `best_values`, `offer_values` (row + bound check,
  budget check via `offer`), `finish_now`, `heuristics_done`; WASM `solve_model_lp / solve_best_values / solve_heuristics_done /
  solve_offer / solve_finish`. Test `external_engine_round_trip` (7 solve_tests pass).
- Frontend: `highs@1.15.4` (exact), `src/highsEngine.ts` (lazy `import('highs')`, MIP start via setSolution, improving-solution
  and logging callbacks streamed), worker switches to HiGHS after microlp heuristics, `engine` setting (auto/microlp/HiGHS) in
  Sidebar; Stop during HiGHS terminates the worker (plan kept in UI). Dist: highs chunk 101 KB js + 3.53 MB wasm (gzip 1.22 MB), lazy.
- Browser bench: `react-frontend/bench.html` + `src/bench.ts` (dev only), driver `bench/browser_bench.py` (vite dev :5174).
  Smoke: HMF alt bld 42.59 (microlp 42.64), Somersloop bld 148.8 (microlp 156.7), HiGHS load 442 ms.
- Running: full browser bench microlp vs HiGHS, 4 s -> scratchpad bb_*.jsonl. Next: decide auto, live test, commit.

## HiGHS done (to commit)
- Browser bench done (`data/browser_*4s.jsonl`, `browser_table.md`), Auto = HiGHS. Live test `shots/highs/` (fixes: busy signal before HiGHS
  blocks, stopped elapsed, restarted Keep solving keeps the better plan, gap tightened by any same-model bound). cargo test 64 pass.

## Regression fix (coordinator 2026-11-22): Somersloop Resources 5275 -> 5725
- Cause (experiments with env switches, native): the budget fallback plan (6239) was used as the full-MILP warm start; microlp
  searching from it found 5725 in 4 s, searching without a start (as 058f17c did, whose hint was rejected) finds 5275.
  Also the fallback's columns enlarged the restricted MILP (spent its 1 s for nothing).
- Fix in `lp.rs`: `fallback_version`; while the best plan is the fallback, no warm start for the full MILP and the restricted
  MILP does not add the fallback's columns. Native: 5275 / 5251 (3 runs).
- Auto: `solver.ts` now races two workers (lane 0 microlp, lane 1 HiGHS) and shows the better plan at every moment; a proof by
  either ends both. tsc ok. Running: full native bench (after_fix2). Next: wasm rebuild, browser bench microlp vs auto, live check.
- Done: native 105 scenarios 0 regressions > 0.1 % vs 058f17c; browser Auto vs microlp 0 regressions; Auto = race of two workers.
  Live re-check `shots/regression-fix/` (race Keep solving keeps the stopped lane's plan: `raceBest`). cargo test 64 pass.
