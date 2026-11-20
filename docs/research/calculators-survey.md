# Production-planner survey: DSP, Factorio (2.0 / Space Age), Satisfactory 1.x

Date: 2026-11-20. Purpose: inform one multi-game planner built on the existing Rust→WASM MILP core
(`src/planner/lp.rs`: integer buildings, shards, Somersloops, custom linear cost, duals).

Legend: **[opened]** = I opened the URL and the claim comes from it. **[code]** = read from the repo
source. **[unverified]** = from memory or a search snippet only; check before relying on it.
Licences were re-checked through the GitHub API / Factorio mod-portal API on 2026-11-20.

---

## 1. Existing calculators

### 1.1 Cross-game

| Tool | Solver | Optimises | Notes | Data | Licence |
|---|---|---|---|---|---|
| **FactorioLab** — https://github.com/factoriolab/factoriolab [opened] | GLPK compiled to JS/WASM (`glpk-ts`, `src/solver/solver.ts`) [code]; `GlpkAlgorithm` = `simplex` (float) or `exact` (GLPK exact rational simplex) [code]; LP only, no integer variables | Weighted LP. Cost keys [code]: `factor`, `machine`, `footprint`, `recycling`, `unproduceable`, `excluded`, `surplus`, `maximize`; per-recipe `cost` | 14 games incl. Factorio, DSP, Satisfactory. Objectives: output / input / maximize / limit, on items or recipes. Modules, beacons with `profile`, quality, belts, inserters, wagons, rocket silo, DSP proliferator, Somersloop. Exact rational arithmetic in the model | `public/data/<game>/data.json` (+ `defaults.json`, `hash.json`, `i18n/*.json`, `icons.webp`). Games: `spa` (Space Age), `2.0`, `dsp`, `sfy`, plus many mod sets [code]. Data versions [code]: `spa`/`2.0` = Factorio 2.0.77, `sfy` = Satisfactory 1.2, `dsp` = DSP 0.10.29.21950 (older than dsp-calc; Dark Fog lens added by hand). Factorio mod data via the *FactorioLab Export* mod (https://mods.factorio.com/mod/factoriolab-export, MIT, mod-portal API [opened]) | MIT (code + JSON); `icons.webp` is game art © the developers |

FactorioLab is the closest existing thing to the planned system and its JSON schema (`src/data/schema/*.ts`)
is a proven multi-game model. See §4.

### 1.2 Factorio

| Tool | Solver | Optimises / features | Data | Licence |
|---|---|---|---|---|
| **Kirk McDonald calculator** — https://github.com/KirkMcDonald/kirkmcdonald.github.io [opened] | Own simplex in exact rationals (`solve.js` imports `simplex.js`, `matrix.js`, `cycle.js`, `rational.js`) [code] | Rates, modules, beacons, belts, fuel, Sankey/graph view | `factoriodump` (separate factorio-tools repo) dumps data + sprites | Apache-2.0 |
| **YAFC-CE** — https://github.com/Yafc-CE/yafc-ce (moved from shpaass/yafc-ce) [opened] | Google OR-Tools, `GLOP_LINEAR_PROGRAMMING` (`Yafc.Model/Data/DataUtils.cs`) [code] | Built for deep recursive mod packs (Pyanodon). Milestones, automatic module filling, per-object *cost analysis* (cost of each item via optimal recipes), accessibility / flow analysis, dependency explorer | Runs the mods' Lua data stage itself (`data.data_crawler`) | GPL-3.0 |
| **Factory Solver** (mod) — https://mods.factorio.com/mod/factory_solver [opened] | Interior-point LP | Handles loops incl. quality recycling, modules, beacons, quality, research productivity; power, pollution; generators/boilers/mining/labs/reactors/spoilage as *virtual recipes*; upper/lower/equal bounds on machines and items; external source/sink recipes priced as inputs; fluid temperatures split | Live game prototypes; imports Factory Planner / Helmod / YAFC | MIT |
| **Factory Planner** (mod) — https://mods.factorio.com/mod/factoryplanner [opened] | Sequential by default, optional matrix solver | Subfloors, modules per line, timescale, mining productivity, 2.0/Space Age | Live prototypes | MIT |
| **Helmod** (mod) — https://mods.factorio.com/mod/helmod [opened] | Not stated on page (matrix/simplex solver **[unverified]**) | Ingredients, machines, modules, beacons, power; multiplayer sheets | Live prototypes | MIT |
| **Foreman 2.0** — https://github.com/DanielKote/Foreman2 [opened, README] | Google OR-Tools (LP) | Desktop (.NET, Windows) node-graph planner, Factorio 2.0 vanilla preset, mod presets | Exported game data | Non-SPDX "Other" licence (LICENSE.md) — check before reuse |
| **Rate Calculator** (mod) — https://mods.factorio.com/mod/RateCalculator [mod-portal API] | None (no solver) | Rates of an already placed selection of machines | Live game | MIT |

### 1.3 Dyson Sphere Program (DSP)

| Tool | Solver | Features | Data | Licence |
|---|---|---|---|---|
| **戴森球计划量化计算器 (新UI版)** — https://github.com/DSPCalculator/dsp-calc, online https://dsp-calc.pro/ [opened] | Hybrid: finds the *cycle-critical items*, runs an LP (simplex) only on them; single-path items are resolved by recursion. The topological order is meant to drive dynamic programming for automatic proliferator choice (planned) | Proliferator modelled as **point layers (增产点数)**, not as levels, to support mixed spraying later; custom proliferator cost planned | `data/` in a `game_data` format with an `item_graph`; mod data importable in the same format | MulanPSL-2.0 |
| **idkwhodatis/dsp-calc** (React 19 rewrite of the above) — https://github.com/idkwhodatis/dsp-calc [opened] | Same LP | Multi-source lines (fixed rate or fixed factory count), byproduct linking, shared orbital collection (H₂/D₂), belt + sorter logistics estimate, sorter level −1..6, mod data incl. Dark Fog, **28 regression scenarios** (Vitest fixtures) | Game data v0.10.31.24710, 241 recipes | MulanPSL-2.0 |
| **dspqc** — https://github.com/PingGuojiang/dspqc [opened] | `javascript-lp-solver` for full chains | Separate **混带 mode**: capacity of one mixed-belt recipe segment (see §2.1.4) | `src/data/dsp_vanilla.json` (from dsp-calc, MulanPSL-2.0) | GPL-3.0 |
| **DSPCalculator** in-game mod (jinxOAO) — https://thunderstore.io/c/dyson-sphere-program/p/jinxOAO/DSPCalculator/ [opened] | Uses MathNet.Numerics; method not stated | Factory counts, raw needs, **generates a blueprint** of the line; Genesis Book / MoreMegaStructure compatible | Live game | Repo https://github.com/jinxOAO/DSPCalculator, **no licence file** (GitHub API) → all rights reserved, do not copy code |
| Others (search results; licences via GitHub API) | – | `122474363/DSQ` (old 量化计算器, continued; GPL-3.0), `makuwa8992/DSP_web_quantizing_calculator`, `qsdaqsdafqr/GenesisBookCalculatorTool`, `HuangFuSL/dsp-calculator` ("Desktop DSP calculator", MIT), `Eurydia/project-aquamarine` (renamed from eurydia/project-dsp-calculator; MIT) | – | features **[unverified]** |
| FactorioLab DSP — https://factoriolab.github.io/dsp | GLPK | Proliferator as module pairs (see §2.1.1) | `public/data/dsp/data.json` [code] | MIT |

### 1.4 Satisfactory

| Tool | Solver | Features | Data | Licence |
|---|---|---|---|---|
| **SatisfactoryTools** (old) — https://github.com/greeny/SatisfactoryTools [opened] | Not stated in README (server-side solver **[unverified]**) | Archived; replaced by SFTools | `Docs.json` → `yarn parseDocs` → `data.json`; icons via UE Viewer | MIT for code/data (LICENSE [opened]); the Coffee Stain images in `www/assets/images/items` may **not** be reused by forks/derivatives |
| **SFTools** — https://github.com/SatisfactoryTools/SFTools [opened] | **HiGHS** LP | Editable graph (AntV X6), codex, map, custom game versions + mods, account sync, Tauri desktop | Data/icons from `api.new.satisfactorytools.com` | MIT |
| **Yet Another Factory Planner** — https://github.com/hamuz/yet-another-factory-planner, https://yafp.game.gottselig.ca/ [opened] | In-browser LP, `glpk.js` (`client/src/utilities/production-solver`) [code] | **User-editable resource weights**; goals per item rate, recipe count or "maximize from resources"; AWESOME points as goal; nuclear; hand-gathered inputs; power, build area, build-material estimate. Last commit 2023-07, i.e. **pre-1.0** (no Somersloops in the repo) | `satisfactory-docs-parser` (lydianlights) | MIT (GitHub API) |
| **Satisfactory Logistics** — https://satisfactory-logistics.xyz, repo https://github.com/rockfactory/satisfactory-logistics [code] | **HiGHS**, pure LP (the `GENERAL` integer section is commented out in `src/solver/algorithm/SolverContext.ts`) | Multi-factory input/output tracking, per-node Somersloop + overclock, 1.1 data, disabled factories, PWA (features from changelog, search snippet) | ? | MIT |
| **SCIM** — https://satisfactory-calculator.com [wiki listing opened] | ? | Interactive map; save-file viewer/editor; production planner | ? | closed **[unverified]** |
| **Satisfactory Planner** — https://satisfactoryplanner.net [wiki listing] | ? | Tier 0 → nuclear chains, 4 languages | ? | **[unverified]** |
| **Satisfactory Modeler** — https://satisfactorymodeler.github.io/docs/ [opened], Steam app 3187030 | Not described; user places and connects recipes, sets limits, tool computes flows; modes Full/Basic/Manual/None | Desktop, visual | ? | Not open source as far as found |
| **GaiusJ/Satisfactory** — https://github.com/GaiusJ/Satisfactory [README opened] | **MILP (Gurobi, `gurobipy`)** | Max AWESOME points/min over the whole map: integer machines, power shards (McCormick linearisation of machines×shards), Somersloop groups with global budget and quadratic power penalty, APA (bilinear, McCormick), generator + fuel choice, nuclear waste, belt/pipe limits. **Assumes the *Linear Overclock* mod** (vanilla c^1.321928 power not modelled); needs a Gurobi licence | Resource nodes by purity | **No licence file** → all rights reserved |
| Tool list — https://satisfactory.wiki.gg/wiki/Tools [opened] | | Also lists Satisfactory Factories, Satisfactory Hub, FactorioLab | | |

**Takeaways**
- LP is the norm (GLPK, HiGHS, OR-Tools/GLOP, interior point). Among the tools checked, the only
  whole-building MILP with shard/Somersloop trade-offs is GaiusJ's Gurobi script, and it relies on the
  Linear Overclock mod, a commercial solver and has no licence. What stays distinctive here is narrower:
  integer buildings with the **vanilla** c^1.321928 clock-power curve, in a free in-browser solver, for a
  general objective (not only sink points). Not an exhaustive search of all tools.
- Weighted multi-term costs (FactorioLab cost keys, YAFP editable weights) match our `CostWeights`.
- Factory Solver's "everything is a recipe" (generators, mining, labs, spoilage as virtual recipes) and
  its source/sink recipes are the cleanest way to bring power generation into the LP.

---

## 2. Game-specific mechanics

### 2.1 Dyson Sphere Program

#### 2.1.1 Proliferator (增产剂)
Source: https://wiki.biligame.com/dsp/%E5%A2%9E%E4%BA%A7%E5%89%82 [opened]; FactorioLab `dsp/data.json` [code].

| Points (增产点数) | Proliferator | Sprays per item (自喷涂) | Extra products | Speedup | Power |
|---|---|---|---|---|---|
| 0 | none | – | 0 | 0 | 0 |
| 1 | Mk.I | 12 (13) | +12.5 % | +25 % | +30 % |
| 2 | Mk.II | 24 (28) | +20 % | +50 % | +70 % |
| 3 | (mixed) | – | +22.5 % | +75 % | +110 % |
| 4 | Mk.III | 60 (75) | +25 % | +100 % | +150 % |

Rules to implement:
- The effect depends on **points per ingredient item**. A recipe uses the **lowest** level among its
  ingredients; unsprayed counts as 0. Points 3 only arises from mixing (wiki table row without a name).
- Per recipe the user picks **extra products** (产出 × (1+e), same time) **or speedup** (time ÷ (1+s)).
  Both raise power by the same factor.
- Speedup only: antimatter/mass-energy, X-ray cracking, reforming refine, recipes using buildings as
  ingredients (belts, sorters, assemblers, smelters, chemical plants…), solar sail, small carrier rocket,
  graviton lens in the ray receiver; fuel rods and accumulators get more energy, not more items.
  Labs (矩阵研究站) in **research mode**: extra products only (= more research progress per matrix);
  labs *making* matrices allow both modes (wiki [opened]).
- Proliferator consumption: every sprayed ingredient item uses 1/sprays proliferator, so per recipe run
  `Σ ingredient_count / sprays` (60 for Mk.III, 75 if the proliferator itself is Mk.III-sprayed).
  This creates a cycle (proliferator needs sprayed coal etc.) → must be in the LP, not post-processed.
- FactorioLab encodes it as two module-like "effects" per tier: `proliferator-3-products`
  `{productivity: 0.25, consumption: 1.5, sprays: 60, limitation: "productivity"}` and
  `proliferator-3-speed` `{speed: 1, consumption: 1.5, sprays: 60}`; `limitation` lists recipes allowed
  extra products. dsp-calc uses point layers instead to allow partial/mixed spraying.

#### 2.1.2 Belts and sorters
Sources: https://wiki.biligame.com/dsp/%E4%BC%A0%E9%80%81%E5%B8%A6, https://wiki.biligame.com/dsp/%E5%88%86%E6%8B%A3%E5%99%A8 [opened].
- Belts: Mk.I 6/s, Mk.II 12/s, Mk.III 30/s. Stacking up to 4 items per cargo (集装物流系统 tech) → Mk.III ×4 = 120/s.
- Sorters (**cargo**/s at 1 / 2 / 3 grid distance; the wiki table is in cargo/min): Mk.I 1.5/0.75/0.5;
  Mk.II 3/1.5/1; Mk.III 6/3/2; Pile sorter 10/5/2.5 base; one cargo holds 1–4 identical items. The wiki
  lists the fully upgraded pile sorter (改良6) at 7200 cargo/min = **120 cargo/s** at 1 grid (×4 items
  per cargo); 2/3-grid values for that level are not listed. Whether Mk.I–III sorters carry a stacked
  cargo whole is not stated **[unverified]**.
- dsp-calc stores the sorter tier as −1..6 (none / Mk.I…pile + upgrades).

#### 2.1.3 Special buildings
- **Fractionator (分馏塔)** — https://wiki.biligame.com/dsp/%E5%88%86%E9%A6%8F%E5%A1%94 [opened]:
  rate is *throughput-bound*, not time-bound: D₂ out = H₂ belt flow × 1 % (2 % with Mk.III-sprayed
  H₂; Mk.I/II not on the wiki — FactorioLab applies the speed effect, i.e. 1.25 % / 1.5 % **[unverified]**);
  unconverted H₂ passes through. Power (wiki formula, linear in flow, = 720 kW at 30 items/s, less below):
  `((items/s − 30)·50 + 1000) × prolif_factor × 720 kW / 1000`, prolif_factor = 1 + power bonus
  (2.5 for Mk.III); standby 18 kW. Check: 120 items/s → 3.96 MW (9.90 MW with Mk.III). FactorioLab models it as recipe
  `in {hydrogen: 0.01}, out {deuterium: 0.01}, time 1` with machine speed = belt speed (`MachineJson.speed`
  undefined ⇒ "speed is based on belt speed") [code]. Series fractionators lose ~1 % each (wiki).
- **Ray receiver (射线接收站), photon mode**: FactorioLab uses fixed recipes of 6 / 12 / 24 critical
  photons per minute (no lens / graviton lens / Dark Fog lens; the Dark Fog graviton lens is a v0.10.35
  item, 1 graviton lens + 12 Dark Fog matrix) [code]; lens use 0.1/min as machine `consumption` [code] and Mk.III spraying doubles the lens effect (wiki [opened]). Real output also depends on
  Dyson sphere power, continuous-reception progress (×(1+1.5·progress)) and ray-transmission tech; the
  wiki's 480 MW / 48-per-min figures are under specific assumptions **[treat as approximate]**.
- **Orbital collector / gas giants, oil extractor, water pump, miners**: "mining" recipes whose rate
  depends on vein count / planet multipliers and mining-speed research (not opened; **[unverified]**
  specifics). dsp-calc has a *collection multiplier* setting and shared H₂/D₂ orbital collection [opened].
- **Logistics stations (PLS/ILS)**: not production; relevant for drone/vessel capacity & stacking.
  Not researched in detail **[unverified]**.

#### 2.1.4 混带 (mixed belt) calculation — what it computes
dspqc's mode [opened] takes one recipe, building, belt tier, actual stack layers, sorter settings,
proliferator strategy, a redundancy factor (e.g. 110 %) and the input feed ratio; it outputs the
**maximum number of machines one mixed belt can supply**, the feed sequence, entrance capacity margin,
and a 2-D sketch. The underlying arithmetic (my formulation, consistent with that description):
- Per-machine input demand `d_i = in_i · speed / time` (incl. proliferator speed/extra settings).
- Belt bound: `N ≤ belt_speed · stack / (redundancy · Σ_i d_i)` (mixed belt carries all inputs, and
  often the output/return lane).
- Sorter bound per machine: `d_i ≤ k_i · sorter_rate(tier, distance) · stack_i` for each input and the
  output, with k_i sorters on that item (k_i = 1 in the simple case; sorter_rate in cargo/s).
- Proliferator: if sprayed on the mixed belt, sprayer capacity and proliferator consumption
  `Σ d_i / sprays` per machine.
The standard "matrix" blueprint ratios (e.g. per-belt counts of labs/assemblers) are just this formula
applied per tier; no single authoritative source was opened **[unverified]**.

### 2.2 Factorio 2.0 / Space Age

#### 2.2.1 Modules — https://wiki.factorio.com/Module [opened]
Normal / Uncommon / Rare / Epic / Legendary values:

| Module | Primary effect | Side effects |
|---|---|---|
| Speed 1/2/3 | +20/26/32/38/50 %; +30/39/48/57/75 %; +50/65/80/95/125 % | energy +50/60/70 %, quality −1/−1.5/−2.5 % |
| Productivity 1/2/3 | +4/5/6/7/10 %; +6/7/9/11/15 %; +10/13/16/19/25 % | energy +40/60/80 %, speed −5/−10/−15 %, pollution +5/7/10 % |
| Efficiency 1/2/3 | energy −30/39/48/57/75 %; −40…−100 %; −50…−125 % | – |
| Quality 1/2/3 | +1/1.3/1.6/1.9/2.5 %; +2/2.6/3.2/3.8/5 %; +2.5/3.2/4/4.7/**6.2** % | speed −5 % |

(Legendary Q3 is **6.2 %**: the Quality page says quality-module effects round *down* to 0.1 % and all
other modules to 1 %, and FactorioLab `spa` data stores `0.062` [code]. The Module page's 6.25 % is the
unrounded value. The evaluator must apply this floor rounding, e.g. uncommon Prod 2 = ⌊6·1.3⌋ = 7 %.)
- Effects **add** across modules, beacons, machine base effect and research.
- Clamps: speed, energy, pollution ≥ −80 % (i.e. ≥ 20 % of base) (Module page [opened]); quality ≥ 0 and
  any lower bound on productivity are not stated there **[unverified]**, check in game; productivity ≤
  `maximum_productivity` of the recipe, **default 3.0 (= +300 %)** —
  https://lua-api.factorio.com/latest/prototypes/RecipePrototype.html [opened]. Also per recipe:
  `allow_productivity` (default false), `allow_quality` (default true), `allowed_module_categories`,
  `surface_conditions` (planet restrictions), per-product `ignored_by_productivity` (catalysts).
- Quality of a module scales its positive effect by +30 % per quality level, then rounds down (0.1 % for
  quality modules, 1 % otherwise) (Quality page [opened]). Simplest: take per-quality values from data
  (FactorioLab `qualityRecord`) instead of computing them.

#### 2.2.2 Beacons — https://wiki.factorio.com/Beacon [opened]
- Distribution efficiency by quality: 1.5 / 1.7 / 1.9 / 2.1 / 2.5; 2 module slots; speed/efficiency only
  (no productivity or quality modules); 9×9 area; 480/400/320/240/80 kW.
- **Diminishing rule (2.0.7+)**: each beacon transmits `efficiency / √n`, n = number of beacons
  affecting that machine. With n identical beacons the machine gets `efficiency · √n · Σ module effects`
  (1 → 150 %, 4 → 300 %, 9 → 450 % for one module-worth). Data stores this as a `profile` array
  (FactorioLab `BeaconJson.profile`) [code]: 1/√n truncated to 4 decimals, defined up to n = 100 (0.1),
  present in both `2.0` and `spa` data — use the table, not exact √n. Beacons draw power even when idle; count beacon power per
  machine as `n_beacons · usage / sharing_factor` (sharing is a user setting).

#### 2.2.3 Quality — https://wiki.factorio.com/Quality, https://wiki.factorio.com/Tutorial:Quality_upcycling_math [opened]
- Tiers normal(0) / uncommon(1) / rare(2) / epic(3) / legendary(level 5, one tier above epic). Machine crafting speed scales
  +30 %/level (e.g. AM3 1.25 → 3.125).
- With total quality chance q, output tier distribution from tier t: stay with 1−q, +1 with 0.9q, +2 0.09q,
  +3 0.009q, +4 0.001q, truncated at legendary (the overflow lands on legendary).
- **Recycler** — https://wiki.factorio.com/Recycler [opened]: returns 25 % of ingredients (fluids lost),
  time = 1/16 of the recipe, speed 0.5, 4 slots, no productivity; items with no reversible recipe return
  themselves 25 %.
- **LP formulation (recommended)**: expand each item into `item@quality` and each recipe into
  `recipe@input_quality` whose outputs are split by the matrix above; recyclers become recipes too. The
  upcycling loop is then just a cycle the LP solves; no Markov closed form needed. (The wiki derives the
  closed form `legendary = rᵀ(I − T)⁻¹ n₀`, useful as a test oracle.)

#### 2.2.4 Research productivity
Space Age infinite techs: asteroid, LDS, plastic, processing unit, research, rocket fuel, rocket part,
scrap recycling, steel plate productivity; mining productivity — https://wiki.factorio.com/Research
[opened, names only]. Implement as per-recipe additive productivity, capped by `maximum_productivity`.
Per-level value +10 %/level confirmed for steel plate, scrap recycling and research productivity (wiki
search results); the others assumed the same **[unverified]**. Research productivity acts on labs.

#### 2.2.5 Belts, inserters, rockets, fluids
- Belts 15 / 30 / 45 / 60 items/s (half per lane) — https://wiki.factorio.com/Transport_belts/Physics [opened].
  Space Age (2.0.7+) belt stacking via the stack inserter: its research sets max belt stack 2, two
  further researches raise it to 4 (2 → 3 → 4) — https://wiki.factorio.com/Stack_inserter [opened]. Stack inserter ≈ +33 % over bulk; hand size up to 16.
- Rocket silo — https://wiki.factorio.com/Rocket_silo [opened]: 100 parts/launch (base) or 50 (Space Age);
  base part = 10 LDS + 10 rocket fuel + 10 processing units, SA part = 1 + 1 + 1; minimum launch
  interval 27.20 s base / 26.95 s SA (normal silo; 18.67 / 18.29 s legendary); weight-limited
  cargo (per-item rocket capacity, FactorioLab `rocketCapacity`); 4 module slots. Model the launch as
  a recipe with a fixed extra time (FactorioLab `silo` field).
- Fluids: treat as items with their own units; temperature variants as separate items (Factory Solver
  does this). FactorioLab prices fluids (items without `stack`) at 1/10 of an item in costs when the
  dataset has the `fluidCostRatio` flag (Factorio sets) [code].
- Space platforms / asteroids / spoilage / planets: asteroid crushing = recipes; spoilage = recipe with
  time = spoil time (Factory Solver virtual recipes); planet = `locations` filter on recipes/machines
  (FactorioLab `locations`) [code]. Details of thrust/platform speed **not researched**.

### 2.3 Satisfactory — what the current planner lacks
Current core (PROGRESS.md): integer buildings, shards, Somersloops, clock power c^1.321928, cost weights,
duals. Not yet: generators as recipes, per-recipe clocks.

- **Production amplifier** — https://satisfactory.wiki.gg/wiki/Production_Amplifier [opened] (already
  implemented; for checking): output × (1 + filled/slots), slots 1/2/4 by building, power × (1 + filled/slots)²
  × clock^1.321928 (full + 250 % ≈ 13.43×).
- **Generators**: clock scaling is **linear** for generators — https://satisfactory.wiki.gg/wiki/Clock_speed
  [opened]. Model each generator × fuel as a recipe `fuel(+water) → MW` with negative power; let MW be an
  item with a balance row so the LP chooses fuel.
- **Nuclear** — https://satisfactory.wiki.gg/wiki/Nuclear_Power_Plant [opened]: 2500 MW, water 240 m³/min;
  uranium rod 300 s → 50 waste (10/min); plutonium rod 600 s → 10 waste (1/min); ficsonium 60 s, no waste.
  Overclock linear. Waste must be a byproduct with sink cost = 0 *only if* a consuming chain exists
  (it is not sinkable), so model "store waste" as an explicit expensive disposal option or force full
  reprocessing.
- **Alien Power Augmenter** — https://satisfactory.wiki.gg/wiki/Alien_Power_Augmenter [opened]: each APA
  adds a flat 500 MW, and a grid multiplier of +10 % (unfuelled) or +30 % (fuelled with 5 Alien Power
  Matrix/min each), **additive across APAs**, and the boost also applies to the APAs' own 500 MW:
  `total = (gen + 500·n) × (1 + 0.1·n_u + 0.3·n_f)`, n = n_u + n_f (wiki example: 5000 MW + 2 unfuelled
  → 6000 × 1.2 = 7200 MW). Costs 10
  Somersloops each. Bilinear (gen × count) → handle as an integer parameter / enumerate n (few), not an LP
  variable.
- **Belts / pipes**: belts 60/120/270/480/780/1200 per min — https://satisfactory.wiki.gg/wiki/Conveyor_Belts
  [opened]; pipes Mk.1/Mk.2 300/600 m³/min — https://satisfactory.wiki.gg/wiki/Pipelines [opened]. Useful as per-edge capacity constraints in the
  graph view and as an optional "max belts per item" constraint.
- **Packager loops**: packaging recipes are ordinary cycles; FactorioLab flags them `noCostMultiplier`
  [code] (unaffected by recipe-cost multipliers). Nothing special for the LP beyond allowing cycles and
  a surplus/byproduct cost.
- **AWESOME Sink** — https://satisfactory.wiki.gg/wiki/AWESOME_Sink [opened]: per-item point values are
  a data table (not a formula); coupon cost `250·(⌈n/3⌉−1)² + 1000`. Add a "maximize points" objective
  (sink recipe: item → points) and a per-item points field.

---

## 3. Machine-readable data sources

| Game | Source | Format | Licence / notes |
|---|---|---|---|
| Factorio | `factorio --dump-data` → `script-output/data-raw-dump.json` (+ `--dump-icon-sprites`, `--dump-prototype-locale`) — https://wiki.factorio.com/Command_line_parameters [opened] | Full `data.raw` JSON after mods | Game data © Wube; generate locally, ship only derived numbers |
| Factorio | FactorioLab `public/data/spa/data.json`, `2.0/data.json` + many mod sets [code]; exported by the *FactorioLab Export* mod | Compact calculator JSON (items/recipes/machines/modules/beacons/belts/quality) | Repo MIT; underlying game data © Wube |
| Factorio | Kirk McDonald `factoriodump` (factorio-tools) [opened] | JSON + sprite sheet | Apache-2.0 |
| DSP | dsp-calc `data/` (`game_data`, 0.10.31.24710, mod sets incl. Genesis Book, Dark Fog) [opened] | JSON | MulanPSL-2.0 (permissive, attribution) |
| DSP | FactorioLab `public/data/dsp/data.json` (+ `i18n/`) [code] — game 0.10.29.21950, older than dsp-calc | FactorioLab schema | MIT (JSON); icons are game art |
| Satisfactory | `Docs.json` / `en-US.json` shipped in `CommunityResources/Docs` of the game (what we already use via SatisfactoryTools `data1.0.json`; see `data/SOURCE.md`) | UE class dump JSON (UTF-16) | © Coffee Stain |
| Satisfactory | FactorioLab `public/data/sfy/data.json` [code] — labelled Satisfactory **1.2** (ours is 1.0 data; diff before mixing); `satisfactory-docs-parser` (used by YAFP) | JSON | MIT / see repos |
| Satisfactory | SatisfactoryTools `data/data1.0.json` (what we use) | JSON | MIT; its item images are excluded from reuse |

---

## 4. Recommendation

### 4.1 Game-agnostic model (one LP/MILP core)

Adopt FactorioLab's schema shape (proven for 14 games) with our integer extensions:

```
Item       { id, name, kind: solid|fluid|virtual(power, points, research), stack, tags,
             quality_levels?: [q] }                      // Factorio expands item@q
Recipe     { id, time, in{item:amt}, out{item:amt}, catalyst{item:amt}, producers[machine],
             flags[mining|burn|recycling|launch|sink|throughput_bound],
             max_productivity=∞, allowed_effects[], locations[], cost? }
Machine    { id, speed | speed_from_belt, power_kW, drain_kW, power_curve: linear|pow(e),
             fuel_categories[], module_slots, base_effect{}, size, quality_record{} ,
             boost_slots?: {kind:"somersloop", n, out_mult, power_mult} }
Effect     { id, kind: module|proliferator|shard|beacon, speed, productivity, consumption,
             pollution, quality, sprays?, limitation(recipe set), max_per_machine }
Beacon     { effectivity_by_quality[], slots, range, usage, profile[n] }
Logistics  { belts[{speed, stack_max, lanes}], inserters/sorters[{rate(dist), stack}], pipes[] }
GridEffect { e.g. APA: flat_MW, multiplier, cost }      // applied after the solve or by enumeration
```

Solver mapping (extends `lp.rs`):
- Variables per (recipe, machine, effect-config): work x, buildings n (int), plus game-specific integers
  (shards, sloops, beacons). Effects are **pre-combined configurations**: for each recipe the data layer
  enumerates a small set of configs (e.g. "4×Prod3 + 8 beacons×2 Speed3", "Mk.III extra", "200 % + 2 sloops")
  and computes `speed, out multiplier, power` with the game's clamp rules. The LP picks among configs.
  This keeps the core linear and game-agnostic; the per-game code is only the *config evaluator*.
- Satisfactory clock remains the special case already handled (perspective of c^1.32).
- Power is an item (`virtual:power`): consumers use it, generators produce it → generator choice inside LP.
- Proliferator / module *items* are consumed per craft (DSP sprays, Factorio module building cost optional).
- Quality: item@q / recipe@q expansion in the data layer; no solver change.
- Throughput-bound recipes (DSP fractionator): machine speed = chosen belt speed.
- Bounds/sources/sinks à la Factory Solver; keep our duals → item values in every game.

### 4.2 Per-game extension points
- `effect_evaluator(game, recipe, machine, config) → {speed, prod, power, extra_inputs}`
  (Factorio: additive effects + clamps + beacon `profile` + per-quality module values (floor-rounded); DSP: points/min rule + recipe
  limitation; Satisfactory: clock + shards + sloops).
- `config_enumerator(game, recipe, settings)` (which module/beacon/proliferator combos to offer).
- `logistics_check(game, plan)` (Satisfactory belts/pipes; DSP 混带 + sorter capacity; Factorio lanes,
  stacking, inserters) — post-solve report, optionally a constraint.
- `data_importer(game)` (Docs.json; data-raw-dump / FactorioLab JSON; dsp-calc / FactorioLab JSON).

### 4.3 Build order
1. **Data layer refactor**: game-neutral structs + importer for FactorioLab `data.json` (gives Factorio
   2.0.77 and DSP data immediately, MIT; DSP set is 0.10.29 — cross-check against dsp-calc 0.10.31; do
   not ship `icons.webp`) while keeping our Docs.json Satisfactory importer.
2. **Config-enumeration effects** in the core (replaces Satisfactory-only shard/sloop code paths with
   the general mechanism; keep the clock perspective special case).
3. **Power as an item + generators as recipes** (closes the Satisfactory gap, needed by all three).
4. **DSP proliferator** (points, extra/speed choice, spray consumption, speed-only limitation) and
   fractionator/photon special recipes; cross-check against dsp-calc's 28 Vitest scenarios (these are
   regression fixtures of dsp-calc's own earlier output, not in-game measurements; MulanPSL-2.0 →
   keep attribution if copied).
5. **Factorio modules + beacons + research productivity** with clamps; then **quality expansion and
   recyclers** (test with the wiki upcycling closed form).
6. Logistics reports: Satisfactory belts/pipes, DSP 混带 machines-per-belt, Factorio belts/inserters.
7. Extras: AWESOME points objective, APA enumeration, nuclear waste handling, rockets/space platforms.
