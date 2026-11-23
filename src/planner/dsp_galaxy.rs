//! DSP: plan a whole galaxy from its seed (memo step 13; spec `docs/research/dsp-seed/00-最终报告.md`).
//!
//! 1. **Seed import** (report §2, §9.1): the vendored DoubleUTH generator (`vendor/dsp_seed_gen`,
//!    Apache-2.0) gives stars, planets, vein amounts and node counts (oil: seeps), gas giants, oceans.
//! 2. **Infinite threshold** (report §4): with vein utilisation at level L and every cube going into
//!    research from L on, the stock consumed from then on is a_r·ρ·S(L), S(L) = Σ_{n≥L} 4000(n−5)·0.94^(n−1).
//!    A resource is safe at L when a_r·ρ·k_r·S(L) ≤ B_r (k_r: stock points per unit mined, 1 for ores).
//! 3. **Multi-system LP** (report §5.1, §5.3, §9.4): production per star system, interstellar shipping
//!    (stations ∝ distance, warpers per trip), finite stocks as the homogeneous quota
//!    m_{r,s}·ρ·k_r·S(L)/(1−φ) ≤ B_{r,s}·z, oil / gas-giant / photon rates, target or maximum z.
//! 4. **Dark Fog** (report §5.4): drops as rate-limited sources at farmed systems, rates entered by the
//!    user (no public data), gated by the BWIKI unlock table, normal items scaled by (1+0.04L).
//!
//! Choices the report leaves open are marked MINE in the comments and in `docs/research/dsp-seed/_过程文件/progress-impl.md`.
use super::data::{GameData, Recipe};
use super::lp::allowed_recipes;
use super::rules::{self, Mode};
use super::{ItemRate, Plan, PlanSettings, Status};
use microlp::{ComparisonOp, OptimizationDirection, Problem, Variable};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const CUBE: &str = "universe-matrix";
pub const WARPER: &str = "space-warper";
pub const PHOTON: &str = "critical-photon";
/// Vein utilisation: stock consumed per unit mined is 0.94^level (BWIKI 矿物利用, Saros readme).
pub const Q: f64 = 0.94;
/// Research station sprayed with Mk.III: hashes ×1.25, cubes per level ×0.8 (report §3, 存疑).
pub const RHO_SPRAYED: f64 = 0.8;
/// Oil seep floor that is never mined, in seep amount points (0.1/s; report §2.3, 存疑).
pub const SEEP_FLOOR: f64 = 2500.0;
/// Orbital collector: base ×8, 30 MW own use, at most 40 per giant (BWIKI 轨道采集器).
pub const COLLECTOR_BASE: f64 = 8.0;
pub const COLLECTOR_SELF_MW: f64 = 30.0;
pub const COLLECTORS_PER_GIANT: f64 = 40.0;
/// Fuel value MJ: hydrogen 8, deuterium 9, fire ice 4.8 (report §5.5, 存疑).
pub fn heat_mj(item: &str) -> f64 {
    match item {
        "hydrogen" => 8.0,
        "deuterium" => 9.0,
        "fire-ice" => 4.8,
        _ => 0.0,
    }
}
/// One critical photon = 1200 MJ of ray-receiver power (120 MW → 6/min; report §3).
pub const PHOTON_MJ: f64 = 1200.0;

// ---------------------------------------------------------------------------------------------
// Seed import

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SeedSettings {
    pub seed: i32,
    pub stars: usize,
    pub resource_multiplier: f64,
    /// Dark Fog settings that change hive counts (report §2.1; default 1 = the generator's default).
    pub hive_initial_colonize: f64,
    pub hive_max_density: f64,
}

impl Default for SeedSettings {
    fn default() -> Self {
        Self { seed: 12345678, stars: 32, resource_multiplier: 0.1, hive_initial_colonize: 1.0, hive_max_density: 1.0 }
    }
}

impl SeedSettings {
    /// The settings the generator really runs with: 32–64 stars, a positive finite resource multiplier
    /// (≥ 99.5 = infinite), hive settings ≥ 0. `Galaxy::settings` holds this form (review 8, L5).
    pub fn sanitized(&self) -> Self {
        let fin = |x: f64, lo: f64, hi: f64, d: f64| if x.is_finite() { x.clamp(lo, hi) } else { d };
        Self {
            seed: self.seed,
            stars: self.stars.clamp(32, 64),
            resource_multiplier: fin(self.resource_multiplier, 0.01, 100.0, 1.0),
            hive_initial_colonize: fin(self.hive_initial_colonize, 0.0, 10.0, 1.0),
            hive_max_density: fin(self.hive_max_density, 0.0, 10.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vein {
    pub item: String,
    /// Ores: units. Oil: seep amount points (× 4e-5 = oil/s at level 0).
    pub amount: f64,
    pub nodes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasRate {
    pub item: String,
    /// Base rate of the planet (generator `gases`, rare tier ×0.8 included), per second.
    pub base: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanetInfo {
    pub index: usize,
    pub kind: String,
    pub theme: String,
    /// "water" | "sulfuric-acid" | null
    pub ocean: Option<String>,
    pub orbit_around: Option<usize>,
    pub gases: Vec<GasRate>,
    pub veins: Vec<Vein>,
    /// Oil seeps and their initial rate at level 0 (oil/s).
    pub seeps: i32,
    pub oil_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarInfo {
    pub index: usize,
    pub name: String,
    pub star_type: String,
    pub spectr: String,
    pub luminosity: f64,
    pub position: [f64; 3],
    /// Light years from the birth star.
    pub distance: f64,
    pub dyson_radius: i32,
    pub initial_hives: i32,
    pub max_hives: i32,
    pub planets: Vec<PlanetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockLine {
    pub item: String,
    /// Stock points: ores = units; oil = seep points above the floor (2500 per seep).
    pub stock: f64,
    /// Planets that hold it.
    pub planets: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Galaxy {
    pub settings: SeedSettings,
    pub rare_tier: bool,
    pub infinite: bool,
    pub stars: Vec<StarInfo>,
    pub totals: Vec<StockLine>,
    pub gas_giants: usize,
    /// Σ base gas rate per item over all giants (per second).
    pub gas_totals: Vec<GasRate>,
    pub oil_rate: f64,
    pub seeps: i32,
    pub generate_ms: f64,
    /// `dsp_seed_gen::fingerprint` in hex: the same on every target for the same settings (review 8, H1).
    pub fingerprint: String,
}

pub fn vein_item(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "Iron" => "iron-ore",
        "Copper" => "copper-ore",
        "Silicium" => "silicon-ore",
        "Titanium" => "titanium-ore",
        "Stone" => "stone",
        "Coal" => "coal",
        "Oil" => "crude-oil",
        "Fireice" => "fire-ice",
        "Diamond" => "kimberlite-ore",
        "Fractal" => "fractal-silicon",
        "Crysrub" => "organic-crystal",
        "Grat" => "optical-grating-crystal",
        "Bamboo" => "spiniform-stalagmite-crystal",
        "Mag" => "unipolar-magnet",
        _ => return None,
    })
}

fn gas_item(id: i32) -> Option<&'static str> {
    Some(match id {
        1120 => "hydrogen",
        1121 => "deuterium",
        1011 => "fire-ice",
        _ => return None,
    })
}

/// Finite stocks, in the order of the report's tables.
pub const FINITE: [&str; 14] = [
    "iron-ore", "copper-ore", "silicon-ore", "titanium-ore", "stone", "coal", "crude-oil", "fire-ice",
    "kimberlite-ore", "fractal-silicon", "organic-crystal", "optical-grating-crystal",
    "spiniform-stalagmite-crystal", "unipolar-magnet",
];

/// Stock points removed per unit mined at level 0 (report §3): ores 1; crude oil derived from the seep
/// half-life, 5 h at ≤ 0.3× (≈1 point per oil) and multiplier × 12 h above (0.4 / multiplier). 存疑.
pub fn points_per_unit(item: &str, multiplier: f64) -> f64 {
    if item != "crude-oil" {
        return 1.0;
    }
    if multiplier <= 0.3001 {
        1.0
    } else {
        (0.4 / multiplier).min(1.0)
    }
}

pub fn generate(s: &SeedSettings) -> Galaxy {
    let t0 = web_time::Instant::now();
    let s = &s.sanitized();
    let desc = dsp_seed_gen::GameDesc {
        star_count: s.stars,
        resource_multiplier: s.resource_multiplier as f32,
        hive_initial_colonize: s.hive_initial_colonize,
        hive_max_density: s.hive_max_density,
        use_actual_veins: true,
    };
    let raw = dsp_seed_gen::generate(s.seed, &desc);
    let birth = raw.first().map(|st| st.position).unwrap_or((0.0, 0.0, 0.0));
    let stars: Vec<StarInfo> = raw
        .iter()
        .map(|st| {
            let (x, y, z) = st.position;
            let d = ((x - birth.0).powi(2) + (y - birth.1).powi(2) + (z - birth.2).powi(2)).sqrt();
            StarInfo {
                index: st.index,
                name: st.name.clone(),
                star_type: st.star_type.clone(),
                spectr: st.spectr.clone(),
                luminosity: st.luminosity as f64,
                position: [x, y, z],
                distance: d,
                dyson_radius: st.dyson_radius,
                initial_hives: st.initial_hives,
                max_hives: st.max_hives,
                planets: st
                    .planets
                    .iter()
                    .map(|p| {
                        let veins: Vec<Vein> = p
                            .veins
                            .iter()
                            .filter_map(|v| {
                                vein_item(v.kind).map(|it| Vein { item: it.into(), amount: v.amount as f64, nodes: v.nodes })
                            })
                            .collect();
                        let oil = veins.iter().find(|v| v.item == "crude-oil");
                        PlanetInfo {
                            index: p.index,
                            kind: p.kind.into(),
                            theme: p.theme.into(),
                            ocean: match p.water_item {
                                1000 => Some("water".into()),
                                1116 => Some("sulfuric-acid".into()),
                                _ => None,
                            },
                            orbit_around: p.orbit_around,
                            gases: p
                                .gases
                                .iter()
                                .filter_map(|(id, r)| gas_item(*id).map(|it| GasRate { item: it.into(), base: *r as f64 }))
                                .collect(),
                            seeps: oil.map(|o| o.nodes).unwrap_or(0),
                            oil_rate: oil.map(|o| o.amount * 4e-5).unwrap_or(0.0),
                            veins,
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    let mut totals: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    let mut gas: BTreeMap<String, f64> = BTreeMap::new();
    let (mut giants, mut oil_rate, mut seeps) = (0, 0.0, 0);
    for st in &stars {
        for p in &st.planets {
            for v in &p.veins {
                let e = totals.entry(v.item.clone()).or_default();
                e.0 += stock_of(v);
                e.1 += 1;
            }
            if !p.gases.is_empty() {
                giants += 1;
            }
            for g in &p.gases {
                *gas.entry(g.item.clone()).or_default() += g.base;
            }
            oil_rate += p.oil_rate;
            seeps += p.seeps;
        }
    }
    let order = |it: &str| FINITE.iter().position(|x| *x == it).unwrap_or(99);
    let mut totals: Vec<StockLine> =
        totals.into_iter().map(|(item, (stock, planets))| StockLine { item, stock, planets }).collect();
    totals.sort_by_key(|l| order(&l.item));
    Galaxy {
        rare_tier: s.resource_multiplier <= 0.1001,
        infinite: s.resource_multiplier >= 99.5,
        settings: s.clone(),
        stars,
        totals,
        gas_giants: giants,
        gas_totals: gas.into_iter().map(|(item, base)| GasRate { item, base }).collect(),
        oil_rate,
        seeps,
        generate_ms: t0.elapsed().as_secs_f64() * 1e3,
        fingerprint: format!("{:016x}", dsp_seed_gen::fingerprint(&raw)),
    }
}

/// Stock points of one vein entry: oil above the per-seep floor, ores as they are.
pub fn stock_of(v: &Vein) -> f64 {
    if v.item == "crude-oil" {
        (v.amount - SEEP_FLOOR * v.nodes as f64).max(0.0)
    } else {
        v.amount
    }
}

// ---------------------------------------------------------------------------------------------
// Vein utilisation math (report §4.1)

/// S(L) = Σ_{n≥L} 4000·(n−5)·0.94^(n−1) (closed form; L < 6 is taken as 6).
pub fn s_of(level: u32) -> f64 {
    let m = level.max(6) as f64 - 5.0;
    4000.0 * Q.powi(4) * Q.powf(m) * (m - (m - 1.0) * Q) / (1.0 - Q).powi(2)
}

/// Cubes spent to reach level L (research of levels 6..L−1): 2000·(L−6)(L−5)·ρ.
pub fn cubes_to_reach(level: u32, rho: f64) -> f64 {
    let l = level.max(6) as f64;
    2000.0 * (l - 6.0) * (l - 5.0) * rho
}

/// Mining speed multiplier while researching level L (L−1 done): 1 + 0.1·(L−1).
pub fn speed_at(level: u32) -> f64 {
    1.0 + 0.1 * (level.max(1) - 1) as f64
}

/// Smallest L ≥ 6 at which `per_cube` units per cube can be used forever from stock `stock`.
pub fn threshold_level(per_cube: f64, stock: f64, rho: f64, k: f64) -> Option<u32> {
    if per_cube <= 1e-12 {
        return Some(6);
    }
    if stock <= 0.0 {
        return None;
    }
    let need = |l: u32| per_cube * rho * k * s_of(l) <= stock;
    if need(6) {
        return Some(6);
    }
    let (mut lo, mut hi) = (6u32, 10_000u32);
    if !need(hi) {
        return None;
    }
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if need(mid) {
            hi = mid
        } else {
            lo = mid
        }
    }
    Some(hi)
}

/// Net output per orbital collector per second for one giant at speed f (BWIKI 轨道采集器): gross
/// 8·f·base per gas; 30 MW own use taken from the gases in proportion to their heat (report §5.5).
pub fn collector_net(gases: &[GasRate], speed: f64) -> Vec<(String, f64)> {
    let gross: Vec<(String, f64)> = gases.iter().map(|g| (g.item.clone(), COLLECTOR_BASE * speed * g.base)).collect();
    let heat: f64 = gross.iter().map(|(i, r)| r * heat_mj(i)).sum();
    let keep = if heat > COLLECTOR_SELF_MW { 1.0 - COLLECTOR_SELF_MW / heat } else { 0.0 };
    gross.into_iter().map(|(i, r)| (i, r * keep)).collect()
}

// ---------------------------------------------------------------------------------------------
// Dark Fog (report §5.4)

/// BWIKI 黑雾掉落: Dark Fog level from which an item drops (items not listed: from level 0).
pub fn drop_unlock_level(item: &str) -> u32 {
    match item {
        "processor" | "graphene" => 9,
        "carbon-nanotube" | "df-dark-fog-matrix" => 12,
        "optical-grating-crystal" | "particle-container" | "casimir-crystal" | "df-matter-recombinator" => 15,
        "strange-matter" | "df-silicon-based-neuron" => 18,
        "antimatter" | "quantum-chip" | "graviton-lens" | "space-warper" | "df-negentropy-singularity" => 21,
        "unipolar-magnet" | "antimatter-fuel-rod" | "df-core-element" => 24,
        _ => 0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DarkFog {
    pub enabled: bool,
    /// Dark Fog (enemy) level: gates which items drop.
    pub level: u32,
    /// Farmed ground bases per star index.
    pub bases: Vec<(usize, f64)>,
    /// Measured drops per base per minute (no public data: in-game measurement, report §5.4).
    pub drops: Vec<ItemRate>,
    /// Vein-utilisation level being researched when `drops` were measured (L_m − 1 levels done, the same
    /// convention as the plan's level L); normal items scale by (1+0.04(L−1))/(1+0.04(L_m−1)).
    pub measured_at_level: u32,
}

impl DarkFog {
    /// Drops per base per minute while level `level` is researched (`level − 1` done, like the mining
    /// speed), after the unlock gate.
    pub fn rates(&self, level: u32) -> (Vec<(String, f64)>, Vec<String>) {
        let mut out = vec![];
        let mut warn = vec![];
        if !self.enabled {
            return (out, warn);
        }
        let done = |l: u32| l.saturating_sub(1) as f64;
        let scale = (1.0 + 0.04 * done(level)) / (1.0 + 0.04 * done(self.measured_at_level));
        for d in &self.drops {
            if !(d.rate.is_finite() && d.rate > 0.0) {
                continue;
            }
            let lv = drop_unlock_level(&d.item);
            if self.level < lv {
                warn.push(format!("Dark Fog: {} drops from Dark Fog level {lv} (set {}): ignored", d.item, self.level));
                continue;
            }
            // exclusive df-* items are not scaled by the debris bonus (BWIKI, Saros readme)
            let k = if d.item.starts_with("df-") { 1.0 } else { scale };
            out.push((d.item.clone(), d.rate * k));
        }
        (out, warn)
    }
    pub fn total_bases(&self) -> f64 {
        self.bases.iter().map(|b| b.1.max(0.0)).sum()
    }
}

// ---------------------------------------------------------------------------------------------
// Threshold button (report §4.3, §9.2): per resource, the level from which its unconstrained use can
// be released, and the first level from which the stock-constrained LP is feasible.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThresholdSettings {
    pub seed: SeedSettings,
    /// Recipe settings (alternates, proliferator effects, buildings); targets are replaced by 1 cube/min.
    pub plan: PlanSettings,
    /// Cubes/min, to turn cumulative cubes into hours.
    pub rate: f64,
    /// Remaining share of every stock (0..1): B_r must be what is left, not the initial amount.
    pub remaining: f64,
    /// Per-item stock override (stock points), e.g. 笑笑's 142 k magnets.
    pub stock_override: HashMap<String, f64>,
    pub dark_fog: DarkFog,
    /// Vein-utilisation level for the Dark Fog drop scaling and the level profile.
    pub level: u32,
}

impl Default for ThresholdSettings {
    fn default() -> Self {
        let mut plan = PlanSettings::default();
        plan.game = "dsp".into();
        plan.allow_all_alternates = true;
        plan.objective = super::Objective::Buildings;
        Self {
            seed: SeedSettings::default(),
            plan,
            rate: 750.0,
            remaining: 1.0,
            stock_override: HashMap::new(),
            dark_fog: DarkFog::default(),
            level: 81,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdRow {
    pub item: String,
    pub stock: f64,
    /// Units per cube in the unconstrained plan (mined, after Dark Fog drops).
    pub per_cube: f64,
    /// Release level, cubes to reach it and hours at `rate`: [ρ = 1, ρ = 0.8].
    pub level: [Option<u32>; 2],
    pub cubes: [Option<f64>; 2],
    pub hours: [Option<f64>; 2],
    /// Safe quota per cube at the profile level: B / (ρ·k·S(L)), ρ = 1.
    pub quota_at_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdReport {
    pub rows: Vec<ThresholdRow>,
    /// First level at which the stock-constrained LP (all quotas at once) is feasible, ρ = 1 / 0.8.
    pub feasible_from: [Option<u32>; 2],
    /// Fewest-building plan for 1 cube/min under the quotas at `feasible_from[0]`: raw per cube.
    pub raw_at_feasible: Vec<ItemRate>,
    /// Quota-constrained raw per cube at the chosen `level` (ρ = 1).
    pub raw_at_level: Vec<ItemRate>,
    pub level: u32,
    pub s_level: f64,
    pub warnings: Vec<String>,
}

fn stocks_for(g: &Galaxy, remaining: f64, over: &HashMap<String, f64>) -> BTreeMap<String, f64> {
    let mut m: BTreeMap<String, f64> = FINITE.iter().map(|i| (i.to_string(), 0.0)).collect();
    for l in &g.totals {
        m.insert(l.item.clone(), l.stock * remaining.clamp(0.0, 1.0));
    }
    for (k, v) in over {
        if v.is_finite() && *v >= 0.0 {
            m.insert(k.clone(), *v);
        }
    }
    m
}

/// Unit-rate (1 cube/min) single-site plan with each finite stock capped at its quota for (L, ρ);
/// `None` level = no quotas. Exclusive Dark Fog items, logs and plant fuel only come from drops.
fn unit_plan(data: &GameData, ts: &ThresholdSettings, g: &Galaxy, stocks: &BTreeMap<String, f64>, quota: Option<(u32, f64)>) -> Plan {
    let mut st = ts.plan.clone();
    st.game = "dsp".into();
    st.integer = false;
    // report §4.3: the quota LP minimises buildings (the sidebar objective is for the main plan)
    st.objective = super::Objective::Buildings;
    st.targets = vec![ItemRate { item: CUBE.into(), rate: 1.0 }];
    st.ignore_limits = false;
    st.resource_limits.clear();
    for r in &data.resources {
        if r.item.starts_with("df-") || r.item == "log" || r.item == "plant-fuel" {
            st.resource_limits.insert(r.item.clone(), Some(0.0));
        }
    }
    let (drops, _) = ts.dark_fog.rates(ts.level);
    let bases = ts.dark_fog.total_bases();
    st.supplies = drops
        .iter()
        .map(|(i, r)| ItemRate { item: i.clone(), rate: r * bases / ts.rate.max(1e-9) })
        .collect();
    if let Some((l, rho)) = quota {
        let mult = g.settings.resource_multiplier;
        for (k, b) in stocks {
            st.resource_limits.insert(k.clone(), Some(b / (rho * points_per_unit(k, mult) * s_of(l))));
        }
    }
    super::plan(data, &st)
}

fn raw_lines(p: &Plan) -> Vec<ItemRate> {
    p.raw.iter().filter(|r| r.rate > 1e-9).map(|r| ItemRate { item: r.item.clone(), rate: r.rate }).collect()
}

pub fn thresholds(data: &GameData, g: &Galaxy, ts: &ThresholdSettings) -> ThresholdReport {
    let stocks = stocks_for(g, ts.remaining, &ts.stock_override);
    let mut warnings = ts.dark_fog.rates(ts.level).1;
    let free = unit_plan(data, ts, g, &stocks, None);
    if free.status != Status::Optimal {
        warnings.push(format!("unconstrained plan: {:?} {}", free.status, free.warnings.join("; ")));
    }
    let raw = super::lp::raw_map(&free);
    let mult = g.settings.resource_multiplier;
    let rate = ts.rate.max(1e-9);
    let rows = stocks
        .iter()
        .map(|(item, &stock)| {
            let a = raw.get(item).copied().unwrap_or(0.0);
            let k = points_per_unit(item, mult);
            let level = [threshold_level(a, stock, 1.0, k), threshold_level(a, stock, RHO_SPRAYED, k)];
            let rhos = [1.0, RHO_SPRAYED];
            let cubes = [0, 1].map(|j| level[j].map(|l| cubes_to_reach(l, rhos[j])));
            ThresholdRow {
                item: item.clone(),
                stock,
                per_cube: a,
                level,
                cubes,
                hours: [0, 1].map(|j| cubes[j].map(|c| c / rate / 60.0)),
                quota_at_level: stock / (k * s_of(ts.level)),
            }
        })
        .collect();
    // first feasible level of the quota LP: feasibility is monotone in L (quotas only grow)
    let feasible = |l: u32, rho: f64| unit_plan(data, ts, g, &stocks, Some((l, rho))).status == Status::Optimal;
    let first = |rho: f64| -> Option<u32> {
        if feasible(6, rho) {
            return Some(6);
        }
        let (mut lo, mut hi) = (6u32, 400u32);
        if !feasible(hi, rho) {
            return None;
        }
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if feasible(mid, rho) {
                hi = mid
            } else {
                lo = mid
            }
        }
        Some(hi)
    };
    let feasible_from = [first(1.0), first(RHO_SPRAYED)];
    let raw_at_feasible = feasible_from[0]
        .map(|l| raw_lines(&unit_plan(data, ts, g, &stocks, Some((l, 1.0)))))
        .unwrap_or_default();
    let at_level = unit_plan(data, ts, g, &stocks, Some((ts.level, 1.0)));
    let raw_at_level = if at_level.status == Status::Optimal { raw_lines(&at_level) } else { vec![] };
    ThresholdReport { rows, feasible_from, raw_at_feasible, raw_at_level, level: ts.level, s_level: s_of(ts.level), warnings }
}

// ---------------------------------------------------------------------------------------------
// Multi-system LP (report §5.1, §5.3)

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GalaxyGoal {
    /// Fixed z cubes/min, fewest buildings.
    #[default]
    Target,
    /// Largest z under the building cap and / or the Dyson (photon) cap.
    Max,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OilRate {
    /// Initial seep rate × speed (an upper bound; seeps decay, report §5.5).
    #[default]
    Initial,
    /// Depleted seeps: 0.1/s × speed per seep (long-run floor, report §5.1, 存疑).
    Floor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GalaxyPlanSettings {
    pub seed: SeedSettings,
    pub plan: PlanSettings,
    pub goal: GalaxyGoal,
    /// Target z (cubes/min) for `Target`.
    pub target: f64,
    /// Objective under `Target`: "buildings" (default) or "photons" (fewest photons, then buildings).
    pub objective: String,
    /// Level being researched (L−1 done): quotas use S(L), mining speed 1 + 0.1(L−1).
    pub level: u32,
    /// Cubes per level factor: 1, or 0.8 with sprayed research (report §3, 存疑).
    pub rho: f64,
    /// Share of cubes not spent on vein utilisation (report §4.1: quota ÷ (1−φ)).
    pub phi: f64,
    /// Apply the finite-stock quotas (off = rates only, for comparison).
    pub stock_quotas: bool,
    pub remaining: f64,
    pub stock_override: HashMap<String, f64>,
    /// Active star indices (empty = automatic pick, `auto_systems`).
    pub systems: Vec<usize>,
    pub oil: OilRate,
    /// Total buildings cap (production + miners + pumps + oil extractors + collectors + receivers + stations).
    pub building_cap: Option<f64>,
    /// Ray-receiver power available, GW, over all systems (Dyson spheres; not computed from the seed).
    pub photon_gw: Option<f64>,
    /// Items per logistics vessel trip; warpers per item shipped = `warpers_per_trip` / capacity.
    pub vessel_capacity: f64,
    pub warpers_per_trip: f64,
    /// Interstellar station capacity A / (d + d0) items/min per station at d ly; stations at both ends.
    /// MINE fit to 笑笑 (BV 18, 76:44–88:32): "7 光年内可达 4 条蓝带，12 光年 3 条蓝带" with his belt of
    /// that episode, "五条蓝带运力=5×7200" (four-stacked, 7200/min): 28 800/min at 7 ly, 21 600 at 12 ly
    /// → A = 432 000, d0 = 8 (review 8, M1: 108 000 took the belt as 1800/min, four times too many stations).
    pub station_a: f64,
    pub station_d0: f64,
    pub dark_fog: DarkFog,
}

impl Default for GalaxyPlanSettings {
    fn default() -> Self {
        let t = ThresholdSettings::default();
        Self {
            seed: SeedSettings::default(),
            plan: t.plan,
            goal: GalaxyGoal::Target,
            target: 1000.0,
            objective: "buildings".into(),
            level: 81,
            rho: 1.0,
            phi: 0.0,
            stock_quotas: true,
            remaining: 1.0,
            stock_override: HashMap::new(),
            systems: vec![],
            oil: OilRate::Initial,
            building_cap: None,
            photon_gw: None,
            vessel_capacity: 400.0,
            warpers_per_trip: 2.0,
            station_a: 432_000.0,
            station_d0: 8.0,
            dark_fog: DarkFog::default(),
        }
    }
}

/// At most this many systems in one LP (pairwise shipping grows with the square).
pub const MAX_SYSTEMS: usize = 12;

/// Resource kinds a star system offers: vein items, gas items, ocean items.
fn kinds(st: &StarInfo) -> BTreeSet<String> {
    let mut k = BTreeSet::new();
    for p in &st.planets {
        for v in &p.veins {
            k.insert(v.item.clone());
        }
        for gr in &p.gases {
            k.insert(format!("gas:{}", gr.item));
        }
        if let Some(o) = &p.ocean {
            k.insert(o.clone());
        }
    }
    k
}

/// MINE: default systems = birth star, then nearest-first every star that adds a resource kind not yet
/// covered (veins, gas, oceans), at most 8. The user can add or remove systems.
pub fn auto_systems(g: &Galaxy) -> Vec<usize> {
    let mut order: Vec<&StarInfo> = g.stars.iter().collect();
    order.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
    let mut covered = BTreeSet::new();
    let mut out = vec![];
    for st in order {
        let k = kinds(st);
        if out.is_empty() || k.iter().any(|x| !covered.contains(x)) {
            out.push(st.index);
            covered.extend(k);
        }
        if out.len() >= 8 {
            break;
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysRecipe {
    pub recipe: String,
    pub mode: String,
    pub building: String,
    pub machines: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extraction {
    pub item: String,
    pub rate: f64,
    /// Upper bound per minute at this z (stock quota or rate cap), if any.
    pub cap: Option<f64>,
    /// "quota" | "rate" | "" — which limit applies.
    pub limit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResult {
    pub star: usize,
    pub name: String,
    pub distance: f64,
    pub cubes: f64,
    pub production_buildings: f64,
    pub miners: f64,
    pub pumps: f64,
    pub oil_extractors: f64,
    pub collectors: f64,
    pub receivers: f64,
    pub stations: f64,
    pub power_mw: f64,
    pub recipes: Vec<SysRecipe>,
    pub extraction: Vec<Extraction>,
    pub drops: Vec<ItemRate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shipment {
    pub item: String,
    pub from: usize,
    pub to: usize,
    pub rate: f64,
    pub ly: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RareUse {
    pub item: String,
    /// Mined per cube over all systems.
    pub per_cube: f64,
    /// Quota per cube over the active systems: Σ B / (ρ k S(L)) · (1−φ).
    pub quota_per_cube: f64,
    pub binding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GalaxyPlan {
    pub status: Status,
    pub z: f64,
    pub level: u32,
    pub speed: f64,
    pub s_level: f64,
    pub buildings: f64,
    pub production_buildings: f64,
    pub miners: f64,
    pub pumps: f64,
    pub oil_extractors: f64,
    pub collectors: f64,
    pub receivers: f64,
    pub stations: f64,
    pub photons_per_min: f64,
    /// Ray-receiver power for the photons: photons × 1200 MJ.
    pub receiver_gw: f64,
    pub warpers_per_min: f64,
    pub power_mw: f64,
    pub systems: Vec<SystemResult>,
    pub shipments: Vec<Shipment>,
    pub rares: Vec<RareUse>,
    pub raw_per_cube: Vec<ItemRate>,
    pub columns: usize,
    pub rows: usize,
    pub solve_ms: f64,
    pub warnings: Vec<String>,
}

struct UnitCol<'a> {
    recipe: &'a Recipe,
    mode: Mode,
    /// Net per building per minute (outputs − inputs).
    net: Vec<(String, f64)>,
}

fn unit_cols<'a>(data: &'a GameData, s: &PlanSettings, targets: &[&str]) -> Vec<UnitCol<'a>> {
    let mut all = vec![];
    for r in allowed_recipes(data, s) {
        for m in rules::modes(data, r, s) {
            let mut net: BTreeMap<String, f64> = BTreeMap::new();
            for (i, q) in r.inputs.iter().chain(&m.extra_inputs) {
                *net.entry(i.clone()).or_default() -= q * m.speed;
            }
            for (i, q) in rules::outputs(r, &m).iter().chain(&m.extra_outputs) {
                *net.entry(i.clone()).or_default() += q * m.speed;
            }
            all.push(UnitCol { recipe: r, mode: m, net: net.into_iter().filter(|(_, v)| v.abs() > 1e-12).collect() });
        }
    }
    // upstream closure of the targets
    let mut needed: BTreeSet<String> = targets.iter().map(|t| t.to_string()).collect();
    let mut chosen = vec![false; all.len()];
    let mut queue: Vec<String> = needed.iter().cloned().collect();
    while let Some(it) = queue.pop() {
        for (k, u) in all.iter().enumerate() {
            if chosen[k] || !u.net.iter().any(|(i, q)| *i == it && *q > 0.0) {
                continue;
            }
            chosen[k] = true;
            for (i, q) in &u.net {
                if *q < 0.0 && needed.insert(i.clone()) {
                    queue.push(i.clone());
                }
            }
        }
    }
    all.into_iter().zip(chosen).filter(|(_, c)| *c).map(|(u, _)| u).collect()
}

fn dist(a: &StarInfo, b: &StarInfo) -> f64 {
    let d: f64 = (0..3).map(|k| (a.position[k] - b.position[k]).powi(2)).sum();
    d.sqrt()
}

pub fn plan_galaxy(data: &GameData, g: &Galaxy, gs: &GalaxyPlanSettings) -> GalaxyPlan {
    let t0 = web_time::Instant::now();
    let mut out = GalaxyPlan { level: gs.level.clamp(1, 10_000), ..Default::default() };
    let level = out.level;
    let speed = speed_at(level);
    out.speed = speed;
    out.s_level = s_of(level);
    let mut sys: Vec<usize> = if gs.systems.is_empty() { auto_systems(g) } else { gs.systems.clone() };
    sys.retain(|i| *i < g.stars.len());
    // a star listed twice would count its stock twice (`dedup` only drops neighbours)
    let mut seen = BTreeSet::new();
    sys.retain(|i| seen.insert(*i));
    if sys.len() > MAX_SYSTEMS {
        out.warnings.push(format!("{} systems chosen; only the first {MAX_SYSTEMS} are planned", sys.len()));
        sys.truncate(MAX_SYSTEMS);
    }
    if sys.is_empty() {
        out.status = Status::Infeasible;
        out.warnings.push("no star systems chosen".into());
        return out;
    }
    let stars: Vec<&StarInfo> = sys.iter().map(|i| &g.stars[*i]).collect();
    let mut ps = gs.plan.clone();
    ps.game = "dsp".into();
    let units = unit_cols(data, &ps, &[CUBE, WARPER]);
    let mut items: BTreeSet<String> = units.iter().flat_map(|u| u.net.iter().map(|(i, _)| i.clone())).collect();
    items.insert(CUBE.into());
    items.insert(WARPER.into());
    let items: Vec<String> = items.into_iter().collect();
    let idx: HashMap<&str, usize> = items.iter().enumerate().map(|(k, i)| (i.as_str(), k)).collect();
    let ns = stars.len();
    let mult = g.settings.resource_multiplier;
    let rho = if gs.rho.is_finite() && gs.rho > 0.0 { gs.rho } else { 1.0 };
    let phi = gs.phi.clamp(0.0, 0.95);
    let stocks_scale = gs.remaining.clamp(0.0, 1.0);
    let (drops, dw) = gs.dark_fog.rates(level);
    out.warnings.extend(dw);
    let target_mode = gs.goal == GalaxyGoal::Target;
    let photons_obj = target_mode && gs.objective == "photons";
    if !target_mode && gs.building_cap.is_none() && gs.photon_gw.is_none() {
        out.status = Status::Unbounded;
        out.warnings.push("maximum z needs a building cap or a Dyson (photon) power cap".into());
        return out;
    }
    let bld_w = if target_mode { if photons_obj { 1e-3 } else { 1.0 } } else { -1e-6 };
    // tie-break weights: costs, so negative when maximising z
    let tb = if target_mode { 1.0 } else { -1e-3 };
    let dir = if target_mode { OptimizationDirection::Minimize } else { OptimizationDirection::Maximize };
    let mut p = Problem::new(dir);
    let mut rows: Vec<(Vec<(Variable, f64)>, ComparisonOp, f64)> = vec![];
    // balance terms per (item, system)
    let mut bal: Vec<Vec<Vec<(Variable, f64)>>> = vec![vec![vec![]; ns]; items.len()];
    let mut bld_terms: Vec<(Variable, f64)> = vec![];
    let z = if target_mode {
        let t = if gs.target.is_finite() { gs.target.max(0.0) } else { 0.0 };
        if !gs.target.is_finite() {
            out.warnings.push("target is not a number: planned 0 cubes/min".into());
        }
        p.add_var(0.0, (t, t))
    } else {
        p.add_var(1.0, (0.0, f64::INFINITY))
    };
    // production
    let mut prod: Vec<(usize, usize, Variable)> = vec![];
    for s in 0..ns {
        for (k, u) in units.iter().enumerate() {
            let v = p.add_var(bld_w, (0.0, f64::INFINITY));
            for (i, q) in &u.net {
                bal[idx[i.as_str()]][s].push((v, *q));
            }
            bld_terms.push((v, 1.0));
            prod.push((s, k, v));
        }
    }
    // cubes consumed per system: Σ y_s = z
    let mut ys = vec![];
    for s in 0..ns {
        let y = p.add_var(0.0, (0.0, f64::INFINITY));
        bal[idx[CUBE]][s].push((y, -1.0));
        ys.push(y);
    }
    let mut t: Vec<(Variable, f64)> = ys.iter().map(|y| (*y, 1.0)).collect();
    t.push((z, -1.0));
    rows.push((t, ComparisonOp::Eq, 0.0));
    // extraction
    struct Ext {
        s: usize,
        item: String,
        v: Variable,
        per_bld: f64,
        kind: u8, // 0 miner, 1 pump, 2 receiver, 3 oil extractor, 4 drop
        quota: Option<f64>,
        rate_cap: Option<f64>,
    }
    let mut ext: Vec<Ext> = vec![];
    let miner_rate = data.resource("iron-ore").map(|r| r.extractor_rate).unwrap_or(60.0) * speed;
    let pump_rate = data.resource("water").map(|r| r.extractor_rate).unwrap_or(50.0) * speed;
    let receiver_rate = data.resource(PHOTON).map(|r| r.extractor_rate).unwrap_or(6.0);
    let mut collectors: Vec<(usize, Variable, Vec<(String, f64)>)> = vec![];
    let mut photon_vars = vec![];
    for (s, st) in stars.iter().enumerate() {
        let mut stock: BTreeMap<String, f64> = BTreeMap::new();
        let (mut oil_rate, mut seeps) = (0.0, 0);
        let mut oceans = BTreeSet::new();
        for pl in &st.planets {
            for v in &pl.veins {
                *stock.entry(v.item.clone()).or_default() += stock_of(v);
            }
            oil_rate += pl.oil_rate;
            seeps += pl.seeps;
            if let Some(o) = &pl.ocean {
                oceans.insert(o.clone());
            }
            if !pl.gases.is_empty() && pl.gases.iter().any(|gr| idx.contains_key(gr.item.as_str())) {
                let net: Vec<(String, f64)> = collector_net(&pl.gases, speed).into_iter().map(|(i, r)| (i, r * 60.0)).collect();
                if net.iter().any(|(_, r)| *r > 0.0) {
                    let c = p.add_var(bld_w, (0.0, COLLECTORS_PER_GIANT));
                    bld_terms.push((c, 1.0));
                    for (i, r) in &net {
                        if let Some(&k) = idx.get(i.as_str()) {
                            bal[k][s].push((c, *r));
                        }
                    }
                    collectors.push((s, c, net));
                }
            }
        }
        for (item, b) in &stock {
            if !idx.contains_key(item.as_str()) {
                continue;
            }
            let rate_cap = if item == "crude-oil" {
                Some(match gs.oil {
                    OilRate::Initial => oil_rate * 60.0 * speed,
                    OilRate::Floor => seeps as f64 * 0.1 * 60.0 * speed,
                })
            } else {
                None
            };
            // an override is the remaining galaxy stock itself (as in `thresholds`): this system's share of
            // it; otherwise the generated stock × the remaining share
            let b = match gs.stock_override.get(item).filter(|o| o.is_finite() && **o >= 0.0) {
                Some(o) => o * b / total_stock(g, item).max(1e-9),
                None => b * stocks_scale,
            };
            // oil: one extractor per seep, used in proportion to the rate taken (review 8, L3)
            let per_bld = if item == "crude-oil" {
                match rate_cap {
                    Some(c) if c > 0.0 => seeps as f64 / c,
                    _ => 0.0,
                }
            } else {
                1.0 / miner_rate
            };
            let v = p.add_var(bld_w * per_bld + tb * 1e-6, (0.0, rate_cap.unwrap_or(f64::INFINITY)));
            if per_bld > 0.0 {
                bld_terms.push((v, per_bld));
            }
            bal[idx[item.as_str()]][s].push((v, 1.0));
            let quota = if gs.stock_quotas && !g.infinite {
                let k = points_per_unit(item, mult);
                // m·ρ·k·S(L) ≤ B·(1−φ)·z
                rows.push((vec![(v, rho * k * s_of(level)), (z, -b * (1.0 - phi))], ComparisonOp::Le, 0.0));
                Some(b * (1.0 - phi) / (rho * k * s_of(level)))
            } else {
                None
            };
            ext.push(Ext { s, item: item.clone(), v, per_bld, kind: if item == "crude-oil" { 3 } else { 0 }, quota, rate_cap });
        }
        for o in &oceans {
            if let Some(&k) = idx.get(o.as_str()) {
                let v = p.add_var(bld_w / pump_rate + tb * 1e-6, (0.0, f64::INFINITY));
                bld_terms.push((v, 1.0 / pump_rate));
                bal[k][s].push((v, 1.0));
                ext.push(Ext { s, item: o.clone(), v, per_bld: 1.0 / pump_rate, kind: 1, quota: None, rate_cap: None });
            }
        }
        if let Some(&k) = idx.get(PHOTON) {
            let w = if photons_obj { 1.0 } else { tb * 1e-6 };
            let v = p.add_var(bld_w / receiver_rate + w, (0.0, f64::INFINITY));
            bld_terms.push((v, 1.0 / receiver_rate));
            bal[k][s].push((v, 1.0));
            photon_vars.push(v);
            ext.push(Ext { s, item: PHOTON.into(), v, per_bld: 1.0 / receiver_rate, kind: 2, quota: None, rate_cap: None });
        }
        // Dark Fog drops at farmed bases of this star
        let bases: f64 = gs.dark_fog.bases.iter().filter(|b| b.0 == st.index).map(|b| b.1.max(0.0)).sum();
        if bases > 0.0 {
            for (item, r) in &drops {
                if let Some(&k) = idx.get(item.as_str()) {
                    let v = p.add_var(0.0, (0.0, r * bases));
                    bal[k][s].push((v, 1.0));
                    ext.push(Ext { s, item: item.clone(), v, per_bld: 0.0, kind: 4, quota: None, rate_cap: Some(r * bases) });
                }
            }
        }
    }
    if let Some(gw) = gs.photon_gw {
        let cap = gw.max(0.0) * 1000.0 * 60.0 / PHOTON_MJ;
        rows.push((photon_vars.iter().map(|v| (*v, 1.0)).collect(), ComparisonOp::Le, cap));
    }
    // shipping
    let mut ships: Vec<(usize, usize, usize, Variable, f64)> = vec![];
    let warper_k = idx[WARPER];
    let wpi = gs.warpers_per_trip.max(0.0) / gs.vessel_capacity.max(1.0);
    for a in 0..ns {
        for b in 0..ns {
            if a == b {
                continue;
            }
            let d = dist(stars[a], stars[b]);
            let st_per = 2.0 * (d + gs.station_d0.max(0.0)) / gs.station_a.max(1.0);
            for (k, _) in items.iter().enumerate() {
                let v = p.add_var(bld_w * st_per + tb * 1e-7 * d, (0.0, f64::INFINITY));
                bal[k][a].push((v, -1.0));
                bal[k][b].push((v, 1.0));
                bal[warper_k][a].push((v, -wpi));
                bld_terms.push((v, st_per));
                ships.push((a, b, k, v, d));
            }
        }
    }
    if let Some(cap) = gs.building_cap {
        rows.push((bld_terms.clone(), ComparisonOp::Le, cap.max(0.0)));
    }
    let mut nrows = rows.len();
    for (k, per) in bal.iter().enumerate() {
        for terms in per {
            if terms.is_empty() {
                continue;
            }
            let _ = k;
            // a warper flow appears twice in the warper row of its origin (shipped and burnt): merge
            let mut merged: BTreeMap<usize, (Variable, f64)> = BTreeMap::new();
            for (v, c) in terms {
                merged.entry(v.idx()).or_insert((*v, 0.0)).1 += c;
            }
            let terms: Vec<(Variable, f64)> = merged.into_values().collect();
            p.add_constraint(terms.as_slice(), ComparisonOp::Ge, 0.0);
            nrows += 1;
        }
    }
    for (terms, op, rhs) in &rows {
        p.add_constraint(terms.as_slice(), *op, *rhs);
    }
    out.columns = prod.len() + ext.len() + ships.len() + collectors.len() + ys.len() + 1;
    out.rows = nrows;
    let sol = match p.solve().map(|o| o.into_solution()) {
        Ok(Ok(s)) => s,
        Ok(Err(_)) => {
            out.status = Status::Infeasible;
            out.warnings.push("solver stopped before finding a plan".into());
            return out;
        }
        Err(microlp::Error::Infeasible) => {
            out.status = Status::Infeasible;
            out.warnings.push(infeasible_hint(gs, g, &sys));
            out.solve_ms = t0.elapsed().as_secs_f64() * 1e3;
            return out;
        }
        Err(microlp::Error::Unbounded) => {
            out.status = Status::Unbounded;
            out.warnings.push("unbounded: add a building or photon cap".into());
            return out;
        }
        Err(e) => {
            out.status = Status::Infeasible;
            out.warnings.push(format!("solver: {e}"));
            return out;
        }
    };
    out.status = Status::Optimal;
    let zv = sol[z];
    out.z = zv;
    let mut systems: Vec<SystemResult> = stars
        .iter()
        .enumerate()
        .map(|(s, st)| SystemResult {
            star: st.index,
            name: st.name.clone(),
            distance: st.distance,
            cubes: sol[ys[s]],
            production_buildings: 0.0,
            miners: 0.0,
            pumps: 0.0,
            oil_extractors: 0.0,
            collectors: 0.0,
            receivers: 0.0,
            stations: 0.0,
            power_mw: 0.0,
            recipes: vec![],
            extraction: vec![],
            drops: vec![],
        })
        .collect();
    for (s, k, v) in &prod {
        let n = sol[*v];
        if n > 1e-7 {
            let u = &units[*k];
            let sr = &mut systems[*s];
            sr.production_buildings += n;
            sr.power_mw += n * u.mode.power_base;
            sr.recipes.push(SysRecipe {
                recipe: u.recipe.id.clone(),
                mode: u.mode.label.clone(),
                building: u.mode.building.clone(),
                machines: n,
            });
        }
    }
    let mut mined: BTreeMap<String, f64> = BTreeMap::new();
    let mut quota_sum: BTreeMap<String, f64> = BTreeMap::new();
    for e in &ext {
        let r = sol[e.v];
        if e.kind != 4 {
            if let Some(q) = e.quota {
                *quota_sum.entry(e.item.clone()).or_default() += q;
            }
        }
        if r <= 1e-7 {
            continue;
        }
        let sr = &mut systems[e.s];
        match e.kind {
            0 => sr.miners += r * e.per_bld,
            1 => sr.pumps += r * e.per_bld,
            2 => {
                sr.receivers += r * e.per_bld;
                out.photons_per_min += r;
            }
            3 => sr.oil_extractors += r * e.per_bld,
            _ => {}
        }
        if e.kind == 4 {
            sr.drops.push(ItemRate { item: e.item.clone(), rate: r });
            continue;
        }
        *mined.entry(e.item.clone()).or_default() += r;
        let (cap, limit) = match (e.quota.map(|q| q * zv), e.rate_cap) {
            (Some(q), Some(c)) if c < q => (Some(c), "rate"),
            (Some(q), _) => (Some(q), "quota"),
            (None, Some(c)) => (Some(c), "rate"),
            _ => (None, ""),
        };
        sr.extraction.push(Extraction { item: e.item.clone(), rate: r, cap, limit: limit.into() });
    }
    for (s, c, net) in &collectors {
        let n = sol[*c];
        if n > 1e-7 {
            systems[*s].collectors += n;
            for (i, r) in net {
                *mined.entry(i.clone()).or_default() += n * r;
            }
            for (i, r) in net {
                systems[*s].extraction.push(Extraction { item: format!("{i} (gas)"), rate: n * r, cap: Some(COLLECTORS_PER_GIANT * r), limit: "rate".into() });
            }
        }
    }
    for (a, b, k, v, d) in &ships {
        let r = sol[*v];
        if r > 1e-6 {
            let st_per = 2.0 * (d + gs.station_d0.max(0.0)) / gs.station_a.max(1.0);
            systems[*a].stations += r * st_per / 2.0;
            systems[*b].stations += r * st_per / 2.0;
            out.warpers_per_min += r * wpi;
            out.shipments.push(Shipment { item: items[*k].clone(), from: stars[*a].index, to: stars[*b].index, rate: r, ly: *d });
        }
    }
    out.shipments.sort_by(|x, y| (y.rate * y.ly).partial_cmp(&(x.rate * x.ly)).unwrap());
    for sr in &mut systems {
        sr.recipes.sort_by(|a, b| b.machines.partial_cmp(&a.machines).unwrap());
        out.production_buildings += sr.production_buildings;
        out.miners += sr.miners;
        out.pumps += sr.pumps;
        out.oil_extractors += sr.oil_extractors;
        out.collectors += sr.collectors;
        out.receivers += sr.receivers;
        out.stations += sr.stations;
        out.power_mw += sr.power_mw;
    }
    out.buildings =
        out.production_buildings + out.miners + out.pumps + out.oil_extractors + out.collectors + out.receivers + out.stations;
    out.receiver_gw = out.photons_per_min * PHOTON_MJ / 60.0 / 1000.0;
    if zv > 1e-9 {
        out.raw_per_cube = mined.iter().map(|(i, r)| ItemRate { item: i.clone(), rate: r / zv }).collect();
        out.rares = FINITE
            .iter()
            .filter_map(|it| {
                let q = quota_sum.get(*it)?;
                let m = mined.get(*it).copied().unwrap_or(0.0);
                Some(RareUse { item: it.to_string(), per_cube: m / zv, quota_per_cube: *q, binding: m / zv >= q * (1.0 - 1e-6) - 1e-9 && m > 0.0 })
            })
            .collect();
    }
    out.systems = systems;
    out.solve_ms = t0.elapsed().as_secs_f64() * 1e3;
    out
}

fn total_stock(g: &Galaxy, item: &str) -> f64 {
    g.totals.iter().find(|l| l.item == item).map(|l| l.stock).unwrap_or(0.0)
}

fn infeasible_hint(gs: &GalaxyPlanSettings, g: &Galaxy, sys: &[usize]) -> String {
    let mut have = BTreeSet::new();
    for i in sys {
        have.extend(kinds(&g.stars[*i]));
    }
    let missing: Vec<&str> = ["water", "sulfuric-acid", "gas:hydrogen", "crude-oil", "unipolar-magnet"]
        .into_iter()
        .filter(|k| !have.contains(*k))
        .collect();
    let mut s = format!("infeasible at L{} with these systems", gs.level);
    if !missing.is_empty() {
        s += &format!("; not available in them: {}", missing.join(", "));
    }
    if gs.stock_quotas {
        s += "; quotas may be too tight at this level (raise L, add systems, or check the threshold table)";
    }
    s
}

// ---------------------------------------------------------------------------------------------
// Galaxy total: how many cubes the galaxy's finite stock makes at best (Xinyu 2026-11-23:
// 「必须要求有一个 quota 是不合理的…机器和太阳帆的总数其实是不受限制的…这个星系的所有物质够我最优解的生产白糖能生产多少」)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TotalSettings {
    pub seed: SeedSettings,
    /// Recipe settings from the sidebar (alternates, proliferator); targets are set by the model.
    pub plan: PlanSettings,
    /// Vein-utilisation levels done (stock consumed per unit mined = 0.94^done); 0 = none.
    pub levels_done: u32,
    pub remaining: f64,
    pub stock_override: HashMap<String, f64>,
    /// Star indices whose stock counts (empty = every star of the galaxy).
    pub systems: Vec<usize>,
    /// Charge interstellar warpers for ore mined outside the hub (off: vessels fly without warpers,
    /// slower, which costs nothing when time is unlimited).
    pub warpers: bool,
    pub vessel_capacity: f64,
    pub warpers_per_trip: f64,
}

impl Default for TotalSettings {
    fn default() -> Self {
        Self {
            seed: SeedSettings::default(),
            plan: ThresholdSettings::default().plan,
            levels_done: 0,
            remaining: 1.0,
            stock_override: HashMap::new(),
            systems: vec![],
            warpers: false,
            vessel_capacity: 400.0,
            warpers_per_trip: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalUse {
    pub item: String,
    /// Units minable: stock points / points per unit / 0.94^done.
    pub available: f64,
    pub used: f64,
    pub per_cube: f64,
    /// All of it used and more of it would give more cubes (marginal > 0).
    pub binding: bool,
    /// Extra cubes per extra unit of this stock (0 when not binding).
    pub marginal: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalRecipe {
    pub recipe: String,
    pub mode: String,
    pub building: String,
    /// Machines per 1 cube/min.
    pub machines_per_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TotalReport {
    pub status: Status,
    /// Total cubes over the life of the stock.
    pub cubes: f64,
    /// The same with no vein utilisation (the total scales with 1/0.94^done).
    pub cubes_no_util: f64,
    pub levels_done: u32,
    pub stars: usize,
    pub hub: Option<String>,
    pub warpers: f64,
    pub finite: Vec<TotalUse>,
    /// Sources taken as unlimited over unlimited time (oceans, gas giants, oil seep floor, photons).
    pub unlimited: Vec<String>,
    pub raw_per_cube: Vec<ItemRate>,
    pub recipes: Vec<TotalRecipe>,
    pub solve_ms: f64,
    pub warnings: Vec<String>,
}

type TotalSol = (f64, Vec<f64>, Vec<(f64, f64)>);

/// One LP over totals (not rates): maximise cubes N with every finite vein used at most once.
/// Machines, stations, sails and time are unlimited, so rate-limited sources (oceans, gas giants,
/// oil seeps at their floor, Dyson photons) are unlimited in total and only finite veins bind.
/// `bump` multiplies one item's stock (for marginals).
fn total_lp(
    units: &[UnitCol],
    avail: &BTreeMap<String, (f64, f64)>,
    unlimited: &BTreeSet<String>,
    wpi: f64,
    bump: Option<(&str, f64)>,
) -> Result<TotalSol, String> {
    let mut items: BTreeSet<String> = units.iter().flat_map(|u| u.net.iter().map(|(i, _)| i.clone())).collect();
    items.insert(CUBE.into());
    items.insert(WARPER.into());
    let items: Vec<String> = items.into_iter().collect();
    let idx: HashMap<&str, usize> = items.iter().enumerate().map(|(k, i)| (i.as_str(), k)).collect();
    let mut p = Problem::new(OptimizationDirection::Maximize);
    let mut bal: Vec<Vec<(Variable, f64)>> = vec![vec![]; items.len()];
    let n = p.add_var(1.0, (0.0, f64::INFINITY));
    bal[idx[CUBE]].push((n, -1.0));
    // MINE: a 1e-9 cost per machine-minute picks the leanest of equally many cubes (totals are ~1e6+)
    let xs: Vec<Variable> = units
        .iter()
        .map(|u| {
            let v = p.add_var(-1e-9, (0.0, f64::INFINITY));
            for (i, q) in &u.net {
                bal[idx[i.as_str()]].push((v, *q));
            }
            v
        })
        .collect();
    // finite stock: (hub part, elsewhere part); elsewhere costs warpers to ship
    let mut mined = vec![];
    for (item, (hub, away)) in avail {
        let Some(&k) = idx.get(item.as_str()) else {
            mined.push(None);
            continue;
        };
        let f = match bump {
            Some((b, s)) if b == item => s,
            _ => 1.0,
        };
        // MINE: 1e-10 per unit mined so stock that adds no cube stays in the ground (else "used" is noise)
        let h = p.add_var(-1e-10, (0.0, hub * f));
        let a = p.add_var(-1e-10, (0.0, away * f));
        bal[k].push((h, 1.0));
        bal[k].push((a, 1.0));
        if wpi > 0.0 {
            bal[idx[WARPER]].push((a, -wpi));
        }
        mined.push(Some((h, a)));
    }
    for it in unlimited {
        if let Some(&k) = idx.get(it.as_str()) {
            let v = p.add_var(-1e-12, (0.0, f64::INFINITY));
            bal[k].push((v, 1.0));
        }
    }
    for terms in &bal {
        if !terms.is_empty() {
            let mut merged: BTreeMap<usize, (Variable, f64)> = BTreeMap::new();
            for (v, c) in terms {
                merged.entry(v.idx()).or_insert((*v, 0.0)).1 += c;
            }
            let t: Vec<(Variable, f64)> = merged.into_values().collect();
            p.add_constraint(t.as_slice(), ComparisonOp::Ge, 0.0);
        }
    }
    let sol = match p.solve() {
        Ok(o) => o.into_solution().map_err(|_| "solver stopped before finding a plan".to_string())?,
        Err(microlp::Error::Unbounded) => return Err("unbounded".into()),
        Err(e) => return Err(e.to_string()),
    };
    let xv = xs.iter().map(|v| sol[*v]).collect();
    let mv = mined.iter().map(|m| m.map(|(h, a)| (sol[h], sol[a])).unwrap_or((0.0, 0.0))).collect();
    Ok((sol[n], xv, mv))
}

pub fn galaxy_total(data: &GameData, g: &Galaxy, ts: &TotalSettings) -> TotalReport {
    let t0 = web_time::Instant::now();
    // above ~500 levels 0.94^done underflows the bounds; the UI caps at 500 too
    let mut out = TotalReport { levels_done: ts.levels_done.min(500), ..Default::default() };
    let mut sys: Vec<usize> = if ts.systems.is_empty() { (0..g.stars.len()).collect() } else { ts.systems.clone() };
    sys.retain(|i| *i < g.stars.len());
    sys.sort();
    sys.dedup();
    out.stars = sys.len();
    if sys.is_empty() {
        out.status = Status::Infeasible;
        out.warnings.push("no star systems chosen".into());
        return out;
    }
    if g.infinite {
        out.status = Status::Unbounded;
        out.warnings.push("infinite resources: the total is unlimited".into());
        return out;
    }
    let mut ps = ts.plan.clone();
    ps.game = "dsp".into();
    let mut units = unit_cols(data, &ps, &[CUBE, WARPER]);
    // Dark Fog items, logs and plant fuel have no source here (drops are a rate, not a stock: left out)
    units.retain(|u| u.net.iter().all(|(i, q)| *q > 0.0 || !(i.starts_with("df-") || i == "log" || i == "plant-fuel")));
    let mult = g.settings.resource_multiplier;
    let consume = Q.powi(out.levels_done as i32);
    let scale = ts.remaining.clamp(0.0, 1.0);
    // unlimited over unlimited time: oceans, gas giants, oil seeps (never mined below the floor), photons
    let mut unlimited: BTreeSet<String> = [PHOTON.to_string()].into();
    let mut stock: BTreeMap<String, Vec<(usize, f64)>> = BTreeMap::new();
    for &i in &sys {
        for pl in &g.stars[i].planets {
            if let Some(o) = &pl.ocean {
                unlimited.insert(o.clone());
            }
            for gr in &pl.gases {
                if gr.base > 0.0 {
                    unlimited.insert(gr.item.clone());
                }
            }
            if pl.seeps > 0 {
                unlimited.insert("crude-oil".into());
            }
            for v in &pl.veins {
                stock.entry(v.item.clone()).or_default().push((i, stock_of(v)));
            }
        }
    }
    // stock split (hub, elsewhere), at level 0: the total at `done` levels is exactly 1/0.94^done times
    // that (every finite bound scales alike), and level-0 bounds keep the LP well scaled
    let avail_for = |hub: Option<usize>| -> BTreeMap<String, (f64, f64)> {
        let mut avail: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        for (it, v) in &stock {
            if unlimited.contains(it) {
                continue;
            }
            let k = points_per_unit(it, mult);
            // an override is the remaining galaxy stock itself (as in `thresholds`), spread over the
            // stars in proportion to their stock; with a subset of stars, that subset's share of it
            let over = ts.stock_override.get(it).filter(|o| o.is_finite() && **o >= 0.0).map(|o| o / total_stock(g, it).max(1e-9));
            let e = avail.entry(it.clone()).or_default();
            for (s, b) in v {
                let b = b * over.unwrap_or(scale) / k;
                if hub.is_none_or(|h| h == *s) {
                    e.0 += b;
                } else {
                    e.1 += b;
                }
            }
        }
        avail
    };
    let wpi = if ts.warpers { ts.warpers_per_trip.max(0.0) / ts.vessel_capacity.max(1.0) } else { 0.0 };
    // hub: with warpers, every system is tried and the one giving the most cubes kept
    let hubs: Vec<Option<usize>> = if wpi > 0.0 { sys.iter().map(|s| Some(*s)).collect() } else { vec![None] };
    let mut best: Option<(Option<usize>, BTreeMap<String, (f64, f64)>, TotalSol)> = None;
    let mut err = None;
    for h in hubs {
        let avail = avail_for(h);
        match total_lp(&units, &avail, &unlimited, wpi, None) {
            Ok(r) => {
                if best.as_ref().is_none_or(|b| r.0 > b.2 .0) {
                    best = Some((h, avail, r));
                }
            }
            Err(e) => err = Some(e),
        }
    }
    let res = match best {
        Some(b) => Ok(b),
        None => Err(err.unwrap_or_default()),
    };
    let (hub, avail, (n, xv, mv)) = match res {
        Ok(r) => r,
        Err(e) if e == "unbounded" => {
            out.status = Status::Unbounded;
            out.warnings.push("unbounded: cubes can be made from unlimited sources alone".into());
            return out;
        }
        Err(e) => {
            out.status = Status::Infeasible;
            out.warnings.push(format!("solver: {e}"));
            return out;
        }
    };
    out.status = Status::Optimal;
    out.hub = hub.map(|h| g.stars[h].name.clone());
    out.cubes = n / consume;
    out.cubes_no_util = n;
    if n <= 1e-9 {
        out.warnings.push("no cubes: a needed finite resource is missing from these systems".into());
    }
    for (k, (it, (ah, aa))) in avail.iter().enumerate() {
        let (h, a) = mv[k];
        out.warpers += a * wpi;
        let (used, available) = ((h + a) / consume, (ah + aa) / consume);
        let binding = n > 1e-9 && available > 0.0 && used >= available * (1.0 - 1e-6);
        out.finite.push(TotalUse { item: it.clone(), available, used, per_cube: if n > 1e-9 { (h + a) / n } else { 0.0 }, binding, marginal: 0.0 });
    }
    // marginal cubes per unit: re-solve with that stock 1 % larger
    // (cubes and stock both scale by 1/consume, so the ratio is the level-0 one)
    for u in out.finite.iter_mut().filter(|u| u.binding) {
        if let Ok((n2, _, _)) = total_lp(&units, &avail, &unlimited, wpi, Some((&u.item, 1.01))) {
            u.marginal = ((n2 - n) / (0.01 * u.available * consume)).max(0.0);
        }
        // fully used but more gives nothing (a degenerate vertex): not a limit
        u.binding = u.marginal > 1e-9;
    }
    if n > 1e-9 {
        let mut raw: BTreeMap<String, f64> = BTreeMap::new();
        for u in out.finite.iter().filter(|u| u.used > 0.0) {
            raw.insert(u.item.clone(), u.per_cube);
        }
        for (k, u) in units.iter().enumerate() {
            let x = xv[k];
            if x <= 1e-9 * n {
                continue;
            }
            out.recipes.push(TotalRecipe {
                recipe: u.recipe.id.clone(),
                mode: u.mode.label.clone(),
                building: u.mode.building.clone(),
                machines_per_rate: x / n,
            });
            // unlimited raw items: what the recipes take of them, gross (MINE, for reading only)
            for (i, q) in &u.net {
                if *q < 0.0 && unlimited.contains(i) {
                    *raw.entry(i.clone()).or_default() += -q * x / n;
                }
            }
        }
        out.raw_per_cube = raw.into_iter().map(|(item, rate)| ItemRate { item, rate }).collect();
        out.recipes.sort_by(|a, b| b.machines_per_rate.partial_cmp(&a.machines_per_rate).unwrap());
    }
    out.unlimited = unlimited.into_iter().collect();
    out.solve_ms = t0.elapsed().as_secs_f64() * 1e3;
    out
}
