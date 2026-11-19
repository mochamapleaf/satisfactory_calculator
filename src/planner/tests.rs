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
fn overclock_power_exponent() {
    let d = data();
    let mut s = settings(&[("Reinforced Iron Plate", 60.0)]);
    s.clock_percent = 250.0;
    let p = plan(&d, &s);
    let m = machines(&p);
    assert!(close(m["Reinforced Iron Plate"], 4.8));
    let rip = p.recipes.iter().find(|r| r.name == "Reinforced Iron Plate").unwrap();
    let expected = 4.8 * 15.0 * 2.5f64.powf(1.321929);
    assert!((rip.power - expected).abs() < 1e-3, "{} vs {}", rip.power, expected);
    assert_balanced(&p);
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
                let p = plan(&d, &s);
                assert_eq!(p.status, Status::Optimal, "{target} {obj:?} alts={alts}");
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
