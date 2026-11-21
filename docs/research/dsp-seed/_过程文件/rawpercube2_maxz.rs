// Order-of-magnitude max white cubes/min for seed 12345678 (32 stars, x0.1, actual veins),
// research of level L in progress (mining level L-1, speed x(1+0.1(L-1))), fewest-buildings LP.
// Stock quotas: z * B_r / S(L) (solid ores, crude-oil stock 10.12M, fire-ice planet stock).
// Rate caps (/min): gas-giant net hydrogen/deuterium/fire-ice (40 collectors per giant, 30 MW self-use
// deducted by heat value), crude oil <= initial seep rate x speed (upper bound; seeps decay to an unknown floor).
// Photons and water/sulfuric acid unbounded (Dyson power unknown). Binary search on z.
use satisfactory_calculator::planner::*;
use satisfactory_calculator::planner::data::GameData;

fn s_of(l: i32) -> f64 { (l..6000).map(|n| 4000.0 * (n - 5) as f64 * 0.94f64.powi(n - 1)).sum() }

fn feasible(z: f64, l: i32, prolif: bool, giant: (f64, f64, f64)) -> Option<Plan> {
    let stock = [("iron-ore",127.78),("copper-ore",129.60),("silicon-ore",53.34),("titanium-ore",114.15),("stone",65.55),("coal",17.79),("organic-crystal",1.78),("kimberlite-ore",7.07),("optical-grating-crystal",5.39),("spiniform-stalagmite-crystal",2.76),("fractal-silicon",3.09),("unipolar-magnet",0.182)];
    let df = ["df-energy-shard","df-negentropy-singularity","df-matter-recombinator","df-dark-fog-matrix","df-silicon-based-neuron","df-core-element"];
    let speed = 1.0 + 0.1 * (l - 1) as f64;
    let sl = s_of(l);
    let d = GameData::for_game("dsp");
    let mut st = PlanSettings::default();
    st.game = "dsp".into(); st.integer = false; st.allow_all_alternates = true;
    st.objective = Objective::Custom; let mut w = std::collections::HashMap::new(); w.insert("critical-photon".to_string(), 1.0); st.cost = CostWeights { resources: 1.0, buildings: 1e-6, power: 0.0, shards: 0.0, surplus: 0.0, resource_weights: w, building_weights: std::collections::HashMap::new() };
    st.targets = vec![ItemRate { item: "universe-matrix".into(), rate: z }];
    if prolif { st.effects = vec!["proliferator-3-products".into()]; }
    for (k, v) in stock.iter() { st.resource_limits.insert(k.to_string(), Some(z * v * 1e6 / sl)); }
    let oil = (z * 10.12e6 / sl).min(404.8 * speed * 60.0);
    st.resource_limits.insert("crude-oil".into(), Some(oil));
    st.resource_limits.insert("fire-ice".into(), Some(z * 24.55e6 / sl + giant.2 * 60.0));
    st.resource_limits.insert("hydrogen".into(), Some(giant.0 * 60.0));
    st.resource_limits.insert("deuterium".into(), Some(giant.1 * 60.0));
    for k in df { st.resource_limits.insert(k.to_string(), Some(0.0)); }
    let p = plan(&d, &st);
    if p.status == Status::Optimal { Some(p) } else { None }
}

fn main() {
    let giant = (22860.0, 1080.0, 11094.0); // net /s at mining level 80
    for &prolif in &[false, true] {
        for &z in &[1000.0, 10000.0, 20000.0, 50000.0, 100000.0, 200000.0] {
            match feasible(z, 81, prolif, giant) {
                Some(p) => {
                    let ph = p.raw.iter().find(|r| r.item == "critical-photon").map(|r| r.rate).unwrap_or(0.0);
                    let mut raw: Vec<_> = p.raw.iter().filter(|r| r.rate > 1e-6).map(|r| format!("{} {:.0}", r.item, r.rate)).collect();
                    raw.sort();
                    println!("## L=81 prolif={prolif} z={z}: photons/cube {:.3}, ray-receiver power {:.1} GW, buildings {:.0}\n  {}", ph / z, ph * 1200.0 / 60.0 / 1000.0, p.buildings, raw.join(", "));
                }
                None => println!("## L=81 prolif={prolif} z={z}: infeasible"),
            }
        }
    }
}
