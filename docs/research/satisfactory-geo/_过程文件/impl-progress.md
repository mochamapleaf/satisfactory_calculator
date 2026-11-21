# impl-progress — satisfactory-geo implementation (G1-G3)

## 2026-11-21 G1 done
- Fetched WorldResourceNodes.json @bb997b42 (HTTP 200, 169 384 bytes) -> data/satisfactory-geo/raw/.
- tools/geo_nodes.py -> data/satisfactory-geo/nodes.json (626 records: 459 node, 118 well, 17 well_core, 31 geyser, 1 deposit).
  Asserts: totals == game-data.json limits (12 items), item ids exist, iron purity 39/42/46, wells 13125 m3/min. Deterministic (md5 stable).
- SOURCE.md section appended. Next: G2 transport.json.

## 2026-11-21 G2 done
- tools/geo_transport.py -> data/satisfactory-geo/transport.json (belts, pipes, rail, train, drone, truck, power; each value has source + confidence wiki/derived/estimate; 1.0 / 1.2 variants for fluid car and truck dock).
- Asserts reproduce report tables: 1793.1, 2247.8, fluid 979.1, under-filled 2027, section 2.2 table points (1370/480/2181/1084), truck 1470/1260, drone 210/1050/130. Output deterministic (md5 stable).
- Next: G3 tools/geo_sites.py.

## 2026-11-21 G3 done
- tools/geo_sites.py -> data/satisfactory-geo/sites.json (39 grid sites, 0 transit needed since the 39 cells are already 8-connected, 119 belt arcs, 12 hubs) + _过程文件/sites.png.
- Hub rule: base site (nearest to node bbox centre, --base overrides) + K//3 top raw-output sites (proxy for the belt-only solve, which needs the solver) + farthest-point sampling. K<=12 asserted.
- Asserts: 39 grid cells, every non-deposit node in exactly one site, supply per item conserved, all grid sites belt-connected, max feed distance <= 1.4 km x 1.3. sites.json md5 stable over reruns. Transit-cell code only exercised trivially (no transit needed at 1 km or 500 m).
- Nothing pushed. Next (not done): G4 Rust data loader for these files.

## 2026-11-21 G4 done (solver timing recorded, bench not rerun)
Measured during the report review (fable-review I5, report section 8.3); same machine, microlp 0.8.0 native release
(opt-level 3), mip_gap 1e-3, 60 s limit, models exported by `scripts/geo_toy_hubk.py` (DEMSCALE=4 POORBASE=1,
machines continuous); HiGHS via scipy.optimize.milp:

| instance | vars / int | microlp | HiGHS |
|---|---|---|---|
| 12 hubs, LP relaxation | 5094 / 0 | 0.6-0.7 s | - |
| 703 pairs, LP relaxation | 21656 / 0 | 40.4 s | - |
| belt only (no rail vars) | 3404 / 270 | 59.1 s to 0.1 % gap | 6.2 s |
| 703 pairs, p <= 0 | 21656 / 1674 | no feasible point in 60 s | 4.8 s optimal |
| 12 hubs, p free | 5094 / 400 | 60 s: 1738.2 (+0.7 %, unproven) | 12.4 s optimal (CARUB 16) / 22.9 s (CARUB 64) |
| 12 hubs, p <= 2 | 5094 / 400 | 60 s: 1754.8 (unproven) | - |

Decision kept: the siting layer uses HiGHS; microlp stays for the totals layer and per-site refinement.
Not rerun: the bench needs a cargo build plus 60 s runs per case and adds nothing new; the real-data HiGHS times
are recorded under G5/G7 below (highspy 1.16.3 is already installed in the user's Python, no install needed).
Next: G5 tools/geo_solve.py.

## 2026-11-21 G5 in progress: tools/geo_solve.py written, first solves
- Model as in report 7.2 (machines continuous, integer belts/pipes per directed arc, 12 hubs / 66 direct lines,
  per-direction train binary + integer cars, car and platform constraints, sum h <= p). Costs in ore equivalents
  (min-raw LP over 1.0 base recipes); building/belt/pipe/rail BOMs from SatisfactoryTools data1.0.json @2192bd15
  (fetched to scratch, values copied into the script), stack sizes from the same file; belt 0.5 unit/m (wiki).
- Fixed: Steel Beam id is Desc_SteelPlate_C (was Desc_SteelBeam_C -> OE 0); OE LP now asserts item exists.
- Default instance 10 HMF/min at base S0_m1, tier 8 (Mk.5 belt chosen as cheapest OE per capacity, 1167 OE/km;
  double track 1826 OE/km; F = 2 stations + 2000 = 3490): belt-only 16710.03 optimal 7.3 s; rail p free 16710.03
  optimal 9.1 s, no hub opened. 60 HMF/min belt-only: 120 s limit, gap 4.5 %.
  [Superseded by R1 below: per_min() scaled machine counts by 60/time; corrected default objective 35 417.31.]
- Next: tests (tools/geo_solve_test.py), then G7 sweep.

## 2026-11-21 G5 tests pass (tools/geo_solve_test.py, 238 s total)
- formulas (RtD 2.903 min / platform 2027 / 2247.8 points), OE (plate 1.5, pipe 3, beam 8), flow conservation via an
  independent checker (and the checker catches a broken solution), p=0 == rail disabled (16710.032051, both optimal at
  gap 1e-6), rail p free <= belt-only, determinism (two solves, identical solution digest), far-demand stress passes checker.
- Solve times (HiGHS 1.16 via highspy, 1 thread): default instance 6.8-10.5 s optimal each; 6674 vars / 580 int / 1483 rows
  with rail, 4352 / 238 / 823 belt-only. 120 HMF/min at Sm3_0: 180 s limit, gap 6.3 %. 60 HMF/min at base: 120 s, gap 4.5 %.
- Next: G7 sweep running (p=0..12, F 0.5/1/2x, 120 s limit each).

## 2026-11-21 G7 sweep done (default instance) + extra test
- `python -I tools/geo_solve.py --sweep` -> _过程文件/p_sweep.json/.png, solution_best.json, solution_map.png.
  39 solves (p=0..12 x F 0.5/1/2x = 1745/3490/6980 OE), all Optimal, 6.4-14 s each, 384 s total. cost(p) is flat at
  16710.03 OE: no hub opens for any p or F. Whole chain sits at the base S0_m1 plus one belt from the next cell.
- Fixed: sweep reused OE rounded to 4 decimals (obj 16709.97 vs 16710.03); OE kept unrounded now.
- Rail-vs-belt in this instance: no hub opens, because nothing needs transport (one belt over 1.2 km). The larger
  HMF runs (60/120/200 HMF/min, gaps 4-6 %) are time-limited incumbents and cannot rank the modes.
  [Corrected in R1: "rail never wins" was wrong. Rail wins above a volume x distance threshold, e.g. 12 000
  plate/min at Sm3_0: rail 95 123.6 vs belts 99 386.2, both proven optimal. See R1 results below.]
- New test test_rail_chosen_when_belts_expensive (belts x20, 10 HMF at Sm3_0): optimal 42 s, hubs Sm2_m1+Sm3_0, rail
  only; checker now also verifies car and platform capacity per direction. All 8 tests pass (288 s).
- Cross-process determinism: two CLI solves give identical solution JSON (md5 1896e83c...).
- Running: illustrative sweep --belt-mult 20 --base Sm3_0 --pmax 4 (tag _belts20) to show F moving p*.

## 2026-11-21 G7 illustrative sweep (belts x20, 10 HMF/min at Sm3_0, p=0..4) done
- `--sweep --base Sm3_0 --belt-mult 20 --pmax 4 --time-limit 90 --tag _belts20` -> p_sweep_belts20.json/.png,
  solution_best_belts20.json, solution_map_belts20.png. All 15 solves Optimal, 7.9-40 s, 295 s total.
- cost(p): p=0,1 106452 OE (belt only); p=2 72681 (F 1x) / 69191 (0.5x) / 79661 (2x); flat after p=2. p* = 2 for all
  three F (hubs Sm2_m1 + Sm3_0, one line, no belts). Shows the method; not a statement about the real belt cost.
  [Superseded by the R1 rerun below (machine counts and tiered supply changed).]
- Open: shared track segments / candidate paths, power balance, water from open water (currently "anywhere"),
  integer-belt gaps on bigger instances (4-6 % after 120-240 s; LP relaxation of belt counts is weak).

## 2026-11-21 revision R1 in progress (machine counts, tiered supply, work weight)
- tools/geo_solve.py: per_min() no longer multiplies by 60/time (game-data.json rates are already per minute;
  machine counts were 60/time too small: x2 HMF, x10 plates/screws, x30 smelters). Default instance 10 HMF/min now
  35417.31 OE optimal (machines 26888, extraction 7122, belts 1407); was 16710.03.
- Supply per node now depends on the tier: miner Mk.1/2/3 (T0/T4/T8), oil T5, wells T8, 250 % clock from T3,
  capped at one belt / pipe of the tier (pure node at T8: 780, not 1200).
- New knobs: work_km (OE of player work per belt-km, pipe-km, track-km), machines_at, cbar_mult (car bound x2,
  check() reports a binding bound); check() now also checks pipe capacity, fluid cars/platforms and recomputes the
  objective from quantities.
- Next: tests with hand values, then reruns.
- R1 tests: new test_machine_counts_hand_values (5 Manufacturers / 25 Assemblers for 10 HMF; 600 Constructors +
  600 Smelters for 12 000 plate), test_tier_supply, test_two_site_closed_form (6.5 km route: 780/min -> one belt
  7583.3 OE; 7800/min -> rail 62 199 OE, 21 cars), test_fluid_pipes (1500 m3 oil at S0_m1). Pass.
- tools/geo_breakeven.py (closed-form table + breakeven.png) and tools/geo_experiments.py (rail on/off pairs).
- Running: full test suite, default sweep, sweep _plate12k (12000 plate at Sm3_0, p 0..4), _belts20, experiments.
- First results: volume at Sm3_0 (all proven optimal): 1500/3000/6000 plate no rail; 12000 rail saves 4.3 %
  (95 124 vs 99 386, hubs Sm1_2+Sm3_0, 5 cars, 5 760/min by rail); 24000 rail saves 7.9 % (3 hubs).
  L9: hubs re-picked from the belt-only flow lose the useful pair -> no rail, 99 386 (sites.json rule is better here).
- Fixed: solution_best*.json dump failed on meta['supply'] (tuple keys) -> excluded; check_obj tolerance 1e-5 rel
  (binaries within the solver's integrality tolerance). Sweeps restarted 14:00 (default, _belts20, _plate12k,
  _plate12k_work250 done: p*=2, 102 701 OE).
- Experiments done (all proven optimal): 9000 plate at Sm3_0 still belt-only (17 belts); work 250 -> rail saves
  7.9 %, work 1000 -> 15.9 %; 10 HMF at Sm3_0 with machines only at Sm3_0: no rail (hauls <= 2 km).
- Next: report section 8.5 + summary item, then full test rerun, commit.

## 2026-11-21 R1 results (corrected model; all numbers below supersede the G5/G7 ones above)
- Default sweep (10 HMF/min at S0_m1, p 0..12 x F 0.5/1/2x): 39 solves all Optimal, 9-17 s each, 470 s total;
  flat at 35 417.31 OE, no hub for any p or F (machines 26 888, extraction 7 122, one belt 1 407). Reason: no
  transport to choose a mode for, not "rail never wins".
- Break-even (tools/geo_breakeven.py -> breakeven.json/png), full Mk.5 belts above which one rail line is cheaper,
  F 1x, work 0, route 1/2/3/5/7/10 km: stack 100: -/31/14/8/6/5; stack 200: -/11/7/5/4/4; stack 500: -/11/7/4/4/3.
  Stack 200 at 3 km: F 0.5x 5, 2x 10; work 250 OE/km 5, 1000 OE/km 4.
- Volume at Sm3_0 (tools/geo_experiments.py volume, all Optimal): plate 1500/3000/6000/9000 belts only;
  12000 rail 95 123.6 vs 99 386.2 (-4.3 %, transport -10 %; hubs Sm1_2+Sm3_0, 3.5 km, 5 cars, 5 760/min);
  24000 rail 209 065.5 vs 227 064.8 (-7.9 %, 3 hubs, private tracks -> conservative).
- p sweep _plate12k (p 0..4): p* = 2 at F 0.5x (91 633.6) and 1x (95 123.6); F 2x p* = 0 (99 386.2).
  _plate12k_work250 (F 1x): p* = 2, 102 700.98.
- work: 12000 plate, work 250: -7.9 %; work 1000: -15.9 %. base_only: 10 HMF at Sm3_0 (machines only there or
  anywhere): no rail (hauls 1.5 km). hubs (L9): hubs from belt-only outflow lose Sm1_2/Sm3_0 pair -> 99 386.2.
- _belts20 (illustration, 10 HMF at Sm3_0): F1x 138 077 (p<=1) -> 103 277 (p=2) -> 98 326 (p=3); F2x p=3,4 hit 90 s.
  Test test_rail_chosen_when_belts_expensive now uses p=2 (p free took >180 s to prove).
- Plots regenerated (machine circles now scale with sqrt(count)). Report 00-最终报告.md: new §8.5, summary item 8,
  §11 items. Next: full test rerun, commit, Resolution (geo) in docs/review/fable-review-3.md.
- Full suite: python -I tools/geo_solve_test.py -> 12 tests pass, 303 s (stress 120 HMF at Sm3_0 stops at 180 s, gap 3.4 %, passes checker). Committed.
