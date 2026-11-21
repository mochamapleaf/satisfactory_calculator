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
    ComparisonOp, OptimizationDirection, Problem, Solution, SolutionStatus, SolveOptions,
    TerminationReason, Variable,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

/// Breakpoints (clock as a fraction) of the piecewise-linear power curve.
const POWER_BREAKS: [f64; 9] = [0.0, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 2.5];
const DEFAULT_WATER_WEIGHT: f64 = 0.001;
/// Iron Ore world limit: scarcity weights are relative to it (Iron Ore = 1).
const SCARCITY_REF: f64 = 92_100.0;
const MAX_BUILDINGS: i32 = 1_000_000;
/// Relative gap at which a whole-building plan counts as optimal.
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
fn relevant<'a>(units: &[Unit<'a>], targets: &[String]) -> Vec<Unit<'a>> {
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
    rv: Vec<RecipeVars>,
    raw: Vec<(String, Variable)>,
    sup: Vec<(String, Variable)>,
    surplus: Vec<(String, Variable)>,
    balance: Vec<(String, microlp::Constraint)>,
    segments: Vec<Vec<(f64, f64)>>,
}

/// One way of running one recipe: a column of the model.
#[derive(Clone)]
struct Unit<'a> {
    recipe: &'a Recipe,
    mode: Mode,
    key: String,
    /// Inputs (incl. the mode's extra inputs) and products per unit of work.
    ins: Vec<(String, f64)>,
    outs: Vec<(String, f64)>,
}

impl<'a> Unit<'a> {
    fn new(recipe: &'a Recipe, mode: Mode) -> Self {
        let ins = recipe.inputs.iter().chain(&mode.extra_inputs).cloned().collect();
        let outs = rules::outputs(recipe, &mode);
        Unit { recipe, key: format!("{}#{}", recipe.id, mode.key), mode, ins, outs }
    }
}

struct Inputs<'a> {
    units: Vec<Unit<'a>>,
    items: BTreeSet<String>,
    targets: BTreeMap<String, f64>,
    supplies: BTreeMap<String, f64>,
    weights: CostWeights,
}

fn shards_per_machine(c_max: f64) -> u32 {
    if c_max <= 1.0 + 1e-9 {
        0
    } else {
        ((c_max - 1.0) / 0.5 - 1e-9).ceil() as u32
    }
}

fn build(data: &GameData, s: &PlanSettings, inp: &Inputs, integer: bool) -> Model {
    build_scaled(data, s, inp, integer, &HashMap::new())
}

/// `scale`: per-recipe multiplier on the building cost (heuristic re-weighting).
fn build_scaled(
    data: &GameData,
    s: &PlanSettings,
    inp: &Inputs,
    integer: bool,
    scale: &HashMap<String, f64>,
) -> Model {
    let w = &inp.weights;
    let c_max = s.max_clock();
    let k_max = shards_per_machine(c_max);
    let mut p = Problem::new(OptimizationDirection::Minimize);
    let mut rv = Vec::new();
    let mut segments = Vec::new();
    for u in &inp.units {
        let f = &u.mode;
        let x = p.add_var(0.0, (0.0, f64::INFINITY));
        // building weight plus the per-building compute terms (sorters, logistics, coaters);
        // added here, not in `scale`, which is a multiplier rebuilt every heuristic round
        let wb = w.buildings
            * compute::machine_cost(w, u.recipe, f, s.mixed_belts)
            * scale.get(&u.key).copied().unwrap_or(1.0);
        let (c_max, k_max) = if f.overclock { (c_max, k_max) } else { (1.0, 0) };
        let n = if integer {
            p.add_integer_var(wb, (0, MAX_BUILDINGS))
        } else {
            p.add_var(wb, (0.0, f64::INFINITY))
        };
        p.add_constraint(&[(x, 1.0), (n, -c_max * f.speed)], ComparisonOp::Le, 0.0);
        let sv = if k_max > 0 {
            let sv = if integer {
                p.add_integer_var(w.shards, (0, MAX_BUILDINGS))
            } else {
                p.add_var(w.shards, (0.0, f64::INFINITY))
            };
            p.add_constraint(&[(x, 1.0), (n, -f.speed), (sv, -0.5 * f.speed)], ComparisonOp::Le, 0.0);
            p.add_constraint(&[(sv, 1.0), (n, -(k_max as f64))], ComparisonOp::Le, 0.0);
            Some(sv)
        } else {
            None
        };
        let segs = power_segments(f.power_exp, c_max);
        let pv = if w.power > 0.0 && f.power_base > 0.0 {
            let pv = p.add_var(w.power, (0.0, f64::INFINITY));
            for (a, b) in &segs {
                // p ≥ P·(a·x + b·n)
                p.add_constraint(
                    &[(pv, 1.0), (x, -f.power_base * a / f.speed), (n, -f.power_base * b)],
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
    // boost items held per building (Somersloops, modules) against their budgets
    for (item, budget) in budgets(s) {
        let row: Vec<(Variable, f64)> = inp
            .units
            .iter()
            .zip(&rv)
            .filter_map(|(u, v)| u.mode.held.iter().find(|(i, _)| *i == item).map(|(_, q)| (v.n, *q)))
            .collect();
        if !row.is_empty() {
            p.add_constraint(row.as_slice(), ComparisonOp::Le, budget);
        }
    }
    if let (Some(budget), true) = (s.shard_budget, k_max > 0) {
        let row: Vec<(Variable, f64)> = rv.iter().filter_map(|v| v.s.map(|s| (s, 1.0))).collect();
        if !row.is_empty() {
            p.add_constraint(row.as_slice(), ComparisonOp::Le, budget as f64);
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
            raw.push((it.clone(), p.add_var(c, (0.0, limit.unwrap_or(f64::INFINITY)))));
        }
    }
    let sup: Vec<(String, Variable)> =
        inp.supplies.iter().map(|(it, q)| (it.clone(), p.add_var(0.0, (0.0, *q)))).collect();
    let surplus: Vec<(String, Variable)> = inp
        .items
        .iter()
        .map(|it| (it.clone(), p.add_var(w.surplus, (0.0, f64::INFINITY))))
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
        balance.push((it.clone(), p.add_constraint(row.as_slice(), ComparisonOp::Eq, b)));
    }
    Model { p, rv, raw, sup, surplus, balance, segments }
}

/// Solver-independent snapshot of a solution, keyed by recipe / item id.
#[derive(Default)]
struct Point {
    rec: HashMap<String, (f64, f64, f64)>, // x, n, shards
    flows: HashMap<(u8, String), f64>,      // (0 raw | 1 supply | 2 surplus, item)
}

fn point_of(m: &Model, inp: &Inputs, sol: &Solution) -> Point {
    let mut pt = Point::default();
    for (u, v) in inp.units.iter().zip(&m.rv) {
        let sh = v.s.map(|s| sol[s]).unwrap_or(0.0);
        pt.rec.insert(u.key.clone(), (sol[v.x].max(0.0), sol[v.n].max(0.0), sh.max(0.0)));
    }
    for (k, list) in [(0u8, &m.raw), (1, &m.sup), (2, &m.surplus)] {
        for (it, v) in list {
            pt.flows.insert((k, it.clone()), sol[*v].max(0.0));
        }
    }
    pt
}

/// MILP starting point from `pt`. With `round_up`, buildings are rounded up from the
/// relaxation (always feasible unless a shard budget binds).
fn hint_of(m: &Model, inp: &Inputs, pt: &Point, c_max: f64, round_up: bool) -> Vec<(Variable, f64)> {
    let mut hint = Vec::new();
    for ((u, v), segs) in inp.units.iter().zip(&m.rv).zip(&m.segments) {
        let f = &u.mode;
        let c_max = if f.overclock { c_max } else { 1.0 };
        let (x, mut n, mut sh) = pt.rec.get(&u.key).copied().unwrap_or((0.0, 0.0, 0.0));
        if round_up {
            n = if x <= 1e-9 { 0.0 } else { (x / (c_max * f.speed) - 1e-9).ceil() };
            let c = if n > 0.0 { x / (n * f.speed) } else { 0.0 };
            sh = if c > 1.0 { ((c - 1.0) / 0.5 - 1e-9).ceil() * n } else { 0.0 };
        } else {
            n = n.round();
            sh = sh.round();
        }
        hint.push((v.x, x));
        hint.push((v.n, n));
        if let Some(s) = v.s {
            hint.push((s, sh));
        }
        if let Some(p) = v.p {
            let pw = segs.iter().map(|(a, b)| f.power_base * (a * x / f.speed + b * n)).fold(0.0, f64::max);
            hint.push((p, pw));
        }
    }
    for (k, list) in [(0u8, &m.raw), (1, &m.sup), (2, &m.surplus)] {
        for (it, v) in list {
            hint.push((*v, pt.flows.get(&(k, it.clone())).copied().unwrap_or(0.0)));
        }
    }
    hint
}

/// Inputs restricted to the recipes in `keep`.
fn restrict<'a>(inp: &Inputs<'a>, keep: &BTreeSet<String>) -> Inputs<'a> {
    let units: Vec<Unit> = inp.units.iter().filter(|u| keep.contains(&u.key)).cloned().collect();
    Inputs {
        items: items_of(&units, &inp.targets, &inp.supplies),
        units,
        targets: inp.targets.clone(),
        supplies: inp.supplies.clone(),
        weights: inp.weights.clone(),
    }
}

fn milp_options(ms: u64, hint: Vec<(Variable, f64)>) -> SolveOptions {
    let mut o = SolveOptions::default();
    o.time_limit = Some(Duration::from_millis(ms.max(50)));
    o.mip_gap = MIP_GAP;
    o.warm_start = Some(hint);
    o
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

fn finish(
    data: &GameData,
    s: &PlanSettings,
    inp: &Inputs,
    m: &Model,
    sol: &Solution,
    integer: bool,
    plan: &mut Plan,
) {
    let name = |id: &str| data.item_name(id);
    let clean = |v: f64| if v.abs() < 1e-7 { 0.0 } else { v };
    let c_max = s.max_clock();
    let mut summary: Option<ComputeSummary> = None;
    // DSP: compute cost of every plan (whatever the objective), so presets compare
    let cw = (data.game == "dsp").then(|| compute::model().weights(&s.generator));
    for (u, v) in inp.units.iter().zip(&m.rv) {
        let (r, f) = (u.recipe, &u.mode);
        let c_max = if f.overclock { c_max } else { 1.0 };
        let x = clean(sol[v.x]);
        if x <= 0.0 {
            continue;
        }
        let mut n = clean(sol[v.n]);
        let mut sh = v.s.map(|s| clean(sol[s])).unwrap_or(0.0);
        if integer {
            n = n.round();
            sh = sh.round();
        }
        let groups = machine_groups(n, x / f.speed, sh, c_max, integer);
        let power: f64 = groups
            .iter()
            .map(|g| g.count * f.power_base * (g.clock_percent / 100.0).powf(f.power_exp))
            .sum();
        let shards = if integer { sh } else { groups.iter().map(|g| g.count * g.shards as f64).sum() };
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
            beacons: n * f.beacons,
        });
        if let Some(w) = &cw {
            let k = compute::sorters_per_machine(r, f, s.mixed_belts, compute::model().sorter_capacity);
            let bw = w.building_weights.get(&f.building).copied().unwrap_or(1.0);
            // every placed entity costs, working or idle (spec §4.2): whole buildings also
            // when the plan is fractional; power stays as solved
            let ne = if integer { n } else { (n - 1e-6).ceil() };
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
    for (it, v) in &m.raw {
        let q = clean(sol[*v]);
        if q > 0.0 {
            let res = data.resource(it).unwrap();
            plan.extraction_power += q / res.extractor_rate * res.extractor_power;
            plan.raw.push(RateLine::new(it, &name(it), q));
        }
    }
    for (it, v) in &m.sup {
        let q = clean(sol[*v]);
        if q > 0.0 {
            plan.supplied.push(RateLine::new(it, &name(it), q));
        }
    }
    for (it, v) in &m.surplus {
        let q = clean(sol[*v]);
        if q > 1e-6 {
            plan.surplus.push(RateLine::new(it, &name(it), q));
        }
    }
    for (it, q) in &inp.targets {
        plan.targets.push(RateLine::new(it, &name(it), *q));
    }
    plan.production_power = plan.recipes.iter().map(|r| r.power).sum();
    // Buildings to place per type. Fractional quality plans: the tiers of a base recipe run
    // on shared machines that switch recipes, so round up per (base recipe, machine,
    // loadout), not per tier column.
    let mut groups: BTreeMap<(String, String), (String, f64)> = BTreeMap::new();
    for r in &plan.recipes {
        let base = if plan.quality { quality::split(&r.id).0 } else { r.id.as_str() };
        let g = groups.entry((base.to_string(), r.mode.clone())).or_insert((r.building.clone(), 0.0));
        g.1 += r.machines;
    }
    let mut counts: BTreeMap<String, f64> = BTreeMap::new();
    for (_, (b, n)) in groups {
        *counts.entry(b).or_insert(0.0) += if integer { n } else { (n - 1e-6).ceil() };
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

pub fn solve(data: &GameData, s: &PlanSettings) -> Plan {
    let started = web_time::Instant::now();
    let mut plan = Plan::default();
    let name = |id: &str| data.item_name(id);
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
        return plan;
    }

    let allowed = allowed_recipes(data, s);
    let target_ids: Vec<String> = targets.keys().cloned().collect();
    // quality plans: expanded recipes `r@i` with loadout modes (pruned at the base level first)
    let expanded = if quality_on { quality::expand(data, s, &allowed, &target_ids) } else { vec![] };
    let all_units: Vec<Unit> = if quality_on {
        expanded.iter().flat_map(|(r, ms)| ms.iter().map(move |m| Unit::new(r, m.clone()))).collect()
    } else {
        // item recycling only makes sense with quality
        allowed
            .iter()
            .filter(|r| !r.recycling)
            .flat_map(|r| rules::modes(data, r, s).into_iter().map(move |mode| Unit::new(r, mode)))
            .collect()
    };
    let units = relevant(&all_units, &target_ids);
    let items = items_of(&units, &targets, &supplies);
    let mut weights = s.weights();
    for v in [&mut weights.resources, &mut weights.buildings, &mut weights.power, &mut weights.shards, &mut weights.surplus] {
        if !v.is_finite() || *v < 0.0 {
            *v = 0.0;
        }
    }
    let inp = Inputs { units, items, targets, supplies, weights };
    plan.columns = inp.units.len();
    plan.rows = inp.items.len();
    plan.quality = quality_on;
    if s.max_columns > 0 && plan.columns > s.max_columns {
        plan.status = Status::TooLarge;
        plan.warnings.push(format!(
            "Large model: {} columns × {} items (the limit for automatic solving is {}).",
            plan.columns, plan.rows, s.max_columns
        ));
        plan.solve_ms = started.elapsed().as_secs_f64() * 1e3;
        return plan;
    }
    // Quality loops run tiers of a recipe on shared machines that switch recipes, which
    // per-column whole buildings would overcount (report §2.1): solve them continuously.
    let integer = s.integer && !quality_on;
    if quality_on && s.integer {
        plan.warnings.push("Quality plans are solved with fractional machines: the higher tiers of a recipe normally share machines that switch recipes.".into());
    }

    // 1. continuous relaxation: lower bound, shadow prices, warm start
    let lp_model = build(data, s, &inp, false);
    let lp = match lp_model.p.solve().map(|o| o.into_solution()) {
        Ok(Ok(sol)) => sol,
        Ok(Err(_)) => {
            plan.status = Status::Infeasible;
            plan.warnings.push("Solver stopped before finding a plan.".into());
            return plan;
        }
        Err(microlp::Error::Infeasible) => {
            plan.status = Status::Infeasible;
            plan.warnings.push(
                "No feasible plan: targets exceed resource limits or need a disabled recipe.".into(),
            );
            let off: Vec<String> = inp
                .items
                .iter()
                .filter(|it| data.resource(it).is_some_and(|r| !data.located(&r.locations, &s.planets)))
                .filter(|it| !inp.supplies.contains_key(*it))
                .map(|it| {
                    let r = data.resource(it).unwrap();
                    let at: Vec<&str> = r
                        .locations
                        .iter()
                        .map(|l| data.locations.iter().find(|x| x.id == *l).map(|x| x.name.as_str()).unwrap_or(l))
                        .collect();
                    format!("{} ({})", name(it), at.join(", "))
                })
                .collect();
            if !off.is_empty() {
                plan.warnings.push(format!(
                    "Not on the chosen planets: {}. Add the planet or give the item as an input.",
                    off.join("; ")
                ));
            }
            return plan;
        }
        Err(microlp::Error::Unbounded) => {
            plan.status = Status::Unbounded;
            return plan;
        }
        Err(e) => {
            plan.status = Status::Infeasible;
            plan.warnings.push(format!("Solver error: {e}"));
            return plan;
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

    if !integer {
        plan.status = Status::Optimal;
        plan.objective = lp.objective();
        plan.proven_optimal = true;
        finish(data, s, &inp, &lp_model, &lp, false, &mut plan);
        plan.solve_ms = started.elapsed().as_secs_f64() * 1e3;
        return plan;
    }

    // 2. primal heuristic: MILP over the recipes the relaxation uses (few integer
    //    variables, solves fast), then 3. the full MILP warm-started from it.
    let c_max = s.max_clock();
    let budget = s.time_limit_ms.clamp(200, 300_000);
    let lp_pt = point_of(&lp_model, &inp, &lp);
    let m = build(data, s, &inp, true);
    let mut hint = hint_of(&m, &inp, &lp_pt, c_max, true);
    let mut support: BTreeSet<String> =
        lp_pt.rec.iter().filter(|(_, v)| v.0 > 1e-7).map(|(k, _)| k.clone()).collect();
    // Whole buildings make small recipes dear: re-solve the LP charging each recipe
    // ceil(n)/n of its building cost at its last rate (sparser plans), and keep every
    // recipe these LPs use as the neighbourhood for the restricted MILP.
    if inp.weights.buildings > 0.0 {
        let mut prev = lp_pt.rec.clone();
        for _ in 0..4 {
            let scale: HashMap<String, f64> = prev
                .iter()
                .filter(|(_, v)| v.1 > 1e-7)
                .map(|(k, v)| (k.clone(), ((v.1 - 1e-9).ceil() / v.1).min(1e3)))
                .collect();
            let rm = build_scaled(data, s, &inp, false, &scale);
            match rm.p.solve().map(|o| o.into_solution()) {
                Ok(Ok(sol)) => {
                    let pt = point_of(&rm, &inp, &sol);
                    support.extend(pt.rec.iter().filter(|(_, v)| v.0 > 1e-7).map(|(k, _)| k.clone()));
                    prev = pt.rec;
                }
                _ => break,
            }
        }
    }
    if support.len() < inp.units.len() {
        let sub_inp = restrict(&inp, &support);
        let sub = build(data, s, &sub_inp, true);
        let sub_hint = hint_of(&sub, &sub_inp, &lp_pt, c_max, true);
        if let Ok(Ok(sol)) = sub.p.solve_with(milp_options(budget / 4, sub_hint)).map(|o| o.into_solution()) {
            hint = hint_of(&m, &inp, &point_of(&sub, &sub_inp, &sol), c_max, false);
        }
    }
    let left = budget.saturating_sub(started.elapsed().as_millis() as u64);
    match m.p.solve_with(milp_options(left, hint)) {
        Ok(outcome) => match outcome.into_solution() {
            Ok(sol) => {
                plan.status = Status::Optimal;
                plan.objective = sol.objective();
                plan.proven_optimal = sol.status() == SolutionStatus::Optimal
                    || sol.termination_reason() == TerminationReason::MipGap;
                plan.gap = if sol.status() == SolutionStatus::Optimal { None } else { sol.gap() };
                if !plan.proven_optimal {
                    plan.warnings.push(format!(
                        "Time limit reached: best whole-building plan found, not proven optimal{}.",
                        sol.gap().map(|g| format!(" (within {:.1} % of the bound)", g * 100.0)).unwrap_or_default()
                    ));
                }
                finish(data, s, &inp, &m, &sol, true, &mut plan);
            }
            Err(_) => {
                plan.status = Status::Infeasible;
                plan.warnings.push("Time limit reached before a whole-building plan was found.".into());
            }
        },
        Err(microlp::Error::Infeasible) => {
            plan.status = Status::Infeasible;
            plan.warnings.push("No whole-building plan fits (check the shard budget).".into());
        }
        Err(e) => {
            plan.status = Status::Infeasible;
            plan.warnings.push(format!("Solver error: {e}"));
        }
    }
    plan.solve_ms = started.elapsed().as_secs_f64() * 1e3;
    plan
}

/// Map from item id to total raw input (handy in tests).
pub fn raw_map(plan: &Plan) -> HashMap<String, f64> {
    plan.raw.iter().map(|r| (r.item.clone(), r.rate)).collect()
}
