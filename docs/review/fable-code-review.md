# Code review: Factory Planner, `ecbbf77..HEAD` on `refactor` + uncommitted icon tools

Reviewed 2026-11-21. Scope: `react-frontend/src`, `src/planner/data.rs`, `tools/`, `data/*.json`, plus uncommitted
`tools/flat2d.py` and `tools/build_icons.py`. No source files were edited.

**Checks run**
- `npx tsc -b` and `npx vite build`: pass.
- `cargo test -q`: 22 passed, 0 failed (dead-code warnings only).
- `data/*.json` against `ecbbf77`: after removing `group`/`row`/`order` from every item the three files are
  identical to the old ones; every item has all three fields; every item has an icon in the sprite index.
- Browser (preview `planner-dist`, port 4173): Satisfactory aluminium, Factorio Kovarex, DSP X-ray cracking,
  empty target list, item that is both target and supplied input; 375 px width; dark scheme; icons Off.
- `flat2d.redraw` on synthetic icons (transparent, 3x3, single colour, full cell, 1 px line, alpha 100, 32 px) and
  six real Factorio icons.

**Checked and found fine** (no finding)
- `netSelfLoops` deleting/setting the current key while iterating a `Map` is safe in JS; the netted numbers are right.
- The balance table balances after netting in all plans tried (made = used for every item).
- Empty plan renders "Nothing to draw." with no console error.
- Item that is target and supplied input: 40 made + 10 supplied = 30 used + 20 output.
- No horizontal page overflow at 375 px. `--muted` passes AA in both themes.
- `flat2d` does not crash on tiny/transparent icons or when `k` exceeds the distinct colours, and is deterministic
  run-to-run on this machine.

Counts: High 2, Medium 7, Low 8.

---

## High

### H1. Sankey can draw the main product flow as the "loop" and the real recycle as a normal band
- `react-frontend/src/ItemFlow.tsx:69-84` (DFS back-edge detection, root order at line 70)
- Confidence: high (reproduced)
- Scenario: Satisfactory, target Aluminum Ingot 60/min, default settings. The chart reports
  `60/min Alumina Solution: Alumina Solution -> Aluminum Scrap (loop)` as the dashed loop edge, and draws the
  30/min Water return (Aluminum Scrap -> Water -> Alumina Solution) as an ordinary forward band. The Aluminum Scrap
  refinery ends up to the left of the Alumina Solution refinery that feeds it.
- Cause: the DFS marks whichever edge closes a cycle first. Roots are the source nodes in `plan.raw` order, so
  starting from "mined Coal" enters the cycle at the Scrap recipe and the closing edge is the main flow. Which edge
  becomes "the loop" depends on raw-resource ordering, not on which flow is the recycle.
- Fix: choose back edges by rate, not by DFS order. For example: repeat { find a cycle; mark its smallest-rate edge
  as back } until acyclic. A cheaper approximation is to visit each node's out-edges in descending rate and start
  from the source with the largest flow, but the min-rate-per-cycle rule is the one that matches the hint text.

### H2. HEAD offers the "Minimal" icon style but its sprite sheets and generator are not committed
- `react-frontend/src/Icon.tsx:5,9,29` (committed) vs `react-frontend/public/icons/*-mono.webp`, `tools/flat2d.py`,
  `tools/build_icons.py` (untracked / modified)
- Confidence: high (`git ls-files react-frontend/public/icons` has no `*-mono.webp`)
- Scenario: clean checkout or deploy of `a77183e`, choose Icons -> Minimal. Every icon request 404s. Because the
  index JSON still loads, `useIconsOn()` is true, so the item picker renders 32 px buttons with no icon and no text
  (`ItemPicker.tsx:69`); items can only be told apart by hovering. The committed `*-flat.webp` are also the old
  cel-shaded style, not what the working tree shows.
- Also seen during the review: the sheets in the working tree were being rewritten (mtimes 01:02-01:16) and one
  `Image.open` of `factorio-original.webp` failed mid-write, so the icon set was not in a settled state.
- Fix: commit `flat2d.py`, `build_icons.py` and the six flat/mono sheets together, after the regeneration finishes.
  Optionally make `Icon` fall back to text when the sheet image fails to load.

---

## Medium

### M1. Keyboard focus is invisible on checkboxes and range sliders (regression from 61c586d)
- `react-frontend/src/index.css:143-145`
- Confidence: high (computed style checked: `outline: none`, border `0px none` on focused checkbox and range)
- Scenario: Tab through the sidebar. `input:focus { outline: none }` has higher specificity than
  `:focus-visible { outline: 2px ... }`, and `input:focus-visible` only sets `outline-offset`. Text and number inputs
  get a 1 px border colour change; checkboxes and the Max clock slider get nothing. The old stylesheet had no focus
  rules, so browser default rings were present before.
- Fix: drop `outline: none` from `input:focus`, or add
  `input:focus-visible { outline: 2px solid var(--accent); outline-offset: 0 }` (offset 2px for checkbox/range).

### M2. "loop" tag in the balance table is wrong for items a recipe recycles internally
- `react-frontend/src/ItemFlow.tsx:291-312` (`reach` reads `r.outputs`, the un-netted plan data)
- Confidence: high (reproduced)
- Scenario: Factorio, target Uranium-235 10/min. Uranium-238 is tagged LOOP although after netting nothing produces
  it downstream of its consumer (made 29.38 by Uranium processing, used 29.38 by Kovarex). The chart shows no loop
  edge, so table and chart disagree. Uranium-235, which Kovarex really recycles (↻ 391.72), gets no tag.
- Fix: run `reach` on the netted `made`/`used` maps (store each recipe's netted outputs when building them), or
  derive the tag from the chart's back edges so both views agree.

### M3. Swapping a target to an item already in the list silently deletes the row and its rate
- `react-frontend/src/Sidebar.tsx:20-29` (`pick`)
- Confidence: high (code path)
- Scenario: targets Iron Plate 20 and Reinforced Iron Plate 5. Click "Iron Plate", pick Reinforced Iron Plate. The
  Iron Plate row disappears; the result is Reinforced Iron Plate 5, the 20/min is lost with no message. In "add"
  mode, picking an item already listed closes the picker and does nothing.
- Fix: mark items already in the list as disabled in the picker (pass the list's ids), or keep the picker open and
  say "already a target"; do not drop the row.

### M4. Two pickers can be open at once; the outside-click exemption is too broad
- `react-frontend/src/ItemPicker.tsx:38-46`
- Confidence: high (reproduced: 2 `.picker` elements)
- Scenario: open "+ Add an item" under Targets, then click "+ Add an item" under Available inputs. The first picker
  ignores the mousedown because the target matches `button[aria-expanded]`, but that button belongs to the other
  `ItemRates`, whose click only changes its own state. Both pickers stay open, both with `autoFocus`.
- Fix: exempt only the trigger of this picker (pass a ref or id of the opener, or check that the matched button is
  inside the same `ItemRates` container).

### M5. Long bands run straight through intermediate nodes and their labels
- `react-frontend/src/ItemFlow.tsx:86-108` (layering, sinks forced to the last column at line 102), `188-197`
- Confidence: high (reproduced), severity is visual
- Scenario: Satisfactory, targets Iron Plate 20 + Reinforced Iron Plate 5. Iron Plate sits in column 5; its bands to
  the Assembler (column 8) and to "output" (column 10) are single curves that pass over the Screw constructor bar,
  the Screw item bar and both labels. Any link spanning more than one column does this, and pinning every sink to
  the last column creates such links for every early target.
- Fix: insert invisible dummy nodes per skipped layer (standard Sankey/Sugiyama step) so the barycentre pass routes
  long links around nodes; and place a sink one column right of its item instead of at `maxLayer`.

### M6. Satisfactory "Fluids" tab is mostly not fluids; byproduct recipes decide an item's group
- `tools/convert_data.py:55-71` (`SF_GROUP`, `sf_group`), `107-112` (`made_in`)
- Confidence: high (from regenerated `data/game-data.json`)
- Scenario: the Fluids tab holds 39 items including Silica, Empty Canister, Empty Fluid Tank, Compacted Coal,
  Plastic, Rubber, Battery, Cooling System, Fused Modular Frame. Two causes: (1) Refinery/Blender/Packager all map
  to "fluids" regardless of the item's form; (2) `made_in` takes the first non-alternate recipe that lists the item
  among its products, so a byproduct wins: Silica comes from "Alumina Solution" (Refinery) before "Silica"
  (Constructor); Empty Canister from an "Unpackage ..." recipe; Compacted Coal from "Ionized Fuel".
- Fix: prefer recipes where the item is the first product (and skip Unpackage recipes); give solid Refinery/Blender
  products their own group (e.g. "refined") and keep "fluids" for `liquid` items and packaged fluids.

### M7. `--faint` text is below AA; control borders are below 3:1
- `react-frontend/src/index.css:38,71` (`--faint`), `138` (placeholder), `40,73` (`--rule-strong`)
- Confidence: high (computed from the oklch values, sRGB relative luminance)

| Token on `--bg` | Light | Dark | AA text 4.5 | Non-text 3.0 |
|---|---|---|---|---|
| `--fg` | 15.53 | 14.58 | pass | pass |
| `--muted` | 5.81 | 6.93 | pass | pass |
| `--faint` | 3.02 | 3.98 | fail | pass (light is marginal; 2.77 on `--hover`) |
| `--accent-text` | 7.16 | 10.25 | pass | pass |
| `--rule-strong` (input/button border) | 1.69 | 1.85 | n/a | fail |

  On `--raised` (input background): `--muted` 5.98 / 6.57, `--faint` 3.10 / 3.77.
- Scenario: placeholders use `--faint` and carry information here: "∞" for unlimited budgets and limits, the default
  scarcity weight per resource, "1" for building weights. At 3.0-3.1:1 in the light theme they are hard to read.
  Inputs are told apart from the page only by a 1.7:1 border.
- Fix: use `--muted` for placeholders (or darken light `--faint` to about L 0.57 for 4.5:1); raise `--rule-strong`
  on form controls to about L 0.62 light / 0.52 dark for 3:1.

---

## Low

### L1. The ↻ note is appended to the recipe label and is cut off in the chart
- `react-frontend/src/ItemFlow.tsx:49-51`, label box at `232`
- Confidence: high. The Kovarex node label becomes "Kovarex enrichment process · ↻ 391.72 Uranium-235, ↻ 19.59
  Uranium-238" inside a 168 px box with ellipsis, so the note the hint text promises is only readable in the tooltip;
  it also bloats every link tooltip. Fix: keep the note in its own field and render it as a third line or a small
  ↻ badge.

### L2. Tooltips and hint wording
- `ItemFlow.tsx:198-201`: `nodeName` has a three-way ternary whose branches are identical; item and recipe share
  names, so tooltips read "Alumina Solution -> Alumina Solution". Prefix recipes with the building.
- `ItemFlow.tsx:206`: singular case reads "One loop edge ... is drawn as thin dashed lines".
- `ItemFlow.tsx:205` vs `113-116,150`: "each item bar is as tall as its total rate" is not true for nodes with loop
  flow on both sides (height counts forward flow only) or below the 3 px minimum.
- Confidence: high.

### L3. Balance table and chart net and name recipes differently
- `ItemFlow.tsx:276-284` nets per plan row; `build` (40-48) nets per merged recipe id. `who` is
  `name (building)`, which is not unique: Factorio has two "Coal liquefaction" recipes in the oil refinery, so the
  table merges them and `recipeOf` keeps only the last. Also `ItemFlow.tsx:50`: the recipe icon is the first
  output left after netting, which can differ from the recipe's main product.
- Confidence: medium (no wrong number observed). Fix: key by recipe id, share one netting helper.

### L4. Picker accessibility and keyboard gaps
- `ItemPicker.tsx:74-91`. `role="dialog"` but focus is not returned to the opener after Esc or a pick (focus drops
  to `body`); `role="tab"` buttons have no `tabpanel` and no arrow-key movement; search results are capped at 120
  with no "more" note (line 88); Enter picks the first match with no visible highlight of which one that is.
  Enter that confirms an IME composition can pick an item in Safari (check `e.nativeEvent.isComposing`); relevant
  for Chinese DSP names.
- Confidence: high for focus and cap, low for the IME case (not tested).

### L5. Flat/mono icons lose contrast in the dark theme
- `tools/flat2d.py:66-69` (ink is always a dark tone). In dark mode the building icons in the Recipes table and
  dark items (coal) are close to the page colour. Confidence: medium (visual judgement from one screenshot).
  Fix: lighter ink variant per theme, or a faint light backing in dark mode.

### L6. Runtime dependency on Google Fonts
- `react-frontend/index.html` (added in 61c586d). The app was fully self-contained before; now it requests
  `fonts.googleapis.com` (blocked or slow in mainland China, absent offline) and sends a third-party request.
  It falls back to system fonts, so nothing breaks. Fix: self-host the three Barlow weights.
  Confidence: high.

### L7. `flat2d.redraw` robustness details
- `tools/flat2d.py:19-21`: assumes RGBA. An RGB image raises `IndexError`; a P-mode image returns a meaningless
  result without error. Callers pass RGBA today. Fix: `img = img.convert("RGBA")` first.
- `flat2d.py:64`: `iterations=int(UP * 0.45)` is 1 at `UP = 4` but 0 at `UP <= 2`, and scipy treats 0 as "erode
  until nothing changes". Use `max(1, ...)`.
- `flat2d.py:22-24`: icons that are under 50 % alpha everywhere fall back to the untouched original, so they keep
  the game style inside a flat sheet (synthetic alpha-100 square confirmed).
- `flat2d.py:41`: chroma x1.2 is clipped in sRGB, which shifts hue for already saturated icons
  (input 230,40,40 comes out 246,0,21).
- `flat2d.py:7-8` with `build_icons.py:23-27`: the thread env vars are set after `numpy` is imported, so they do not
  limit BLAS. Output is identical run-to-run here; bit-identical sheets across machines are not guaranteed.
- Confidence: high for the first three, medium for the rest.

### L8. `build_icons.py` leftovers
- `tools/build_icons.py:88-110`: `flat()` is now dead, with unused `io`, `ImageFilter`, `ImageEnhance` imports.
- `build_icons.py:124`: sheets are written in place; a reader (dev server, another script) can see a half-written
  file, as happened once during this review. Write to a temp name and `os.replace`.
- Confidence: high.

---

Not in scope but noticed: the app reads the URL hash only at startup, so pasting a different plan link into an open
tab does nothing and the next settings change overwrites it (`urlState.ts:29-33`, no `hashchange` listener).
