//! Production planner: data model, the original square incidence-matrix solver, and the
//! LP solver that generalises it (multiple recipes per item, byproducts, limits).
pub mod compute;
pub mod dsp_galaxy;
#[cfg(test)]
mod dsp_galaxy_tests;
pub mod data;
pub mod flows;
pub mod lp;
pub mod matrix;
pub mod mixed_belt;
pub mod quality;
pub mod rules;
#[cfg(test)]
mod quality_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod solve_tests;
#[cfg(test)]
mod mixed_belt_tests;

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
    /// DSP: minimise compute (UPS): per-building tick cost + sorters + logistics +
    /// generation, coefficients from the dsp-ups research model (`compute.rs`).
    Compute,
    /// User-defined weights in `cost`.
    Custom,
}

/// Linear cost function minimised by the solver:
///
///   resources·Σ w_i·raw_i + buildings·Σ w_b·n_r + power·MW + shards·Σ s_r
///   + surplus·Σ byproduct_i
///   + Σ_r n_r·(sorter·k_r + per_machine + coater·[sprayed])      (compute terms)
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
    /// Per sorter port of each building (k_r ports, see `compute::sorters_per_machine`).
    pub sorter: f64,
    /// Per building, flat (belt + logistics-station share).
    pub per_machine: f64,
    /// Per building of a proliferator mode (spray-coater share).
    pub coater: f64,
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
            Objective::Compute => return compute::model().weights(compute::GENERATORS[0]),
        };
        CostWeights {
            resources,
            buildings,
            power,
            shards,
            surplus: 0.0,
            resource_weights: HashMap::new(),
            building_weights: HashMap::new(),
            sorter: 0.0,
            per_machine: 0.0,
            coater: 0.0,
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
    /// "satisfactory" (default) | "dsp" | "factorio"
    pub game: String,
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
    /// Somersloops per recipe id (clamped to the building's slots); fixes that recipe.
    pub somersloops: HashMap<String, u32>,
    /// Let the solver place Somersloops on recipes not fixed in `somersloops`.
    pub auto_somersloops: bool,
    /// Somersloops available in total (fixed + automatic); `None` = unlimited.
    pub somersloop_budget: Option<u32>,
    /// Whole buildings and power shards (MILP). `false` = continuous LP relaxation.
    pub integer: bool,
    /// Total power shards available; `None` = unlimited.
    pub shard_budget: Option<u32>,
    /// Buildings not to use (DSP / Factorio machine tiers).
    pub disabled_buildings: Vec<String>,
    /// Enabled boosts: DSP proliferator modes, Factorio modules (effect ids).
    pub effects: Vec<String>,
    /// Used when `objective` is `Custom`.
    pub cost: CostWeights,
    /// Branch-and-bound time limit; the best plan found so far is returned on timeout.
    pub time_limit_ms: u64,
    /// Compute objective: generator powering the plan (`compute::GENERATORS`).
    pub generator: String,
    /// Compute objective: all inputs of a machine share one mixed belt (one input sorter).
    pub mixed_belts: bool,
    /// Factorio data set: "2.0" (base 2.0.77, default), "2x1", "2q1", "spa" (`data::FACTORIO_DATASETS`).
    pub dataset: String,
    /// Factorio: quality tier of all modules (0 normal … 4 legendary).
    pub module_quality: u8,
    /// Factorio quality plans: module ids for the quality / productivity slots ("" = none).
    pub quality_module: String,
    pub prod_module: String,
    /// Factorio quality plans: recycling scope "chain" (targets' ingredient chain, default),
    /// "all" (every recycling recipe) or "list" (only `recycle_allow`).
    pub recycling: String,
    /// Items (or recycling recipe ids) whose recycling is allowed in any scope.
    pub recycle_allow: Vec<String>,
    /// Researched productivity per recipe id (0.1 per level), counted against the +300 % cap.
    pub research_productivity: HashMap<String, f64>,
    /// Do not solve models with more columns than this (0 = no limit): the plan comes back
    /// with status `TooLarge` and its size, so the front end can ask before a minutes-long solve.
    pub max_columns: usize,
    /// Factorio Space Age: planets (and "space-platform") whose recipes and resources may be used.
    /// Default Nauvis only, so Gleba plants and Vulcanus calcite are not free; items from other
    /// planets can still be given as supplies. Ignored on data without locations.
    pub planets: Vec<String>,
    /// Factorio beacons: module in every beacon ("" = no beacons), beacon counts per machine
    /// offered as modes, beacon quality tier, and machines sharing one beacon (cost / power / held
    /// modules per machine are divided by it; 1 = each machine pays for all its beacons).
    pub beacon_module: String,
    pub beacon_counts: Vec<u32>,
    pub beacon_quality: u8,
    /// Factorio: quality tier of every crafting machine (0 normal … 4 legendary): crafting speed
    /// from the data's per-tier speeds (`Building::quality_speed`), power per machine unchanged.
    pub machine_quality: u8,
    pub beacon_share: f64,
    /// Factorio: objective cost of one module held by a machine (or its beacons), per module
    /// quality tier (normal … legendary; a shorter list repeats its last value). Empty = modules are
    /// free. Added to the cost function under every objective, in its units (Raw: 1 = one ore/min).
    pub module_costs: Vec<f64>,
    /// DSP mixed-belt (混带) layout tab (`mixed_belt.rs`). Not used by the solver; not named
    /// `mixed_belt` because `mixed_belts` above is the solver's sorter-cost switch.
    /// Read leniently (`mixed_belt::lenient`): a bad number in the tab must not break the solve.
    #[serde(deserialize_with = "mixed_belt::lenient")]
    pub belt_layout: mixed_belt::BeltLayoutSettings,
}

impl Default for PlanSettings {
    fn default() -> Self {
        Self {
            game: "satisfactory".into(),
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
            auto_somersloops: false,
            somersloop_budget: None,
            disabled_buildings: vec![],
            effects: vec![],
            integer: true,
            shard_budget: None,
            cost: CostWeights::default(),
            time_limit_ms: 4000,
            generator: compute::GENERATORS[0].into(),
            mixed_belts: false,
            dataset: String::new(),
            module_quality: 0,
            quality_module: "quality-module-3".into(),
            prod_module: "productivity-module-3".into(),
            recycling: "chain".into(),
            recycle_allow: vec![],
            research_productivity: HashMap::new(),
            max_columns: 0,
            planets: vec!["nauvis".into()],
            beacon_module: String::new(),
            beacon_counts: vec![8],
            beacon_quality: 0,
            machine_quality: 0,
            beacon_share: 1.0,
            module_costs: vec![],
            belt_layout: Default::default(),
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
            Objective::Compute => compute::model().weights(&self.generator),
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
    /// Model larger than `PlanSettings::max_columns`: not solved (`columns` / `rows` are set).
    #[serde(rename = "too_large")]
    TooLarge,
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
    /// Mode key ("" = plain) and its label, e.g. "2/4 Somersloop".
    pub mode: String,
    pub mode_label: String,
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
    /// DSP: this recipe's compute cost (units per tick, incl. its power), any objective.
    #[serde(default)]
    pub compute: f64,
    /// Factorio: beacons charged to this recipe (machines x beacons per machine / sharing).
    #[serde(default)]
    pub beacons: f64,
    /// Factorio quality plans: whole machines of this row's shared group (the tiers r@0 … r@4 of
    /// one base recipe on one machine type and loadout switch recipes on the same machines);
    /// `machines` is then this tier's share of them. 0 outside quality plans.
    #[serde(default)]
    pub group_machines: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingCount {
    pub building: String,
    pub count: f64,
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

/// DSP compute cost: totals in model units (1 = one machine per tick) and CPU time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputeSummary {
    pub total: f64,
    /// Buildings (c_bld), sorters, belts + stations, coaters, generation.
    pub machines: f64,
    pub sorters: f64,
    pub logistics: f64,
    pub coaters: f64,
    pub generation: f64,
    pub generator: String,
    pub cost_per_mw: f64,
    /// CPU µs per tick summed over threads: estimate and range (report §2.1).
    pub us_per_tick: f64,
    pub us_low: f64,
    pub us_high: f64,
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
    /// Buildings to place per building name (sums to `buildings`).
    #[serde(default)]
    pub building_counts: Vec<BuildingCount>,
    pub shards: f64,
    pub somersloops: f64,
    /// Integer solve: proven optimal, or the best plan found before the time limit.
    pub proven_optimal: bool,
    /// Relative gap to the best bound when not proven optimal.
    pub gap: Option<f64>,
    /// The same gap in objective weights of the cheapest building the plan uses (not a count of
    /// buildings a better plan saves: it may also trade resources). None when proven or when no
    /// building costs anything.
    pub gap_buildings: Option<f64>,
    /// Objective of the continuous relaxation (a lower bound).
    pub lp_bound: f64,
    pub solve_ms: f64,
    /// Shadow prices from the continuous relaxation.
    pub item_values: Vec<ItemValue>,
    pub flows: Vec<Flow>,
    pub warnings: Vec<String>,
    /// DSP plans: compute (UPS) cost under the dsp-ups model, whatever the objective.
    pub compute: Option<ComputeSummary>,
    /// Model size: solver columns (recipe × mode units) and item balance rows.
    pub columns: usize,
    pub rows: usize,
    /// Factorio quality expansion was used (`quality.rs`).
    pub quality: bool,
}

pub fn plan(data: &GameData, settings: &PlanSettings) -> Plan {
    lp::solve(data, settings)
}
