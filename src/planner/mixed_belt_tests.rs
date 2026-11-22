//! Mixed-belt planner tests: the asserts of `docs/research/dsp-mixed-belt/_过程文件/proto_mixed_belt.py`
//! (笑笑's worked cases, report §2.3–2.5) plus the plan-level checks of report §4 P1 / P4.
use super::mixed_belt::*;
use super::*;

const C: f64 = 30.0;

fn eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn none() -> TwoOverride {
    TwoOverride::default()
}

// ---- prototype `simulate` with his layouts as inputs: arithmetic agrees with his sheets

#[test]
fn his_layouts_give_his_minima() {
    // ep19 Mk.II proliferator: Y 3, D 1.5; unit 3:2; Y after 3, then D after 2, Y after 4 [BV1fXkxYDEbW 34:05]
    let (m, _, _) = run_two(C, [3.0, 1.5], [18.0, 12.0], 0, 3, [4, 2], 30);
    assert!(eq(m[0], 4.5) && eq(m[1], 4.5), "{m:?}");
    // ep19 carbon nanotube: G 0.75, T 0.25; unit 4:1; G after 8, T after 8, G after 24 [62:49]
    let (m, _, _) = run_two(C, [0.75, 0.25], [24.0, 6.0], 0, 8, [24, 8], 72);
    assert!(eq(m[0], 2.0) && eq(m[1], 2.0), "{m:?}");
    // ep19 Mk.III proliferator: P 0.75, N 0.375; unit 18:12; P after k, N after 10, P after 20 [76:44]
    for (k, want) in [(16, [1.5, 2.25]), (17, [1.875, 1.875]), (18, [2.25, 1.5])] {
        let (m, _, _) = run_two(C, [0.75, 0.375], [18.0, 12.0], 0, k, [20, 10], k + 60);
        assert!(eq(m[0], want[0]) && eq(m[1], want[1]), "k={k} {m:?}");
    }
    // ep20 turbine: M 0.75, L 0.75; unit 3:2; M after 1, L after 10, M after 10 [BV1iQrKYoEoy 26:46]
    let (m, _, _) = run_two(C, [0.75, 0.75], [18.0, 12.0], 0, 1, [10, 10], 61);
    assert!(eq(m[0], 3.75) && eq(m[1], 3.75), "{m:?}");
}

#[test]
fn closed_form_matches_the_cases() {
    assert!(eq(closed_sum(C, 6.0, 3.0, 1.5), 9.0));
    assert!(eq(closed_sum(C, 32.0, 0.75, 0.25), 4.0));
    assert!(eq(closed_sum(C, 30.0, 0.75, 0.375), 3.75));
    assert!(eq(closed_sum(C, 20.0, 0.75, 0.75), 7.5));
    // 9-period Mk.II proliferator: S = −1.5, "不行" [BV1iQrKYoEoy 01:44]
    assert!(eq(closed_sum(C, 9.0, 3.0, 1.5), -1.5));
    // processors (not used to fit anything) [BV1iQrKYoEoy 56:00–56:40]: per belt position 1 + 1
    assert!(eq(closed_sum(C, 20.0, 1.0, 1.0), 0.0));
    assert!(eq(closed_sum(C, 18.0, 1.0, 1.0), 3.0));
    assert!(eq(closed_sum(C, 14.0, 1.0, 1.0), 9.0));
}

#[test]
fn largest_period_by_the_necessary_condition() {
    // prototype max_p: largest P with S ≥ 2m and P·max(a, b) ≤ C
    let maxp = |a: f64, b: f64| choose_period(C, [a, b], 1.5, 1, None, None).2.last().unwrap().period;
    assert_eq!([maxp(3.0, 1.5), maxp(0.75, 0.25), maxp(0.75, 0.375), maxp(0.75, 0.75)], [6, 32, 30, 24]);
    assert_eq!(ratio(3.0, 1.5), (2, 1, 0.0));
    assert_eq!(ratio(0.75, 0.25), (3, 1, 0.0));
}

// ---- return flows at the control unit (computed, compared with what he reads)

#[test]
fn return_flows_and_unit_kind() {
    // carbon tubes: unit 16 machines after a Ti refill = machine 32: G 8 / T 6 [BV1fXkxYDEbW 67:28–67:39]
    let (_, _, e) = run_two(C, [0.75, 0.25], [24.0, 6.0], 0, 8, [24, 8], 32);
    assert!(eq(e[0], 8.0) && eq(e[1], 6.0), "{e:?}");
    assert!(e[0] + e[1] > 12.0);
    assert_eq!(unit_kind(e, 0).0, "double"); // he built the two-green-belt unit [67:42]
    // turbines, 60 machines = 3 periods: limited item back at exactly its 2/5 [BV1iQrKYoEoy 28:05]
    let (_, _, e) = run_two(C, [0.75, 0.75], [18.0, 12.0], 0, 1, [10, 10], 60);
    assert!(eq(e[0], 4.5) && eq(e[1], 12.0), "{e:?}");
    assert_eq!(unit_kind(e, 0).0, "double");
    // Mk.II proliferator, 30 machines: Y 13.5, D 12, total 25.5 > 12 [BV1fXkxYDEbW 79:46]
    let (_, _, e) = run_two(C, [3.0, 1.5], [18.0, 12.0], 0, 3, [4, 2], 30);
    assert!(eq(e[0], 13.5) && eq(e[1], 12.0), "{e:?}");
    assert_eq!(unit_kind(e, 0).0, "double");
    assert_eq!(unit_kind([5.0, 6.0], 0).0, "standard");
    assert_eq!(unit_kind([25.0, 6.0], 0).0, "unknown");
    assert_eq!(unit_kind([10.0, 13.0], 0).0, "unknown");
}

// ---- the (σ, first, k) search

#[test]
fn search_reaches_half_of_s() {
    for (a, b, n) in [(3.0, 1.5, [4, 2]), (0.75, 0.25, [24, 8]), (0.75, 0.375, [20, 10]), (0.75, 0.75, [10, 10])] {
        let p = (n[0] + n[1]) as f64;
        let best = best_for(C, [a, b], n, 1, &none()).unwrap();
        assert!(eq(best.min, closed_sum(C, p, a, b) / 2.0), "{a} {b} {best:?}");
    }
    // Mk.III proliferator, one belt for two rows: even positions only → 1.5 (k 16 or 18) [BV1fXkxYDEbW 77:08]
    let b = best_for(C, [0.75, 0.375], [20, 10], 2, &none()).unwrap();
    assert!(eq(b.min, 1.5), "{b:?}");
    assert!(b.k % 2 == 0);
    // single-row turbine P = 24 reaches its S/2 = 1.5
    assert!(eq(best_for(C, [0.75, 0.75], [12, 12], 1, &none()).unwrap().min, 1.5));
    // 9-period Mk.II proliferator is infeasible whatever k and σ [BV1iQrKYoEoy 01:44]
    assert!(best_for(C, [3.0, 1.5], [6, 3], 1, &none()).unwrap().min < 0.0);
    // odd intervals with two rows: no layout
    assert!(best_for(C, [0.75, 0.375], [2, 1], 2, &none()).is_none());
}

#[test]
fn period_choice_matches_his_lines() {
    let pick = |a: f64, b: f64, r: Option<u32>| {
        let (_, _, c, ch) = choose_period(C, [a, b], 1.5, 1, r, None);
        c[ch.unwrap().0].period
    };
    assert_eq!(pick(3.0, 1.5, Some(15)), 6); // Mk.II proliferator
    assert_eq!(pick(0.75, 0.25, None), 32); // carbon tubes
    assert_eq!(pick(0.75, 0.375, Some(15)), 30); // Mk.III proliferator
    assert_eq!(pick(0.75, 0.75, Some(15)), 20); // turbines: 24 would need a 120-machine unit
    assert_eq!(pick(0.75, 0.75, None), 24);
    // processors, per machine 0.5 + 0.5 (he: interval 7, 8, 9 positions all fine, 10 exactly zero).
    // One row: P = 36 at 1.5 / 1.5. Two rows per belt (his line): only even positions, so interval
    // 9 positions (P = 36) reaches only 1 / 2 and the choice is interval 8 (P = 32), inside his 7–9.
    // (Report §2.3 (b) gives 1.5 / 1.5 for interval 9; that holds without the even-position rule.)
    let (_, _, c, ch) = choose_period(C, [0.5, 0.5], 1.5, 1, None, None);
    let (i, b) = ch.unwrap();
    assert_eq!(c[i].period, 36);
    assert!(eq(b.min, 1.5));
    assert!(eq(best_for(C, [0.5, 0.5], [18, 18], 2, &none()).unwrap().min, 1.0));
    let (_, _, c, ch) = choose_period(C, [0.5, 0.5], 1.5, 2, None, None);
    assert_eq!(c[ch.unwrap().0].period, 32);
}

// ---- ladder groups (scheme B)

#[test]
fn ladder_groups_match_his_lines() {
    let opts = feed_options(2, 2);
    let rho = 1.0 / 12.0;
    // electric motor 1.5 / 0.75 / 0.75 → 10 [BV1iQrKYoEoy 18:32]
    let (n, f) = ladder(&[1.5, 0.75, 0.75], C, &opts, rho, 12, 24).unwrap();
    assert_eq!(n, 10);
    assert_eq!(f.iter().map(|x| x.rate).collect::<Vec<_>>(), vec![15.0, 7.5, 7.5]);
    // particle container 0.375 × 3 → 24 at 9.75 each, also without the cap [40:12–43:26]
    for cap in [24, 40] {
        let (n, f) = ladder(&[0.375; 3], C, &opts, rho, 12, cap).unwrap();
        assert_eq!(n, 24);
        assert!(f.iter().all(|x| eq(x.rate, 9.75)), "{f:?}");
    }
    // titanium alloy 1 / 0.5 / 0.5: belts only → 12 (12/6/6, as he built it); belt + sorters → 13
    let belts: Vec<(f64, String)> = opts.iter().filter(|o| [6.0, 12.0, 30.0].contains(&o.0)).cloned().collect();
    let (n, f) = ladder(&[1.0, 0.5, 0.5], C, &belts, rho, 12, 40).unwrap();
    assert_eq!(n, 12);
    assert_eq!(f.iter().map(|x| x.rate).collect::<Vec<_>>(), vec![12.0, 6.0, 6.0]);
    assert_eq!(ladder(&[1.0, 0.5, 0.5], C, &opts, rho, 12, 40).unwrap().0, 13);
    // 9.75 = belt 6 + 3 + 0.75 [BV1iQrKYoEoy 40:57]
    assert!(opts.iter().any(|o| eq(o.0, 9.75)));
}

// ---- simulator

#[test]
fn simulator_agrees_with_the_line_and_reports_blocking() {
    let ls = BeltLayoutSettings::default();
    let (d, _) = design_two(C, [3.0, 1.5], ["Y", "D"], 30, &ls, &none()).unwrap();
    let t = simulate(&two_events(&d, C, true), C, 2);
    assert!(t.converged);
    for i in 0..2 {
        assert!(eq(t.mins[i], d.line_mins[i]), "{:?} {:?}", t.mins, d.line_mins);
        assert!(eq(t.returns[i], d.returns[i]));
    }
    // a return over the unit's share is reported, naming the item
    let ev = vec![
        Event::ControlUnit { shares: vec![6.0, 24.0], jump: 1 },
        Event::Machine { use_: vec![1.0, 1.0] },
        Event::FillFeed { item: 0, cap: C },
    ];
    let t = simulate(&ev, C, 2);
    assert!(eq(t.returns[0], 7.0));
    assert!(t.violations.iter().any(|v| v.contains("item 0") && v.contains("backs up")), "{:?}", t.violations);
    // a ladder ring settles with every feed delivered in full
    let opts = feed_options(2, 2);
    let (n, feeds) = ladder(&[1.5, 0.75, 0.75], C, &opts, 1.0 / 12.0, 12, 24).unwrap();
    let ld = LadderDesign { group: n, groups: 1, last_group: n, rho: 0.0, total: 30.0, feeds, c: vec![1.5, 0.75, 0.75], recover_sorters: vec![] };
    let t = simulate(&ladder_events(&ld, false), C, 3);
    assert!(t.converged && t.violations.is_empty(), "{:?}", t.violations);
    assert!(t.mins.iter().all(|m| *m >= -1e-9), "{:?}", t.mins);
}

#[test]
fn design_two_overrides_and_returns() {
    let ls = BeltLayoutSettings::default();
    // 9-period override: flagged below the margin
    let (d, notes) = design_two(C, [3.0, 1.5], ["Y", "D"], 30, &ls, &TwoOverride { period: Some(9), ..none() }).unwrap();
    assert_eq!(d.period, 9);
    assert!(d.best < 0.0 && notes.iter().any(|n| n.level == "warn"));
    assert_eq!(d.recommended.unwrap().0, 6);
    // his carbon layout as an override: 32 machines → returns 8 / 6, two-green-belt unit
    let ov = TwoOverride { period: Some(32), first: Some(0), k: Some(8), sigma5: Some(4) };
    let (d, _) = design_two(C, [0.75, 0.25], ["G", "T"], 32, &ls, &ov).unwrap();
    assert!(eq(d.returns[0], 8.0) && eq(d.returns[1], 6.0));
    assert_eq!(d.unit, "double");
    assert!(eq(d.best, 2.0));
    // a line that ends right after a refill sends one item back over its share: flagged, with the
    // number of machines that completes the period
    let bad = (24..=35).find_map(|n| {
        let (d, notes) = design_two(C, [3.0, 1.5], ["Y", "D"], n, &ls, &none()).unwrap();
        notes.iter().any(|x| x.level == "fail" && x.text.contains("backs up")).then_some(d)
    });
    let d = bad.expect("some line length ends over a share");
    assert!(d.complete_add > 0);
    let (_, notes) = design_two(C, [3.0, 1.5], ["Y", "D"], d.machines_per_unit + d.complete_add, &ls, &none()).unwrap();
    assert!(!notes.iter().any(|x| x.text.contains("backs up")));
}

// ---- plan level (report §4 P1, P4)

fn dsp_settings(effects: &[&str]) -> (GameData, PlanSettings) {
    let d = GameData::for_game("dsp");
    let s = PlanSettings { game: "dsp".into(), effects: effects.iter().map(|e| e.to_string()).collect(), ..Default::default() };
    (d, s)
}

fn one_recipe(id: &str, mode: &str, machines: f64) -> Plan {
    Plan {
        recipes: vec![PlanRecipe {
            id: id.into(),
            mode: mode.into(),
            mode_label: String::new(),
            name: id.into(),
            alt: false,
            building: mode.split('|').next().unwrap().into(),
            machines,
            machines_ceil: machines.ceil(),
            throughput: machines,
            shards: 0.0,
            groups: vec![],
            power: 0.0,
            inputs: vec![],
            outputs: vec![],
            somersloops: 0,
            compute: 0.0,
            beacons: 0.0,
        }],
        ..Default::default()
    }
}

#[test]
fn sprayed_two_ingredient_recipe_is_scheme_a() {
    // Mk.II proliferator, recomposing ×3, Mk.III speed-up: diamond 1.5 / Mk.I 3 stacks/s; the
    // proliferator spent on it is not a third ingredient (report §2.1)
    let (d, mut s) = dsp_settings(&["proliferator-3-speed"]);
    s.belt_layout.row_len = Some(15);
    let rep = mixed_belt::layout_for_plan(&one_recipe("proliferator-2", "df-recomposing-assembler|proliferator-3-speed", 30.0), &d, &s);
    let l = &rep.recipes[0];
    assert_eq!(l.scheme, "A");
    assert_eq!(l.inputs.len(), 2);
    let t = l.two.as_ref().unwrap();
    assert!(eq(t.c[0], 1.5) && eq(t.c[1], 3.0), "{:?}", t.c);
    assert_eq!(t.period, 6);
    assert!(eq(t.best, 4.5));
    assert_eq!(t.unit, "double", "{:?}", l.notes);
    assert!(l.coaters.is_some());
    assert_eq!(l.sorters.input, "blue"); // 18 items/s per machine, blue 6 × 4 = 24 > 18
}

#[test]
fn plan_layouts_for_his_recipes() {
    let (d, mut s) = dsp_settings(&["proliferator-3-speed", "proliferator-3-products"]);
    s.belt_layout.row_len = Some(15);
    // turbines, extra products: 0.75 / 0.75, P = 20, 60 machines → returns 4.5 / 12, double unit;
    // output belt (1.875 items/s each) full at 16 [BV1iQrKYoEoy 29:15]
    let rep = mixed_belt::layout_for_plan(&one_recipe("electromagnetic-turbine", "df-recomposing-assembler|proliferator-3-products", 60.0), &d, &s);
    let l = &rep.recipes[0];
    let t = l.two.as_ref().unwrap();
    assert_eq!(t.period, 20);
    assert!(eq(t.best, 3.75));
    assert_eq!(l.sorters.outputs[0].full_at, 16);
    assert_eq!(t.unit, "double");
    // among the equally good layouts the one with the smaller return is taken: his 4.5 / 12 [BV1iQrKYoEoy 28:05]
    let mut r = t.returns;
    r.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(eq(r[0], 4.5) && eq(r[1], 12.0), "{:?}", t.returns);
    // carbon nanotube, quantum chemical plant, speed-up: 0.75 / 0.25 → P = 32 (no row length)
    s.belt_layout.row_len = None;
    let rep = mixed_belt::layout_for_plan(&one_recipe("carbon-nanotube", "quantum-chemical-plant|proliferator-3-speed", 32.0), &d, &s);
    let t = rep.recipes[0].two.as_ref().unwrap();
    assert_eq!(t.period, 32);
    assert!(eq(t.best, 2.0));
    // electric motor: three ingredients → ladder groups of 10
    let rep = mixed_belt::layout_for_plan(&one_recipe("electric-motor", "df-recomposing-assembler|proliferator-3-speed", 25.0), &d, &s);
    let l = &rep.recipes[0];
    assert_eq!(l.scheme, "B");
    let g = l.ladder.as_ref().unwrap();
    assert_eq!((g.group, g.groups, g.last_group), (10, 3, 5));
    // a single-ingredient recipe is not a mixed belt
    let rep = mixed_belt::layout_for_plan(&one_recipe("proliferator-1", "df-recomposing-assembler", 4.0), &d, &s);
    assert!(rep.recipes.is_empty() && rep.single.len() == 1);
}

#[test]
fn mk2_proliferator_180_machines_sorter_count() {
    // 笑笑: 180 Mk.II proliferator machines take < 400 sorters mixed (180 + 211), 540 separate [BV1fXkxYDEbW 54:53]
    let (d, s) = dsp_settings(&["proliferator-3-speed"]);
    let rep = mixed_belt::layout_for_plan(&one_recipe("proliferator-2", "df-recomposing-assembler|proliferator-3-speed", 180.0), &d, &s);
    let so = &rep.recipes[0].sorters;
    assert_eq!(so.separate, 540);
    assert!((360..=400).contains(&so.mixed), "{}", so.mixed);
}

#[test]
fn solved_plan_lays_out() {
    // end to end: a solved DSP plan with spraying; every ≥ 2-ingredient recipe gets a layout
    let (d, mut s) = dsp_settings(&["proliferator-3-speed"]);
    s.targets = vec![ItemRate { item: "proliferator-2".into(), rate: 600.0 }];
    s.objective = Objective::Buildings;
    s.time_limit_ms = 2000;
    let plan = super::plan(&d, &s);
    assert_eq!(plan.status, Status::Optimal);
    let rep = mixed_belt::layout_for_plan(&plan, &d, &s);
    let multi = plan.recipes.iter().filter(|r| d.recipe(&r.id).is_some_and(|x| x.inputs.len() >= 2)).count();
    assert_eq!(rep.recipes.len(), multi);
    assert!(rep.recipes.iter().all(|l| l.scheme == "A" || l.scheme == "B" || l.status == "fail"));
    let json = serde_json::to_string(&rep).unwrap();
    assert!(json.len() < 2_000_000);
}
