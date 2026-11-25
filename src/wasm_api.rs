//! JSON-in / JSON-out API for the web front end.
use crate::planner::{self, GameData, PlanSettings};
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;

fn data(game: &str, dataset: &str) -> &'static GameData {
    static SF: OnceLock<GameData> = OnceLock::new();
    static DSP: OnceLock<GameData> = OnceLock::new();
    static FAC: OnceLock<GameData> = OnceLock::new();
    static F2X1: OnceLock<GameData> = OnceLock::new();
    static F2Q1: OnceLock<GameData> = OnceLock::new();
    static FSPA: OnceLock<GameData> = OnceLock::new();
    let cell = match (game, dataset) {
        ("dsp", _) => &DSP,
        ("factorio", "2x1") => &F2X1,
        ("factorio", "2q1") => &F2Q1,
        ("factorio", "spa") => &FSPA,
        ("factorio", _) => &FAC,
        _ => &SF,
    };
    cell.get_or_init(|| GameData::for_set(game, dataset))
}

/// The embedded game data (items, recipes, buildings, resources, effects) as JSON.
/// `dataset` picks a Factorio data set ("2.0", "2x1", "2q1", "spa"; "" = default).
#[wasm_bindgen]
pub fn game_data_json(game: &str, dataset: &str) -> String {
    planner::data::embedded_set(game, dataset).to_string()
}

/// DSP Compute (UPS) objective weights for a generator (`artificial_star`, `wind`, …), as a
/// `CostWeights` JSON; coefficients from docs/research/dsp-ups/model.json.
#[wasm_bindgen]
pub fn compute_weights_json(generator: &str) -> String {
    serde_json::to_string(&planner::compute::model().weights(generator)).unwrap()
}

/// Solve a plan. `settings_json` is a `PlanSettings` (its `game` picks the data); returns a
/// `Plan` as JSON.
#[wasm_bindgen]
pub fn solve_json(settings_json: &str) -> String {
    let settings: PlanSettings = match serde_json::from_str(settings_json) {
        Ok(s) => s,
        Err(e) => {
            return serde_json::json!({"status": "error", "warnings": [format!("bad settings: {e}")]})
                .to_string()
        }
    };
    serde_json::to_string(&planner::plan(data(&settings.game, &settings.dataset), &settings)).unwrap()
}

// ---- stepped whole-building solve (report §6 step 3): the worker calls `solve_start`, then
// `solve_step` until `done`, posting each update; the best plan is sent whenever it changes.

thread_local! {
    static RUN: std::cell::RefCell<Option<planner::lp::Run<'static>>> = const { std::cell::RefCell::new(None) };
}

thread_local! {
    /// The settings as the page sent them (before `self_powered_settings`), for `finish`.
    static ASKED: std::cell::RefCell<Option<PlanSettings>> = const { std::cell::RefCell::new(None) };
}

thread_local! {
    static SCALE: std::cell::Cell<Option<f64>> = const { std::cell::Cell::new(None) };
}

/// Notes on a plan (self-powered cover, nuclear waste, max-output scale), as `planner::plan` adds them.
fn finish(mut p: planner::Plan) -> planner::Plan {
    ASKED.with(|a| {
        if let Some(s) = a.borrow().as_ref() {
            planner::finish_power(data(&s.game, &s.dataset), s, &mut p);
            planner::finish_max_output(s, SCALE.with(|c| c.get()), &mut p);
        }
    });
    p
}

fn update(run: &planner::lp::Run, last_version: Option<u32>) -> String {
    let pr = run.progress();
    let plan = if last_version != Some(pr.version) {
        serde_json::to_value(finish(run.plan())).unwrap()
    } else {
        serde_json::Value::Null
    };
    serde_json::json!({
        "done": pr.done, "plan": plan, "version": pr.version, "elapsed_ms": pr.elapsed_ms,
        "objective": pr.objective, "bound": pr.bound, "gap": pr.gap, "proven": pr.proven,
        "phase": pr.phase, "nodes": pr.nodes, "deadline_ms": pr.deadline_ms,
    })
    .to_string()
}

thread_local! {
    static SENT: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

/// Start a solve. Returns `{done, plan, ...}`: a finished plan (LP, infeasible, too large, or
/// proven at once) has `done: true`; otherwise `plan` is the first whole-building plan and the
/// caller continues with `solve_step`.
#[wasm_bindgen]
pub fn solve_start(settings_json: &str) -> String {
    RUN.with(|r| *r.borrow_mut() = None);
    let settings: PlanSettings = match serde_json::from_str(settings_json) {
        Ok(s) => s,
        Err(e) => {
            let p = serde_json::json!({"status": "error", "warnings": [format!("bad settings: {e}")]});
            return serde_json::json!({"done": true, "plan": p}).to_string();
        }
    };
    planner::data::set_lang(&settings.lang);
    ASKED.with(|a| *a.borrow_mut() = Some(settings.clone()));
    // max output with budgets may need several whole-building solves at smaller scales (planner::plan
    // steps down): solve it in one go, without streaming
    if settings.max_output && (settings.somersloop_budget.is_some() || settings.shard_budget.is_some()) {
        let p = planner::plan(data(&settings.game, &settings.dataset), &settings);
        return serde_json::json!({"done": true, "plan": p}).to_string();
    }
    let (settings, scale) = planner::prepared_settings(data(&settings.game, &settings.dataset), &settings);
    SCALE.with(|c| c.set(scale));
    match planner::lp::start(data(&settings.game, &settings.dataset), &settings) {
        planner::lp::Start::Done(p) => serde_json::json!({"done": true, "plan": finish(p)}).to_string(),
        planner::lp::Start::Running(run) => {
            let out = update(&run, None);
            SENT.with(|s| s.set(Some(run.progress().version)));
            RUN.with(|r| *r.borrow_mut() = Some(*run));
            out
        }
    }
}

/// Continue the current solve for about `slice_ms`. Returns an update (`plan` null when unchanged;
/// the final update always carries the plan).
#[wasm_bindgen]
pub fn solve_step(slice_ms: f64) -> String {
    RUN.with(|r| {
        let mut r = r.borrow_mut();
        let Some(run) = r.as_mut() else { return serde_json::json!({"done": true, "gone": true}).to_string() };
        let done = run.step(Some(slice_ms));
        let last = SENT.with(|s| s.get());
        let out = if done { update(run, None) } else { update(run, last) };
        SENT.with(|s| s.set(Some(run.progress().version)));
        out
    })
}

/// Give the current solve `ms` more ("keep solving"). Returns false when there is none.
#[wasm_bindgen]
pub fn solve_extend(ms: f64) -> bool {
    RUN.with(|r| match r.borrow_mut().as_mut() {
        Some(run) => {
            run.extend(ms);
            true
        }
        None => false,
    })
}

// ---- external MILP engine (HiGHS in the worker, report §6 step 5)

/// The current solve's whole-building model (after the exact reductions) as CPLEX LP text,
/// columns `v0, v1, …`; empty when there is none.
#[wasm_bindgen]
pub fn solve_model_lp() -> String {
    RUN.with(|r| r.borrow().as_ref().map(|run| run.model_lp()).unwrap_or_default())
}

/// The relative MIP gap an external engine should stop at (lp.rs `Run::engine_gap`: one building
/// where buildings are a main cost, else MIP_GAP).
#[wasm_bindgen]
pub fn solve_engine_gap() -> f64 {
    RUN.with(|r| r.borrow().as_ref().map(|run| run.engine_gap()).unwrap_or(1e-4))
}

/// The best plan in hand as one value per model column (empty without one): the engine's start.
#[wasm_bindgen]
pub fn solve_best_values() -> Vec<f64> {
    RUN.with(|r| r.borrow().as_ref().map(|run| run.best_values()).unwrap_or_default())
}

/// True once the heuristic phases (re-weighted LPs, restricted MILP) are done: the full search
/// can go to an external engine.
#[wasm_bindgen]
pub fn solve_heuristics_done() -> bool {
    RUN.with(|r| r.borrow().as_ref().is_none_or(|run| run.heuristics_done()))
}

/// An engine solution (values per model column; may be empty) with its bound (NaN = none);
/// `proven` ends the solve as optimal. Returns an update like `solve_step`.
#[wasm_bindgen]
pub fn solve_offer(values: Vec<f64>, bound: f64, proven: bool) -> String {
    RUN.with(|r| {
        let mut r = r.borrow_mut();
        let Some(run) = r.as_mut() else { return serde_json::json!({"done": true, "gone": true}).to_string() };
        run.offer_values(&values, bound.is_finite().then_some(bound), proven);
        let last = SENT.with(|s| s.get());
        let done = run.progress().done;
        let out = if done { update(run, None) } else { update(run, last) };
        SENT.with(|s| s.set(Some(run.progress().version)));
        out
    })
}

/// Ends the current solve with the plan in hand (the engine used up the time). Final update.
#[wasm_bindgen]
pub fn solve_finish() -> String {
    RUN.with(|r| {
        let mut r = r.borrow_mut();
        let Some(run) = r.as_mut() else { return serde_json::json!({"done": true, "gone": true}).to_string() };
        run.finish_now();
        SENT.with(|s| s.set(Some(run.progress().version)));
        update(run, None)
    })
}

// ---- DSP mixed-belt layout (docs/research/dsp-mixed-belt/00-最终报告.md §4 P2)

/// Mixed-belt (混带) layout of a solved plan. `settings_json` is the `PlanSettings` the plan was
/// solved with (its `belt_layout` holds the tab's settings); `plan_json` the `Plan`. Separate from
/// the solve, so changing belt settings does not re-solve. Returns a `MixedBeltReport` as JSON.
#[wasm_bindgen]
pub fn mixed_belt_json(settings_json: &str, plan_json: &str) -> String {
    let settings: PlanSettings = match serde_json::from_str(settings_json) {
        Ok(s) => s,
        Err(e) => return serde_json::json!({"error": format!("bad settings: {e}")}).to_string(),
    };
    let plan: planner::Plan = match serde_json::from_str(plan_json) {
        Ok(p) => p,
        Err(e) => return serde_json::json!({"error": format!("bad plan: {e}")}).to_string(),
    };
    let rep = planner::mixed_belt::layout_for_plan(&plan, data(&settings.game, &settings.dataset), &settings);
    serde_json::to_string(&rep).unwrap()
}

// ---- DSP seed planning (docs/research/dsp-seed/00-最终报告.md §9 steps 1–4, 7)

thread_local! {
    static GALAXY: std::cell::RefCell<Option<std::rc::Rc<planner::dsp_galaxy::Galaxy>>> = const { std::cell::RefCell::new(None) };
}

/// The galaxy of a seed, generated once per `SeedSettings` (the generator takes ~0.1–1 s).
fn galaxy_for(s: &planner::dsp_galaxy::SeedSettings) -> std::rc::Rc<planner::dsp_galaxy::Galaxy> {
    // compared in the form the generator runs with (`Galaxy::settings` is sanitized); the cell is not
    // borrowed while the generator runs, so a trap inside it cannot leave the cache locked
    let s = s.sanitized();
    if let Some(cur) = GALAXY.with(|g| g.borrow().clone()) {
        if cur.settings == s {
            return cur;
        }
    }
    let new = std::rc::Rc::new(planner::dsp_galaxy::generate(&s));
    GALAXY.with(|g| *g.borrow_mut() = Some(new.clone()));
    new
}

fn bad(e: impl std::fmt::Display) -> String {
    serde_json::json!({"error": format!("bad settings: {e}")}).to_string()
}

/// Seed import: `SeedSettings` JSON → `Galaxy` JSON (stars, planets, veins, gas giants, totals).
#[wasm_bindgen]
pub fn dsp_galaxy_json(seed_json: &str) -> String {
    match serde_json::from_str::<planner::dsp_galaxy::SeedSettings>(seed_json) {
        Ok(s) => {
            let g = galaxy_for(&s);
            let mut v = serde_json::to_value(&*g).unwrap();
            v["auto_systems"] = serde_json::json!(planner::dsp_galaxy::auto_systems(&g));
            v.to_string()
        }
        Err(e) => bad(e),
    }
}

/// Threshold button: `ThresholdSettings` JSON → `ThresholdReport` JSON.
#[wasm_bindgen]
pub fn dsp_thresholds_json(settings_json: &str) -> String {
    match serde_json::from_str::<planner::dsp_galaxy::ThresholdSettings>(settings_json) {
        Ok(ts) => {
            planner::data::set_lang(&ts.plan.lang);
            let g = galaxy_for(&ts.seed);
            serde_json::to_string(&planner::dsp_galaxy::thresholds(data("dsp", ""), &g, &ts)).unwrap()
        }
        Err(e) => bad(e),
    }
}

/// Multi-system LP: `GalaxyPlanSettings` JSON → `GalaxyPlan` JSON.
#[wasm_bindgen]
pub fn dsp_galaxy_plan_json(settings_json: &str) -> String {
    match serde_json::from_str::<planner::dsp_galaxy::GalaxyPlanSettings>(settings_json) {
        Ok(gs) => {
            planner::data::set_lang(&gs.plan.lang);
            let g = galaxy_for(&gs.seed);
            serde_json::to_string(&planner::dsp_galaxy::plan_galaxy(data("dsp", ""), &g, &gs)).unwrap()
        }
        Err(e) => bad(e),
    }
}

/// Satisfactory speedrun: `SpeedrunSettings` JSON → `SpeedrunReport` JSON (memo step 17).
#[wasm_bindgen]
pub fn speedrun_json(settings_json: &str) -> String {
    match serde_json::from_str::<planner::speedrun::SpeedrunSettings>(settings_json) {
        Ok(ss) => serde_json::to_string(&planner::speedrun::speedrun(data("satisfactory", ""), &ss)).unwrap(),
        Err(e) => bad(e),
    }
}

/// Highest cube rate: `RateSettings` JSON → `RateReport` JSON.
#[wasm_bindgen]
pub fn dsp_galaxy_rate_json(settings_json: &str) -> String {
    match serde_json::from_str::<planner::dsp_galaxy::RateSettings>(settings_json) {
        Ok(rs) => {
            planner::data::set_lang(&rs.plan.lang);
            let g = galaxy_for(&rs.seed);
            serde_json::to_string(&planner::dsp_galaxy::galaxy_rate(data("dsp", ""), &g, &rs)).unwrap()
        }
        Err(e) => bad(e),
    }
}

/// Galaxy total: `TotalSettings` JSON → `TotalReport` JSON (most cubes the finite stock makes).
#[wasm_bindgen]
pub fn dsp_galaxy_total_json(settings_json: &str) -> String {
    match serde_json::from_str::<planner::dsp_galaxy::TotalSettings>(settings_json) {
        Ok(ts) => {
            planner::data::set_lang(&ts.plan.lang);
            let g = galaxy_for(&ts.seed);
            serde_json::to_string(&planner::dsp_galaxy::galaxy_total(data("dsp", ""), &g, &ts)).unwrap()
        }
        Err(e) => bad(e),
    }
}
