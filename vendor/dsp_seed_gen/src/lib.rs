//! DSP galaxy generation for one seed (DoubleUTH/DSP-Seed-Finder, Apache-2.0; see NOTICE.md).
//! `generate` runs the full star → planet → terrain → vein generation ("actual veins") and returns
//! an owned summary the planner uses.
#![allow(dead_code, unused_imports, unused_variables, clippy::all)]
pub mod data;
pub mod detmath;
pub mod worldgen;

use data::enums::{PlanetType, VeinType};
pub use data::game_desc::GameDesc;
use std::cell::Cell;

#[derive(Debug, Clone)]
pub struct VeinSummary {
    /// Upstream vein type name ("Iron", "Oil", "Mag", ...).
    pub kind: &'static str,
    /// Sum over kept nodes. Ores: units of ore. Oil: seep amounts (× 4e-5 = oil/s).
    pub amount: i64,
    /// Nodes kept (oil: seeps).
    pub nodes: i32,
}

#[derive(Debug, Clone)]
pub struct PlanetSummary {
    pub index: usize,
    /// "Volcano" | "Ocean" | "Desert" | "Ice" | "Gas"
    pub kind: &'static str,
    pub theme: &'static str,
    pub theme_id: i32,
    /// Ocean item id: 1000 water, 1116 sulfuric acid, 0 none.
    pub water_item: i32,
    pub orbit_around: Option<usize>,
    pub radius: f32,
    pub sun_distance: f32,
    pub wind: f32,
    /// Gas giants: (item id, base rate per collector); 1120 hydrogen, 1121 deuterium, 1011 fire ice.
    pub gases: Vec<(i32, f32)>,
    pub veins: Vec<VeinSummary>,
}

#[derive(Debug, Clone)]
pub struct StarSummary {
    pub index: usize,
    pub name: String,
    /// "MainSeqStar" | "GiantStar" | "WhiteDwarf" | "NeutronStar" | "BlackHole"
    pub star_type: String,
    pub spectr: String,
    pub luminosity: f32,
    /// Light years from the origin (the birth star is at 0).
    pub position: (f64, f64, f64),
    pub dyson_radius: i32,
    pub initial_hives: i32,
    pub max_hives: i32,
    pub resource_coef: f32,
    pub planets: Vec<PlanetSummary>,
}

pub fn vein_name(v: VeinType) -> &'static str {
    match v {
        VeinType::Iron => "Iron",
        VeinType::Copper => "Copper",
        VeinType::Silicium => "Silicium",
        VeinType::Titanium => "Titanium",
        VeinType::Stone => "Stone",
        VeinType::Coal => "Coal",
        VeinType::Oil => "Oil",
        VeinType::Fireice => "Fireice",
        VeinType::Diamond => "Diamond",
        VeinType::Fractal => "Fractal",
        VeinType::Crysrub => "Crysrub",
        VeinType::Grat => "Grat",
        VeinType::Bamboo => "Bamboo",
        VeinType::Mag => "Mag",
        _ => "None",
    }
}

fn planet_kind(t: &PlanetType) -> &'static str {
    match t {
        PlanetType::Volcano => "Volcano",
        PlanetType::Ocean => "Ocean",
        PlanetType::Desert => "Desert",
        PlanetType::Ice => "Ice",
        PlanetType::Gas => "Gas",
        PlanetType::None => "None",
    }
}

/// Generate the galaxy of `seed` with actual veins (terrain-checked; ~1 s native for 64 stars).
pub fn generate(seed: i32, desc: &GameDesc) -> Vec<StarSummary> {
    let mut desc = *desc;
    desc.use_actual_veins = true;
    let habitable = Cell::new(0_i32);
    let galaxy = worldgen::galaxy_gen::create_galaxy(seed, &desc, &habitable);
    galaxy
        .stars
        .iter()
        .map(|sp| {
            let st = &sp.star;
            let planets = sp
                .get_planets()
                .iter()
                .map(|p| {
                    let theme = p.get_theme();
                    let orbit_around = p.orbit_around.borrow().map(|o| o.index);
                    PlanetSummary {
                        index: p.index,
                        kind: planet_kind(p.get_type()),
                        theme: theme.name,
                        theme_id: theme.id,
                        water_item: theme.water_item_id,
                        orbit_around,
                        radius: p.radius,
                        sun_distance: p.get_sun_distance(),
                        wind: theme.wind,
                        gases: p.get_gases().clone(),
                        veins: p
                            .get_actual_veins()
                            .iter()
                            .map(|v| VeinSummary { kind: vein_name(v.vein_type), amount: v.amount as i64, nodes: v.nodes })
                            .collect(),
                    }
                })
                .collect();
            StarSummary {
                index: st.index,
                name: sp.name.clone(),
                star_type: format!("{:?}", st.star_type),
                spectr: format!("{:?}", st.get_spectr()),
                luminosity: st.get_luminosity(),
                position: (st.position.0, st.position.1, st.position.2),
                dyson_radius: st.get_dyson_radius(),
                initial_hives: st.get_initial_hive_count(),
                max_hives: st.get_max_hive_count(),
                resource_coef: st.get_resource_coef(),
                planets,
            }
        })
        .collect()
}

/// FNV-1a over every generated number, bit for bit (star luminosity / position / Dyson radius / hives,
/// planet radius / orbit, gas rates, vein amounts and node counts). The native and the WASM build must
/// give the same value for a seed (review 8, H1): the tests pin it natively, `wasm_vs_native.mjs` and
/// the live test compare the browser's.
pub fn fingerprint(stars: &[StarSummary]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut add = |v: u64| {
        for b in v.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    for s in stars {
        add(s.index as u64);
        add(s.luminosity.to_bits() as u64);
        add(s.position.0.to_bits());
        add(s.position.1.to_bits());
        add(s.position.2.to_bits());
        add(s.dyson_radius as u64);
        add(s.initial_hives as u64);
        add(s.max_hives as u64);
        add(s.resource_coef.to_bits() as u64);
        for p in &s.planets {
            add(p.index as u64);
            add(p.theme_id as u64);
            add(p.radius.to_bits() as u64);
            add(p.sun_distance.to_bits() as u64);
            for (i, r) in &p.gases {
                add(*i as u64);
                add(r.to_bits() as u64);
            }
            for v in &p.veins {
                for b in v.kind.bytes() {
                    add(b as u64);
                }
                add(v.amount as u64);
                add(v.nodes as u64);
            }
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same numbers on every target: these values are also what the WASM build must print
    /// (`docs/research/dsp-seed/_过程文件/wasm_vs_native.mjs`). They change only when the generator does.
    #[test]
    fn fingerprints_are_pinned() {
        let fp = |seed, stars, mult| format!("{:016x}", fingerprint(&generate(seed, &desc(stars, mult))));
        assert_eq!(fp(12345678, 32, 0.1), FP_12345678_32_01);
        assert_eq!(fp(29031989, 64, 1.0), FP_29031989_64_1);
    }
    const FP_12345678_32_01: &str = "a42df4b4a9e7ccc9";
    const FP_29031989_64_1: &str = "0ac61e60b96eab08";

    /// Values players published from the game (all before the Dark Fog update, so a spot check, not proof):
    /// - Steam discussion 3190239190928374828 (Nekogod, 2022-01-26), seed 29031989, 64 stars, home planet:
    ///   "2.3m iron", "49.78 oil per second" (its "8.3m coal" is 8.59 M here: not matched);
    /// - Gamersky handbook 202102/1365548 (bgli100, 2021-02-25), 64-star seeds: 13636977 has an O star of
    ///   luminosity 2.45 at 2.43 ly; 39497430 one of 2.47 at 5.95 ly.
    #[test]
    fn published_in_game_values() {
        let g = generate(29031989, &desc(64, 1.0));
        let home = &g[0].planets[2];
        assert_eq!(home.theme, "Ocean 1");
        let v = |k: &str| home.veins.iter().find(|v| v.kind == k).unwrap();
        assert_eq!((v("Iron").amount as f64 / 1e5).round(), 23.0);
        assert_eq!(((v("Oil").amount as f64) * 4e-5 * 100.0).round(), 4978.0);
        for (seed, lum, ly) in [(13636977, 2.45, 2.43), (39497430, 2.47, 5.95)] {
            let g = generate(seed, &desc(64, 1.0));
            let o = g
                .iter()
                .filter(|s| s.spectr == "O")
                .map(|s| (s.luminosity as f64, (s.position.0.powi(2) + s.position.1.powi(2) + s.position.2.powi(2)).sqrt()))
                .find(|(l, d)| (l - lum).abs() < 0.006 && (d - ly).abs() < 0.006);
            assert!(o.is_some(), "seed {seed}: no O star of luminosity {lum} at {ly} ly");
        }
    }

    fn desc(stars: usize, mult: f32) -> GameDesc {
        GameDesc { star_count: stars, resource_multiplier: mult, hive_initial_colonize: 1.0, hive_max_density: 1.0, use_actual_veins: true }
    }

    /// Values from the research prototype (DoubleUTH websocket, `_过程文件/sample_seed12345678_32stars_x0.1_actual.txt`).
    #[test]
    fn seed_12345678_matches_prototype_sample() {
        let g = generate(12345678, &desc(32, 0.1));
        assert_eq!(g.len(), 32);
        let s0 = &g[0];
        assert_eq!(s0.name, "AngelStern");
        assert!((s0.luminosity - 0.982).abs() < 5e-4);
        assert_eq!(s0.dyson_radius, 21700);
        let p0 = &s0.planets[0];
        assert_eq!(p0.kind, "Volcano");
        let iron = p0.veins.iter().find(|v| v.kind == "Iron").unwrap();
        assert_eq!(iron.amount, 1553315);
        let p2 = &s0.planets[2];
        let oil = p2.veins.iter().find(|v| v.kind == "Oil").unwrap();
        assert!(((oil.amount as f64) * 4e-5 - 26.70).abs() < 0.006);
        assert!(oil.nodes >= 1);
        // galaxy totals (report §2.3)
        let total = |k: &str| -> i64 { g.iter().flat_map(|s| &s.planets).flat_map(|p| &p.veins).filter(|v| v.kind == k).map(|v| v.amount).sum() };
        assert_eq!((total("Iron") as f64 / 1e5).round(), 1278.0);
        assert_eq!((total("Mag") as f64 / 1e3).round(), 182.0);
        let oil_rate: f64 = total("Oil") as f64 * 4e-5;
        assert!((oil_rate - 404.8).abs() < 0.1, "{oil_rate}");
        let giants = g.iter().flat_map(|s| &s.planets).filter(|p| p.kind == "Gas").count();
        assert_eq!(giants, 15);
    }
}
