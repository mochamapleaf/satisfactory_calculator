# progress-impl — memo step 13 (DSP seed implementation)
Branch: worktree-agent-a993b9de598a620ce (reset to refactor 53736ff). Spec: ../00-最终报告.md §9 steps 1–4, 7.

Plan: S1 vendor DoubleUTH generator (Apache-2.0, c050ff0) → S2 galaxy resource table → S3 threshold (单站, report §4.3)
→ S4 multi-system LP (max/target z, stock quotas, gas/oil rates, transport) → S5 Dark Fog drop sources (user rates)
→ S6 WASM API → S7 UI tab → S8 build + live test on :4175 + shots → S9 PROGRESS.md, commit.

- 2026-11-22 S0: read report, reviews, xiaoxiao-hints, 笑笑 note, memo step. Cloned DoubleUTH c050ff02 to scratchpad/ext/dsf.
- S1 done (738ec86): vendor/dsp_seed_gen (data/+worldgen/ of DoubleUTH c050ff02, Apache-2.0, LICENSE.txt + NOTICE.md; fork adds ActualVein.nodes = seeps). Test seed 12345678/32/0.1 equals prototype sample (AngelStern, iron 1553315, oil 26.70/s, totals iron 127.8M, mag 182k, oil 404.8/s, 15 giants). release 0.07 s.
- S2–S6 done (47f22f6): src/planner/dsp_galaxy.rs (generate, S(L), threshold_level, collector_net, DarkFog, thresholds(), plan_galaxy()) + dsp_galaxy_tests.rs (10 tests). WASM dsp_galaxy_json / dsp_thresholds_json / dsp_galaxy_plan_json. LP 3.6k cols 108 ms native.
- S7–S8 done: Galaxy tab (Galaxy.tsx, galaxyWorker.ts, lazy), live test `live_galaxy.py` on vite preview :4175 → 24/24 checks (`shots-web/live-checks.txt`), shots desktop + 390 px light/dark in `shots-web/`.
  Fix found live: the threshold LP took the sidebar objective (Raw) → oil 11/cube; now always fewest buildings (report §4.3) → 2/cube.

## Checks against the report (all pass, native tests + browser)
| Report | Value | Where |
|---|---|---|
| §4.1 S(L) table, 13 values | ±1 | test s_of_matches_report_table |
| §4.2 笑笑 142k, 3.28/cube → L81 ("after 80") | L81 | threshold_levels… |
| §4.3(3) magnets 10/cube → L98 / L94; 3.28 → L77 / L72 | same | tests + browser |
| §4.3 oil fixed recipes A/F/E → L19/25/40 | same | test |
| §4.3(2) all quotas feasible from L6 (ρ 1 / 0.8), oil 2/cube | same | test + browser |
| §4.3 table L81 magnets/cube 4.20 (all magnet stars active) | 4.195 | test |
| §2.3 totals (iron 127.8 M … magnets 0.18 M, oil 404.8/s, 15 giants, gas 8.48/0.40/4.21) | same (native) | tests |
| §5.5 collectors at L81: H 22 860, D 1080, fire ice 11 094 /s | ±0.5 % | test |
| §5.5 z 1000: 1 photon/cube, 20 GW; Mk.III 0.8 → 20 GW gives 1250/min | same | tests + browser |

## MINE (choices the report left open)
- Star systems are the LP sites (report §5.2 "按恒星系聚合"); transport inside a system is free.
- Default systems: birth star, then nearest-first every star adding an uncovered resource kind, max 8; ≤ 12 per LP (pairwise flows).
- Shipping cost: stations = 2·(d + 8)/108 000 per item/min (fit to 笑笑 4 blue belts at 7 ly, 3 at 12 ly; both ends), counted as buildings;
  warpers = 2 per trip / 400 items (笑笑: vessel 400 after two capacity levels; "one per leg" 存疑), consumed at the origin as an LP item.
- Oil stock = seep points − 2500·seeps (report: 10.12 M upper bound; 9.54 M now); oil points per unit 1 at ≤ 0.3×, 0.4/multiplier above (report §3 derivation, 存疑).
- Logs and plant fuel unavailable; df-* items only from Dark Fog drops.
- Dark Fog: user-entered drops per base per minute (no data), scaled by (1+0.04L)/(1+0.04L_measured) for normal items, gated by the BWIKI level table; the 40 % pickup rule and defence costs are not modelled.
- Not modelled: power plants / energy balance (generators are not recipes yet, memo step 8), Dyson sphere power from the seed (user GW cap), multi-period model (§9.6), one-off building materials (§9.5).

## Known
- WASM vs native generator: on seed 12345678 one planet of 145 (star 0 P2, the home ocean planet) differs: oil 24.12 vs 26.70/s, coal 0.80 vs 1.06 M (5 of 717 values; `wasm_vs_native.mjs`). Floating-point maths (wasm libm vs Windows CRT) on a terrain height test; the browser total oil is 402.3/s. Neither build is checked against the game; shown as a note in the tab.

## Review 8 (2026-11-22, docs/review/fable-review-8.md) — what changed here
- WASM vs native: FIXED. Cause: `std` sin/cos/acos/pow = Windows CRT natively, Rust libm in WASM; the planet grid differed in the last bit. `vendor/dsp_seed_gen/src/detmath.rs` (libm crate on every target). Both builds now give the native numbers (26.70 oil/s); `fingerprint` pinned in tests and checked by `wasm_vs_native.mjs` (pass/fail) and `live_review8.py`.
- MINE corrected: stations A = 432 000 (笑笑's belt in BV 18 is 7200/min: 「五条蓝带运力=5×7200」), was 108 000.
- Oil extractors count as buildings; Dark Fog scale uses finished levels (L−1) for both levels; a stock override is not multiplied by "remaining".
- Still MINE / open: systems pooled (笑笑 opens one planet per system), vessel 400 + 2 warpers per trip (episode 11) next to episode-18 belts, miner 60/min × speed, heat values, no override field in the tab.
- Published values checked (pre-Dark-Fog posts): 4 seeds star luminosity + distance, seed 29031989 home iron 2.3 M and oil 49.78/s match; its coal 8.3 M vs 8.59 M does not.
