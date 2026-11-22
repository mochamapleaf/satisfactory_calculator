
// Stepped-solve benchmark (as the web worker drives it: lp::start, then step(150 ms) until done).
// Records first_ms = time until the first whole-building plan is in hand.
// cargo run --release --example bench_stream -- [ms=4000] [f=filter,...] [label=x]
use satisfactory_calculator::planner::*;

fn st(game: &str, targets: &[(&str, f64)]) -> PlanSettings {
    PlanSettings {
        game: game.into(),
        targets: targets.iter().map(|(i, r)| ItemRate { item: i.to_string(), rate: *r }).collect(),
        ..Default::default()
    }
}
fn v(x: &[&str]) -> Vec<String> {
    x.iter().map(|s| s.to_string()).collect()
}

fn scenarios() -> Vec<(String, String, PlanSettings)> {
    let mut out: Vec<(String, String, PlanSettings)> = vec![];
    let objs = [("res", Objective::Resources), ("bld", Objective::Buildings), ("pow", Objective::Power)];
    // ---- Satisfactory
    let p4: &[(&str, f64)] = &[
        ("Assembly Director System", 4.0),
        ("Magnetic Field Generator", 4.0),
        ("Thermal Propulsion Rocket", 2.0),
        ("Nuclear Pasta", 1.0),
    ];
    let p5: &[(&str, f64)] = &[
        ("Biochemical Sculptor", 2.0),
        ("Ballistic Warp Drive", 0.5),
        ("AI Expansion Server", 1.0),
        ("Nuclear Pasta", 1.0),
    ];
    let sf: Vec<(&str, &[(&str, f64)])> = vec![
        ("rip10", &[("Reinforced Iron Plate", 10.0)]),
        ("hmf7", &[("Heavy Modular Frame", 7.3)]),
        ("turbo_rip", &[("Reinforced Iron Plate", 10.0), ("Turbo Motor", 5.0)]),
        ("pasta", &[("Nuclear Pasta", 1.0)]),
        ("phase4", p4),
        ("phase5", p5),
    ];
    for (name, t) in &sf {
        for alt in [false, true] {
            for (on, o) in objs {
                let mut s = st("satisfactory", t);
                s.allow_all_alternates = alt;
                s.objective = o;
                out.push((format!("sf_{name}_{}_{on}", if alt { "alt" } else { "std" }), "".into(), s));
            }
        }
    }
    for (on, o) in objs {
        let mut s = st("satisfactory", p4);
        s.allow_all_alternates = true;
        s.objective = o;
        s.max_clock_percent = 250.0;
        out.push((format!("sf_phase4_oc250_{on}"), "".into(), s.clone()));
        s.max_clock_percent = 100.0;
        s.auto_somersloops = true;
        s.somersloop_budget = Some(20);
        out.push((format!("sf_phase4_sloop20_{on}"), "".into(), s));
    }
    // ---- DSP
    let df = v(&["df-negentropy-smelter", "df-recomposing-assembler", "df-self-evolution-lab"]);
    let p3 = v(&["proliferator-3-speed", "proliferator-3-products"]);
    let pall = v(&[
        "proliferator-1-products",
        "proliferator-2-products",
        "proliferator-3-products",
        "proliferator-1-speed",
        "proliferator-2-speed",
        "proliferator-3-speed",
    ]);
    let dsp_t: Vec<(&str, &[(&str, f64)])> = vec![
        ("um60", &[("universe-matrix", 60.0)]),
        (
            "multi",
            &[("universe-matrix", 60.0), ("small-carrier-rocket", 5.0), ("solar-sail", 60.0), ("antimatter-fuel-rod", 10.0)],
        ),
    ];
    let dobjs = [("res", Objective::Resources), ("bld", Objective::Buildings), ("cmp", Objective::Compute)];
    for (name, t) in &dsp_t {
        for (en, eff) in [("plain", vec![]), ("p3", p3.clone()), ("pall", pall.clone())] {
            for (on, o) in dobjs {
                let mut s = st("dsp", t);
                s.objective = o;
                s.effects = eff.clone();
                s.disabled_buildings = df.clone();
                out.push((format!("dsp_{name}_{en}_{on}"), "dsp".into(), s));
            }
        }
    }
    // ---- Factorio
    let sci6: &[(&str, f64)] = &[
        ("automation-science-pack", 60.0),
        ("logistic-science-pack", 60.0),
        ("military-science-pack", 60.0),
        ("chemical-science-pack", 60.0),
        ("production-science-pack", 60.0),
        ("utility-science-pack", 60.0),
    ];
    let sci7: &[(&str, f64)] = &[
        ("automation-science-pack", 60.0),
        ("logistic-science-pack", 60.0),
        ("military-science-pack", 60.0),
        ("chemical-science-pack", 60.0),
        ("production-science-pack", 60.0),
        ("utility-science-pack", 60.0),
        ("space-science-pack", 60.0),
    ];
    let fobjs = [("res", Objective::Resources), ("bld", Objective::Buildings), ("pow", Objective::Power)];
    for (ds, dn) in [("", "fa"), ("2x1", "fx")] {
        for (name, t) in [("sci6", sci6), ("sci7", sci7)] {
            for (mn, eff, bm, bc) in [
                ("plain", vec![], "", vec![8u32]),
                ("mod", v(&["productivity-module-3", "speed-module-3"]), "", vec![8]),
                (
                    "modall",
                    v(&["productivity-module-3", "speed-module-3", "efficiency-module-3", "productivity-module-2", "speed-module-2"]),
                    "",
                    vec![8],
                ),
                ("mod_b8", v(&["productivity-module-3", "speed-module-3"]), "speed-module-3", vec![8]),
                ("mod_b4", v(&["productivity-module-3", "speed-module-3"]), "speed-module-3", vec![2, 4, 8, 12]),
            ] {
                for (on, o) in fobjs {
                    let mut s = st("factorio", t);
                    s.dataset = ds.into();
                    s.objective = o;
                    s.effects = eff.clone();
                    s.beacon_module = bm.into();
                    s.beacon_counts = bc.clone();
                    out.push((format!("{dn}_{name}_{mn}_{on}"), ds.into(), s));
                }
            }
        }
    }
    out
}

fn main() {
    let mut ms = 4000u64;
    let mut filter = String::new();
    let mut label = String::from("prod");
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').unwrap_or((a.as_str(), "1"));
        match k {
            "ms" => ms = v.parse().unwrap(),
            "f" => filter = v.into(),
            "label" => label = v.into(),
            _ => panic!("unknown arg {a}"),
        }
    }
    let filters: Vec<&str> = filter.split(',').collect();
    for (name, ds, mut s) in scenarios() {
        if !filter.is_empty() && !filters.iter().any(|f| if let Some(x) = f.strip_prefix('=') { name == x } else { name.contains(f) }) {
            continue;
        }
        let d = if s.game == "factorio" { GameData::for_set("factorio", &ds) } else { GameData::for_game(&s.game) };
        s.time_limit_ms = ms;
        let t0 = std::time::Instant::now();
        let mut first_ms: Option<f64> = None;
        let p = match lp::start(&d, &s) {
            lp::Start::Done(p) => p,
            lp::Start::Running(mut run) => {
                if run.progress().objective.is_some() {
                    first_ms = Some(t0.elapsed().as_secs_f64() * 1e3);
                }
                while !run.step(Some(150.0)) {
                    if first_ms.is_none() && run.progress().objective.is_some() {
                        first_ms = Some(t0.elapsed().as_secs_f64() * 1e3);
                    }
                }
                run.plan()
            }
        };
        let wall = t0.elapsed().as_secs_f64() * 1e3;
        let warn: Vec<String> = p.warnings.iter().map(|w| w.replace('"', "'")).collect();
        println!(
            "{{\"name\":\"{name}\",\"variant\":\"{label}\",\"status\":\"{:?}\",\"obj\":{:.6},\"proven\":{},\"gap\":{},\"lp_bound\":{:.6},\"solve_ms\":{:.1},\"wall_ms\":{:.1},\"buildings\":{},\"columns\":{},\"power\":{:.3},\"first_ms\":{},\"warn\":{:?}}}",
            p.status, p.objective, p.proven_optimal, p.gap.map(|g| format!("{g:.6}")).unwrap_or("null".into()), p.lp_bound, p.solve_ms, wall, p.buildings, p.columns, p.production_power, first_ms.map(|f| format!("{f:.1}")).unwrap_or("null".into()), warn
        );
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }
}
