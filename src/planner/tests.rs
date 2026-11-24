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

    // review 3 L2: negative / non-finite values count as 0
    for r in &fam.recipes {
        s.research_productivity.insert(r.clone(), -2.0);
    }
    assert!(close(plates(&plan(&d, &s)), 300.0));
    for r in &fam.recipes {
        s.research_productivity.insert(r.clone(), f64::NAN);
    }
    assert!(close(plates(&plan(&d, &s)), 300.0));

    // review 3 M1: a value left over from a Space Age set does nothing in a set without the
    // technology (2.1 base + quality has no research)
    let q = GameData::for_set("factorio", "2q1");
    assert!(q.research.is_empty());
    s.dataset = "2q1".into();
    s.planets = vec![];
    for r in &fam.recipes {
        s.research_productivity.insert(r.clone(), 1.0);
    }
    assert!(close(plates(&plan(&q, &s)), 300.0), "{}", plates(&plan(&q, &s)));
}

/// Review 3 H3: scrap recycling productivity research applies although the recycler (and the
/// recipe) refuse productivity modules. Fulgora, 60 holmium ore/min: 6 000 scrap/min (holmium ore is
/// 1 % of scrap recycling output, 0.2 per 4 scrap per 0.2 s run) and 40 recyclers; +50 % → 4 000 and 26.7.
#[test]
fn scrap_recycling_research() {
    for set in ["spa", "2x1"] {
        let d = GameData::for_set("factorio", set);
        let mut s = settings(&[("holmium-ore", 60.0)]);
        s.game = "factorio".into();
        s.dataset = set.into();
        s.integer = false;
        s.planets = vec!["fulgora".into()];
        let rid = &d.research.iter().find(|f| f.id == "scrap-recycling-productivity").unwrap().recipes[0];
        assert!(!d.recipe(rid).unwrap().productivity_allowed);
        let scrap = |p: &Plan| p.raw.iter().find(|l| l.item == "scrap").map(|l| l.rate).unwrap_or(0.0);
        let rec = |p: &Plan| p.recipes.iter().find(|r| r.id == *rid).map(|r| r.machines).unwrap_or(0.0);
        let p = plan(&d, &s);
        assert_eq!(p.status, Status::Optimal, "{set} {:?}", p.warnings);
        assert!(close(scrap(&p), 6000.0), "{set}: {}", scrap(&p));
        assert!(close(rec(&p), 40.0), "{set}: {}", rec(&p));
        s.research_productivity.insert(rid.clone(), 0.5);
        let p = plan(&d, &s);
        assert!(close(scrap(&p), 4000.0), "{set}: {}", scrap(&p));
        assert!(close(rec(&p), 80.0 / 3.0), "{set}: {}", rec(&p));
        assert_balanced(&p);
    }
}

fn factorio_set(set: &str, targets: &[(&str, f64)], planets: &[&str]) -> (GameData, PlanSettings) {
    let d = GameData::for_set("factorio", set);
    let mut s = settings(targets);
    s.game = "factorio".into();
    s.dataset = set.into();
    s.integer = false;
    s.planets = planets.iter().map(|p| p.to_string()).collect();
    (d, s)
}

fn input_of(p: &Plan, recipe: &str, item: &str) -> f64 {
    p.recipes.iter().filter(|r| r.id == recipe).flat_map(|r| &r.inputs).filter(|l| l.item == item).map(|l| l.rate).sum()
}

fn output_of(p: &Plan, recipe: &str, item: &str) -> f64 {
    p.recipes.iter().filter(|r| r.id == recipe).flat_map(|r| &r.outputs).filter(|l| l.item == item).map(|l| l.rate).sum()
}

fn machines_of(p: &Plan, recipe: &str) -> f64 {
    p.recipes.iter().filter(|r| r.id == recipe).map(|r| r.machines).sum()
}

/// Review 3 H4: heat exchangers burn heat (600 MJ/min each), reactors and heating towers burn fuel,
/// boilers and burner furnaces burn coal, the biochamber burns nutrients; generators are gone.
#[test]
fn heat_and_fuel() {
    // one heat exchanger (6 185.567 steam/min) = 600 MJ/min of heat = 0.25 reactor (40 MW, no
    // neighbour bonus) = 0.075 uranium cells/min (8 GJ each), returned as spent cells
    let rate = 6000.0 * 60.0 / 58.2;
    for set in ["2x1", "2q1", "spa"] {
        // the exchanger's 500 °C steam is "steam" or "steam-500" depending on the set
        let d = GameData::for_set("factorio", set);
        let steam = d.recipe("heat-exchanger-steam-boil").map(|r| r.outputs[0].0.clone()).unwrap_or("steam".into());
        let (d, mut s) = factorio_set(set, &[(steam.as_str(), rate)], &["nauvis"]);
        assert!(d.buildings.iter().all(|b| b.power >= 0.0), "{set}: generators dropped");
        assert!(d.building("steam-turbine").is_none());
        let hx = d.building("heat-exchanger").unwrap();
        assert_eq!(hx.power, 0.0, "{set}");
        s.disabled_buildings = vec!["heating-tower".into()];
        let p = plan(&d, &s);
        assert_eq!(p.status, Status::Optimal, "{set} {:?}", p.warnings);
        assert_balanced(&p);
        assert!(close(machines_of(&p, "heat-exchanger-steam-boil"), 1.0), "{set}");
        assert!(close(input_of(&p, "heat-exchanger-steam-boil", "heat"), 600.0), "{set}");
        assert!(close(machines_of(&p, "nuclear-reactor-reactor"), 0.25), "{set} {}", machines_of(&p, "nuclear-reactor-reactor"));
        assert!(close(input_of(&p, "nuclear-reactor-reactor", "uranium-fuel-cell"), 0.075), "{set}");
        assert!(close(output_of(&p, "nuclear-reactor-reactor", "depleted-uranium-fuel-cell"), 0.075), "{set}");
        assert!(p.production_power < 0.1, "{set}: exchanger draws no electricity ({})", p.production_power);
        // heating tower (Space Age): 16 MW of coal at 250 % → 40 MW of heat, 0.25 tower = 60 coal/min
        if set != "2q1" {
            s.disabled_buildings = vec!["nuclear-reactor".into()];
            let p = plan(&d, &s);
            assert_eq!(p.status, Status::Optimal, "{set} {:?}", p.warnings);
            assert!(close(machines_of(&p, "heating-tower-reactor"), 0.25), "{set}");
            assert!(close(input_of(&p, "heating-tower-reactor", "coal"), 60.0), "{set}");
        }
    }
    // base 2.0: a boiler (1.8 MW of coal, 3 600 steam/min) burns 27 coal/min and no electricity
    let (d, s) = factorio_set("2.0", &[("steam", 6000.0)], &[]);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    assert!(close(input_of(&p, "steam", "coal"), 45.0), "{}", input_of(&p, "steam", "coal"));
    assert!(close(p.production_power, 0.0));
    // stone furnace (90 kW of coal): 60 plates/min = 3.2 furnaces = 4.32 coal/min
    let (d, mut s) = factorio_set("2.0", &[("iron-plate", 60.0)], &[]);
    s.disabled_buildings = vec!["steel-furnace".into(), "electric-furnace".into()];
    let p = plan(&d, &s);
    assert!(close(machines_of(&p, "iron-plate"), 3.2));
    assert!(close(input_of(&p, "iron-plate", "coal"), 4.32), "{}", input_of(&p, "iron-plate", "coal"));
    // the steel furnace is twice as fast for the same fuel: half the coal per plate
    s.disabled_buildings = vec!["stone-furnace".into(), "electric-furnace".into()];
    assert!(close(input_of(&plan(&d, &s), "iron-plate", "coal"), 2.16));
    // biochamber: 500 kW of nutrients (2 MJ) = 15/min per machine; efficiency modules scale it down
    // to the 20 % floor like electric power: 4 × efficiency 3 → 3/min per machine
    let d = GameData::for_set("factorio", "2x1");
    let mut s = factorio_set("2x1", &[], &["gleba"]).1;
    s.effects = vec!["efficiency-module-3".into()];
    let r = d.recipe("bioflux").unwrap();
    let modes = rules::modes(&d, r, &s);
    let nutrients = |key: &str| {
        let m = modes.iter().find(|m| m.key == key).unwrap();
        m.extra_inputs.iter().find(|x| x.0 == "nutrients").unwrap().1 * m.speed
    };
    assert!(close(nutrients("biochamber"), 15.0));
    assert!(close(nutrients("biochamber|efficiency-module-3"), 3.0), "{}", nutrients("biochamber|efficiency-module-3"));
}

/// Review 3 M2: machine locations (crusher on the space platform, captive biter spawner on Nauvis)
/// and a hint naming the target whose recipes the planet filter removed.
#[test]
fn planet_filter_machines_and_hint() {
    // Nauvis with carbonic chunks supplied: no crusher on Nauvis
    let (d, mut s) = factorio_set("2x1", &[("carbon", 60.0)], &["nauvis"]);
    s.supplies = vec![ItemRate { item: "carbonic-asteroid-chunk".into(), rate: 1e6 }];
    let p = plan(&d, &s);
    assert!(!p.recipes.iter().any(|r| r.id.contains("asteroid-crushing")), "{:?}", p.recipes.iter().map(|r| &r.id).collect::<Vec<_>>());
    s.planets.push("space-platform".into());
    assert!(plan(&d, &s).recipes.iter().any(|r| r.id == "carbonic-asteroid-crushing"));
    // biter eggs come from the captive spawner on Nauvis (burning bioflux), not from nowhere
    let (d, s) = factorio_set("2x1", &[("biter-egg", 60.0)], &["gleba"]);
    assert!(d.resource("biter-egg").is_none());
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Infeasible);
    assert!(p.warnings.iter().any(|w| w.contains("Biter egg (Nauvis)")), "{:?}", p.warnings);
    let (d, s) = factorio_set("2x1", &[("biter-egg", 60.0)], &["nauvis", "gleba"]);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert!(input_of(&p, "biter-egg", "bioflux") > 0.0);
    // a Vulcanus-only target on Nauvis names the target and its planet
    let (d, s) = factorio_set("2x1", &[("metallurgic-science-pack", 1.0)], &["nauvis"]);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Infeasible);
    assert!(p.warnings.iter().any(|w| w.contains("Metallurgic science pack (Vulcanus)")), "{:?}", p.warnings);
    // only items without a source: steel on Nauvis is fine, so an impossible extra target is the only one named
    let (d, s) = factorio_set("2x1", &[("steel-plate", 60.0), ("tungsten-plate", 1.0)], &["nauvis"]);
    let p = plan(&d, &s);
    let w = p.warnings.iter().find(|w| w.starts_with("Not on the chosen planets")).expect("hint");
    // tungsten plate is cast from tungsten ore and molten iron (calcite or lava); nothing else is named
    assert!(w.contains("Tungsten ore") && w.contains("Calcite"), "{w}");
    assert!(!w.contains("Scrap") && !w.contains("chunk") && !w.contains("Jellynut") && !w.contains("Yumako"), "{w}");
}

/// Review 3 L3: beacons draw full power whatever the clock of the machines they serve; review 3
/// L5: DSP / Factorio modes are plain first, then the boosts in `effects` order (b34e62c order).
#[test]
fn beacon_power_and_mode_order() {
    let (d, mut s) = factorio_set("2.0", &[("iron-gear-wheel", 1000.0)], &[]);
    s.supplies = vec![ItemRate { item: "iron-plate".into(), rate: 1e9 }];
    s.disabled_buildings = vec!["assembling-machine-1".into(), "assembling-machine-2".into()];
    s.objective = Objective::Buildings;
    s.cost = Objective::Buildings.preset();
    s.beacon_module = "speed-module-3".into();
    s.beacon_counts = vec![8];
    s.beacon_share = 8.0;
    s.integer = true;
    let p = plan(&d, &s);
    let r = p.recipes.iter().find(|r| r.id == "iron-gear-wheel").unwrap();
    assert!(r.mode.ends_with("|b8"), "{}", r.mode);
    let m = rules::modes(&d, d.recipe("iron-gear-wheel").unwrap(), &s).into_iter().find(|m| m.key == r.mode).unwrap();
    // 1 000 gears/min need 1000 / (120 × 6.5525) = 1.272 machine-loads on 2 whole machines
    let load: f64 = r.groups.iter().map(|g| g.count * g.clock_percent / 100.0).sum();
    assert!(close(r.machines, 2.0) && load < 1.99, "{} {load}", r.machines);
    assert!(close(m.beacon_power, 0.48), "{}", m.beacon_power);
    assert!(close(r.power, m.power_base * load + m.beacon_power * r.machines), "{} vs {}", r.power, m.power_base * load + m.beacon_power * r.machines);

    // review 3 L4: at most 4 beacon counts (smallest kept)
    s.beacon_counts = vec![12, 1, 2, 4, 8, 6];
    let opts = rules::beacon_options(&d, &s);
    assert_eq!(opts.iter().map(|o| o.count).collect::<Vec<_>>(), vec![0, 1, 2, 4, 6]);

    let d = GameData::for_game("dsp");
    let mut s = settings(&[]);
    s.game = "dsp".into();
    s.effects = vec!["proliferator-3-speed".into(), "proliferator-3-products".into()];
    let r = d.recipe("circuit-board").unwrap();
    let keys: Vec<String> = rules::modes(&d, r, &s).into_iter().map(|m| m.key).collect();
    let b = &r.producers[0];
    assert_eq!(keys[0], *b);
    assert_eq!(keys[1], format!("{b}|proliferator-3-speed"));
    assert_eq!(keys[2], format!("{b}|proliferator-3-products"));
}

/// Xinyu 2026-11-21 「为何生产1级2级增产剂不用3级的？」: with all tiers enabled the steady-state
/// optimum (Buildings / Compute) still sprays a few cheap recipes (Mk.I / Mk.II making, circuit
/// boards…) with Mk.I, because a Mk.I spray costs far fewer upstream buildings. The plan says so,
/// and unticking the lower tiers gives an all-Mk.III plan that costs slightly more.
#[test]
fn dsp_proliferator_tier_mix_is_optimal_and_explained() {
    let all: Vec<String> =
        ["1", "2", "3"].iter().flat_map(|t| [format!("proliferator-{t}-speed"), format!("proliferator-{t}-products")]).collect();
    let (d, mut s) = dsp_compute(&[("universe-matrix", 60.0)]);
    s.integer = false;
    s.objective = Objective::Buildings;
    s.effects = all.clone();
    let mix = plan(&d, &s);
    assert_eq!(mix.status, Status::Optimal);
    let modes = sprayed_modes(&mix);
    assert!(modes["proliferator-2"].contains("proliferator-1-"), "{modes:?}");
    assert!(mix.warnings.iter().any(|w| w.contains("steady-state optimum")), "{:?}", mix.warnings);
    // Mk.III only: every sprayed recipe uses Mk.III, no hint, and the objective is not better
    s.effects = all[4..].to_vec();
    let top = plan(&d, &s);
    assert!(top.recipes.iter().filter(|r| r.mode.contains("proliferator")).all(|r| r.mode.contains("proliferator-3-")));
    assert!(!top.warnings.iter().any(|w| w.contains("steady-state optimum")), "{:?}", top.warnings);
    assert!(mix.objective <= top.objective + 1e-9 && top.objective < 1.01 * mix.objective, "{} vs {}", mix.objective, top.objective);
    // Raw-resource objective: Mk.III everywhere already, no hint
    s.effects = all;
    s.objective = Objective::Resources;
    let raw = plan(&d, &s);
    assert!(!raw.warnings.iter().any(|w| w.contains("steady-state optimum")), "{:?}", raw.warnings);
}

/// Review 4 (M4): the tier hint only counts recipes that could have used the best tier in the
/// same mode. With Mk.III enabled for speed only and Mk.I for extra products, a recipe on
/// Mk.I extra products had no Mk.III choice: no hint about "lower-tier sprays".
#[test]
fn dsp_proliferator_hint_ignores_modes_the_best_tier_does_not_offer() {
    let (d, mut s) = dsp_compute(&[("universe-matrix", 60.0)]);
    s.integer = false;
    s.objective = Objective::Buildings;
    s.effects = vec!["proliferator-3-speed".into(), "proliferator-1-products".into()];
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal);
    let modes: Vec<&str> = p.recipes.iter().map(|r| r.mode.as_str()).collect();
    assert!(modes.iter().any(|m| m.ends_with("proliferator-1-products")), "the case needs a Mk.I extra-products recipe: {modes:?}");
    assert!(!p.warnings.iter().any(|w| w.contains("steady-state optimum")), "{:?}", p.warnings);
    // same tiers, both in speed mode: a Mk.I speed recipe is a real tier choice
    s.effects = vec!["proliferator-3-speed".into(), "proliferator-1-speed".into()];
    let p = plan(&d, &s);
    let lower = p.recipes.iter().any(|r| r.mode.ends_with("proliferator-1-speed"));
    assert_eq!(lower, p.warnings.iter().any(|w| w.contains("steady-state optimum")), "{:?}", p.warnings);
}

#[test]
fn generators_make_electricity_and_power_the_plan() {
    let d = data();
    // memo step 8: 750 MW from coal = 10 coal generators, 150 coal and 450 water per minute
    let mut s = settings(&[(ELECTRICITY, 750.0)]);
    s.disabled_recipes = d.recipes.iter().filter(|r| r.generator && r.building != "Desc_GeneratorCoal_C").map(|r| r.id.clone()).collect();
    s.disabled_recipes.push("Generator_Desc_CompactedCoal_C".into());
    s.disabled_recipes.push("Generator_Desc_PetroleumCoke_C".into());
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert_balanced(&p);
    let g = p.recipes.iter().find(|r| r.id == "Generator_Desc_Coal_C").expect("coal generators");
    assert!(close(g.throughput, 10.0), "{}", g.throughput);
    let coal: f64 = g.inputs.iter().filter(|i| i.item == "Desc_Coal_C").map(|i| i.rate).sum();
    assert!(close(coal, 150.0), "{coal}");
    // nuclear: 2500 MW per plant, 0.2 rods/min, 10 uranium waste/min
    let n = d.recipe("Generator_Desc_NuclearFuelRod_C").unwrap();
    assert!(n.inputs.iter().any(|(i, q)| i == "Desc_NuclearFuelRod_C" && close(*q, 0.2)));
    assert!(n.inputs.iter().any(|(i, q)| i == "Desc_Water_C" && close(*q, 240.0)));
    assert!(n.outputs.iter().any(|(i, q)| i == "Desc_NuclearWaste_C" && close(*q, 10.0)));
    // self-powered: generators cover the plan's own draw, fuel chain included
    let mut sp = settings(&[("Desc_Computer_C", 10.0)]);
    sp.self_power = true;
    let p = plan(&d, &sp);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    let made: f64 = p
        .recipes
        .iter()
        .filter_map(|r| d.recipe(&r.id).filter(|x| x.generator).map(|x| x.outputs[0].1 * r.throughput))
        .sum();
    let draw = p.production_power + p.extraction_power;
    eprintln!("self-powered computers: {made:.1} MW made, {draw:.1} MW drawn; {:?}", p.warnings.last());
    for r in p.recipes.iter().filter(|r| d.recipe(&r.id).is_some_and(|x| x.generator)) {
        eprintln!("  {} ×{} ({})", r.name, r.machines, r.mode_label);
    }
    assert!(draw > 0.0 && made >= draw * (1.0 - 1e-6), "{made} vs {draw}");
    // whole buildings with shards (review: the relaxation-sized target under-covered these)
    for (item, rate) in [("Desc_AluminumIngot_C", 10.0), ("Desc_ModularFrameHeavy_C", 7.0), ("Desc_Motor_C", 10.0), ("Desc_Computer_C", 10.0)] {
        for obj in [Objective::Buildings, Objective::Resources, Objective::Power] {
            let mut si = settings(&[(item, rate)]);
            si.self_power = true;
            si.integer = true;
            si.objective = obj;
            si.max_clock_percent = 250.0;
            let pi = plan(&d, &si);
            assert_eq!(pi.status, Status::Optimal, "{item} {obj:?} {:?}", pi.warnings);
            let made_i: f64 = pi.recipes.iter().flat_map(|r| r.outputs.iter()).filter(|o| o.item == ELECTRICITY).map(|o| o.rate).sum();
            let draw = pi.production_power + pi.extraction_power;
            eprintln!("{item} {obj:?}: {made_i:.1} MW made, {draw:.1} drawn");
            assert!(made_i >= draw * (1.0 - 1e-6) - 1e-6, "{item} {obj:?}: {made_i} < {draw}");
            assert!(pi.targets.iter().all(|t| t.item != ELECTRICITY));
        }
    }
    // nuclear waste left over is flagged
    let mut nuc = settings(&[(ELECTRICITY, 10000.0)]);
    nuc.disabled_recipes = d.recipes.iter().filter(|r| r.generator && r.building != "Desc_GeneratorNuclear_C").map(|r| r.id.clone()).collect();
    let pn = plan(&d, &nuc);
    assert!(pn.warnings.iter().any(|w| w.contains("Nuclear waste")), "{:?}", pn.warnings);
    // without the option nothing changes
    let p0 = plan(&d, &settings(&[("Desc_Computer_C", 10.0)]));
    assert!(p0.recipes.iter().all(|r| !d.recipe(&r.id).is_some_and(|x| x.generator)));
}

