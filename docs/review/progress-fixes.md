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

## 2. Frontend controls (M1, M3, M4, M7, L4, L5, L6, hashchange) - done
- Files: react-frontend/src/{index.css,ItemPicker.tsx,Sidebar.tsx,App.tsx,Icon.tsx}, react-frontend/index.html,
  react-frontend/public/fonts/{barlow-latin-400/500/600-normal.woff2,OFL.txt} (from @fontsource/barlow 5.3.0)
- M1: no global input:focus outline none; checkbox/range keep the 2px ring. M7: placeholders use --muted, light --faint
  0.66->0.62, new --control token (L 0.64 light / 0.52 dark, >=3:1) on input/select borders only.
- M3: items already in the list are disabled in the picker; swap never drops a row. M4: outside-click exemption only for
  openers inside the same list (scope ref). Verified: second list's "+ Add" closes the first picker; 1 picker open.
- L4: focus returns to the opener after pick/Esc; tabs have tabpanel + arrow keys; ">120 results" note; Enter target
  outlined (dashed accent) and named in the footer; IME isComposing guard.
- L5: dark theme drop-shadow halo on .icon-flat/.icon-mono (Icon gets class icon-<style>); sheets not regenerated.
- L6: Barlow self-hosted (latin subset), Google Fonts links removed; build emits ../fonts/ URLs (relative base works).
- hashchange: App reloads settings when the hash changes (verified by setting location.hash without reload).
- Tests: tsc + vite build pass.
- Next: M6 (convert_data.py group logic), L7/L8 (flat2d/build_icons).

## 3. Data groups (M6) and icon tools (L7, L8) - done
- Files: tools/convert_data.py, data/game-data.json (group field only; verified identical otherwise), wasm rebuilt
  (gitignored), tools/flat2d.py, tools/build_icons.py, react-frontend/public/icons/*-{flat,mono}.webp regenerated.
- M6: group from the first standard recipe whose main product is the item, Unpackage skipped; Refinery/Blender solids
  -> "refined" (11 items), fluids now 24 (liquids + packaged). 15 items moved. Picker label "Refined" added.
- L7: convert("RGBA"), max(1, iterations), translucent icons judged by their peak alpha, chroma fitted into sRGB at
  constant hue instead of per-channel clip, BLAS thread vars set before numpy import. Output changed (10/42 sample
  renders) so flat+mono sheets regenerated (original sheets and index json unchanged).
- L8: dead flat() and unused imports removed; sheets and json written to a temp name and os.replace'd.
- Tests: cargo test 22 passed; tsc + vite build pass. Visual: DSP universe matrix 60, SF aluminium light/dark at
  375 px (no horizontal overflow, Barlow loaded from /fonts), dark picker icons with halo.
- Next: Resolution section in fable-code-review.md, final commit.
