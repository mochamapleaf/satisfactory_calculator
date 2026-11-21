use super::*;
use std::collections::HashMap;

fn data() -> GameData {
    GameData::default_data()
}

fn settings(targets: &[(&str, f64)]) -> PlanSettings {
    PlanSettings {
        targets: targets.iter().map(|(i, r)| ItemRate { item: i.to_string(), rate: *r }).collect(),
        ..Default::default()
    }
}

fn machines(p: &Plan) -> HashMap<String, f64> {
    p.recipes.iter().map(|r| (r.name.clone(), r.machines)).collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6 * (1.0 + b.abs())
}

/// Net item balance must equal targets + surplus for every item.
fn assert_balanced(p: &Plan) {
    let mut net: HashMap<String, f64> = HashMap::new();
    for r in &p.recipes {
        for o in &r.outputs {
            *net.entry(o.item.clone()).or_default() += o.rate;
        }
        for i in &r.inputs {
            *net.entry(i.item.clone()).or_default() -= i.rate;
        }
    }
    for l in p.raw.iter().chain(&p.supplied) {
        *net.entry(l.item.clone()).or_default() += l.rate;
    }
    for l in p.targets.iter().chain(&p.surplus) {
        *net.entry(l.item.clone()).or_default() -= l.rate;
    }
    for (k, v) in net {
        assert!(v.abs() < 1e-4, "item {k} unbalanced by {v}");
    }
}

#[test]
fn data_loads() {
    let d = data();
    assert!(d.recipes.len() > 250);
    assert!(d.resolve_item("Reinforced Iron Plate").is_some());
    assert!(d.resolve_recipe("Cast Screw").unwrap().alt);
}

/// readme.md example with 1.0 rates: 60 RIP/min.
const RIP_CHAIN: [&str; 5] = [
    "Recipe_IngotIron_C",
    "Recipe_IronPlate_C",
    "Recipe_IronRod_C",
    "Recipe_Screw_C",
    "Recipe_IronPlateReinforced_C",
];

#[test]
fn square_matrix_rip() {
    let d = data();
    let ids: Vec<String> = RIP_CHAIN.iter().map(|s| s.to_string()).collect();
    let sol = matrix::solve_square(&d, &ids, &[("Desc_IronPlateReinforced_C".into(), 60.0)]).unwrap();
    let m: HashMap<_, _> = sol.machines.into_iter().collect();
    assert!(close(m["Recipe_IronPlateReinforced_C"], 12.0));
    assert!(close(m["Recipe_IronPlate_C"], 18.0));
    assert!(close(m["Recipe_Screw_C"], 18.0));
    assert!(close(m["Recipe_IronRod_C"], 12.0));
    assert!(close(m["Recipe_IngotIron_C"], 24.0));
    assert_eq!(sol.inputs.len(), 1);
    assert!(close(sol.inputs[0].1, 720.0)); // same as readme.md
}

#[test]
fn square_matrix_detects_non_square() {
    let d = data();
    // two screw recipes -> linearly dependent (readme "多配方处理")
    let mut ids: Vec<String> = RIP_CHAIN.iter().map(|s| s.to_string()).collect();
    ids.push("Recipe_Alternate_Screw_C".into());
    let r = matrix::solve_square(&d, &ids, &[("Desc_IronPlateReinforced_C".into(), 60.0)]);
    assert!(r.is_err());
}

#[test]
fn lp_matches_square_matrix() {
    let d = data();
    let p = plan(&d, &settings(&[("Reinforced Iron Plate", 60.0)]));
    assert_eq!(p.status, Status::Optimal);
    let m = machines(&p);
    assert!(close(m["Reinforced Iron Plate"], 12.0));
    assert!(close(m["Iron Plate"], 18.0));
    assert!(close(m["Screw"], 18.0));
    assert!(close(m["Iron Rod"], 12.0));
    assert!(close(m["Iron Ingot"], 24.0));
    assert!(close(lp::raw_map(&p)["Desc_OreIron_C"], 720.0));
    // power: 12*15 + (18+18+12)*4 + 24*4 = 180 + 192 + 96
    assert!(close(p.production_power, 468.0));
    assert_eq!(p.buildings, 84.0);
    assert_balanced(&p);
}

#[test]
fn byproduct_plastic() {
    let d = data();
    let p = plan(&d, &settings(&[("Plastic", 60.0)]));
    assert_eq!(p.status, Status::Optimal);
    assert!(close(lp::raw_map(&p)["Desc_LiquidOil_C"], 90.0));
    assert!(close(machines(&p)["Plastic"], 3.0));
    let hor = p.surplus.iter().find(|s| s.item == "Desc_HeavyOilResidue_C").unwrap();
    assert!(close(hor.rate, 30.0));
    assert_balanced(&p);
}

#[test]
fn overclock_with_shards() {
    let d = data();
    let mut s = settings(&[("Reinforced Iron Plate", 60.0)]);
    s.max_clock_percent = 250.0;
    s.objective = Objective::Buildings;
    let p = plan(&d, &s);
    assert!(p.proven_optimal);
    let rip = p.recipes.iter().find(|r| r.name == "Reinforced Iron Plate").unwrap();
    // 12 machine-equivalents at ≤ 250 % → 5 buildings; 12 ≤ 5 + 0.5·shards → 14 shards
    assert_eq!(rip.machines, 5.0);
    assert_eq!(rip.shards, 14.0);
    let expected = 15.0 * (2.0f64.powf(1.321929) + 4.0 * 2.5f64.powf(1.321929));
    assert!((rip.power - expected).abs() < 1e-2, "{} vs {}", rip.power, expected);
    assert!(rip.groups.iter().all(|g| g.clock_percent <= 250.0 + 1e-6));
    assert_balanced(&p);
    // no shards available → same as 100 %
    s.shard_budget = Some(0);
    let p = plan(&d, &s);
    assert_eq!(machines(&p)["Reinforced Iron Plate"], 12.0);
    assert_eq!(p.shards, 0.0);
}

#[test]
fn integer_plan_is_whole_and_no_worse_than_rounding() {
    let d = data();
    for (target, rate) in [("Reinforced Iron Plate", 7.0), ("Modular Frame", 3.3), ("Computer", 1.7)] {
        for alts in [false, true] {
            let mut s = settings(&[(target, rate)]);
            s.objective = Objective::Buildings;
            s.allow_all_alternates = alts;
            let ip = plan(&d, &s);
            assert_eq!(ip.status, Status::Optimal, "{target}");
            assert_balanced(&ip);
            for r in &ip.recipes {
                assert_eq!(r.machines, r.machines.round(), "{} not whole", r.name);
                assert!(r.throughput <= r.machines + 1e-6, "{} over 100 %", r.name);
            }
            s.integer = false;
            let lp = plan(&d, &s);
            let rounded: f64 = lp.recipes.iter().map(|r| r.machines_ceil).sum();
            assert!(ip.buildings <= rounded + 1e-9, "{target}: {} > {}", ip.buildings, rounded);
            assert!(ip.objective >= lp.objective - 1e-6);
        }
    }
}

#[test]
fn item_values_are_shadow_prices() {
    let d = data();
    let p = plan(&d, &settings(&[("Reinforced Iron Plate", 60.0)]));
    let v: HashMap<_, _> = p.item_values.iter().map(|v| (v.item.clone(), v.value)).collect();
    // 12 Iron Ore per plate at weight 1, plus a small per-building tie-break
    let rip = v["Desc_IronPlateReinforced_C"];
    assert!(rip > 12.0 && rip < 12.5, "{rip}");
}

#[test]
fn custom_cost_function() {
    let d = data();
    let mut s = settings(&[("Reinforced Iron Plate", 10.0)]);
    s.allow_all_alternates = true;
    s.objective = Objective::Custom;
    // iron is expensive, copper free → alternates that swap iron for copper win
    s.cost = CostWeights { resources: 1.0, buildings: 0.0, ..Default::default() };
    s.cost.resource_weights.insert("Desc_OreCopper_C".into(), 0.0);
    let cheap_copper = plan(&d, &s);
    s.cost.resource_weights.insert("Desc_OreCopper_C".into(), 100.0);
    let dear_copper = plan(&d, &s);
    let raw = |p: &Plan, it: &str| lp::raw_map(p).get(it).copied().unwrap_or(0.0);
    assert!(raw(&cheap_copper, "Desc_OreIron_C") <= raw(&dear_copper, "Desc_OreIron_C"));
    assert_eq!(raw(&dear_copper, "Desc_OreCopper_C"), 0.0);
    assert_balanced(&cheap_copper);
}

#[test]
fn power_objective_underclocks() {
    let d = data();
    let mut s = settings(&[("Iron Ingot", 45.0)]);
    s.objective = Objective::Custom;
    s.cost = CostWeights { resources: 0.0, buildings: 0.0, power: 1.0, ..Default::default() };
    let p = plan(&d, &s);
    // free buildings: power falls with more, slower smelters (down to the 10 % breakpoint)
    assert!(p.recipes[0].machines > 2.0);
    s.cost.buildings = 100.0;
    let p = plan(&d, &s);
    assert_eq!(p.recipes[0].machines, 2.0);
    assert!((p.recipes[0].groups[0].clock_percent - 75.0).abs() < 1e-6);
}

#[test]
fn somersloop_doubles_output_quadruples_power() {
    let d = data();
    let mut s = settings(&[("Iron Ingot", 60.0)]);
    s.somersloops.insert("Recipe_IngotIron_C".into(), 1);
    let p = plan(&d, &s);
    let r = &p.recipes[0];
    assert!(close(r.machines, 1.0)); // 30/min base -> 60/min amplified
    assert!(close(r.power, 16.0));
    assert!(close(lp::raw_map(&p)["Desc_OreIron_C"], 30.0));
    assert_balanced(&p);
}

#[test]
fn resource_limit_infeasible() {
    let d = data();
    let mut s = settings(&[("Reinforced Iron Plate", 60.0)]);
    s.resource_limits.insert("Desc_OreIron_C".into(), Some(100.0));
    assert_eq!(plan(&d, &s).status, Status::Infeasible);
    s.allow_ore_conversion = true;
    // 1.0 Converter recipes (Iron Ore (Limestone) + SAM) make this feasible ...
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert!(p.recipes.iter().any(|r| r.building == "Converter"));
    assert_balanced(&p);
    // ... unless SAM is unavailable too
    s.resource_limits.insert("Desc_SAM_C".into(), Some(0.0));
    assert_eq!(plan(&d, &s).status, Status::Infeasible);
    s.ignore_limits = true;
    assert_eq!(plan(&d, &s).status, Status::Optimal);
}

#[test]
fn supplies_reduce_raw() {
    let d = data();
    let mut s = settings(&[("Reinforced Iron Plate", 60.0)]);
    s.supplies.push(ItemRate { item: "Iron Plate".into(), rate: 360.0 });
    let p = plan(&d, &s);
    assert!(close(lp::raw_map(&p)["Desc_OreIron_C"], 180.0));
    assert_balanced(&p);
}

#[test]
fn alternates_never_worse() {
    let d = data();
    let base = plan(&d, &settings(&[("Reinforced Iron Plate", 60.0)]));
    let mut s = settings(&[("Reinforced Iron Plate", 60.0)]);
    s.allow_all_alternates = true;
    let alt = plan(&d, &s);
    assert_eq!(alt.status, Status::Optimal);
    assert!(alt.objective <= base.objective + 1e-9);
    assert_balanced(&alt);
}

#[test]
fn objectives_on_complex_chains() {
    let d = data();
    for target in ["Heavy Modular Frame", "Turbo Motor", "Aluminum Ingot", "Fused Modular Frame", "Nuclear Pasta"] {
        for obj in [Objective::Resources, Objective::Buildings, Objective::Power] {
            for alts in [false, true] {
                let mut s = settings(&[(target, 10.0)]);
                s.objective = obj;
                s.allow_all_alternates = alts;
                let mut p = plan(&d, &s);
                // debug builds under a parallel test run are slow: a timeout without any
                // whole-building plan is a timing artefact here, so retry with more time
                if p.status == Status::Infeasible && p.warnings.iter().any(|w| w.starts_with("Time limit")) {
                    s.time_limit_ms = 20_000;
                    p = plan(&d, &s);
                }
                assert_eq!(p.status, Status::Optimal, "{target} {obj:?} alts={alts} {:?}", p.warnings);
                assert_balanced(&p);
                assert!(!p.flows.is_empty());
            }
        }
    }
}

#[test]
fn flows_conserve_target() {
    let d = data();
    let p = plan(&d, &settings(&[("Reinforced Iron Plate", 60.0)]));
    let into_target: f64 = p.flows.iter().filter(|f| f.to == "out:Desc_IronPlateReinforced_C").map(|f| f.rate).sum();
    assert!(close(into_target, 60.0));
    let screws: f64 = p.flows.iter().filter(|f| f.item == "Desc_IronScrew_C").map(|f| f.rate).sum();
    assert!(close(screws, 720.0));
}

#[test]
fn auto_somersloops_respect_budget() {
    let d = data();
    let mut s = settings(&[("Reinforced Iron Plate", 30.0)]);
    s.auto_somersloops = true;
    s.somersloop_budget = Some(4);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert!(p.somersloops <= 4.0 + 1e-9 && p.somersloops > 0.0, "{}", p.somersloops);
    assert_balanced(&p);
    // free Somersloops cut raw use below the plain plan
    let plain = plan(&d, &settings(&[("Reinforced Iron Plate", 30.0)]));
    assert!(lp::raw_map(&p)["Desc_OreIron_C"] < lp::raw_map(&plain)["Desc_OreIron_C"]);
    s.somersloop_budget = Some(0);
    assert_eq!(plan(&d, &s).somersloops, 0.0);
}

fn game(g: &str, targets: &[(&str, f64)]) -> (GameData, PlanSettings) {
    let mut s = settings(targets);
    s.game = g.into();
    (GameData::for_game(g), s)
}

#[test]
fn dsp_machine_tiers() {
    let (d, mut s) = game("dsp", &[("iron-ingot", 60.0)]);
    s.objective = Objective::Buildings;
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    // Plane Smelter (speed 2) does 120/min: one building
    assert_eq!(p.buildings, 1.0);
    assert_ne!(p.recipes[0].building, "电弧熔炉");
    assert!(close(lp::raw_map(&p)["iron-ore"], 60.0));
    // only Arc Smelters allowed: 1 ingot/s each → 1 building as well; 61/min needs 2
    s.disabled_buildings = vec!["plane-smelter".into(), "df-negentropy-smelter".into()];
    s.targets[0].rate = 61.0;
    assert_eq!(plan(&d, &s).buildings, 2.0);
}

#[test]
fn dsp_proliferator_extra_products() {
    let (d, mut s) = game("dsp", &[("circuit-board", 120.0)]);
    let plain = plan(&d, &s);
    assert_eq!(plain.status, Status::Optimal);
    assert_balanced(&plain);
    // free Mk.III proliferator: extra products (+25 %) cut raw ore on the board step
    s.effects = vec!["proliferator-3-products".into()];
    s.supplies.push(ItemRate { item: "proliferator-3".into(), rate: 1000.0 });
    let pro = plan(&d, &s);
    assert_balanced(&pro);
    let ore = |p: &Plan| lp::raw_map(p).values().sum::<f64>();
    assert!(ore(&pro) < ore(&plain) - 1.0, "{} vs {}", ore(&pro), ore(&plain));
    assert!(pro.supplied.iter().any(|l| l.item == "proliferator-3"));
}

#[test]
fn factorio_circuits_and_modules() {
    let (d, mut s) = game("factorio", &[("electronic-circuit", 60.0)]);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert_balanced(&p);
    let raw = lp::raw_map(&p);
    // 1 iron plate + 3 cables (1.5 copper plate) per circuit
    assert!(close(raw["iron-ore"] + raw["copper-ore"], 150.0), "{raw:?}");
    s.effects = vec!["productivity-module-3".into()];
    s.supplies.push(ItemRate { item: "productivity-module-3".into(), rate: 0.0 });
    let pm = plan(&d, &s);
    assert_balanced(&pm);
    let r2 = lp::raw_map(&pm);
    assert!(r2["iron-ore"] + r2["copper-ore"] < 150.0 - 1.0, "{r2:?}");
}

#[test]
fn every_game_solves_its_items() {
    for (g, ds) in [("dsp", ""), ("factorio", ""), ("factorio", "2x1"), ("factorio", "2q1"), ("factorio", "spa")] {
        let d = GameData::for_set(g, ds);
        let produced: std::collections::BTreeSet<_> =
            d.recipes.iter().flat_map(|r| r.outputs.iter().map(|o| o.0.clone())).collect();
        let mut bad = vec![];
        for it in &produced {
            let mut s = settings(&[(it.as_str(), 10.0)]);
            s.game = g.into();
            s.dataset = ds.into();
            s.integer = false;
            s.planets = d.locations.iter().map(|l| l.id.clone()).collect();
            let p = plan(&d, &s);
            if p.status != Status::Optimal {
                bad.push(it.clone());
            } else {
                assert_balanced(&p);
            }
        }
        assert!(bad.is_empty(), "{g} {ds}: unsolvable {bad:?}");
    }
}

fn dsp_compute(targets: &[(&str, f64)]) -> (GameData, PlanSettings) {
    let (d, mut s) = game("dsp", targets);
    s.objective = Objective::Compute;
    s.disabled_buildings = vec!["df-negentropy-smelter".into(), "df-recomposing-assembler".into(), "df-self-evolution-lab".into()];
    (d, s)
}

#[test]
fn compute_model_loads_report_coefficients() {
    let m = compute::model();
    assert_eq!(m.buildings["assembling-machine-3"], 1.0);
    assert_eq!(m.buildings["matrix-lab"], 1.5);
    assert!(close(m.sorter, 0.12) && close(m.per_machine, 0.13) && close(m.coater, 0.02));
    assert!(close(m.cost_per_mw("artificial_star"), 0.0056) && close(m.cost_per_mw("wind"), 0.5));
    assert_eq!(m.us_per_unit, (0.5, 0.2, 1.0));
    assert_eq!(m.sorter_capacity, 7200.0);
}

#[test]
fn compute_objective_hand_computed() {
    // 120 iron ingots/min: one Plane Smelter (speed 2, 1.44 MW), 1 in + 1 out sorter.
    let (d, s) = dsp_compute(&[("iron-ingot", 120.0)]);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert_eq!(p.buildings, 1.0);
    let c = p.compute.as_ref().unwrap();
    let expect = 1.0 + 0.12 * 2.0 + 0.13 + 0.0056 * 1.44;
    assert!(close(c.total, expect), "{} vs {expect}", c.total);
    assert!(close(p.recipes[0].compute, expect));
    // objective = compute + raw tie-break (0.001 × weight 1 × 120 ore)
    assert!(close(p.objective, expect + 0.001 * 120.0), "{}", p.objective);
    assert!(close(c.us_per_tick, 0.5 * expect) && close(c.us_low, 0.2 * expect));
    // mixed belts: still one input port here; wind power: generation 0.5/MW
    let mut w = s.clone();
    w.generator = "wind".into();
    let pw = plan(&d, &w);
    assert!(close(pw.compute.unwrap().total, 1.0 + 0.24 + 0.13 + 0.5 * 1.44));
}

#[test]
fn compute_prefers_top_tier_and_counts_sorters() {
    // 600 ingots/min: 5 Plane Smelters (not 10 Arc Smelters), even with wind power
    for g in ["artificial_star", "wind"] {
        let (d, mut s) = dsp_compute(&[("iron-ingot", 600.0)]);
        s.generator = g.into();
        let p = plan(&d, &s);
        assert_eq!(p.buildings, 5.0, "{g}");
        assert!(p.recipes.iter().all(|r| r.building != "电弧熔炉"), "{g}");
    }
    // mixed belts merge the inputs into one sorter: a 3-input recipe gets cheaper by 2 × 0.12
    let (d, mut s) = dsp_compute(&[("processor", 60.0)]);
    s.integer = false;
    let sep = plan(&d, &s).compute.unwrap();
    s.mixed_belts = true;
    let mix = plan(&d, &s).compute.unwrap();
    assert!(mix.sorters < sep.sorters - 1e-6 && close(mix.machines, sep.machines));
}

/// Review 2 M4: per-entity terms count whole buildings also in a fractional (LP) plan.
#[test]
fn compute_counts_whole_entities_in_lp() {
    // 60 ingots/min = 0.5 Plane Smelter: one smelter, two sorters, one belt+station share
    let (d, mut s) = dsp_compute(&[("iron-ingot", 60.0)]);
    s.integer = false;
    let p = plan(&d, &s);
    let r = &p.recipes[0];
    assert!(close(r.machines, 0.5) && r.machines_ceil == 1.0);
    let c = p.compute.as_ref().unwrap();
    assert!(close(c.machines, 1.0) && close(c.sorters, 0.24) && close(c.logistics, 0.13), "{c:?}");
    assert!(close(c.total, 1.0 + 0.24 + 0.13 + 0.0056 * r.power), "{c:?}");
    assert_eq!(p.buildings, 1.0);
}

/// Review 2 L1: fractionators have no sorters; stacked labs pay a share of one set.
#[test]
fn compute_sorter_share() {
    let d = GameData::for_game("dsp");
    let s = PlanSettings { game: "dsp".into(), ..Default::default() };
    let m = compute::model();
    let k = |rid: &str| {
        let r = d.recipe(rid).unwrap();
        let f = rules::modes(&d, r, &s).into_iter().next().unwrap();
        compute::sorters_per_machine(r, &f, false, m.sorter_capacity)
    };
    assert_eq!(k("deuterium-fractionation"), 0.0);
    // universe matrix: 6 inputs + 1 output, one set per stack of 10
    assert!(close(k("universe-matrix"), 0.7), "{}", k("universe-matrix"));
    assert_eq!(k("iron-ingot"), 2.0);
}

fn sprayed_modes(p: &Plan) -> HashMap<String, String> {
    p.recipes.iter().map(|r| (r.id.clone(), r.mode.clone())).collect()
}

#[test]
fn compute_picks_proliferator_mode_per_recipe() {
    // Report §3.2/§4.3: top recipes (universe matrix, processors…) use extra products,
    // bottom smelting uses speedup; the proliferator line itself is in the plan.
    let (d, mut s) = dsp_compute(&[("universe-matrix", 60.0)]);
    s.integer = false;
    s.effects = vec!["proliferator-3-speed".into(), "proliferator-3-products".into()];
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert_balanced(&p);
    let modes = sprayed_modes(&p);
    assert!(modes["universe-matrix"].ends_with("proliferator-3-products"), "{modes:?}");
    assert!(modes["iron-ingot"].ends_with("proliferator-3-speed"), "{modes:?}");
    assert!(modes.contains_key("proliferator-3"));
    // much cheaper than without proliferator; never worse than the Buildings plan
    let c = p.compute.as_ref().unwrap().total;
    let mut none = s.clone();
    none.effects.clear();
    let c_none = plan(&d, &none).compute.unwrap().total;
    assert!(c < 0.7 * c_none, "{c} vs {c_none}");
    let mut b = s.clone();
    b.objective = Objective::Buildings;
    let c_b = plan(&d, &b).compute.unwrap().total;
    assert!(c <= c_b + 1e-6, "{c} vs buildings {c_b}");
    // wind power makes MW dear: fewer sprayed recipes (report §4.3)
    let mut wind = s.clone();
    wind.generator = "wind".into();
    let pw = plan(&d, &wind);
    let sprayed = |p: &Plan| p.recipes.iter().filter(|r| r.mode.contains("proliferator")).count();
    assert!(sprayed(&pw) < sprayed(&p), "{} vs {}", sprayed(&pw), sprayed(&p));
}

#[test]
#[ignore]
fn compute_universe_matrix_report() {
    // cargo test -q compute_universe -- --ignored --nocapture: 60 universe matrix/min, preset comparison
    let (d, mut s) = dsp_compute(&[("universe-matrix", 60.0)]);
    for (gen, fx) in [("artificial_star", true), ("wind", true), ("artificial_star", false)] {
        s.generator = gen.into();
        s.effects = if fx { vec!["proliferator-3-speed".into(), "proliferator-3-products".into()] } else { vec![] };
        for obj in [Objective::Compute, Objective::Buildings] {
            s.objective = obj;
            let p = plan(&d, &s);
            let c = p.compute.as_ref().unwrap();
            let sprayed = p.recipes.iter().filter(|r| r.mode.contains("proliferator")).count();
            println!(
                "{gen:15} prolif {fx:5} {obj:?}: {} buildings, {:.1} MW, compute {:.1} (sorters {:.1}, gen {:.1}) ≈ {:.0} µs, sprayed {sprayed}/{} gap {:?}",
                p.buildings, p.production_power, c.total, c.sorters, c.generation, c.us_per_tick, p.recipes.len(), p.gap
            );
        }
    }
}

fn space_age(targets: &[(&str, f64)], planets: &[&str]) -> (GameData, PlanSettings) {
    let d = GameData::for_set("factorio", "2x1");
    let mut s = settings(targets);
    s.game = "factorio".into();
    s.dataset = "2x1".into();
    s.integer = false;
    s.planets = planets.iter().map(|p| p.to_string()).collect();
    (d, s)
}

#[test]
fn planet_filter_space_age() {
    let all = ["nauvis", "vulcanus", "gleba", "fulgora", "aquilo", "space-platform"];
    let d = GameData::for_set("factorio", "2x1");
    assert_eq!(d.locations.len(), 6);
    assert_eq!(d.resource("calcite").unwrap().locations, vec!["vulcanus".to_string()]);
    assert!(d.recipe("yumako-tree").unwrap().locations == vec!["gleba".to_string()]);
    // base data has no locations: the filter never applies
    assert!(GameData::for_set("factorio", "").locations.is_empty());
    assert_eq!(PlanSettings::default().planets, vec!["nauvis".to_string()]);

    // Gleba plants grow from their own seeds (no raw cost): with every planet, plastic is free
    // bioplastic; on Nauvis only it comes from crude oil
    let (d, s) = space_age(&[("plastic-bar", 60.0)], &all);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert!(p.recipes.iter().any(|r| r.id == "bioplastic"), "all planets: bioplastic");
    let (d, s) = space_age(&[("plastic-bar", 60.0)], &["nauvis"]);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert_balanced(&p);
    assert!(!p.recipes.iter().any(|r| r.id == "bioplastic" || r.id.starts_with("yumako") || r.id.starts_with("jelly")));
    // coal liquefaction / crude oil, not plants
    assert!(p.raw.iter().any(|l| (l.item == "crude-oil" || l.item == "coal") && l.rate > 1.0), "{:?}", p.raw);
    assert!(p.raw.iter().all(|l| d.resource(&l.item).unwrap().locations.iter().any(|x| x == "nauvis") || d.resource(&l.item).unwrap().locations.is_empty()));

    // tungsten is Vulcanus-only: infeasible on Nauvis with a hint, fine with Vulcanus or as a supply
    let (d, s) = space_age(&[("tungsten-plate", 10.0)], &["nauvis"]);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Infeasible);
    assert!(p.warnings.iter().any(|w| w.contains("Tungsten ore (Vulcanus)")), "{:?}", p.warnings);
    let (d, s) = space_age(&[("tungsten-plate", 10.0)], &["nauvis", "vulcanus"]);
    assert_eq!(plan(&d, &s).status, Status::Optimal);
    let (d, mut s) = space_age(&[("tungsten-plate", 10.0)], &["nauvis"]);
    // tungsten plate is cast with molten iron, which needs calcite (also Vulcanus)
    s.supplies = vec![
        ItemRate { item: "tungsten-ore".into(), rate: 1000.0 },
        ItemRate { item: "calcite".into(), rate: 1000.0 },
    ];
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert!(p.supplied.iter().any(|l| l.item == "tungsten-ore"));

    // asteroid chunks only with the space platform
    let (d, s) = space_age(&[("carbon", 60.0)], &["nauvis"]);
    let p = plan(&d, &s);
    assert!(p.raw.iter().all(|l| !l.item.contains("asteroid")));
}

#[test]
fn research_productivity_families() {
    let d = GameData::for_set("factorio", "2x1");
    let fam = d.research.iter().find(|f| f.id == "steel-plate-productivity").unwrap();
    assert_eq!(fam.recipes, vec!["steel-plate".to_string(), "casting-steel".to_string()]);
    assert!(close(fam.step, 0.1));
    assert_eq!(d.research.len(), 8);
    // 60 steel/min, furnaces only (no foundry casting): 5 iron plates per steel, +50 % at level 5
    let mut s = settings(&[("steel-plate", 60.0)]);
    s.game = "factorio".into();
    s.dataset = "2x1".into();
    s.integer = false;
    s.disabled_recipes = vec!["casting-steel".into(), "casting-iron".into()];
    let base = plan(&d, &s);
    let plates = |p: &Plan| p.recipes.iter().find(|r| r.id == "steel-plate").unwrap().inputs[0].rate;
    assert!(close(plates(&base), 300.0), "{}", plates(&base));
    for r in &fam.recipes {
        s.research_productivity.insert(r.clone(), 5.0 * fam.step);
    }
    let p = plan(&d, &s);
    assert!(close(plates(&p), 200.0), "{}", plates(&p));
    // capped at +300 %: level 40 counts as 30
    for r in &fam.recipes {
        s.research_productivity.insert(r.clone(), 40.0 * fam.step);
    }
    assert!(close(plates(&plan(&d, &s)), 75.0));
}
