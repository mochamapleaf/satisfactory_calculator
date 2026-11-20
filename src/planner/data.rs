//! Game data model (compact form produced by `tools/convert_data.py`).
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub liquid: bool,
    pub sink: f64,
    pub raw: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub alt: bool,
    pub building: String,
    pub time: f64,
    /// (item id, per minute at 100 % clock)
    #[serde(rename = "in")]
    pub inputs: Vec<(String, f64)>,
    #[serde(rename = "out")]
    pub outputs: Vec<(String, f64)>,
    /// Average power (MW) for variable-power recipes; otherwise the building's power is used.
    #[serde(default)]
    pub power: Option<f64>,
    #[serde(default)]
    pub power_range: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub id: String,
    pub name: String,
    pub power: f64,
    pub power_exp: f64,
    #[serde(default)]
    pub somersloop_slots: u32,
    /// Crafting speed (DSP / Factorio machine tiers); 1 for Satisfactory.
    #[serde(default = "one")]
    pub speed: f64,
    /// Recipe category this building serves (recipes name a category or a building id).
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub item: String,
    /// World maximum per minute; `None` = unlimited (water).
    pub limit: Option<f64>,
    pub extractor: String,
    pub extractor_rate: f64,
    pub extractor_power: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameData {
    /// "satisfactory" | "dsp" | "factorio" — selects the rules in `rules.rs`.
    #[serde(default = "satisfactory")]
    pub game: String,
    pub version: String,
    pub source: String,
    pub items: Vec<Item>,
    pub recipes: Vec<Recipe>,
    pub buildings: Vec<Building>,
    pub resources: Vec<Resource>,
    pub belts: Vec<f64>,
    pub pipes: Vec<f64>,
}

fn one() -> f64 {
    1.0
}
fn satisfactory() -> String {
    "satisfactory".into()
}

pub const DEFAULT_GAME_DATA: &str = include_str!("../../data/game-data.json");

impl GameData {
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
    pub fn default_data() -> Self {
        Self::from_json(DEFAULT_GAME_DATA).expect("embedded game-data.json is valid")
    }
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
    pub fn recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.id == id)
    }
    pub fn building(&self, id: &str) -> Option<&Building> {
        self.buildings.iter().find(|b| b.id == id)
    }
    pub fn resource(&self, item: &str) -> Option<&Resource> {
        self.resources.iter().find(|r| r.item == item)
    }
    /// Lookup an item id by id or (case-insensitive) display name.
    pub fn resolve_item(&self, key: &str) -> Option<&Item> {
        self.item(key)
            .or_else(|| self.items.iter().find(|i| i.name.eq_ignore_ascii_case(key)))
    }
    pub fn resolve_recipe(&self, key: &str) -> Option<&Recipe> {
        self.recipe(key)
            .or_else(|| self.recipes.iter().find(|r| r.name.eq_ignore_ascii_case(key)))
    }
    pub fn item_names(&self) -> HashMap<&str, &str> {
        self.items.iter().map(|i| (i.id.as_str(), i.name.as_str())).collect()
    }
}
