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
}

/// Buildings that can run a recipe: `recipe.building` is a building id or a category.
pub fn buildings_for<'a>(data: &'a GameData, r: &Recipe) -> Vec<&'a super::data::Building> {
    let v: Vec<_> = data
        .buildings
        .iter()
        .filter(|b| b.id == r.building || b.category.as_deref() == Some(r.building.as_str()))
        .collect();
    v
}

pub fn modes(data: &GameData, r: &Recipe, s: &PlanSettings) -> Vec<Mode> {
    match data.game.as_str() {
        _ => satisfactory_modes(data, r, s),
    }
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
            }
        })
        .collect()
}
