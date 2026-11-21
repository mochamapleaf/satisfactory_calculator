//! Per-game rules: the ways ("modes") a recipe can be run.
//!
//! The optimisation model in `lp.rs` is game-agnostic. A mode turns one recipe into a
//! linear activity: which building, how fast, output multiplier, extra inputs per unit of
//! work (e.g. DSP proliferator), boost items each building holds (Somersloops, modules;
//! these can be budgeted) and the power curve. Satisfactory overclocking (clock + shards)
//! is the one non-mode mechanic and is switched on per mode with `overclock`.
use super::data::{GameData, Recipe};
use super::PlanSettings;

/// Boost item id used for Somersloop budgets.
pub const SOMERSLOOP: &str = "Desc_WAT1_C";

#[derive(Clone, Debug)]
pub struct Mode {
    /// Unique within the recipe; "" for the plain mode.
    pub key: String,
    /// Shown in the UI (empty for the plain mode).
    pub label: String,
    pub building: String,
    /// Crafting speed relative to the recipe's per-minute rates.
    pub speed: f64,
    pub out_mult: f64,
    /// MW of one building at 100 % clock.
    pub power_base: f64,
    pub power_exp: f64,
    pub overclock: bool,
    /// Extra inputs per unit of work (work = buildings × speed × clock).
    pub extra_inputs: Vec<(String, f64)>,
    /// Boost items held by each building of this mode.
    pub held: Vec<(String, f64)>,
    pub somersloops: u32,
    /// Runs on proliferated inputs (needs spray coaters).
    pub sprayed: bool,
    /// Products per unit of work when they are not `recipe.outputs × out_mult` (Factorio
    /// quality modes: products split over quality tiers).
    pub outputs: Option<Vec<(String, f64)>>,
    /// Factorio beacons charged to each building of this mode (count / sharing; 0 = none).
    pub beacons: f64,
}

/// Objective cost per building of mode `m` for the modules it holds (`PlanSettings::module_costs`,
/// by the held item's quality tier; Somersloops and other boosts are not modules).
pub fn module_cost(data: &GameData, s: &PlanSettings, m: &Mode) -> f64 {
    if s.module_costs.is_empty() || data.game != "factorio" {
        return 0.0;
    }
    m.held
        .iter()
        .map(|(id, n)| {
            let (base, q) = super::quality::split(id);
            if !data.effects.iter().any(|e| e.kind == "module" && e.item == base) {
                return 0.0;
            }
            let c = s.module_costs.get(q as usize).or(s.module_costs.last()).copied().unwrap_or(0.0);
            n * if c.is_finite() && c > 0.0 { c } else { 0.0 }
        })
        .sum()
}

/// Beacon option for Factorio modes: `count` beacons around each machine, each holding the
/// beacon module; speed / consumption / quality are the summed transmitted effect.
#[derive(Clone, Debug, Default)]
pub struct BeaconOption {
    pub count: u32,
    pub speed: f64,
    pub consumption: f64,
    pub quality: f64,
    /// Beacons and beacon modules charged per machine (count / `beacon_share`), MW per machine.
    pub per_machine: f64,
    pub modules_per_machine: f64,
    pub power: f64,
    /// Held beacon module id (at `module_quality`).
    pub module: String,
}

impl BeaconOption {
    /// "8 beacons" (empty without beacons).
    pub fn label(&self) -> String {
        if self.count == 0 {
            String::new()
        } else {
            format!("{} beacon{}", self.count, if self.count == 1 { "" } else { "s" })
        }
    }
}

/// Beacon options from the settings: always "none", plus one per count in `beacon_counts` when a
/// beacon module is chosen. Transmitted effect per machine (Factorio 2.0):
///   n x effectivity(beacon quality) x profile[n-1] x slots x module effect (at `module_quality`),
/// the speed modules' quality penalty included. Beacons only reach machines with module slots.
pub fn beacon_options(data: &GameData, s: &PlanSettings) -> Vec<BeaconOption> {
    let mut out = vec![BeaconOption::default()];
    let (Some(b), Some(e)) = (data.beacons.first(), data.effect(&s.beacon_module)) else { return out };
    let v = e.tier(s.module_quality);
    if e.kind != "module" || v.productivity > 0.0 || v.quality > 0.0 {
        return out; // beacons take speed / efficiency modules only
    }
    let bt = b.tier(s.beacon_quality.min(4));
    let share = if s.beacon_share.is_finite() && s.beacon_share >= 1.0 { s.beacon_share } else { 1.0 };
    let mq = if data.qualities.is_empty() { 0 } else { s.module_quality.min(4) };
    let mut counts: Vec<u32> = s.beacon_counts.iter().copied().filter(|&n| n > 0 && n <= 64).collect();
    counts.sort_unstable();
    counts.dedup();
    for n in counts {
        let k = n as f64 * bt.effectivity * b.profile_at(n) * b.modules as f64;
        out.push(BeaconOption {
            count: n,
            speed: k * v.speed,
            consumption: k * v.consumption,
            quality: k * v.quality,
            per_machine: n as f64 / share,
            modules_per_machine: n as f64 * b.modules as f64 / share,
            power: n as f64 * bt.power / share,
            module: super::quality::qid(&e.item, mq),
        });
    }
    out
}

/// Products per unit of work of `r` run in mode `m`. Productivity (`out_mult`) only
/// multiplies the non-catalyst part of a product (Kovarex: 40 of the 41 U-235 are catalyst).
pub fn outputs(r: &Recipe, m: &Mode) -> Vec<(String, f64)> {
    if let Some(o) = &m.outputs {
        return o.clone();
    }
    r.outputs
        .iter()
        .map(|(i, q)| {
            let c = r.catalyst_of(i).min(*q);
            (i.clone(), c + (q - c) * m.out_mult)
        })
        .collect()
}

pub fn modes(data: &GameData, r: &Recipe, s: &PlanSettings) -> Vec<Mode> {
    match data.game.as_str() {
        "dsp" | "factorio" => tiered_modes(data, r, s),
        _ => satisfactory_modes(data, r, s),
    }
}

/// Factorio productivity cap (+300 %, recipe `maximum_productivity` default).
pub const MAX_PRODUCTIVITY: f64 = 3.0;

/// Researched productivity of recipe `r`. The technology changes the recipe itself, so it applies
/// whatever modules the machine or recipe accepts (scrap recycling in the recycler; FactorioLab
/// `adjustment.ts` does the same). Only recipes a technology of this data set names count: a value
/// left over from another data set (or a link) is ignored. Clamped to 0 … +300 %, non-finite = 0.
pub fn research_bonus(data: &GameData, s: &PlanSettings, r: &Recipe) -> f64 {
    if !data.research.iter().any(|f| f.recipes.iter().any(|x| *x == r.id)) {
        return 0.0;
    }
    let v = s.research_productivity.get(&r.id).copied().unwrap_or(0.0);
    if v.is_finite() {
        v.clamp(0.0, MAX_PRODUCTIVITY)
    } else {
        0.0
    }
}

/// DSP / Factorio: every allowed machine tier, plain or with one enabled boost.
/// DSP proliferator: one spray per input item, `sprays` items per proliferator; speed,
/// extra products and power multipliers per the game. Factorio: all slots filled with one
/// module type at `module_quality` (quality plans use mixed loadouts, `quality.rs`), plus the
/// machine's built-in and researched productivity, capped at +300 %.
fn tiered_modes(data: &GameData, r: &Recipe, s: &PlanSettings) -> Vec<Mode> {
    let research = research_bonus(data, s, r);
    let producers: Vec<&str> = if r.producers.is_empty() {
        vec![r.building.as_str()]
    } else {
        r.producers.iter().map(|p| p.as_str()).collect()
    };
    let beacons = beacon_options(data, s);
    let mut out = Vec::new();
    for pid in producers {
        if s.disabled_buildings.iter().any(|d| d == pid) {
            continue;
        }
        let Some(b) = data.building(pid) else { continue };
        let tier_label = if r.producers.len() > 1 { data.pick(&b.name, &b.name_zh).to_string() } else { String::new() };
        // Built-in machine productivity (foundry / EM plant / biochamber +50 %; 0 for DSP) and
        // researched productivity apply to every recipe the machine runs; the recipe flag only
        // forbids productivity modules.
        let base_p = b.base_productivity + research;
        // module fills: (key, label, speed, productivity, consumption sums, held modules); plain first
        let mut fills: Vec<(String, String, f64, f64, f64, Vec<(String, f64)>)> =
            vec![(b.id.clone(), String::new(), 0.0, 0.0, 0.0, vec![])];
        if b.modules > 0 {
            for eid in &s.effects {
                let Some(e) = data.effect(eid) else { continue };
                let v = e.tier(if e.kind == "module" { s.module_quality } else { 0 });
                if e.productivity > 0.0
                    && (!r.productivity_allowed || b.disallowed_effects.iter().any(|x| x == "productivity"))
                {
                    continue;
                }
                if e.products_only && !r.productivity_allowed {
                    continue;
                }
                let en = data.pick(&e.name, &e.name_zh);
                let label = if tier_label.is_empty() { en.to_string() } else { format!("{tier_label} · {en}") };
                if e.kind == "proliferator" {
                    let sprays = e.sprays.unwrap_or(1.0).max(1e-9);
                    let per_work: f64 = r.inputs.iter().map(|(_, q)| q).sum::<f64>() / sprays;
                    out.push(Mode {
                        key: format!("{}|{}", b.id, e.id),
                        label,
                        building: b.id.clone(),
                        speed: b.speed * (1.0 + e.speed),
                        out_mult: 1.0 + e.productivity + base_p,
                        power_base: b.power * (1.0 + e.consumption),
                        power_exp: 1.0,
                        overclock: false,
                        extra_inputs: vec![(e.item.clone(), per_work)],
                        held: vec![],
                        somersloops: 0,
                        sprayed: true,
                        outputs: None,
                        beacons: 0.0,
                    });
                    continue;
                }
                let n = b.modules as f64;
                // module quality (data sets with quality): shown on the label, held at that tier
                let mq = if data.qualities.is_empty() { 0 } else { s.module_quality.min(4) };
                let qname = if mq > 0 {
                    format!(" \u{2605}{}", data.qualities.get(mq as usize).map(|q| q.to_lowercase()).unwrap_or_default())
                } else {
                    String::new()
                };
                fills.push((
                    format!("{}|{}", b.id, e.id),
                    format!("{label}{qname} \u{d7}{}", b.modules),
                    n * v.speed,
                    n * v.productivity,
                    n * v.consumption,
                    vec![(super::quality::qid(&e.item, mq), n)],
                ));
            }
        }
        for (key, label, sp, pr, co, held) in fills {
            for bo in &beacons {
                if bo.count > 0 && b.modules == 0 {
                    continue;
                }
                let mut held = held.clone();
                if bo.count > 0 {
                    held.push((bo.module.clone(), bo.modules_per_machine));
                }
                let bl = bo.label();
                let label = [label.as_str(), bl.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" \u{b7} ");
                out.push(Mode {
                    key: if bo.count == 0 { key.clone() } else { format!("{key}|b{}", bo.count) },
                    label,
                    building: b.id.clone(),
                    speed: b.speed * (1.0 + sp + bo.speed).max(0.2),
                    out_mult: 1.0 + (base_p + pr).min(MAX_PRODUCTIVITY),
                    power_base: b.power * (1.0 + co + bo.consumption).max(0.2) + bo.power,
                    power_exp: 1.0,
                    overclock: false,
                    extra_inputs: vec![],
                    held,
                    somersloops: 0,
                    sprayed: false,
                    outputs: None,
                    beacons: bo.per_machine,
                });
            }
        }
    }
    out
}

fn satisfactory_modes(data: &GameData, r: &Recipe, s: &PlanSettings) -> Vec<Mode> {
    let b = data.building(&r.building);
    let slots = b.map(|b| b.somersloop_slots).unwrap_or(0);
    let base = r.power.unwrap_or_else(|| b.map(|b| b.power).unwrap_or(0.0));
    let exp = b.map(|b| b.power_exp).unwrap_or(1.321928);
    let levels: Vec<u32> = match s.somersloops.get(&r.id) {
        Some(&k) => vec![k.min(slots)],
        None if s.auto_somersloops => (0..=slots).collect(),
        None => vec![0],
    };
    levels
        .into_iter()
        .map(|k| {
            let frac = if slots == 0 { 0.0 } else { k as f64 / slots as f64 };
            Mode {
                key: if k == 0 { String::new() } else { format!("sloop{k}") },
                label: if k == 0 { String::new() } else { format!("{k}/{slots} Somersloop") },
                building: r.building.clone(),
                speed: 1.0,
                out_mult: 1.0 + frac,
                power_base: base * (1.0 + frac).powi(2),
                power_exp: exp,
                overclock: true,
                extra_inputs: vec![],
                held: if k == 0 { vec![] } else { vec![(SOMERSLOOP.into(), k as f64)] },
                somersloops: k,
                sprayed: false,
                outputs: None,
                beacons: 0.0,
            }
        })
        .collect()
}
