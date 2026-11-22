//! Game data model (compact form produced by `tools/convert_data.py`).
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    pub liquid: bool,
    pub sink: f64,
    pub raw: bool,
    /// Item picker layout: group (crafting-menu tab or derived class), row within it, sort key.
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub row: u32,
    #[serde(default)]
    pub order: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    pub alt: bool,
    /// Default building (first producer).
    pub building: String,
    /// All buildings that can run it (DSP / Factorio tiers); empty = just `building`.
    #[serde(default)]
    pub producers: Vec<String>,
    /// Productivity effects (DSP extra products, Factorio prod modules) allowed.
    #[serde(default = "yes")]
    pub productivity_allowed: bool,
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
    /// Factorio: item recycling (reverse or self recycling in the recycler). Only used in
    /// quality plans, within the recycling scope (`quality.rs`).
    #[serde(default)]
    pub recycling: bool,
    /// Quality modules allowed (Factorio `disallowedEffects`).
    #[serde(default = "yes")]
    pub quality_allowed: bool,
    /// Catalyst amount per product (per minute): productivity only multiplies the rest.
    #[serde(default)]
    pub catalyst: Vec<(String, f64)>,
    /// Space Age: planets / space platform where this recipe can run (empty = anywhere).
    #[serde(default)]
    pub locations: Vec<String>,
}

impl Recipe {
    /// Catalyst amount of `item` among the products (0 if none).
    pub fn catalyst_of(&self, item: &str) -> f64 {
        self.catalyst.iter().find(|(i, _)| i == item).map(|c| c.1).unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    /// Module / proliferator slots.
    #[serde(default)]
    pub modules: u32,
    #[serde(default)]
    pub disallowed_effects: Vec<String>,
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
    /// Built-in productivity (Factorio foundry / EM plant +50 %).
    #[serde(default)]
    pub base_productivity: f64,
    /// Factorio: fuel or heat the building burns (boilers, furnaces, reactors, biochamber
    /// nutrients, heat exchanger heat); none = electric only.
    #[serde(default)]
    pub fuel: Option<Fuel>,
    /// Space Age: where the building can be placed (crusher: space platform; empty = anywhere).
    #[serde(default)]
    pub locations: Vec<String>,
    /// Factorio: crafting speed per machine quality tier (normal … legendary), from FactorioLab
    /// `qualityRecord`; empty = speed does not depend on quality.
    #[serde(default)]
    pub quality_speed: Vec<f64>,
}

impl Building {
    /// Crafting speed of the machine at quality tier `q` (Factorio machine quality).
    pub fn speed_at(&self, q: u8) -> f64 {
        if q == 0 {
            return self.speed;
        }
        self.quality_speed.get(q as usize).copied().unwrap_or(self.speed)
    }
}

/// What a burner building consumes per minute, per building at 100 % consumption (consumption
/// modules and beacons scale it like electric power); `result` comes back one for one (spent
/// uranium fuel cells).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fuel {
    pub item: String,
    pub rate: f64,
    #[serde(default)]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub item: String,
    /// World maximum per minute; `None` = unlimited (water).
    pub limit: Option<f64>,
    pub extractor: String,
    pub extractor_rate: f64,
    pub extractor_power: f64,
    /// Cost of one unit/min; `None` = scarcity from `limit` (Satisfactory).
    #[serde(default)]
    pub weight: Option<f64>,
    /// Space Age: where this resource can be mined or pumped (empty = anywhere).
    #[serde(default)]
    pub locations: Vec<String>,
}

/// A boost: DSP proliferator mode or Factorio module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_zh: Option<String>,
    /// "proliferator" | "module"
    pub kind: String,
    #[serde(default)]
    pub speed: f64,
    #[serde(default)]
    pub productivity: f64,
    #[serde(default)]
    pub consumption: f64,
    #[serde(default)]
    pub quality: f64,
    /// Proliferator: items sprayed per proliferator item.
    #[serde(default)]
    pub sprays: Option<f64>,
    /// Item consumed (proliferator) or held (module).
    pub item: String,
    /// Only on recipes that allow productivity (DSP extra-products mode).
    #[serde(default)]
    pub products_only: bool,
    /// Factorio: values of the module at each quality tier (normal … legendary), from the
    /// data's `qualityRecord` (the game rounds, so never recomputed as ×1.3).
    #[serde(default)]
    pub by_quality: Vec<EffectTier>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct EffectTier {
    #[serde(default)]
    pub speed: f64,
    #[serde(default)]
    pub productivity: f64,
    #[serde(default)]
    pub consumption: f64,
    #[serde(default)]
    pub quality: f64,
}

impl Effect {
    /// Values of this module at quality tier `q` (0 = normal).
    pub fn tier(&self, q: u8) -> EffectTier {
        self.by_quality.get(q as usize).copied().unwrap_or(EffectTier {
            speed: self.speed,
            productivity: self.productivity,
            consumption: self.consumption,
            quality: self.quality,
        })
    }
}

/// Space Age planet or the space platform (FactorioLab `locations`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub name: String,
}

/// Researched productivity family: one infinite technology and the recipes it boosts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Research {
    pub id: String,
    pub name: String,
    pub recipes: Vec<String>,
    /// Productivity per level (0.1).
    pub step: f64,
}

/// Factorio beacon: transmits `effectivity` × its modules' effects to each machine in range,
/// times `profile[n-1]` when a machine is covered by n beacons (2.0 diminishing returns).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beacon {
    pub id: String,
    pub name: String,
    pub effectivity: f64,
    pub modules: u32,
    /// MW per beacon.
    pub power: f64,
    #[serde(default)]
    pub profile: Vec<f64>,
    #[serde(default)]
    pub disallowed_effects: Vec<String>,
    /// Per quality tier (normal … legendary): effectivity and MW.
    #[serde(default)]
    pub by_quality: Vec<BeaconTier>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct BeaconTier {
    pub effectivity: f64,
    pub power: f64,
}

impl Beacon {
    /// Effectivity and MW at quality tier `q`.
    pub fn tier(&self, q: u8) -> BeaconTier {
        self.by_quality
            .get(q as usize)
            .copied()
            .unwrap_or(BeaconTier { effectivity: self.effectivity, power: self.power })
    }
    /// Distance profile for `n` beacons (1 for n ≤ 1; last entry beyond the table).
    pub fn profile_at(&self, n: u32) -> f64 {
        if n == 0 || self.profile.is_empty() {
            return 1.0;
        }
        self.profile.get(n as usize - 1).copied().unwrap_or(*self.profile.last().unwrap())
    }
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
    #[serde(default)]
    pub effects: Vec<Effect>,
    /// Factorio quality tier names (normal … legendary); empty = no quality in this data.
    #[serde(default)]
    pub qualities: Vec<String>,
    /// Factorio data set ("2.0", "2x1", "2q1", "spa").
    #[serde(default)]
    pub dataset: String,
    /// Space Age locations; empty = no planet filter.
    #[serde(default)]
    pub locations: Vec<Location>,
    /// Researched productivity families (Space Age).
    #[serde(default)]
    pub research: Vec<Research>,
    #[serde(default)]
    pub beacons: Vec<Beacon>,
}

fn yes() -> bool {
    true
}
fn one() -> f64 {
    1.0
}
fn satisfactory() -> String {
    "satisfactory".into()
}

pub const DEFAULT_GAME_DATA: &str = include_str!("../../data/game-data.json");
pub const DSP_GAME_DATA: &str = include_str!("../../data/dsp.json");
pub const FACTORIO_GAME_DATA: &str = include_str!("../../data/factorio.json");
pub const FACTORIO_2X1: &str = include_str!("../../data/factorio-2x1.json");
pub const FACTORIO_2Q1: &str = include_str!("../../data/factorio-2q1.json");
pub const FACTORIO_SPA: &str = include_str!("../../data/factorio-spa.json");

/// Factorio data sets: (id, label). "2.0" (base 2.0.77) is the default.
pub const FACTORIO_DATASETS: [(&str, &str); 4] = [
    ("2.0", "2.0.77 base"),
    ("2x1", "2.1.25 Space Age"),
    ("2q1", "2.1.25 base + quality"),
    ("spa", "2.0.77 Space Age"),
];

/// Embedded data for a game id ("satisfactory", "dsp", "factorio").
pub fn embedded(game: &str) -> &'static str {
    embedded_set(game, "")
}

/// Embedded data for a game and data set ("" = the game's default).
pub fn embedded_set(game: &str, dataset: &str) -> &'static str {
    match (game, dataset) {
        ("dsp", _) => DSP_GAME_DATA,
        ("factorio", "2x1") => FACTORIO_2X1,
        ("factorio", "2q1") => FACTORIO_2Q1,
        ("factorio", "spa") => FACTORIO_SPA,
        ("factorio", _) => FACTORIO_GAME_DATA,
        _ => DEFAULT_GAME_DATA,
    }
}

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
        let key = super::quality::split(key).0;
        self.item(key)
            .or_else(|| self.items.iter().find(|i| i.name.eq_ignore_ascii_case(key)))
            .or_else(|| self.items.iter().find(|i| i.name_zh.as_deref() == Some(key)))
    }
    /// Available under the planet filter: data without locations, untagged, or on a chosen planet.
    pub fn located(&self, locations: &[String], planets: &[String]) -> bool {
        self.locations.is_empty() || locations.is_empty() || locations.iter().any(|l| planets.contains(l))
    }
    /// Recipe `r` can run under the planet filter: the recipe is located and some machine that
    /// runs it (not disabled) can be placed on a chosen planet.
    pub fn recipe_located(&self, r: &Recipe, planets: &[String], disabled_buildings: &[String]) -> bool {
        if !self.located(&r.locations, planets) {
            return false;
        }
        let producers: Vec<&String> = if r.producers.is_empty() { vec![&r.building] } else { r.producers.iter().collect() };
        producers.iter().any(|p| {
            !disabled_buildings.contains(p) && self.building(p).map(|b| self.located(&b.locations, planets)).unwrap_or(true)
        })
    }
    pub fn beacon(&self, id: &str) -> Option<&Beacon> {
        self.beacons.iter().find(|b| b.id == id)
    }
    pub fn effect(&self, id: &str) -> Option<&Effect> {
        self.effects.iter().find(|e| e.id == id)
    }
    pub fn for_game(game: &str) -> Self {
        Self::from_json(embedded(game)).expect("embedded game data is valid")
    }
    pub fn for_set(game: &str, dataset: &str) -> Self {
        Self::from_json(embedded_set(game, dataset)).expect("embedded game data is valid")
    }
    /// Display name of an item id, with its quality for `x@q` ids ("Iron gear wheel ★legendary").
    pub fn item_name(&self, id: &str) -> String {
        let (base, q) = super::quality::split(id);
        let n = self.item(base).map(|i| self.pick(&i.name, &i.name_zh)).unwrap_or(base);
        if q == 0 {
            n.to_string()
        } else {
            format!("{n} ★{}", super::quality::tier_name(self, q))
        }
    }
    pub fn resolve_recipe(&self, key: &str) -> Option<&Recipe> {
        self.recipe(key)
            .or_else(|| self.recipes.iter().find(|r| r.name.eq_ignore_ascii_case(key)))
    }
    /// Display names (Chinese for DSP when available).
    pub fn item_names(&self) -> HashMap<&str, &str> {
        self.items.iter().map(|i| (i.id.as_str(), self.pick(&i.name, &i.name_zh))).collect()
    }
    pub fn pick<'a>(&self, en: &'a str, zh: &'a Option<String>) -> &'a str {
        match (self.game.as_str(), zh) {
            ("dsp", Some(z)) if !z.is_empty() => z.as_str(),
            _ => en,
        }
    }
}
