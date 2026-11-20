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

/// Cost-function preset. `Custom` uses `PlanSettings::cost` as given; the others fill it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Objective {
    /// Minimise raw resources, each weighted by scarcity (Iron Ore = 1).
    #[default]
    Resources,
    /// Minimise the number of production buildings.
    Buildings,
    /// Minimise power (production + extraction); one building costs as much as 1 MW.
    Power,
    /// User-defined weights in `cost`.
    Custom,
}

/// Linear cost function minimised by the solver:
///
///   resources·Σ w_i·raw_i + buildings·Σ w_b·n_r + power·MW + shards·Σ s_r
///   + surplus·Σ byproduct_i
///
/// Rates are per minute. Per-item and per-building weights default to scarcity / 1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct CostWeights {
    pub resources: f64,
    pub buildings: f64,
    pub power: f64,
    pub shards: f64,
    pub surplus: f64,
    /// Per raw item id, cost per unit/min (default: 92 100 / world limit, water 0.001).
    pub resource_weights: HashMap<String, f64>,
    /// Per building id, cost per building (default 1).
    pub building_weights: HashMap<String, f64>,
}

impl Default for CostWeights {
    fn default() -> Self {
        Objective::Resources.preset()
    }
}

impl Objective {
    pub fn preset(self) -> CostWeights {
        let (resources, buildings, power, shards) = match self {
            Objective::Resources | Objective::Custom => (1.0, 0.01, 0.0, 0.01),
            Objective::Buildings => (0.001, 1.0, 0.0, 0.01),
            Objective::Power => (0.001, 1.0, 1.0, 0.01),
        };
        CostWeights {
            resources,
            buildings,
            power,
            shards,
            surplus: 0.0,
            resource_weights: HashMap::new(),
            building_weights: HashMap::new(),
        }
    }
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
    /// Highest clock speed in percent (1..=250). Above 100 % a machine needs one power
    /// shard per started 50 %. Machines may always run slower (underclock) for free.
    #[serde(alias = "clock_percent")]
    pub max_clock_percent: f64,
    /// Somersloops per recipe id (clamped to the building's slots).
    pub somersloops: HashMap<String, u32>,
    /// Whole buildings and power shards (MILP). `false` = continuous LP relaxation.
    pub integer: bool,
    /// Total power shards available; `None` = unlimited.
    pub shard_budget: Option<u32>,
    /// Used when `objective` is `Custom`.
    pub cost: CostWeights,
    /// Branch-and-bound time limit; the best plan found so far is returned on timeout.
    pub time_limit_ms: u64,
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
            max_clock_percent: 100.0,
            somersloops: HashMap::new(),
            integer: true,
            shard_budget: None,
            cost: CostWeights::default(),
            time_limit_ms: 4000,
        }
    }
}

impl PlanSettings {
    pub fn max_clock(&self) -> f64 {
        (self.max_clock_percent / 100.0).clamp(0.01, 2.5)
    }
    /// Effective cost weights (preset or custom).
    pub fn weights(&self) -> CostWeights {
        match self.objective {
            Objective::Custom => self.cost.clone(),
            o => o.preset(),
        }
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
    /// Buildings to place (whole number in integer mode).
    pub machines: f64,
    pub machines_ceil: f64,
    /// Work done, in machines at 100 % clock (= machines × average clock).
    pub throughput: f64,
    /// Power shards used.
    pub shards: f64,
    /// How to set the machines up: groups of identical clock / shard count.
    pub groups: Vec<MachineGroup>,
    /// MW for this recipe with the clocks in `groups`.
    pub power: f64,
    pub inputs: Vec<RateLine>,
    pub outputs: Vec<RateLine>,
    pub somersloops: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineGroup {
    pub count: f64,
    pub clock_percent: f64,
    pub shards: u32,
}

/// Marginal cost of one more unit/min of an item, in cost-function units (LP dual).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemValue {
    pub item: String,
    pub name: String,
    pub value: f64,
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
    pub shards: f64,
    /// Integer solve: proven optimal, or the best plan found before the time limit.
    pub proven_optimal: bool,
    /// Relative gap to the best bound when not proven optimal.
    pub gap: Option<f64>,
    /// Objective of the continuous relaxation (a lower bound).
    pub lp_bound: f64,
    pub solve_ms: f64,
    /// Shadow prices from the continuous relaxation.
    pub item_values: Vec<ItemValue>,
    pub flows: Vec<Flow>,
    pub warnings: Vec<String>,
}

pub fn plan(data: &GameData, settings: &PlanSettings) -> Plan {
    lp::solve(data, settings)
}
