# progress-impl — DSP mixed-belt planner P1–P4 (report §4)

- 2026-11-22 step0: read 00-最终报告.md, proto_mixed_belt.py, fable-review.md, progress.md, docs/PROGRESS.md,
  docs/review/fable-review-4.md, planner code (mod.rs, rules.rs, compute.rs, data.rs, wasm_api.rs) and the
  frontend (App.tsx, solver.ts, worker.ts, urlState.ts). Plan: src/planner/mixed_belt.rs (+ mixed_belt_tests.rs),
  PlanSettings.belt_layout (excluded from the solve key so changing it does not re-solve), wasm mixed_belt_json,
  MixedBelt.tsx tab. P5 = 「需要 Xinyu」(in-game calibration). Next: write the Rust core.
- step1 (P1): src/planner/mixed_belt.rs (settings, per_machine/find_mode, simulator, run_two/best_for/choose_period,
  design_two, unit_kind, feed_options/ladder, sorters, coaters, layout_for_plan) + mixed_belt_tests.rs (13 tests: all
  prototype asserts, return flows, search, period choice, ladder, simulator blocking, overrides, plan-level scheme A for
  sprayed Mk.II proliferator, turbine output full at 16, motor ladder 10, 180-machine sorter count 382 in 360–400,
  solved plan end to end). PlanSettings.belt_layout added (serde default). cargo test mixed_belt: 13 pass.
  Finding: processors with two rows per belt: interval 9 positions reaches only 1/2 under the even-position rule (report
  §2.3(b) says 1.5/1.5, true for one row); the planner picks interval 8 (P=32), inside his 7–9.
- step2 (P2 Rust): wasm_api.rs mixed_belt_json(settings_json, plan_json). Next: full cargo test, wasm-pack, TS types.
- step3 (P2 TS + P3 UI): solver.ts types/defaultBeltLayout/mixedBelt(); urlState.ts sanitises belt_layout (merged with
  defaults, overrides kept); App.tsx: 'Mixed belts' tab (DSP only), solve key excludes belt_layout (no re-solve);
  MixedBelt.tsx (settings bar, recipe table, belt diagram SVG, flow lines with m / C, sequence text, manual override,
  candidates, ladder feeds, sorters, coaters, checks with citations); index.css tokens light/dark. tsc + vite build clean.
  Layout runs on the page thread (fast) instead of a worker message (deviation from report P2, no solve involved).
  Next: live browser test on vite preview :4173.
- step4 (live test, round 1): live_test_mixed_belt.py on vite preview :4173 (Chromium): 25/26. Turbine returns came out
  10.5/18 (an equally good σ/k with a larger return). Fix: `refine` tie-break among equal steady-state minima —
  returns within shares and a known unit first, then equal minima, then smaller total return (MINE, only breaks ties).
  Rust test added (turbine returns 4.5/12). Also: m label moved off the 0 label; ladder diagram no margin shading.
- step5 (live test, round 2): 26/26 pass. Cases (his rates): Mk.II proliferator 30 machines → A, P 6, min 4.5/4.5,
  returns 12/13.5 (sum 25.5 > 12 → two-green-belt unit), sequence as report §3; overrides σ 4/5, k 5, P 9 (S −1.5 +
  warning), Reset; no re-solve on override / belt change; carbon nanotube 32 → P 32, 2/2, his layout (G first at 8,
  σ 4/5) returns 8/6 → double unit; Mk.III proliferator two rows R 15 → P 30, min 1.5; processors two rows → P 32;
  turbine 60 → P 20, 3.75, returns 4.5/12, output belt full at 16; electric motor → B, 10 × 3; green belt → P 3 fail;
  390 px: page width 390, diagram scrolls in its box; dark theme screenshot. Shots in shots/.
- P4 done in the same code: sorter counts per recipe (mixed / separate / solver estimate), not written back to the LP.
  180 Mk.II proliferator machines → 382 (his 391, test range 360–400).
- P5 (in-game calibration of m, ρ, unit choice, restart time, re-spray): 「需要 Xinyu」 — needs the game / a sandbox save.
