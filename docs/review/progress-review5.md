# progress-review5 — adversarial review + fixes of the DSP mixed-belt planner (315483a, f20054a)

- step0 (2026-11-22): read report 00-最终报告.md, proto_mixed_belt.py, fable-review.md, progress-impl.md, fable-review-4.md
  (format), mixed_belt.rs, mixed_belt_tests.rs, MixedBelt.tsx, the App/solver/urlState/wasm diffs. Hypotheses listed
  (integer fields break the solve, non-blue belts, processor 1.5 vs 1, machines rounding, huge lines, stale plan,
  recommended vs refine, coaters × units). Next: baseline cargo test, probe tests.
- step1: baseline `cargo test -q mixed_belt` 13 pass. Probes (Rust, since removed) + live probe on :4173 (build of f20054a,
  scratchpad pre_probe.py): R = 7.5 blanks the page (pageerror "reading 'length'"), value persists; Mk.II 27 machines 不通过;
  universe-matrix plan 4 of 20 scheme-A rows 不通过; processors interval 9 best is 1 / 2 (report says 1.5 / 1.5);
  3e6 machines = 40 s; c = 0.005 = 4.7 s (debug).
- step2 (Rust fixes, src/planner/mixed_belt.rs, mod.rs): lenient `belt_layout` deserialization (`from_value`); closed-form
  `steady_mins`; `equivalent_line` / `line_two`; `pick_layout` + `search_period` replace `refine` (return flow is a
  feasibility condition, period searched downwards); `recommended` = layout shown; LINE_MAX 20 000 + shortened trace
  (`trace_omitted`); saturating counts; coaters × units; repeat unit lcm(P, rows·R); non-blue-belt warning; odd-position
  warning; output pile-sorter check; machines rounded up; ladder loop bounded by C/Σc; trace minima.
  Tests: mixed_belt 20 pass (7 new). Next: TS/UI fixes, report fix, build, live test, review doc.
- step3 (TS/UI, report): solver.ts `cleanBeltLayout`, `mixedBelt` never throws / always has lists, `trace_omitted`;
  urlState uses `cleanBeltLayout`; App passes the plan's own settings + `current`; MixedBelt: settings bar without a plan,
  "Earlier plan" note, cleaned writes, Reset always, period list incl. recommended, unit text from the model, Copy with
  feedback + fallback, omission note, tie-break disclosed. Report 00-最终报告.md: processors interval 9 = 1 / 2 (§1.3, §2.1,
  §2.3 b), lcm(P, 2R), return-flow search, non-blue belts. wasm-pack + tsc + vite build clean.
- step4 (live, vite preview :4173, Chromium): original live_test_mixed_belt.py 26/26; new live_test_review5.py 31/31,
  screenshots in docs/research/dsp-mixed-belt/_过程文件/shots/review5/. Next: full cargo test, review doc, commit.
- step5: `cargo test -q` 88 pass, 4 ignored. docs/review/fable-review-5.md written (H1, M1–M5, L1–L12, Resolution);
  docs/PROGRESS.md entry. Committed (explicit paths, branch refactor, not pushed). Left open: control units for
  non-blue belts (flagged only), P5 in-game calibration 「需要 Xinyu」.
