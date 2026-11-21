//! JSON-in / JSON-out API for the web front end.
use crate::planner::{self, GameData, PlanSettings};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

fn data(game: &str) -> &'static GameData {
    static SF: OnceLock<GameData> = OnceLock::new();
    static DSP: OnceLock<GameData> = OnceLock::new();
    static FAC: OnceLock<GameData> = OnceLock::new();
    let cell = match game {
        "dsp" => &DSP,
        "factorio" => &FAC,
        _ => &SF,
    };
    cell.get_or_init(|| GameData::for_game(game))
}

/// The embedded game data (items, recipes, buildings, resources, effects) as JSON.
#[wasm_bindgen]
pub fn game_data_json(game: &str) -> String {
    planner::data::embedded(game).to_string()
}

/// DSP Compute (UPS) objective weights for a generator (`artificial_star`, `wind`, …), as a
/// `CostWeights` JSON; coefficients from docs/research/dsp-ups/model.json.
#[wasm_bindgen]
pub fn compute_weights_json(generator: &str) -> String {
    serde_json::to_string(&planner::compute::model().weights(generator)).unwrap()
}

/// Solve a plan. `settings_json` is a `PlanSettings` (its `game` picks the data); returns a
/// `Plan` as JSON.
#[wasm_bindgen]
pub fn solve_json(settings_json: &str) -> String {
    let settings: PlanSettings = match serde_json::from_str(settings_json) {
        Ok(s) => s,
        Err(e) => {
            return serde_json::json!({"status": "error", "warnings": [format!("bad settings: {e}")]})
                .to_string()
        }
    };
    serde_json::to_string(&planner::plan(data(&settings.game), &settings)).unwrap()
}
