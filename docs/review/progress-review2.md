# Review 2 progress (DSP compute preset + Factorio quality), `e1aa030..HEAD`

- 2026-11-21 start. Read: compute.rs, lp.rs/rules.rs/mod.rs/data.rs/wasm_api.rs diffs, quality.rs, quality_tests.rs,
  all frontend diffs, dsp-ups report §4 + model.json. `cargo test -q` started (background).
  Next: factorio-quality report + quality_lp.py, converter, data spot checks, old-vs-new plan regression, browser.
- `cargo test -q`: 37 passed, 0 failed, 2 ignored, 256 s (recycling_scope_and_full_data alone > 60 s in debug).
- Read factorio-quality report §0-§3, §5; converter; spot-checked data/factorio-{spa,2x1,2q1}.json against raw
  (recycling flags, catalysts, module by_quality, base productivity). data/factorio.json vs e1aa030: only
  base_productivity=0, dataset, version/source text and 3 catalyst recipes; data/dsp.json: only base_productivity=0.
- Regression harness: clones of e1aa030 and HEAD in scratchpad, `examples/dump.rs` (every produced item, 4 settings),
  running. Next: compare, independent oracle check (python), frontend build + browser.
- Regression old (e1aa030) vs HEAD, 1505 plans (Satisfactory 428, DSP 472, Factorio 2.0 605; LP + some MILP):
  15 differ, all Factorio uranium/Kovarex chains with productivity modules (intended catalyst fix) + one
  equal-objective tie (electric-engine-unit MILP, power 14.63 vs 14.73 MW). Satisfactory and DSP identical.
- Oracle numbers recomputed independently (numpy closed form): 79.8786, 78.396, 2726.91, 154.7238, ratios 5.031 / 7.0855: match.
- `npx tsc -b` and `npx vite build`: pass.
- Candidate findings so far: base productivity dropped on recipes that disallow productivity modules; "Everything"
  scope has no guard/cancel and is persisted; quality target kept after switching to a data set without quality;
  compute summary uses fractional n; sorters charged on fractionators. Next: experiments (exp.rs in clone), browser.
- Experiments (exp.rs in the HEAD clone): compute lines hand-checked; raw tie-break effect measured; foundry belt
  plan confirms base productivity is dropped; default quality plans on 2x1 sane (no free loops), 0.6-1.7 s native.
- Browser (planner-dist): DSP compute cards OK; preset row overflows sidebar; quality target survives switch to 2.0
  data; "Everything" scope blocks the worker (stale plan, no cancel). localStorage backed up and restored; the saved
  Factorio state on this origin already had recycling "all" before the review.
- DONE. Findings written to docs/review/fable-review-2.md: High 2, Medium 5, Low 9.
