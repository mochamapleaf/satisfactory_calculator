use satisfactory_calculator::planner::*;
use satisfactory_calculator::planner::data::GameData;
use std::collections::HashMap;
fn run(name: &str, zero: &[&str], effects: &[&str]) { runw(name, zero, effects, None) }
fn runw(name: &str, zero: &[&str], effects: &[&str], w: Option<HashMap<String,f64>>) {
    let d = GameData::for_game("dsp");
    let mut s = PlanSettings::default();
    s.game = "dsp".into();
    s.targets = vec![ItemRate { item: "universe-matrix".into(), rate: 1.0 }];
    s.integer = false;
    s.allow_all_alternates = true;
    s.effects = effects.iter().map(|x| x.to_string()).collect();
    if let Some(w) = w { s.objective = Objective::Custom; s.cost = CostWeights { resources: 1.0, buildings: 0.0001, power: 0.0, shards: 0.0, surplus: 0.0, resource_weights: w, building_weights: HashMap::new() }; }
    let mut lim: HashMap<String, Option<f64>> = HashMap::new();
    for z in zero { lim.insert(z.to_string(), Some(0.0)); }
    s.resource_limits = lim;
    let p = plan(&d, &s);
    let mut raw: Vec<_> = p.raw.iter().map(|r| format!("{} {:.3}", r.item, r.rate)).collect();
    raw.sort();
    println!("## {name}: status {:?} buildings {:.1} power {:.1}MW\n  {}", p.status, p.buildings, p.production_power, raw.join(", "));
    for w in &p.warnings { println!("  warn: {w}"); }
}
fn main() {
    let df = ["df-energy-shard","df-negentropy-singularity","df-matter-recombinator","df-dark-fog-matrix","df-silicon-based-neuron","df-core-element"];
    let rare = ["fire-ice","kimberlite-ore","fractal-silicon","optical-grating-crystal","organic-crystal","spiniform-stalagmite-crystal","unipolar-magnet","sulfuric-acid"];
    let mut nodf_norare: Vec<&str> = df.to_vec(); nodf_norare.extend(rare.iter());
    run("A all rare, no DF, no prolif", &df, &[]);
    run("B no rare, no DF, no prolif", &nodf_norare, &[]);
    let mut c: Vec<&str> = df.to_vec(); c.extend(["spiniform-stalagmite-crystal","unipolar-magnet","optical-grating-crystal"]);
    run("C no grating/stalagmite/magnet, no DF", &c, &[]);
    run("D all rare + DF items allowed", &[], &[]);
    // scarcity weights = 1e6 / galaxy stock (seed 12345678, 32 stars, x0.1, actual veins); renewables small
    let stock = [("iron-ore",127.78),("copper-ore",129.60),("silicon-ore",53.34),("titanium-ore",114.15),("stone",65.55),("coal",17.79),("fire-ice",24.55),("organic-crystal",1.78),("kimberlite-ore",7.07),("optical-grating-crystal",5.39),("spiniform-stalagmite-crystal",2.76),("fractal-silicon",3.09),("unipolar-magnet",0.18)];
    let mut w: HashMap<String,f64> = stock.iter().map(|(k,v)| (k.to_string(), 1.0/v)).collect();
    for r in ["crude-oil","water","sulfuric-acid","hydrogen","deuterium","critical-photon"] { w.insert(r.into(), 0.001); }
    runw("E scarcity-weighted (seed stock), no DF, no prolif", &df, &[], Some(w.clone()));
    runw("F scarcity-weighted, no DF, prolif Mk3 extra", &df, &["proliferator-3-products"], Some(w.clone()));
    for k in df { w.insert(k.into(), 0.0001); }
    // threshold: smallest vein-utilization level L with B_r >= a_r(x) * S(L) for all finite ores
    let s_of = |l: i32| -> f64 { (l..6000).map(|n| 4000.0 * (n - 5) as f64 * 0.94f64.powi(n - 1)).sum() };
    for prolif in [false, true] {
        let mut found = None;
        for l in 6..200 {
            let d = GameData::for_game("dsp");
            let mut st = PlanSettings::default();
            st.game = "dsp".into(); st.integer = false; st.allow_all_alternates = true;
            st.targets = vec![ItemRate { item: "universe-matrix".into(), rate: 1.0 }];
            if prolif { st.effects = vec!["proliferator-3-products".into()]; }
            let sl = s_of(l);
            for (k, v) in stock.iter() { st.resource_limits.insert(k.to_string(), Some(v * 1e6 / sl)); }
            for k in df { st.resource_limits.insert(k.to_string(), Some(0.0)); }
            let p = plan(&d, &st);
            if p.status == Status::Optimal { found = Some((l, sl)); break; }
        }
        println!("## threshold (prolif Mk3 extra = {prolif}): first feasible L = {:?}", found);
    }
    runw("G scarcity-weighted, DF items ~free, prolif Mk3 extra", &[], &["proliferator-3-products"], Some(w));
}
