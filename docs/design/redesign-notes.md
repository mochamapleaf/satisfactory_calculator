# Factory Planner UI redesign (2026-11)

Guide: the open-source Impeccable design skill by Paul Bakaus (pbakaus/impeccable on GitHub). We used its SKILL.md,
`craft-floor.md` (quality floor and refuse list), `operate.md` (product UI depth) and `typeset.md`.
The planner is an **Operate** surface: someone comes to finish a task, so scanability, density and familiar controls
come before expression. Xinyu asked for a minimalist style, which set the direction: quiet palette, one accent,
whitespace and 1px rules, and no decorative fills, shadows or gradients.

Before: `before-*.png`. After: `after-*.png` (light and dark, desktop and 390 px phone width).

## Anti-patterns found (audit of the old UI)

| Finding | Impeccable rule |
|---|---|
| `system-ui` / Segoe stack: the default "assembled, not designed" look | System face as the voice; browser defaults left unthemed |
| Grey screen with one orange accent, used for every selected control, tag and tab | "A grey screen with one accent colour is the category default" |
| Summary stats as a row of same-size bordered cards (big number + small label) | Hero-metric template; cards as the lazy container |
| Machine-graph nodes marked by a 4 px coloured stripe down one side | Ban: "a bar down one side of a rounded box" |
| `×` text glyph as the remove icon | Unicode glyphs standing in for an icon system |
| No `color-scheme`: white scrollbars and controls in dark mode; React Flow minimap and controls stayed white | Browser surfaces must carry the design |
| No `:focus-visible` styles; checkboxes and range sliders in browser blue | States: keyboard focus; theme the native parts |
| Graph hint paragraph overflowed the graph's border box | Layout correctness |
| Header copy in solver jargon ("MILP solver (Rust → WASM)") competing with the game switcher | UX copy: the product's own words; hierarchy |
| Pulsing "solving…" text with no reduced-motion guard | Motion needs a reduced-motion guard |
| Tables boxed, headers grey, tags filled orange: many borders and fills for little meaning | Restrained colour; structure from space |

## What changed

- **Type:** one family, Barlow (Google Fonts, 400/500/600), drawn from road and industrial signage. It is narrow,
  so dense tables fit, and its tabular figures are on for every number (tables, inputs, figures row, Sankey labels).
  CJK names fall back to PingFang SC / Microsoft YaHei. Fixed 1.125 scale (11.5 / 12.5 / 14 / 16 / 19 / 26 px) and
  uppercase micro-labels for the table headers and sidebar sections.
- **Colour:** tokens in OKLCH. Cool graphite neutrals on near-white paper (light) or near-black (dark). There is one
  petrol-teal accent, used only for state: active tab underline, checkboxes, range, focus ring and value bars.
  Amber appears only in warnings. Selected segmented buttons are shown with ink and a hairline, not a colour fill.
  Raw, supplied and output nodes in the machine graph get a full hairline border in their own colour.
- **Layout:** 4 px spacing scale (`--s1`…`--s7`). The header is a plain rule, not a filled bar. The sidebar has no
  tint: sections are split by hairlines with more space above them than below. The tabs are underline tabs and stay
  stuck to the top of the main column. Summary stats are one row of figures between rules, with each label above
  its value. "Buildings" is now an inline list with icons. Tables have hairline rows, no outer box and no header
  fill, and they scroll inside `.tablewrap` on a phone.
- **States:** hover tints, `:focus-visible` ring, themed `::selection`, scrollbar colour, `accent-color` on native
  inputs, and a reduced-motion guard on the solving dot. The remove buttons now use a drawn SVG (`RemoveMark`) with
  `aria-label`. The tabs have `role="tab"` / `aria-selected` and the game switcher has `aria-pressed`.
- **Charts:** React Flow uses `colorMode="system"`, and its minimap, controls, background dots and arrowheads are
  themed from the tokens. Sankey colours still get one hue per item, but saturation, lightness and opacity now come
  from CSS variables (`--sk-sat`, `--sk-node-l`, `--sk-link-l`, `--sk-link-op`) with separate light and dark values.
  Labels get a background-colour halo, so they stay readable where they sit over a band.
- **Copy:** "Optimise for" (was "Cost function"), "Add an item…", "Filter by name", "Reset all settings for this
  game". The header shows "Game data …" (solver details are in its tooltip). Empty and error states now say what to
  do next. The Sankey and graph hints were rewritten in plain sentences.
- **Unchanged:** every class name, the game switcher, the icon-style switcher, the four tabs, and all solver and URL
  behaviour. No page scrolls horizontally at 390 px (checked for all 3 games × 4 tabs × both themes). Only the chart
  and table wrappers scroll sideways.

## How it was checked

`npx tsc -b && npx vite build`, then a Chrome DevTools Protocol script captured every game × tab × theme at
1400 px and 390 px from `vite preview` (port 4173). At each step it also measured `scrollWidth - innerWidth`.
