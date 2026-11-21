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

/// DSP / Factorio: every allowed machine tier, plain or with one enabled boost.
/// DSP proliferator: one spray per input item, `sprays` items per proliferator; speed,
/// extra products and power multipliers per the game. Factorio: all slots filled with one
/// module type at `module_quality` (quality plans use mixed loadouts, `quality.rs`), plus the
/// machine's built-in and researched productivity, capped at +300 %.
fn tiered_modes(data: &GameData, r: &Recipe, s: &PlanSettings) -> Vec<Mode> {
    let research = s.research_productivity.get(&r.id).copied().unwrap_or(0.0);
    let producers: Vec<&str> = if r.producers.is_empty() {
        vec![r.building.as_str()]
    } else {
        r.producers.iter().map(|p| p.as_str()).collect()
    };
    let mut out = Vec::new();
    for pid in producers {
        if s.disabled_buildings.iter().any(|d| d == pid) {
            continue;
        }
        let Some(b) = data.building(pid) else { continue };
        let tier_label = if r.producers.len() > 1 { data.pick(&b.name, &b.name_zh).to_string() } else { String::new() };
        // built-in + researched productivity (Factorio; 0 for DSP), only where productivity is allowed
        let base_p = if r.productivity_allowed { b.base_productivity + research } else { 0.0 };
        out.push(Mode {
            key: b.id.clone(),
            label: String::new(),
            building: b.id.clone(),
            speed: b.speed,
            out_mult: 1.0 + base_p.min(MAX_PRODUCTIVITY),
            power_base: b.power,
            power_exp: 1.0,
            overclock: false,
            extra_inputs: vec![],
            held: vec![],
            somersloops: 0,
            sprayed: false,
            outputs: None,
        });
        if b.modules == 0 {
            continue;
        }
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
            let mode = if e.kind == "proliferator" {
                let sprays = e.sprays.unwrap_or(1.0).max(1e-9);
                let per_work: f64 = r.inputs.iter().map(|(_, q)| q).sum::<f64>() / sprays;
                Mode {
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
                }
            } else {
                let n = b.modules as f64;
                Mode {
                    key: format!("{}|{}", b.id, e.id),
                    label: format!("{label} ×{}", b.modules),
                    building: b.id.clone(),
                    speed: b.speed * (1.0 + n * v.speed).max(0.2),
                    out_mult: 1.0 + (base_p + n * v.productivity).min(MAX_PRODUCTIVITY),
                    power_base: b.power * (1.0 + n * v.consumption).max(0.2),
                    power_exp: 1.0,
                    overclock: false,
                    extra_inputs: vec![],
                    held: vec![(e.item.clone(), n)],
                    somersloops: 0,
                    sprayed: false,
                    outputs: None,
                }
            };
            out.push(mode);
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
            }
        })
        .collect()
}
