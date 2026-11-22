# Review 7 progress (commit 0372822: whole-building quality plans, machine quality)

## 1. Reading — in progress
- Read: lp.rs (all), quality.rs, diff of data.rs / mod.rs / rules.rs / Results.tsx / Sidebar.tsx / solver.ts, progress-int.md.
- Candidate findings so far (to confirm by test): (a) finish() takes tier shares n_c and the group count from
  the raw solver values / recomputes them, not x/speed and N; beacons and beacon MW use the fractional share,
  the objective charges whole N; (b) no cover cuts -> weak bound; (c) settings upgrade for machine_quality.
- Next: check serde defaults / urlState, ItemFlow / FlowGraph, data coverage of quality_speed, run tests.

## 2. First fixes (uncommitted) — done
- H1 quality.rs: plant recipes (agricultural tower) yield normal products whatever the seed tier
  (vanilla: seed quality is lost; forum p=644983, "Quality Seeds" mod). Test `plant_harvest_is_normal`.
- H2 lp.rs: `point_from` takes a shared tier's share as x / speed (never the solver's slack); `offer`
  recomputes each group's whole machines (`group_machines`, tolerance WHOLE_TOL 1e-6). Test
  `integer_shared_engine_slack_is_not_shown` (fails on old code: "widget@4: share 1 is not its work").
- M1 lp.rs `finish`: beacons and beacon MW at the group's whole machines. Test
  `integer_shared_beacons_at_whole_machines` (old code: 15.87 beacons for 2 machines).
- Machine quality checked: wiki Quality page lists crafting speed + health only for crafting machines
  (AM3 1.25/1.625/2/2.375/3.125 = data); no energy row; beacons −1/6 power per tier (separate setting).
- Next: bench (plastic case after H1), cover cuts if the bound is still weak, UI live, URL clamp.

## 3. Bench after H1 + cover-cut experiment — done
- Bench (4 s, native): plastic@4 all planets is now LP 4.5345 → 4.7312, gap 4.2 %, 234 buildings (the
  7-building / 79 % case was the legendary-seed loop of H1). Other rows unchanged.
- Cover cuts (needed-item closure, Σ N_g ≥ 1 / ≥ ⌈rate / best output⌉) tried and removed: qmod 2673 →
  1764 ms, processing unit 1519 → 1924 ms, plastic gap 4.16 → 4.06 % and 5.5 s instead of 4.0 s. No gain.
- Next: frontend (URL clamp, flow tabs, shared rows), wasm + vite build, live test.

## 4. Frontend, build, live test, write-up — done
- urlState.ts (quality tiers clamped 0…4), FlowGraph.tsx (node sums loadouts, "share of N machines"),
  Results.tsx (alt tag on shared rows). `npm run build:all` clean.
- `cargo test -q --release` 95 / 5 ignored; `cargo test -q` 94 / 6 ignored.
- Live (Playwright, :4173, fixed build): `docs/review/scripts/live_test_review7.py` → `docs/review/shots-review7/`
  (live-log.txt): gear × 3 engines, beacons, quality module × 2 engines + flow tabs, plastic all planets with
  Stop / Keep solving, machine quality + old / bad links, 390 px, dark. All pass, no console errors.
- Written: docs/review/fable-review-7.md (findings + Resolution), PROGRESS.md entry, report §1.1 bullet on plants.
- Left: L4 (MIP gap tolerance vs building count) and cross-recipe recycler sharing are owner decisions.
