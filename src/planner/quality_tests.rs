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

/// Recycling scope on full 2x1 data, "chain" (default): only the targets' own chain is
/// recycled, so a legendary gear recycles gears, plates and ore only.
#[test]
fn recycling_scope_chain_full_data() {
    let d = GameData::for_set("factorio", "2x1");
    let p = plan(&d, &lp_settings("2x1", "iron-gear-wheel@4", 30.0));
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert_balanced(&p);
    let recycled: Vec<&str> = p.recipes.iter().filter(|r| r.id.contains("recycling")).map(|r| r.id.as_str()).collect();
    assert!(!recycled.is_empty());
    assert!(recycled.iter().all(|r| r.starts_with("iron-gear-wheel-recycling") || r.starts_with("iron-plate-recycling") || r.starts_with("iron-ore-recycling")), "{recycled:?}");
}

/// Scope semantics on a small iron subset: "chain" = the target's chain, "all" = every
/// recycling recipe, "list" = only `recycle_allow`; and "all" never plans worse than "chain".
#[test]
fn recycling_scopes() {
    let mut d = GameData::for_set("factorio", "2x1");
    keep(
        &mut d,
        &["iron-gear-wheel", "iron-gear-wheel-recycling", "pipe", "pipe-recycling", "iron-chest", "iron-chest-recycling"],
    );
    add_resource(&mut d, "iron-plate", 1.0);
    let mut s = lp_settings("2x1", "iron-gear-wheel@4", 30.0);
    s.disabled_buildings = vec!["assembling-machine-1".into(), "assembling-machine-2".into()];
    let allowed: Vec<&Recipe> = d.recipes.iter().collect();
    let targets: std::collections::BTreeSet<String> = ["iron-gear-wheel".to_string()].into();
    let ids = |s: &PlanSettings| {
        let mut v: Vec<String> = quality::recycling_allowed(s, &allowed, &targets).iter().map(|r| r.id.clone()).collect();
        v.sort();
        v
    };
    assert_eq!(ids(&s), vec!["iron-gear-wheel-recycling"]);
    let chain = plan(&d, &s);
    assert_eq!(chain.status, Status::Optimal, "{:?}", chain.warnings);
    s.recycling = "all".into();
    assert_eq!(ids(&s), vec!["iron-chest-recycling", "iron-gear-wheel-recycling", "pipe-recycling"]);
    let all = plan(&d, &s);
    assert_eq!(all.status, Status::Optimal, "{:?}", all.warnings);
    assert_balanced(&all);
    assert!(all.columns > chain.columns, "{} vs {}", all.columns, chain.columns);
    assert!(all.objective <= chain.objective + 1e-6);
    s.recycling = "list".into();
    s.recycle_allow = vec!["pipe".into()];
    assert_eq!(ids(&s), vec!["pipe-recycling"]);
    // chain scope plus an extra allowed item
    s.recycling = "chain".into();
    assert_eq!(ids(&s), vec!["iron-gear-wheel-recycling", "pipe-recycling"]);
}

/// The "all" scope on full data (16 578 columns, minutes in debug):
/// `cargo test --release -- --ignored all_scope_full_data`.
#[test]
#[ignore]
fn all_scope_full_data() {
    let d = GameData::for_set("factorio", "2x1");
    let mut s = lp_settings("2x1", "iron-gear-wheel@4", 30.0);
    let p = plan(&d, &s);
    s.recycling = "all".into();
    let all = plan(&d, &s);
    assert_eq!(all.status, Status::Optimal, "{:?}", all.warnings);
    assert!(all.columns > 3 * p.columns, "{} vs {}", all.columns, p.columns);
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

/// Built-in machine productivity applies to every recipe the machine runs, also those that
/// forbid productivity modules (spec §3.3, §5 step 2): EM plant / foundry +50 %, research and
/// modules only where allowed.
#[test]
fn builtin_productivity_on_all_recipes() {
    let d = GameData::for_set("factorio", "2x1");
    let mut s = lp_settings("2x1", "transport-belt", 120.0);
    s.effects = vec!["productivity-module-3".into()];
    s.research_productivity.insert("transport-belt".into(), 0.3);
    s.research_productivity.insert("electronic-circuit".into(), 0.2);
    let mult = |rid: &str, key: &str| {
        let r = d.recipe(rid).unwrap();
        rules::modes(&d, r, &s).iter().find(|m| m.key == key).map(|m| m.out_mult)
    };
    // EM plant empty mode on an allowed recipe: 1 + 0.5 built-in + 0.2 research
    assert!((mult("electronic-circuit", "electromagnetic-plant").unwrap() - 1.7).abs() < 1e-9);
    // productivity-disallowed recipes: built-in only, no research, no productivity-module mode
    assert!((mult("transport-belt", "foundry").unwrap() - 1.5).abs() < 1e-9);
    assert!((mult("productivity-module", "electromagnetic-plant").unwrap() - 1.5).abs() < 1e-9);
    assert!((mult("transport-belt", "assembling-machine-3").unwrap() - 1.0).abs() < 1e-9);
    assert!(mult("transport-belt", "foundry|productivity-module-3").is_none());

    // plan: 120 belts/min from the foundry need 40 plates + 40 gears, not 60 + 60
    let mut d2 = d.clone();
    keep(&mut d2, &["transport-belt"]);
    add_resource(&mut d2, "iron-plate", 1.0);
    add_resource(&mut d2, "iron-gear-wheel", 1.0);
    let mut s2 = lp_settings("2x1", "transport-belt", 120.0);
    s2.disabled_buildings = (1..=3).map(|k| format!("assembling-machine-{k}")).collect();
    let p = plan(&d2, &s2);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert!((raw(&p, "iron-plate") - 40.0).abs() < 1e-6, "{}", raw(&p, "iron-plate"));

    // quality path: the plain and quality loadouts in the foundry keep the +50 %
    let mut sq = lp_settings("2x1", "transport-belt@4", 10.0);
    sq.quality_module = "quality-module-3".into();
    sq.prod_module = "productivity-module-3".into();
    sq.research_productivity.insert("transport-belt".into(), 0.3);
    let recipes = vec![d.recipe("transport-belt").unwrap()];
    let ex = quality::expand(&d, &sq, &recipes, &["transport-belt@4".to_string()]);
    let (_, modes) = ex.iter().find(|(r, _)| r.id == "transport-belt").expect("normal tier");
    for m in modes.iter().filter(|m| m.building == "foundry") {
        assert!((m.out_mult - 1.5).abs() < 1e-9, "{} {}", m.key, m.out_mult);
        assert!(!m.key.contains('p') || m.key.ends_with("p0"), "no prod modules: {}", m.key);
    }
    assert!(modes.iter().any(|m| m.key == "foundry|q4p0"));
}
