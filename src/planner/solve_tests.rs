//! Whole-building solve: exact model reductions, best plan in hand, stepped solving
//! (docs/research/solver-speed/00-最终报告.md §6 steps 0, 2, 3).
use super::lp::{self, Start, NO_SHRINK};
use super::*;

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
fn dsp(targets: &[(&str, f64)], o: Objective, effects: &[&str]) -> (GameData, PlanSettings) {
    let mut s = st("dsp", targets);
    s.objective = o;
    s.effects = v(effects);
    s.disabled_buildings = v(&["df-negentropy-smelter", "df-recomposing-assembler", "df-self-evolution-lab"]);
    (GameData::for_game("dsp"), s)
}
fn factorio(set: &str, targets: &[(&str, f64)], o: Objective) -> (GameData, PlanSettings) {
    let mut s = st("factorio", targets);
    s.dataset = set.into();
    s.objective = o;
    (GameData::for_set("factorio", set), s)
}
fn sf(targets: &[(&str, f64)], o: Objective, alt: bool) -> (GameData, PlanSettings) {
    let mut s = st("satisfactory", targets);
    s.objective = o;
    s.allow_all_alternates = alt;
    (GameData::default_data(), s)
}
fn solve(d: &GameData, s: &PlanSettings, shrink: bool) -> Plan {
    NO_SHRINK.with(|c| c.set(!shrink));
    let p = plan(d, s);
    NO_SHRINK.with(|c| c.set(false));
    p
}
const SCI6: &[(&str, f64)] = &[
    ("automation-science-pack", 60.0),
    ("logistic-science-pack", 60.0),
    ("military-science-pack", 60.0),
    ("chemical-science-pack", 60.0),
    ("production-science-pack", 60.0),
    ("utility-science-pack", 60.0),
];

/// Dropping dominated tier / mode columns and the power variables of linear-power machines leaves
/// the optimum unchanged: plans that both versions prove optimal have the same objective (within
/// the MIP gap), and LP relaxations agree to round-off.
#[test]
fn model_reductions_keep_the_optimum() {
    let cases = vec![
        ("dsp_um60_plain_res", dsp(&[("universe-matrix", 60.0)], Objective::Resources, &[])),
        ("dsp_um60_p3_res", dsp(&[("universe-matrix", 60.0)], Objective::Resources, &["proliferator-3-speed", "proliferator-3-products"])),
        ("dsp_processor_bld", dsp(&[("processor", 30.0)], Objective::Buildings, &[])),
        ("dsp_processor_cmp", dsp(&[("processor", 30.0)], Objective::Compute, &[])),
        ("fa_sci6_plain_res", factorio("", SCI6, Objective::Resources)),
        ("fa_circuit_pow", factorio("", &[("electronic-circuit", 60.0)], Objective::Power)),
        ("fa_circuit_bld", factorio("", &[("electronic-circuit", 60.0)], Objective::Buildings)),
        ("sf_phase4_std_bld", sf(&[("Assembly Director System", 4.0), ("Magnetic Field Generator", 4.0), ("Thermal Propulsion Rocket", 2.0), ("Nuclear Pasta", 1.0)], Objective::Buildings, false)),
        ("sf_rip10_std_pow", sf(&[("Reinforced Iron Plate", 10.0)], Objective::Power, false)),
    ];
    for (name, (d, mut s)) in cases {
        s.time_limit_ms = 60_000;
        let a = solve(&d, &s, false);
        let b = solve(&d, &s, true);
        assert!(a.proven_optimal && b.proven_optimal, "{name}: both must prove ({} / {})", a.proven_optimal, b.proven_optimal);
        let rel = (a.objective - b.objective).abs() / a.objective.abs().max(1e-9);
        assert!(rel <= 2e-4, "{name}: objective {} without, {} with reductions", a.objective, b.objective);
        assert!((a.lp_bound - b.lp_bound).abs() <= 1e-6 * (1.0 + a.lp_bound.abs()), "{name}: LP bound {} vs {}", a.lp_bound, b.lp_bound);
        // display tie-break: the reduced model does not show more machine power at an equal objective
        if s.objective != Objective::Power && s.objective != Objective::Compute {
            assert!(b.production_power <= a.production_power * (1.0 + 1e-6) + 1e-6 || rel > 1e-9,
                "{name}: {} MW without, {} MW with reductions", a.production_power, b.production_power);
        }
    }
    // fractional plans of the report's two largest LPs (linear power on 2439 / 875 columns)
    let mut lps = vec![dsp(&[("universe-matrix", 60.0), ("small-carrier-rocket", 5.0), ("solar-sail", 60.0), ("antimatter-fuel-rod", 10.0)], Objective::Compute,
        &["proliferator-1-products", "proliferator-2-products", "proliferator-3-products", "proliferator-1-speed", "proliferator-2-speed", "proliferator-3-speed"])];
    let (d, mut s) = factorio("2x1", SCI6, Objective::Power);
    s.effects = v(&["productivity-module-3", "speed-module-3"]);
    s.beacon_module = "speed-module-3".into();
    s.beacon_counts = vec![2, 4, 8, 12];
    lps.push((d, s));
    for (d, mut s) in lps {
        s.integer = false;
        let a = solve(&d, &s, false);
        let b = solve(&d, &s, true);
        assert_eq!(a.status, Status::Optimal);
        assert!((a.objective - b.objective).abs() <= 1e-7 * (1.0 + a.objective.abs()), "LP {} vs {}", a.objective, b.objective);
    }
}

/// DSP without proliferator, Buildings: the machine tiers tie, so dropping the slower ones lets
/// the solver prove the plan (4 s unproven before, report §4.2).
#[test]
fn dominated_tiers_make_dsp_buildings_provable() {
    let (d, mut s) = dsp(&[("universe-matrix", 60.0)], Objective::Buildings, &[]);
    s.time_limit_ms = 20_000;
    let p = plan(&d, &s);
    assert!(p.proven_optimal, "gap {:?}", p.gap);
}

/// Somersloop budget: rounding the LP up breaks the budget; the fallback still gives a plan within
/// it (it used to end in "Time limit reached before a whole-building plan was found").
#[test]
fn somersloop_budget_always_has_a_plan() {
    let (d, mut s) = sf(&[("Assembly Director System", 4.0), ("Magnetic Field Generator", 4.0), ("Thermal Propulsion Rocket", 2.0), ("Nuclear Pasta", 1.0)], Objective::Buildings, true);
    s.auto_somersloops = true;
    s.somersloop_budget = Some(20);
    s.time_limit_ms = 300;
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert!(p.somersloops <= 20.0 + 1e-9, "{} Somersloops", p.somersloops);
    assert!(p.recipes.iter().all(|r| (r.machines - r.machines.round()).abs() < 1e-9));
}

/// Shard budget: shardless rounding fits any budget.
#[test]
fn shard_budget_always_has_a_plan() {
    let (d, mut s) = sf(&[("Assembly Director System", 4.0), ("Magnetic Field Generator", 4.0)], Objective::Buildings, true);
    s.max_clock_percent = 250.0;
    s.shard_budget = Some(3);
    s.time_limit_ms = 300;
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert!(p.shards <= 3.0 + 1e-9, "{} shards", p.shards);
}

/// The largest model of the report (2439 columns, Power): a time limit far below what the MILP
/// needs still returns the rounded plan instead of "no plan".
#[test]
fn short_time_limit_keeps_the_plan_in_hand() {
    let (d, mut s) = factorio("2x1", SCI6, Objective::Power);
    s.effects = v(&["productivity-module-3", "speed-module-3"]);
    s.beacon_module = "speed-module-3".into();
    s.beacon_counts = vec![2, 4, 8, 12];
    s.time_limit_ms = 200;
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert!(!p.proven_optimal && p.gap.is_some());
    assert!(p.warnings.iter().any(|w| w.starts_with("Time limit reached: best")));
}

/// Stepped solve: a plan right after `start`, objectives never get worse, the bound never drops,
/// and `extend` resumes the same search after the deadline.
#[test]
fn stepped_solve_streams_improving_plans() {
    let (d, mut s) = sf(&[("Heavy Modular Frame", 7.3)], Objective::Buildings, true);
    s.time_limit_ms = 600;
    let Start::Running(mut run) = lp::start(&d, &s) else { panic!("whole-building solve expected") };
    let first = run.plan();
    assert_eq!(first.status, Status::Optimal);
    assert!(first.warnings.iter().all(|w| !w.starts_with("Time limit")));
    let (mut obj, mut bound) = (run.progress().objective.unwrap(), run.progress().bound);
    let mut steps = 0;
    while !run.step(Some(50.0)) {
        let pr = run.progress();
        assert!(pr.objective.unwrap() <= obj + 1e-9 && pr.bound >= bound - 1e-9);
        (obj, bound) = (pr.objective.unwrap(), pr.bound);
        steps += 1;
        assert!(steps < 10_000);
    }
    assert!(steps > 3, "{steps} steps");
    let done = run.plan();
    assert!((done.objective - obj).abs() < 1e-9);
    if !done.proven_optimal {
        let nodes = run.progress().nodes;
        run.extend(300.0);
        assert!(!run.progress().done);
        while !run.step(Some(50.0)) {}
        assert!(run.progress().objective.unwrap() <= obj + 1e-9);
        assert!(run.progress().nodes > nodes, "extend continues the same tree");
    }
}

/// External-engine hooks (HiGHS in the browser): the LP text names every column, the best plan's
/// values pass the model's own row check and come back unchanged, a worse or broken vector is refused,
/// and a proven offer ends the run.
#[test]
fn external_engine_round_trip() {
    let (d, mut s) = sf(&[("Heavy Modular Frame", 7.3)], Objective::Buildings, true);
    s.time_limit_ms = 300;
    let Start::Running(mut run) = lp::start(&d, &s) else { panic!() };
    let lp = run.model_lp();
    let vals = run.best_values();
    assert!(!vals.is_empty());
    assert!(lp.contains(&format!(" v{}", vals.len() - 1)) && lp.starts_with("Minimize") && lp.contains("General"));
    let (obj, version) = (run.progress().objective.unwrap(), run.progress().version);
    run.offer_values(&vals, None, false);
    assert_eq!(run.progress().version, version, "same plan is no improvement");
    // breaking one balance: take the work of the first used column away
    let mut bad = vals.clone();
    if let Some(i) = bad.iter().position(|v| *v > 1.0) {
        bad[i] = 0.0;
    }
    run.offer_values(&bad, None, false);
    assert!((run.progress().objective.unwrap() - obj).abs() < 1e-9);
    run.offer_values(&[], Some(obj), true);
    assert!(run.progress().done && run.progress().proven);
    assert!(run.plan().proven_optimal);
}
