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
