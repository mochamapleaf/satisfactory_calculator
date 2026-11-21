# Fixes for review 3 (docs/review/fable-review-3.md), non-geo items
- 2026-11-21 start. Scope: H3, H4, M1, M2, L1-L7 (geo H1/H2/M3-M5/L8/L9 by another agent). Base commit b6c29d5.
- H3 + M1 + L2 done: rules::research_bonus (only recipes named by this set's research, clamp 0..3, NaN -> 0), applied
  whatever productivity_allowed says (tiered + quality loadouts). Tests: scrap_recycling_research (spa + 2x1, Fulgora,
  60 holmium: 6000 scrap / 40 recyclers -> 4000 / 26.67 at +50 %), research_productivity_families (+ -2, NaN, 2q1 leftover).
  Frontend: research hint corrected; withDataset drops research levels for sets without research (DATASETS.research).
- L6 done: level input clamped to 0..300, level shown = max over the family's recipes.
- L1 done: urlState.sanitize() drops fields of the wrong type (arrays/records/numbers/strings) before defaults; tsc ok.
