# Progress: applying docs/review/fable-code-review.md

## 1. ItemFlow (H1, M2, M5, L1, L2, L3) - done
- Files: react-frontend/src/ItemFlow.tsx, react-frontend/src/index.css (.slabel height, .loopnote)
- H1: back edges chosen per cycle at the min-rate edge (markBackEdges). Verified: SF Aluminum Ingot 60 -> loop edge is
  30/min Water Aluminum Scrap -> Water; Scrap refinery now right of Alumina Solution refinery.
- M5: dummy nodes per skipped column; sinks at item.layer+1. Verified Iron Plate 20 + RIP 5: band routes around Screw.
- M2/L3: one netting helper recipeFlows() keyed by recipe id for chart + table; loop tag = chart back-edge items + items
  recycled inside a recipe (tag title says which). Factorio U-235 10: U-235 and U-238 tagged (Kovarex recycles both).
- L1: recycle note is a third label line (.loopnote), slot height reserved. L2: tooltips "Recipe (Building)", singular hint.
- Tests: tsc + vite build pass.
- Next: M1/M3/M4/M7/L4/L5/L6/hashchange (frontend), then M6 + L7/L8.
