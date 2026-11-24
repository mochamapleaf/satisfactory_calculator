//! Tests for `dsp_galaxy.rs` against the numbers of `docs/research/dsp-seed/00-最终报告.md`.
use super::dsp_galaxy::*;
use super::data::GameData;
use super::{ItemRate, Status};
use std::sync::OnceLock;

fn galaxy() -> &'static Galaxy {
    static G: OnceLock<Galaxy> = OnceLock::new();
    G.get_or_init(|| generate(&SeedSettings::default()))
}
fn dsp() -> &'static GameData {
    static D: OnceLock<GameData> = OnceLock::new();
    D.get_or_init(|| GameData::for_game("dsp"))
}
fn total(item: &str) -> f64 {
    galaxy().totals.iter().find(|l| l.item == item).map(|l| l.stock).unwrap_or(0.0)
}

#[test]
fn s_of_matches_report_table() {
    // report §4.1 table and 笑笑's S = 815448 (BV1sQBtYxEsm 21:47)
    for (l, v) in [(6, 815_449.0), (19, 649_350.0), (20, 630_964.0), (25, 538_567.0), (40, 302_425.0), (49, 204_069.0),
        (60, 122_368.0), (77, 53_022.0), (81, 43_286.0), (85, 35_270.0), (94, 22_111.0), (98, 17_923.0), (100, 16_128.0)]
    {
        assert!((s_of(l) - v).abs() < 1.0, "S({l}) = {} vs {v}", s_of(l));
    }
    // brute-force sum agrees with the closed form
    let brute: f64 = (81..6000).map(|n| 4000.0 * (n - 5) as f64 * 0.94f64.powi(n - 1)).sum();
    assert!((brute - s_of(81)).abs() < 1e-6 * brute);
    // cumulative cubes 2000(L−6)(L−5): L77 10.22 M, L81 11.40 M, L98 17.11 M
    assert_eq!((cubes_to_reach(77, 1.0) / 1e4).round(), 1022.0);
    assert_eq!((cubes_to_reach(81, 1.0) / 1e4).round(), 1140.0);
    assert_eq!((cubes_to_reach(98, 1.0) / 1e4).round(), 1711.0);
    assert!((speed_at(81) - 9.0).abs() < 1e-12);
}

#[test]
fn threshold_levels_match_report_and_xiaoxiao() {
    // 笑笑: 142 k magnets, 3.28 per cube → "after level 80" = researching 81 (report §4.2)
    assert_eq!(threshold_level(3.28, 142_000.0, 1.0, 1.0), Some(81));
    // report §4.3(3), B = 0.182 M: 10/cube → L98 / L94, 3.28 → L77 / L72
    let b = total("unipolar-magnet");
    assert!((b / 1e3 - 181.6).abs() < 0.5, "magnet stock {b}");
    assert_eq!(threshold_level(10.0, b, 1.0, 1.0), Some(98));
    assert_eq!(threshold_level(10.0, b, RHO_SPRAYED, 1.0), Some(94));
    assert_eq!(threshold_level(3.28, b, 1.0, 1.0), Some(77));
    assert_eq!(threshold_level(3.28, b, RHO_SPRAYED, 1.0), Some(72));
    // crude oil, fixed recipes A / F / E (report §4.3 table, B = 10.12 M upper bound)
    assert_eq!(threshold_level(15.5, 10.12e6, 1.0, 1.0), Some(19));
    assert_eq!(threshold_level(18.3, 10.12e6, 1.0, 1.0), Some(25));
    assert_eq!(threshold_level(32.5, 10.12e6, 1.0, 1.0), Some(40));
    assert_eq!(threshold_level(0.0, 0.0, 1.0, 1.0), Some(6));
    assert_eq!(threshold_level(1.0, 0.0, 1.0, 1.0), None);
}

#[test]
fn galaxy_totals_match_prototype() {
    let g = galaxy();
    assert_eq!(g.stars.len(), 32);
    assert!(g.rare_tier);
    assert_eq!(g.gas_giants, 15);
    assert!((g.oil_rate - 404.8).abs() < 0.1);
    assert!(g.seeps > 0);
    // oil stock = seep points above the 2500 floor: below the report's 10.12 M upper bound
    let oil = total("crude-oil");
    assert!(oil < 10.12e6 && oil > 10.12e6 - 2500.0 * g.seeps as f64 - 1.0, "oil {oil}");
    for (item, m) in [("iron-ore", 127.8), ("copper-ore", 129.6), ("titanium-ore", 114.2), ("stone", 65.6), ("silicon-ore", 53.3),
        ("fire-ice", 24.6), ("coal", 17.8), ("kimberlite-ore", 7.07), ("optical-grating-crystal", 5.39), ("fractal-silicon", 3.09),
        ("spiniform-stalagmite-crystal", 2.76), ("organic-crystal", 1.78)]
    {
        let rel = (total(item) / 1e6 - m).abs() / m;
        assert!(rel < 0.01, "{item}: {} M vs {m} M", total(item) / 1e6);
    }
    // magnets only on the neutron star and the black hole (report §2.3)
    let holders: Vec<&str> = g
        .stars
        .iter()
        .filter(|s| s.planets.iter().any(|p| p.veins.iter().any(|v| v.item == "unipolar-magnet")))
        .map(|s| s.star_type.as_str())
        .collect();
    assert_eq!(holders.len(), 2);
    assert!(holders.contains(&"NeutronStar") && holders.contains(&"BlackHole"));
    // gas totals (report §2.3): hydrogen 8.48, deuterium 0.40, fire ice 4.21
    let gt = |i: &str| g.gas_totals.iter().find(|x| x.item == i).unwrap().base;
    assert!((gt("hydrogen") - 8.48).abs() < 0.01 && (gt("deuterium") - 0.40).abs() < 0.01 && (gt("fire-ice") - 4.21).abs() < 0.01);
}

#[test]
fn collector_net_matches_report_l81() {
    // report §5.5: 15 giants × 40 collectors at speed ×9 → H 22 860/s, D 1080/s, fire ice 11 094/s
    let mut sum = std::collections::BTreeMap::<String, f64>::new();
    for p in galaxy().stars.iter().flat_map(|s| &s.planets).filter(|p| !p.gases.is_empty()) {
        for (i, r) in collector_net(&p.gases, speed_at(81)) {
            *sum.entry(i).or_default() += r * 40.0;
        }
    }
    for (i, v) in [("hydrogen", 22_860.0), ("deuterium", 1_080.0), ("fire-ice", 11_094.0)] {
        assert!((sum[i] - v).abs() / v < 0.005, "{i}: {} vs {v}", sum[i]);
    }
}

#[test]
fn threshold_report_on_seed() {
    let mut ts = ThresholdSettings::default();
    let r = thresholds(dsp(), galaxy(), &ts);
    let mag = r.rows.iter().find(|x| x.item == "unipolar-magnet").unwrap();
    // unconstrained fewest-building plan: 10 magnets per cube, release L98 / L94 (report §4.3(3))
    assert!((mag.per_cube - 10.0).abs() < 1e-6, "{}", mag.per_cube);
    assert_eq!(mag.level, [Some(98), Some(94)]);
    // stock-constrained LP is feasible from L6 for both ρ (report §4.3(2))
    assert_eq!(r.feasible_from, [Some(6), Some(6)]);
    let oil = r.raw_at_feasible.iter().find(|x| x.item == "crude-oil").map(|x| x.rate).unwrap_or(0.0);
    assert!((oil - 2.0).abs() < 1e-6, "oil per cube at L6 {oil}");
    // Mk.III extra products: 3.28 magnets per cube → L77 / L72
    ts.plan.effects = vec!["proliferator-3-products".into()];
    let r = thresholds(dsp(), galaxy(), &ts);
    let mag = r.rows.iter().find(|x| x.item == "unipolar-magnet").unwrap();
    assert!((mag.per_cube - 3.28).abs() < 0.01, "{}", mag.per_cube);
    assert_eq!(mag.level, [Some(77), Some(72)]);
}

#[test]
fn dark_fog_drops_lower_the_magnet_threshold() {
    let mut ts = ThresholdSettings::default();
    ts.rate = 1000.0;
    ts.dark_fog = DarkFog {
        enabled: true,
        level: 24,
        bases: vec![(0, 10.0)],
        // hypothetical measurement: 50 magnets / min per base → 500/min = 0.5 per cube at 1000/min
        drops: vec![ItemRate { item: "unipolar-magnet".into(), rate: 50.0 }],
        measured_at_level: 81,
    };
    ts.level = 81;
    let r = thresholds(dsp(), galaxy(), &ts);
    let mag = r.rows.iter().find(|x| x.item == "unipolar-magnet").unwrap();
    assert!((mag.per_cube - 9.5).abs() < 1e-6, "{}", mag.per_cube);
    assert!(mag.level[0].unwrap() <= 98);
    // below Dark Fog level 24 magnets do not drop
    ts.dark_fog.level = 20;
    let r = thresholds(dsp(), galaxy(), &ts);
    assert!(r.warnings.iter().any(|w| w.contains("黑雾 24 级")));
    let mag = r.rows.iter().find(|x| x.item == "unipolar-magnet").unwrap();
    assert!((mag.per_cube - 10.0).abs() < 1e-6);
}

fn gp() -> GalaxyPlanSettings {
    GalaxyPlanSettings::default()
}

#[test]
fn auto_systems_cover_every_resource() {
    let g = galaxy();
    let s = auto_systems(g);
    assert_eq!(s[0], 0);
    assert!(s.len() <= 8);
    let has = |item: &str| s.iter().any(|i| g.stars[*i].planets.iter().any(|p| p.veins.iter().any(|v| v.item == item)));
    assert!(has("unipolar-magnet") && has("spiniform-stalagmite-crystal") && has("crude-oil"));
}

#[test]
fn galaxy_plan_target_1000_at_l81() {
    let t = web_time::Instant::now();
    let mut s = gp();
    s.objective = "photons".into();
    let p = plan_galaxy(dsp(), galaxy(), &s);
    eprintln!("target 1000: {:?} {:.0} ms, cols {} rows {}, bld {:.0} (prod {:.0}), photons/cube {:.3}, ships {}, warpers {:.2}/min",
        p.status, t.elapsed().as_secs_f64() * 1e3, p.columns, p.rows, p.buildings, p.production_buildings,
        p.photons_per_min / p.z, p.shipments.len(), p.warpers_per_min);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert!((p.z - 1000.0).abs() < 1e-6);
    // report §5.5: 1 photon per cube without proliferator, 20 GW of receivers at z = 1000
    assert!((p.photons_per_min / p.z - 1.0).abs() < 0.01);
    assert!((p.receiver_gw - 20.0).abs() < 0.3);
    // magnets within the quota of the chosen systems (≤ galaxy quota 182 k / S(81) = 4.2 per cube)
    let mag = p.rares.iter().find(|r| r.item == "unipolar-magnet").unwrap();
    assert!(mag.per_cube <= mag.quota_per_cube + 1e-6);
    assert!(mag.quota_per_cube > 0.0 && mag.quota_per_cube <= total("unipolar-magnet") / s_of(81) + 1e-6);
    // with every magnet star active the quota is the galaxy's (report §4.3 table: 4.20 at L81)
    let mut s2 = s.clone();
    let mut sys = auto_systems(galaxy());
    for st in &galaxy().stars {
        if st.planets.iter().any(|p| p.veins.iter().any(|v| v.item == "unipolar-magnet")) && !sys.contains(&st.index) {
            sys.push(st.index);
        }
    }
    s2.systems = sys;
    s2.objective = "buildings".into();
    let p2 = plan_galaxy(dsp(), galaxy(), &s2);
    let mag = p2.rares.iter().find(|r| r.item == "unipolar-magnet").unwrap();
    assert!((mag.quota_per_cube - total("unipolar-magnet") / s_of(81)).abs() < 1e-6);
    assert!((mag.quota_per_cube - 4.20).abs() < 0.01);
    eprintln!("all magnet stars: magnets/cube {:.3} (quota {:.3}), buildings {:.0}", mag.per_cube, mag.quota_per_cube, p2.buildings);
    // cubes produced = cubes consumed
    let cubes: f64 = p.systems.iter().map(|x| x.cubes).sum();
    assert!((cubes - 1000.0).abs() < 1e-6);
}

#[test]
fn galaxy_plan_max_with_photon_cap() {
    let mut s = gp();
    s.goal = GalaxyGoal::Max;
    s.photon_gw = Some(20.0);
    let p = plan_galaxy(dsp(), galaxy(), &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    // 20 GW → 1000 photons/min → 1000 cubes/min at 1 photon per cube
    assert!((p.z - 1000.0).abs() < 1.0, "z {}", p.z);
    s.plan.effects = vec!["proliferator-3-products".into()];
    let p = plan_galaxy(dsp(), galaxy(), &s);
    assert!((p.z - 1250.0).abs() < 2.0, "z sprayed {}", p.z);
    // no cap → unbounded is reported, not solved
    s.photon_gw = None;
    assert_eq!(plan_galaxy(dsp(), galaxy(), &s).status, Status::Unbounded);
}

#[test]
fn galaxy_plan_building_cap_and_shipping() {
    let mut s = gp();
    s.goal = GalaxyGoal::Max;
    s.building_cap = Some(5000.0);
    let p = plan_galaxy(dsp(), galaxy(), &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    assert!(p.buildings <= 5000.0 + 1e-3);
    assert!(p.z > 100.0, "z {}", p.z);
    // magnets only exist far from home: they have to be shipped in or used where they are mined
    assert!(!p.shipments.is_empty());
    assert!(p.warpers_per_min > 0.0);
}

// ---- review 8

#[test]
fn galaxy_fingerprint_is_the_pinned_one() {
    // the value `wasm_vs_native.mjs` and the live test expect from the WASM build (review 8, H1)
    assert_eq!(galaxy().fingerprint, "a42df4b4a9e7ccc9");
}

#[test]
fn seed_settings_are_sanitized() {
    let s = SeedSettings { seed: 1, stars: 5, resource_multiplier: -3.0, hive_initial_colonize: f64::NAN, hive_max_density: 99.0 }.sanitized();
    assert_eq!((s.stars, s.resource_multiplier, s.hive_initial_colonize, s.hive_max_density), (32, 0.01, 1.0, 10.0));
    assert_eq!(SeedSettings::default().sanitized(), SeedSettings::default());
}

#[test]
fn station_default_is_xiaoxiaos_four_stacked_belts() {
    // 笑笑 BV 18: 4 blue belts within 7 ly, 3 at 12 ly, one belt = 7200/min ("五条蓝带运力=5×7200")
    let s = gp();
    assert!((s.station_a / (7.0 + s.station_d0) - 4.0 * 7200.0).abs() < 1e-6);
    assert!((s.station_a / (12.0 + s.station_d0) - 3.0 * 7200.0).abs() < 1e-6);
}

#[test]
fn a_system_listed_twice_counts_once() {
    let mut a = gp();
    a.systems = auto_systems(galaxy());
    let mut b = a.clone();
    b.systems.push(a.systems[0]);
    b.systems.insert(1, *a.systems.last().unwrap());
    let (pa, pb) = (plan_galaxy(dsp(), galaxy(), &a), plan_galaxy(dsp(), galaxy(), &b));
    assert_eq!(pa.systems.len(), pb.systems.len());
    assert!((pa.buildings - pb.buildings).abs() < 1e-6 * pa.buildings, "{} vs {}", pa.buildings, pb.buildings);
    for (x, y) in pa.rares.iter().zip(&pb.rares) {
        assert!((x.quota_per_cube - y.quota_per_cube).abs() < 1e-9, "{} quota {} vs {}", x.item, x.quota_per_cube, y.quota_per_cube);
    }
}

#[test]
fn dark_fog_scale_counts_finished_levels() {
    let df = |m| DarkFog { enabled: true, level: 24, bases: vec![(0, 1.0)], drops: vec![ItemRate { item: "processor".into(), rate: 10.0 }, ItemRate { item: "df-core-element".into(), rate: 10.0 }], measured_at_level: m };
    let rate = |d: &DarkFog, l| d.rates(l).0;
    // measured and planned at the same level: unchanged, also at level 1 (nothing researched)
    assert!((rate(&df(81), 81)[0].1 - 10.0).abs() < 1e-12);
    assert!((rate(&df(1), 1)[0].1 - 10.0).abs() < 1e-12);
    // measured while researching 51 (50 done, +200 %), planned while researching 101 (100 done, +400 %)
    assert!((rate(&df(51), 101)[0].1 - 10.0 * 5.0 / 3.0).abs() < 1e-9);
    // exclusive items do not scale
    assert!((rate(&df(51), 101)[1].1 - 10.0).abs() < 1e-12);
}

#[test]
fn stock_override_is_the_remaining_stock() {
    let mut s = gp();
    s.systems = auto_systems(galaxy());
    for st in &galaxy().stars {
        if st.planets.iter().any(|p| p.veins.iter().any(|v| v.item == "unipolar-magnet")) && !s.systems.contains(&st.index) {
            s.systems.push(st.index);
        }
    }
    s.remaining = 0.5;
    let quota = |s: &GalaxyPlanSettings, item: &str| plan_galaxy(dsp(), galaxy(), s).rares.iter().find(|r| r.item == item).map(|r| r.quota_per_cube).unwrap();
    // no override: half of the generated stock
    assert!((quota(&s, "unipolar-magnet") - 0.5 * total("unipolar-magnet") / s_of(81)).abs() < 1e-9);
    // 笑笑's 142 k is what is left: the quota is 142 k / S(81) = 3.28 per cube whatever "remaining" says
    s.stock_override.insert("unipolar-magnet".into(), 142_000.0);
    assert!((quota(&s, "unipolar-magnet") - 142_000.0 / s_of(81)).abs() < 1e-9);
    assert!((quota(&s, "unipolar-magnet") - 3.28).abs() < 0.001);
}

#[test]
fn oil_extractors_are_buildings() {
    // only the home system, no gas giants in reach of the plan's need: oil is used
    let mut s = gp();
    s.target = 200.0;
    let p = plan_galaxy(dsp(), galaxy(), &s);
    assert_eq!(p.status, Status::Optimal, "{:?}", p.warnings);
    let oil: f64 = p.systems.iter().flat_map(|x| &x.extraction).filter(|e| e.item == "crude-oil").map(|e| e.rate).sum();
    let seeps: i32 = p.systems.iter().map(|x| galaxy().stars[x.star].planets.iter().map(|pl| pl.seeps).sum::<i32>()).sum();
    if oil > 1e-6 {
        assert!(p.oil_extractors > 0.0 && p.oil_extractors <= seeps as f64 + 1e-6, "{} of {seeps}", p.oil_extractors);
    }
    let sum = p.production_buildings + p.miners + p.pumps + p.oil_extractors + p.collectors + p.receivers + p.stations;
    assert!((p.buildings - sum).abs() < 1e-6);
    // an oil-only run: no gas giants in the plan → extractors = seeps × share of the rate cap
    let mut o = gp();
    o.systems = vec![0];
    o.target = 10.0;
    let p = plan_galaxy(dsp(), galaxy(), &o);
    if p.status == Status::Optimal {
        let e = p.systems[0].extraction.iter().find(|e| e.item == "crude-oil");
        if let Some(e) = e {
            let seeps: i32 = galaxy().stars[0].planets.iter().map(|pl| pl.seeps).sum();
            assert!((p.oil_extractors - seeps as f64 * e.rate / e.cap.unwrap().max(e.rate)).abs() < 1e-6 || e.limit == "quota");
        }
    }
}

#[test]
fn twelve_systems_solve_in_reasonable_time() {
    let mut s = gp();
    s.systems = (0..12).collect();
    let t = web_time::Instant::now();
    let p = plan_galaxy(dsp(), galaxy(), &s);
    eprintln!("12 systems: {:?} {} cols × {} rows, {:.0} ms", p.status, p.columns, p.rows, t.elapsed().as_secs_f64() * 1e3);
    assert_eq!(p.systems.len().max(1), if p.status == Status::Optimal { 12 } else { 1 });
}

#[test]
fn galaxy_total_needs_no_quota_and_no_magnets() {
    // Xinyu 2026-11-23: total cubes from the whole galaxy's stock, machines unlimited, no quota
    let ts = TotalSettings::default();
    let r = galaxy_total(dsp(), galaxy(), &ts);
    // conservation: used ≤ available, per cube × cubes = used
    for u in &r.finite {
        assert!(u.used <= u.available * (1.0 + 1e-7) + 1e-6, "{}", u.item);
        assert!((u.per_cube * r.cubes - u.used).abs() <= 1e-6 * u.used.max(1.0), "{}", u.item);
    }
    assert_eq!(r.status, Status::Optimal, "{:?}", r.warnings);
    eprintln!("total {:.3e} cubes, hub {:?}, warpers {:.0}, {:.0} ms", r.cubes, r.hub, r.warpers, r.solve_ms);
    for u in &r.finite {
        eprintln!("  {:32} avail {:>12.0} used {:>12.0} per cube {:8.3} {} marginal {:.4}", u.item, u.available, u.used, u.per_cube, if u.binding { "BIND" } else { "" }, u.marginal);
    }
    for x in r.recipes.iter().take(12) {
        eprintln!("  {:40} {:10} {:.3}", x.recipe, x.mode, x.machines_per_rate);
    }
    assert!(r.cubes > 0.0);
    assert!(r.finite.iter().any(|u| u.binding));
    // his words: without the unipolar-magnet alternate there are still many cubes, only fewer
    let mut no_mag = ts.clone();
    let mag_recipes: Vec<String> =
        dsp().recipes.iter().filter(|x| x.inputs.iter().any(|(i, _)| i == "unipolar-magnet")).map(|x| x.id.clone()).collect();
    assert!(!mag_recipes.is_empty());
    no_mag.plan.disabled_recipes.extend(mag_recipes);
    let r0 = galaxy_total(dsp(), galaxy(), &no_mag);
    assert_eq!(r0.status, Status::Optimal);
    assert!(r0.finite.iter().all(|u| u.item != "unipolar-magnet" || u.used == 0.0));
    eprintln!("without magnet recipes: {:.3e}", r0.cubes);
    assert!(r0.cubes > 0.5 * r.cubes, "{} vs {}", r0.cubes, r.cubes);
    assert!(r0.cubes <= r.cubes * (1.0 + 1e-6));
    // warpers only cost; proliferator modes only add options
    let mut w = ts.clone();
    w.warpers = true;
    let rw = galaxy_total(dsp(), galaxy(), &w);
    assert!(rw.cubes <= r.cubes * (1.0 + 1e-6) && rw.cubes > 0.5 * r.cubes, "{} vs {}", rw.cubes, r.cubes);
    let mut pr = ts.clone();
    pr.plan.effects = vec!["proliferator-3-speed".into(), "proliferator-3-products".into()];
    let rp = galaxy_total(dsp(), galaxy(), &pr);
    eprintln!("with proliferator Mk.III: {:.3e}, warpers on: {:.3e}", rp.cubes, rw.cubes);
    assert!(rp.cubes >= r.cubes * (1.0 - 1e-6));
    // without research counted, vein utilisation scales the total by 1/0.94^done
    let mut l = ts.clone();
    l.levels_done = 10;
    l.research = false;
    let r10 = galaxy_total(dsp(), galaxy(), &l);
    assert!((r10.cubes * 0.94f64.powi(10) / r.cubes - 1.0).abs() < 1e-4);
    // research counted: done levels are sunk; going on from d0 to d costs ρ(S(d0+1) − S(d+1)) level-0 cubes
    let mut lr = l.clone();
    lr.research = true;
    let rr = galaxy_total(dsp(), galaxy(), &lr);
    assert!((rr.cubes / r10.cubes - 1.0).abs() < 1e-9, "no further research: the headline is unchanged");
    // this galaxy (2.45 M level-0 cubes ≥ S(11)) can research forever
    assert!(rr.infinite && (rr.research_need - s_of(11)).abs() < 1e-6);
    // brute force: cubes after researching 10 → d by the sum of 4000(n−5)·0.94^(n−1)
    let brute = |n0: f64, rho: f64, d0: u32, d: u32| -> f64 {
        let spent: f64 = (d0 + 1..=d).filter(|k| *k >= 6).map(|k| 4000.0 * (k as f64 - 5.0) * 0.94f64.powi(k as i32 - 1)).sum::<f64>() * rho;
        (n0 - spent) / 0.94f64.powi(d as i32)
    };
    for (rem, rho, d0) in [(0.1, 1.0, 0u32), (0.05, 0.8, 3), (0.05, 1.0, 12)] {
        let mut p = lr.clone();
        p.remaining = rem;
        p.rho = rho;
        p.levels_done = d0;
        let rp = galaxy_total(dsp(), galaxy(), &p);
        let n0 = rp.cubes_no_util;
        let best = (d0..=500).map(|d| brute(n0, rho, d0, d)).fold(f64::MIN, f64::max);
        eprintln!("{rem} left, ρ {rho}, from {d0}: now {:.4e}, infinite {}, best {} → {:.4e} ({:.0} research cubes)", rp.cubes, rp.infinite, rp.best_levels_done, rp.best_cubes, rp.research_cubes);
        assert_eq!(rp.infinite, n0 >= rho * s_of((d0 + 1).max(6)));
        assert!((rp.best_cubes / best - 1.0).abs() < 1e-9, "{} vs {best}", rp.best_cubes);
        assert!(rp.best_cubes >= rp.cubes * (1.0 - 1e-12));
        assert!((brute(n0, rho, d0, rp.best_levels_done) / rp.best_cubes - 1.0).abs() < 1e-9);
    }
    // sunk research: 10 % left with 50 levels done is infinite (S(51) ≈ 186 k < 245 k)
    let mut late = lr.clone();
    late.remaining = 0.1;
    late.levels_done = 50;
    assert!(galaxy_total(dsp(), galaxy(), &late).infinite);
    // using fewer systems never gives more
    let mut few = ts.clone();
    few.systems = vec![0, 1, 2];
    let rf = galaxy_total(dsp(), galaxy(), &few);
    assert!(rf.cubes <= r.cubes * (1.0 + 1e-6));
}

#[test]
fn galaxy_rate_is_bounded_by_rates_and_dyson() {
    // Xinyu 2026-11-23: past the threshold the stock is infinite; rates and the Dyson spheres bound z
    let mut rs = RateSettings::default();
    rs.plan.effects = vec!["proliferator-3-speed".into(), "proliferator-3-products".into()];
    rs.dyson = std::env::var("NO_DYSON").is_err();
    let r = galaxy_rate(dsp(), galaxy(), &rs);
    assert_eq!(r.status, Status::Optimal, "{:?}", r.warnings);
    eprintln!(
        "L{} z {:.0}/min, dyson {:.0} GW (used {:.0}), swarm {:?} {:.0} sails/min {:.0} GW, infinite from L{:?}, dyson-bound from L{:?}, {:.0} ms",
        r.level, r.cubes_per_min, r.dyson_gw, r.dyson_used_gw, r.swarm_star, r.sails_per_min, r.swarm_gw, r.infinite_level, r.dyson_level, r.solve_ms
    );
    for u in &r.uses {
        eprintln!("  {:24} {:5} cap {:>12.0} used {:>12.0} per cube {:7.3} {} m {:.4} inf {:?}", u.item, u.source, u.cap, u.used, u.per_cube, if u.binding { "BIND" } else { "" }, u.marginal, u.infinite_from);
        assert!(u.used <= u.cap * (1.0 + 1e-7) + 1e-6, "{}", u.item);
    }
    for p in &r.curve {
        eprintln!("  curve L{} {:.0} {}", p.level, p.cubes_per_min, p.dyson_bound);
    }
    eprintln!("power {:.0} GW ({:.1} MW per cube/min), shells-only ceiling {:.0}/min", r.power_gw, r.power_mw_per_rate, r.ceiling_shells);
    assert!(r.cubes_per_min > 0.0);
    assert!(r.uses.iter().any(|u| u.binding));
    // fire ice comes from veins and giants: both count (review H1)
    let fi = r.uses.iter().find(|u| u.item == "fire-ice").unwrap();
    assert_eq!(fi.source, "vein+gas");
    assert!(r.power_gw > 0.0 && r.ceiling_shells >= r.cubes_per_min * (1.0 - 1e-6));
    // the curve never falls with the level
    for w in r.curve.windows(2) {
        assert!(w[1].cubes_per_min >= w[0].cubes_per_min * (1.0 - 1e-6), "{:?}", r.curve);
    }
    // the Dyson spheres cap photons: photons used ≤ Σ max power
    assert!(r.dyson_used_gw <= r.dyson_gw * (1.0 + 1e-6) + 1e-9);
    // without the swarm: the level from which the shells are full
    let mut ns = rs.clone();
    ns.swarm = false;
    let rn = galaxy_rate(dsp(), galaxy(), &ns);
    assert!(rn.cubes_per_min <= r.cubes_per_min * (1.0 + 1e-6));
    if let Some(d) = rn.dyson_level {
        let mut hi = ns.clone();
        hi.level = d.max(rs.level) + 50;
        let rh = galaxy_rate(dsp(), galaxy(), &hi);
        // past it the shells are used in full (photons bind; more ore still helps by spending them better)
        assert!(rh.uses.iter().any(|u| u.item == "critical-photon" && u.binding), "{:?}", rh.uses.iter().filter(|u| u.binding).map(|u| &u.item).collect::<Vec<_>>());
        assert!(rh.dyson_used_gw >= rh.dyson_gw * (1.0 - 1e-6));
        let mut hs = rs.clone();
        hs.level = hi.level;
        let rhs = galaxy_rate(dsp(), galaxy(), &hs);
        eprintln!("L{}: shells only {:.0}, with swarm {:.0} ({:.0} sails/min, {:.0} GW)", hi.level, rh.cubes_per_min, rhs.cubes_per_min, rhs.sails_per_min, rhs.swarm_gw);
        // the swarm is an option: never fewer cubes (with factory power charged it may not pay)
        assert!(rhs.cubes_per_min >= rh.cubes_per_min * (1.0 - 1e-6));
        let mut np = hs.clone();
        np.power = false;
        let rnp = galaxy_rate(dsp(), galaxy(), &np);
        eprintln!("  without factory power: {:.0}/min, {:.0} sails/min", rnp.cubes_per_min, rnp.sails_per_min);
        assert!(rnp.cubes_per_min >= rhs.cubes_per_min * (1.0 - 1e-6));
    }
    // shell power: birth star one layer ≈ 38.7 GW, ten layers ≈ 254 GW (research note)
    let birth = &galaxy().stars[0];
    let p = dyson_max_gw(birth);
    assert!(p > 150.0 && p < 400.0, "{} {} {}", birth.luminosity, birth.dyson_radius, p);
    // more miners per node never gives fewer cubes; a subset of stars never more
    let mut m2 = rs.clone();
    m2.miners_per_node = 2.0;
    assert!(galaxy_rate(dsp(), galaxy(), &m2).cubes_per_min >= r.cubes_per_min * (1.0 - 1e-6));
    let mut sub = rs.clone();
    sub.systems = vec![0, 1, 2];
    assert!(galaxy_rate(dsp(), galaxy(), &sub).cubes_per_min <= r.cubes_per_min * (1.0 + 1e-6));
}
