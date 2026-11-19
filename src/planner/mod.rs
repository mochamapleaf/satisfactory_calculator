//! Production planner: data model, the original square incidence-matrix solver, and the
//! LP solver that generalises it (multiple recipes per item, byproducts, limits).
pub mod data;
pub mod flows;
pub mod lp;
pub mod matrix;
#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use data::GameData;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Objective {
    /// Minimise raw resources, each weighted by 1 / world limit (scarcity).
    #[default]
    Resources,
    /// Minimise the number of production buildings.
    Buildings,
    /// Minimise power (production + extraction estimate).
    Power,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemRate {
    pub item: String,
    pub rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanSettings {
    pub targets: Vec<ItemRate>,
    /// Items already available (e.g. from another factory), up to `rate`, free.
    pub supplies: Vec<ItemRate>,
    pub objective: Objective,
    pub allow_all_alternates: bool,
    pub enabled_alternates: Vec<String>,
    pub disabled_recipes: Vec<String>,
    /// Override world limit per raw item id; `null` = unlimited.
    pub resource_limits: HashMap<String, Option<f64>>,
    pub ignore_limits: bool,
    /// Allow Converter recipes that turn one raw ore into another (e.g. Bauxite (Copper)).
    /// Off by default: scarcity weighting otherwise favours them despite ~250 MW each.
    pub allow_ore_conversion: bool,
    /// Clock speed in percent (1..=250) applied to every production building.
    pub clock_percent: f64,
    /// Somersloops per recipe id (clamped to the building's slots).
    pub somersloops: HashMap<String, u32>,
}

impl Default for PlanSettings {
    fn default() -> Self {
        Self {
            targets: vec![],
            supplies: vec![],
            objective: Objective::Resources,
            allow_all_alternates: false,
            enabled_alternates: vec![],
            disabled_recipes: vec![],
            resource_limits: HashMap::new(),
            ignore_limits: false,
            allow_ore_conversion: false,
            clock_percent: 100.0,
            somersloops: HashMap::new(),
        }
    }
}

impl PlanSettings {
    pub fn clock(&self) -> f64 {
        (self.clock_percent / 100.0).clamp(0.01, 2.5)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Empty,
    Optimal,
    Infeasible,
    Unbounded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLine {
    pub item: String,
    pub name: String,
    pub rate: f64,
}
impl RateLine {
    pub fn new(item: &str, name: &str, rate: f64) -> Self {
        Self { item: item.into(), name: name.into(), rate }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRecipe {
    pub id: String,
    pub name: String,
    pub alt: bool,
    pub building: String,
    /// Fractional number of machines at the chosen clock.
    pub machines: f64,
    pub machines_ceil: f64,
    /// MW for the fractional machine count (equivalently: underclocked last machine).
    pub power: f64,
    pub inputs: Vec<RateLine>,
    pub outputs: Vec<RateLine>,
    pub somersloops: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub from: String,
    pub to: String,
    pub item: String,
    pub name: String,
    pub rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Plan {
    pub status: Status,
    pub objective: f64,
    pub targets: Vec<RateLine>,
    pub recipes: Vec<PlanRecipe>,
    pub raw: Vec<RateLine>,
    pub supplied: Vec<RateLine>,
    pub surplus: Vec<RateLine>,
    pub production_power: f64,
    /// Estimate: Miner Mk.3 / extractors at 100 % on normal nodes.
    pub extraction_power: f64,
    pub buildings: f64,
    pub flows: Vec<Flow>,
    pub warnings: Vec<String>,
}

pub fn plan(data: &GameData, settings: &PlanSettings) -> Plan {
    lp::solve(data, settings)
}
