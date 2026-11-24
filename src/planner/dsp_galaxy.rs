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
pub const SAIL: &str = "solar-sail";
/// Swarm sail: 36 kW × luminosity (dsp-wiki Dyson_Sphere, player-measured).
pub const SAIL_KW: f64 = 36.0;
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
                warn.push(format!("黑雾：{} 要黑雾 {lv} 级才掉（现在设的是 {} 级），不计", d.item, self.level));
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
        warnings.push(format!("不限配额的方案：{:?} {}", free.status, free.warnings.join("；")));
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
        out.warnings.push(format!("选了 {} 个星系，只规划前 {MAX_SYSTEMS} 个", sys.len()));
        sys.truncate(MAX_SYSTEMS);
    }
    if sys.is_empty() {
        out.status = Status::Infeasible;
        out.warnings.push("没有选星系".into());
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
        out.warnings.push("求最大白糖需要建筑上限或戴森球功率上限".into());
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
            out.warnings.push("目标不是数字：按 0 白糖/分钟规划".into());
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
            out.warnings.push("求解器还没找到方案就停了".into());
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
            out.warnings.push("无上限：请设建筑上限或戴森球功率上限".into());
            return out;
        }
        Err(e) => {
            out.status = Status::Infeasible;
            out.warnings.push(format!("求解器：{e}"));
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
    let mut s = format!("在 {} 级、这些星系下无解", gs.level);
    if !missing.is_empty() {
        s += &format!("；这些星系里没有：{}", missing.join("、"));
    }
    if gs.stock_quotas {
        s += "；这一级的配额可能太紧（提高等级、加星系，或看无限阈值表）";
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
    /// Count the ore spent on further vein-utilisation research (Xinyu 2026-11-23: 「之后的 improvement
    /// 可以是考虑一下我在爬科技的时候需要消耗的矿石」). `levels_done` are done already (sunk; the stock is
    /// what is left now); researching on to d costs Σ_{n=done+1..d} 4000(n−5) cubes made at 0.94^(n−1),
    /// in level-0 cubes ρ·(S(done+1) − S(d+1)) (report §4.1). MINE: levels 1–5 and every other tech are
    /// not counted (they use lower matrices), nor the spray for ρ = 0.8.
    pub research: bool,
    /// Cubes per research level factor: 1, or 0.8 with sprayed labs.
    pub rho: f64,
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
            research: true,
            rho: 1.0,
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
    /// The stock never runs out if every cube goes into research from now on (ρ·S(done+1) ≤ level-0
    /// cubes): the total has no limit (笑笑 BV1QF24YDE7H 03:22).
    pub infinite: bool,
    /// Level-0 cubes that researching from now to infinity costs: ρ·S(done+1).
    pub research_need: f64,
    /// Finite case: research on to this many levels done, then produce; the cubes left for production
    /// and the cubes spent on that research.
    pub best_levels_done: u32,
    pub best_cubes: f64,
    pub research_cubes: f64,
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
    extra: &[Vec<(String, f64)>],
) -> Result<TotalSol, String> {
    let mut items: BTreeSet<String> = units.iter().flat_map(|u| u.net.iter().map(|(i, _)| i.clone())).collect();
    items.extend(extra.iter().flat_map(|c| c.iter().map(|(i, _)| i.clone())));
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
    // extra columns (the Dyson swarm: sails in, photons out), no cost
    let ev: Vec<Variable> = extra
        .iter()
        .map(|c| {
            let v = p.add_var(0.0, (0.0, f64::INFINITY));
            for (i, q) in c {
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
    // machine counts, then the extra columns' values
    let xv = xs.iter().chain(&ev).map(|v| sol[*v]).collect();
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
        out.warnings.push("没有选星系".into());
        return out;
    }
    if g.infinite {
        out.status = Status::Unbounded;
        out.warnings.push("资源无限：总量没有上限".into());
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
        match total_lp(&units, &avail, &unlimited, wpi, None, &[]) {
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
            out.warnings.push("无上限：只用无限的来源就能造白糖".into());
            return out;
        }
        Err(e) => {
            out.status = Status::Infeasible;
            out.warnings.push(format!("求解器：{e}"));
            return out;
        }
    };
    out.status = Status::Optimal;
    out.hub = hub.map(|h| g.stars[h].name.clone());
    out.cubes = n / consume;
    out.cubes_no_util = n;
    if ts.research {
        let rho = if ts.rho.is_finite() && ts.rho > 0.0 { ts.rho } else { 1.0 };
        let d0 = out.levels_done;
        // research of levels 1..=5 uses lower matrices: not in S (s_of clamps L < 6 to 6)
        let s_from = |d: u32| if d + 1 >= 6 { s_of(d + 1) } else { s_of(6) };
        // cubes left for production after researching on from d0 to d, made at 0.94^d each
        let left = |d: u32| -> f64 { (n - rho * (s_from(d0) - s_from(d))) / Q.powi(d as i32) };
        out.research_need = rho * s_from(d0);
        out.infinite = n >= out.research_need;
        let (bd, bc) = (d0..=500u32.max(d0)).map(|d| (d, left(d))).fold((d0, left(d0)), |b, x| if x.1 > b.1 { x } else { b });
        out.best_levels_done = bd;
        out.best_cubes = bc;
        let reach = |d: u32| if d >= 6 { cubes_to_reach(d + 1, rho) } else { 0.0 };
        out.research_cubes = reach(bd) - reach(d0);
    }
    if n <= 1e-9 {
        out.warnings.push("造不出白糖：这些星系缺一种必需的有限资源".into());
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
        if let Ok((n2, _, _)) = total_lp(&units, &avail, &unlimited, wpi, Some((&u.item, 1.01)), &[]) {
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

// ---------------------------------------------------------------------------------------------
// Highest cube rate (Xinyu 2026-11-23: 「你算出来了星系资源，你就知道最多戴森球能量有多少。限制的就只有
// 星球的资源产出速率。参考笑笑的视频，在矿物利用跑到一定程度就无限使用了，所以速率决定最高白糖速率」).
// Past the infinite threshold the stock never runs out (笑笑 BV1QF24YDE7H 03:22–04:08), so the cube
// rate is bounded only by extraction rates at the vein-utilisation level and by the Dyson spheres.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RateSettings {
    pub seed: SeedSettings,
    pub plan: PlanSettings,
    /// Vein-utilisation level being researched (L−1 done): mining speed 1 + 0.1(L−1) (BWIKI 矿物利用).
    pub level: u32,
    /// Star indices whose resources count (empty = every star).
    pub systems: Vec<usize>,
    /// Advanced mining machines per vein node. MINE default 1: overlapping fields are allowed in game
    /// (each machine mines the shared node in full, dsp-wiki Mining_Machine), but how many fit is a
    /// layout question.
    pub miners_per_node: f64,
    /// Advanced mining machine speed setting, 1.0–3.0 (100 %–300 %, more power).
    pub miner_speed: f64,
    pub oil: OilRate,
    /// Cap critical photons by the stars' full Dyson spheres (off = photons unlimited).
    pub dyson: bool,
    /// Share of each star's full sphere that is built, 0..1.
    pub dyson_share: f64,
    /// Dyson swarm on the brightest star: sails launched continuously, each lasting `sail_life_s` and
    /// giving 36 kW × luminosity (dsp-wiki Dyson_Sphere). Sails are unlimited (Xinyu 2026-11-23:
    /// 「机器和太阳帆的总数其实是不受限制的」), so the swarm is bounded by what the sails are made of.
    pub swarm: bool,
    /// Sail lifetime, s: 5400 base, 9000 with every upgrade (dsp-wiki; infinite tech → 9000).
    pub sail_life_s: f64,
    /// Cubes per research level factor for the infinite threshold (1, or 0.8 sprayed).
    pub rho: f64,
    /// Production buildings draw their power from the Dyson spheres too: power_base MW per machine
    /// = MW/20 photons per minute (1200 MJ per photon). MINE: miners, pumps and logistics are not counted.
    pub power: bool,
    pub warpers: bool,
    pub vessel_capacity: f64,
    pub warpers_per_trip: f64,
}

impl Default for RateSettings {
    fn default() -> Self {
        Self {
            seed: SeedSettings::default(),
            plan: ThresholdSettings::default().plan,
            level: 81,
            systems: vec![],
            miners_per_node: 1.0,
            miner_speed: 1.0,
            oil: OilRate::Initial,
            dyson: true,
            dyson_share: 1.0,
            swarm: true,
            sail_life_s: 9000.0,
            rho: 1.0,
            power: true,
            warpers: false,
            vessel_capacity: 400.0,
            warpers_per_trip: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateUse {
    pub item: String,
    /// "vein" | "oil" | "gas" | "dyson"
    pub source: String,
    /// Most per minute the chosen stars give at this level.
    pub cap: f64,
    pub used: f64,
    pub per_cube: f64,
    /// Fully used and more of it gives more cubes.
    pub binding: bool,
    /// Extra cubes/min per extra 1/min of it.
    pub marginal: f64,
    /// Finite stock: first level from which this use per cube never runs out (None: never; gas and
    /// photons: 1).
    pub infinite_from: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DysonStar {
    pub index: usize,
    pub name: String,
    pub luminosity: f64,
    pub max_gw: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatePoint {
    pub level: u32,
    pub cubes_per_min: f64,
    pub dyson_bound: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RateReport {
    pub status: Status,
    pub cubes_per_min: f64,
    pub level: u32,
    pub speed: f64,
    pub stars: usize,
    pub hub: Option<String>,
    pub dyson_gw: f64,
    /// Ray-receiver power the plan draws, GW.
    pub dyson_used_gw: f64,
    pub dyson: Vec<DysonStar>,
    /// Swarm star and photons per sail launched (0: no swarm).
    pub swarm_star: Option<String>,
    pub photons_per_sail: f64,
    /// Sails launched per minute and the swarm's power, GW.
    pub sails_per_min: f64,
    pub swarm_gw: f64,
    pub uses: Vec<RateUse>,
    /// Unlimited sources (oceans).
    pub unlimited: Vec<String>,
    /// Level from which every finite vein this mix uses never runs out (None: some never).
    pub infinite_level: Option<u32>,
    /// Level from which every star's shells are used in full (None: not by level 10 000, or Dyson off).
    pub dyson_level: Option<u32>,
    pub curve: Vec<RatePoint>,
    pub recipes: Vec<TotalRecipe>,
    pub raw_per_cube: Vec<ItemRate>,
    pub warpers_per_min: f64,
    pub machines: f64,
    /// Production power, GW (drawn from the spheres when `power`).
    pub power_gw: f64,
    pub power_mw_per_rate: f64,
    /// Shells only, vein utilisation 10 000 (rates no longer bind): the most cubes/min the shells feed.
    pub ceiling_shells: f64,
    pub solve_ms: f64,
    pub warnings: Vec<String>,
}

/// Rates at speed 1 per star (veins, oil) and the per-star photon caps.
struct RateInput {
    caps: BTreeMap<String, (String, Vec<(usize, f64)>)>,
    unlimited: BTreeSet<String>,
    photons: Option<Vec<(usize, f64)>>,
}

fn rate_input(data: &GameData, g: &Galaxy, rs: &RateSettings, sys: &[usize]) -> RateInput {
    let miner = data.resource("iron-ore").map(|r| r.extractor_rate).unwrap_or(60.0)
        * rs.miners_per_node.max(0.0)
        * rs.miner_speed.clamp(1.0, 3.0);
    let mut caps: BTreeMap<String, (String, Vec<(usize, f64)>)> = BTreeMap::new();
    let mut unlimited = BTreeSet::new();
    let mut add = |item: &str, src: &str, s: usize, r: f64| {
        let e = caps.entry(item.into()).or_insert_with(|| (src.into(), vec![]));
        if e.0 != src {
            e.0 = "vein+gas".into();
        }
        if r > 0.0 {
            e.1.push((s, r));
        }
    };
    for &s in sys {
        for pl in &g.stars[s].planets {
            if let Some(o) = &pl.ocean {
                unlimited.insert(o.clone());
            }
            for v in &pl.veins {
                if v.item == "crude-oil" {
                    let r = match rs.oil {
                        OilRate::Initial => pl.oil_rate * 60.0,
                        OilRate::Floor => v.nodes as f64 * 0.1 * 60.0,
                    };
                    add("crude-oil", "oil", s, r);
                } else {
                    add(&v.item, "vein", s, v.nodes as f64 * miner);
                }
            }
            for gr in &pl.gases {
                if gr.base > 0.0 {
                    add(&gr.item, "gas", s, 0.0);
                }
            }
        }
    }
    let photons = rs.dyson.then(|| {
        sys.iter()
            .map(|&s| (s, dyson_max_gw(&g.stars[s]) * rs.dyson_share.clamp(0.0, 1.0) * 1e3 * 60.0 / PHOTON_MJ))
            .collect()
    });
    RateInput { caps, unlimited, photons }
}

/// Caps at `level` as the (hub, elsewhere) map `total_lp` takes. Gas is rebuilt per level: the
/// collector's fixed 30 MW own use makes it not linear in speed.
fn rate_caps(g: &Galaxy, inp: &RateInput, level: u32, hub: Option<usize>, sys: &[usize]) -> BTreeMap<String, (f64, f64)> {
    let speed = speed_at(level);
    let mut m: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut put = |it: &str, s: usize, r: f64| {
        let e = m.entry(it.to_string()).or_default();
        if hub.is_none_or(|h| h == s) {
            e.0 += r
        } else {
            e.1 += r
        }
    };
    // gas entries hold no rates (built per level below); an item from veins and giants keeps both
    for (it, (_, v)) in &inp.caps {
        for (s, r) in v {
            put(it, *s, r * speed);
        }
    }
    for &s in sys {
        for pl in &g.stars[s].planets {
            if !pl.gases.is_empty() {
                for (i, r) in collector_net(&pl.gases, speed) {
                    if r > 0.0 {
                        put(&i, s, r * 60.0 * COLLECTORS_PER_GIANT);
                    }
                }
            }
        }
    }
    if let Some(ph) = &inp.photons {
        for (s, r) in ph {
            put(PHOTON, *s, *r);
        }
    }
    m
}

pub fn galaxy_rate(data: &GameData, g: &Galaxy, rs: &RateSettings) -> RateReport {
    let t0 = web_time::Instant::now();
    let level = rs.level.clamp(1, 10_000);
    let mut out = RateReport { level, speed: speed_at(level), ..Default::default() };
    let mut sys: Vec<usize> = if rs.systems.is_empty() { (0..g.stars.len()).collect() } else { rs.systems.clone() };
    sys.retain(|i| *i < g.stars.len());
    sys.sort();
    sys.dedup();
    out.stars = sys.len();
    if sys.is_empty() {
        out.status = Status::Infeasible;
        out.warnings.push("没有选星系".into());
        return out;
    }
    let mut ps = rs.plan.clone();
    ps.game = "dsp".into();
    let swarm_on = rs.dyson && rs.swarm && rs.sail_life_s > 0.0;
    let mut units = unit_cols(data, &ps, if swarm_on { &[CUBE, WARPER, SAIL] } else { &[CUBE, WARPER] });
    units.retain(|u| u.net.iter().all(|(i, q)| *q > 0.0 || !(i.starts_with("df-") || i == "log" || i == "plant-fuel")));
    if rs.power && rs.dyson {
        for u in units.iter_mut() {
            let ph = u.mode.power_base.max(0.0) * 60.0 / PHOTON_MJ;
            if ph > 0.0 {
                match u.net.iter_mut().find(|(i, _)| i == PHOTON) {
                    Some(e) => e.1 -= ph,
                    None => u.net.push((PHOTON.to_string(), -ph)),
                }
            }
        }
    }
    let inp = rate_input(data, g, rs, &sys);
    // swarm column: 1 sail/min keeps life/60 sails alive → photons/min = alive·36 kW·L·60 / 1200 MJ
    let brightest = sys.iter().copied().max_by(|a, b| g.stars[*a].luminosity.partial_cmp(&g.stars[*b].luminosity).unwrap());
    let swarm: Vec<Vec<(String, f64)>> = match (swarm_on, brightest) {
        (true, Some(b)) => {
            let ph = rs.sail_life_s / 60.0 * SAIL_KW * g.stars[b].luminosity * 1e-3 * 60.0 / PHOTON_MJ;
            out.swarm_star = Some(g.stars[b].name.clone());
            out.photons_per_sail = ph;
            vec![vec![(SAIL.to_string(), -1.0), (PHOTON.to_string(), ph)]]
        }
        _ => vec![],
    };
    let share = rs.dyson_share.clamp(0.0, 1.0);
    out.dyson = sys
        .iter()
        .map(|&s| {
            let st = &g.stars[s];
            DysonStar { index: s, name: st.name.clone(), luminosity: st.luminosity, max_gw: dyson_max_gw(st) * share }
        })
        .filter(|d| d.max_gw > 0.0)
        .collect();
    out.dyson.sort_by(|a, b| b.max_gw.partial_cmp(&a.max_gw).unwrap());
    out.dyson_gw = if rs.dyson { out.dyson.iter().map(|d| d.max_gw).sum() } else { 0.0 };
    let mut unlimited = inp.unlimited.clone();
    if !rs.dyson {
        unlimited.insert(PHOTON.into());
    }
    out.unlimited = unlimited.iter().cloned().collect();
    let wpi = if rs.warpers { rs.warpers_per_trip.max(0.0) / rs.vessel_capacity.max(1.0) } else { 0.0 };
    let hubs: Vec<Option<usize>> = if wpi > 0.0 { sys.iter().map(|s| Some(*s)).collect() } else { vec![None] };
    type Best = (Option<usize>, BTreeMap<String, (f64, f64)>, TotalSol);
    let solve_at = |lv: u32| -> Result<Best, String> {
        let mut best: Option<Best> = None;
        let mut err = None;
        for h in &hubs {
            let caps = rate_caps(g, &inp, lv, *h, &sys);
            match total_lp(&units, &caps, &unlimited, wpi, None, &swarm) {
                Ok(r) => {
                    if best.as_ref().is_none_or(|b| r.0 > b.2 .0) {
                        best = Some((*h, caps, r));
                    }
                }
                Err(e) => err = Some(e),
            }
        }
        best.ok_or_else(|| err.unwrap_or_default())
    };
    let (hub, caps, (n, xv, mv)) = match solve_at(level) {
        Ok(r) => r,
        Err(e) if e == "unbounded" => {
            out.status = Status::Unbounded;
            out.warnings.push("无上限：只用无限的来源（海洋、不限的光子）就能造白糖".into());
            return out;
        }
        Err(e) => {
            out.status = Status::Infeasible;
            out.warnings.push(format!("求解器：{e}"));
            return out;
        }
    };
    out.status = Status::Optimal;
    out.cubes_per_min = n;
    out.hub = hub.map(|h| g.stars[h].name.clone());
    if n <= 1e-9 {
        out.warnings.push("造不出白糖：这些星系缺一种必需的原料".into());
    }
    let mult = g.settings.resource_multiplier;
    let stock_of_item = |it: &str| -> f64 {
        sys.iter().flat_map(|&s| g.stars[s].planets.iter()).flat_map(|p| p.veins.iter()).filter(|v| v.item == it).map(stock_of).sum()
    };
    let speed = speed_at(level);
    let gas_cap = |it: &str| -> f64 {
        sys.iter()
            .flat_map(|&s| g.stars[s].planets.iter())
            .filter(|p| !p.gases.is_empty())
            .flat_map(|p| collector_net(&p.gases, speed))
            .filter(|(i, _)| i == it)
            .map(|(_, r)| r.max(0.0) * 60.0 * COLLECTORS_PER_GIANT)
            .sum()
    };
    for (k, (it, (ch, ca))) in caps.iter().enumerate() {
        let (h, a) = mv[k];
        out.warpers_per_min += a * wpi;
        let cap = ch + ca;
        let used = h + a;
        let source = if it == PHOTON { "dyson".to_string() } else { inp.caps.get(it).map(|c| c.0.clone()).unwrap_or_else(|| "gas".into()) };
        let per_cube = if n > 1e-9 { used / n } else { 0.0 };
        let infinite_from = match source.as_str() {
            "vein" | "oil" if !g.infinite => threshold_level(per_cube, stock_of_item(it), rs.rho, points_per_unit(it, mult)),
            // giants first (they never run out), veins for the rest
            "vein+gas" if !g.infinite && n > 1e-9 => {
                threshold_level((used - gas_cap(it)).max(0.0) / n, stock_of_item(it), rs.rho, points_per_unit(it, mult))
            }
            _ => Some(1),
        };
        let binding = n > 1e-9 && cap > 0.0 && used >= cap * (1.0 - 1e-6);
        out.uses.push(RateUse { item: it.clone(), source, cap, used, per_cube, binding, marginal: 0.0, infinite_from });
    }
    for u in out.uses.iter_mut().filter(|u| u.binding) {
        if let Ok((n2, _, _)) = total_lp(&units, &caps, &unlimited, wpi, Some((&u.item, 1.01)), &swarm) {
            u.marginal = ((n2 - n) / (0.01 * u.cap)).max(0.0);
        }
        // fully used but more gives nothing (a degenerate vertex): not a limit
        u.binding = u.marginal > 1e-9;
    }
    out.uses.sort_by(|a, b| b.binding.cmp(&a.binding).then(b.marginal.partial_cmp(&a.marginal).unwrap()).then(a.item.cmp(&b.item)));
    out.infinite_level = out.uses.iter().filter(|u| u.used > 1e-9 * n.max(1.0)).try_fold(1u32, |m, u| u.infinite_from.map(|l| m.max(l)));
    if let Some(ph) = out.uses.iter().find(|u| u.item == PHOTON) {
        out.dyson_used_gw = ph.used * PHOTON_MJ / 60.0 / 1e3;
    }
    if n > 1e-9 {
        let mut raw: BTreeMap<String, f64> = BTreeMap::new();
        if let Some(&s) = xv.get(units.len()) {
            out.sails_per_min = s;
            out.swarm_gw = s * out.photons_per_sail * PHOTON_MJ / 60.0 / 1e3;
        }
        for (k, u) in units.iter().enumerate() {
            let x = xv[k];
            if x <= 1e-9 * n {
                continue;
            }
            out.machines += x;
            out.power_gw += x * u.mode.power_base.max(0.0) / 1e3;
            out.recipes.push(TotalRecipe { recipe: u.recipe.id.clone(), mode: u.mode.label.clone(), building: u.mode.building.clone(), machines_per_rate: x / n });
            for (i, q) in &u.net {
                if *q < 0.0 && unlimited.contains(i) {
                    *raw.entry(i.clone()).or_default() += -q * x / n;
                }
            }
        }
        for u in out.uses.iter().filter(|u| u.used > 0.0) {
            raw.insert(u.item.clone(), u.per_cube);
        }
        out.raw_per_cube = raw.into_iter().map(|(item, rate)| ItemRate { item, rate }).collect();
        out.recipes.sort_by(|a, b| b.machines_per_rate.partial_cmp(&a.machines_per_rate).unwrap());
    }
    // the level from which the shells are used in full (rates grow with the level, shells do not);
    // past it z still rises a little as the mix shifts to fewer photons per cube
    let z_at = |lv: u32| if lv == level { n } else { solve_at(lv).map(|r| r.2 .0).unwrap_or(0.0) };
    if rs.dyson && out.dyson_gw > 0.0 {
        let z_cap = z_at(10_000);
        // Dyson-bound: the photon cap is used in full (z stops growing with the level)
        // shells full = more shell power gives more cubes (a used-in-full test alone flips between
        // equally good vertices, review)
        let bound = |lv: u32| -> bool {
            match solve_at(lv) {
                Ok((h, c, (z, _, _))) if z > 0.0 => match total_lp(&units, &c, &unlimited, wpi, Some((PHOTON, 1.01)), &swarm) {
                    Ok((z2, _, _)) => {
                        let _ = h;
                        z2 > z * (1.0 + 1e-7)
                    }
                    Err(_) => false,
                },
                _ => false,
            }
        };
        if z_cap > 0.0 && bound(10_000) {
            let (mut lo, mut hi) = (1u32, 10_000u32);
            if bound(1) {
                hi = 1;
            }
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                if bound(mid) {
                    hi = mid
                } else {
                    lo = mid
                }
            }
            out.dyson_level = Some(hi);
        }
    }
    if rs.dyson && out.dyson_gw > 0.0 {
        let caps = rate_caps(g, &inp, 10_000, None, &sys);
        if let Ok((z, _, _)) = total_lp(&units, &caps, &unlimited, wpi, None, &[]) {
            out.ceiling_shells = z;
        }
    }
    out.power_mw_per_rate = if n > 1e-9 { out.power_gw * 1e3 / n } else { 0.0 };
    let mut lv: BTreeSet<u32> = [6, 20, 40, 81, 120, 200, 400, level].into();
    if let Some(d) = out.dyson_level {
        lv.insert(d);
    }
    for l in lv {
        out.curve.push(RatePoint { level: l, cubes_per_min: z_at(l), dyson_bound: out.dyson_level.is_some_and(|d| l >= d) });
    }
    if rs.miners_per_node > 1.0 + 1e-9 {
        out.warnings.push(format!("每个矿点按 {} 台大型采矿机重叠开采计算", rs.miners_per_node));
    }
    out.solve_ms = t0.elapsed().as_secs_f64() * 1e3;
    out
}

/// Full Dyson shell power of one star, GW: 10 layers at R, R−1000, … (layers ≥ 1000 m apart, dsp-wiki
/// Dyson_Sphere), R = `dyson_radius` (the editor's largest orbit, m). Per layer of radius r, fitted
/// in game by wiki.biligame.com/dsp 戴森球发电量 (r 8000–245 520 m, ±1 %): 52·r²/10⁴ cell points × 15 kW,
/// 1.22·r frame structure points + 60 nodes × 30 points, × 96 kW each, all × luminosity (the displayed
/// 光度 = generator `luminosity`). Checks: birth star (0.982, 21 700 m) one layer 38.7 GW (bili ≈ 30);
/// O star (2.462, 74 400 m) one layer 1085 GW (bili ≈ 1000). The swarm is separate (`RateSettings::swarm`).
pub fn dyson_max_gw(st: &StarInfo) -> f64 {
    let r0 = st.dyson_radius as f64;
    let mut kw = 0.0;
    for k in 0..10 {
        let r = r0 - 1000.0 * k as f64;
        // MINE: a layer needs room above the star; layers under 1000 m are not counted
        if r < 1000.0 {
            break;
        }
        kw += 52.0 * r * r / 1e4 * 15.0 + (1.22 * r + 60.0 * 30.0) * 96.0;
    }
    kw * st.luminosity.max(0.0) * 1e-6
}
