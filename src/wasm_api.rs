//! JSON-in / JSON-out API for the web front end.
use crate::planner::{self, GameData, PlanSettings};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

fn data() -> &'static GameData {
    static DATA: OnceLock<GameData> = OnceLock::new();
    DATA.get_or_init(GameData::default_data)
}

/// The embedded game data (items, recipes, buildings, resources) as JSON.
#[wasm_bindgen]
pub fn game_data_json() -> String {
    planner::data::DEFAULT_GAME_DATA.to_string()
}

/// Solve a plan. `settings_json` is a `PlanSettings`; returns a `Plan` as JSON.
#[wasm_bindgen]
pub fn solve_json(settings_json: &str) -> String {
    let settings: PlanSettings = match serde_json::from_str(settings_json) {
        Ok(s) => s,
        Err(e) => {
            return serde_json::json!({"status": "error", "warnings": [format!("bad settings: {e}")]})
                .to_string()
        }
    };
    serde_json::to_string(&planner::plan(data(), &settings)).unwrap()
}
