//! DSP mixed-belt (混带) layout planner, `docs/research/dsp-mixed-belt/00-最终报告.md` §2.
//!
//! Works on a solved plan: for every DSP recipe with two or more ingredients it lays out one
//! mixed belt per line, in the order of things along the belt (1D, no geometry):
//!
//! * two ingredients (scheme A, §2.3–2.4): a control unit (笑笑's 强制摇匀) and direct refills
//!   (直怼补料) along the line. Steady state: with per-machine use a, b (stacks/s), belt capacity C
//!   and period P, the two minima sum to S = C − P(a² + ab + b²)/(a + b). The period, the control
//!   unit's share σ (fifths), the first refilled item and its position k are searched; the largest
//!   feasible period is taken (within the repeat-unit cap when a row length is given); the return
//!   flow at the control unit picks the standard or the two-green-belt unit.
//! * three or more (scheme B, §2.5): fixed-rate ladder groups (梯口), feeds from one rate-limited
//!   belt plus up to two sorters; groups longer than 12 get the 1/12 surplus.
//! * a 1D flow simulator (§2.6) checks both and gives the belt diagram.
//!
//! Units on the belt are stacks/s (叠/秒): items/s ÷ stack height. Ingredients come from the game
//! data's recipe, never from `PlanRecipe.inputs` (which also holds proliferator and fuel, §2.1).
//! The model is a statistical average-flow model; margins and the ladder surplus are fitted to
//! 笑笑's lines (MINE) and need sandbox calibration (report §4 P5).
use super::data::{GameData, Recipe};
use super::rules::{self, Mode};
use super::{Plan, PlanRecipe, PlanSettings};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const EPS: f64 = 1e-9;
/// Belt speeds in cells/s: yellow, green, blue (wiki Conveyor Belts; dsp.json 360/720/1800 per min).
pub const BELTS: [f64; 3] = [6.0, 12.0, 30.0];
/// One green belt (cells/s): the standard control unit's return limit (§2.4).
pub const GREEN_BELT: f64 = 12.0;
/// Sorter trips/s by tier (yellow, green, blue) and span (1, 2, 3 grids) (wiki Sorters). A normal
/// sorter takes a whole stack from a belt, so its rate in stacks/s equals its trips/s.
pub const SORTERS: [[f64; 3]; 3] = [[1.5, 0.75, 0.5], [3.0, 1.5, 1.0], [6.0, 3.0, 2.0]];
pub const SORTER_NAMES: [&str; 3] = ["yellow", "green", "blue"];
/// Pile sorter items/s over 1 grid by upgrade level: Lv0 800/min, Lv5 1600/min, max 7200/min
/// (wiki Pile Sorter table).
pub const PILE: [(&str, f64); 3] = [("Lv0", 800.0 / 60.0), ("Lv5", 1600.0 / 60.0), ("max", 120.0)];

// ---------------------------------------------------------------------------------------------
// settings

/// Manual choice for one two-ingredient recipe (report §3 手动覆盖); unset fields are searched.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TwoOverride {
    pub period: Option<u32>,
    /// 0 = refill the first ingredient first, 1 = the second.
    pub first: Option<u8>,
    pub k: Option<u32>,
    /// Control-unit share of the first ingredient in fifths (1..=4).
    pub sigma5: Option<u8>,
}

/// Mixed-belt tab settings (`PlanSettings::belt_layout`). Not part of the solve: changing them
/// re-runs only the layout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BeltLayoutSettings {
    /// Mixed belt tier: 0 yellow, 1 green, 2 blue.
    pub belt: u8,
    /// Stack height on the belt (1..=4); 笑笑's ep19 lines are all 4 [BV1fXkxYDEbW 26:15].
    pub stack: u8,
    /// Machine rows fed by one belt (1 or 2). Two rows: refills only every 2 machines [77:08].
    pub rows: u8,
    /// Machines per row R (optional): the repeat unit lcm(P, R) must stay within `unit_max`.
    pub row_len: Option<u32>,
    /// Repeat-unit cap; default 4R (MINE: he took 60 and refused 120 at R = 15).
    pub unit_max: Option<u32>,
    /// Highest normal sorter tier available (0 yellow, 1 green, 2 blue).
    pub sorter_max: u8,
    /// Pile sorter upgrade (index into `PILE`).
    pub pile_level: u8,
    /// Margin m: lowest flow of either ingredient anywhere on a scheme-A line (stacks/s).
    /// Default 1.5 (MINE: the lowest value he accepted).
    pub margin: f64,
    /// Ladder surplus ρ for groups longer than `ladder_short` (MINE, default 1/12).
    pub ladder_rho: f64,
    pub ladder_short: u32,
    /// Longest ladder group (MINE, default 24: his longest).
    pub ladder_cap: u32,
    /// Split a scheme-A line into control units of at most this many machines (None = hint only).
    pub unit_cap: Option<u32>,
    /// Restart hint: input below this (items/s per machine) and more than `hint_machines` on one unit.
    pub hint_items: f64,
    pub hint_machines: u32,
    /// Last machines with a pile-sorter input: before the control unit (A) / the recovery (B).
    pub end_pile_a: u32,
    pub end_pile_b: u32,
    /// Per "recipe|mode".
    pub overrides: HashMap<String, TwoOverride>,
}

impl Default for BeltLayoutSettings {
    fn default() -> Self {
        Self {
            belt: 2,
            stack: 4,
            rows: 1,
            row_len: None,
            unit_max: None,
            sorter_max: 2,
            pile_level: 0,
            margin: 1.5,
            ladder_rho: 1.0 / 12.0,
            ladder_short: 12,
            ladder_cap: 24,
            unit_cap: None,
            hint_items: 12.0,
            hint_machines: 60,
            end_pile_a: 2,
            end_pile_b: 3,
            overrides: HashMap::new(),
        }
    }
}

impl BeltLayoutSettings {
    pub fn capacity(&self) -> f64 {
        BELTS[(self.belt as usize).min(2)]
    }
    pub fn stack(&self) -> f64 {
        self.stack.clamp(1, 4) as f64
    }
    pub fn step(&self) -> u32 {
        if self.rows >= 2 {
            2
        } else {
            1
        }
    }
    fn margin(&self) -> f64 {
        if self.margin.is_finite() && self.margin >= 0.0 {
            self.margin
        } else {
            1.5
        }
    }
}

// ---------------------------------------------------------------------------------------------
// per-machine rates (§2.1)

/// Per machine at full speed, items/s: recipe ingredients (game data, not `PlanRecipe.inputs`)
/// and products (× out_mult), using the mode's speed.
pub fn per_machine(r: &Recipe, m: &Mode) -> (Vec<(String, f64)>, Vec<(String, f64)>) {
    let ins = r.inputs.iter().map(|(i, q)| (i.clone(), q * m.speed / 60.0)).collect();
    let outs = rules::outputs(r, m).into_iter().map(|(i, q)| (i, q * m.speed / 60.0)).collect();
    (ins, outs)
}

/// The plan recipe's mode, rebuilt from its key (§2.1).
pub fn find_mode(data: &GameData, s: &PlanSettings, r: &Recipe, key: &str) -> Option<Mode> {
    rules::modes(data, r, s).into_iter().find(|m| m.key == key)
}

// ---------------------------------------------------------------------------------------------
// 1D flow simulator (§2.6)

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// A machine takes `use_[i]` of every item.
    Machine { use_: Vec<f64> },
    /// Direct refill (直怼): fills the free space, at most `cap`.
    FillFeed { item: usize, cap: f64 },
    /// Ladder feed: adds min(rate, free space).
    FixedFeed { item: usize, rate: f64 },
    /// Filtered recovery sorter: takes min(flow, cap).
    Recover { item: usize, cap: f64 },
    /// Control unit: the belt leaves it full at `shares[i]` (stacks/s, Σ = C); `jump` is the item
    /// the pile sorter lifts over the limiter (the other one goes through the limiter lane).
    ControlUnit { shares: Vec<f64>, jump: usize },
    /// Spray coater: bookkeeping only.
    Coater,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TracePoint {
    /// Machines passed so far.
    pub pos: u32,
    pub kind: String,
    /// Item refilled / recovered (index), if any.
    pub item: Option<usize>,
    /// Amount added (feeds) or taken (recovery), stacks/s.
    pub amount: f64,
    /// Flows after this event, per item (stacks/s).
    pub flows: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Trace {
    pub capacity: f64,
    pub points: Vec<TracePoint>,
    /// Lowest flow per item (after any machine, and at the start).
    pub mins: Vec<f64>,
    /// Flows arriving back at the start of the ring (control unit / feed block).
    pub returns: Vec<f64>,
    /// Ring passes until the returns settled.
    pub passes: u32,
    pub converged: bool,
    /// Rule violations found by the simulator (blocking at the control unit, overfull belt).
    pub violations: Vec<String>,
}

/// Runs `events` once from `start` flows. Returns the trace of that pass.
fn pass(events: &[Event], capacity: f64, start: &[f64], ret: &[f64]) -> Trace {
    let mut f: Vec<f64> = start.to_vec();
    let mut mins = f.clone();
    let mut points = vec![];
    let mut violations = vec![];
    let mut pos = 0u32;
    for e in events {
        let (kind, item, amount) = match e {
            Event::Machine { use_ } => {
                pos += 1;
                for (i, u) in use_.iter().enumerate() {
                    f[i] -= u;
                    mins[i] = mins[i].min(f[i]);
                }
                ("machine", None, 0.0)
            }
            Event::FillFeed { item, cap } => {
                let free = (capacity - f.iter().sum::<f64>()).max(0.0);
                let add = free.min(*cap);
                f[*item] += add;
                ("fill", Some(*item), add)
            }
            Event::FixedFeed { item, rate } => {
                let free = (capacity - f.iter().sum::<f64>()).max(0.0);
                let add = free.min(*rate);
                if add < rate - EPS {
                    violations.push(format!("feed of item {item} limited to {add:.3} by a full belt"));
                }
                f[*item] += add;
                ("feed", Some(*item), add)
            }
            Event::Recover { item, cap } => {
                let take = f[*item].max(0.0).min(*cap);
                f[*item] -= take;
                ("recover", Some(*item), take)
            }
            Event::ControlUnit { shares, jump } => {
                // returns ret[i] arrive; each item may come back at most at its share, else it backs
                // up into the unit and upsets the ratio [BV1iQrKYoEoy 27:50–28:15]
                for (i, s) in shares.iter().enumerate() {
                    if ret.get(i).copied().unwrap_or(0.0) > s + EPS {
                        violations.push(format!("return of item {i} {:.3} > its share {:.3}: it backs up into the control unit", ret[i], s));
                    }
                }
                let _ = jump;
                for (i, s) in shares.iter().enumerate() {
                    f[i] = *s;
                    mins[i] = mins[i].min(f[i]);
                }
                ("unit", None, 0.0)
            }
            Event::Coater => ("coater", None, 0.0),
        };
        if f.iter().sum::<f64>() > capacity + 1e-6 {
            violations.push(format!("belt over capacity after machine {pos}"));
        }
        points.push(TracePoint { pos, kind: kind.into(), item, amount, flows: f.clone() });
    }
    Trace { capacity, returns: f.clone(), mins, points, passes: 1, converged: true, violations }
}

/// Ring simulation: the flows at the end of the events come back to the start. Iterates until the
/// return vector settles (< 1e-9) or `max_passes`. A line whose events start with a `ControlUnit`
/// converges in two passes; a ladder ring starts from an empty belt.
pub fn simulate(events: &[Event], capacity: f64, items: usize) -> Trace {
    let mut ret = vec![0.0; items];
    let mut last: Option<Trace> = None;
    for p in 1..=50 {
        let t = pass(events, capacity, &ret, &ret);
        let settled = last.as_ref().is_some_and(|l| l.returns.iter().zip(&t.returns).all(|(a, b)| (a - b).abs() < EPS));
        ret = t.returns.clone();
        let mut t = t;
        t.passes = p;
        if settled {
            // the control-unit check needs the settled returns, which the last pass used
            return t;
        }
        last = Some(t);
    }
    let mut t = last.unwrap_or_default();
    t.converged = false;
    t.violations.push("returns did not settle (flow keeps growing or shrinking)".into());
    t
}

// ---------------------------------------------------------------------------------------------
// scheme A: two ingredients (§2.3–2.4)

/// S(P) = C − P(a² + ab + b²)/(a + b): sum of the two steady-state minima.
pub fn closed_sum(capacity: f64, period: f64, a: f64, b: f64) -> f64 {
    capacity - period * (a * a + a * b + b * b) / (a + b)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Feed {
    /// After this many machines.
    pub pos: u32,
    pub item: usize,
    pub amount: f64,
}

/// One run of the two-item line (prototype `simulate`): control-unit output `start`, first refill
/// of `first` after `k` machines, then the other item after its interval (`n[item]` machines between
/// the previous refill and this one), for `machines` machines.
pub fn run_two(capacity: f64, c: [f64; 2], start: [f64; 2], first: usize, k: u32, n: [u32; 2], machines: u32) -> ([f64; 2], Vec<Feed>, [f64; 2]) {
    let mut f = start;
    let mut mins = f;
    let mut feeds = vec![];
    let (mut item, mut at) = (first, k);
    for m in 1..=machines {
        for i in 0..2 {
            f[i] -= c[i];
            mins[i] = mins[i].min(f[i]);
        }
        if m == at {
            let free = capacity - f[0] - f[1];
            f[item] += free;
            feeds.push(Feed { pos: m, item, amount: free });
            item = 1 - item;
            at = m + n[item].max(1);
        }
    }
    (mins, feeds, f)
}

/// Fast minimum of `run_two` (no allocation), for the search.
fn min_two(capacity: f64, c: [f64; 2], start: [f64; 2], first: usize, k: u32, n: [u32; 2], machines: u32) -> [f64; 2] {
    let mut f = start;
    let mut mins = f;
    let (mut item, mut at) = (first, k);
    for m in 1..=machines {
        f[0] -= c[0];
        f[1] -= c[1];
        if f[0] < mins[0] {
            mins[0] = f[0];
        }
        if f[1] < mins[1] {
            mins[1] = f[1];
        }
        if m == at {
            f[item] = capacity - f[1 - item];
            item = 1 - item;
            at = m + n[item].max(1);
        }
    }
    mins
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Best {
    /// min(min_A, min_B) over the start, steady state (k + 2P machines).
    pub min: f64,
    pub mins: [f64; 2],
    pub sigma5: u8,
    pub first: u8,
    pub k: u32,
}

/// Prototype `best_for`: best achievable min over (σ, first item, k) for intervals `n`
/// (n[0] machines before an A refill, n[1] before a B refill). `step` = 2 when one belt feeds two
/// rows (positions only every 2 machines). Fixed parts of `ov` are not searched.
pub fn best_for(capacity: f64, c: [f64; 2], n: [u32; 2], step: u32, ov: &TwoOverride) -> Option<Best> {
    let period = n[0] + n[1];
    if period == 0 || n[0] % step != 0 || n[1] % step != 0 {
        return None;
    }
    let mut best: Option<Best> = None;
    let sigmas: Vec<u8> = match ov.sigma5 {
        Some(s) if (1..=4).contains(&s) => vec![s],
        _ => vec![1, 2, 3, 4],
    };
    let firsts: Vec<u8> = match ov.first {
        Some(f) if f < 2 => vec![f],
        _ => vec![0, 1],
    };
    let ks: Vec<u32> = match ov.k {
        Some(k) if k >= 1 => vec![k],
        _ => (step..=period).step_by(step as usize).collect(),
    };
    for &s5 in &sigmas {
        let start = [capacity * s5 as f64 / 5.0, capacity * (5 - s5) as f64 / 5.0];
        for &first in &firsts {
            for &k in &ks {
                let mins = min_two(capacity, c, start, first as usize, k, n, k + 2 * period);
                let v = mins[0].min(mins[1]);
                let better = match &best {
                    None => true,
                    Some(b) => v > b.min + EPS || (v > b.min - EPS && (mins[0] - mins[1]).abs() < (b.mins[0] - b.mins[1]).abs() - EPS),
                };
                if better {
                    best = Some(Best { min: v, mins, sigma5: s5, first, k });
                }
            }
        }
    }
    best
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}
fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        0
    } else {
        a / gcd(a, b) * b
    }
}

/// a : b as a small integer ratio p : q. Exact when some q ≤ 1000 gives it (DSP recipes have
/// integer amounts, so always in practice); otherwise the closest p/q with p + q ≤ 24 and its
/// relative deviation.
pub fn ratio(a: f64, b: f64) -> (u32, u32, f64) {
    for q in 1..=1000u32 {
        let p = (a / b * q as f64).round();
        if p >= 1.0 && (p * b - a * q as f64).abs() <= 1e-9 * a.max(b) * q as f64 {
            let g = gcd(p as u64, q as u64) as u32;
            return (p as u32 / g, q / g, 0.0);
        }
    }
    let mut best = (1, 1, f64::INFINITY);
    for s in 2..=24u32 {
        for p in 1..s {
            let q = s - p;
            let dev = ((p as f64 / q as f64) / (a / b) - 1.0).abs();
            if dev < best.2 - EPS {
                best = (p, q, dev);
            }
        }
    }
    best
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Candidate {
    pub period: u32,
    pub n: [u32; 2],
    /// Closed-form S(P).
    pub sum: f64,
    /// Repeat unit lcm(P, R) (0 without R).
    pub unit: u32,
    /// Best min over (σ, first, k); None = not evaluated (a larger period was already feasible)
    /// or not possible (odd positions with two rows).
    pub best: Option<f64>,
    pub feasible: bool,
    /// Why not: "S < 2m", "repeat unit too large", "positions", "min < m".
    pub reason: String,
}

/// Candidate periods P = t(p + q) with P·max(a, b) ≤ C and S(P) ≥ 2m (two rows: both intervals
/// even), and the chosen one: the largest feasible P (report §2.3 step 4; fitted to his 6/32/30/20).
pub fn choose_period(capacity: f64, c: [f64; 2], margin: f64, step: u32, row_len: Option<u32>, unit_max: Option<u32>) -> (u32, u32, Vec<Candidate>, Option<(usize, Best)>) {
    let (p, q, _) = ratio(c[0], c[1]);
    let mut cands = vec![];
    for t in 1..=4000u32 {
        let period = t * (p + q);
        if period as f64 * c[0].max(c[1]) > capacity + EPS || period > 4000 {
            break; // one refill point carries at most one belt [BV1fXkxYDEbW 39:00]
        }
        let sum = closed_sum(capacity, period as f64, c[0], c[1]);
        if sum < 2.0 * margin - EPS {
            break; // S falls with P
        }
        let n = [t * p, t * q];
        let unit = row_len.map(|r| lcm(period as u64, r as u64) as u32).unwrap_or(0);
        let mut cand = Candidate { period, n, sum, unit, ..Default::default() };
        if n[0] % step != 0 || n[1] % step != 0 {
            cand.reason = "positions".into();
        } else if let Some(r) = row_len.filter(|&r| r > 0) {
            if unit > unit_max.unwrap_or(4 * r) {
                cand.reason = "repeat unit too large".into();
            }
        }
        cands.push(cand);
    }
    let mut chosen = None;
    for i in (0..cands.len()).rev() {
        if !cands[i].reason.is_empty() {
            continue;
        }
        let Some(b) = best_for(capacity, c, cands[i].n, step, &TwoOverride::default()) else {
            cands[i].reason = "positions".into();
            continue;
        };
        cands[i].best = Some(b.min);
        if b.min >= margin - EPS {
            cands[i].feasible = true;
            chosen = Some((i, b));
            break;
        }
        cands[i].reason = "min < m".into();
    }
    (p, q, cands, chosen)
}

/// Standard / two-green-belt control unit from the return flows (§2.4).
pub fn unit_kind(ret: [f64; 2], jump: usize) -> (&'static str, Vec<String>) {
    let total = ret[0] + ret[1];
    if total <= GREEN_BELT + EPS {
        return ("standard", vec![]);
    }
    let other = 1 - jump;
    let mut why = vec![];
    if ret[jump] > 2.0 * GREEN_BELT + EPS {
        why.push(format!("jumping item returns {:.3} > 24 (two green belts)", ret[jump]));
    }
    if ret[other] > GREEN_BELT + EPS {
        why.push(format!("limited item returns {:.3} > 12 (one green belt)", ret[other]));
    }
    if why.is_empty() {
        ("double", vec![])
    } else {
        ("unknown", why)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TwoItemDesign {
    /// Per machine, stacks/s (the first and second ingredient of the recipe).
    pub c: [f64; 2],
    pub p: u32,
    pub q: u32,
    /// Ratio deviation when a : b is not a small integer ratio (0 = exact).
    pub ratio_dev: f64,
    pub period: u32,
    /// Machines before a refill of item 0 / item 1.
    pub n: [u32; 2],
    pub sum: f64,
    pub sigma5: u8,
    pub first: u8,
    pub k: u32,
    /// Steady-state best (k + 2P machines) and its two minima.
    pub best: f64,
    pub best_mins: [f64; 2],
    /// Over the actual line (machines per unit).
    pub line_mins: [f64; 2],
    pub candidates: Vec<Candidate>,
    /// The searched recommendation (differs from the shown design when overridden).
    pub recommended: Option<(u32, u8, u8, u32, f64)>,
    pub overridden: bool,
    pub feeds: Vec<Feed>,
    /// Flows arriving back at the control unit.
    pub returns: [f64; 2],
    /// Control-unit shares (stacks/s).
    pub shares: [f64; 2],
    /// Item the pile sorter lifts over the limiter (the larger share), and the limited one's lane.
    pub jump: usize,
    pub limiter_lane: String,
    /// "standard" | "double" | "unknown"
    pub unit: String,
    /// Machines to add so the line ends on a full period (0 = it does).
    pub complete_add: u32,
    /// Belts from the tower per item (first-fit decreasing of the refill amounts, §2.9).
    pub tower_belts: [u32; 2],
    pub units: u32,
    pub machines_per_unit: u32,
}

/// Among the layouts as good as `best` (same steady-state min), the one that suits this line of
/// `machines`: return flows within the control unit's shares and a known unit first, then equal
/// minima (his preference [BV1fXkxYDEbW 34:05]), then the smaller total return (less through the
/// unit). This order is MINE; it only breaks ties of the report's search.
fn refine(capacity: f64, c: [f64; 2], n: [u32; 2], step: u32, ov: &TwoOverride, best: Best, machines: u32) -> Best {
    let period = n[0] + n[1];
    let score = |b: &Best| {
        let shares = [capacity * b.sigma5 as f64 / 5.0, capacity * (5 - b.sigma5) as f64 / 5.0];
        let (_, _, r) = run_two(capacity, c, shares, b.first as usize, b.k, n, machines);
        let jump = if b.sigma5 >= 3 { 0 } else { 1 };
        let ok = r[0] <= shares[0] + EPS && r[1] <= shares[1] + EPS && unit_kind(r, jump).0 != "unknown";
        (if ok { 0 } else { 1 }, (b.mins[0] - b.mins[1]).abs(), r[0] + r[1])
    };
    let mut pick = (score(&best), best);
    for s5 in 1..=4u8 {
        if ov.sigma5.is_some_and(|x| x != s5) {
            continue;
        }
        let start = [capacity * s5 as f64 / 5.0, capacity * (5 - s5) as f64 / 5.0];
        for first in 0..2u8 {
            if ov.first.is_some_and(|x| x != first) {
                continue;
            }
            for k in (step..=period).step_by(step as usize) {
                if ov.k.is_some_and(|x| x != k) {
                    continue;
                }
                let mins = min_two(capacity, c, start, first as usize, k, n, k + 2 * period);
                let v = mins[0].min(mins[1]);
                if v < best.min - EPS {
                    continue;
                }
                let b = Best { min: v, mins, sigma5: s5, first, k };
                let sc = score(&b);
                let better = sc.0 < pick.0 .0
                    || (sc.0 == pick.0 .0 && (sc.1 < pick.0 .1 - EPS || ((sc.1 - pick.0 .1).abs() <= EPS && sc.2 < pick.0 .2 - EPS)));
                if better {
                    pick = (sc, b);
                }
            }
        }
    }
    pick.1
}

/// Lays out one two-ingredient line of `machines` machines (§2.3 design algorithm, §2.4 checks).
/// Errors are reasons the scheme does not apply.
pub fn design_two(capacity: f64, c: [f64; 2], names: [&str; 2], machines: u32, ls: &BeltLayoutSettings, ov: &TwoOverride) -> Result<(TwoItemDesign, Vec<Note>), String> {
    let mut notes = vec![];
    let margin = ls.margin();
    let step = ls.step();
    if c[0] <= 0.0 || c[1] <= 0.0 {
        return Err("an ingredient has zero use".into());
    }
    if c[0] + c[1] >= capacity - EPS {
        return Err(format!("one machine uses {:.3} stacks/s, a whole belt ({capacity})", c[0] + c[1]));
    }
    let (p, q, ratio_dev) = ratio(c[0], c[1]);
    if ratio_dev > 0.0 {
        notes.push(Note::warn(format!("use ratio is not a small integer ratio; using {p}:{q} ({:.1} % off), flows drift", 100.0 * ratio_dev), ""));
    }
    let (_, _, mut cands, chosen) = choose_period(capacity, c, margin, step, ls.row_len, ls.unit_max);
    let recommended = chosen.map(|(i, b)| (cands[i].period, b.sigma5, b.first, b.k, b.min));
    // the period shown: an override (a multiple of p + q) or the recommendation
    let mut period_n: Option<(u32, [u32; 2])> = chosen.map(|(i, _)| (cands[i].period, cands[i].n));
    let mut overridden = false;
    if let Some(pp) = ov.period {
        if pp > 0 && pp % (p + q) == 0 {
            let t = pp / (p + q);
            period_n = Some((pp, [t * p, t * q]));
            overridden = true;
        } else {
            notes.push(Note::warn(format!("period {pp} is not a multiple of {} (ratio {p}:{q}); using the recommended one", p + q), ""));
        }
    }
    overridden |= ov.first.is_some() || ov.k.is_some() || ov.sigma5.is_some();
    // nothing feasible: show one period (the smallest) with its shortfall
    let (period, n) = period_n.unwrap_or((p + q, [p, q]));
    if chosen.is_none() && ov.period.is_none() {
        notes.push(Note::fail(
            format!("no period keeps both ingredients ≥ m = {margin} (best S(P)/2 = {:.3}); shown with P = {period}", closed_sum(capacity, period as f64, c[0], c[1]) / 2.0),
            "BV1iQrKYoEoy 01:44",
        ));
    }
    let best = if !overridden {
        chosen.map(|x| x.1).or_else(|| best_for(capacity, c, n, 1, ov))
    } else {
        best_for(capacity, c, n, step, ov).or_else(|| best_for(capacity, c, n, 1, ov))
    };
    let Some(best) = best else { return Err("no layout".into()) };
    if overridden && best.min < margin - EPS {
        notes.push(Note::warn(format!("manual layout: lowest flow {:.3} < m = {margin}", best.min), "BV1fXkxYDEbW 33:31"));
    }
    if (period as f64) * c[0].max(c[1]) > capacity + EPS {
        notes.push(Note::fail(format!("a refill of {:.2} stacks/s exceeds one belt", period as f64 * c[0].max(c[1])), "BV1fXkxYDEbW 39:00"));
    }
    for cd in cands.iter_mut() {
        if cd.period == period && overridden {
            cd.best = Some(best.min);
        }
    }
    // keep the list short: the evaluated (larger) periods, the shown one and five below it
    let shown = cands.iter().position(|cd| cd.period == period).or(chosen.map(|x| x.0)).unwrap_or(cands.len());
    let lo = shown.saturating_sub(5);
    let cands: Vec<Candidate> = cands.into_iter().skip(lo).collect();
    // split into control units (optional cap)
    let (units, per_unit) = match ls.unit_cap.filter(|&u| u > 0 && machines > u) {
        Some(u) => {
            let k = machines.div_ceil(u);
            (k, machines.div_ceil(k))
        }
        None => (1, machines.max(1)),
    };
    let best = refine(capacity, c, n, if best.k % step == 0 { step } else { 1 }, ov, best, per_unit);
    let s5 = best.sigma5 as f64;
    let shares = [capacity * s5 / 5.0, capacity * (5.0 - s5) / 5.0];
    let (line_mins, feeds, returns) = run_two(capacity, c, shares, best.first as usize, best.k, n, per_unit);
    if line_mins[0].min(line_mins[1]) < margin - EPS && best.min >= margin - EPS {
        notes.push(Note::warn(format!("lowest flow on this line {:.3} < m", line_mins[0].min(line_mins[1])), "BV1fXkxYDEbW 79:26"));
    }
    let jump = if best.sigma5 >= 3 { 0 } else { 1 };
    let limited_share = best.sigma5.min(5 - best.sigma5);
    let limiter_lane = if limited_share == 2 { "green belt (2/5)" } else { "yellow belt, or blue belt + blue sorter (1/5)" }.to_string();
    // §2.4 checks
    let mut unit = "standard";
    for i in 0..2 {
        if returns[i] > shares[i] + EPS {
            notes.push(Note::fail(
                format!("{} returns {:.3} > its control-unit share {:.3}: it backs up and upsets the ratio", names[i], returns[i], shares[i]),
                "BV1iQrKYoEoy 27:50",
            ));
        }
    }
    let (kind, why) = unit_kind(returns, jump);
    match kind {
        "standard" => {}
        "double" => {
            unit = "double";
            notes.push(Note::info(
                format!("returns {} {:.3} + {} {:.3} = {:.3} > one green belt (12): use the two-green-belt control unit", names[0], returns[0], names[1], returns[1], returns[0] + returns[1]),
                "BV1fXkxYDEbW 66:46",
            ))
        }
        _ => {
            unit = "unknown";
            notes.push(Note::fail(format!("return flow beyond the known control units: {}", why.join("; ")), "BV1fXkxYDEbW 66:03"));
        }
    }
    let complete_add = (period - per_unit % period) % period;
    if complete_add > 0 && returns.iter().zip(&shares).any(|(r, s)| r > &(s + EPS)) {
        notes.push(Note::info(format!("adding {complete_add} machine(s) completes the period (returns then equal the unit's shares)"), "BV1iQrKYoEoy 28:05"));
    }
    // tower belts per item: first-fit decreasing of refill amounts into belts of C (§2.9)
    let mut tower_belts = [0u32; 2];
    for (i, tb) in tower_belts.iter_mut().enumerate() {
        let mut amounts: Vec<f64> = feeds.iter().filter(|f| f.item == i).map(|f| f.amount).collect();
        amounts.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let mut bins: Vec<f64> = vec![];
        for a in amounts {
            match bins.iter_mut().find(|b| **b + a <= capacity + EPS) {
                Some(b) => *b += a,
                None => bins.push(a),
            }
        }
        *tb = bins.len() as u32;
    }
    Ok((
        TwoItemDesign {
            c,
            p,
            q,
            ratio_dev,
            period,
            n,
            sum: closed_sum(capacity, period as f64, c[0], c[1]),
            sigma5: best.sigma5,
            first: best.first,
            k: best.k,
            best: best.min,
            best_mins: best.mins,
            line_mins,
            candidates: cands,
            recommended,
            overridden,
            feeds,
            returns,
            shares,
            jump,
            limiter_lane,
            unit: unit.into(),
            complete_add,
            tower_belts,
            units,
            machines_per_unit: per_unit,
        },
        notes,
    ))
}

/// The scheme-A line as simulator events (control unit, machines, refills, coaters on the refill
/// branches when sprayed).
pub fn two_events(d: &TwoItemDesign, capacity: f64, sprayed: bool) -> Vec<Event> {
    let mut ev = vec![Event::ControlUnit { shares: d.shares.to_vec(), jump: d.jump }];
    let mut fi = d.feeds.iter().peekable();
    for m in 1..=d.machines_per_unit {
        ev.push(Event::Machine { use_: d.c.to_vec() });
        while let Some(f) = fi.peek() {
            if f.pos != m {
                break;
            }
            if sprayed {
                ev.push(Event::Coater);
            }
            ev.push(Event::FillFeed { item: f.item, cap: capacity });
            fi.next();
        }
    }
    ev
}

// ---------------------------------------------------------------------------------------------
// scheme B: ladder groups (§2.5)

/// Feed rates a ladder can set (stacks/s): one rate-limited belt (0 or a belt tier up to the mixed
/// belt) plus up to two sorters (tier ≤ `sorter_max`, 1–3 grids), sorters before the belt. Each
/// value keeps its simplest combination (fewest parts).
pub fn feed_options(belt: u8, sorter_max: u8) -> Vec<(f64, String)> {
    let mut belts: Vec<(f64, String)> = vec![(0.0, String::new())];
    for (i, b) in BELTS.iter().enumerate().take(belt as usize + 1) {
        belts.push((*b, format!("{} belt", ["yellow", "green", "blue"][i])));
    }
    let mut sorters: Vec<(f64, String)> = vec![(0.0, String::new())];
    for t in 0..=(sorter_max as usize).min(2) {
        for g in 0..3 {
            sorters.push((SORTERS[t][g], format!("{} sorter ({} grid{})", SORTER_NAMES[t], g + 1, if g == 0 { "" } else { "s" })));
        }
    }
    let mut out: Vec<(f64, String, usize)> = vec![];
    for (bv, bn) in &belts {
        for (i, (s1, n1)) in sorters.iter().enumerate() {
            for (s2, n2) in sorters.iter().skip(i) {
                let v = bv + s1 + s2;
                if v <= 0.0 {
                    continue;
                }
                let parts: Vec<&str> = [bn.as_str(), n1.as_str(), n2.as_str()].into_iter().filter(|x| !x.is_empty()).collect();
                let key = (v * 1e6).round();
                match out.iter_mut().find(|o| (o.0 * 1e6).round() == key) {
                    Some(o) if o.2 <= parts.len() => {}
                    Some(o) => *o = (v, parts.join(" + "), parts.len()),
                    None => out.push((v, parts.join(" + "), parts.len())),
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    out.into_iter().map(|(v, n, _)| (v, n)).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LadderFeed {
    pub item: usize,
    /// N·c·(1 + ρ), stacks/s.
    pub need: f64,
    pub rate: f64,
    pub combo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LadderDesign {
    /// Machines per group.
    pub group: u32,
    pub groups: u32,
    /// Machines in the last group (= group when they divide).
    pub last_group: u32,
    pub rho: f64,
    pub feeds: Vec<LadderFeed>,
    pub total: f64,
    pub c: Vec<f64>,
    /// Recovery sorters per item (normal sorters take whole stacks; best tier available).
    pub recover_sorters: Vec<u32>,
}

/// Prototype `ladder`: the longest group N ≤ cap whose cheapest feeds Fᵢ ≥ N·cᵢ·(1 + ρ if N > short)
/// fit one belt. Returns (N, feeds).
pub fn ladder(c: &[f64], capacity: f64, opts: &[(f64, String)], rho: f64, short: u32, cap: u32) -> Option<(u32, Vec<LadderFeed>)> {
    for n in (1..=cap).rev() {
        let surplus = if n > short { 1.0 + rho } else { 1.0 };
        let mut feeds = vec![];
        let mut ok = true;
        for (i, ci) in c.iter().enumerate() {
            let need = n as f64 * ci * surplus;
            match opts.iter().find(|o| o.0 >= need - EPS) {
                Some(o) => feeds.push(LadderFeed { item: i, need, rate: o.0, combo: o.1.clone() }),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && feeds.iter().map(|f| f.rate).sum::<f64>() <= capacity + EPS {
            return Some((n, feeds));
        }
    }
    None
}

/// Ladder group as simulator events: feed block (each feed followed by its buffer cell), machines,
/// recovery block.
pub fn ladder_events(d: &LadderDesign, sprayed: bool) -> Vec<Event> {
    let mut ev = vec![];
    for f in &d.feeds {
        if sprayed {
            ev.push(Event::Coater);
        }
        ev.push(Event::FixedFeed { item: f.item, rate: f.rate });
    }
    for _ in 0..d.group {
        ev.push(Event::Machine { use_: d.c.clone() });
    }
    for f in &d.feeds {
        ev.push(Event::Recover { item: f.item, cap: f64::INFINITY });
    }
    ev
}

// ---------------------------------------------------------------------------------------------
// sorters (§2.7)

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SorterPlan {
    /// Input sorter tier name for each machine ("yellow" … "pile"), and the last machines on pile sorters.
    pub input: String,
    pub input_items: f64,
    pub end_pile: u32,
    /// Per product: sorter tier, rate used for the choice (items/s), machines until the single-stack
    /// output belt is full (later ones need pile sorters).
    pub outputs: Vec<OutputSorter>,
    /// Sorters with this layout: inputs + outputs + control unit / feed / recovery sorters.
    pub mixed: u32,
    /// Without mixed belts: one input sorter per ingredient.
    pub separate: u32,
    /// The solver's per-machine estimate (`compute::sorters_per_machine`) × machines.
    pub lp_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputSorter {
    pub item: String,
    pub name: String,
    pub rate: f64,
    pub sized_for: f64,
    pub tier: String,
    pub full_at: u32,
}

fn sorter_tier(rate_items: f64, per_trip: f64, strict: bool, ls: &BeltLayoutSettings) -> String {
    for t in 0..=(ls.sorter_max as usize).min(2) {
        let v = SORTERS[t][0] * per_trip;
        if (strict && v > rate_items + EPS) || (!strict && v >= rate_items - EPS) {
            return SORTER_NAMES[t].into();
        }
    }
    "pile".into()
}

// ---------------------------------------------------------------------------------------------
// plan layout

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Note {
    /// "fail" | "warn" | "info"
    pub level: String,
    pub text: String,
    /// 笑笑 transcript reference ("BV… mm:ss"), empty for model notes.
    pub cite: String,
}
impl Note {
    fn new(level: &str, text: String, cite: &str) -> Self {
        Self { level: level.into(), text, cite: cite.into() }
    }
    pub fn fail(text: String, cite: &str) -> Self {
        Self::new("fail", text, cite)
    }
    pub fn warn(text: String, cite: &str) -> Self {
        Self::new("warn", text, cite)
    }
    pub fn info(text: String, cite: &str) -> Self {
        Self::new("info", text, cite)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Rate {
    pub item: String,
    pub name: String,
    pub items: f64,
    pub stacks: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Coaters {
    pub coaters: u32,
    /// Items sprayed per second (new items entering the belt).
    pub sprays_per_s: f64,
    /// Proliferator items per second.
    pub proliferator_per_s: f64,
    pub proliferator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecipeLayout {
    /// "recipe|mode", the override key.
    pub key: String,
    pub id: String,
    pub mode: String,
    pub mode_label: String,
    pub name: String,
    pub building: String,
    pub machines: u32,
    pub inputs: Vec<Rate>,
    pub outputs: Vec<Rate>,
    /// "A" control unit + refills, "B" ladder groups, "none" no mixed belt.
    pub scheme: String,
    /// "ok" | "warn" | "fail"
    pub status: String,
    pub notes: Vec<Note>,
    pub two: Option<TwoItemDesign>,
    pub ladder: Option<LadderDesign>,
    pub sorters: SorterPlan,
    pub coaters: Option<Coaters>,
    pub trace: Trace,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MixedBeltReport {
    pub capacity: f64,
    pub stack: f64,
    pub margin: f64,
    pub recipes: Vec<RecipeLayout>,
    /// Recipes with a single ingredient (one item per tower, no mixed belt [BV1fXkxYDEbW 03:44]).
    pub single: Vec<String>,
    pub warnings: Vec<String>,
}

fn machines_of(r: &PlanRecipe, integer: bool) -> u32 {
    let n = if integer { r.machines.round() } else { r.machines_ceil.max(r.machines.ceil()) };
    n.max(0.0) as u32
}

/// Mixed-belt layout of every DSP recipe with ≥ 2 ingredients in `plan`.
pub fn layout_for_plan(plan: &Plan, data: &GameData, s: &PlanSettings) -> MixedBeltReport {
    let ls = &s.belt_layout;
    let capacity = ls.capacity();
    let stack = ls.stack();
    let mut out = MixedBeltReport { capacity, stack, margin: ls.margin(), ..Default::default() };
    if data.game != "dsp" {
        out.warnings.push("mixed belts are modelled for Dyson Sphere Program only".into());
        return out;
    }
    for pr in &plan.recipes {
        let Some(r) = data.recipe(&pr.id) else { continue };
        if r.inputs.len() < 2 {
            out.single.push(pr.name.clone());
            continue;
        }
        let machines = machines_of(pr, s.integer);
        if machines == 0 {
            continue;
        }
        out.recipes.push(layout_recipe(data, s, r, pr, machines));
    }
    out
}

fn layout_recipe(data: &GameData, s: &PlanSettings, r: &Recipe, pr: &PlanRecipe, machines: u32) -> RecipeLayout {
    let ls = &s.belt_layout;
    let capacity = ls.capacity();
    let stack = ls.stack();
    let key = format!("{}|{}", pr.id, pr.mode);
    let mut lay = RecipeLayout {
        key: key.clone(),
        id: pr.id.clone(),
        mode: pr.mode.clone(),
        mode_label: pr.mode_label.clone(),
        name: pr.name.clone(),
        building: pr.building.clone(),
        machines,
        scheme: "none".into(),
        ..Default::default()
    };
    let Some(mode) = find_mode(data, s, r, &pr.mode) else {
        lay.status = "fail".into();
        lay.notes.push(Note::fail("this recipe's machine / proliferator mode is no longer enabled: solve again".into(), ""));
        return lay;
    };
    let (ins, outs) = per_machine(r, &mode);
    lay.inputs = ins.iter().map(|(i, q)| Rate { item: i.clone(), name: data.item_name(i), items: *q, stacks: q / stack }).collect();
    lay.outputs = outs.iter().map(|(i, q)| Rate { item: i.clone(), name: data.item_name(i), items: *q, stacks: q / stack }).collect();
    let c: Vec<f64> = lay.inputs.iter().map(|x| x.stacks).collect();
    let d_items: f64 = ins.iter().map(|x| x.1).sum();
    let mut notes: Vec<Note> = vec![];
    // §2.2: a single machine over a refill / the belt
    if c.iter().any(|&x| x > capacity + EPS) || c.iter().sum::<f64>() >= capacity - EPS {
        lay.status = "fail".into();
        lay.notes.push(Note::fail(format!("one machine uses {:.2} stacks/s, more than a {capacity} cells/s belt: no mixed belt", c.iter().sum::<f64>()), ""));
        return lay;
    }
    let effect = pr.mode.rsplit('|').next().and_then(|e| data.effect(e)).filter(|e| e.kind == "proliferator");
    let mut unit_sorters = 0u32; // control unit / feed / recovery sorters
    let mut feed_points = 0u32;
    let mut end_pile = 0u32;
    let mut units = 1u32;
    let mut per_line = machines;
    if c.len() == 2 {
        lay.scheme = "A".into();
        let ov = ls.overrides.get(&key).cloned().unwrap_or_default();
        let names = [lay.inputs[0].name.clone(), lay.inputs[1].name.clone()];
        match design_two(capacity, [c[0], c[1]], [&names[0], &names[1]], machines, ls, &ov) {
            Ok((d, n)) => {
                notes.extend(n);
                lay.trace = simulate(&two_events(&d, capacity, mode.sprayed), capacity, 2);
                units = d.units;
                per_line = d.machines_per_unit;
                feed_points = d.feeds.len() as u32;
                // pile sorter over the limiter (two in the double unit), limiter sorter for the
                // 1/5 blue-belt variant counted as 1; refills sharing a tower belt take a pile sorter each
                let shared: u32 = feed_points - d.tower_belts.iter().sum::<u32>().min(feed_points);
                unit_sorters = units * ((if d.unit == "double" { 2 } else { 1 }) + shared);
                end_pile = ls.end_pile_a.min(per_line);
                // restart hint (§2.3 step 7)
                if d_items < ls.hint_items && d.machines_per_unit > ls.hint_machines {
                    notes.push(Note::info(
                        format!(
                            "slow recipe ({d_items:.2} items/s per machine) with {} machines on one control unit: restarts after a shortage are slow; consider one unit per tower (set a unit cap)",
                            d.machines_per_unit
                        ),
                        "BV1fXkxYDEbW 63:43",
                    ));
                }
                if d.units > 1 {
                    notes.push(Note::info(format!("split into {} control units of {} machines (unit cap)", d.units, d.machines_per_unit), ""));
                }
                if ls.rows >= 2 && per_line % 2 == 1 {
                    notes.push(Note::info("two rows per belt and an odd machine count: one position has a single machine".into(), ""));
                }
                lay.two = Some(d);
            }
            Err(e) => {
                lay.status = "fail".into();
                notes.push(Note::fail(e, ""));
            }
        }
    } else {
        lay.scheme = "B".into();
        let opts = feed_options(ls.belt, ls.sorter_max);
        let rho = if ls.ladder_rho.is_finite() && ls.ladder_rho >= 0.0 { ls.ladder_rho } else { 1.0 / 12.0 };
        match ladder(&c, capacity, &opts, rho, ls.ladder_short, ls.ladder_cap.max(1)) {
            Some((n, feeds)) => {
                let groups = machines.div_ceil(n);
                let last = machines - (groups - 1) * n;
                let best_sorter = SORTERS[(ls.sorter_max as usize).min(2)][0];
                let recover: Vec<u32> = feeds.iter().map(|f| (f.rate / best_sorter - EPS).ceil().max(1.0) as u32).collect();
                let feed_sorters: u32 = feeds.iter().map(|f| f.combo.matches("sorter").count() as u32).sum();
                unit_sorters = groups * (feed_sorters + recover.iter().sum::<u32>());
                feed_points = groups * feeds.len() as u32;
                end_pile = ls.end_pile_b.min(n);
                units = groups;
                per_line = n;
                let d = LadderDesign { group: n, groups, last_group: last, rho, total: feeds.iter().map(|f| f.rate).sum(), feeds, c: c.clone(), recover_sorters: recover };
                lay.trace = simulate(&ladder_events(&d, mode.sprayed), capacity, c.len());
                if n > ls.ladder_short {
                    notes.push(Note::info(format!("group of {n} > {}: feeds carry the {:.1} % surplus (fitted to his 24-machine particle-container line)", ls.ladder_short, 100.0 * rho), "BV1iQrKYoEoy 40:28"));
                }
                notes.push(Note::info("leave at least one buffer cell after every feed".into(), "BV1iQrKYoEoy 17:00"));
                lay.ladder = Some(d);
            }
            None => {
                lay.status = "fail".into();
                notes.push(Note::fail("no ladder group fits one belt, even a single machine".into(), ""));
            }
        }
    }
    // sorters (§2.7)
    let input = sorter_tier(d_items, stack, true, ls);
    if input == "pile" && PILE[(ls.pile_level as usize).min(2)].1 <= d_items {
        notes.push(Note::fail(format!("input {d_items:.2} items/s exceeds a pile sorter at {}", PILE[(ls.pile_level as usize).min(2)].0), ""));
    }
    let mut outputs = vec![];
    for (i, o) in &outs {
        // extra products come in bursts: size for the same tier's speed-up rate [BV1iQrKYoEoy 33:17]
        let sized_for = match effect {
            Some(e) if e.products_only => {
                let sp = data.effects.iter().find(|x| x.kind == "proliferator" && x.item == e.item && !x.products_only).map(|x| x.speed).unwrap_or(0.0);
                o / (1.0 + e.productivity) * (1.0 + sp)
            }
            _ => *o,
        };
        let tier = sorter_tier(sized_for, 1.0, false, ls);
        let full_at = ((capacity + EPS) / o).floor().min(u32::MAX as f64) as u32;
        if full_at < per_line {
            notes.push(Note::info(
                format!("{}: the single-stack output belt is full after {full_at} machines; later ones need pile sorters (switch early)", data.item_name(i)),
                "BV1iQrKYoEoy 34:12",
            ));
        }
        outputs.push(OutputSorter { item: i.clone(), name: data.item_name(i), rate: *o, sized_for, tier, full_at });
    }
    let n_out = outs.len() as u32;
    let lp_per = super::compute::sorters_per_machine(r, &mode, s.mixed_belts, super::compute::model().sorter_capacity);
    lay.sorters = SorterPlan {
        input,
        input_items: d_items,
        end_pile,
        outputs,
        mixed: machines + machines * n_out + unit_sorters,
        separate: machines * (ins.len() as u32 + n_out),
        lp_estimate: lp_per * machines as f64,
    };
    if !s.mixed_belts {
        notes.push(Note::info("the solver did not cost sorters as mixed belts (Sidebar: mixed input belts)".into(), ""));
    }
    // coaters (§2.10): on each refill / feed branch, before it joins the belt
    if let (true, Some(e)) = (mode.sprayed, effect) {
        let coaters = match lay.scheme.as_str() {
            "A" => feed_points + 2 * units,
            _ => feed_points,
        };
        let sprays = machines as f64 * d_items;
        lay.coaters = Some(Coaters {
            coaters,
            sprays_per_s: sprays,
            proliferator_per_s: sprays / e.sprays.unwrap_or(1.0).max(1.0),
            proliferator: data.item_name(&e.item),
        });
        notes.push(Note::info("spray coaters on the refill branches, before they join, so only new items are sprayed".into(), "BV11R2BYkExW 39:48"));
    }
    for v in &lay.trace.violations {
        let mut v = v.clone();
        for (i, x) in lay.inputs.iter().enumerate() {
            v = v.replace(&format!("item {i} "), &format!("{} ", x.name));
        }
        notes.push(Note::warn(format!("simulator: {v}"), ""));
    }
    lay.status = if notes.iter().any(|n| n.level == "fail") || lay.status == "fail" {
        "fail"
    } else if notes.iter().any(|n| n.level == "warn") {
        "warn"
    } else {
        "ok"
    }
    .into();
    lay.notes = notes;
    lay
}
