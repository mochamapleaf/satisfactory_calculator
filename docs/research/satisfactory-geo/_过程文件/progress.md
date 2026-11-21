# progress — satisfactory-geo research

## 2026-11-21 step 0 — context read
- Read docs/research/calculators-survey.md, docs/PROGRESS.md, src/planner/lp.rs header, data/game-data.json.
- Current data: 13 resources with only world-total limits (no coordinates). Belts 60..1200, pipes 300/600.
- Solver: microlp 0.8 MILP (B&B), single site, 4 s default time limit; Buildings objective weak (10–25 % gap) on big chains.
- Next: web survey (numbers, tools, node dataset, literature).

## step 1 — numbers (wiki.gg, opened 2026-11-21)
- Freight Car 32 slots, fluid 2400 m3 (1.2; was 1600). Freight Platform 48 slots, 27.08 s dock lock, 50 MW while (un)loading, 0.1 idle, 2 in/2 out belts.
- Train throughput tutorial: TtF = stack*32/belt + 0.4513 min; table Mk6x2: stack100 RtD 102 s -> 1793/min.
- Loco 120 km/h own power, 25-110 MW, 2000 kN. Drone 9 slots, 50-100 m/s by fuel, takeoff+landing 51 s each, port 100 MW. Truck 48 slots, station 20 MW, 8 s dock (1.2).
- 1.2 Game Modes: Resource Node Randomization (Default/Random type-same-location/Basic|Advanced|Fossil rich), purity options, world seed.

## step 2 — node dataset
- rockfactory/satisfactory-logistics src/recipes/WorldResourceNodes.json (MIT), derived from Hirashi3630/satisfactory_node_heatmap resources/nodes_vanilla.json (MIT, Ficsit Networking extraction, vanilla 1.0).
- 626 records: 459 node, 17 frackingCore, 118 frackingSatellite, 31 geyser, 1 deposit. Recomputed world limits == data/game-data.json exactly (iron 92100 ...). Water wells 13125/min max.
- Script: scratchpad/scripts/cluster.py. 300 m single-linkage: 37 clusters; 1 km grid: 39 cells; 750 m: 67; water wells in 9/39 km cells.
- Next: literature + solver options, then formulation.

## step 3 — literature, solver, toy MILP
- Crossref-verified refs: O'Kelly 1987 EJOR 32(3):393; Campbell 1994 EJOR 72:387; Alumur&Kara 2008 EJOR 190(1):1; Farahani+ 2013 CIE 64(4):1096; Magnanti&Wong 1984 TS 18(1):1; Gendron/Crainic/Frangioni 1999 (10.1007/978-1-4615-5087-7_1); Geoffrion&Graves 1974 MS 20(5):822; Van Roy&Erlenkotter 1982 MS 28(10):1091; Melo+ 2009 EJOR 196(2):401; Contreras+ 2011 OR 59(6):1477; Ernst&Krishnamoorthy 1996 LS 4(3):139.
- highs npm (highs-js) 1.16.0 MIT, unpacked 4.47 MB; LP/MILP, CPLEX LP text input. microlp 0.8: plain B&B, duals, warm start.
- Milestones: belts Mk1 T0, Mk2 T2, Mk3 T4, Mk4+Truck T5, Pipe Mk2 + trains T6, Mk5 T7, Drone + Miner Mk3 + Well pressurizer T8, Mk6 T9; Tractor + Pipe Mk1 T3; Miner Mk2 T4. Space Elevator phases 1-5 parts recorded.
- Power lines: no capacity limit, lossless. Pressurizer 150 MW; satellites 30/60/120. Water extractor 120 m3/min 20 MW, deep water only.
- Toy MILP scratchpad/scripts/geo_toy.py (HiGHS via scipy 1.16.3): 38 cells, 21.5k vars, 1.8k int, 4.2k rows.
  Free hub count: 60 s -> gap 27 %. Fixed p=0/1: optimal 2.3-2.5 s (obj 383 < 518 found by free run). p=2: 16 % @30 s; p=3: 8.6 %.
  Demand x4: p=0 0.5 % @40 s, p=2 18 %; when trains used, 98.7 % of train flow is products not ore.
  Machines continuous: optimal 21 s (p free).
- Next: write report.

## step 3b — transport tables + build costs
- Transport per-car/drone/truck tables computed (inline python; values in report §2). Stack sizes from SatisfactoryTools data1.0.json (ore/ingot 100, plate 200, wire/concrete 500, frames 50).
- data1.0.json has schematics (tier, unlock recipes, cost) and build recipes: track = 1 steel pipe + 1 beam per unit; station, platform, loco, car, belt, drone port costs recorded. Length per unit 存疑.
- Recipe ratios verified in data/game-data.json (iron plate 30->20, wire 15->30, concrete 45->15, steel 45+45->45, beam 60->15...).
- Next: write 00-最终报告.md.

## step 4 — report written
- 00-最终报告.md drafted; scripts copied to _过程文件/scripts/ (geo_toy.py, cluster.py).
- Next: critic subagent (Opus) -> _过程文件/00-最终报告.review.md, then apply fixes.

## step 5 — critic done (review in _过程文件/00-最终报告.review.md); applying fixes
- Toy fixed: per-direction belts, Σh ≤ p, POORBASE option, prints dual bound. Rerun DEMSCALE=4 POORBASE=1, 30 s:
  free p: 2239 (bound 1407, gap 37 %); p≤0: 1739.7 (bound 1732); p≤1: same 1739.7; p≤2: 1866.5 (bound 1423) — failed to find the p=0 solution; p≤3: 1875.6 (bound 1408).

## step 6 — fixes applied, done
- Report rewritten with review fixes: platform-lock cap in train table, item-vs-stack compression, drone T0 204 s, fluid cars separate, rail segment graph + station/headway caveat, p-scan with warm start and bounds, solver wording, file-parser, minor fixes. Toy rerun numbers in §8.1.
- Deliverable: 00-最终报告.md. Intermediate: _过程文件/{progress.md, 00-最终报告.review.md, scripts/}.

## step 7 — fable-review revision (2026-11-21)
- Reran geo_toy_hubk.py (DEMSCALE=4 POORBASE=1, 60 s): 12 hubs relax-mach 1728.14 opt 22.9 s; +CARUB=16 1726.67 opt 12.4 s; all 703 pairs relax-mach 1860.6 bound 1463.7 gap 21 %; 12 hubs int machines 1929.1 bound 1609.3 gap 17 %. Machine constant 1195.6.
- Verified arithmetic: platform under-fill (2027/1858 vs 2248), t_acc 13 s table, truck 1.0 ~1260 vs 1.2 ~1470, cluster.py water wells 966–3819 m from bbox centre.
- Next: rewrite report sections 0, 2, 5, 7, 8, 9, 10, 11.
- microlp spot check (rust 1.91, opt 3): 12 hubs CARUB=16 LP 0.6 s (1555.0); MILP 60 s -> 1738.24 feasible, matches review.
- Writing report: §0–§2 (summary, transport table t_acc 13/30 s, truck 1.0/1.2, fluid 979, map size verified).
- Report rewritten in full (§0–§11): new §2.2 table (t_acc 13/30 s), platform/fluid/direction constraints, candidate hubs K, path selection + double track, weights section, p envelope, §8.1 two tables with transport-part gaps, microlp measured table, HiGHS required, slope scaling refs. Scripts copied to _过程文件/scripts/{geo_toy_hubk.py, microlp_bench/}.
- Next: Resolution section in fable-review file.
- Resolution section appended to 00-最终报告.fable-review.md (all items fixed). Done; nothing committed.
