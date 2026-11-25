//! Satisfactory speedrun planner (memo step 17; Xinyu 2026-11-24: 「考虑到每种物品的生产，然后以及爬科技树的速度
//! 在各个阶段不同的需要建造的工厂。假设建造工厂也需要…每建造一个建筑需要一定的时间…可能需要个20到30秒，然后以及
//! 剩下的这个拉线路的时间都是需要时间的。那么按照这个时间，然后以及生产速度大概能爬多少科技，在固定时间爬多少科技」).
//!
//! Model (every simplification MINE unless cited):
//! - The tech tree is climbed one step at a time: in each tier the cheapest milestone whose cost the
//!   recipes unlocked so far can make, only those the Space Elevator parts need (`needed_only`), with
//!   the elevator phase that gates the next tiers in between (phase 1 before tier 3, 2 before 5, 3 before
//!   7, 4 before 9; satisfactory.wiki.gg Project_Assembly), phase 5 last.
//! - Recipes: the tutorial unlocks + Iron Ingot / Plate / Rod, each milestone's unlocks once shipped, MAM
//!   recipes once the MAM exists (with `mam_cost`, the first step that uses one pays its node and its
//!   tree's `_0` entry node; other prerequisites and research time are not in the dump), no hard-drive
//!   alternates except MAM ones.
//! - Each step adds factories (continuous LP, fewest buildings) that make its cost (after the stockpile)
//!   at rate cost / τ, earlier factories' output being free supplies; τ minimises build + τ for the step
//!   alone (a heuristic: with overlap the whole timeline is not optimised). Build time = (new buildings +
//!   extractors) × (place + wire s); the new buildings' parts are added to the cost.
//! - Timeline (v2): the player builds steps in order; with `overlap` the next step is built while this
//!   one produces, with the recipes shipped by then (else from the first later ship that suffices);
//!   milestones ship in order, one HUB ship at a time, phases in order; with `stockpile` finished
//!   factories' output piles up and pays later costs first.
//! - Not modelled: hand-crafting, walking, node purity / distance / counts, splitting a phase into
//!   parts, several steps in parallel beyond the overlap, belts and poles as materials.
use super::data::{GameData, MamNode, Milestone};
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
    /// Skip milestones the Space Elevator parts do not need (equipment, vehicles, …), as a speedrun does
    /// (review: forcing all 42 is not a speedrun).
    pub needed_only: bool,
    /// Build generators for each step's power once a generator is unlocked (before that: hand-fed burners,
    /// not counted).
    pub power: bool,
    /// v2: build the next step while this one's factories produce (with what is shipped by then).
    pub overlap: bool,
    /// v2: what finished factories keep making piles up and pays later costs first.
    pub stockpile: bool,
    /// v2: a MAM node's cost is paid by the first step that uses one of its recipes.
    pub mam_cost: bool,
    /// UI language for names ("zh" | "en").
    pub lang: String,
}

impl Default for SpeedrunSettings {
    fn default() -> Self {
        Self { up_to_tier: 9, place_s: 25.0, wire_s: 20.0, elevator: true, hours: 0.0, needed_only: true, power: true, overlap: true, stockpile: true, mam_cost: true, lang: String::new() }
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
    /// Part of the cost paid from the stockpile of earlier factories.
    pub from_stock: Vec<ItemRate>,
    /// MAM research done in this step (its cost is in the step's production).
    pub research: Vec<String>,
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
    /// What the new factories run: recipe name and machines (continuous), and the new raw input per minute.
    pub recipes: Vec<StageBuild>,
    pub raw: Vec<ItemRate>,
    /// New generators' output, MW (0 when power is off or no generator is unlocked yet).
    pub power_mw: f64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeedrunReport {
    pub status: Status,
    pub stages: Vec<Stage>,
    pub total_min: f64,
    pub total_buildings: f64,
    /// Where the time goes: the player building (Σ build), and the rest (waiting for production, ships, phases).
    pub build_min: f64,
    pub idle_min: f64,
    /// Stages done within `hours` (when set), and the last one.
    pub done_in_hours: Option<usize>,
    pub reached: Option<String>,
    /// Milestones left out as not needed for the Space Elevator (`needed_only`).
    pub skipped: Vec<String>,
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
    recipes: Vec<StageBuild>,
    raw: Vec<ItemRate>,
    power_mw: f64,
    tau: f64,
    total: f64,
    build_min: f64,
    buildings: f64,
    miners: f64,
    build: BTreeMap<String, f64>,
    supplies_used: BTreeMap<String, f64>,
    made: Vec<ItemRate>,
    /// MAM nodes this step researches.
    research: Vec<String>,
    research_names: Vec<String>,
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
    // minutes per building; at least 1 s (0 made the factory size unbounded, review)
    let sec = (ss.place_s.max(0.0) + ss.wire_s.max(0.0)).max(1.0) / 60.0;
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
    // items the Space Elevator parts need, through every standard and MAM recipe (any tier)
    if ss.needed_only && ss.elevator {
        let mut need: BTreeSet<String> = PHASES.iter().flat_map(|p| p.iter().map(|(i, _)| i.to_string())).collect();
        let prod: Vec<&super::data::Recipe> =
            data.recipes.iter().filter(|r| !r.generator && (!r.alt || prog.mam_recipes.contains(&r.id))).collect();
        loop {
            let n = need.len();
            for r in &prod {
                if r.outputs.iter().any(|(i, _)| need.contains(i)) {
                    need.extend(r.inputs.iter().map(|(i, _)| i.clone()));
                }
            }
            if need.len() == n {
                break;
            }
        }
        // buildings those recipes run in, extractors and generators (their build recipes)
        let mut builds: BTreeSet<String> = BTreeSet::new();
        for r in prod.iter().filter(|r| r.outputs.iter().any(|(i, _)| need.contains(i))) {
            if let Some(bc) = prog.building_costs.get(&r.building) {
                builds.insert(bc.recipe.clone());
            }
        }
        for (b, bc) in &prog.building_costs {
            if b.starts_with("Desc_Miner") || b.contains("Pump") || b.contains("Fracking") || b.starts_with("Desc_Generator") {
                builds.insert(bc.recipe.clone());
            }
        }
        let useful = |m: &Milestone| {
            m.recipes.iter().any(|r| {
                builds.contains(r) || r == "Recipe_Mam_C" || data.recipe(r).is_some_and(|x| x.outputs.iter().any(|(i, _)| need.contains(i)))
            })
        };
        ms.retain(|m| {
            let keep = useful(m);
            if !keep {
                out.skipped.push(m.name.clone());
            }
            keep
        });
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

    // ---- the timeline (v2): the player builds one step after another; each step's factories produce
    // while the player already builds the next one (overlap), using only the recipes of milestones
    // shipped by then (else the player waits for the ship). What finished factories keep making piles up
    // and pays later costs first (stockpiling). MAM nodes are paid by the first step that uses them.
    let mam_of: BTreeMap<&str, &MamNode> = prog.mam_nodes.iter().flat_map(|n| n.recipes.iter().map(move |r| (r.as_str(), n))).collect();
    let mut researched: BTreeSet<String> = BTreeSet::new();
    let mut factory: BTreeMap<String, f64> = BTreeMap::new(); // free output of finished factories, per minute
    let mut stock: BTreeMap<String, f64> = BTreeMap::new();
    let mut stock_t = 0.0; // stock is counted up to this time
    // factories still producing for their step: (ready time, their output rates)
    let mut running: Vec<(f64, Vec<ItemRate>)> = vec![];
    // milestones submitted: (submit time, recipes they unlock)
    let mut shipped: Vec<(f64, Vec<String>)> = vec![];
    let start_unlocked = unlocked.clone();
    let mut player = 0.0; // when the player is free to build the next step
    let mut last_submit = 0.0;
    let mut last_phase = 0.0;
    let mut ship_back = 0.0; // when the HUB ship of the last milestone returns
    let mut total_buildings = 0.0;
    // stock and finished factories up to time t
    let advance = |t: f64, factory: &mut BTreeMap<String, f64>, stock: &mut BTreeMap<String, f64>, stock_t: &mut f64, running: &mut Vec<(f64, Vec<ItemRate>)>| {
        running.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        loop {
            let next = running.first().map(|r| r.0).filter(|r| *r <= t);
            let upto = next.unwrap_or(t);
            if ss.stockpile && upto > *stock_t {
                for (i, r) in factory.iter() {
                    *stock.entry(i.clone()).or_default() += r * (upto - *stock_t);
                }
            }
            *stock_t = (*stock_t).max(upto);
            match next {
                Some(_) => {
                    let (_, made) = running.remove(0);
                    for m in made {
                        *factory.entry(m.item).or_default() += m.rate;
                    }
                }
                None => break,
            }
        }
    };
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
        // try at once (overlap) with what is shipped by then; else wait for the last milestone
        let unlocked_at = |t: f64| -> BTreeSet<String> {
            let mut u = start_unlocked.clone();
            for (st, rs) in &shipped {
                if *st <= t + 1e-9 {
                    u.extend(rs.iter().cloned());
                }
            }
            u
        };
        // the first time from which the step can be made: now, then each later ship (review: not only the last)
        let mut tries: Vec<f64> = if ss.overlap { vec![player] } else { vec![player.max(last_submit)] };
        if ss.overlap {
            let mut later: Vec<f64> = shipped.iter().map(|(t, _)| *t).filter(|t| *t > player).collect();
            later.sort_by(|a, b| a.partial_cmp(b).unwrap());
            later.dedup();
            tries.extend(later);
        }
        let mut found = None;
        for &t in &tries {
            advance(t, &mut factory, &mut stock, &mut stock_t, &mut running);
            let u = unlocked_at(t);
            // stock pays first
            let mut take: BTreeMap<String, f64> = BTreeMap::new();
            let left: Vec<(String, f64)> = cost
                .iter()
                .map(|(i, q)| {
                    let have = stock.get(i).copied().unwrap_or(0.0).min(*q);
                    take.insert(i.clone(), have);
                    (i.clone(), q - have)
                })
                .collect();
            if let Some(tr) = solve_stage(data, prog, ss, sec, &always, &u, &researched, &mam_of, &factory, &left) {
                found = Some((t, u, take, tr));
                break;
            }
        }
        let Some((start, _u, take, t)) = found else {
            out.warnings.push(super::data::tr(
                &format!("{name}: cannot be made with the recipes unlocked so far; skipped"),
                &format!("{name}：用目前解锁的配方做不出来，跳过"),
            ));
            continue;
        };
        for (i, q) in &take {
            if let Some(s) = stock.get_mut(i) {
                *s = (*s - q).max(0.0);
            }
        }
        let build_end = start + t.build_min;
        let ready = build_end + t.tau;
        // milestones go in order, one HUB ship at a time; phases need only their parts
        // milestones ship in order, one HUB ship at a time; phases in order among themselves
        let submit = if kind == "milestone" { ready.max(ship_back).max(last_submit) } else { ready.max(last_phase) };
        let wait = submit - ready;
        total_buildings += t.buildings + t.miners;
        // this step's factories take their supplies from now on and add their output once ready
        for (i, q) in &t.supplies_used {
            if let Some(f) = factory.get_mut(i) {
                *f = (*f - q).max(0.0);
            }
        }
        running.push((ready, t.made.clone()));
        researched.extend(t.research.iter().cloned());
        let mut unlocks = vec![];
        if let StageDef::Milestone(m) = d {
            ship_back = submit + m.time / 60.0;
            shipped.push((submit, m.recipes.clone()));
            for r in &m.recipes {
                if unlocked.insert(r.clone()) {
                    if let Some(x) = data.recipe(r) {
                        unlocks.push(data.pick(&x.name, &x.name_zh).to_string());
                    }
                }
            }
            last_submit = submit;
        } else {
            last_submit = last_submit.max(submit);
            last_phase = submit;
        }
        player = if ss.overlap { build_end } else { submit };
        out.stages.push(Stage {
            kind: kind.into(),
            id,
            name,
            tier,
            cost: cost.iter().map(|(i, q)| ItemRate { item: i.clone(), rate: *q }).collect(),
            from_stock: take.iter().filter(|(_, q)| **q > 1e-6).map(|(i, q)| ItemRate { item: i.clone(), rate: *q }).collect(),
            research: t.research_names.clone(),
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
            recipes: t.recipes,
            raw: t.raw,
            power_mw: t.power_mw,
            note: String::new(),
        });
    }
    let clock = out.stages.iter().map(|s| s.end_min).fold(0.0, f64::max);
    out.status = if out.stages.is_empty() { Status::Infeasible } else { Status::Optimal };
    out.total_min = clock;
    out.build_min = out.stages.iter().map(|s| s.build_min).sum();
    out.idle_min = (clock - out.build_min).max(0.0);
    out.total_buildings = total_buildings;
    if ss.hours > 0.0 {
        let done: Vec<&Stage> = out.stages.iter().filter(|s| s.end_min <= ss.hours * 60.0 + 1e-9).collect();
        out.done_in_hours = Some(done.len());
        out.reached = done.last().map(|s| s.name.clone());
    }
    out.solve_ms = t0.elapsed().as_secs_f64() * 1e3;
    out
}

/// One step: the factories that make `cost` (after stock) fastest, given the recipes unlocked and the
/// output of the factories already running (free supplies). None when it cannot be made.
#[allow(clippy::too_many_arguments)]
fn solve_stage(
    data: &GameData,
    prog: &super::data::Progression,
    ss: &SpeedrunSettings,
    sec: f64,
    always: &BTreeSet<String>,
    unlocked: &BTreeSet<String>,
    researched: &BTreeSet<String>,
    mam_of: &BTreeMap<&str, &MamNode>,
    factory: &BTreeMap<String, f64>,
    cost: &[(String, f64)],
) -> Option<Trial> {
    if cost.iter().all(|(_, q)| *q <= 1e-9) {
        // the stockpile pays it all
        return Some(Trial {
            recipes: vec![], raw: vec![], power_mw: 0.0, tau: 0.0, total: 0.0, build_min: 0.0, buildings: 0.0, miners: 0.0,
            build: BTreeMap::new(), supplies_used: BTreeMap::new(), made: vec![], research: vec![], research_names: vec![],
        });
    }
    let mut allowed = unlocked.clone();
    // MAM recipes once the MAM exists (with `mam_cost`, an unresearched node is paid by the trial below)
    if unlocked.contains("Recipe_Mam_C") {
        allowed.extend(prog.mam_recipes.iter().cloned());
    }
    allowed.extend(always.iter().cloned());
    let disabled: Vec<String> = data.recipes.iter().filter(|r| !allowed.contains(&r.id)).map(|r| r.id.clone()).collect();
    // parts the unlocked recipes make; others (Portable Miner: Equipment Workshop) count as hand-crafted
    // raw resources count too (MAM nodes cost ore; review: dropping them made research free)
    let made_by_machine: BTreeSet<&str> = data.resources.iter().map(|r| r.item.as_str()).chain(
        data.recipes.iter().filter(|r| allowed.contains(&r.id)).flat_map(|r| r.outputs.iter().map(|(i, _)| i.as_str())),
    ).collect();
    // generators whose building is unlocked (their build recipe), for the step's own power
    let gens: Vec<String> = data
        .recipes
        .iter()
        .filter(|r| r.generator && prog.building_costs.get(&r.building).is_some_and(|bc| unlocked.contains(&bc.recipe)))
        .map(|r| r.id.clone())
        .collect();
    let power_on = ss.power && !gens.is_empty();
    let disabled: Vec<String> = disabled.into_iter().filter(|r| !(power_on && gens.contains(r))).collect();
    let base = PlanSettings {
        game: "satisfactory".into(),
        objective: Objective::Buildings,
        cost: Objective::Buildings.preset(),
        integer: false,
        disabled_recipes: disabled,
        allow_all_alternates: false,
        // MAM alternates (Polyester Fabric for the Gas Filter: Fabric needs hand-gathered Mycelia)
        enabled_alternates: allowed.iter().filter(|r| data.recipe(r).is_some_and(|x| x.alt)).cloned().collect(),
        self_power: power_on,
        lang: ss.lang.clone(),
        ..Default::default()
    };
    // one trial: make `cost` (plus the new buildings' own parts) in τ minutes
    let trial = |tau: f64| -> Option<Trial> {
        let mut extra: BTreeMap<String, f64> = BTreeMap::new();
        let mut last: Option<Trial> = None;
        let mut research: BTreeSet<String> = BTreeSet::new();
        for _ in 0..3 {
            let mut want: BTreeMap<String, f64> = cost.iter().cloned().collect();
            for (i, q) in &extra {
                *want.entry(i.clone()).or_default() += q;
            }
            let mut s = base.clone();
            s.targets = want.iter().map(|(i, q)| ItemRate { item: i.clone(), rate: q / tau }).collect();
            s.supplies = factory.iter().filter(|(_, r)| **r > 1e-9).map(|(i, r)| ItemRate { item: i.clone(), rate: *r }).collect();
            let mut p = plan(data, &s);
            if p.status != Status::Optimal && s.self_power {
                // no generator whose fuel these recipes can make (the biomass burner burns gathered
                // leaves / wood): power by hand-fed burners, not counted
                s.self_power = false;
                p = plan(data, &s);
            }
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
            // MAM nodes of the MAM recipes used, not researched yet, and their tree's entry node (MINE: the
            // dump has no MAM prerequisites; every tree starts at its `_0` node)
            if ss.mam_cost {
                for r in &p.recipes {
                    if let Some(n) = mam_of.get(r.id.as_str()).filter(|n| !researched.contains(&n.id)) {
                        research.insert(n.id.clone());
                        if let Some(tree) = n.id.strip_prefix("Research_").and_then(|x| x.split('_').next()) {
                            let root = format!("Research_{tree}_0_C");
                            if !researched.contains(&root) && prog.mam_nodes.iter().any(|m| m.id == root) {
                                research.insert(root);
                            }
                        }
                    }
                }
                for n in prog.mam_nodes.iter().filter(|n| research.contains(&n.id)) {
                    for (i, q) in n.cost.iter().filter(|(i, _)| made_by_machine.contains(i.as_str())) {
                        *extra.entry(i.clone()).or_default() += q;
                    }
                }
            }
            for (b, n) in &build {
                if let Some(bc) = prog.building_costs.get(b) {
                    // parts no unlocked recipe makes (the Miner's Portable Miner) are hand-crafted: left out
                    for (i, q) in bc.cost.iter().filter(|(i, _)| made_by_machine.contains(i.as_str())) {
                        *extra.entry(i.clone()).or_default() += q * n;
                    }
                }
            }
            let build_min = (buildings + miners) * sec;
            let power_mw = p.recipes.iter().flat_map(|r| r.outputs.iter()).filter(|o| o.item == super::ELECTRICITY).map(|o| o.rate).sum();
            last = Some(Trial {
                recipes: p.recipes.iter().map(|r| StageBuild { building: r.name.clone(), count: r.machines }).collect(),
                raw: p.raw.iter().map(|l| ItemRate { item: l.item.clone(), rate: l.rate }).collect(),
                power_mw,
                tau,
                total: build_min + tau,
                build_min,
                buildings,
                miners,
                build,
                supplies_used: p.supplied.iter().map(|l| (l.item.clone(), l.rate)).collect(),
                made: s.targets.iter().filter(|t| t.item != super::ELECTRICITY).cloned().collect(),
                research: research.iter().cloned().collect(),
                research_names: prog.mam_nodes.iter().filter(|n| research.contains(&n.id)).map(|n| n.name.clone()).collect(),
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
    best
}

/// Recipe ids unlocked by tier `tier` (memo step 14, report G9): the start of the game, every
/// milestone of tier ≤ `tier`, MAM recipes from tier 1 (Field Research builds the MAM; research cost
/// not counted), byproduct handling, and generators whose building is unlocked. No hard-drive
/// alternates except MAM ones.
pub fn recipes_up_to_tier(data: &GameData, tier: u32) -> Vec<String> {
    let Some(prog) = data.progression.as_ref() else { return data.recipes.iter().map(|r| r.id.clone()).collect() };
    let mut u: BTreeSet<String> = prog.tutorial_recipes.iter().cloned().chain(START.iter().map(|s| s.to_string())).collect();
    for m in prog.milestones.iter().filter(|m| m.tier <= tier) {
        u.extend(m.recipes.iter().cloned());
    }
    if u.contains("Recipe_Mam_C") {
        u.extend(prog.mam_recipes.iter().cloned());
    }
    for r in &data.recipes {
        let byproduct = r.name.starts_with("Residual") || r.name.starts_with("Unpackage");
        let generator = r.generator && prog.building_costs.get(&r.building).is_some_and(|bc| u.contains(&bc.recipe));
        if byproduct || generator {
            u.insert(r.id.clone());
        }
    }
    data.recipes.iter().filter(|r| u.contains(&r.id)).map(|r| r.id.clone()).collect()
}
