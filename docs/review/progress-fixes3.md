# Fixes for review 3 (docs/review/fable-review-3.md), non-geo items
- 2026-11-21 start. Scope: H3, H4, M1, M2, L1-L7 (geo H1/H2/M3-M5/L8/L9 by another agent). Base commit b6c29d5.
- H3 + M1 + L2 done: rules::research_bonus (only recipes named by this set's research, clamp 0..3, NaN -> 0), applied
  whatever productivity_allowed says (tiered + quality loadouts). Tests: scrap_recycling_research (spa + 2x1, Fulgora,
  60 holmium: 6000 scrap / 40 recyclers -> 4000 / 26.67 at +50 %), research_productivity_families (+ -2, NaN, 2q1 leftover).
  Frontend: research hint corrected; withDataset drops research levels for sets without research (DATASETS.research).
- L6 done: level input clamped to 0..300, level shown = max over the family's recipes.
- L1 done: urlState.sanitize() drops fields of the wrong type (arrays/records/numbers/strings) before defaults; tsc ok.
- H4 done (converter + Rust): Building.fuel {item, rate/min, result}; rules::fuel_io adds fuel per unit of work
  (rate x consumption / speed) as extra inputs and spent cells as Mode.extra_outputs (tiered, proliferator, quality
  loadouts). Converter: electric power only (2.0 boiler 1.8 MW and 2.0/spa heat exchanger 10 MW no longer electric),
  burner fuel by category, heat exchanger burns heat (MJ, 600/min), reactor / heating tower recipes (synthesised for
  2.0/spa; spa tower = 2.1 values), input-less recipes of fuelled machines stay recipes (biter egg), spent cells make
  depleted cells non-raw, generators dropped. Quality closure follows non-raw fuels only (coal synthesis would add
  700 columns). Data regenerated; dsp.json byte-identical; semantic diff only the intended fields.
  Test heat_and_fuel: 2x1/2q1/spa one exchanger = 600 MJ/min = 0.25 reactor = 0.075 cells/min (+0.075 depleted),
  heating tower 0.25 = 60 coal/min, 2.0 boiler 6000 steam = 45 coal, stone furnace 60 plates = 4.32 coal, steel
  2.16, biochamber 15 nutrients/min -> 3 with 4 eff. 3.
- M2 done: Building.locations (crusher platform, spawner Nauvis) filter producers; recipe locations intersect
  machine locations in the converter; infeasible hint = blocking_items() (forward-obtainable fixed point, backward
  search through unobtainable items) with planets from resources, recipes and machines. Test
  planet_filter_machines_and_hint. quality_paths test: asteroid reprocessing now needs space-platform.
- L3 done: Mode.beacon_power kept out of the clock factor (LP row, hint, report). L5 done: tiered modes back in
  b34e62c order (plain, then effects in order; Fill struct).
