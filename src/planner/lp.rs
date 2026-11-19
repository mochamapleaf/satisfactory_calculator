//! LP formulation of the production plan.
//!
//! Same incidence matrix A as `matrix.rs` (rows = recipes, cols = items), but instead of
//! inverting a square Aᵀ we solve
//!
//!   min  cᵀx + wᵀs + εᵀe
//!   s.t. Aᵀx + s + u − e = b          (one equality per item)
//!        0 ≤ s ≤ world limit           (raw extraction)
//!        0 ≤ u ≤ user supply           (items already available)
//!        x, e ≥ 0                      (machines, surplus / byproducts)
//!
//! which handles several recipes per item, byproducts and cycles. When the chain is
//! one-recipe-per-item the optimum equals the square-matrix solution.
use super::data::{GameData, Recipe};
use super::{Objective, Plan, PlanRecipe, PlanSettings, RateLine, Status};
use minilp::{ComparisonOp, OptimizationDirection, Problem, Variable};
use std::collections::{BTreeMap, BTreeSet, HashMap};

const EPS_SURPLUS: f64 = 0.0; // byproducts are free to leave the factory (sink them)
const EPS_BUILDING: f64 = 1e-5;
const WATER_WEIGHT: f64 = 1e-4;

pub struct RecipeFactors {
    pub out_mult: f64,
    pub power_per_machine: f64,
}

/// Output multiplier and per-machine power of a recipe under the settings.
pub fn recipe_factors(data: &GameData, r: &Recipe, s: &PlanSettings) -> RecipeFactors {
    let b = data.building(&r.building);
    let slots = b.map(|b| b.somersloop_slots).unwrap_or(0);
    let filled = s.somersloops.get(&r.id).copied().unwrap_or(0).min(slots);
    let frac = if slots == 0 { 0.0 } else { filled as f64 / slots as f64 };
    let base = r.power.unwrap_or_else(|| b.map(|b| b.power).unwrap_or(0.0));
    let exp = b.map(|b| b.power_exp).unwrap_or(1.321928);
    let clock = s.clock();
    RecipeFactors {
        out_mult: (1.0 + frac) * clock,
        // per *physical* machine at the chosen clock
        power_per_machine: base * (1.0 + frac).powi(2) * clock.powf(exp),
    }
}

/// Recipes allowed by the settings.
pub fn allowed_recipes<'a>(data: &'a GameData, s: &PlanSettings) -> Vec<&'a Recipe> {
    data.recipes
        .iter()
        .filter(|r| !s.disabled_recipes.contains(&r.id))
        .filter(|r| !r.alt || s.allow_all_alternates || s.enabled_alternates.contains(&r.id))
        .filter(|r| s.allow_ore_conversion || !is_ore_conversion(data, r))
        .collect()
}

/// Converter recipe whose product is a raw resource (ore → ore).
pub fn is_ore_conversion(data: &GameData, r: &Recipe) -> bool {
    r.building == "Desc_Converter_C"
        && r.outputs.iter().any(|(it, _)| data.item(it).map(|i| i.raw).unwrap_or(false))
}

/// Recipes that can contribute to the targets (upstream closure).
fn relevant<'a>(recipes: &[&'a Recipe], targets: &[String]) -> Vec<&'a Recipe> {
    let mut needed: BTreeSet<String> = targets.iter().cloned().collect();
    let mut chosen: BTreeSet<usize> = BTreeSet::new();
    loop {
        let mut grew = false;
        for (i, r) in recipes.iter().enumerate() {
            if chosen.contains(&i) {
                continue;
            }
            if r.outputs.iter().any(|(it, _)| needed.contains(it)) {
                chosen.insert(i);
                for (it, _) in &r.inputs {
                    needed.insert(it.clone());
                }
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    chosen.into_iter().map(|i| recipes[i]).collect()
}

pub fn solve(data: &GameData, s: &PlanSettings) -> Plan {
    let mut plan = Plan::default();
    let names = data.item_names();
    let name = |id: &str| names.get(id).copied().unwrap_or(id).to_string();

    // resolve targets
    let mut targets: BTreeMap<String, f64> = BTreeMap::new();
    for t in &s.targets {
        match data.resolve_item(&t.item) {
            Some(it) => *targets.entry(it.id.clone()).or_insert(0.0) += t.rate,
            None => plan.warnings.push(format!("unknown target item '{}'", t.item)),
        }
    }
    let mut supplies: BTreeMap<String, f64> = BTreeMap::new();
    for t in &s.supplies {
        if let Some(it) = data.resolve_item(&t.item) {
            *supplies.entry(it.id.clone()).or_insert(0.0) += t.rate;
        }
    }
    if targets.is_empty() {
        plan.status = Status::Empty;
        return plan;
    }

    let allowed = allowed_recipes(data, s);
    let target_ids: Vec<String> = targets.keys().cloned().collect();
    let recipes = relevant(&allowed, &target_ids);

    // all items touched
    let mut items: BTreeSet<String> = targets.keys().cloned().collect();
    items.extend(supplies.keys().cloned());
    for r in &recipes {
        for (it, _) in r.inputs.iter().chain(r.outputs.iter()) {
            items.insert(it.clone());
        }
    }

    let mut p = Problem::new(OptimizationDirection::Minimize);
    let factors: Vec<RecipeFactors> = recipes.iter().map(|r| recipe_factors(data, r, s)).collect();

    // x_r: physical machines at the chosen clock
    let xs: Vec<Variable> = recipes
        .iter()
        .zip(&factors)
        .map(|(_r, f)| {
            let c = match s.objective {
                Objective::Buildings => 1.0,
                Objective::Power => f.power_per_machine.max(1e-3),
                Objective::Resources => EPS_BUILDING,
            };
            p.add_var(c, (0.0, f64::INFINITY))
        })
        .collect();

    // raw extraction variables
    let mut raw_vars: Vec<(String, Variable)> = Vec::new();
    for it in &items {
        if let Some(res) = data.resource(it) {
            let limit = s
                .resource_limits
                .get(it)
                .copied()
                .flatten()
                .or(if s.resource_limits.contains_key(it) { None } else { res.limit });
            let limit = if s.ignore_limits { None } else { limit };
            let w = match res.limit {
                Some(l) if l > 0.0 => 1000.0 / l,
                _ => WATER_WEIGHT,
            };
            let ext_power = res.extractor_power / res.extractor_rate;
            let c = match s.objective {
                Objective::Resources => w,
                Objective::Buildings => 1e-3 * w,
                Objective::Power => ext_power + 1e-3 * w,
            };
            let v = p.add_var(c, (0.0, limit.unwrap_or(f64::INFINITY)));
            raw_vars.push((it.clone(), v));
        }
    }
    let sup_vars: Vec<(String, Variable)> = supplies
        .iter()
        .map(|(it, q)| (it.clone(), p.add_var(0.0, (0.0, *q))))
        .collect();
    let surplus_vars: Vec<(String, Variable)> = items
        .iter()
        .map(|it| (it.clone(), p.add_var(EPS_SURPLUS, (0.0, f64::INFINITY))))
        .collect();

    // item balance rows
    for it in &items {
        let mut row: Vec<(Variable, f64)> = Vec::new();
        for ((r, f), &x) in recipes.iter().zip(&factors).zip(&xs) {
            let mut a = 0.0;
            for (i, q) in &r.outputs {
                if i == it {
                    a += q * f.out_mult;
                }
            }
            for (i, q) in &r.inputs {
                if i == it {
                    a -= q * s.clock();
                }
            }
            if a != 0.0 {
                row.push((x, a));
            }
        }
        for (i, v) in raw_vars.iter().chain(sup_vars.iter()) {
            if i == it {
                row.push((*v, 1.0));
            }
        }
        for (i, v) in &surplus_vars {
            if i == it {
                row.push((*v, -1.0));
            }
        }
        let b = targets.get(it).copied().unwrap_or(0.0);
        p.add_constraint(row.as_slice(), ComparisonOp::Eq, b);
    }

    let sol = match p.solve() {
        Ok(sol) => sol,
        Err(minilp::Error::Infeasible) => {
            plan.status = Status::Infeasible;
            plan.warnings.push(
                "No feasible plan: targets exceed resource limits or need a disabled recipe.".into(),
            );
            return plan;
        }
        Err(minilp::Error::Unbounded) => {
            plan.status = Status::Unbounded;
            return plan;
        }
    };
    plan.status = Status::Optimal;
    plan.objective = sol.objective();

    let clean = |v: f64| if v.abs() < 1e-7 { 0.0 } else { v };
    for ((r, f), &x) in recipes.iter().zip(&factors).zip(&xs) {
        let m = clean(sol[x]);
        if m <= 0.0 {
            continue;
        }
        let b = data.building(&r.building);
        plan.recipes.push(PlanRecipe {
            id: r.id.clone(),
            name: r.name.clone(),
            alt: r.alt,
            building: b.map(|b| b.name.clone()).unwrap_or_else(|| r.building.clone()),
            machines: m,
            machines_ceil: (m - 1e-6).ceil(),
            power: m * f.power_per_machine,
            inputs: r.inputs.iter().map(|(i, q)| RateLine::new(i, &name(i), q * s.clock() * m)).collect(),
            outputs: r.outputs.iter().map(|(i, q)| RateLine::new(i, &name(i), q * f.out_mult * m)).collect(),
            somersloops: s.somersloops.get(&r.id).copied().unwrap_or(0),
        });
    }
    plan.recipes.sort_by(|a, b| a.name.cmp(&b.name));
    for (it, v) in &raw_vars {
        let q = clean(sol[*v]);
        if q > 0.0 {
            let res = data.resource(it).unwrap();
            plan.extraction_power += q / res.extractor_rate * res.extractor_power;
            plan.raw.push(RateLine::new(it, &name(it), q));
        }
    }
    for (it, v) in &sup_vars {
        let q = clean(sol[*v]);
        if q > 0.0 {
            plan.supplied.push(RateLine::new(it, &name(it), q));
        }
    }
    for (it, v) in &surplus_vars {
        let q = clean(sol[*v]);
        if q > 1e-6 {
            plan.surplus.push(RateLine::new(it, &name(it), q));
        }
    }
    for (it, q) in &targets {
        plan.targets.push(RateLine::new(it, &name(it), *q));
    }
    plan.production_power = plan.recipes.iter().map(|r| r.power).sum();
    plan.buildings = plan.recipes.iter().map(|r| r.machines_ceil).sum();
    plan.flows = super::flows::allocate(&plan);
    plan
}

/// Map from item id to total raw input (handy in tests).
pub fn raw_map(plan: &Plan) -> HashMap<String, f64> {
    plan.raw.iter().map(|r| (r.item.clone(), r.rate)).collect()
}
