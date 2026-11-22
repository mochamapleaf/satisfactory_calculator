//! Optimisation model of the production plan (LP relaxation + MILP).
//!
//! Same incidence matrix A as `matrix.rs` (rows = recipes, cols = items). Per recipe r:
//!
//!   x_r  work done, in machines at 100 % clock (continuous)
//!   n_r  buildings placed                      (integer in MILP mode)
//!   s_r  power shards                          (integer in MILP mode)
//!   p_r  MW drawn                              (only when power has a weight)
//!
//!   min  Σ_i w_i·raw_i + Σ_r w_b·n_r + w_P·(Σ p_r + extraction MW) + w_s·Σ s_r + w_e·Σ e_i
//!   s.t. Σ_r A_ri·x_r + raw_i + u_i − e_i = target_i      (item balance)
//!        x_r ≤ n_r + 0.5·s_r,  x_r ≤ c_max·n_r,  s_r ≤ k_max·n_r   (clock / shards)
//!        p_r ≥ P_r·(a_k·x_r + b_k·n_r)  for each segment k   (power, see below)
//!        0 ≤ raw_i ≤ limit_i,  0 ≤ u_i ≤ supply_i,  Σ s_r ≤ shard budget
//!
//! Power: one machine at clock c draws P·c^1.32. With n machines sharing work x the
//! cheapest split is equal clocks, n·P·(x/n)^1.32, the perspective of a convex function.
//! Replacing c^1.32 by its piecewise-linear interpolant g(c) = max_k(a_k c + b_k) makes
//! n·g(x/n) = max_k(a_k x + b_k n): linear, exact at the breakpoints, slightly high
//! between them. Reported power is computed exactly from the final clocks.
//!
//! Byproducts leave through e_i. One-recipe-per-item chains reduce to the square-matrix
//! solution of `matrix.rs` (in LP mode).
use super::data::{GameData, Recipe};
use super::compute;
use super::quality;
use super::rules::{self, Mode};

use super::{
    BuildingCount, ComputeSummary, CostWeights, ItemValue, MachineGroup, Plan, PlanRecipe, PlanSettings,
    RateLine, Status,
};
use microlp::{
    ComparisonOp, OptimizationDirection, Problem, Solution, SolveOptions, SolveOutcome, TerminationReason,
    Variable,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

/// Breakpoints (clock as a fraction) of the piecewise-linear power curve.
const POWER_BREAKS: [f64; 9] = [0.0, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 2.5];
const DEFAULT_WATER_WEIGHT: f64 = 0.001;
/// Iron Ore world limit: scarcity weights are relative to it (Iron Ore = 1).
const SCARCITY_REF: f64 = 92_100.0;
const MAX_BUILDINGS: i32 = 1_000_000;
/// Relative gap at which a whole-building plan counts as optimal, unless that is more than one of
/// its cheapest buildings in objective units (`Run::unit_tol`).
const MIP_GAP: f64 = 1e-4;

/// Recipes allowed by the settings.
pub fn allowed_recipes<'a>(data: &'a GameData, s: &PlanSettings) -> Vec<&'a Recipe> {
    data.recipes
        .iter()
        .filter(|r| !s.disabled_recipes.contains(&r.id))
        .filter(|r| !r.alt || s.allow_all_alternates || s.enabled_alternates.contains(&r.id))
        .filter(|r| s.allow_ore_conversion || !is_ore_conversion(data, r))
        .filter(|r| data.located(&r.locations, &s.planets))
        .collect()
}

/// Converter recipe whose product is a raw resource (ore → ore).
pub fn is_ore_conversion(data: &GameData, r: &Recipe) -> bool {
    r.building == "Desc_Converter_C"
        && r.outputs.iter().any(|(it, _)| data.item(it).map(|i| i.raw).unwrap_or(false))
}

/// Units that can contribute to the targets (upstream closure, extra inputs included).
fn relevant(units: &[Unit], targets: &[String]) -> Vec<Unit> {
    let mut by_out: HashMap<&str, Vec<usize>> = HashMap::new();
    for (k, u) in units.iter().enumerate() {
        for (it, q) in &u.outs {
            if *q > 0.0 {
                by_out.entry(it.as_str()).or_default().push(k);
            }
        }
    }
    let mut needed: BTreeSet<&str> = targets.iter().map(|t| t.as_str()).collect();
    let mut queue: Vec<&str> = needed.iter().copied().collect();
    let mut chosen = vec![false; units.len()];
    while let Some(it) = queue.pop() {
        for &k in by_out.get(it).map(|v| v.as_slice()).unwrap_or(&[]) {
            if chosen[k] {
                continue;
            }
            chosen[k] = true;
            for (i, _) in &units[k].ins {
                if needed.insert(i.as_str()) {
                    queue.push(i.as_str());
                }
            }
        }
    }
    units.iter().zip(&chosen).filter(|(_, c)| **c).map(|(u, _)| u.clone()).collect()
}

/// Items that block an infeasible plan: unobtainable leaves (no unit makes them, not supplied, no
/// resource on the chosen planets) reached from the targets through items that cannot be made.
/// An item counts as obtainable when supplied, mined on a chosen planet, or made by a unit whose
/// inputs are all obtainable (forward fixed point); limits are ignored.
fn blocking_items(data: &GameData, s: &PlanSettings, inp: &Inputs) -> Vec<String> {
    let mined = |it: &str| {
        data.resource(quality::split(it).0).is_some_and(|r| data.located(&r.locations, &s.planets))
    };
    let mut ok: BTreeSet<&str> =
        inp.items.iter().map(|i| i.as_str()).filter(|i| inp.supplies.contains_key(*i) || mined(i)).collect();
    loop {
        let before = ok.len();
        for u in &inp.units {
            if u.ins.iter().all(|(i, _)| ok.contains(i.as_str())) {
                ok.extend(u.outs.iter().filter(|o| o.1 > 0.0).map(|o| o.0.as_str()));
            }
        }
        if ok.len() == before {
            break;
        }
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut queue: Vec<&str> = inp.targets.keys().map(|t| t.as_str()).filter(|t| !ok.contains(t)).collect();
    let mut leaves = Vec::new();
    while let Some(it) = queue.pop() {
        if !seen.insert(it) {
            continue;
        }
        let makers: Vec<&Unit> = inp.units.iter().filter(|u| u.outs.iter().any(|o| o.0 == it && o.1 > 0.0)).collect();
        if makers.is_empty() {
            leaves.push(it.to_string());
        }
        for u in makers {
            queue.extend(u.ins.iter().map(|i| i.0.as_str()).filter(|i| !ok.contains(i)));
        }
    }
    leaves.sort();
    leaves
}

/// Every item a set of units touches, plus targets and supplies.
fn items_of(
    units: &[Unit],
    targets: &BTreeMap<String, f64>,
    supplies: &BTreeMap<String, f64>,
) -> BTreeSet<String> {
    let mut items: BTreeSet<String> = targets.keys().cloned().collect();
    items.extend(supplies.keys().cloned());
    for u in units {
        for (it, _) in u.ins.iter().chain(&u.outs) {
            items.insert(it.clone());
        }
    }
    items
}

/// Budgets on boost items held by buildings.
fn budgets(s: &PlanSettings) -> Vec<(String, f64)> {
    s.somersloop_budget.map(|b| vec![(rules::SOMERSLOOP.to_string(), b as f64)]).unwrap_or_default()
}

/// Cost of one unit/min of a raw resource under the weights.
pub fn resource_weight(data: &GameData, w: &CostWeights, item: &str) -> f64 {
    if let Some(v) = w.resource_weights.get(item) {
        return *v;
    }
    if let Some(v) = data.resource(item).and_then(|r| r.weight) {
        return v;
    }
    match data.resource(item).and_then(|r| r.limit) {
        Some(l) if l > 0.0 => SCARCITY_REF / l,
        _ => DEFAULT_WATER_WEIGHT,
    }
}

/// Secant lines (slope, intercept) of c^exp between consecutive breakpoints.
fn power_segments(exp: f64, c_max: f64) -> Vec<(f64, f64)> {
    let mut pts: Vec<f64> = POWER_BREAKS.iter().copied().filter(|&c| c < c_max - 1e-9).collect();
    pts.push(c_max);
    pts.windows(2)
        .map(|w| {
            let (c0, c1) = (w[0], w[1]);
            let (g0, g1) = (c0.powf(exp), c1.powf(exp));
            let a = (g1 - g0) / (c1 - c0);
            (a, g0 - a * c0)
        })
        .collect()
}

struct RecipeVars {
    x: Variable,
    n: Variable,
    s: Option<Variable>,
    p: Option<Variable>,
}

struct Model {
    p: Problem,
    /// Objective coefficient of every variable, by index (objective of a point without a solve).
    cost: Vec<f64>,
    /// Bounds and integrality of every variable, and every row (for `lp_text`).
    bounds: Vec<(f64, f64, bool)>,
    rows: Vec<(Vec<(usize, f64)>, ComparisonOp, f64)>,
    rv: Vec<RecipeVars>,
    raw: Vec<(String, Variable)>,
    sup: Vec<(String, Variable)>,
    surplus: Vec<(String, Variable)>,
    balance: Vec<(String, microlp::Constraint)>,
    segments: Vec<Vec<(f64, f64)>>,
    /// Shared-machine groups (`Inputs::shared`): whole machines per group key.
    groups: Vec<(String, Variable)>,
}

/// One way of running one recipe: a column of the model.
#[derive(Clone)]
struct Unit {
    recipe: Arc<Recipe>,
    mode: Mode,
    key: String,
    /// Inputs (incl. the mode's extra inputs) and products per unit of work.
    ins: Vec<(String, f64)>,
    outs: Vec<(String, f64)>,
}

impl Unit {
    fn new(recipe: Arc<Recipe>, mode: Mode) -> Self {
        let ins = recipe.inputs.iter().chain(&mode.extra_inputs).cloned().collect();
        let mut outs = rules::outputs(&recipe, &mode);
        outs.extend(mode.extra_outputs.iter().cloned());
        let key = format!("{}#{}", recipe.id, mode.key);
        Unit { recipe, key, mode, ins, outs }
    }
}

#[derive(Clone)]
struct Inputs {
    units: Vec<Unit>,
    items: BTreeSet<String>,
    targets: BTreeMap<String, f64>,
    supplies: BTreeMap<String, f64>,
    weights: CostWeights,
    /// Quality plans with whole buildings (report §2.1, §5 step 6): the tier columns of one base
    /// recipe on one machine type and loadout share machines that switch recipes. Each column keeps
    /// a continuous share n_c >= x_c / speed; one integer N per `group_key` carries the building
    /// cost, with sum_c n_c <= N.
    shared: bool,
}

/// Shared-machine group of a unit: base recipe (tier suffix dropped) and mode (machine + loadout
/// + beacons). Every column of a group has the same speed, power and held modules.
fn group_key(u: &Unit) -> String {
    format!("{}#{}", quality::split(&u.recipe.id).0, u.mode.key)
}

/// Tolerance on a group's whole machines: work of up to this many machines above a whole number
/// still fits (solver feasibility tolerance; the same in `finish`).
const WHOLE_TOL: f64 = 1e-6;

/// Whole machines per shared group at `pt`: ⌈Σ tier shares⌉.
fn group_machines(inp: &Inputs, pt: &Point) -> HashMap<String, f64> {
    let mut out: HashMap<String, f64> = HashMap::new();
    for u in &inp.units {
        if let Some(v) = pt.rec.get(&u.key) {
            *out.entry(group_key(u)).or_insert(0.0) += v.1;
        }
    }
    for n in out.values_mut() {
        *n = (*n - WHOLE_TOL).ceil().max(0.0);
    }
    out
}

fn shards_per_machine(c_max: f64) -> u32 {
    if c_max <= 1.0 + 1e-9 {
        0
    } else {
        ((c_max - 1.0) / 0.5 - 1e-9).ceil() as u32
    }
}

/// Machine power is linear in work (`power_exp` = 1, no overclocking): no power variable and no
/// segment rows; the MW go straight onto the objective of x and n. Exact (report §4.2): at
/// c_max = 1 every segment reads p ≥ P·x/speed + beacons·n, and p appears only in the objective.
fn linear_power(w: &CostWeights, f: &Mode) -> bool {
    shrink() && w.power > 0.0 && f.power_exp == 1.0 && !f.overclock
}

/// Objective weight of one building of `u`: building weight with the per-building compute terms,
/// modules, and the beacon MW when power is linear.
fn building_weight(data: &GameData, s: &PlanSettings, w: &CostWeights, u: &Unit) -> f64 {
    w.buildings * compute::machine_cost(w, &u.recipe, &u.mode, s.mixed_belts)
        + rules::module_cost(data, s, &u.mode)
        + if linear_power(w, &u.mode) { w.power * u.mode.beacon_power } else { 0.0 }
}

fn build(data: &GameData, s: &PlanSettings, inp: &Inputs, integer: bool) -> Model {
    build_scaled(data, s, inp, integer, &HashMap::new(), &HashMap::new())
}

/// `scale`: per-recipe multiplier on the building cost (heuristic re-weighting).
/// `n_cap`: upper bounds on the buildings of some units (budget fallback).
fn build_scaled(
    data: &GameData,
    s: &PlanSettings,
    inp: &Inputs,
    integer: bool,
    scale: &HashMap<String, f64>,
    n_cap: &HashMap<String, f64>,
) -> Model {
    let w = &inp.weights;
    let c_max = s.max_clock();
    let k_max = shards_per_machine(c_max);
    let mut p = Problem::new(OptimizationDirection::Minimize);
    let mut cost: Vec<f64> = Vec::new();
    let mut bounds: Vec<(f64, f64, bool)> = Vec::new();
    let rows_out = std::cell::RefCell::new(Vec::new());
    let con = |p: &mut Problem, terms: &[(Variable, f64)], op: ComparisonOp, rhs: f64| {
        rows_out.borrow_mut().push((terms.iter().map(|(v, a)| (v.idx(), *a)).collect::<Vec<_>>(), op, rhs));
        p.add_constraint(terms, op, rhs)
    };
    let mut var = |p: &mut Problem, c: f64, b: (f64, f64), int: bool| {
        cost.push(c);
        bounds.push((b.0, if int { b.1.min(MAX_BUILDINGS as f64) } else { b.1 }, int));
        if int {
            p.add_integer_var(c, (b.0 as i32, b.1.min(MAX_BUILDINGS as f64) as i32))
        } else {
            p.add_var(c, b)
        }
    };
    let mut rv = Vec::new();
    let mut segments = Vec::new();
    let shared = inp.shared;
    // shared machines: power is linear in work (quality loadouts never overclock) and the beacons'
    // MW go with the group's whole machines, so no power variables
    let lin_of = |f: &Mode| linear_power(w, f) || (shared && w.power > 0.0 && f.power_exp == 1.0 && !f.overclock);
    for u in &inp.units {
        let f = &u.mode;
        let lin = lin_of(f);
        let x = var(&mut p, if lin { w.power * f.power_base / f.speed } else { 0.0 }, (0.0, f64::INFINITY), false);
        // building weight plus the per-building compute terms (sorters, logistics, coaters);
        // added here, not in `scale`, which is a multiplier rebuilt every heuristic round
        let wb = w.buildings
            * compute::machine_cost(w, &u.recipe, f, s.mixed_belts)
            * scale.get(&u.key).copied().unwrap_or(1.0)
            + rules::module_cost(data, s, f)
            + if lin { w.power * f.beacon_power } else { 0.0 };
        let (c_max, k_max) = if f.overclock { (c_max, k_max) } else { (1.0, 0) };
        let cap = n_cap.get(&u.key).copied().unwrap_or(f64::INFINITY);
        // shared: this column's continuous share of its group's machines, at no cost of its own
        let n = if shared { var(&mut p, 0.0, (0.0, cap), false) } else { var(&mut p, wb, (0.0, cap), integer) };
        con(&mut p, &[(x, 1.0), (n, -c_max * f.speed)], ComparisonOp::Le, 0.0);
        let sv = if k_max > 0 {
            let sv = var(&mut p, w.shards, (0.0, f64::INFINITY), integer);
            con(&mut p, &[(x, 1.0), (n, -f.speed), (sv, -0.5 * f.speed)], ComparisonOp::Le, 0.0);
            con(&mut p, &[(sv, 1.0), (n, -(k_max as f64))], ComparisonOp::Le, 0.0);
            Some(sv)
        } else {
            None
        };
        let segs = power_segments(f.power_exp, c_max);
        let pv = if !lin && w.power > 0.0 && (f.power_base > 0.0 || f.beacon_power > 0.0) {
            let pv = var(&mut p, w.power, (0.0, f64::INFINITY), false);
            for (a, b) in &segs {
                // p ≥ P·(a·x + b·n) + beacons·n (beacons draw full power whatever the clock)
                con(
                    &mut p,
                    &[(pv, 1.0), (x, -f.power_base * a / f.speed), (n, -f.power_base * b - f.beacon_power)],
                    ComparisonOp::Ge,
                    0.0,
                );
            }
            Some(pv)
        } else {
            None
        };
        segments.push(segs);
        rv.push(RecipeVars { x, n, s: sv, p: pv });
    }
    // one whole-machine count per shared group: sum_c n_c <= N_g
    let mut groups = Vec::new();
    if shared {
        let mut by: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (k, u) in inp.units.iter().enumerate() {
            by.entry(group_key(u)).or_default().push(k);
        }
        for (g, idx) in by {
            let u = &inp.units[idx[0]];
            let f = &u.mode;
            let wb = w.buildings
                * compute::machine_cost(w, &u.recipe, f, s.mixed_belts)
                * scale.get(&g).copied().unwrap_or(1.0)
                + rules::module_cost(data, s, f)
                + if lin_of(f) { w.power * f.beacon_power } else { 0.0 };
            let nv = var(&mut p, wb, (0.0, f64::INFINITY), integer);
            let mut row: Vec<(Variable, f64)> = idx.iter().map(|&k| (rv[k].n, 1.0)).collect();
            row.push((nv, -1.0));
            con(&mut p, row.as_slice(), ComparisonOp::Le, 0.0);
            groups.push((g, nv));
        }
    }
    // boost items held per building (Somersloops, modules) against their budgets
    for (item, budget) in budgets(s) {
        let row: Vec<(Variable, f64)> = inp
            .units
            .iter()
            .zip(&rv)
            .filter_map(|(u, v)| u.mode.held.iter().find(|(i, _)| *i == item).map(|(_, q)| (v.n, *q)))
            .collect();
        if !row.is_empty() {
            con(&mut p, row.as_slice(), ComparisonOp::Le, budget);
        }
    }
    if let (Some(budget), true) = (s.shard_budget, k_max > 0) {
        let row: Vec<(Variable, f64)> = rv.iter().filter_map(|v| v.s.map(|s| (s, 1.0))).collect();
        if !row.is_empty() {
            con(&mut p, row.as_slice(), ComparisonOp::Le, budget as f64);
        }
    }

    let mut raw = Vec::new();
    for it in &inp.items {
        if let Some(res) = data.resource(it).filter(|r| data.located(&r.locations, &s.planets)) {
            let limit = match s.resource_limits.get(it) {
                Some(v) => *v,
                None => res.limit,
            };
            let limit = if s.ignore_limits { None } else { limit };
            let ext_mw = res.extractor_power / res.extractor_rate;
            let c = w.resources * resource_weight(data, w, it) + w.power * ext_mw;
            raw.push((it.clone(), var(&mut p, c, (0.0, limit.unwrap_or(f64::INFINITY)), false)));
        }
    }
    let sup: Vec<(String, Variable)> =
        inp.supplies.iter().map(|(it, q)| (it.clone(), var(&mut p, 0.0, (0.0, *q), false))).collect();
    let surplus: Vec<(String, Variable)> = inp
        .items
        .iter()
        .map(|it| (it.clone(), var(&mut p, w.surplus, (0.0, f64::INFINITY), false)))
        .collect();

    // item balance rows, built in one pass over the units (net coefficient per unit and item)
    let mut rows: HashMap<&str, Vec<(Variable, f64)>> = HashMap::new();
    for (u, v) in inp.units.iter().zip(&rv) {
        let mut net: BTreeMap<&str, f64> = BTreeMap::new();
        for (i, q) in &u.outs {
            *net.entry(i.as_str()).or_insert(0.0) += q;
        }
        for (i, q) in &u.ins {
            *net.entry(i.as_str()).or_insert(0.0) -= q;
        }
        for (i, a) in net {
            if a != 0.0 {
                rows.entry(i).or_default().push((v.x, a));
            }
        }
    }
    for (i, v) in raw.iter().chain(sup.iter()) {
        rows.entry(i.as_str()).or_default().push((*v, 1.0));
    }
    for (i, v) in &surplus {
        rows.entry(i.as_str()).or_default().push((*v, -1.0));
    }
    let mut balance = Vec::new();
    for it in &inp.items {
        let row = rows.remove(it.as_str()).unwrap_or_default();
        let b = inp.targets.get(it).copied().unwrap_or(0.0);
        balance.push((it.clone(), con(&mut p, row.as_slice(), ComparisonOp::Eq, b)));
    }
    Model { p, cost, bounds, rows: rows_out.into_inner(), rv, raw, sup, surplus, balance, segments, groups }
}

/// Solver-independent snapshot of a solution, keyed by recipe / item id.
#[derive(Default, Clone)]
struct Point {
    rec: HashMap<String, (f64, f64, f64)>, // x, n, shards
    flows: HashMap<(u8, String), f64>,      // (0 raw | 1 supply | 2 surplus, item)
    groups: HashMap<String, f64>,           // shared-machine groups: whole machines
}

fn point_of(m: &Model, inp: &Inputs, sol: &Solution) -> Point {
    point_from(m, inp, |v| sol[v])
}

fn point_from(m: &Model, inp: &Inputs, sol: impl Fn(Variable) -> f64) -> Point {
    let mut pt = Point::default();
    for (u, v) in inp.units.iter().zip(&m.rv) {
        let sh = v.s.map(&sol).unwrap_or(0.0);
        let x = sol(v.x).max(0.0);
        // shared machines: a tier's share is its work at full speed. The share variable costs
        // nothing, so a solver may return any value up to the group's spare capacity (review 7 H2)
        let n = if inp.shared { x / u.mode.speed } else { sol(v.n).max(0.0) };
        pt.rec.insert(u.key.clone(), (x, n, sh.max(0.0)));
    }
    for (k, list) in [(0u8, &m.raw), (1, &m.sup), (2, &m.surplus)] {
        for (it, v) in list {
            pt.flows.insert((k, it.clone()), sol(*v).max(0.0));
        }
    }
    for (g, v) in &m.groups {
        pt.groups.insert(g.clone(), sol(*v).max(0.0));
    }
    pt
}

/// Whole-building plan from a relaxed point: buildings rounded up, work and flows unchanged, so
/// every row holds but the boost-item and shard budgets. `shards = false` rounds to shardless
/// machines (n = ⌈x / speed⌉), which always fits a shard budget.
fn rounded(inp: &Inputs, pt: &Point, c_max: f64, shards: bool) -> Point {
    let mut out = Point { rec: HashMap::new(), flows: pt.flows.clone(), groups: HashMap::new() };
    for u in &inp.units {
        let f = &u.mode;
        let Some(&(x, _, _)) = pt.rec.get(&u.key) else { continue };
        if inp.shared {
            // each tier's share stays fractional; its group's machines are rounded up below
            let n = if x <= 1e-9 { 0.0 } else { x / f.speed };
            out.rec.insert(u.key.clone(), (x, n, 0.0));
            *out.groups.entry(group_key(u)).or_insert(0.0) += n;
            continue;
        }
        let cm = if f.overclock && shards { c_max } else { 1.0 };
        let n = if x <= 1e-9 { 0.0 } else { (x / (cm * f.speed) - 1e-9).ceil() };
        let c = if n > 0.0 { x / (n * f.speed) } else { 0.0 };
        let sh = if c > 1.0 { ((c - 1.0) / 0.5 - 1e-9).ceil() * n } else { 0.0 };
        out.rec.insert(u.key.clone(), (x, n, sh));
    }
    for n in out.groups.values_mut() {
        *n = (*n - 1e-9).ceil().max(0.0);
    }
    out
}

/// The boost-item (Somersloop) and shard budgets hold at `pt` (whole buildings and shards).
fn within_budgets(s: &PlanSettings, inp: &Inputs, pt: &Point) -> bool {
    for (item, budget) in budgets(s) {
        let used: f64 = inp
            .units
            .iter()
            .filter_map(|u| {
                let q = u.mode.held.iter().find(|(i, _)| *i == item)?.1;
                Some(q * pt.rec.get(&u.key)?.1.round())
            })
            .sum();
        if used > budget + 1e-6 {
            return false;
        }
    }
    if let Some(b) = s.shard_budget {
        let used: f64 = pt.rec.values().map(|v| v.2.round()).sum();
        if used > b as f64 + 1e-6 {
            return false;
        }
    }
    true
}

/// Values of every model variable at a whole-building point (buildings and shards rounded,
/// power from the segments).
fn values_of(m: &Model, inp: &Inputs, pt: &Point) -> Vec<(Variable, f64)> {
    let mut hint = Vec::new();
    for ((u, v), segs) in inp.units.iter().zip(&m.rv).zip(&m.segments) {
        let f = &u.mode;
        let (x, n, sh) = pt.rec.get(&u.key).copied().unwrap_or((0.0, 0.0, 0.0));
        // shared-machine shares are continuous; whole buildings and shards are not
        let n = if m.bounds[v.n.idx()].2 { n.round() } else { n };
        let sh = sh.round();
        hint.push((v.x, x));
        hint.push((v.n, n));
        if let Some(s) = v.s {
            hint.push((s, sh));
        }
        if let Some(p) = v.p {
            let pw = segs.iter().map(|(a, b)| f.power_base * (a * x / f.speed + b * n)).fold(0.0, f64::max)
                + f.beacon_power * n;
            hint.push((p, pw));
        }
    }
    for (k, list) in [(0u8, &m.raw), (1, &m.sup), (2, &m.surplus)] {
        for (it, v) in list {
            hint.push((*v, pt.flows.get(&(k, it.clone())).copied().unwrap_or(0.0)));
        }
    }
    for (g, v) in &m.groups {
        let n = pt.groups.get(g).copied().unwrap_or(0.0);
        hint.push((*v, if m.bounds[v.idx()].2 { n.round() } else { n }));
    }
    hint
}

/// Warm start with the integer variables only (buildings, shards): microlp fixes them and
/// completes the rest by LP. That completion cannot trip over the tolerance of a fixed
/// continuous value and is at least as cheap as the point itself.
fn int_hint(m: &Model, inp: &Inputs, pt: &Point) -> Vec<(Variable, f64)> {
    let mut hint = Vec::new();
    for (u, v) in inp.units.iter().zip(&m.rv) {
        let (_, n, sh) = pt.rec.get(&u.key).copied().unwrap_or((0.0, 0.0, 0.0));
        if m.bounds[v.n.idx()].2 {
            hint.push((v.n, n.round()));
        }
        if let Some(s) = v.s {
            hint.push((s, sh.round()));
        }
    }
    for (g, v) in &m.groups {
        hint.push((*v, pt.groups.get(g).copied().unwrap_or(0.0).round()));
    }
    hint
}

fn objective_of(m: &Model, values: &[(Variable, f64)]) -> f64 {
    values.iter().map(|(v, x)| m.cost[v.idx()] * x).sum()
}

/// Inputs restricted to the recipes in `keep`.
fn restrict(inp: &Inputs, keep: &BTreeSet<String>) -> Inputs {
    let units: Vec<Unit> = inp.units.iter().filter(|u| keep.contains(&u.key)).cloned().collect();
    Inputs {
        items: items_of(&units, &inp.targets, &inp.supplies),
        units,
        targets: inp.targets.clone(),
        supplies: inp.supplies.clone(),
        weights: inp.weights.clone(),
        shared: inp.shared,
    }
}

/// First MILP call: root LP and warm-start check with no time limit and no branching. A deadline
/// inside the hint's LP completion would silently drop the hint (report §3.4).
fn milp_root(hint: Vec<(Variable, f64)>, gap: f64) -> SolveOptions {
    let mut o = SolveOptions::default();
    o.time_limit = None;
    o.node_limit = Some(0);
    o.mip_gap = gap;
    o.warm_start = Some(hint);
    o
}

fn resume_options(ms: f64, gap: f64) -> microlp::ResumeOptions {
    let mut o = microlp::ResumeOptions::default();
    o.time_limit = Some(Duration::from_secs_f64(ms.clamp(5.0, 1e9) / 1e3));
    o.node_limit = None;
    o.mip_gap = Some(gap);
    o
}

/// Drops a mode column dominated by another mode of the same recipe: same inputs and products
/// per unit of work, at least as fast, no dearer per building, no more power when power has a
/// weight, no more boost items held, same overclocking. A plan using the dominated column can move
/// its work and buildings to the dominating one at no higher cost, so the optimum is unchanged
/// (report §4.2). Returns the kept units and, per kept unit, the dropped units it dominates
/// (dominance is transitive), for the display tie-break in `detier`.
fn drop_dominated(
    data: &GameData,
    s: &PlanSettings,
    units: Vec<Unit>,
    w: &CostWeights,
) -> (Vec<Unit>, HashMap<String, Vec<Unit>>) {
    let same = |a: &Vec<(String, f64)>, b: &Vec<(String, f64)>| {
        let (ma, mb): (BTreeMap<&str, f64>, BTreeMap<&str, f64>) =
            (a.iter().map(|(i, q)| (i.as_str(), *q)).collect(), b.iter().map(|(i, q)| (i.as_str(), *q)).collect());
        ma.len() == mb.len() && ma.iter().all(|(k, v)| mb.get(k).is_some_and(|x| (x - v).abs() <= 1e-12 * (1.0 + v.abs())))
    };
    let held_le = |a: &Mode, b: &Mode| a.held.iter().all(|(i, q)| b.held.iter().any(|(j, r)| i == j && q <= r));
    let wb: Vec<f64> = units.iter().map(|u| building_weight(data, s, w, u)).collect();
    let dom = |j: usize, k: usize| {
        let (uj, uk) = (&units[j], &units[k]);
        uj.recipe.id == uk.recipe.id
            && uj.mode.overclock == uk.mode.overclock
            && uj.mode.power_exp == uk.mode.power_exp
            && uj.mode.speed >= uk.mode.speed
            && wb[j] <= wb[k]
            && (w.power <= 0.0 || (uj.mode.power_base <= uk.mode.power_base && uj.mode.beacon_power <= uk.mode.beacon_power))
            && held_le(&uj.mode, &uk.mode)
            && same(&uj.ins, &uk.ins)
            && same(&uj.outs, &uk.outs)
    };
    let mut by: Vec<Option<usize>> = vec![None; units.len()];
    let mut by_recipe: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, u) in units.iter().enumerate() {
        by_recipe.entry(u.recipe.id.as_str()).or_default().push(i);
    }
    for idx in by_recipe.values() {
        for &k in idx {
            for &j in idx {
                // identical columns dominate each other: keep the first
                if j != k && by[j].is_none() && dom(j, k) && !(dom(k, j) && k < j) {
                    by[k] = Some(j);
                    break;
                }
            }
        }
    }
    // follow chains (j itself dropped later) to the kept unit
    let root = |mut k: usize| {
        while let Some(j) = by[k] {
            k = j;
        }
        k
    };
    let roots: Vec<usize> = (0..units.len()).map(root).collect();
    let keys: Vec<String> = units.iter().map(|u| u.key.clone()).collect();
    let mut kept = Vec::new();
    let mut dropped: HashMap<String, Vec<Unit>> = HashMap::new();
    for (k, u) in units.into_iter().enumerate() {
        if roots[k] == k {
            kept.push(u);
        } else {
            dropped.entry(keys[roots[k]].clone()).or_default().push(u);
        }
    }
    (kept, dropped)
}

/// Display tie-break after `drop_dominated`: where a dropped (slower) tier runs the same work on
/// the same whole buildings at the same objective weight and holds the same boosts, show the one
/// drawing the least power. The objective is unchanged; without this the plan would always show
/// the fastest tier (more MW) where tiers tie (Resources / Buildings objectives).
fn detier(
    data: &GameData,
    s: &PlanSettings,
    inp: &Inputs,
    dropped: &HashMap<String, Vec<Unit>>,
    pt: &Point,
) -> Option<(Inputs, Point)> {
    // (shared machines: a tier column cannot move to another machine type alone)
    if dropped.is_empty() || inp.shared {
        return None;
    }
    let w = &inp.weights;
    let (mut units, mut out, mut changed) = (inp.units.clone(), pt.clone(), false);
    for u in &inp.units {
        let Some(alts) = dropped.get(&u.key) else { continue };
        let Some(&(x, n, sh)) = pt.rec.get(&u.key) else { continue };
        if x <= 1e-9 || n < 0.5 || u.mode.overclock {
            continue;
        }
        let power = |m: &Mode| n * m.power_base * (x / (n * m.speed)).powf(m.power_exp) + m.beacon_power * n;
        let wu = building_weight(data, s, w, u);
        let best = alts
            .iter()
            .filter(|k| {
                k.mode.speed * n >= x * (1.0 - 1e-9)
                    && k.mode.held == u.mode.held
                    && (building_weight(data, s, w, k) - wu).abs() <= 1e-9 * (1.0 + wu.abs())
            })
            .min_by(|a, b| power(&a.mode).total_cmp(&power(&b.mode)));
        if let Some(k) = best.filter(|k| power(&k.mode) < power(&u.mode) - 1e-9) {
            out.rec.remove(&u.key);
            out.rec.insert(k.key.clone(), (x, n, sh));
            units.push(k.clone());
            changed = true;
        }
    }
    changed.then(|| (Inputs { units, ..inp.clone() }, out))
}

/// Split `x` units of work over `n` machines holding `s` shards, minimising power
/// (equal clocks, capped by each machine's shards).
pub fn machine_groups(n: f64, x: f64, s: f64, c_max: f64, integer: bool) -> Vec<MachineGroup> {
    if n <= 1e-9 || x <= 1e-9 {
        return vec![];
    }
    let c = x / n;
    let need = |c: f64| if c > 1.0 + 1e-9 { ((c - 1.0) / 0.5 - 1e-9).ceil() as u32 } else { 0 };
    if !integer || c <= 1.0 + 1e-9 {
        return vec![MachineGroup { count: n, clock_percent: c * 100.0, shards: need(c) }];
    }
    let (n_i, s_i) = (n.round() as u64, s.round() as u64);
    let (q, r) = (s_i / n_i, s_i % n_i);
    let lo = (1.0 + 0.5 * q as f64).min(c_max);
    if c <= lo + 1e-9 {
        return vec![MachineGroup { count: n, clock_percent: c * 100.0, shards: need(c) }];
    }
    // n−r machines at their cap `lo`, the r machines with one more shard take the rest
    let rest = (x - (n_i - r) as f64 * lo) / r.max(1) as f64;
    let mut g = vec![MachineGroup { count: (n_i - r) as f64, clock_percent: lo * 100.0, shards: q as u32 }];
    if r > 0 {
        g.push(MachineGroup { count: r as f64, clock_percent: rest * 100.0, shards: need(rest) });
    }
    g.retain(|g| g.count > 0.0);
    g
}

/// Fills the plan's recipe rows, flows and totals from a point (LP solution or whole-building plan).
fn finish(data: &GameData, s: &PlanSettings, inp: &Inputs, pt: &Point, integer: bool, plan: &mut Plan) {
    let name = |id: &str| data.item_name(id);
    let clean = |v: f64| if v.abs() < 1e-7 { 0.0 } else { v };
    let c_max = s.max_clock();
    let mut summary: Option<ComputeSummary> = None;
    // DSP: compute cost of every plan (whatever the objective), so presets compare
    let cw = (data.game == "dsp").then(|| compute::model().weights(&s.generator));
    // shared machines: beacons stand at (and draw power for) every whole machine of a group, idle
    // or not, as the objective charges them; each tier row carries its share of them (review 7 M1)
    let mut placed: HashMap<String, f64> = HashMap::new();
    if inp.shared {
        let whole = group_machines(inp, pt);
        let mut work: HashMap<String, f64> = HashMap::new();
        for u in &inp.units {
            if let Some(v) = pt.rec.get(&u.key).filter(|v| v.0.abs() >= 1e-7) {
                *work.entry(group_key(u)).or_insert(0.0) += v.1;
            }
        }
        for (g, n) in work {
            if n > 0.0 {
                placed.insert(g.clone(), whole.get(&g).copied().unwrap_or(n).max(n) / n);
            }
        }
    }
    for u in &inp.units {
        let (r, f) = (&*u.recipe, &u.mode);
        let c_max = if f.overclock { c_max } else { 1.0 };
        let Some(&(x, n, sh)) = pt.rec.get(&u.key) else { continue };
        let x = clean(x);
        if x <= 0.0 {
            continue;
        }
        let mut n = clean(n);
        let mut sh = clean(sh);
        // shared machines: n is this tier's share of its group's whole machines
        let whole = integer && !inp.shared;
        if whole {
            n = n.round();
            sh = sh.round();
        }
        let groups = machine_groups(n, x / f.speed, sh, c_max, whole);
        // machines this row's beacons serve: its share of the group's whole machines when shared
        let nb = if inp.shared { n * placed.get(&group_key(u)).copied().unwrap_or(1.0) } else { n };
        let power: f64 = groups
            .iter()
            .map(|g| g.count * f.power_base * (g.clock_percent / 100.0).powf(f.power_exp))
            .sum::<f64>()
            + f.beacon_power * nb;
        let shards = if whole { sh } else { groups.iter().map(|g| g.count * g.shards as f64).sum() };
        let b = data.building(&f.building);
        plan.recipes.push(PlanRecipe {
            id: r.id.clone(),
            mode: f.key.clone(),
            mode_label: f.label.clone(),
            name: data.pick(&r.name, &r.name_zh).to_string(),
            alt: r.alt,
            building: b.map(|b| data.pick(&b.name, &b.name_zh).to_string()).unwrap_or_else(|| f.building.clone()),
            machines: n,
            machines_ceil: (n - 1e-6).ceil(),
            throughput: x,
            shards,
            groups,
            power,
            inputs: u.ins.iter().map(|(i, q)| RateLine::new(i, &name(i), q * x)).collect(),
            outputs: u
                .outs
                .iter()
                .filter(|(_, q)| q * x > 1e-9)
                .map(|(i, q)| RateLine::new(i, &name(i), q * x))
                .collect(),
            somersloops: f.somersloops,
            compute: 0.0,
            beacons: nb * f.beacons,
            group_machines: 0.0,
        });
        if let Some(w) = &cw {
            let k = compute::sorters_per_machine(r, f, s.mixed_belts, compute::model().sorter_capacity);
            let bw = w.building_weights.get(&f.building).copied().unwrap_or(1.0);
            // every placed entity costs, working or idle (spec §4.2): whole buildings also
            // when the plan is fractional; power stays as solved
            let ne = if whole { n } else { (n - 1e-6).ceil() };
            let parts = [
                ne * bw,
                ne * w.sorter * k,
                ne * w.per_machine,
                if f.sprayed { ne * w.coater } else { 0.0 },
                w.power * power,
            ];
            let c = summary.get_or_insert_with(ComputeSummary::default);
            for (acc, v) in [&mut c.machines, &mut c.sorters, &mut c.logistics, &mut c.coaters, &mut c.generation]
                .into_iter()
                .zip(parts)
            {
                *acc += v;
            }
            plan.recipes.last_mut().unwrap().compute = parts.iter().sum();
        }
    }
    plan.recipes.sort_by(|a, b| a.name.cmp(&b.name));
    let flow = |k: u8, it: &String| clean(pt.flows.get(&(k, it.clone())).copied().unwrap_or(0.0));
    for it in &inp.items {
        let q = flow(0, it);
        if q > 0.0 {
            if let Some(res) = data.resource(it) {
                plan.extraction_power += q / res.extractor_rate * res.extractor_power;
            }
            plan.raw.push(RateLine::new(it, &name(it), q));
        }
    }
    for it in inp.supplies.keys() {
        let q = flow(1, it);
        if q > 0.0 {
            plan.supplied.push(RateLine::new(it, &name(it), q));
        }
    }
    for it in &inp.items {
        let q = flow(2, it);
        if q > 1e-6 {
            plan.surplus.push(RateLine::new(it, &name(it), q));
        }
    }
    for (it, q) in &inp.targets {
        plan.targets.push(RateLine::new(it, &name(it), *q));
    }
    plan.production_power = plan.recipes.iter().map(|r| r.power).sum();
    // Buildings to place per type. Quality plans: the tiers of a base recipe run on shared
    // machines that switch recipes, so whole machines per (base recipe, machine, loadout), not
    // per tier column (with whole buildings this is the solved group count, report §5 step 6).
    let mut groups: BTreeMap<(String, String), (String, f64)> = BTreeMap::new();
    for r in &plan.recipes {
        let base = if plan.quality { quality::split(&r.id).0 } else { r.id.as_str() };
        let g = groups.entry((base.to_string(), r.mode.clone())).or_insert((r.building.clone(), 0.0));
        g.1 += r.machines;
    }
    let whole_of = |n: f64| if integer && !inp.shared { n } else { (n - WHOLE_TOL).ceil() };
    if plan.quality {
        for r in plan.recipes.iter_mut() {
            let n = groups.get(&(quality::split(&r.id).0.to_string(), r.mode.clone())).map(|g| g.1).unwrap_or(0.0);
            r.group_machines = whole_of(n);
        }
    }
    let mut counts: BTreeMap<String, f64> = BTreeMap::new();
    for (_, (b, n)) in groups {
        *counts.entry(b).or_insert(0.0) += whole_of(n);
    }
    // Factorio beacons (whole beacons in total; per machine they may be shared)
    let beacons: f64 = plan.recipes.iter().map(|r| r.beacons).sum();
    if beacons > 1e-9 {
        let name = data.beacons.first().map(|b| b.name.clone()).unwrap_or_else(|| "Beacon".into());
        counts.insert(name, (beacons - 1e-6).ceil());
    }
    plan.building_counts = counts.into_iter().map(|(building, count)| BuildingCount { building, count }).collect();
    plan.buildings = plan.building_counts.iter().map(|b| b.count).sum();
    plan.shards = plan.recipes.iter().map(|r| r.shards).sum();
    plan.somersloops = plan.recipes.iter().map(|r| r.machines * r.somersloops as f64).sum();
    plan.flows = super::flows::allocate(plan);
    if let Some(h) = rules::proliferator_tier_hint(data, s, &plan.recipes) {
        plan.warnings.push(h);
    }
    if let Some(w) = &cw {
        let mut c = summary.unwrap_or_default();
        let model = compute::model();
        c.generation += w.power * plan.extraction_power;
        c.total = c.machines + c.sorters + c.logistics + c.coaters + c.generation;
        c.generator = s.generator.clone();
        c.cost_per_mw = w.power;
        let (us, lo, hi) = model.us_per_unit;
        (c.us_per_tick, c.us_low, c.us_high) = (c.total * us, c.total * lo, c.total * hi);
        plan.compute = Some(c);
    }
}

#[cfg(test)]
thread_local! {
    /// Tests: build the model without the exact reductions (dominated columns, linear power:
    /// see `shrink` in `linear_power` and `start_units`), to check that they leave the optimum unchanged.
    pub(crate) static NO_SHRINK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
fn shrink() -> bool {
    #[cfg(test)]
    {
        !NO_SHRINK.with(|c| c.get())
    }
    #[cfg(not(test))]
    {
        true
    }
}

/// Outcome of `start`: a finished plan (LP mode, infeasible, too large, …) or a whole-building
/// solve to drive with `Run::step`.
pub enum Start<'a> {
    Done(Plan),
    Running(Box<Run<'a>>),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    /// Re-weighted LPs (k done): neighbourhood for the restricted MILP, plus their rounded plans.
    Reweight(u8),
    /// MILP over the recipes the LPs use.
    Sub,
    /// The full MILP, warm-started from the best plan in hand.
    Full,
    Done,
}

struct SubRun {
    inp: Inputs,
    m: Model,
    out: SolveOutcome,
    /// ms since start
    until: f64,
}

/// Progress of a running whole-building solve.
#[derive(Clone, Debug)]
pub struct Progress {
    pub elapsed_ms: f64,
    /// Objective of the best plan in hand.
    pub objective: Option<f64>,
    /// Lower bound on the objective (LP relaxation, then the MILP's best bound).
    pub bound: f64,
    /// Relative gap of the best plan to the bound.
    pub gap: Option<f64>,
    pub proven: bool,
    pub done: bool,
    pub phase: &'static str,
    /// Bumped whenever the best plan changes.
    pub version: u32,
    pub nodes: u64,
    pub deadline_ms: f64,
}

/// A whole-building solve run in steps, holding the best plan found so far (report §6 steps 0, 3):
/// the rounded-up LP plan is there as soon as the LP is solved, the heuristics and the MILP only
/// ever improve it, and a MILP that ends without an incumbent no longer loses it.
pub struct Run<'a> {
    data: &'a GameData,
    s: PlanSettings,
    started: web_time::Instant,
    budget: f64,
    /// ms since start
    deadline: f64,
    /// Plan without recipe rows: LP bound, item values, model size, warnings so far.
    base: Plan,
    inp: Inputs,
    dropped: HashMap<String, Vec<Unit>>,
    m: Model,
    c_max: f64,
    lp_pt: Point,
    lp_ms: f64,
    support: BTreeSet<String>,
    prev: HashMap<String, (f64, f64, f64)>,
    sub: Option<SubRun>,
    full: Option<SolveOutcome>,
    best: Option<(f64, Point)>,
    version: u32,
    bound: f64,
    proven: bool,
    phase: Phase,
    failure: Option<String>,
    nodes: u64,
    /// `version` right after the budget fallback became the best plan (it still is while equal)
    fallback_version: Option<u32>,
    /// Objective weight of the cheapest whole building in the model (infinite when no building
    /// costs anything): the unit before there is a plan.
    model_unit: f64,
    /// Objective weight of one of the cheapest buildings the best plan uses (`model_unit` without a
    /// plan): a plan less than this above the bound has no rival that saves one of its buildings.
    unit: f64,
    /// Buildings are a main cost: weight at least the resource weight and a tenth of the MW weight.
    buildings_primary: bool,
    /// (gap, version) of the last MipGap stop that did not prove (see `step_full`).
    last_mipgap: Option<(f64, u32)>,
}

/// The whole-building columns of a model: each unit's building count, or in shared mode each
/// group's machines (the units' counts are then continuous shares with no cost). Shards and
/// Somersloops carry tie-break weights and do not count.
fn building_cols(m: &Model) -> Vec<usize> {
    m.rv.iter()
        .map(|v| v.n.idx())
        .chain(m.groups.iter().map(|(_, v)| v.idx()))
        .filter(|&j| m.bounds[j].2 && m.cost[j] > 1e-12)
        .collect()
}

/// Smallest objective weight of one whole building among `cols` where `used` (infinite if none).
fn cheapest(m: &Model, cols: &[usize], used: impl Fn(usize) -> bool) -> f64 {
    cols.iter().filter(|&&j| used(j)).map(|&j| m.cost[j]).fold(f64::INFINITY, f64::min)
}

impl<'a> Run<'a> {
    fn ms(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1e3
    }

    fn gap(&self) -> Option<f64> {
        self.best.as_ref().map(|(o, _)| (o - self.bound).max(0.0) / o.abs().max(1e-10))
    }

    /// Absolute tolerance when buildings are a main term of the objective (Buildings, Power,
    /// Compute: `buildings_primary`): just under one of the cheapest
    /// buildings the plan uses, so a plan called optimal has no rival that is the same but for one
    /// building fewer (MIP_GAP alone let Ballistic Warp Drive under Power be "optimal" 6 buildings
    /// above the plan found with this rule). Infinite under Resources, where buildings are only a
    /// 0.01 tie-break: there the point is the resources, and MIP_GAP alone decides.
    fn unit_tol(&self) -> f64 {
        if self.buildings_primary { 0.99 * self.unit } else { f64::INFINITY }
    }

    /// Objective weight of the building the tolerance is measured in (tests).
    #[cfg(test)]
    pub fn unit(&self) -> f64 {
        self.unit
    }

    /// The plan in hand is optimal to tolerance (`tol`).
    fn closed(&self, slack: f64) -> bool {
        self.best.as_ref().is_some_and(|(o, _)| o - self.bound <= self.tol(*o, slack))
    }

    /// Absolute tolerance at objective `o`: MIP_GAP relative (x `slack` for engines that measure it
    /// differently), and where buildings are a main cost also just under one building; never below
    /// 1e-6 relative, the size of the objective error the engines' feasibility tolerances allow.
    /// (The building rule alone, without MIP_GAP, made small plans worse: Heavy Modular Frame
    /// under Power 450 buildings instead of 446.)
    fn tol(&self, o: f64, slack: f64) -> f64 {
        (slack * MIP_GAP * o.abs()).min(self.unit_tol()).max(1e-6 * o.abs()).max(1e-9)
    }

    /// The relative gap an engine stops at: `tol` for a plan of the size in hand (or of the LP
    /// bound before there is one).
    pub fn engine_gap(&self) -> f64 {
        let o = self.best.as_ref().map(|b| b.0).unwrap_or(self.bound).abs().max(1e-10);
        (self.tol(o, 1.0) / o).clamp(1e-6, 0.5)
    }

    /// The best plan's gap to the bound in objective weights of the cheapest building it uses
    /// (None without a plan, or when the building rule is off: `unit_tol`).
    fn gap_units(&self) -> Option<f64> {
        let (o, _) = self.best.as_ref()?;
        self.unit_tol().is_finite().then(|| (o - self.bound).max(0.0) / self.unit)
    }

    /// Takes `pt` (whole buildings) as the best plan when it fits the budgets and is cheaper.
    fn offer(&mut self, mut pt: Point) {
        if self.inp.shared {
            // canonical form: every group holds exactly the whole machines its tiers' work needs
            // (a solver may leave a group above that when its machines cost nothing or within its
            // tolerance), so the objective, the building counts and the rows shown agree
            pt.groups = group_machines(&self.inp, &pt);
        }
        if !within_budgets(&self.s, &self.inp, &pt) {
            return;
        }
        let obj = objective_of(&self.m, &values_of(&self.m, &self.inp, &pt));
        if !obj.is_finite() {
            return;
        }
        if self.best.as_ref().is_none_or(|(b, _)| obj < b - 1e-9 * b.abs().max(1.0)) {
            // the cheapest building this plan uses
            let vals: HashMap<usize, f64> =
                values_of(&self.m, &self.inp, &pt).into_iter().map(|(v, x)| (v.idx(), x)).collect();
            let u = cheapest(&self.m, &building_cols(&self.m), |j| vals.get(&j).is_some_and(|x| *x > 0.5));
            self.unit = if u.is_finite() { u } else { self.model_unit };
            self.best = Some((obj, pt));
            self.version += 1;
        }
    }

    /// A plan within the budgets when rounding up the LP breaks them (report §5.1): shardless
    /// machines for a shard budget; for a Somersloop budget, the buildings holding them capped at
    /// ⌊LP buildings⌋ (then at 0) and the LP re-solved, the rest rounded up.
    fn budget_fallback(&mut self) -> Option<Point> {
        let pt = rounded(&self.inp, &self.lp_pt, self.c_max, false);
        if within_budgets(&self.s, &self.inp, &pt) {
            return Some(pt);
        }
        // caps per unit holding a budgeted item: ⌊LP buildings⌋, then the budget left over
        // handed out (one more building each) to the largest fractional parts; also all-floor and zero
        let mut floor: HashMap<String, f64> = HashMap::new();
        let mut greedy: HashMap<String, f64> = HashMap::new();
        for (item, budget) in budgets(&self.s) {
            let mut left = budget;
            let mut frac: Vec<(f64, f64, String)> = Vec::new();
            for u in self.inp.units.iter() {
                let Some(q) = u.mode.held.iter().find(|(i, _)| *i == item).map(|h| h.1) else { continue };
                let n = self.lp_pt.rec.get(&u.key).map(|v| v.1).unwrap_or(0.0);
                let f = (n + 1e-9).floor();
                floor.insert(u.key.clone(), f);
                greedy.insert(u.key.clone(), f);
                left -= q * f;
                if n - f > 1e-7 {
                    frac.push((n - f, q, u.key.clone()));
                }
            }
            frac.sort_by(|a, b| b.0.total_cmp(&a.0));
            for (_, q, k) in frac {
                if q <= left + 1e-9 {
                    left -= q;
                    *greedy.get_mut(&k).unwrap() += 1.0;
                }
            }
        }
        let zero: HashMap<String, f64> = floor.keys().map(|k| (k.clone(), 0.0)).collect();
        let mut best: Option<(f64, Point)> = None;
        for caps in [greedy, floor, zero] {
            let lpm = build_scaled(self.data, &self.s, &self.inp, false, &HashMap::new(), &caps);
            if let Ok(Ok(sol)) = lpm.p.solve().map(|o| o.into_solution()) {
                let p = point_of(&lpm, &self.inp, &sol);
                for shards in [true, false] {
                    let r = rounded(&self.inp, &p, self.c_max, shards);
                    if within_budgets(&self.s, &self.inp, &r) {
                        let obj = objective_of(&self.m, &values_of(&self.m, &self.inp, &r));
                        if best.as_ref().is_none_or(|b| obj < b.0) {
                            best = Some((obj, r));
                        }
                        break;
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// Runs the next piece of work, about `slice_ms` long (`None`: each phase to its end).
    /// Returns true when the solve is finished (proven, out of time, or failed).
    pub fn step(&mut self, slice_ms: Option<f64>) -> bool {
        if self.phase != Phase::Done && self.closed(1.0) {
            self.proven = true;
            self.phase = Phase::Done;
        }
        match self.phase {
            Phase::Reweight(k) => self.step_reweight(k),
            Phase::Sub => self.step_sub(slice_ms),
            Phase::Full => self.step_full(slice_ms),
            Phase::Done => {}
        }
        if self.phase != Phase::Done && self.closed(1.0) {
            self.proven = true;
            self.phase = Phase::Done;
        }
        self.phase == Phase::Done
    }

    /// Whole buildings make small recipes dear: re-solve the LP charging each recipe ceil(n)/n of
    /// its building cost at its last rate (sparser plans), and keep every recipe these LPs use as
    /// the neighbourhood of the restricted MILP. Skipped when the LP alone took a tenth of the budget.
    fn step_reweight(&mut self, k: u8) {
        if k >= 4 || self.inp.weights.buildings <= 0.0 || self.lp_ms > self.budget / 10.0 || self.ms() >= self.deadline {
            self.phase = Phase::Sub;
            return;
        }
        let per_key: HashMap<String, f64> = if self.inp.shared {
            // shared machines: re-weight each group by its own rounding
            let mut sums: HashMap<String, f64> = HashMap::new();
            for u in &self.inp.units {
                if let Some(v) = self.prev.get(&u.key) {
                    *sums.entry(group_key(u)).or_insert(0.0) += v.1;
                }
            }
            sums
        } else {
            self.prev.iter().map(|(k, v)| (k.clone(), v.1)).collect()
        };
        let scale: HashMap<String, f64> = per_key
            .into_iter()
            .filter(|(_, n)| *n > 1e-7)
            .map(|(k, n)| (k, ((n - 1e-9).ceil() / n).min(1e3)))
            .collect();
        let rm = build_scaled(self.data, &self.s, &self.inp, false, &scale, &HashMap::new());
        match rm.p.solve().map(|o| o.into_solution()) {
            Ok(Ok(sol)) => {
                let pt = point_of(&rm, &self.inp, &sol);
                self.support.extend(pt.rec.iter().filter(|(_, v)| v.0 > 1e-7).map(|(k, _)| k.clone()));
                let r = rounded(&self.inp, &pt, self.c_max, true);
                self.prev = pt.rec;
                self.offer(r);
                self.phase = Phase::Reweight(k + 1);
            }
            _ => self.phase = Phase::Sub,
        }
    }

    /// MILP over the recipes the LPs (and the best plan) use: few integer variables, so it
    /// often finds a better plan quickly. A quarter of the budget.
    fn step_sub(&mut self, slice_ms: Option<f64>) {
        let now = self.ms();
        let run = match self.sub.take() {
            Some(mut sub) => {
                let left = sub.until.min(self.deadline) - now;
                if left <= 0.0 {
                    self.phase = Phase::Full;
                    return;
                }
                match sub.out.resume_with(resume_options(slice_ms.unwrap_or(f64::INFINITY).min(left), MIP_GAP)) {
                    Ok(out) => {
                        sub.out = out;
                        Some(sub)
                    }
                    Err(_) => None,
                }
            }
            None => {
                let mut keep = self.support.clone();
                // (not the budget fallback's: a feasible restricted MILP around it only spends time,
                // the plan is already in hand)
                if let Some((_, b)) = self.best.as_ref().filter(|_| self.fallback_version != Some(self.version)) {
                    keep.extend(b.rec.iter().filter(|(_, v)| v.0 > 1e-7).map(|(k, _)| k.clone()));
                }
                if keep.len() >= self.inp.units.len() || now >= self.deadline {
                    self.phase = Phase::Full;
                    return;
                }
                let sub_inp = restrict(&self.inp, &keep);
                let m = build(self.data, &self.s, &sub_inp, true);
                let from = self.best.as_ref().map(|b| b.1.clone()).unwrap_or_else(|| rounded(&self.inp, &self.lp_pt, self.c_max, true));
                let hint = int_hint(&m, &sub_inp, &from);
                let until = now + self.budget / 4.0;
                match m.p.solve_with(milp_root(hint, MIP_GAP)) {
                    Ok(out) => Some(SubRun { inp: sub_inp, m, out, until }),
                    Err(_) => None,
                }
            }
        };
        let Some(sub) = run else {
            self.phase = Phase::Full;
            return;
        };
        if let Some(sol) = sub.out.solution() {
            let pt = point_of(&sub.m, &sub.inp, sol);
            self.offer(pt);
        }
        let open = matches!(sub.out.termination_reason(), TerminationReason::TimeLimit | TerminationReason::NodeLimit);
        if open && self.ms() < sub.until.min(self.deadline) {
            self.sub = Some(sub);
        } else {
            self.phase = Phase::Full;
        }
    }

    /// The full MILP, warm-started from the best plan, resumed in slices until the deadline.
    fn step_full(&mut self, slice_ms: Option<f64>) {
        let now = self.ms();
        let gap = self.engine_gap();
        let res = match self.full.take() {
            Some(out) => {
                let left = self.deadline - now;
                if left <= 0.0 {
                    self.full = Some(out);
                    self.phase = Phase::Done;
                    return;
                }
                out.resume_with(resume_options(slice_ms.unwrap_or(f64::INFINITY).min(left), gap))
            }
            None => {
                if now >= self.deadline {
                    self.phase = Phase::Done;
                    return;
                }
                let from = self.best.as_ref().map(|b| b.1.clone()).unwrap_or_else(|| rounded(&self.inp, &self.lp_pt, self.c_max, true));
                // A budget-fallback plan is kept in hand but not given as warm start: it is far from
                // optimal, and searching from it found worse plans in 4 s than searching without a
                // start (Somersloop budget, Resources: 5725 vs 5275, as the code before the fallback).
                let hint = if self.fallback_version == Some(self.version) { vec![] } else { int_hint(&self.m, &self.inp, &from) };
                self.m.p.solve_with(milp_root(hint, gap))
            }
        };
        match res {
            Ok(out) => {
                let st = out.stats();
                self.nodes = st.nodes_solved;
                if let Some(b) = st.best_bound.filter(|b| b.is_finite()) {
                    self.bound = self.bound.max(b);
                }
                if let Some(sol) = out.solution() {
                    let pt = point_of(&self.m, &self.inp, sol);
                    self.offer(pt);
                }
                match out.termination_reason() {
                    // microlp met the relative gap it was given; that gap came from the plan in hand
                    // when the call started, so the proof stands only if the building rule holds
                    // for the plan in hand now (a stale gap would prove too early)
                    TerminationReason::MipGap if self.closed(1.0) => {
                        self.proven = true;
                        self.phase = Phase::Done;
                    }
                    TerminationReason::MipGap if self.last_mipgap == Some((gap, self.version)) => {
                        // the same gap again and no better plan (microlp's incumbent refused): a
                        // further resume would return at once, so the run ends unproven
                        self.phase = Phase::Done;
                    }
                    TerminationReason::ProvenOptimal => {
                        // proven optimal (tree exhausted): the best plan is at least as good as the engine's
                        self.proven = self.best.is_some();
                        self.phase = Phase::Done;
                    }
                    reason => {
                        if reason == TerminationReason::MipGap {
                            self.last_mipgap = Some((gap, self.version));
                        }
                        self.full = Some(out);
                        if self.ms() >= self.deadline {
                            self.phase = Phase::Done;
                        }
                    }
                }
            }
            Err(microlp::Error::Infeasible) => {
                self.failure = Some("No whole-building plan fits the Somersloop / power shard budget.".into());
                self.phase = Phase::Done;
            }
            Err(e) => {
                self.failure = Some(format!("Solver error: {e}"));
                self.phase = Phase::Done;
            }
        }
    }

    /// Gives the solve `ms` more from now ("keep solving"); resumes the MILP where it stopped.
    pub fn extend(&mut self, ms: f64) {
        self.deadline = self.ms() + ms.clamp(100.0, 300_000.0);
        if self.phase == Phase::Done && !self.proven && self.failure.is_none() {
            self.phase = Phase::Full;
        }
    }

    /// The whole-building model (after the exact reductions) as CPLEX LP text, columns `v0, v1, …`
    /// in model order, for an external MILP engine (HiGHS in the browser).
    pub fn model_lp(&self) -> String {
        lp_text(&self.m)
    }

    /// Current best plan as a value per model column (`v0, v1, …`), for an engine's initial
    /// solution; empty without one.
    pub fn best_values(&self) -> Vec<f64> {
        let Some((_, pt)) = &self.best else { return vec![] };
        let mut out = vec![0.0; self.m.cost.len()];
        for (v, x) in values_of(&self.m, &self.inp, pt) {
            out[v.idx()] = x;
        }
        out
    }

    /// A whole-building solution from an external engine (values per column of `model_lp`): taken
    /// as the best plan when it fits the budgets and is cheaper. `bound`: the engine's proven lower
    /// bound (its optimum when it proved optimality). `proven` ends the solve as optimal only when
    /// the plan in hand is within the gap of that bound: when the engine's own solution was refused
    /// (row or budget check) the plan in hand is an older, dearer one, and stays unproven (review 4 H1).
    pub fn offer_values(&mut self, values: &[f64], bound: Option<f64>, proven: bool) {
        if values.len() == self.m.cost.len() && values.iter().all(|v| v.is_finite()) {
            let m = &self.m;
            // whole buildings and shards rounded; continuous columns (shared-machine shares) as given
            let vals: Vec<f64> =
                values.iter().zip(&m.bounds).map(|(v, (_, _, int))| if *int { v.round() } else { *v }).collect();
            let pt = point_from(m, &self.inp, |v| vals[v.idx()]);
            if self.rows_hold(&pt) {
                self.offer(pt);
            }
        }
        if let Some(b) = bound.filter(|b| b.is_finite()) {
            // the engine's bound is over the same model; never above the plan in hand
            let cap = self.best.as_ref().map(|b| b.0).unwrap_or(f64::INFINITY);
            self.bound = self.bound.max(b.min(cap));
        }
        if proven && self.closed(2.0) {
            self.proven = true;
            self.phase = Phase::Done;
        }
    }

    /// Every row and bound of the whole-building model holds at `pt` (to 1e-6 relative): an
    /// external engine's values are checked before they become a plan.
    fn rows_hold(&self, pt: &Point) -> bool {
        let vals = values_of(&self.m, &self.inp, pt);
        let mut x = vec![0.0; self.m.cost.len()];
        for (v, a) in vals {
            x[v.idx()] = a;
        }
        let tol = |r: f64| 1e-6 * (1.0 + r.abs());
        let bounds_ok = self.m.bounds.iter().zip(&x).all(|((lo, hi, _), v)| *v >= lo - tol(*lo) && *v <= hi + tol(*hi));
        bounds_ok
            && self.m.rows.iter().all(|(terms, op, rhs)| {
                let lhs: f64 = terms.iter().map(|(i, a)| a * x[*i]).sum();
                let t = tol(*rhs).max(1e-6 * terms.iter().map(|(i, a)| (a * x[*i]).abs()).fold(0.0, f64::max));
                match op {
                    ComparisonOp::Le => lhs <= rhs + t,
                    ComparisonOp::Ge => lhs >= rhs - t,
                    ComparisonOp::Eq => (lhs - rhs).abs() <= t,
                }
            })
    }

    /// Ends the microlp search: the plan in hand is final (an external engine took over).
    pub fn finish_now(&mut self) {
        self.phase = Phase::Done;
    }

    /// Runs the heuristic phases (reweighted LPs, restricted MILP) only, leaving the full search.
    pub fn heuristics_done(&self) -> bool {
        matches!(self.phase, Phase::Full | Phase::Done)
    }

    pub fn progress(&self) -> Progress {
        Progress {
            elapsed_ms: self.ms(),
            objective: self.best.as_ref().map(|b| b.0),
            bound: self.bound,
            gap: self.gap(),
            proven: self.proven,
            done: self.phase == Phase::Done,
            phase: match self.phase {
                Phase::Reweight(_) => "heuristic",
                Phase::Sub => "restricted",
                Phase::Full => "search",
                Phase::Done => "done",
            },
            version: self.version,
            nodes: self.nodes,
            deadline_ms: self.deadline,
        }
    }

    /// The best plan in hand (a finished plan once `step` returned true).
    pub fn plan(&self) -> Plan {
        let mut plan = self.base.clone();
        let done = self.phase == Phase::Done;
        plan.solve_ms = self.ms();
        let Some((obj, pt)) = &self.best else {
            plan.status = Status::Infeasible;
            if done {
                plan.warnings.push(
                    self.failure.clone().unwrap_or_else(|| "Time limit reached before a whole-building plan was found.".into()),
                );
            }
            return plan;
        };
        plan.status = Status::Optimal;
        plan.objective = *obj;
        plan.proven_optimal = self.proven;
        let gap = self.gap().unwrap_or(0.0);
        plan.gap = if gap > 0.0 { Some(gap) } else { None };
        plan.gap_buildings = if self.proven { None } else { self.gap_units() };
        if done && !self.proven {
            let units = match self.gap_units() {
                Some(u) => format!(", about {u:.1} of the cheapest building"),
                None => String::new(),
            };
            plan.warnings.push(format!(
                "Time limit reached: best whole-building plan found, not proven optimal (within {:.2} % of the bound{units}).",
                gap * 100.0
            ));
        }
        match detier(self.data, &self.s, &self.inp, &self.dropped, pt) {
            Some((inp, pt)) => finish(self.data, &self.s, &inp, &pt, true, &mut plan),
            None => finish(self.data, &self.s, &self.inp, pt, true, &mut plan),
        }
        plan
    }
}

/// CPLEX LP text of a model (columns v0, v1, …).
fn lp_text(m: &Model) -> String {
    use std::fmt::Write;
    let mut s = String::from("Minimize\n obj:");
    let mut k = 0;
    for (i, c) in m.cost.iter().enumerate() {
        if *c != 0.0 {
            write!(s, " {:+.17e} v{}", c, i).unwrap();
            k += 1;
            if k % 8 == 0 {
                s.push('\n');
            }
        }
    }
    if k == 0 {
        s.push_str(" 0 v0");
    }
    s.push_str("\nSubject To\n");
    for (r, (terms, op, rhs)) in m.rows.iter().enumerate() {
        if terms.is_empty() {
            continue;
        }
        write!(s, " c{}:", r).unwrap();
        for (j, (v, a)) in terms.iter().enumerate() {
            write!(s, " {:+.17e} v{}", a, v).unwrap();
            if j % 8 == 7 {
                s.push('\n');
            }
        }
        let op = match op {
            ComparisonOp::Le => "<=",
            ComparisonOp::Eq => "=",
            ComparisonOp::Ge => ">=",
        };
        writeln!(s, " {} {:.17e}", op, rhs).unwrap();
    }
    s.push_str("Bounds\n");
    for (i, (lo, hi, _)) in m.bounds.iter().enumerate() {
        if hi.is_finite() {
            writeln!(s, " {:.17e} <= v{} <= {:.17e}", lo, i, hi).unwrap();
        } else if *lo != 0.0 {
            writeln!(s, " v{} >= {:.17e}", i, lo).unwrap();
        }
    }
    s.push_str("General\n");
    for (i, (_, _, int)) in m.bounds.iter().enumerate() {
        if *int {
            writeln!(s, " v{}", i).unwrap();
        }
    }
    s.push_str("End\n");
    s
}

/// Sets up a solve: model, LP relaxation, and (whole buildings) the first plan in hand.
pub fn start<'a>(data: &'a GameData, s: &PlanSettings) -> Start<'a> {
    let started = web_time::Instant::now();
    let mut plan = Plan::default();
    // quality tiers (`x@q`) only where the data has them and some target or supply asks for one
    let quality_on = quality::active(data, s);
    let resolve = |key: &str| {
        let q = if quality_on { quality::split(key).1 } else { 0 };
        let it = data.resolve_item(key)?;
        // fluids have no quality
        Some(quality::qid(&it.id, if it.liquid { 0 } else { q }))
    };

    let mut targets: BTreeMap<String, f64> = BTreeMap::new();
    for t in &s.targets {
        match resolve(&t.item) {
            Some(id) => *targets.entry(id).or_insert(0.0) += t.rate,
            None => plan.warnings.push(format!("unknown target item '{}'", t.item)),
        }
    }
    let mut supplies: BTreeMap<String, f64> = BTreeMap::new();
    for t in &s.supplies {
        if let Some(id) = resolve(&t.item) {
            *supplies.entry(id).or_insert(0.0) += t.rate;
        }
    }
    if !quality_on && s.targets.iter().chain(&s.supplies).any(|t| quality::split(&t.item).1 > 0) {
        plan.warnings.push(
            "This data set has no quality: items asked for at a quality above normal are planned as normal.".into(),
        );
    }
    if targets.is_empty() {
        plan.status = Status::Empty;
        return Start::Done(plan);
    }

    let allowed = allowed_recipes(data, s);
    let target_ids: Vec<String> = targets.keys().cloned().collect();
    if quality_on {
        // quality plans: expanded recipes `r@i` with loadout modes (pruned at the base level
        // first); the units borrow these local recipes, so the plan is finished here
        let expanded = quality::expand(data, s, &allowed, &target_ids);
        let all_units: Vec<Unit> = expanded
            .into_iter()
            .flat_map(|(r, ms)| {
                let r = Arc::new(r);
                ms.into_iter().map(move |m| Unit::new(r.clone(), m))
            })
            .collect();
        let units = relevant(&all_units, &target_ids);
        return start_units(data, s, started, plan, units, targets, supplies);
    }
    // item recycling only makes sense with quality
    let all_units: Vec<Unit> = allowed
        .iter()
        .filter(|r| !r.recycling)
        .flat_map(|r| {
            let rc = Arc::new((*r).clone());
            rules::modes(data, r, s).into_iter().map(move |mode| Unit::new(rc.clone(), mode))
        })
        .collect();
    let units = relevant(&all_units, &target_ids);
    start_units(data, s, started, plan, units, targets, supplies)
}

fn start_units<'b>(
    data: &'b GameData,
    s: &PlanSettings,
    started: web_time::Instant,
    mut plan: Plan,
    units: Vec<Unit>,
    targets: BTreeMap<String, f64>,
    supplies: BTreeMap<String, f64>,
) -> Start<'b> {
    let name = |id: &str| data.item_name(id);
    let quality_on = quality::active(data, s);
    let mut weights = s.weights();
    for v in [&mut weights.resources, &mut weights.buildings, &mut weights.power, &mut weights.shards, &mut weights.surplus] {
        if !v.is_finite() || *v < 0.0 {
            *v = 0.0;
        }
    }
    plan.columns = units.len();
    plan.quality = quality_on;
    let integer = s.integer;
    // Quality loops run the tiers of a recipe on shared machines that switch recipes (report
    // §2.1): whole machines per (base recipe, machine, loadout) group, not per tier column.
    let shared = integer && quality_on;
    // whole buildings: drop dominated machine tiers / modes (exact, report §6 step 2; per column,
    // so not with shared machines, where a column's group may have spare capacity)
    let (units, dropped) = if integer && !shared && shrink() {
        drop_dominated(data, s, units, &weights)
    } else {
        (units, HashMap::new())
    };
    let items = items_of(&units, &targets, &supplies);
    let inp = Inputs { units, items, targets, supplies, weights, shared };
    plan.rows = inp.items.len();
    if s.max_columns > 0 && plan.columns > s.max_columns {
        plan.status = Status::TooLarge;
        plan.warnings.push(format!(
            "Large model: {} columns × {} items (the limit for automatic solving is {}).",
            plan.columns, plan.rows, s.max_columns
        ));
        plan.solve_ms = started.elapsed().as_secs_f64() * 1e3;
        return Start::Done(plan);
    }

    // 1. continuous relaxation: lower bound, shadow prices, first plan
    let lp_model = build(data, s, &inp, false);
    let lp = match lp_model.p.solve().map(|o| o.into_solution()) {
        Ok(Ok(sol)) => sol,
        Ok(Err(_)) => {
            plan.status = Status::Infeasible;
            plan.warnings.push("Solver stopped before finding a plan.".into());
            return Start::Done(plan);
        }
        Err(microlp::Error::Infeasible) => {
            plan.status = Status::Infeasible;
            plan.warnings.push(
                "No feasible plan: targets exceed resource limits or need a disabled recipe.".into(),
            );
            // items with no source left in the model whose recipes or resource the planet filter
            // removed, with the planets that would provide them (review 3 M2)
            let loc_name = |l: &String| data.locations.iter().find(|x| x.id == *l).map(|x| x.name.clone()).unwrap_or(l.clone());
            let blocking = blocking_items(data, s, &inp);
            let off: Vec<String> = blocking
                .iter()
                .filter_map(|it| {
                    let base = quality::split(it).0;
                    let mut at: BTreeSet<String> = BTreeSet::new();
                    if let Some(r) = data.resource(base) {
                        if data.located(&r.locations, &s.planets) {
                            return None; // mined here: limited, not missing
                        }
                        at.extend(r.locations.iter().cloned());
                    }
                    for r in data.recipes.iter().filter(|r| !r.recycling && r.outputs.iter().any(|o| o.0 == base)) {
                        if s.disabled_recipes.contains(&r.id) || data.recipe_located(r, &s.planets, &s.disabled_buildings) {
                            continue;
                        }
                        if !r.locations.is_empty() {
                            at.extend(r.locations.iter().cloned());
                        } else {
                            for p in r.producers.iter().chain(std::iter::once(&r.building)) {
                                at.extend(data.building(p).map(|b| b.locations.clone()).unwrap_or_default());
                            }
                        }
                    }
                    at.retain(|l| !s.planets.contains(l));
                    (!at.is_empty() && !data.locations.is_empty())
                        .then(|| format!("{} ({})", name(it), at.iter().map(loc_name).collect::<Vec<_>>().join(", ")))
                })
                .collect();
            if !off.is_empty() {
                plan.warnings.push(format!(
                    "Not on the chosen planets: {}. Add the planet or give the item as an input.",
                    off.join("; ")
                ));
            }
            return Start::Done(plan);
        }
        Err(microlp::Error::Unbounded) => {
            plan.status = Status::Unbounded;
            return Start::Done(plan);
        }
        Err(e) => {
            plan.status = Status::Infeasible;
            plan.warnings.push(format!("Solver error: {e}"));
            return Start::Done(plan);
        }
    };
    plan.lp_bound = lp.objective();
    if let Some(duals) = lp.duals() {
        for (it, c) in &lp_model.balance {
            let v = duals[c.idx()];
            if v.abs() > 1e-9 {
                plan.item_values.push(ItemValue { item: it.clone(), name: name(it), value: v });
            }
        }
        plan.item_values.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
    }
    let lp_pt = point_of(&lp_model, &inp, &lp);

    if !integer {
        plan.status = Status::Optimal;
        plan.objective = lp.objective();
        plan.proven_optimal = true;
        finish(data, s, &inp, &lp_pt, false, &mut plan);
        plan.solve_ms = started.elapsed().as_secs_f64() * 1e3;
        return Start::Done(plan);
    }

    // 2. whole buildings: the LP rounded up is the first plan in hand
    let c_max = s.max_clock();
    let m = build(data, s, &inp, true);
    let support: BTreeSet<String> = lp_pt.rec.iter().filter(|(_, v)| v.0 > 1e-7).map(|(k, _)| k.clone()).collect();
    let budget = s.time_limit_ms.clamp(200, 300_000) as f64;
    let lp_ms = started.elapsed().as_secs_f64() * 1e3;
    let mut run = Run {
        data,
        s: s.clone(),
        started,
        budget,
        deadline: budget,
        bound: plan.lp_bound,
        base: plan,
        prev: lp_pt.rec.clone(),
        inp,
        dropped,
        m,
        c_max,
        lp_pt,
        lp_ms,
        support,
        sub: None,
        full: None,
        best: None,
        version: 0,
        proven: false,
        phase: Phase::Reweight(0),
        failure: None,
        nodes: 0,
        fallback_version: None,
        model_unit: f64::INFINITY,
        unit: f64::INFINITY,
        // buildings a main cost: not below the resource weight, nor below a tenth of the MW weight
        // (Power preset 1 vs 1: on; buildings as a 0.01 tie-break to power or resources: off)
        last_mipgap: None,
        buildings_primary: {
            let w = s.weights();
            w.buildings > 0.0 && w.buildings >= w.resources && w.buildings >= 0.1 * w.power
        },
    };
    run.model_unit = cheapest(&run.m, &building_cols(&run.m), |_| true);
    run.unit = run.model_unit;
    let first = rounded(&run.inp, &run.lp_pt, c_max, true);
    if within_budgets(s, &run.inp, &first) {
        run.offer(first);
    } else if let Some(pt) = run.budget_fallback() {
        run.offer(pt);
        run.fallback_version = Some(run.version);
    }
    if run.closed(1.0) {
        run.proven = true;
        run.phase = Phase::Done;
    }
    Start::Running(Box::new(run))
}

/// Solves a plan to the end (time limit or proof).
pub fn solve(data: &GameData, s: &PlanSettings) -> Plan {
    match start(data, s) {
        Start::Done(p) => p,
        Start::Running(mut run) => {
            while !run.step(None) {}
            run.plan()
        }
    }
}

/// Map from item id to total raw input (handy in tests).
pub fn raw_map(plan: &Plan) -> HashMap<String, f64> {
    plan.raw.iter().map(|r| (r.item.clone(), r.rate)).collect()
}
