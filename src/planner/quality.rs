//! Factorio quality: `item@q` expansion (docs/research/factorio-quality/00-最终报告.md §2, §5).
//!
//! Every solid item gets tiers q = 0 (normal, the bare id) … 4 (legendary, `id@4`); fluids have no
//! quality. Every recipe on the target's chain becomes `r@i` (i = input tier; recipes with only
//! fluid inputs exist only at i = 0) and each machine × module loadout is one mode whose products
//! are split over tiers by the quality matrix Q(q). Recyclers are ordinary recipes, restricted to a
//! recycling scope (otherwise pruning keeps almost the whole game). The LP core is unchanged.
//!
//! Q(q)[i][j]: stay 1−q, one tier up 0.9q, two 0.09q, three 0.009q; whatever would pass
//! legendary lands on legendary (normal → legendary 0.001q). Productivity multiplies the
//! non-catalyst part of a product; extra products share the rolled quality.
use super::data::{EffectTier, GameData, Recipe};
use super::rules::{Mode, MAX_PRODUCTIVITY};
use super::PlanSettings;
use std::collections::{BTreeSet, HashMap, HashSet};

pub const TIERS: u8 = 5;
pub const LEGENDARY: u8 = 4;
const DEFAULT_NAMES: [&str; 5] = ["normal", "uncommon", "rare", "epic", "legendary"];

/// "iron-gear-wheel@4" → ("iron-gear-wheel", 4); ids without a tier suffix → (id, 0).
pub fn split(id: &str) -> (&str, u8) {
    if let Some((base, t)) = id.rsplit_once('@') {
        if let Ok(q) = t.parse::<u8>() {
            if q < TIERS && !base.is_empty() {
                return (base, q);
            }
        }
    }
    (id, 0)
}

/// Item / recipe id at tier `q` (normal keeps the bare id).
pub fn qid(base: &str, q: u8) -> String {
    if q == 0 {
        base.to_string()
    } else {
        format!("{base}@{q}")
    }
}

/// Lower-case tier name from the data ("legendary").
pub fn tier_name(data: &GameData, q: u8) -> String {
    data.qualities
        .get(q as usize)
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| DEFAULT_NAMES.get(q as usize).copied().unwrap_or("?").to_string())
}

/// Row `i` of the quality transition matrix for total quality chance `q`.
pub fn qrow(q: f64, i: u8) -> [f64; 5] {
    let q = q.clamp(0.0, 1.0);
    let mut row = [0.0; 5];
    let i = i as usize;
    if i >= LEGENDARY as usize {
        row[LEGENDARY as usize] = 1.0;
        return row;
    }
    row[i] = 1.0 - q;
    for (j, r) in row.iter_mut().enumerate().take(LEGENDARY as usize).skip(i + 1) {
        *r = q * 0.9 * 0.1f64.powi((j - i - 1) as i32);
    }
    row[LEGENDARY as usize] = q * 0.1f64.powi((LEGENDARY as usize - 1 - i) as i32);
    row
}

/// Quality expansion is on when the data has tiers and a target or supply names a tier > 0.
pub fn active(data: &GameData, s: &PlanSettings) -> bool {
    !data.qualities.is_empty()
        && s.targets.iter().chain(&s.supplies).any(|t| split(&t.item).1 > 0)
}

/// A module chosen for quality loadouts, at the configured module tier.
#[derive(Clone)]
struct Module {
    item: String,
    v: EffectTier,
}

fn module(data: &GameData, id: &str, tier: u8) -> Option<Module> {
    if id.is_empty() {
        return None;
    }
    data.effect(id).filter(|e| e.kind == "module").map(|e| Module { item: e.item.clone(), v: e.tier(tier) })
}

/// Upstream closure (base ids) over `recipes`: items needed for `targets` and the recipes making them.
fn closure<'a>(recipes: &[&'a Recipe], targets: &BTreeSet<String>) -> (BTreeSet<String>, Vec<&'a Recipe>) {
    let mut by_out: HashMap<&str, Vec<usize>> = HashMap::new();
    for (k, r) in recipes.iter().enumerate() {
        for (o, _) in &r.outputs {
            by_out.entry(o.as_str()).or_default().push(k);
        }
    }
    let mut needed: BTreeSet<String> = targets.clone();
    let mut queue: Vec<String> = targets.iter().cloned().collect();
    let mut chosen = vec![false; recipes.len()];
    while let Some(it) = queue.pop() {
        for &k in by_out.get(it.as_str()).map(|v| v.as_slice()).unwrap_or(&[]) {
            if chosen[k] {
                continue;
            }
            chosen[k] = true;
            for (i, _) in &recipes[k].inputs {
                if needed.insert(i.clone()) {
                    queue.push(i.clone());
                }
            }
        }
    }
    let rs = recipes.iter().zip(&chosen).filter(|(_, c)| **c).map(|(r, _)| *r).collect();
    (needed, rs)
}

/// Recycling recipes allowed by the scope: "chain" (default) = items on the targets' own
/// ingredient chain, "all" = every recycling recipe (finds detours, near-full model size),
/// "list" = only `recycle_allow`. Items in `recycle_allow` are always allowed.
pub fn recycling_allowed<'a>(
    s: &PlanSettings,
    allowed: &[&'a Recipe],
    target_bases: &BTreeSet<String>,
) -> Vec<&'a Recipe> {
    let crafting: Vec<&Recipe> = allowed.iter().copied().filter(|r| !r.recycling).collect();
    let chain = match s.recycling.as_str() {
        "all" | "list" => BTreeSet::new(),
        _ => closure(&crafting, target_bases).0,
    };
    allowed
        .iter()
        .copied()
        .filter(|r| r.recycling)
        .filter(|r| {
            let input = r.inputs.first().map(|i| i.0.as_str()).unwrap_or("");
            s.recycling == "all"
                || chain.contains(input)
                || s.recycle_allow.iter().any(|a| a == input || *a == r.id)
        })
        .collect()
}

/// Expanded recipes `r@i` with their loadout modes, for the base recipes that can contribute
/// to the targets (pruned at the base level, recycling within its scope).
pub fn expand(data: &GameData, s: &PlanSettings, allowed: &[&Recipe], target_ids: &[String]) -> Vec<(Recipe, Vec<Mode>)> {
    let target_bases: BTreeSet<String> = target_ids.iter().map(|t| split(t).0.to_string()).collect();
    let mut pool: Vec<&Recipe> = allowed.iter().copied().filter(|r| !r.recycling).collect();
    pool.extend(recycling_allowed(s, allowed, &target_bases));
    let (_, chain) = closure(&pool, &target_bases);
    let liquid: HashSet<&str> = data.items.iter().filter(|i| i.liquid).map(|i| i.id.as_str()).collect();
    let solid = |it: &str| !liquid.contains(it);
    let qm = module(data, &s.quality_module, s.module_quality).filter(|m| m.v.quality > 0.0);
    let pm = module(data, &s.prod_module, s.module_quality).filter(|m| m.v.productivity > 0.0);
    let mut out = Vec::new();
    for r in chain {
        let max_in = if r.inputs.iter().any(|(i, _)| solid(i)) { LEGENDARY } else { 0 };
        for i in 0..=max_in {
            let id = qid(&r.id, i);
            if i > 0 && s.disabled_recipes.contains(&id) {
                continue;
            }
            let tier = |it: &str| if solid(it) { qid(it, i) } else { it.to_string() };
            let suffix = if i == 0 { String::new() } else { format!(" ★{}", tier_name(data, i)) };
            let er = Recipe {
                id,
                name: format!("{}{suffix}", r.name),
                name_zh: r.name_zh.as_ref().map(|z| format!("{z}{suffix}")),
                inputs: r.inputs.iter().map(|(it, q)| (tier(it), *q)).collect(),
                outputs: r.outputs.iter().map(|(it, q)| (tier(it), *q)).collect(),
                catalyst: r.catalyst.iter().map(|(it, q)| (tier(it), *q)).collect(),
                ..r.clone()
            };
            let modes = loadout_modes(data, s, r, i, &qm, &pm, &solid);
            if !modes.is_empty() {
                out.push((er, modes));
            }
        }
    }
    out
}

/// Machine × loadout modes of base recipe `r` run at input tier `i`: k quality modules and
/// the other slots productivity modules (k = 0 … slots; plus a plain k = 0 mode), products
/// split by Q(k·q_m). Quality modules only where they can matter (tier < legendary, solid
/// products, allowed by recipe and machine).
fn loadout_modes(
    data: &GameData,
    s: &PlanSettings,
    r: &Recipe,
    i: u8,
    qm: &Option<Module>,
    pm: &Option<Module>,
    solid: &dyn Fn(&str) -> bool,
) -> Vec<Mode> {
    let producers: Vec<&str> =
        if r.producers.is_empty() { vec![r.building.as_str()] } else { r.producers.iter().map(|p| p.as_str()).collect() };
    let research = s.research_productivity.get(&r.id).copied().unwrap_or(0.0);
    let solid_out = r.outputs.iter().any(|(o, _)| solid(o));
    let mut out = Vec::new();
    for pid in producers {
        if s.disabled_buildings.iter().any(|d| d == pid) {
            continue;
        }
        let Some(b) = data.building(pid) else { continue };
        let n = b.modules;
        let prod_ok = r.productivity_allowed && !b.disallowed_effects.iter().any(|x| x == "productivity");
        let q_ok = qm.is_some()
            && r.quality_allowed
            && !b.disallowed_effects.iter().any(|x| x == "quality")
            && i < LEGENDARY
            && solid_out
            && n > 0;
        let mut loadouts: Vec<(u32, u32)> = Vec::new(); // (quality, productivity) modules
        let full_p = if prod_ok && pm.is_some() { n } else { 0 };
        loadouts.push((0, 0));
        if full_p > 0 {
            loadouts.push((0, full_p));
        }
        if q_ok {
            for k in 1..=n {
                loadouts.push((k, if full_p > 0 { n - k } else { 0 }));
            }
        }
        let tier_label = if r.producers.len() > 1 { data.pick(&b.name, &b.name_zh).to_string() } else { String::new() };
        for (k, m) in loadouts {
            let (kq, km) = (k as f64, m as f64);
            let zero = EffectTier::default();
            let qv = qm.as_ref().map(|x| x.v).unwrap_or(zero);
            let pv = pm.as_ref().map(|x| x.v).unwrap_or(zero);
            let qchance = (kq * qv.quality + km * pv.quality).max(0.0);
            let p = if r.productivity_allowed {
                (b.base_productivity + research + km * pv.productivity).min(MAX_PRODUCTIVITY)
            } else {
                0.0
            };
            let speed = b.speed * (1.0 + kq * qv.speed + km * pv.speed).max(0.2);
            let power = b.power * (1.0 + kq * qv.consumption + km * pv.consumption).max(0.2);
            let row = qrow(qchance, i);
            let mut outs: Vec<(String, f64)> = Vec::new();
            for (o, a) in &r.outputs {
                let c = r.catalyst_of(o).min(*a);
                let amount = c + (a - c) * (1.0 + p);
                if !solid(o) {
                    outs.push((o.clone(), amount));
                    continue;
                }
                for (j, share) in row.iter().enumerate() {
                    if *share > 1e-12 {
                        outs.push((qid(o, j as u8), amount * share));
                    }
                }
            }
            let load = match (k, m) {
                (0, 0) => String::new(),
                (k, 0) => format!("{k}Q"),
                (0, m) => format!("{m}P"),
                (k, m) => format!("{k}Q {m}P"),
            };
            let label = [tier_label.as_str(), load.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ");
            let mut held = Vec::new();
            if let (Some(q), true) = (qm, k > 0) {
                held.push((q.item.clone(), kq));
            }
            if let (Some(pp), true) = (pm, m > 0) {
                held.push((pp.item.clone(), km));
            }
            out.push(Mode {
                key: format!("{}|q{k}p{m}", b.id),
                label,
                building: b.id.clone(),
                speed,
                out_mult: 1.0 + p,
                power_base: power,
                power_exp: 1.0,
                overclock: false,
                extra_inputs: vec![],
                held,
                somersloops: 0,
                sprayed: false,
                outputs: Some(outs),
            });
        }
    }
    out
}
