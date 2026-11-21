// Stock-constrained threshold scan (seed 12345678, 32 stars, x0.1, actual veins).
// For each vein-utilization level L: limit every finite resource r (solid ores AND crude oil)
// to B_r / (f * S(L)) per white cube, objective = fewest buildings, continuous LP, 1 cube/min.
// f = 1.0 (research without proliferator) or 0.8 (Mk.III sprayed cubes, x1.25 hashes; unverified).
// B_oil = sum of seep amounts = 404.8/s / 4e-5 = 10.12M (upper bound: per-seep floor 2500 not subtracted).
use satisfactory_calculator::planner::*;
use satisfactory_calculator::planner::data::GameData;
use std::collections::HashMap;

fn s_of(l: i32) -> f64 { (l..6000).map(|n| 4000.0 * (n - 5) as f64 * 0.94f64.powi(n - 1)).sum() }

fn solve(l: i32, f: f64, prolif: bool, oil_cap: bool) -> Plan { solve2(l, f, prolif, oil_cap, false, false) }
pub fn solve2(l: i32, f: f64, prolif: bool, oil_cap: bool, no_rare: bool, no_gas: bool) -> Plan {
    let stock = [("iron-ore",127.78),("copper-ore",129.60),("silicon-ore",53.34),("titanium-ore",114.15),("stone",65.55),("coal",17.79),("fire-ice",24.55),("organic-crystal",1.78),("kimberlite-ore",7.07),("optical-grating-crystal",5.39),("spiniform-stalagmite-crystal",2.76),("fractal-silicon",3.09),("unipolar-magnet",0.182),("crude-oil",10.12)];
    let df = ["df-energy-shard","df-negentropy-singularity","df-matter-recombinator","df-dark-fog-matrix","df-silicon-based-neuron","df-core-element"];
    let d = GameData::for_game("dsp");
    let mut st = PlanSettings::default();
    st.game = "dsp".into(); st.integer = false; st.allow_all_alternates = true;
    st.objective = Objective::Buildings;
    st.targets = vec![ItemRate { item: "universe-matrix".into(), rate: 1.0 }];
    if prolif { st.effects = vec!["proliferator-3-products".into()]; }
    let sl = f * s_of(l);
    for (k, v) in stock.iter() {
        if *k == "crude-oil" && !oil_cap { continue; }
        st.resource_limits.insert(k.to_string(), Some(v * 1e6 / sl));
    }
    for k in df { st.resource_limits.insert(k.to_string(), Some(0.0)); }
    if no_rare { for k in ["fire-ice","kimberlite-ore","fractal-silicon","optical-grating-crystal","organic-crystal","spiniform-stalagmite-crystal","unipolar-magnet"] { st.resource_limits.insert(k.to_string(), Some(0.0)); } }
    if no_gas { for k in ["hydrogen","deuterium"] { st.resource_limits.insert(k.to_string(), Some(0.0)); } }
    plan(&d, &st)
}

fn show(tag: &str, l: i32, p: &Plan) {
    let mut raw: Vec<_> = p.raw.iter().filter(|r| r.rate > 1e-6).map(|r| format!("{} {:.2}", r.item, r.rate)).collect();
    raw.sort();
    println!("  {tag} L={l}: {:?} buildings {:.1}\n    {}", p.status, p.buildings, raw.join(", "));
}

fn main() {
    for l in [6, 19, 25, 40, 60, 81, 98] { println!("S({l}) = {:.0}", s_of(l)); }
    for &prolif in &[false, true] {
        for &f in &[1.0, 0.8] {
            for &oil in &[false, true] {
                let mut first = None;
                for l in 6..200 { if solve(l, f, prolif, oil).status == Status::Optimal { first = Some(l); break; } }
                println!("## prolif_mk3_extra={prolif} research_factor={f} oil_capped={oil}: first feasible L = {:?}", first);
                if let Some(l) = first { show("at threshold", l, &solve(l, f, prolif, oil)); }
            }
            // what the min-building plan looks like at selected levels (oil capped)
            for l in [40, 60, 81, 98, 120] {
                let p = solve(l, f, prolif, true);
                if p.status == Status::Optimal { show("profile", l, &p); }
            }
        }
    }
    println!("# variants (oil capped, fewest buildings, research factor 1)");
    for &prolif in &[false, true] { for &(nr, ng) in &[(true, false), (false, true), (true, true)] {
        let mut first = None;
        for l in 6..200 { if solve2(l, 1.0, prolif, true, nr, ng).status == Status::Optimal { first = Some(l); break; } }
        println!("## prolif={prolif} no_solid_rare={nr} no_gas_giant={ng}: first feasible L = {:?}", first);
        if let Some(l) = first { show("at threshold", l, &solve2(l, 1.0, prolif, true, nr, ng)); }
    } }
}
