//! Oracle tests for Factorio quality: numbers from
//! docs/research/factorio-quality/_过程文件/quality_lp.py (scipy/HiGHS) and the report §2.
//! These hold for the vanilla data (quality never goes down).
use super::data::{GameData, Item, Recipe, Resource};
use super::quality::{qrow, split};
use super::*;

fn lp_settings(dataset: &str, target: &str, rate: f64) -> PlanSettings {
    let mut s = PlanSettings {
        game: "factorio".into(),
        dataset: dataset.into(),
        targets: vec![ItemRate { item: target.into(), rate }],
        integer: false,
        objective: Objective::Custom,
        module_quality: 4,
        ..Default::default()
    };
    s.cost = CostWeights { resources: 1.0, buildings: 0.0, shards: 0.0, ..Objective::Resources.preset() };
    s
}

fn add_resource(d: &mut GameData, item: &str, weight: f64) {
    d.resources.retain(|r| r.item != item);
    d.resources.push(Resource {
        item: item.into(),
        limit: None,
        extractor: "test".into(),
        extractor_rate: 60.0,
        extractor_power: 0.0,
        weight: Some(weight),
    });
}

fn keep(d: &mut GameData, ids: &[&str]) {
    d.recipes.retain(|r| ids.contains(&r.id.as_str()));
}

fn raw(p: &Plan, item: &str) -> f64 {
    p.raw.iter().filter(|l| l.item == item).map(|l| l.rate).sum()
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-12)
}

/// Every item balances: production − consumption + raw + supplied = target + surplus.
fn assert_balanced(p: &Plan) {
    let mut net: HashMap<String, f64> = HashMap::new();
    for r in &p.recipes {
        for l in &r.outputs {
            *net.entry(l.item.clone()).or_default() += l.rate;
        }
        for l in &r.inputs {
            *net.entry(l.item.clone()).or_default() -= l.rate;
        }
    }
    for l in p.raw.iter().chain(&p.supplied) {
        *net.entry(l.item.clone()).or_default() += l.rate;
    }
    for l in p.targets.iter().chain(&p.surplus) {
        *net.entry(l.item.clone()).or_default() -= l.rate;
    }
    for (it, v) in net {
        assert!(v.abs() < 1e-4 * (1.0 + v.abs()), "{it} unbalanced by {v}");
    }
}

#[test]
fn quality_matrix_rows() {
    let r = qrow(0.124, 0);
    let want = [0.876, 0.1116, 0.01116, 0.001116, 0.000124];
    for (a, b) in r.iter().zip(want) {
        assert!((a - b).abs() < 1e-12, "{r:?}");
    }
    assert!((qrow(0.248, 3)[4] - 0.248).abs() < 1e-12);
    assert!((qrow(0.3, 2).iter().sum::<f64>() - 1.0).abs() < 1e-12);
    assert_eq!(split("iron-gear-wheel@4"), ("iron-gear-wheel", 4));
    assert_eq!(split("iron-gear-wheel"), ("iron-gear-wheel", 0));
    assert_eq!(split("Desc_OreIron_C"), ("Desc_OreIron_C", 0));
}

#[test]
fn space_age_data_sets() {
    for ds in ["2x1", "2q1", "spa"] {
        let d = GameData::for_set("factorio", ds);
        assert_eq!(d.qualities.len(), 5, "{ds}");
        assert!(d.recipes.iter().filter(|r| r.recycling).count() > 150, "{ds}");
        let rec = d.building("recycler").unwrap();
        assert_eq!(rec.modules, 4);
        assert!(rec.disallowed_effects.iter().any(|e| e == "productivity"));
        let q3 = d.effect("quality-module-3").unwrap();
        let leg = q3.tier(4).quality;
        assert!(leg == if ds == "spa" { 0.062 } else { 0.0625 }, "{ds}: {leg}");
        // scrap recycling is a production recipe, not item recycling
        if let Some(r) = d.recipe("scrap-recycling") {
            assert!(!r.recycling);
        }
    }
    let d = GameData::for_set("factorio", "2x1");
    let em = d.building("electromagnetic-plant").unwrap();
    assert_eq!((em.modules, em.base_productivity), (5, 0.5));
    assert_eq!(d.building("foundry").unwrap().base_productivity, 0.5);
    assert_eq!(d.building("cryogenic-plant").unwrap().modules, 8);
    // 2.1: asteroid reprocessing disallows quality modules
    assert!(!d.recipe("metallic-asteroid-reprocessing").unwrap().quality_allowed);
    assert!(GameData::for_set("factorio", "spa").recipe("metallic-asteroid-reprocessing").unwrap().quality_allowed);
    // the default data set is still base 2.0.77
    assert_eq!(GameData::for_game("factorio").dataset, "2.0");
}

/// Productivity only multiplies the non-catalyst part (Kovarex: 41 U-235 out, 40 catalyst).
#[test]
fn catalyst_is_not_multiplied() {
    let d = GameData::for_game("factorio");
    let k = d.recipe("kovarex-enrichment-process").unwrap();
    let mut s = lp_settings("2.0", "uranium-235", 1.0);
    s.effects = vec!["productivity-module-3".into()];
    s.module_quality = 0;
    let modes = rules::modes(&d, k, &s);
    let m = modes.iter().find(|m| m.key.contains("productivity")).expect("prod mode");
    let outs = rules::outputs(k, m);
    let per = 60.0 / 60.0; // per minute per craft (time 60 s)
    let u235 = outs.iter().find(|o| o.0 == "uranium-235").unwrap().1;
    // centrifuge 2 slots × 10 % = +20 % on the 1 non-catalyst U-235
    assert!((u235 - per * (40.0 + 1.0 * 1.2)).abs() < 1e-9, "{u235}");
}

/// scottmsul/FactorioQualityOptimizer README: 0.5 legendary gear/s from normal plates with
/// AM3, legendary Q3/P3 (2.0 values), recipes `iron-gear-wheel` + its recycling only:
/// 79.8786 normal ingredient sets per legendary gear (159.76 plates).
#[test]
fn oracle_scottmsul_gear() {
    let mut d = GameData::for_set("factorio", "spa");
    keep(&mut d, &["iron-gear-wheel", "iron-gear-wheel-recycling"]);
    add_resource(&mut d, "iron-plate", 1.0);
    let mut s = lp_settings("spa", "iron-gear-wheel@4", 30.0);
    s.disabled_buildings = vec!["assembling-machine-1".into(), "assembling-machine-2".into()];
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert_balanced(&p);
    let sets = raw(&p, "iron-plate") / 2.0 / 30.0;
    assert!(close(sets, 79.8786, 1e-5), "gear: {sets} sets per legendary gear");
    // optimal loadout: 2Q 2P at every tier below legendary, 4P at legendary
    for r in &p.recipes {
        if r.id.starts_with("iron-gear-wheel") && !r.id.contains("recycling") && r.throughput > 1e-6 {
            let want = if r.id.ends_with("@4") { "4P" } else { "2Q 2P" };
            assert!(r.mode_label.ends_with(want), "{} {}", r.id, r.mode_label);
        }
    }
    println!("gear oracle: {sets:.6} sets/legendary, {} columns, {:.1} ms", p.columns, p.solve_ms);
}

/// Same loop with 2.1.25 values (Q3 legendary 6.25 %): report §1.2 gives 78.40.
#[test]
fn gear_with_2_1_values() {
    let mut d = GameData::for_set("factorio", "2q1");
    keep(&mut d, &["iron-gear-wheel", "iron-gear-wheel-recycling"]);
    add_resource(&mut d, "iron-plate", 1.0);
    let mut s = lp_settings("2q1", "iron-gear-wheel@4", 30.0);
    s.disabled_buildings = vec!["assembling-machine-1".into(), "assembling-machine-2".into()];
    let p = plan(&d, &s);
    let sets = raw(&p, "iron-plate") / 2.0 / 30.0;
    assert!((sets - 78.40).abs() < 0.005, "{sets}");
}

/// Pure self-recycling (uranium-238 recycles to itself), 4 × legendary Q3 (24.8 %):
/// 1 legendary per 2726.9 normal (report §2.2, script D).
#[test]
fn oracle_self_recycling() {
    let mut d = GameData::for_set("factorio", "spa");
    keep(&mut d, &["uranium-238-recycling"]);
    add_resource(&mut d, "uranium-238", 1.0);
    let p = plan(&d, &lp_settings("spa", "uranium-238@4", 1.0));
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    let r = raw(&p, "uranium-238");
    assert!((r - 2726.9).abs() < 0.05, "{r}");
}

/// One-ingredient test loop in `building`: part → widget (1:1), widget recycles to 0.25 part.
fn loop_data(building: &str, productivity: bool) -> GameData {
    let mut d = GameData::for_set("factorio", "spa");
    d.recipes.clear();
    for id in ["widget", "part"] {
        d.items.push(Item {
            id: id.into(),
            name: id.into(),
            name_zh: None,
            liquid: false,
            sink: 0.0,
            raw: false,
            group: String::new(),
            row: 0,
            order: 0.0,
        });
    }
    let r = |id: &str, b: &str, i: (&str, f64), o: (&str, f64), rec: bool| Recipe {
        id: id.into(),
        name: id.into(),
        name_zh: None,
        alt: false,
        building: b.into(),
        producers: vec![b.into()],
        productivity_allowed: productivity && !rec,
        time: 1.0,
        inputs: vec![(i.0.into(), i.1)],
        outputs: vec![(o.0.into(), o.1)],
        power: None,
        power_range: None,
        recycling: rec,
        quality_allowed: true,
        catalyst: vec![],
    };
    d.recipes.push(r("widget", building, ("part", 60.0), ("widget", 60.0), false));
    d.recipes.push(r("widget-recycling", "recycler", ("widget", 60.0), ("part", 15.0), true));
    d
}

/// wiki Tutorial:Quality_upcycling_math yield table: legendary products per normal product
/// entering the recycler loop, every integer loadout mixable per tier (script H):
/// EM plant 2.722332 %, cryogenic plant 4.835199 % (= wiki), AM3 0.583017 %,
/// chemical plant 0.341205 %, foundry 1.608046 % (wiki's fractional module counts give a bit more).
#[test]
fn oracle_wiki_yield_table() {
    for (b, want) in [
        ("electromagnetic-plant", 2.722332),
        ("cryogenic-plant", 4.835199),
        ("assembling-machine-3", 0.583017),
        ("chemical-plant", 0.341205),
        ("foundry", 1.608046),
    ] {
        let mut d = loop_data(b, true);
        add_resource(&mut d, "widget", 1.0);
        let p = plan(&d, &lp_settings("spa", "widget@4", 1.0));
        assert_eq!(p.status, Status::Optimal, "{b}: {:?}", p.warnings);
        assert_balanced(&p);
        let y = 100.0 / raw(&p, "widget");
        assert!((y - want).abs() < 2e-6, "{b}: {y:.6} % (want {want})");
    }
}

/// Normal : (uncommon … epic) crafts in the loop (script I): 4 quality modules (recipes that
/// take no productivity) 7.09 : 1; 2Q 2P 5.03 : 1 and 79.88 sets per legendary. Without
/// productivity the legendary stage has none either: 154.7238 sets (the script's 116.05
/// assumes 4 P3 at the legendary stage; same closed form with p = 0 there gives 154.7238).
#[test]
fn oracle_normal_to_higher_ratio() {
    for (prod, ratio, sets) in [(false, 7.0855, 154.7238), (true, 5.03, 79.88)] {
        let mut d = loop_data("assembling-machine-3", prod);
        add_resource(&mut d, "part", 1.0);
        let p = plan(&d, &lp_settings("spa", "widget@4", 1.0));
        let work = |id: &str| p.recipes.iter().filter(|r| r.id == id).map(|r| r.throughput).sum::<f64>();
        let r = work("widget") / (work("widget@1") + work("widget@2") + work("widget@3"));
        let got = raw(&p, "part");
        assert!((r - ratio).abs() < 0.005 && (got - sets).abs() < 0.005, "prod {prod}: ratio {r}, sets {got}");
    }
}

/// LDS shuffle (report §1.6, script J): foundry casting at +300 % (built-in 50 % + 4 legendary
/// P3 + research level 15), recycler 4 × Q3: one legendary plastic per normal plastic.
#[test]
fn oracle_lds_shuffle() {
    let mut d = GameData::for_set("factorio", "spa");
    keep(&mut d, &["casting-low-density-structure", "low-density-structure-recycling"]);
    add_resource(&mut d, "plastic-bar", 1.0);
    for f in ["molten-iron", "molten-copper"] {
        add_resource(&mut d, f, 0.0);
    }
    let mut s = lp_settings("spa", "plastic-bar@4", 60.0);
    s.research_productivity.insert("casting-low-density-structure".into(), 1.5);
    // LDS is not on plastic's ingredient chain: allow its recycling explicitly
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Infeasible);
    s.recycle_allow = vec!["low-density-structure".into()];
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert_balanced(&p);
    let per = raw(&p, "plastic-bar") / 60.0;
    assert!((per - 1.0).abs() < 1e-3, "{per} normal plastic per legendary plastic");
    // research level 5 (+50 %): lossy, 0.048 legendary per plastic (script J)
    s.research_productivity.insert("casting-low-density-structure".into(), 0.5);
    let p = plan(&d, &s);
    let y = 60.0 / raw(&p, "plastic-bar");
    assert!((y - 0.048).abs() < 0.0015, "{y}");
}

/// Recycling scope: by default only the targets' own chain is recycled, so a legendary gear
/// needs gears, plates and ore only; "all" lets every iron-bearing item's recycling in.
#[test]
fn recycling_scope_and_full_data() {
    let d = GameData::for_set("factorio", "2x1");
    let mut s = lp_settings("2x1", "iron-gear-wheel@4", 30.0);
    let p = plan(&d, &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert_balanced(&p);
    let recycled: Vec<&str> = p.recipes.iter().filter(|r| r.id.contains("recycling")).map(|r| r.id.as_str()).collect();
    assert!(recycled.iter().all(|r| r.starts_with("iron-gear-wheel-recycling") || r.starts_with("iron-plate-recycling") || r.starts_with("iron-ore-recycling")), "{recycled:?}");
    let chain_cols = p.columns;
    // all recipes available: never worse than the restricted 79.88 sets (in iron ore)
    let ore = raw(&p, "iron-ore") / 2.0 / 30.0;
    assert!(ore <= 79.8786, "{ore}");
    s.recycling = "all".into();
    let all = plan(&d, &s);
    assert_eq!(all.status, Status::Optimal, "{:?}", all.warnings);
    assert!(all.columns > 3 * chain_cols, "{} vs {}", all.columns, chain_cols);
    assert!(all.objective <= p.objective + 1e-6);
}

/// Normal targets never expand quality; a quality target on base 2.0 data (no tiers) is plain.
#[test]
fn normal_plans_unchanged() {
    let d = GameData::for_set("factorio", "2x1");
    let mut s = lp_settings("2x1", "iron-gear-wheel", 60.0);
    s.integer = true;
    let p = plan(&d, &s);
    assert!(!p.quality && !p.recipes.iter().any(|r| r.id.contains("recycling")));
    let base = GameData::for_game("factorio");
    let p = plan(&base, &lp_settings("2.0", "iron-gear-wheel@4", 60.0));
    assert!(!p.quality && p.status == Status::Optimal);
}

/// Scale (report §5 step 0): model size and native solve time; `cargo test --release -- --ignored --nocapture scale`.
#[test]
#[ignore]
fn quality_scale_report() {
    let d = GameData::for_set("factorio", "2x1");
    for (target, scope) in [
        ("iron-gear-wheel@4", "chain"),
        ("iron-gear-wheel@4", "all"),
        ("processing-unit@4", "chain"),
        ("processing-unit@4", "all"),
        ("quantum-processor@4", "chain"),
    ] {
        let mut s = lp_settings("2x1", target, 60.0);
        s.cost = Objective::Resources.preset();
        s.recycling = scope.into();
        let t = std::time::Instant::now();
        let p = plan(&d, &s);
        println!(
            "{target:22} {scope:5}: {:?} {} columns, {} items, {:.0} ms, objective {:.2}, {} recipes used",
            p.status,
            p.columns,
            p.rows,
            t.elapsed().as_secs_f64() * 1e3,
            p.objective,
            p.recipes.len()
        );
    }
}
