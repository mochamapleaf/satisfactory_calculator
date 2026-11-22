import sys
p = sys.argv[1]
t = open(p, encoding='utf-8').read()

ins = r'''
/// Experiment switches (scratch benchmark only).
#[derive(Clone, Debug)]
pub struct Tune {
    pub gap: f64,
    pub skip_sub: bool,
    pub no_hint: bool,
    pub presolve: bool,
    pub slice_ms: u64,
    pub dominated: bool,
    pub tighten: bool,
    pub lattice: bool,
    pub s_cont: bool,
    pub linpow: bool,
    pub h1: bool,
    pub hint_int: bool,
    pub ls: f64,
    pub subfrac: f64,
    pub dump: Option<String>,
}
impl Default for Tune {
    fn default() -> Self {
        Tune { gap: MIP_GAP, skip_sub: false, no_hint: false, presolve: true, slice_ms: 0, dominated: false,
               tighten: false, lattice: false, s_cont: false, linpow: false, h1: false, hint_int: false, ls: 0.0, subfrac: 0.25, dump: None }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Trace {
    pub units: usize, pub units_dropped: usize, pub int_vars: usize, pub vars: usize, pub rows: usize,
    pub t_lp: f64, pub lp_obj: f64,
    pub t_support: f64, pub support: usize,
    pub ceil_obj: f64, pub t_ceil: f64,
    pub t_sub: f64, pub sub_obj: f64, pub sub_nodes: u64, pub sub_proven: bool,
    pub t_end: f64, pub obj: f64, pub bound: f64, pub nodes: u64, pub lp_iters: u64, pub proven: bool,
    pub tightened: usize,
    pub ls_obj: f64, pub t_ls: f64, pub ls_evals: usize, pub ls_moves: usize,
    /// (ms since start, incumbent, best bound, nodes)
    pub traj: Vec<(f64, f64, f64, u64)>,
}
thread_local! {
    pub static TUNE: std::cell::RefCell<Tune> = std::cell::RefCell::new(Tune::default());
    pub static TRACE: std::cell::RefCell<Trace> = std::cell::RefCell::new(Trace::default());
}
fn tune() -> Tune { TUNE.with(|t| t.borrow().clone()) }

#[derive(Default, Clone)]
pub struct Dump {
    pub vars: Vec<(f64, f64, f64, bool)>,
    pub rows: Vec<(Vec<(usize, f64)>, i8, f64)>,
}
impl Dump {
    pub fn write_lp(&self, path: &str) {
        use std::fmt::Write;
        let mut s = String::from("Minimize\n obj:");
        let mut k = 0;
        for (i, v) in self.vars.iter().enumerate() {
            if v.0 != 0.0 { write!(s, " {:+.17e} v{}", v.0, i).unwrap(); k += 1; if k % 6 == 0 { s.push('\n'); } }
        }
        s.push_str("\nSubject To\n");
        for (r, (terms, op, rhs)) in self.rows.iter().enumerate() {
            if terms.is_empty() { continue; }
            write!(s, " c{}:", r).unwrap();
            for (j, (v, a)) in terms.iter().enumerate() { write!(s, " {:+.17e} v{}", a, v).unwrap(); if j % 6 == 5 { s.push('\n'); } }
            writeln!(s, " {} {:.17e}", match op { -1 => "<=", 0 => "=", _ => ">=" }, rhs).unwrap();
        }
        s.push_str("Bounds\n");
        for (i, v) in self.vars.iter().enumerate() {
            if v.2.is_finite() { writeln!(s, " {:.17e} <= v{} <= {:.17e}", v.1, i, v.2).unwrap(); }
            else if v.1 != 0.0 { writeln!(s, " v{} >= {:.17e}", i, v.1).unwrap(); }
        }
        s.push_str("General\n");
        for (i, v) in self.vars.iter().enumerate() { if v.3 { writeln!(s, " v{}", i).unwrap(); } }
        s.push_str("End\n");
        std::fs::write(path, s).unwrap();
    }
}
struct PB { p: Problem, d: Dump }
impl PB {
    fn add_var(&mut self, c: f64, b: (f64, f64)) -> Variable { self.d.vars.push((c, b.0, b.1, false)); self.p.add_var(c, b) }
    fn add_integer_var(&mut self, c: f64, b: (i32, i32)) -> Variable { self.d.vars.push((c, b.0 as f64, b.1 as f64, true)); self.p.add_integer_var(c, b) }
    fn add_constraint(&mut self, t: &[(Variable, f64)], op: ComparisonOp, rhs: f64) -> microlp::Constraint {
        let o = match op { ComparisonOp::Le => -1, ComparisonOp::Eq => 0, ComparisonOp::Ge => 1 };
        self.d.rows.push((t.iter().map(|(v, a)| (v.idx(), *a)).collect(), o, rhs));
        self.p.add_constraint(t, op, rhs)
    }
}

struct RecipeVars {'''


def rep(old, new):
    global t
    assert old in t, old[:60]
    t = t.replace(old, new, 1)


rep('\nstruct RecipeVars {', ins)
rep('''    segments: Vec<Vec<(f64, f64)>>,
}''', '''    segments: Vec<Vec<(f64, f64)>>,
    dump: Dump,
}''')
rep('''struct Inputs<'a> {
    units: Vec<Unit<'a>>,''', '''struct Inputs<'a> {
    /// per unit key: (lower, upper) bound on buildings from own presolve
    nb: HashMap<String, (i32, i32)>,
    units: Vec<Unit<'a>>,''')
rep("    let mut p = Problem::new(OptimizationDirection::Minimize);\n    let mut rv = Vec::new();",
    "    let tn = tune();\n    let mut p = PB { p: Problem::new(OptimizationDirection::Minimize), d: Dump::default() };\n    let mut rv = Vec::new();")
rep('''        let n = if integer {
            p.add_integer_var(wb, (0, MAX_BUILDINGS))
        } else {
            p.add_var(wb, (0.0, f64::INFINITY))
        };''', '''        let (nlo, nhi) = inp.nb.get(&u.key).copied().unwrap_or((0, MAX_BUILDINGS));
        let wb = if tn.lattice && integer { (wb * 1024.0).round() / 1024.0 } else { wb };
        let n = if integer {
            p.add_integer_var(wb, (nlo, nhi))
        } else {
            p.add_var(wb, (0.0, f64::INFINITY))
        };''')
rep('''            let sv = if integer {
                p.add_integer_var(w.shards, (0, MAX_BUILDINGS))''', '''            let sv = if integer && !tn.s_cont {
                p.add_integer_var(w.shards, (0, MAX_BUILDINGS))''')
rep("    Model { p, rv, raw, sup, surplus, balance, segments }",
    "    Model { p: p.p, dump: p.d, rv, raw, sup, surplus, balance, segments }")
rep('''    Inputs {
        items: items_of(&units, &inp.targets, &inp.supplies),''', '''    Inputs {
        nb: inp.nb.clone(),
        items: items_of(&units, &inp.targets, &inp.supplies),''')
rep('''    o.mip_gap = MIP_GAP;
    o.warm_start = Some(hint);''', '''    let tn = tune();
    o.mip_gap = tn.gap;
    o.presolve = tn.presolve;
    o.warm_start = if tn.no_hint { None } else { Some(hint) };''')

rep('''    let inp = Inputs { units, items, targets, supplies, weights };
    plan.columns = inp.units.len();''', '''    let tn = tune();
    let mut tr = Trace::default();
    let n_before = units.len();
    let units = if tn.dominated { drop_dominated(data, s, units, &weights) } else { units };
    let items = if tn.dominated { items_of(&units, &targets, &supplies) } else { items };
    tr.units = units.len();
    tr.units_dropped = n_before - units.len();
    if tn.lattice {
        weights.resources = 0.0;
        weights.shards = 0.0;
        weights.surplus = 0.0;
        weights.power = 0.0;
    }
    let mut inp = Inputs { nb: HashMap::new(), units, items, targets, supplies, weights };
    if tn.tighten && s.integer {
        inp.nb = implied_bounds(data, s, &inp);
        tr.tightened = inp.nb.values().filter(|b| b.0 > 0).count();
    }
    plan.columns = inp.units.len();''')

rep('''    plan.lp_bound = lp.objective();''', '''    plan.lp_bound = lp.objective();
    tr.t_lp = started.elapsed().as_secs_f64() * 1e3;
    tr.lp_obj = lp.objective();''')


# linear power (power_exp == 1): no p variable, no segment rows; cost goes on x and n
rep("""        let x = p.add_var(0.0, (0.0, f64::INFINITY));""", """        let lin = tn.linpow && w.power > 0.0 && u.mode.power_exp == 1.0 && !u.mode.overclock;
        let x = p.add_var(if lin { w.power * u.mode.power_base / u.mode.speed } else { 0.0 }, (0.0, f64::INFINITY));""")
rep("""            + rules::module_cost(data, s, f);""", """            + rules::module_cost(data, s, f)
            + if lin { w.power * f.beacon_power } else { 0.0 };""")
rep("""        let pv = if w.power > 0.0 && (f.power_base > 0.0 || f.beacon_power > 0.0) {""", """        let pv = if !lin && w.power > 0.0 && (f.power_base > 0.0 || f.beacon_power > 0.0) {""")

a = t.index('    // 2. primal heuristic: MILP over the recipes the relaxation uses')
b = t.index('    plan.solve_ms = started.elapsed().as_secs_f64() * 1e3;\n    plan\n}')
new = r'''    let c_max = s.max_clock();
    let budget = s.time_limit_ms.clamp(200, 300_000);
    let lp_pt = point_of(&lp_model, &inp, &lp);
    let m = build(data, s, &inp, true);
    tr.vars = m.dump.vars.len();
    tr.rows = m.dump.rows.len();
    tr.int_vars = m.dump.vars.iter().filter(|v| v.3).count();
    if let Some(path) = &tn.dump {
        m.dump.write_lp(path);
    }
    let mut hint = hint_of(&m, &inp, &lp_pt, c_max, true);
    // objective of the ceil(LP) plan = first incumbent, available right after the LP
    tr.ceil_obj = hint.iter().map(|(v, x)| m.dump.vars[v.idx()].0 * x).sum();
    tr.t_ceil = started.elapsed().as_secs_f64() * 1e3;
    let mut support: BTreeSet<String> =
        lp_pt.rec.iter().filter(|(_, v)| v.0 > 1e-7).map(|(k, _)| k.clone()).collect();
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
    if tn.h1 {
        // neighbourhood: every mode of the recipes the LPs use
        let recs: BTreeSet<&str> = inp.units.iter().filter(|u| support.contains(&u.key)).map(|u| u.recipe.id.as_str()).collect();
        support.extend(inp.units.iter().filter(|u| recs.contains(u.recipe.id.as_str())).map(|u| u.key.clone()));
    }
    tr.t_support = started.elapsed().as_secs_f64() * 1e3;
    tr.support = support.len();
    // Local search over supports ("ban and re-solve"): the plan of a support is the ceil of its
    // LP, so a move is judged by one LP. Ban one used column at a time, keep the ban when the
    // rounded plan gets cheaper, restart; stop at a local optimum or after `ls` of the budget.
    let mut ls_pt: Option<Point> = None;
    tr.ls_obj = f64::NAN;
    if tn.ls > 0.0 {
        let cost = |h: &Vec<(Variable, f64)>| -> f64 { h.iter().map(|(v, x)| m.dump.vars[v.idx()].0 * x).sum() };
        let deadline = started.elapsed().as_secs_f64() * 1e3 + budget as f64 * tn.ls;
        let eval = |banned: &BTreeSet<String>| -> Option<(f64, Point)> {
            let keep: BTreeSet<String> = inp.units.iter().filter(|u| !banned.contains(&u.key)).map(|u| u.key.clone()).collect();
            let sub_inp = restrict(&inp, &keep);
            let lpm = build(data, s, &sub_inp, false);
            let sol = lpm.p.solve().ok()?.into_solution().ok()?;
            let pt = point_of(&lpm, &sub_inp, &sol);
            let h = hint_of(&m, &inp, &pt, c_max, true);
            Some((cost(&h), pt))
        };
        let mut banned: BTreeSet<String> = BTreeSet::new();
        let mut best = (tr.ceil_obj, Point { rec: lp_pt.rec.clone(), flows: lp_pt.flows.clone() });
        let mut improved = true;
        'outer: while improved {
            improved = false;
            // used columns, most rounding waste first
            let mut used: Vec<(f64, String)> = inp.units.iter().filter_map(|u| {
                let (x, _, _) = best.1.rec.get(&u.key).copied()?;
                if x <= 1e-7 { return None; }
                let cm = if u.mode.overclock { c_max } else { 1.0 };
                let nn = x / (cm * u.mode.speed);
                Some(((nn - 1e-9).ceil() - nn, u.key.clone()))
            }).collect();
            used.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            for (_, k) in used {
                if started.elapsed().as_secs_f64() * 1e3 > deadline { break 'outer; }
                let mut b2 = banned.clone();
                b2.insert(k);
                tr.ls_evals += 1;
                if let Some(r) = eval(&b2) {
                    if r.0 < best.0 - 1e-9 * best.0.abs() {
                        best = r;
                        banned = b2;
                        tr.ls_moves += 1;
                        improved = true;
                        continue 'outer;
                    }
                }
            }
        }
        tr.ls_obj = best.0;
        support.extend(best.1.rec.iter().filter(|(_, v)| v.0 > 1e-7).map(|(k, _)| k.clone()));
        hint = hint_of(&m, &inp, &best.1, c_max, true);
        ls_pt = Some(best.1);
    }
    tr.t_ls = started.elapsed().as_secs_f64() * 1e3;
    tr.sub_obj = f64::NAN;
    if support.len() < inp.units.len() && !tn.skip_sub {
        let sub_inp = restrict(&inp, &support);
        let sub = build(data, s, &sub_inp, true);
        let mut sub_hint = hint_of(&sub, &sub_inp, ls_pt.as_ref().unwrap_or(&lp_pt), c_max, true);
        if tn.hint_int {
            sub_hint.retain(|(v, _)| sub.dump.vars[v.idx()].3);
        }
        if let Ok(out) = sub.p.solve_with(milp_options((budget as f64 * tn.subfrac) as u64, sub_hint)) {
            tr.sub_nodes = out.stats().nodes_solved;
            if let Ok(sol) = out.into_solution() {
                tr.sub_obj = sol.objective();
                tr.sub_proven = sol.status() == SolutionStatus::Optimal || sol.termination_reason() == TerminationReason::MipGap;
                hint = hint_of(&m, &inp, &point_of(&sub, &sub_inp, &sol), c_max, false);
            }
        }
    }
    tr.t_sub = started.elapsed().as_secs_f64() * 1e3;
    let left = budget.saturating_sub(started.elapsed().as_millis() as u64);
    let first = if tn.slice_ms > 0 { tn.slice_ms.min(left.max(50)) } else { left };
    if tn.hint_int {
        hint.retain(|(v, _)| m.dump.vars[v.idx()].3);
    }
    let mut res = m.p.solve_with(milp_options(first, hint));
    if tn.slice_ms > 0 {
        loop {
            let el = started.elapsed().as_secs_f64() * 1e3;
            match res {
                Ok(out) => {
                    let st = out.stats();
                    let inc = out.solution().map(|s| s.objective()).unwrap_or(f64::NAN);
                    tr.traj.push((el, inc, st.best_bound.unwrap_or(f64::NAN), st.nodes_solved));
                    let done = !matches!(out.termination_reason(), TerminationReason::TimeLimit);
                    if done || el >= budget as f64 {
                        res = Ok(out);
                        break;
                    }
                    let mut ro = microlp::ResumeOptions::default();
                    ro.time_limit = Some(Duration::from_millis(tn.slice_ms.min((budget as f64 - el).max(20.0) as u64)));
                    ro.mip_gap = Some(tn.gap);
                    res = out.resume_with(ro);
                }
                Err(e) => { res = Err(e); break; }
            }
        }
    }
    match res {
        Ok(outcome) => {
            let st = outcome.stats();
            tr.nodes = st.nodes_solved;
            tr.lp_iters = st.lp_iterations;
            tr.bound = st.best_bound.unwrap_or(f64::NAN);
            match outcome.into_solution() {
            Ok(sol) => {
                plan.status = Status::Optimal;
                plan.objective = sol.objective();
                plan.proven_optimal = sol.status() == SolutionStatus::Optimal
                    || sol.termination_reason() == TerminationReason::MipGap;
                plan.gap = if sol.status() == SolutionStatus::Optimal { None } else { sol.gap() };
                tr.obj = sol.objective();
                tr.proven = plan.proven_optimal;
                finish(data, s, &inp, &m, &sol, true, &mut plan);
            }
            Err(_) => {
                plan.status = Status::Infeasible;
                plan.warnings.push("Time limit reached before a whole-building plan was found.".into());
            }
        }},
        Err(microlp::Error::Infeasible) => {
            plan.status = Status::Infeasible;
            plan.warnings.push("No whole-building plan fits (check the shard budget).".into());
        }
        Err(e) => {
            plan.status = Status::Infeasible;
            plan.warnings.push(format!("Solver error: {e}"));
        }
    }
    tr.t_end = started.elapsed().as_secs_f64() * 1e3;
    TRACE.with(|t| *t.borrow_mut() = tr);
'''
t = t[:a] + new + t[b:]

t += r'''

/// Own presolve A: drop a mode column dominated by another mode of the same recipe (same inputs
/// and products per unit of work, at least as fast, no dearer per building, no more power, holds
/// no more boost items). Any plan using the dominated column can move its work and buildings to
/// the dominating one at no higher cost.
fn drop_dominated<'a>(data: &GameData, s: &PlanSettings, units: Vec<Unit<'a>>, w: &CostWeights) -> Vec<Unit<'a>> {
    let same = |a: &Vec<(String, f64)>, b: &Vec<(String, f64)>| {
        let (ma, mb): (BTreeMap<&str, f64>, BTreeMap<&str, f64>) =
            (a.iter().map(|(i, q)| (i.as_str(), *q)).collect(), b.iter().map(|(i, q)| (i.as_str(), *q)).collect());
        ma.len() == mb.len() && ma.iter().all(|(k, v)| mb.get(k).is_some_and(|x| (x - v).abs() <= 1e-12 * (1.0 + v.abs())))
    };
    let wb = |u: &Unit| w.buildings * compute::machine_cost(w, u.recipe, &u.mode, s.mixed_belts) + rules::module_cost(data, s, &u.mode);
    let held_le = |a: &Mode, b: &Mode| a.held.iter().all(|(i, q)| b.held.iter().any(|(j, r)| i == j && q <= r));
    let dom = |j: &Unit, k: &Unit| {
        j.recipe.id == k.recipe.id
            && j.mode.overclock == k.mode.overclock
            && j.mode.power_exp == k.mode.power_exp
            && j.mode.speed >= k.mode.speed
            && wb(j) <= wb(k)
            && (w.power <= 0.0 || (j.mode.power_base <= k.mode.power_base && j.mode.beacon_power <= k.mode.beacon_power))
            && held_le(&j.mode, &k.mode)
            && same(&j.ins, &k.ins)
            && same(&j.outs, &k.outs)
    };
    let mut keep = vec![true; units.len()];
    let mut by_recipe: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, u) in units.iter().enumerate() {
        by_recipe.entry(u.recipe.id.as_str()).or_default().push(i);
    }
    for idx in by_recipe.values() {
        for &k in idx {
            for &j in idx {
                if j != k && keep[j] && dom(&units[j], &units[k]) && !(dom(&units[k], &units[j]) && k < j) {
                    keep[k] = false;
                    break;
                }
            }
        }
    }
    units.into_iter().zip(keep).filter(|(_, k)| *k).map(|(u, _)| u).collect()
}

/// Own presolve B: lower bounds on work by propagation from the targets through items with a
/// single producing column and no other source, turned into integer lower bounds on buildings.
fn implied_bounds(data: &GameData, s: &PlanSettings, inp: &Inputs) -> HashMap<String, (i32, i32)> {
    let c_max = s.max_clock();
    let n = inp.units.len();
    let mut lo = vec![0.0f64; n];
    let nets: Vec<BTreeMap<&str, f64>> = inp.units.iter().map(|u| {
        let mut net: BTreeMap<&str, f64> = BTreeMap::new();
        for (i, q) in &u.outs { *net.entry(i.as_str()).or_insert(0.0) += q; }
        for (i, q) in &u.ins { *net.entry(i.as_str()).or_insert(0.0) -= q; }
        net
    }).collect();
    let mut prod: HashMap<&str, Vec<(usize, f64)>> = HashMap::new();
    let mut cons: HashMap<&str, Vec<(usize, f64)>> = HashMap::new();
    for (k, net) in nets.iter().enumerate() {
        for (i, a) in net {
            if *a > 1e-12 { prod.entry(i).or_default().push((k, *a)); }
            if *a < -1e-12 { cons.entry(i).or_default().push((k, -*a)); }
        }
    }
    for _ in 0..200 {
        let mut changed = false;
        for it in &inp.items {
            let i = it.as_str();
            if data.resource(i).is_some_and(|r| data.located(&r.locations, &s.planets)) { continue; }
            let Some(ps) = prod.get(i) else { continue };
            if ps.len() != 1 { continue; }
            let sup = inp.supplies.get(i).copied().unwrap_or(0.0);
            let dem = inp.targets.get(i).copied().unwrap_or(0.0)
                + cons.get(i).map(|c| c.iter().map(|(k, a)| a * lo[*k]).sum::<f64>()).unwrap_or(0.0);
            let (k, a) = ps[0];
            let v = (dem - sup) / a;
            if v > lo[k] * (1.0 + 1e-9) + 1e-9 { lo[k] = v; changed = true; }
        }
        if !changed { break; }
    }
    inp.units.iter().zip(&lo).filter(|(_, l)| **l > 1e-9).map(|(u, l)| {
        let c = if u.mode.overclock { c_max } else { 1.0 };
        (u.key.clone(), (((l * (1.0 - 1e-9)) / (c * u.mode.speed) - 1e-9).ceil().max(0.0) as i32, MAX_BUILDINGS))
    }).collect()
}
'''
open(p, 'w', encoding='utf-8').write(t)
print('patched')
