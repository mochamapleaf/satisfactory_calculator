use satisfactory_calculator::planner::lp::{Tune, TRACE, TUNE};
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut filter = String::new();
    let mut tn = Tune::default();
    let mut ms = 4000u64;
    let mut label = String::from("base");
    let mut traj = false;
    let mut scale = 1.0f64;
    for a in &args {
        let (k, val) = a.split_once('=').unwrap_or((a.as_str(), "1"));
        match k {
            "f" => filter = val.into(),
            "ms" => ms = val.parse().unwrap(),
            "gap" => tn.gap = val.parse().unwrap(),
            "nosub" => tn.skip_sub = true,
            "nohint" => tn.no_hint = true,
            "nopresolve" => tn.presolve = false,
            "slice" => tn.slice_ms = val.parse().unwrap(),
            "dominated" => tn.dominated = true,
            "tighten" => tn.tighten = true,
            "lattice" => tn.lattice = true,
            "scont" => tn.s_cont = true,
            "linpow" => tn.linpow = true,
            "h1" => tn.h1 = true,
            "hintint" => tn.hint_int = true,
            "ls" => tn.ls = val.parse().unwrap(),
            "subfrac" => tn.subfrac = val.parse().unwrap(),
            "dump" => tn.dump = Some(val.into()),
            "label" => label = val.into(),
            "traj" => traj = true,
            "scale" => scale = val.parse().unwrap(),
            _ => panic!("unknown arg {a}"),
        }
    }
    let filters: Vec<&str> = filter.split(',').collect();
    for (name, ds, mut s) in scenarios() {
        if !filter.is_empty()
            && !filters.iter().any(|f| if let Some(x) = f.strip_prefix('=') { name == x } else { name.contains(f) })
        {
            continue;
        }
        let d = if s.game == "factorio" { GameData::for_set("factorio", &ds) } else { GameData::for_game(&s.game) };
        s.time_limit_ms = ms;
        for t in &mut s.targets {
            t.rate *= scale;
        }
        let mut t2 = tn.clone();
        if let Some(p) = &tn.dump {
            t2.dump = Some(format!("{p}/{name}.lp"));
        }
        TUNE.with(|t| *t.borrow_mut() = t2);
        TRACE.with(|t| *t.borrow_mut() = Default::default());
        let p = plan(&d, &s);
        let tr = TRACE.with(|t| t.borrow().clone());
        let gap = if tr.obj != 0.0 { (tr.obj - tr.bound) / tr.obj.abs() } else { 0.0 };
        println!(
            "{{\"name\":\"{name}\",\"variant\":\"{label}\",\"status\":\"{:?}\",\"units\":{},\"dropped\":{},\"items\":{},\"vars\":{},\"int\":{},\"rows\":{},\"t_lp\":{:.1},\"lp_obj\":{:.6},\"ceil_obj\":{:.6},\"t_support\":{:.1},\"support\":{},\"t_sub\":{:.1},\"sub_obj\":\"{:.6}\",\"sub_nodes\":{},\"t_end\":{:.1},\"obj\":{:.6},\"bound\":\"{:.6}\",\"gap\":\"{:.6}\",\"nodes\":{},\"lp_iters\":{},\"proven\":{},\"buildings\":{},\"tightened\":{},\"ls_obj\":\"{:.6}\",\"t_ls\":{:.0},\"ls_evals\":{},\"ls_moves\":{},\"warn\":{}}}",
            p.status, tr.units, tr.units_dropped, p.rows, tr.vars, tr.int_vars, tr.rows, tr.t_lp, tr.lp_obj, tr.ceil_obj, tr.t_support, tr.support,
            tr.t_sub, tr.sub_obj, tr.sub_nodes, tr.t_end, tr.obj, tr.bound, gap, tr.nodes, tr.lp_iters, tr.proven, p.buildings, tr.tightened, tr.ls_obj, tr.t_ls, tr.ls_evals, tr.ls_moves, p.warnings.len()
        );
        if traj {
            for (t, inc, b, n) in &tr.traj {
                println!("  traj {t:.0} {inc:.6} {b:.6} {n}");
            }
        }
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }
}
