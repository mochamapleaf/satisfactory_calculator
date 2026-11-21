# UI redesign progress (Impeccable-guided)

## 2026-11-21 step 1: audit + before shots
- Done: read Impeccable SKILL.md, craft-floor, operate, typeset refs; built dist; CDP screenshot script
  (scratchpad cdp.mjs: all 3 games x 4 tabs x light/dark x 1400/390) -> before shots.
- Files: docs/design/before-*.png
- Findings: system-ui font, default orange accent on grey, hero-metric cards, one-sided coloured bars on graph
  nodes, unicode x as icon, no color-scheme (white scrollbars in dark), React Flow minimap/controls white in dark,
  graph hint overflows its box, no focus-visible styles, jargon header copy.
- Next: rewrite index.css tokens/typography, fix components + copy.

## step 2: first pass
- Done: new index.css (tokens, Barlow type, steel rail, readout strip, tabs, tables, React Flow theming), header/tabs in App.tsx, RemoveMark svg, chips, copy edits, sankey theme vars.
- Files: react-frontend/index.html, src/{index.css,App.tsx,Icon.tsx,Sidebar.tsx,Results.tsx,FlowGraph.tsx,ItemFlow.tsx,ItemValues.tsx}
- Build: tsc+vite pass.
- Next: screenshot all states and iterate.

## step 3: minimal direction (Xinyu prefers minimalist)
- Done: index.css rewritten: one family (Barlow), no dark rail, no fills/shadows, hairline-row tables, figures row between rules, seg selection by ink, accent only for state. Sankey saturation via --sk-sat.
- Build passes. Do not touch tools/build_icons.py or public/icons (another agent).
- Next: review screenshots m-*.png, fix, write notes, commit.

## step 4: verified + notes
- Done: nowrap first table column, final shots (all games/tabs/themes, 1400+390, no page h-scroll), redesign-notes.md, after-*.png, shots.mjs (CDP screenshot script).
- Next: commit.
