use satisfactory_calculator::planner::{plan, GameData, ItemRate, Objective, PlanSettings};
fn main() {
    let d = GameData::default_data();
    for t in ["Heavy Modular Frame", "Turbo Motor"] {
        for ms in [1000u64, 4000, 30000] {
            let s = PlanSettings { targets: vec![ItemRate { item: t.into(), rate: 7.3 }], objective: Objective::Buildings, allow_all_alternates: true, max_clock_percent: 250.0, time_limit_ms: ms, ..Default::default() };
            let p = plan(&d, &s);
            println!("{t:22} limit={ms:6}\t{:7.0} ms\topt={}\tgap={:?}\tbld={}\tobj={:.2}\tlp={:.2}", p.solve_ms, p.proven_optimal, p.gap, p.buildings, p.objective, p.lp_bound);
        }
    }
}
