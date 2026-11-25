//! Satisfactory speedrun planner (memo step 17; Xinyu 2026-11-24: 「考虑到每种物品的生产，然后以及爬科技树的速度
//! 在各个阶段不同的需要建造的工厂。假设建造工厂也需要…每建造一个建筑需要一定的时间…可能需要个20到30秒，然后以及
//! 剩下的这个拉线路的时间都是需要时间的。那么按照这个时间，然后以及生产速度大概能爬多少科技，在固定时间爬多少科技」).
//!
//! Model (v1, every simplification MINE unless cited):
//! - The tech tree is climbed one stage at a time: the milestones up to a tier in tier order (cheapest first
//!   within a tier), with the Space Elevator phase that gates the next tiers in between (phase 1 before
//!   tier 3, 2 before 5, 3 before 7, 4 before 9; satisfactory.wiki.gg Project_Assembly), phase 5 last.
//! - Recipes: the tutorial unlocks + Iron Ingot / Plate / Rod (start of the game), each milestone's unlocks
//!   once it is shipped, MAM research recipes from "Field Research" on (their research cost is not counted).
//!   No alternate recipes (hard drives are a separate hunt).
//! - Each stage adds factories (a continuous LP, fewest buildings) that make the stage's cost at rate
//!   cost / τ, using what earlier factories already make as free supplies. The stage takes
//!   build time + τ, build time = (new buildings + new miners/extractors) × (place + wire seconds), and τ
//!   is chosen to minimise it (more buildings → shorter production). The materials of the new buildings
//!   are added to the cost (one fixed-point pass): as if the stage's factory also made its own parts.
//! - Factories stay and keep producing; the next stage gets their output as supplies.
//! - The HUB ship: a milestone can be submitted only after the previous one's ship returned.
//! - Not modelled: power plants and power, hand-crafting, walking / exploration, belts and poles as
//!   materials (their time is the per-building wiring time), node purity and distance.
use super::data::{GameData, Milestone};
use super::{plan, ItemRate, Objective, PlanSettings, Status};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeedrunSettings {
    /// Climb milestones up to this tier (1–9).
    pub up_to_tier: u32,
    /// Seconds to place one building (Xinyu: 20–30 s, blueprints included).
    pub place_s: f64,
    /// Seconds of belts, pipes and power lines per building.
    pub wire_s: f64,
    /// Space Elevator phases between tiers (and phase 5 at the end when tier 9 is reached).
    pub elevator: bool,
    /// Report how far the climb gets in this many hours (0 = no limit line).
    pub hours: f64,
    /// UI language for names ("zh" | "en").
    pub lang: String,
}

impl Default for SpeedrunSettings {
    fn default() -> Self {
        Self { up_to_tier: 9, place_s: 25.0, wire_s: 20.0, elevator: true, hours: 0.0, lang: String::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageBuild {
    pub building: String,
    pub count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    /// "milestone" | "phase"
    pub kind: String,
    pub id: String,
    pub name: String,
    pub tier: u32,
    pub cost: Vec<ItemRate>,
    /// New production buildings and extractors built for this stage.
    pub buildings: f64,
    pub miners: f64,
    pub build: Vec<StageBuild>,
    pub build_min: f64,
    pub produce_min: f64,
    /// Waiting for the HUB ship of the previous milestone.
    pub wait_min: f64,
    pub start_min: f64,
    pub end_min: f64,
    /// Factory size after this stage (all stages so far).
    pub total_buildings: f64,
    /// Recipes this stage unlocks, by name.
    pub unlocks: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeedrunReport {
    pub status: Status,
    pub stages: Vec<Stage>,
    pub total_min: f64,
    pub total_buildings: f64,
    /// Stages done within `hours` (when set).
    pub done_in_hours: Option<usize>,
    pub solve_ms: f64,
    pub warnings: Vec<String>,
}

/// Space Elevator deliveries per phase at 1× cost (satisfactory.wiki.gg Project_Assembly, 1.0; the same
/// table as react-frontend/src/presets.ts).
const PHASES: [&[(&str, f64)]; 5] = [
    &[("Desc_SpaceElevatorPart_1_C", 50.0)],
    &[("Desc_SpaceElevatorPart_1_C", 1000.0), ("Desc_SpaceElevatorPart_2_C", 1000.0), ("Desc_SpaceElevatorPart_3_C", 100.0)],
    &[("Desc_SpaceElevatorPart_2_C", 2500.0), ("Desc_SpaceElevatorPart_4_C", 500.0), ("Desc_SpaceElevatorPart_5_C", 100.0)],
    &[("Desc_SpaceElevatorPart_7_C", 500.0), ("Desc_SpaceElevatorPart_6_C", 500.0), ("Desc_SpaceElevatorPart_8_C", 250.0), ("Desc_SpaceElevatorPart_9_C", 100.0)],
    &[("Desc_SpaceElevatorPart_9_C", 1000.0), ("Desc_SpaceElevatorPart_10_C", 1000.0), ("Desc_SpaceElevatorPart_11_C", 200.0), ("Desc_SpaceElevatorPart_12_C", 256.0)],
];

/// The game's starting recipes besides the tutorial unlocks (the HUB's iron chain).
const START: [&str; 3] = ["Recipe_IngotIron_C", "Recipe_IronPlate_C", "Recipe_IronRod_C"];

enum StageDef<'a> {
    Milestone(&'a Milestone),
    Phase(usize),
}

/// Extractor rate per minute and its building, for a raw item with the miners unlocked.
fn extractor(item: &str, unlocked: &BTreeSet<String>) -> (f64, &'static str) {
    match item {
        "Desc_Water_C" => (120.0, "Desc_WaterPump_C"),
        "Desc_LiquidOil_C" => (120.0, "Desc_OilPump_C"),
        "Desc_NitrogenGas_C" => (60.0, "Desc_FrackingExtractor_C"),
        // Miner Mk.1/2/3 on a normal node at 100 % (satisfactory.wiki.gg Miner)
        _ if unlocked.contains("Recipe_MinerMk3_C") => (240.0, "Desc_MinerMk3_C"),
        _ if unlocked.contains("Recipe_MinerMk2_C") => (120.0, "Desc_MinerMk2_C"),
        _ => (60.0, "Desc_MinerMk1_C"),
    }
}

struct Trial {
    tau: f64,
    total: f64,
    build_min: f64,
    buildings: f64,
    miners: f64,
    build: BTreeMap<String, f64>,
    supplies_used: BTreeMap<String, f64>,
    made: Vec<ItemRate>,
}

pub fn speedrun(data: &GameData, ss: &SpeedrunSettings) -> SpeedrunReport {
    let t0 = web_time::Instant::now();
    super::data::set_lang(&ss.lang);
    let mut out = SpeedrunReport::default();
    let Some(prog) = data.progression.as_ref() else {
        out.status = Status::Infeasible;
        out.warnings.push(super::data::tr("No tech progression in this game's data.", "这个游戏的数据里没有科技树。"));
        return out;
    };
    let sec = (ss.place_s.max(0.0) + ss.wire_s.max(0.0)) / 60.0; // minutes per building
    let mut unlocked: BTreeSet<String> = prog.tutorial_recipes.iter().cloned().chain(START.iter().map(|s| s.to_string())).collect();
    // byproduct handling recipes come with their machines: allowed whenever the machine's other recipes are
    let always: BTreeSet<String> = data
        .recipes
        .iter()
        .filter(|r| r.name.starts_with("Residual") || r.name.starts_with("Unpackage"))
        .map(|r| r.id.clone())
        .collect();

    // the climb: within each tier the cheapest milestone whose cost the recipes unlocked so far can make
    // (Jump Pads needs Part Assembly's rotors), elevator phases in between
    let mam = |u: &BTreeSet<String>| u.contains("Recipe_Mam_C");
    let reachable = |u: &BTreeSet<String>| -> BTreeSet<String> {
        let mut ok: BTreeSet<String> = data.resources.iter().map(|r| r.item.clone()).collect();
        let rec: Vec<&super::data::Recipe> = data
            .recipes
            .iter()
            .filter(|r| u.contains(&r.id) || always.contains(&r.id) || (mam(u) && prog.mam_recipes.contains(&r.id)))
            .collect();
        loop {
            let n = ok.len();
            for r in &rec {
                if r.inputs.iter().all(|(i, _)| ok.contains(i)) {
                    ok.extend(r.outputs.iter().map(|(i, _)| i.clone()));
                }
            }
            if ok.len() == n {
                return ok;
            }
        }
    };
    let weight = |m: &Milestone| m.cost.iter().map(|c| c.1).sum::<f64>();
    let mut ms: Vec<&Milestone> = vec![];
    {
        let mut u = unlocked.clone();
        for tier in 1..=ss.up_to_tier.clamp(1, 9) {
            let mut left: Vec<&Milestone> = prog.milestones.iter().filter(|m| m.tier == tier).collect();
            left.sort_by(|a, b| weight(a).partial_cmp(&weight(b)).unwrap());
            while !left.is_empty() {
                let ok = reachable(&u);
                // MINE: the cheapest one that can be made now; if none can, the cheapest (it will be reported)
                let k = left.iter().position(|m| m.cost.iter().all(|(i, _)| ok.contains(i))).unwrap_or(0);
                let m = left.remove(k);
                u.extend(m.recipes.iter().cloned());
                ms.push(m);
            }
        }
    }
    let mut defs: Vec<StageDef> = vec![];
    let mut phase_done = 0usize;
    for m in &ms {
        // tiers 3–4 need phase 1, 5–6 phase 2, 7–8 phase 3, 9 phase 4
        let need = ((m.tier.saturating_sub(1)) / 2) as usize;
        while ss.elevator && phase_done < need && phase_done < 4 {
            defs.push(StageDef::Phase(phase_done));
            phase_done += 1;
        }
        defs.push(StageDef::Milestone(m));
    }
    if ss.elevator && ss.up_to_tier >= 9 {
        while phase_done < 5 {
            defs.push(StageDef::Phase(phase_done));
            phase_done += 1;
        }
    }

    let mut factory: BTreeMap<String, f64> = BTreeMap::new(); // what built factories make, per minute
    let mut clock = 0.0;
    let mut ship_back = 0.0; // when the HUB ship of the last milestone returns
    let mut total_buildings = 0.0;
    for d in &defs {
        let (kind, id, name, tier, cost): (&str, String, String, u32, Vec<(String, f64)>) = match d {
            StageDef::Milestone(m) => ("milestone", m.id.clone(), m.name.clone(), m.tier, m.cost.clone()),
            StageDef::Phase(p) => (
                "phase",
                format!("phase-{}", p + 1),
                super::data::tr(&format!("Space Elevator phase {}", p + 1), &format!("太空电梯第 {} 阶段", p + 1)),
                0,
                PHASES[*p].iter().map(|(i, q)| (i.to_string(), *q)).collect(),
            ),
        };
        let mut allowed = unlocked.clone();
        if mam(&unlocked) {
            allowed.extend(prog.mam_recipes.iter().cloned());
        }
        allowed.extend(always.iter().cloned());
        let disabled: Vec<String> = data.recipes.iter().filter(|r| !allowed.contains(&r.id)).map(|r| r.id.clone()).collect();
        // parts the unlocked recipes make; others (Portable Miner: Equipment Workshop) count as hand-crafted
        let made_by_machine: BTreeSet<&str> =
            data.recipes.iter().filter(|r| allowed.contains(&r.id)).flat_map(|r| r.outputs.iter().map(|(i, _)| i.as_str())).collect();
        let base = PlanSettings {
            game: "satisfactory".into(),
            objective: Objective::Buildings,
            cost: Objective::Buildings.preset(),
            integer: false,
            disabled_recipes: disabled,
            allow_all_alternates: false,
            lang: ss.lang.clone(),
            ..Default::default()
        };
        // one trial: make `cost` (plus the new buildings' own parts) in τ minutes
        let trial = |tau: f64| -> Option<Trial> {
            let mut extra: BTreeMap<String, f64> = BTreeMap::new();
            let mut last: Option<Trial> = None;
            for _ in 0..2 {
                let mut want: BTreeMap<String, f64> = cost.iter().cloned().collect();
                for (i, q) in &extra {
                    *want.entry(i.clone()).or_default() += q;
                }
                let mut s = base.clone();
                s.targets = want.iter().map(|(i, q)| ItemRate { item: i.clone(), rate: q / tau }).collect();
                s.supplies = factory.iter().filter(|(_, r)| **r > 1e-9).map(|(i, r)| ItemRate { item: i.clone(), rate: *r }).collect();
                let p = plan(data, &s);
                if p.status != Status::Optimal {
                    return None;
                }
                let mut build: BTreeMap<String, f64> = BTreeMap::new();
                let mut buildings = 0.0;
                for r in &p.recipes {
                    let b = data.recipe(&r.id).map(|x| x.building.clone()).unwrap_or_else(|| r.building.clone());
                    let n = r.machines.max(0.0).ceil();
                    *build.entry(b).or_default() += n;
                    buildings += n;
                }
                let mut miners = 0.0;
                for l in &p.raw {
                    let (rate, b) = extractor(&l.item, &unlocked);
                    let n = (l.rate / rate - 1e-9).ceil().max(0.0);
                    *build.entry(b.to_string()).or_default() += n;
                    miners += n;
                }
                // parts for these buildings, made by the stage's factory too (MINE)
                extra.clear();
                for (b, n) in &build {
                    if let Some(bc) = prog.building_costs.get(b) {
                        // parts no unlocked recipe makes (the Miner's Portable Miner) are hand-crafted: left out
                        for (i, q) in bc.cost.iter().filter(|(i, _)| made_by_machine.contains(i.as_str())) {
                            *extra.entry(i.clone()).or_default() += q * n;
                        }
                    }
                }
                let build_min = (buildings + miners) * sec;
                last = Some(Trial {
                    tau,
                    total: build_min + tau,
                    build_min,
                    buildings,
                    miners,
                    build,
                    supplies_used: p.supplied.iter().map(|l| (l.item.clone(), l.rate)).collect(),
                    made: s.targets.clone(),
                });
            }
            last
        };
        // τ by golden-section search on log τ (the total is close to unimodal: build ∝ 1/τ, plus τ)
        let (mut lo, mut hi) = (0.5f64.ln(), 2000f64.ln());
        let g = 0.618_033_988_75;
        let eval = |x: f64| trial(x.exp());
        let mut best: Option<Trial> = None;
        let keep = |t: Option<Trial>, best: &mut Option<Trial>| -> f64 {
            match t {
                Some(t) => {
                    let v = t.total;
                    if best.as_ref().is_none_or(|b| v < b.total) {
                        *best = Some(t);
                    }
                    v
                }
                None => f64::INFINITY,
            }
        };
        let (mut a, mut b) = (hi - g * (hi - lo), lo + g * (hi - lo));
        let (mut fa, mut fb) = (keep(eval(a), &mut best), keep(eval(b), &mut best));
        for _ in 0..14 {
            if fa < fb {
                hi = b;
                b = a;
                fb = fa;
                a = hi - g * (hi - lo);
                fa = keep(eval(a), &mut best);
            } else {
                lo = a;
                a = b;
                fa = fb;
                b = lo + g * (hi - lo);
                fb = keep(eval(b), &mut best);
            }
        }
        let Some(t) = best else {
            out.warnings.push(super::data::tr(
                &format!("{name}: cannot be made with the recipes unlocked so far; skipped"),
                &format!("{name}：用目前解锁的配方做不出来，跳过"),
            ));
            continue;
        };
        let start = clock;
        let ready = start + t.total;
        let submit = if kind == "milestone" { ready.max(ship_back) } else { ready };
        let wait = submit - ready;
        total_buildings += t.buildings + t.miners;
        // factories stay: their outputs are free for later stages, minus what this stage took from them
        for (i, q) in &t.supplies_used {
            if let Some(f) = factory.get_mut(i) {
                *f = (*f - q).max(0.0);
            }
        }
        for m in &t.made {
            *factory.entry(m.item.clone()).or_default() += m.rate;
        }
        let mut unlocks = vec![];
        if let StageDef::Milestone(m) = d {
            ship_back = submit + m.time / 60.0;
            for r in &m.recipes {
                if unlocked.insert(r.clone()) {
                    if let Some(x) = data.recipe(r) {
                        unlocks.push(data.pick(&x.name, &x.name_zh).to_string());
                    }
                }
            }
        }
        clock = submit;
        out.stages.push(Stage {
            kind: kind.into(),
            id,
            name,
            tier,
            cost: cost.iter().map(|(i, q)| ItemRate { item: i.clone(), rate: *q }).collect(),
            buildings: t.buildings,
            miners: t.miners,
            build: t
                .build
                .iter()
                .filter(|(_, n)| **n > 0.0)
                .map(|(b, n)| StageBuild {
                    building: data
                        .building(b)
                        .map(|x| data.pick(&x.name, &x.name_zh).to_string())
                        .or_else(|| prog.building_costs.get(b).map(|c| c.name.clone()))
                        .unwrap_or_else(|| b.clone()),
                    count: *n,
                })
                .collect(),
            build_min: t.build_min,
            produce_min: t.tau,
            wait_min: wait,
            start_min: start,
            end_min: submit,
            total_buildings,
            unlocks,
            note: String::new(),
        });
    }
    out.status = if out.stages.is_empty() { Status::Infeasible } else { Status::Optimal };
    out.total_min = clock;
    out.total_buildings = total_buildings;
    if ss.hours > 0.0 {
        out.done_in_hours = Some(out.stages.iter().filter(|s| s.end_min <= ss.hours * 60.0 + 1e-9).count());
    }
    out.solve_ms = t0.elapsed().as_secs_f64() * 1e3;
    out
}
