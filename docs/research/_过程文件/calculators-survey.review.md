# Review: calculators-survey.md (critic pass, 2026-11-20)

Method: shallow clone of factoriolab/factoriolab (HEAD, read `src/solver`, `src/data/schema`,
`public/data/{dsp,sfy,2.0,spa}/data.json`); GitHub API for licences/deps of every listed repo; Factorio
mod-portal API; satisfactory.wiki.gg, wiki.factorio.com, lua-api.factorio.com, wiki.biligame.com/dsp pages.
Status: 已核实 = confirmed from source; 存疑 = not confirmed / partly wrong; 不存在 = does not exist as cited.

## A. Severity HIGH (change code or licence decisions)

1. **FactorioLab "simplex or interior point selectable" — wrong.** `GlpkAlgorithm = 'simplex' | 'exact'`
   (`src/state/preferences/glpk-algorithm.ts`). The second option is GLPK's exact rational simplex
   (`glp_exact`), not interior point. Fixed.
2. **Factorio legendary Quality module 3 = 6.2 %, not 6.25 %.** Quality wiki: quality-module effects round
   *down* to 0.1 %, other modules to 1 %; FactorioLab `spa` data stores `0.062`. The survey left it open and
   implied the scaling is exact (+30 %/level). The evaluator must apply the floor rounding
   (e.g. Prod 2 legendary 6 %·2.5 = 15 %, but uncommon 6·1.3 = 7.8 → 7 %). Fixed.
3. **"Nobody does whole-building MILP" overclaim.** GaiusJ/Satisfactory (已核实, README read) is a full
   MILP: integer machines, power shards (McCormick linearisation), Somersloop groups with budget and the
   quadratic power penalty, APA (bilinear, McCormick), generator/fuel choice, nuclear waste, belt/pipe
   limits. Its gaps: requires the *Linear Overclock* mod (vanilla c^1.321928 not modelled), needs a
   commercial Gurobi licence, and has **no licence file** (code not reusable). The differentiator is
   narrower than claimed: vanilla clock-power curve + integer buildings + free in-browser solver. Softened.
4. **Licences that block reuse.** GaiusJ/Satisfactory: none (all rights reserved). jinxOAO/DSPCalculator:
   none. greeny/SatisfactoryTools: MIT code, but the README/LICENSE restrict the Coffee Stain images in
   `www/assets/images/items` to the original repo. Foreman2: non-SPDX "Other" licence. FactorioLab
   `icons.webp` are game art (© Wube/Youthcat/Coffee Stain) even though the repo is MIT. Survey updated.
5. **FactorioLab data versions** (affect "gives DSP data immediately"): `dsp/data.json` is game
   **0.10.29.21950** (older than dsp-calc's 0.10.31.24710; Dark Fog lens patched in by hand),
   `sfy/data.json` is **Satisfactory 1.2**, `spa`/`2.0` are **2.0.77**. Our own Satisfactory data is 1.0.
   Added.

## B. Severity MEDIUM (wrong or misleading facts)

6. Satisfactory Logistics: repo is rockfactory/satisfactory-logistics, **MIT**, solver **HiGHS**, pure LP
   (the `GENERAL` integer section is commented out in `SolverContext.ts`). Survey said "unknown/[unverified]".
7. YAFP: solver is **glpk.js** (`client/src/utilities/production-solver`); last commit **2023-07**, i.e.
   pre-1.0 (no Somersloops/1.0 data in the repo). Licence **MIT** (GitHub API). Survey said "LICENSE not shown".
8. YAFC-CE moved to **Yafc-CE/yafc-ce** (GPL-3.0); backend confirmed `GLOP_LINEAR_PROGRAMMING`
   (`Yafc.Model/Data/DataUtils.cs`). [unverified] → 已核实.
9. Kirk McDonald: `solve.js` imports `simplex.js` + `rational.js` → own simplex in exact rationals. 已核实.
10. Missing major tool: **Foreman 2.0** (DanielKote/Foreman2, desktop Factorio 2.0 graph planner, Google
    OR-Tools). Also **Rate Calculator** mod (rates of placed machines, MIT). Added.
11. DSP pile sorter: the wiki lists 600/300/150 cargo/min (10/5/2.5 per s) and **7200 cargo/min = 120
    cargo/s** at 1 grid for "改良6"; all sorter figures are *cargo* (stack of 1–4) per time. Clarified.
12. Factorio stack inserter: "research raises 1 → 2 → 4" — wiki: the Stack inserter research sets max belt
    stack 2, two further researches raise it to 4 (2 → 3 → 4); SA-only; introduced 2.0.7. Fixed.
13. dsp-calc's 28 Vitest scenarios compare against **its own pre-refactor output** (commit d533…), not
    in-game measurements; useful as a regression cross-check only. Build-order wording fixed.
14. DSP labs: extra-products-only applies to **research mode**; matrix production in labs allows both
    modes (wiki). Fixed.

## C. Severity LOW

15. Fractionator power formula gives < 720 kW below 30 items/s (linear in flow), not "720 kW then rising";
    proliferator factor = 1 + power bonus (2.5 for Mk.III). Clarified.
16. Effect clamps: Module wiki confirms only the 20 % floors (speed, energy, pollution). "quality ≥ 0" and
    any productivity lower bound are not stated there → 存疑, check in game/`data.raw`.
17. Beacon: FactorioLab `profile` is 1/√n truncated to 4 decimals and ends at n = 100 (0.1); use the
    profile table, not exact √n. √n rule since 2.0.7 已核实; applies to base 2.0 too (profile in `2.0` data).
18. Quality tier wording "legendary(5 levels)" → legendary = level 5.
19. Mixed-belt sorter bound assumes one sorter per input; with k sorters it is `d_i ≤ k·rate·stack`.
20. Research productivity +10 %/level confirmed for steel, scrap recycling, research (wiki); others 存疑.
21. eurydia/project-dsp-calculator renamed to Eurydia/project-aquamarine (MIT); HuangFuSL/dsp-calculator
    is MIT ("Desktop DSP calculator", language not checked); 122474363/DSQ GPL-3.0.

## D. Value audit

| Claim | Status |
|---|---|
| FactorioLab MIT, glpk-ts, 14 games, `public/data/{dsp,sfy,2.0,spa}/data.json`, cost keys, `profile`, `silo`, `rocketCapacity`, `locations`, fluid cost 1/10 (flag `fluidCostRatio`), packager `noCostMultiplier` | 已核实 |
| FactorioLab "interior point" | 不存在 (it is `exact`) |
| DSP prolif Mk.I/II/III: +12.5/20/25 % extra, +25/50/100 % speed, +30/70/150 % power, sprays 12/24/60, self 13/28/75, point 3 = 22.5/75/110 % | 已核实 (wiki + FL data) |
| DSP min-level rule | 已核实 (wiki) |
| DSP belts 6/12/30 /s, ×4 stack | 已核实 |
| DSP sorters Mk.I–III, pile 10/5/2.5 | 已核实 (cargo/s) |
| Fractionator 1 %/2 % (Mk.III), 720 kW, formula | 已核实; Mk.I/II not on wiki (FL uses speed effect → 1.25 %/1.5 %, 存疑) |
| Ray receiver 6/12/24 per min, lens 0.1/min, Dark Fog lens | 已核实 in FL data; Dark Fog lens is real (v0.10.35, per dsp-calc README) |
| Factorio module table | 已核实 except legendary Q3 (6.2 %) |
| Beacon 1.5–2.5, 480–80 kW, 2 slots, 9×9, speed/eff only, √n since 2.0.7 | 已核实 |
| max_productivity 3.0, allow_productivity false, allow_quality true | 已核实 |
| Quality chain 0.9q/0.09q/0.009q/0.001q, +30 %/level, AM3 3.125 | 已核实 |
| Recycler 25 %, 1/16 time, speed 0.5, 4 slots, no prod, fluids lost, self 25 % | 已核实 |
| Belts 15/30/45/60; stack inserter +33 %, hand 16 | 已核实 |
| Silo 100/50 parts, SA part 1+1+1, ≈27 s, 4 slots | 已核实 (27.20 s base, 26.95 s SA) |
| Sloop slots 1/2/4, output ×(1+f/s), power ×(1+f/s)²·c^1.321928, 13.43× | 已核实 |
| Generators linear clock | 已核实 |
| Nuclear 2500 MW, 240 m³/min, rods/waste | 已核实 |
| APA 500 MW, +10 %/+30 %, 5 matrix/min, additive, applies incl. own 500 MW, 10 sloops | 已核实 |
| Belts 60–1200, pipes 300/600 | 已核实 |
| AWESOME coupon `250·(⌈n/3⌉−1)²+1000`, waste not sinkable | 已核实 |
| Factory Solver interior point, MIT; Factory Planner matrix solver, MIT; Helmod MIT | 已核实 (mod portal) |
| SFTools MIT + HiGHS; dsp-calc MulanPSL-2.0; dspqc GPL-3.0 + javascript-lp-solver; Kirk Apache-2.0 | 已核实 |
| SCIM closed, Satisfactory Planner, Satisfactory Modeler details | 存疑 (not re-checked) |
