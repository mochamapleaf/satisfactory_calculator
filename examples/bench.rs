use satisfactory_calculator::planner::{plan, GameData, ItemRate, Objective, PlanSettings};
fn main() {
    let d = GameData::default_data();
    for t in ["Reinforced Iron Plate", "Heavy Modular Frame", "Turbo Motor", "Fused Modular Frame", "Nuclear Pasta", "Thermal Propulsion Rocket", "Ballistic Warp Drive"] {
        for obj in [Objective::Resources, Objective::Buildings, Objective::Power] {
            for alts in [false, true] {
                let s = PlanSettings { targets: vec![ItemRate { item: t.into(), rate: 7.3 }], objective: obj, allow_all_alternates: alts, max_clock_percent: 250.0, ..Default::default() };
                let p = plan(&d, &s);
                println!("{t:28} {obj:?}\talts={alts}\t{:?}\t{:7.0} ms\topt={}\tgap={:?}\tbld={}\tobj={:.2}\tlp={:.2}", p.status, p.solve_ms, p.proven_optimal, p.gap, p.buildings, p.objective, p.lp_bound);
            }
        }
    }
}
