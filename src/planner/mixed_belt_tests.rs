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
    // (a manual period: the search itself avoids such a layout, see `return_flow_is_part_of_the_choice`)
    let p6 = TwoOverride { period: Some(6), ..none() };
    let bad = (24..=35).find_map(|n| {
        let (d, notes) = design_two(C, [3.0, 1.5], ["Y", "D"], n, &ls, &p6).unwrap();
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
            group_machines: 0.0,
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

// ---- review 5 (docs/review/fable-review-5.md)

/// H1: a fractional, negative, huge or null number in the tab's settings must not make the
/// settings (and with them the solve) unreadable.
#[test]
fn belt_layout_settings_are_read_leniently() {
    let j = r#"{"game":"dsp","integer":false,"belt_layout":{"row_len":7.5,"ladder_short":-1,"ladder_cap":1e12,"margin":null,
        "belt":9,"stack":0,"rows":2.2,"unit_cap":"x","unit_max":0,"hint_machines":1.5,"junk":[1],
        "overrides":{"a|b":{"k":1e12,"sigma5":7,"first":0.5,"period":-3},"c|d":{"k":2.6,"sigma5":3},"e":5}}}"#;
    let s: PlanSettings = serde_json::from_str(j).expect("settings stay readable");
    assert!(!s.integer);
    let b = &s.belt_layout;
    assert_eq!((b.row_len, b.ladder_short, b.ladder_cap, b.belt, b.stack, b.rows), (Some(8), 0, 1000, 2, 1, 2));
    assert_eq!((b.unit_cap, b.unit_max, b.hint_machines), (None, None, 2));
    assert!(eq(b.margin, 1.5));
    assert_eq!(b.overrides.len(), 2);
    assert_eq!(b.overrides["a|b"], TwoOverride { k: Some(1_000_000), ..none() });
    assert_eq!(b.overrides["c|d"], TwoOverride { k: Some(3), sigma5: Some(3), ..none() });
    // not an object at all, and a normal round trip
    let s: PlanSettings = serde_json::from_str(r#"{"game":"dsp","belt_layout":7}"#).unwrap();
    assert_eq!(s.belt_layout, BeltLayoutSettings::default());
    let mut want = BeltLayoutSettings { rows: 2, row_len: Some(15), margin: 0.75, ..Default::default() };
    want.overrides.insert("x|y".into(), TwoOverride { period: Some(30), first: Some(1), k: Some(18), sigma5: Some(2) });
    let json = serde_json::to_string(&PlanSettings { belt_layout: want.clone(), ..Default::default() }).unwrap();
    assert_eq!(serde_json::from_str::<PlanSettings>(&json).unwrap().belt_layout, want);
}

/// The closed-form minima and the shortened line equal the plain simulation.
#[test]
fn closed_form_minima_and_equivalent_line_match_the_simulation() {
    let mut checked = 0;
    for (c, base) in [([3.0, 1.5], [2u32, 1u32]), ([0.75, 0.25], [3, 1]), ([0.75, 0.375], [2, 1]), ([0.75, 0.75], [1, 1]), ([0.5, 1.25], [2, 5]), ([0.019, 0.019], [1, 1])] {
        for t in [1u32, 2, 3, 5, 8] {
            let n = [t * base[0], t * base[1]];
            let period = n[0] + n[1];
            for s5 in 1..=4u8 {
                let start = [C * s5 as f64 / 5.0, C * (5 - s5) as f64 / 5.0];
                for first in 0..2usize {
                    for k in [1, 2, period / 2 + 1, period, period + 3] {
                        let (sim, _, _) = run_two(C, c, start, first, k, n, k + 2 * period);
                        let cf = steady_mins(C, c, start, first, k, n);
                        assert!(eq(sim[0], cf[0]) && eq(sim[1], cf[1]), "{c:?} {n:?} s{s5} f{first} k{k}: {sim:?} {cf:?}");
                        for machines in [k + 2 * period + 1, 7 * period + 3, 1000] {
                            let short = equivalent_line(c, k, n, machines, 2);
                            assert!(short <= machines && short <= k + 3 * period);
                            let (m1, _, e1) = run_two(C, c, start, first, k, n, machines);
                            let (m2, _, e2) = run_two(C, c, start, first, k, n, short);
                            assert!((0..2).all(|i| (m1[i] - m2[i]).abs() < 1e-7 && (e1[i] - e2[i]).abs() < 1e-7), "{c:?} {n:?} k{k} {machines}→{short}: {e1:?} {e2:?}");
                            checked += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(checked > 3000);
    // an approximated ratio is not shortened and is simulated
    assert_eq!(equivalent_line([0.7, 0.31], 3, [2, 1], 500, 2), 500);
}

/// M2: 笑笑's processor line [BV1iQrKYoEoy 56:00–56:40], 1 + 1 stacks/s per belt position. Positions
/// are whole, so the two minima move in steps of 1 and S/2 is reached only when it is whole:
/// interval 9 gives 1 / 2 (not the 1.5 / 1.5 the report had), 8 gives 3 / 3, 7 gives 4 / 5, 10 gives 0.
#[test]
fn processor_intervals_in_whole_positions() {
    for (n, want, sum) in [(7u32, [4.0, 5.0], 9.0), (8, [3.0, 3.0], 6.0), (9, [1.0, 2.0], 3.0), (10, [0.0, 0.0], 0.0)] {
        let b = best_for(C, [1.0, 1.0], [n, n], 1, &none()).unwrap();
        let mut m = b.mins;
        m.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(eq(m[0], want[0]) && eq(m[1], want[1]), "interval {n}: {b:?}");
        assert!(eq(closed_sum(C, 2.0 * n as f64, 1.0, 1.0), sum));
        // the same line per machine (0.5 + 0.5, two rows per belt, even positions)
        let b2 = best_for(C, [0.5, 0.5], [2 * n, 2 * n], 2, &none()).unwrap();
        assert!(eq(b2.min, want[0]), "interval {n} per machine: {b2:?}");
    }
    // one machine per position at 0.5 + 0.5 (one row) does reach 1.5 / 1.5 at P = 36
    assert!(eq(best_for(C, [0.5, 0.5], [18, 18], 1, &none()).unwrap().min, 1.5));
}

/// M1: the return flow of the actual line is part of the choice (report §2.3 condition 4, step 6).
#[test]
fn return_flow_is_part_of_the_choice() {
    let ls = BeltLayoutSettings::default();
    // Mk.II proliferator (3 / 1.5), 27 machines: the layout with the best steady state (P = 6,
    // k = 3, 4.5 / 4.5) ends 3 machines into a period and sends Y back at 22.5 > 18. Another
    // P = 6 layout (k = 4, lowest flow 3) returns 9 / 7.5 and is taken.
    let (d, notes) = design_two(C, [3.0, 1.5], ["Y", "D"], 27, &ls, &none()).unwrap();
    assert!(!notes.iter().any(|n| n.level == "fail"), "{notes:?}");
    assert_eq!(d.period, 6);
    assert!(eq(d.best, 3.0) && eq(d.returns[0], 9.0) && eq(d.returns[1], 7.5), "{d:?}");
    // 29 machines: no P = 6 layout passes; P = 3 does (returns 16.5 / 9 within 18 / 12)
    let (d, notes) = design_two(C, [3.0, 1.5], ["Y", "D"], 29, &ls, &none()).unwrap();
    assert!(!notes.iter().any(|n| n.level == "fail"), "{notes:?}");
    assert_eq!(d.period, 3);
    assert!(d.returns[0] <= d.shares[0] + 1e-9 && d.returns[1] <= d.shares[1] + 1e-9);
    assert!(d.line_mins[0].min(d.line_mins[1]) >= 1.5);
    assert!(notes.iter().any(|n| n.level == "info" && n.text.contains("instead of the largest")));
    let c6 = d.candidates.iter().find(|c| c.period == 6).unwrap();
    assert!(!c6.feasible && c6.reason.contains("return flow"));
    // the manual P = 6, k = 3 on 27 machines still shows the failure
    let (_, notes) = design_two(C, [3.0, 1.5], ["Y", "D"], 27, &ls, &TwoOverride { period: Some(6), k: Some(3), first: Some(0), sigma5: Some(3) }).unwrap();
    assert!(notes.iter().any(|n| n.level == "fail" && n.text.contains("backs up")));
    // every line length gets a layout that passes; whole periods keep the largest period
    for machines in 1..=120u32 {
        let (d, notes) = design_two(C, [3.0, 1.5], ["Y", "D"], machines, &ls, &none()).unwrap();
        assert!(!notes.iter().any(|n| n.level == "fail"), "{machines}: {notes:?}");
        if machines % 6 == 0 {
            assert_eq!(d.period, 6, "{machines}");
            assert!(eq(d.best, 4.5));
        }
    }
    // his lines are whole periods: unchanged
    for (c, machines, r, period, best) in [([3.0, 1.5], 30, Some(15), 6, 4.5), ([0.75, 0.25], 32, None, 32, 2.0), ([0.75, 0.75], 60, Some(15), 20, 3.75)] {
        let l = BeltLayoutSettings { row_len: r, ..Default::default() };
        let (d, _) = design_two(C, c, ["A", "B"], machines, &l, &none()).unwrap();
        assert_eq!(d.period, period);
        assert!(eq(d.best, best));
    }
    // the recommendation is the layout shown (it used to be the layout before the tie-break)
    let l = BeltLayoutSettings { row_len: Some(15), ..Default::default() };
    let (d, _) = design_two(C, [0.75, 0.75], ["M", "L"], 60, &l, &none()).unwrap();
    assert_eq!(d.recommended, Some((d.period, d.sigma5, d.first, d.k, d.best)));
    let mut r = d.returns;
    r.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(eq(r[0], 4.5) && eq(r[1], 12.0));
    // with σ fixed by hand the recommendation stays the searched one
    let (d2, _) = design_two(C, [0.75, 0.75], ["M", "L"], 60, &l, &TwoOverride { sigma5: Some(4), ..none() }).unwrap();
    assert!(d2.overridden && d2.sigma5 == 4);
    assert_eq!(d2.recommended, d.recommended);
}

/// M5: very long lines stay cheap: an error past LINE_MAX, a shortened diagram, no overflow.
#[test]
fn long_lines_are_bounded() {
    let ls = BeltLayoutSettings::default();
    let t = std::time::Instant::now();
    let err = design_two(C, [0.75, 0.375], ["A", "B"], 3_000_000, &ls, &none()).unwrap_err();
    assert!(err.contains("unit cap"), "{err}");
    let (d, mut s) = dsp_settings(&["proliferator-3-products"]);
    // one ring through many stations: the line is not cut into one module per station
    s.belt_layout.station_ring = true;
    // 12 000 Mk.III proliferator machines on one unit: the diagram is the equivalent short line
    let rep = mixed_belt::layout_for_plan(&one_recipe("proliferator-3", "df-recomposing-assembler|proliferator-3-products", 12_011.0), &d, &s);
    let l = &rep.recipes[0];
    let two = l.two.as_ref().unwrap();
    assert_eq!(two.machines_per_unit, 12_011);
    assert!(l.trace_omitted > 0 && l.trace_omitted % two.period == 0);
    let shown = l.trace.points.iter().filter(|p| p.kind == "machine").count() as u32;
    assert_eq!(shown + l.trace_omitted, 12_011);
    assert!(shown <= mixed_belt::TRACE_MAX + 3 * two.period);
    for i in 0..2 {
        assert!(eq(l.trace.returns[i], two.returns[i]) && eq(l.trace.mins[i], two.line_mins[i]), "{:?} {:?}", l.trace.returns, two.returns);
    }
    // absurd counts: no overflow, a clear failure for scheme A and plain groups for scheme B
    let rep = mixed_belt::layout_for_plan(&one_recipe("proliferator-3", "df-recomposing-assembler|proliferator-3-products", 4e12), &d, &s);
    assert_eq!(rep.recipes[0].status, "fail");
    assert!(rep.recipes[0].notes.iter().any(|n| n.text.contains("unit cap")));
    let rep = mixed_belt::layout_for_plan(&one_recipe("electric-motor", "df-recomposing-assembler|proliferator-3-products", 4e12), &d, &s);
    assert_eq!(rep.recipes[0].scheme, "B");
    // with a unit cap the long line is split and laid out
    s.belt_layout.unit_cap = Some(300);
    let rep = mixed_belt::layout_for_plan(&one_recipe("proliferator-3", "df-recomposing-assembler|proliferator-3-products", 100_000.0), &d, &s);
    let two = rep.recipes[0].two.as_ref().unwrap();
    assert!(two.machines_per_unit <= 300 && two.units as u64 * two.machines_per_unit as u64 >= 100_000);
    // coaters: per unit one on every refill branch and two on the unit's feeds (was counted for one unit)
    assert_eq!(rep.recipes[0].coaters.as_ref().unwrap().coaters, two.units * (two.feeds.len() as u32 + 2));
    // a slow recipe (long periods) and an absurd ladder cap are searched quickly
    let (dd, _) = design_two(C, [0.005, 0.005], ["A", "B"], 500, &ls, &none()).unwrap();
    assert!(dd.period >= 1000);
    assert_eq!(ladder(&[0.375; 3], C, &feed_options(2, 2), 1.0 / 12.0, 12, u32::MAX).unwrap().0, 24);
    assert!(t.elapsed().as_secs_f64() < 20.0, "{:?}", t.elapsed());
}

/// L: repeat unit with two rows, other belts, odd positions by hand, fractional plans, ladder minima.
#[test]
fn review5_smaller_fixes() {
    // one pass of a belt that feeds two rows of R is 2R machines: P = 6, R = 16 repeats every 96
    // machines, over the default cap 4R = 64; with one row lcm(6, 16) = 48 is fine
    let (_, _, c, _) = choose_period(C, [3.0, 1.5], 1.5, 2, Some(16), None);
    let c6 = c.iter().find(|x| x.period == 6).unwrap();
    assert_eq!((c6.unit, c6.reason.as_str()), (96, "repeat unit too large"));
    let (_, _, c, ch) = choose_period(C, [3.0, 1.5], 1.5, 1, Some(16), None);
    assert_eq!(c[ch.unwrap().0].period, 6);
    // his two-row lines at R = 15 keep their periods: Mk.III proliferator 30, turbines 20 (24 → 120)
    let pick = |a: f64, b: f64| {
        let (_, _, c, ch) = choose_period(C, [a, b], 1.5, 2, Some(15), None);
        c[ch.unwrap().0].period
    };
    assert_eq!((pick(0.75, 0.375), pick(0.75, 0.75)), (30, 20));
    // a green mixed belt: the blue-belt control unit is flagged as an extrapolation
    let (d, mut s) = dsp_settings(&["proliferator-3-products"]);
    s.belt_layout.belt = 1;
    let rep = mixed_belt::layout_for_plan(&one_recipe("electromagnetic-turbine", "df-recomposing-assembler|proliferator-3-products", 8.0), &d, &s);
    assert!(rep.recipes[0].notes.iter().any(|n| n.level == "warn" && n.text.contains("blue belt")), "{:?}", rep.recipes[0].notes);
    assert_ne!(rep.recipes[0].status, "ok");
    s.belt_layout.belt = 2;
    let rep = mixed_belt::layout_for_plan(&one_recipe("electromagnetic-turbine", "df-recomposing-assembler|proliferator-3-products", 8.0), &d, &s);
    assert!(!rep.recipes[0].notes.iter().any(|n| n.text.contains("blue belt")));
    // two rows per belt and an odd k typed by hand: said, not silently laid out
    let ls = BeltLayoutSettings { rows: 2, ..Default::default() };
    let (dd, notes) = design_two(C, [0.75, 0.375], ["P", "N"], 132, &ls, &TwoOverride { k: Some(17), ..none() }).unwrap();
    assert_eq!(dd.k, 17);
    assert!(notes.iter().any(|n| n.level == "warn" && n.text.contains("even number")), "{notes:?}");
    let (dd, notes) = design_two(C, [0.75, 0.375], ["P", "N"], 132, &ls, &none()).unwrap();
    assert!(dd.k % 2 == 0 && !notes.iter().any(|n| n.text.contains("even number")));
    // a continuous plan's 2.4 machines are 3 machines, whatever the integer switch says
    s.integer = true;
    let rep = mixed_belt::layout_for_plan(&one_recipe("electromagnetic-turbine", "df-recomposing-assembler|proliferator-3-products", 2.4), &d, &s);
    assert_eq!(rep.recipes[0].machines, 3);
    let rep = mixed_belt::layout_for_plan(&one_recipe("electromagnetic-turbine", "df-recomposing-assembler|proliferator-3-products", 0.3), &d, &s);
    assert_eq!(rep.recipes[0].machines, 1);
    // ladder: the lowest flow is what the last machine leaves (feed − N·c), not the empty belt before the feeds
    let rep = mixed_belt::layout_for_plan(&one_recipe("particle-container", "df-recomposing-assembler|proliferator-3-products", 24.0), &d, &s);
    let l = &rep.recipes[0];
    assert_eq!(l.ladder.as_ref().unwrap().group, 24);
    assert!(l.trace.mins.iter().all(|m| eq(*m, 9.75 - 24.0 * 0.375)), "{:?}", l.trace.mins);
}

/// Every DSP recipe with two or more ingredients, in every mode, on several belts: a layout or a
/// stated reason, finite numbers only (a NaN would reach the page as null).
#[test]
fn every_dsp_recipe_lays_out_with_finite_numbers() {
    fn nulls(v: &serde_json::Value, path: &str, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Null => {
                if !["two", "ladder", "coaters", "item", "best", "recommended", "stations"].iter().any(|k| path.ends_with(k)) {
                    out.push(path.to_string())
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| nulls(x, path, out)),
            serde_json::Value::Object(o) => o.iter().for_each(|(k, x)| nulls(x, &format!("{path}.{k}"), out)),
            _ => {}
        }
    }
    let d = GameData::for_game("dsp");
    let mut s = PlanSettings { game: "dsp".into(), effects: d.effects.iter().map(|e| e.id.clone()).collect(), ..Default::default() };
    let (mut layouts, mut a_total, mut a_fail_blue) = (0, 0, 0);
    for r in d.recipes.iter().filter(|r| r.inputs.len() >= 2) {
        let modes = rules::modes(&d, r, &s);
        for m in modes.iter().step_by(3) {
            for (belt, stack, rows, rl) in [(2u8, 4u8, 1u8, None), (2, 4, 2, Some(15)), (1, 3, 1, None), (0, 1, 2, Some(7))] {
                s.belt_layout = BeltLayoutSettings { belt, stack, rows, row_len: rl, ..Default::default() };
                for mach in [1.0, 7.0, 61.0, 1000.0] {
                    let mut p = one_recipe(&r.id, &m.key, mach);
                    p.recipes[0].building = m.building.clone();
                    let rep = mixed_belt::layout_for_plan(&p, &d, &s);
                    let l = &rep.recipes[0];
                    assert!(l.scheme == "A" && r.inputs.len() == 2 || l.scheme == "B" && r.inputs.len() > 2 || l.scheme == "none" && l.status == "fail", "{} {}", r.id, l.scheme);
                    assert!(l.status != "fail" || !l.notes.is_empty());
                    let mut out = vec![];
                    nulls(&serde_json::to_value(&rep).unwrap(), "", &mut out);
                    assert!(out.is_empty(), "{} {} belt {belt}: {out:?}", r.id, m.key);
                    layouts += 1;
                    if l.scheme == "A" && belt == 2 {
                        a_total += 1;
                        a_fail_blue += (l.status == "fail") as u32;
                    }
                }
            }
        }
    }
    assert!(layouts > 2000, "{layouts}");
    println!("layouts {layouts}; scheme A on a blue belt: {a_fail_blue} of {a_total} fail");
}
fn check_stations(l: &mixed_belt::RecipeLayout, slots: usize) -> &crate::planner::mixed_station::StationPlan {
    let st = l.stations.as_ref().expect("stations");
    let stack = 4.0;
    for x in &st.stations {
        assert!(x.ports.len() <= slots, "{} slots", x.ports.len());
        assert!(x.items <= x.limit * crate::planner::mixed_station::THROUGHPUT_TOL + 1e-6, "{} > {}", x.items, x.limit);
        for p in &x.ports {
            assert!(p.items <= 30.0 * stack + 1e-6, "belt over capacity: {p:?}");
        }
        // what the station's input belts carry = what its machines use + what comes back (refills
        // and the control unit replace exactly what was taken; ladder surplus returns by recovery)
        let n = (x.last + 1 - x.first) as f64;
        let use_: f64 = l.inputs.iter().map(|r| r.items).sum::<f64>() * n;
        let fed: f64 = x.ports.iter().filter(|p| p.dir == "in" && p.role != "proliferator").map(|p| p.items).sum();
        let back: f64 = x.ports.iter().filter(|p| p.role == "recovery").map(|p| p.items).sum();
        if l.ladder.is_some() {
            assert!((fed - back - use_).abs() < 1e-6 * fed.max(1.0) || x.last < l.machines, "fed {fed} back {back} use {use_}");
        }
        // every product belt carries its machines' output
        for o in &l.outputs {
            let out: f64 = x.ports.iter().filter(|p| p.role == "product" && p.item == o.item).map(|p| p.items).sum();
            assert!((out - o.items * n).abs() < 1e-6 * out.max(1.0));
        }
    }
    // every machine once, every refill on a slot that carries its item
    let total: u32 = st.stations.iter().map(|x| x.last + 1 - x.first).sum();
    assert_eq!(st.machines.len() as u32, total);
    for (k, m) in st.machines.iter().enumerate() {
        assert_eq!(m.n, st.machines[0].n + k as u32);
        assert_eq!(m.outputs.len(), l.outputs.len());
    }
    for r in &st.refills {
        assert!(!r.slots.is_empty());
        let x = &st.stations[r.station as usize - 1];
        for sl in &r.slots {
            assert_eq!(x.ports[*sl as usize - 1].item, r.item);
        }
    }
    if let Some(t) = &l.two {
        let fed: f64 = t.feeds.iter().filter(|f| f.pos <= total).map(|f| f.amount).sum();
        let laid: f64 = st.refills.iter().map(|r| r.amount).sum();
        assert!((fed - laid).abs() < 1e-9);
    }
    st
}

#[test]
fn stations_for_his_mk2_line() {
    // 笑笑: 180 Mk.II machines on one ring through 6 planetary stations of 30, 8–9 belts each,
    // 4.5 belts of supply each [BV1fXkxYDEbW 50:08, 50:54, 51:40, 51:56]
    let (d, mut s) = dsp_settings(&["proliferator-3-speed"]);
    s.belt_layout.row_len = Some(15);
    s.belt_layout.station = "pls".into();
    s.belt_layout.station_slots = 12;
    s.belt_layout.station_ring = true;
    let plan = one_recipe("proliferator-2", "df-recomposing-assembler|proliferator-3-speed", 180.0);
    let rep = mixed_belt::layout_for_plan(&plan, &d, &s);
    let l = &rep.recipes[0];
    let st = check_stations(l, 12);
    for x in &st.stations {
        eprintln!("station {}: {}..{} ({}), {} slots, {:.0}/{:.0} items/s", x.index, x.first, x.last, x.last + 1 - x.first, x.ports.len(), x.items, x.limit);
    }
    for q in &st.sequence {
        eprintln!("{q}");
    }
    assert_eq!(st.machines.len(), 180);
    // the supply limit (4.5 belts) decides as for him: 30 machines per station, 6 stations
    assert!(st.stations.iter().all(|x| x.last + 1 - x.first <= 30), "{:?}", st.stations.iter().map(|x| x.last + 1 - x.first).collect::<Vec<_>>());
    assert!((6..=7).contains(&st.stations.len()), "{}", st.stations.len());
    assert_eq!(st.machines.iter().filter(|m| m.input == "pile").count() as u32, l.sorters.end_pile);
    // output piles: once the single-stack belt upstream holds full_at machines
    let full = l.sorters.outputs[0].full_at;
    for x in &st.stations {
        for p in x.ports.iter().filter(|p| p.role == "product") {
            for m in st.machines.iter().filter(|m| p.serves.contains(&m.n)) {
                let up = p.serves.iter().filter(|&&q| q >= m.n).count() as u32;
                assert_eq!(m.outputs[0].1 == "pile", up + 1 > full, "#{} up {up} full {full}", m.n);
            }
        }
    }
    // default: one module (own control unit) per station
    s.belt_layout.station_ring = false;
    let rep = mixed_belt::layout_for_plan(&plan, &d, &s);
    let l = &rep.recipes[0];
    let st = check_stations(l, 12);
    let t = l.two.as_ref().unwrap();
    eprintln!("modules: {} of {} machines, {} stations", t.units, t.machines_per_unit, st.stations_total);
    assert_eq!(st.stations.len(), 1);
    assert!(t.units >= 6 && st.stations_total == t.units);
    assert!(st.stations[0].ports.iter().any(|p| p.role == "control unit"));
}

#[test]
fn stations_for_ladder_groups() {
    let (d, mut s) = dsp_settings(&["proliferator-3-speed"]);
    s.belt_layout.station_slots = 9;
    let rep = mixed_belt::layout_for_plan(&one_recipe("electric-motor", "df-recomposing-assembler|proliferator-3-speed", 25.0), &d, &s);
    let l = &rep.recipes[0];
    assert_eq!(l.scheme, "B");
    let st = check_stations(l, 9);
    let g = l.ladder.as_ref().unwrap();
    for q in &st.sequence {
        eprintln!("{q}");
    }
    // 3 groups (10, 10, 5): 3 feeds + 3 recoveries + products per group; all 25 machines placed
    assert_eq!(st.machines.len(), 25);
    assert_eq!(st.machines.last().unwrap().n, 25);
    assert!(st.stations.iter().all(|x| x.ports.len() <= 9));
    // the last machines of every group take pile sorters
    let piles: Vec<u32> = st.machines.iter().filter(|m| m.input == "pile").map(|m| m.n).collect();
    assert_eq!(piles, vec![8, 9, 10, 18, 19, 20, 23, 24, 25], "{piles:?}");
    // the short last group gives more back
    let back = |x: &crate::planner::mixed_station::Station, at: u32| x.ports.iter().filter(|p| p.role == "recovery" && p.serves == vec![at]).map(|p| p.items).sum::<f64>();
    let all: Vec<&crate::planner::mixed_station::Station> = st.stations.iter().collect();
    let b10: f64 = all.iter().map(|x| back(x, 10)).sum();
    let b25: f64 = all.iter().map(|x| back(x, 25)).sum();
    assert!(b25 > b10, "{b25} {b10}");
    assert_eq!(g.groups, 3);
}
