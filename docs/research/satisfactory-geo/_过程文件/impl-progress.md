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
- Rail-vs-belt in this instance: rail never wins. Also 60/120/200 HMF/min at S0_m1 and 120 HMF/min at Sm3_0 (time-limited,
  gaps 4-6 %): no hub opened. Products compress at the ore, so only a belt or two crosses cells; break-even needs
  several belts of uncompressible flow over several km (Mk.5 belt 1167 OE/km per 780/min vs double track 1826 OE/km
  + 2 hubs 6980 + loco 720 + 2030 per car).
- New test test_rail_chosen_when_belts_expensive (belts x20, 10 HMF at Sm3_0): optimal 42 s, hubs Sm2_m1+Sm3_0, rail
  only; checker now also verifies car and platform capacity per direction. All 8 tests pass (288 s).
- Cross-process determinism: two CLI solves give identical solution JSON (md5 1896e83c...).
- Running: illustrative sweep --belt-mult 20 --base Sm3_0 --pmax 4 (tag _belts20) to show F moving p*.

## 2026-11-21 G7 illustrative sweep (belts x20, 10 HMF/min at Sm3_0, p=0..4) done
- `--sweep --base Sm3_0 --belt-mult 20 --pmax 4 --time-limit 90 --tag _belts20` -> p_sweep_belts20.json/.png,
  solution_best_belts20.json, solution_map_belts20.png. All 15 solves Optimal, 7.9-40 s, 295 s total.
- cost(p): p=0,1 106452 OE (belt only); p=2 72681 (F 1x) / 69191 (0.5x) / 79661 (2x); flat after p=2. p* = 2 for all
  three F (hubs Sm2_m1 + Sm3_0, one line, no belts). Shows the method; not a statement about the real belt cost.
- Open: shared track segments / candidate paths, power balance, water from open water (currently "anywhere"),
  integer-belt gaps on bigger instances (4-6 % after 120-240 s; LP relaxation of belt counts is weak).
