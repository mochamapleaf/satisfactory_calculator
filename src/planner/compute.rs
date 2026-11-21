//! DSP compute (UPS) cost model, from `docs/research/dsp-ups/00-最终报告.md` §4.
//!
//!   compute = Σ_r n_r·(c_bld[b_r] + aux_r) + c_gen·Σ MW
//!   aux_r   = c_sorter·k_r + (c_belt + c_station) + c_coater·[mode sprayed]
//!
//! Unit 1.0 = one assembler-class machine updated once per logic tick (vanilla). Every
//! coefficient comes from `model.json` next to the report (its `source` / `confidence`
//! notes stay there); only the assembler-class baseline has code evidence, the rest are
//! estimates. Logistics (belts, stations) are charged per machine, not per item flow: the
//! report (§4.1) shows flow-based terms undercount belts and stations by ~10×.
use super::data::Recipe;
use super::rules::Mode;
use super::{CostWeights, Objective};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The research model, embedded verbatim (single source of truth).
pub const MODEL_JSON: &str = include_str!("../../docs/research/dsp-ups/model.json");

/// Generators the player can power the plan with: model.json `generation.cost_per_mw` keys.
pub const GENERATORS: [&str; 5] = ["artificial_star", "mini_fusion", "thermal", "solar", "wind"];

#[derive(Debug, Clone)]
pub struct ComputeModel {
    /// Per building id, units per building per tick.
    pub buildings: HashMap<String, f64>,
    /// One sorter (normal path).
    pub sorter: f64,
    /// Items/min one sorter port carries before a second sorter is needed.
    pub sorter_capacity: f64,
    /// Belt + station share per machine.
    pub per_machine: f64,
    /// Spray-coater share per machine of a proliferated mode (coaters/machine × cost).
    pub coater: f64,
    /// Generator → units per MW.
    pub cost_per_mw: HashMap<String, f64>,
    pub default_cost_per_mw: f64,
    /// CPU µs (summed over threads) per unit per tick: (value, low, high).
    pub us_per_unit: (f64, f64, f64),
}

fn num(v: &Value, path: &[&str]) -> f64 {
    let mut v = v;
    for p in path {
        v = &v[*p];
    }
    v.as_f64().unwrap_or_else(|| panic!("model.json: missing number {}", path.join(".")))
}

/// Products of decimal coefficients without float noise (0.1 × 0.2 = 0.02 in the UI).
fn round9(x: f64) -> f64 {
    (x * 1e9).round() / 1e9
}

pub fn model() -> &'static ComputeModel {
    static M: OnceLock<ComputeModel> = OnceLock::new();
    M.get_or_init(|| {
        let j: Value = serde_json::from_str(MODEL_JSON).expect("model.json is valid JSON");
        let buildings = j["buildings"]
            .as_object()
            .expect("model.json: buildings")
            .iter()
            .filter_map(|(k, v)| v["cost"].as_f64().map(|c| (k.clone(), c)))
            .collect();
        let pm = &j["per_machine_terms"];
        let cost_per_mw: HashMap<String, f64> = j["generation"]["cost_per_mw"]
            .as_object()
            .expect("model.json: generation.cost_per_mw")
            .iter()
            .filter_map(|(k, v)| v.as_f64().map(|c| (k.clone(), c)))
            .collect();
        let t = &j["unit_time_estimate_us"];
        ComputeModel {
            buildings,
            sorter: num(&j, &["entities_not_in_plan", "sorter", "cost"]),
            sorter_capacity: num(pm, &["sorter_capacity_items_per_s", "value"]) * 60.0,
            per_machine: round9(num(pm, &["belt_per_machine", "value"]) + num(pm, &["station_per_machine", "value"])),
            coater: round9(
                num(pm, &["spray_coaters_per_machine", "default_with_proliferator"])
                    * num(pm, &["spray_coaters_per_machine", "cost_each"]),
            ),
            default_cost_per_mw: cost_per_mw["default"],
            cost_per_mw,
            us_per_unit: (num(t, &["value"]), t["range"][0].as_f64().unwrap_or(0.0), t["range"][1].as_f64().unwrap_or(0.0)),
        }
    })
}

impl ComputeModel {
    pub fn cost_per_mw(&self, generator: &str) -> f64 {
        self.cost_per_mw.get(generator).copied().unwrap_or(self.default_cost_per_mw)
    }

    /// Cost weights of the Compute objective. Raw resources keep the Buildings preset's
    /// tie-break weight (mining is not modelled, report §4.2).
    pub fn weights(&self, generator: &str) -> CostWeights {
        CostWeights {
            power: self.cost_per_mw(generator),
            building_weights: self.buildings.clone(),
            sorter: self.sorter,
            per_machine: self.per_machine,
            coater: self.coater,
            ..Objective::Buildings.preset()
        }
    }
}

/// Sorters per machine: Σ over ports of ceil(port rate / sorter capacity). With mixed
/// belts all inputs share one port.
pub fn sorters_per_machine(r: &Recipe, f: &Mode, mixed_belts: bool, capacity: f64) -> f64 {
    let port = |q: f64| (q / capacity - 1e-9).ceil().max(1.0);
    let ins: Vec<f64> = r.inputs.iter().map(|(_, q)| q * f.speed).collect();
    let outs = r.outputs.iter().map(|(_, q)| port(q * f.speed * f.out_mult)).sum::<f64>();
    let ins = if mixed_belts {
        if ins.is_empty() { 0.0 } else { port(ins.iter().sum()) }
    } else {
        ins.iter().map(|q| port(*q)).sum()
    };
    ins + outs
}

/// Per-machine cost of one mode under weights `w`, without power: c_bld + aux.
pub fn machine_cost(w: &CostWeights, r: &Recipe, f: &Mode, mixed_belts: bool) -> f64 {
    let bw = w.building_weights.get(&f.building).copied().unwrap_or(1.0);
    let mut aux = w.per_machine;
    if w.sorter > 0.0 {
        aux += w.sorter * sorters_per_machine(r, f, mixed_belts, model().sorter_capacity);
    }
    if f.sprayed {
        aux += w.coater;
    }
    bw + aux
}
