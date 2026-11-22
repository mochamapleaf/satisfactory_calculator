# Review 8 — DSP seed: vendored generator, Galaxy tab, thresholds, multi-system LP, Dark Fog sources

Date 2026-11-22. Scope: `git diff 53736ff..41508da` (738ec86 vendored DoubleUTH generator, 47f22f6 galaxy model + WASM
API + Galaxy tab, 41508da threshold LP objective + live test). Spec: `docs/research/dsp-seed/00-最终报告.md`;
MINE choices: `docs/research/dsp-seed/_过程文件/progress-impl.md`. File:line references are to 41508da.

## What was run

| Check | Result |
|---|---|
| `cargo test -q` at 41508da | 97 passed, 5 ignored |
| `cargo test -q` after the fixes | 105 passed, 5 ignored (debug); `--release` 106 passed, 4 ignored; `-p dsp_seed_gen` 3 passed |
| `npx tsc -b`, `eslint` (Galaxy.tsx, galaxyWorker.ts), `wasm-pack build --release`, `vite build` | clean; dist rebuilt from the fixed tree |
| Native vs WASM trace, before | scratch harness (native example + bare wasm32 module run in node): 109 trace lines differ — the planet grid, its index map, the height field of every rocky planet, and the veins of star 0 P2 |
| Native vs WASM trace, after | identical: grid, index map, all height fields, all veins |
| `wasm_vs_native.mjs` (now pass/fail) | 717 values, 0 differ; fingerprints equal for seeds 12345678/32/0.1× and 29031989/64/1× |
| Live browser, after the fixes | Playwright (Chromium headless) on `vite preview` :4175: `_过程文件/live_review8.py` 41/41, screenshots `_过程文件/shots-web/review8/`; the old `live_galaxy.py` 24/24. Flows listed in the Resolution |
| Published in-game values | 4 seeds: star luminosity + distance; 1 seed: home-planet iron and oil. See H1 |

Counts: **Critical 0 · High 1 · Medium 4 · Low 9.** All High and Medium findings are fixed; open items are at the end.

---

## High

### H1. The browser and the desktop build generated different galaxies
`vendor/dsp_seed_gen/src/data/vector_f3.rs:88` (`slerp`: `acos`, `cos`, `sin`), used by `planet_grid.rs:88-90`; then
`planet_raw_data.rs:29` and `planet.rs:1147`. The same `std` calls are in 14 files (81 call sites).

Root cause: the generator calls `f32::sin`, `f32::acos`, `f64::powf`, … from `std`. Natively these are the Windows C
runtime; on `wasm32-unknown-unknown` they are Rust's bundled libm. The two round differently in the last bit
(measured: `0.1f64.powf(0.7)` = `…93c1` natively, `…93c0` in WASM). It is not f32/f64 width, fused multiply-add,
iteration order, hashing or integer width: the star direction, the planet pose and every integer path were
bit-identical in the trace.

How one bit moves a vein: the 161 604 vertices of the planet grid are built with `slerp`, so they differ in the last
bit; every terrain height is sampled at those vertices and truncated to 0.01 (`as u16`); a vein node is kept when the
interpolated height is ≥ the radius. All 105 rocky planets had a different height field; on one of them a node sat on
the threshold.

Failure scenario: seed 12345678 / 32 stars / 0.1×, home planet: 26.70 oil/s and 1.06 M coal natively (the report's
numbers), 24.12 oil/s and 0.80 M coal in the browser. The page a user sees disagreed with the report, the tests and
the prototype; any other seed could differ on other planets.

Fix: `vendor/dsp_seed_gen/src/detmath.rs` (new). Every transcendental function goes through the pure-Rust `libm` crate
(pinned `=0.2.11`) in double precision on every target; the `f32` versions compute in `f64` and round once, which is
what the game's `UnityEngine.Mathf` does. `sqrt`, `floor` and the arithmetic operators are exact IEEE operations and
stay. Result: both builds give the former native numbers (26.70 oil/s; report unchanged).
Guards: `dsp_seed_gen::fingerprint` (FNV over every generated number, bit for bit) is pinned for two seeds in the
native tests, shown in the tab, and checked from the WASM build by `wasm_vs_native.mjs` and the live test.

Against the game (values players published; all before the Dark Fog update, so a spot check, not proof):

| Source | Seed | Published | Generator |
|---|---|---|---|
| Steam discussion 3190239190928374828 (Nekogod, 2022-01-26) https://steamcommunity.com/app/1366540/discussions/0/3190239190928374828/ | 29031989, 64 stars | home planet "2.3m iron", "49.78 oil per second" | 2 333 289; 49.78/s — match |
| same | same | "1.3m copper", "4.8m stone", "8.3m coal" | 1.38 M, 4.86 M (match if truncated); coal 8.59 M — **differs** |
| Gamersky handbook 202102/1365548 (bgli100, 2021-02-25) https://www.gamersky.com/handbook/202102/1365548.shtml | 13636977 | O star 2.43 ly, luminosity 2.45; neutron star / black hole 5.5 / 6.4 ly | 2.43 ly, 2.452; 5.50 / 6.40 ly — match |
| same | 38734423, 39497430, 46676919 | O stars: 2.7 ly 2.49, 3.6 ly 2.18; 5.95 ly 2.47; 6.05 ly 2.44 | 2.71 / 2.489, 3.58 / 2.184; 5.95 / 2.473; 6.06 / 2.438 — match |

Test `published_in_game_values` holds the iron, oil and two of the luminosity rows. The coal difference is unexplained
(typo, game change since 2022, or a real difference); a current save is still needed.

---

## Medium

### M1. Interstellar stations are counted four times too high
`src/planner/dsp_galaxy.rs:666` (`station_a: 108_000.0`), comment at 641-642.

MINE fit "4 blue belts at 7 ly, 3 at 12 ly" took a belt as 1800/min. 笑笑 in that episode (BV 18, 76:44–88:32) says
「五条蓝带运力=5×7200」: his belt is four-stacked, 7200/min. His own total also only works with 7200: about 50 towers
for the five main ores of a 20 000 cube/min factory is 1.08 M/min at 12 ly, against 0.27 M/min with 1800.
Failure scenario: every shipment costs four times the stations; under a building cap the LP avoids shipping and reports
a lower maximum z; station counts in the plan are 4× too high.
Fix: `station_a = 432_000` (28 800/min at 7 ly, 21 600 at 12 ly), source quoted in the code; test
`station_default_is_xiaoxiaos_four_stacked_belts`.

### M2. Number fields clamp on every keystroke: "48" cannot be typed into Stars
`react-frontend/src/Galaxy.tsx:77-80` (`numIn`), used by every number input of the tab.

Failure scenario (reproduced with real key presses): Stars, select all, type 4 → the field becomes 32, type 8 → 328 →
64. Quota level, type "12" → 62. Clearing Cubes/min gives 1, then typing gives 1750.
Fix: `NumField` (same pattern as `TimeInput`): typing is free, the value is clamped and applied on Enter or when the
field is left; an empty Building cap / Dyson GW means none. Live checks M2.

### M3. Results stay on screen after their settings change, with no mark
`Galaxy.tsx:141-155` (results stored without what they were computed from).

Failure scenario: compute thresholds, switch Mk.III on in the sidebar (or change level, target, ρ, systems, Dark Fog):
the table still shows 10 magnets/cube and L98 beside settings whose answer is 3.28 and L77. The plan view has the same
problem.
Fix: each result remembers the payload it was computed from; when the current payload differs, an "Out of date" line
appears and the result is dimmed until recomputed. A late answer after a new import is dropped. The seed fields also
say when they differ from the imported galaxy. Live checks M3, also at 390 px.

### M4. Apache-2.0: changed files carried no notice, and the web build shipped no licence
`vendor/dsp_seed_gen/src/data/planet.rs`, `vein.rs` (§4(b)); `react-frontend/dist` (§4(a)).

`LICENSE.txt` and `NOTICE.md` in the vendored folder were correct (upstream has no NOTICE; the copy is byte-identical;
only the two documented files differed from c050ff02). Missing: a notice in the changed files themselves, and a copy of
the licence with the distributed object form — the WASM in `dist` contains the generator.
Fix: every changed upstream file starts with a `// planner fork: modified from …` line; `NOTICE.md` lists the new
changes; licence and notice are shipped as `licenses/DSP-Seed-Finder-LICENSE.txt` / `-NOTICE.txt` and linked from the
tab. `libm` is MIT, `once_cell` MIT/Apache-2.0. Open for Xinyu (report §2.4, unchanged): the theme tables in
`theme_proto.rs` are game data.

---

## Low

- **L1. A system listed twice counts its stock twice.** `dsp_galaxy.rs:852`: `dedup` only drops neighbours; systems
  `[0, 9, 0]` doubled the home stock and quotas. Not reachable from the tab (it sends a set). Fixed; test
  `a_system_listed_twice_counts_once`.
- **L2. Dark Fog scaling was one level off.** `dsp_galaxy.rs:391` used (1+0.04L) with L = the level being researched,
  while mining speed uses the L−1 finished levels. Now both levels mean "being researched" and the factor is
  (1+0.04(L−1))/(1+0.04(L_m−1)); label and hint say so. Test `dark_fog_scale_counts_finished_levels`.
- **L3. Oil extractors were not buildings.** `dsp_galaxy.rs:978`: the building cap and total left them out. Now one
  extractor per seep, in proportion to the rate taken; shown in the plan line. Test `oil_extractors_are_buildings`.
- **L4. A stock override was multiplied by "remaining %" in the plan but not in the thresholds.** `dsp_galaxy.rs:977`.
  An override is the remaining stock itself in both now. Test `stock_override_is_the_remaining_stock` (笑笑's 142 k →
  3.28 per cube at L81). The tab still has no field for overrides (open).
- **L5. Seed settings were not checked in Rust; the galaxy cache could lock.** `dsp_galaxy.rs:190`, `wasm_api.rs:207`.
  A non-positive multiplier went to the generator; the cache cell stayed borrowed while generating, so a trap would
  have made every later call fail. `SeedSettings::sanitized`, cell not borrowed during generation, level ≤ 10 000,
  non-finite target → 0 with a warning.
- **L6. A worker that fails to load left the tab on "Generating…" for ever.** `Galaxy.tsx:89`. `onerror` now ends the
  pending calls with an error and a fresh worker is made on the next call. Live check (worker file blocked).
- **L7. "10×" is not a game setting.** `Galaxy.tsx:62`. The game has ten settings (upstream `web/src/util.ts:93`);
  removed, a stored 10 falls back to 1×.
- **L8. Fire ice shown as a finite stock only.** 笑笑: 可燃冰 is renewable (gas giants), not in the depletion table.
  The threshold row now carries "+ gas giants"; the plan already took giant fire ice as a rate. The hint also gives his
  five-level buffer (L80 → L85).
- **L9. WASM size.** The generator adds about 0.28 MB to the one WASM every game loads (3.31 MB on `refactor` without
  it → 3.59 MB; 0.79 MB gzip; `libm` +1.4 kB). Not changed: a second module needs a second crate.

## Checked, no finding

- S(L) closed form = brute-force sum; 13 table values; cubes to reach L; 笑笑 142 k / 3.28 → L81.
- LP units: recipes, miners, pumps per minute; oil and gas rates per second × 60; quota row m·ρ·k·S(L) ≤ B·(1−φ)·z is
  per cube and matches report §4.1 / §5.3.
- Photons: 1000 cubes/min → 1000 photons/min × 1200 MJ / 60 = 20 GW; 120 MW per receiver → 6/min. Mk.III: 0.8 per cube,
  20 GW → 1250/min (native and browser).
- Collectors: 8·speed·base, 30 MW own use by heat share, ≤ 40 per giant; L81 totals within 0.5 % of report §5.5.
- Proliferator inputs are in the recipe closure; warpers are produced and burnt at the origin.
- Home planet without silicon / titanium: the theme table of the home planet has none (other planets of the home
  system do).
- Performance: generator 0.11 s (32 stars) to 0.20 s (64) natively, 0.09–0.15 s in WASM, 0.12 s in the browser. LP: 8
  systems 3 310 × 473 in 0.07 s, 12 systems 7 751 × 743 in 0.18 s in the browser (native 12 systems 9 606 × 807, 0.24 s).
- 390 px, light and dark: no horizontal page scroll, with the new marks.

## MINE choices against 笑笑's method or the report (not changed, for Xinyu)

| Choice | 笑笑 / report | Effect |
|---|---|---|
| A star system is one site; all its planets are pooled | 「每个星系只开一颗星」 under a full-strength Dark Fog | Stocks and oceans of a ticked system are overstated for his way of playing. Now said in the hint. Needs planet-level sites to fix |
| Vessel 400 items, 2 warpers per trip | 400 is his episode-11 value; the 7200/min belts of M1 are from episode 18 | Two tech stages mixed: warpers per item may be up to 2.5× too high late. Both are fields |
| Miner = 60/min × speed per building | report §5.1: limit by miners and nodes | Miner counts are rough; node counts are generated but unused |
| Gas heat values 8 / 9 / 4.8 MJ | report: 存疑 | Collector net rates depend on them; not checked in game |
| Surplus items vanish (balance ≥ 0) | 笑笑: 余氢 needs an outlet | Hydrogen surplus is not costed |
| No power plants, lenses, one-off building materials, multi-period | report §9.5–9.6 | As stated in progress-impl |

---

## Resolution

Fixed in this review: H1, M1–M4, L1–L8. Not fixed: L9 and the table above.

New tests: `dsp_seed_gen`: `fingerprints_are_pinned`, `published_in_game_values`; `dsp_galaxy_tests`:
`galaxy_fingerprint_is_the_pinned_one`, `seed_settings_are_sanitized`, `station_default_is_xiaoxiaos_four_stacked_belts`,
`a_system_listed_twice_counts_once`, `dark_fog_scale_counts_finished_levels`, `stock_override_is_the_remaining_stock`,
`oil_extractors_are_buildings`, `twelve_systems_solve_in_reasonable_time`. The station, duplicate-system, Dark Fog
scale and override tests fail on 41508da by construction.

Tested live in the browser (fixed build, :4175, 41/41):
- import of the default seed: 404.8 oil/s, home planet 26.7/s (19 seeps), fingerprint equal to the native test;
- import of seed 29031989 / 64 stars / 1×: fingerprint, old results cleared, home planet 49.78 oil/s;
- licence files served and linked; ten resource settings;
- typing by key presses into Stars, Quota level, Cubes/min, Building cap;
- thresholds without and with Mk.III (10 → L98/L94; 3.28 → L77/L72), fire-ice tag;
- out-of-date marks after level, sidebar proliferator, target, systems and Dark Fog changes, and their removal;
- plan: target 1000 (20 GW, 1 photon/cube), 12 systems, maximum z at 20 GW, Dark Fog drops used;
- worker file blocked → error, not busy;
- 390 px light and dark.

Not tested live: a stock override (no field in the tab), non-default Dark Fog hive settings, the building-cap maximum
(native test only), and any comparison with a current-version save.
