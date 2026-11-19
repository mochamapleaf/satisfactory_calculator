//! cargo run --example plan -- '<settings json>'
use satisfactory_calculator::planner::{plan, GameData, PlanSettings};
fn main() {
    let arg = std::env::args().nth(1).expect("settings json");
    let s: PlanSettings = serde_json::from_str(&arg).unwrap();
    let p = plan(&GameData::default_data(), &s);
    println!("status {:?} obj {:.4} buildings {} power {:.1}+{:.1} MW", p.status, p.objective, p.buildings, p.production_power, p.extraction_power);
    for r in &p.recipes { println!("  {:>8.3} x {} [{}]{}", r.machines, r.name, r.building, if r.alt {" (alt)"} else {""}); }
    for l in &p.raw { println!("  raw {:>9.2} {}", l.rate, l.name); }
    for l in &p.surplus { println!("  surplus {:>9.2} {}", l.rate, l.name); }
    for w in &p.warnings { println!("  ! {w}"); }
}
