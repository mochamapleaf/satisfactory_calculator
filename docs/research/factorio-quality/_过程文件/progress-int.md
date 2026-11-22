# Step 15: whole-building quality plans (shared machines across tiers)

Design: impl-progress.md step 8 + report §2.1 / §5 step 6 (one integer N per base recipe × building × loadout,
Σ_tiers x/speed ≤ N).

## 1. Unit owns its recipe (Arc) — done
- `lp.rs`: `Unit.recipe: Arc<Recipe>`, lifetimes dropped from Unit / Inputs / SubRun, so quality plans
  (locally expanded recipes) can return `Start::Running` (stream, Stop, Keep solving, HiGHS) instead of
  solving to the end inside `start`.
- Next: shared groups in build_scaled / Point / rounded / values_of / offer_values / finish.

## 2. Shared-machine model — done
- `lp.rs`: `Inputs.shared` (= whole buildings && quality); `group_key` = (base recipe, mode key).
  Per column n_c continuous at cost 0 with x_c ≤ speed·n_c; per group one N_g (integer, building + module
  + beacon-MW cost) with Σ n_c ≤ N_g. Point carries `groups`; `rounded` = ⌈Σ shares⌉ per group;
  `values_of` / `int_hint` round only integer columns; reweight LPs scale per group; `offer_values`
  (HiGHS) rounds only integer columns; `detier` and `drop_dominated` off for shared (per-column moves are
  not exact there). Fractional-only warning removed. `PlanRecipe.group_machines` (mod.rs).
- Tests (quality_tests.rs): `integer_shared_widget_loop_by_hand` (4Q loop, 2/min: normal 6.0067 +
  uncommon…epic 0.8477 → 7 AM3, parts = LP 309.4476, 26 vs 31 per column), `integer_shared_known_optimum`
  (= scipy milp `int_known_optimum.py` 103.878557596; LP 101.412181), `integer_quality_gear_and_engine_values`
  (2x1 gear@4 10/min: 1032.2782, 50 buildings, gap 2e-5; HiGHS value round trip accepted).
- `cargo test -q --release`: 91 pass / 4 ignored; `cargo test -q`: 90 pass / 5 ignored.

## 3. Machine quality — done
- Converter emits `quality_speed` per building (FactorioLab qualityRecord; data/factorio-{2x1,2q1,spa}.json
  regenerated, only that field added). `Building::speed_at`, `PlanSettings.machine_quality`; used in
  rules.rs tiered modes and quality.rs loadouts. UI: Quality section → Machine quality.
- Test `machine_quality_speed` (AM3 1.25 → 3.125: 75 gears/min 0.5 → 0.2 machines, MW per machine equal;
  4Q loadout 3.125 × 0.8).

## 4. Bench, 4 s, native microlp (`quality_integer_bench`, ignored test)
| scenario | cols | LP | first plan | final | gap | buildings (LP per-group ceil) |
|---|---|---|---|---|---|---|
| gear@4 10/min, normal modules | 230 | 14376.6024 | 14376.6212 @ 8 ms | 14376.6212 proven | 0 % | 520 (520) |
| gear@4 10/min, legendary modules | 230 | 1032.2565 | 1032.2782 @ 11 ms | 1032.2782 proven | 0.002 % | 50 (50) |
| quality module 1@4 1/min | 2208 | 104.3236 | 104.4993 @ 383 ms | 104.4931 proven 2.8 s | 0.010 % | 37 (38) |
| electronic circuit@4 30/min | 472 | 402.5331 | 402.5694 @ 33 ms | proven | 0.009 % | 38 (38) |
| processing unit@4 1/min | 2218 | 115.0153 | 115.2042 @ 419 ms | 115.1918 proven 1.4 s | 0.009 % | 30 (32) |
| plastic@4 60/min, all planets (LDS / Gleba) | 3017 | 0.0168 | 0.09 @ 440 ms | 0.0800 not proven | 79 % | 8 (8) |
| LDS shuffle oracle chain 60/min | 47 | 60.5595 | 60.56 @ 9 ms | proven | 0.001 % | 56 (56) |
"proven" = within MIP gap 1e-4. The all-planets plastic plan has (almost) no raw cost (Gleba legendary seeds
loop), so the objective is buildings × 0.01 and the LP bound is weak: 79 % relative = 8 vs ≈1.7 fractional
buildings. In the browser HiGHS gets it to 7 buildings within 0–19 % after Keep solving; microlp 7 at 76 %.

## 5. UI + live browser test — done
- Results.tsx: quality plans show one row per shared group (base recipe × machine × loadout) with a
  "n tiers" tag, whole machines, per-tier split (normal 8.91 / uncommon 1.78 / …), "x % busy", summed in/out;
  × excludes those tiers. Single-tier rows show the group's whole machines and busy %. solver.ts: no more
  "fractional quality" exception (Auto races both engines). Sidebar: hint text, Machine quality select.
- Live (Playwright Chromium on vite preview :4173, build of this tree; `live_int.py`, shots + texts +
  `live-log*.txt` in shots-int/): legendary gear 10/min × microlp / HiGHS / Auto (50 buildings, optimal,
  no fractional warning, 2 shared rows); legendary quality module 1/min × 3 engines (37 buildings, optimal /
  ≈ optimal); LP view; legendary machine quality (21 buildings); Stop at 1.6 s keeps the plan
  ("Best plan found (stopped)", 8 buildings) and Keep solving continues (→ 7) with microlp and HiGHS.
- Hand check (gear4-highs.png): AM3 2Q2P legendary modules: speed 1.25 × (1 − 0.1 − 0.3) = 0.75 → 90 crafts
  = 180 plates/min per machine. Plates in 1603.8 / 320.59 / 97.1 / 29.35 → 8.91 / 1.78 / 0.54 / 0.16
  (= table), Σ 11.39 → 12 machines (per column 9+2+1+1 = 13), 94.9 % busy. Normal gears out
  1603.8/2 × 1.5 × 0.875 = 1052.49 (= table). AM3 12 + 1 (legendary 4P) = 13, furnaces ⌈34.39⌉ = 35,
  recyclers 2 → 50.
- Not tested live: phone width, dark mode, Item flow / Machine graph tabs on these plans.

## 6. Not done
- Quality mining: the report (§5 step 3) defers it to a second phase (mining as a recipe over a virtual
  deposit item); ores stay normal. Still open.
- `cargo test -q --release`: 92 pass / 5 ignored; `cargo test -q`: 91 pass / 6 ignored.
