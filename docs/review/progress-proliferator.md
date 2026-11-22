# Proliferator tier mix (Xinyu 2026-11-21 「为何生产1级2级增产剂不用3级的？」)

## 1. Reproduced (LP, 60 universe matrix/min, all six proliferator modes)
- Buildings / Compute objective: proliferator-1 → Mk.I speed, proliferator-2, circuit board,
  magnetic coil, casimir crystal (+ graphene under Compute) → Mk.I products; all else Mk.III.
- Resources objective: Mk.III everywhere.
- Mk.III only vs all tiers: Buildings obj 96.08 vs 95.56 (91.04 vs 90.47 buildings, 287.7 vs
  273.9 MW, raw 5127 vs 5174); Compute obj 148.85 vs 147.89. Same pattern for proliferator-3 and
  processor targets (+0.9–1.4 %).
- Verdict: not a bug. The LP is already the steady state (it includes the proliferator lines).
  Per spray, Mk.I = 1/12 Mk.I item (1 coal, 1/180 assembler-min); Mk.III = 1/60 of
  (1 CNT + 2 diamond + 4 coal + Mk.III/Mk.II/Mk.I/CNT/diamond machines) ≈ 2.5× the upstream
  buildings for 2× the bonus, so cheap recipes take Mk.I under building-count objectives.
## 2. Fix: hint, no model change
- src/planner/rules.rs `proliferator_tier_hint`, called at the end of `lp::finish`: a plan
  warning lists recipes sprayed below the best enabled tier and says how to force Mk.III
  (untick lower-tier modes; existing Sidebar checkboxes, no UI change needed).
- Test `dsp_proliferator_tier_mix_is_optimal_and_explained` (tests.rs). cargo test -q: 57 passed.
## Next: WASM build, vite build, live check in browser, commit.
## 3. Built and checked live (2026-11-21)
- Hint wording: "in tests this costs about 0.5–1 % more by the objective" (measured 0.5 % universe
  matrix, 0.9–1.1 % processor / Mk.III targets; "well under 1 %" was overclaiming).
- wasm-pack + vite build; vite preview :4173, Playwright (Edge) on the DSP plan, 60 universe
  matrix/min, Buildings, LP: all tiers → hint shown, 5 recipes on Mk.I, 109 buildings;
  Mk.III only → no hint, 39 recipes all Mk.III, 110 buildings. Screenshots
  scratchpad/prolif-mix.png, prolif-top.png. cargo test proliferator: pass.
## Next: commit rules.rs, lp.rs, tests.rs, this file.
