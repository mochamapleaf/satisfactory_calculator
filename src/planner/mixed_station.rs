//! DSP mixed belt: the station module around a line (Xinyu 2026-11-23: 「我希望能看到一个图，比如说填料口在哪里，
//! 然后这个塔在哪里…能不能对输出和输入的所有传送带进行规划…每种物品必须要至少占一个口，然后每种产物也要占一个口…
//! 一般来说就是两种 configuration，一种是一个星际塔配上12个输入口…然后你需要生成出来像笑笑视频里那样的每个机器使用什么爪子，
//! 从哪个传送带上抓东西，然后我们的这个混料的投送到底应该放在哪里？」).
//!
//! Works on a finished `RecipeLayout` (scheme A or B): splits its machines over stations so each
//! station's belts fit its ports and its throughput, gives every port a role (control-unit feed,
//! refill, ladder feed, recovery, product, proliferator), every refill point a port, and every
//! machine its input sorter (from the mixed belt) and output sorter (to which product belt).
//! Geometry is schematic: positions are machine indices along the belt, not grid cells.
use super::data::tr;
use super::mixed_belt::{BeltLayoutSettings, RecipeLayout, PILE};
use serde::{Deserialize, Serialize};

const EPS: f64 = 1e-9;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Port {
    /// 1-based slot on the station (1–12 on a logistics station).
    pub slot: u32,
    /// "in" (station → line) or "out" (line → station).
    pub dir: String,
    pub item: String,
    pub name: String,
    /// "control unit" | "refill" | "feed" | "recovery" | "product" | "proliferator"
    pub role: String,
    /// Items/s on this belt.
    pub items: f64,
    /// Machine indices (within the line) this belt serves: refills after these machines, or the
    /// machines whose products it takes.
    pub serves: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Refill {
    /// After this machine (line index, 1-based; 0 = at the control unit).
    pub after: u32,
    pub item: String,
    pub name: String,
    /// Stacks/s.
    pub amount: f64,
    pub station: u32,
    /// Slots of the refill belts it takes from (two when it takes the rest of one belt and the next).
    pub slots: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MachineSlot {
    /// Line index, 1-based.
    pub n: u32,
    pub station: u32,
    /// Position along the belt (with two rows, two machines share a position).
    pub pos: u32,
    /// "L" | "R" (one row: always "L").
    pub side: String,
    /// Input sorter tier on the mixed belt ("yellow" … "pile").
    pub input: String,
    /// Per product: (station slot of its belt, sorter tier).
    pub outputs: Vec<(u32, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Station {
    pub index: u32,
    /// Line machines [first, last] at this station.
    pub first: u32,
    pub last: u32,
    pub ports: Vec<Port>,
    /// Items/s the station supplies to the line (its imports) and its limit: a hint, not a split
    /// rule (笑笑 runs 4.5 belts from a 4-belt planetary station when the goods come from nearby,
    /// BV1fXkxYDEbW 50:08).
    pub items: f64,
    pub limit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StationPlan {
    /// "ils" | "pls"
    pub kind: String,
    pub slots: u32,
    /// Stations for one line (one control unit, or one ladder group set).
    pub stations: Vec<Station>,
    /// Lines like this one in the plan (control units or ladder groups) and stations in all.
    pub lines: u32,
    pub stations_total: u32,
    pub refills: Vec<Refill>,
    pub machines: Vec<MachineSlot>,
    /// Copyable step list, like 笑笑's on-screen sequence.
    pub sequence: Vec<String>,
    pub notes: Vec<String>,
}

/// Station throughput in full belts: 笑笑's figures (BV1fXkxYDEbW 42:28, 84:44: planetary about 4 full belts,
/// interstellar twice that), not measured here.
pub fn station_belts(kind: &str) -> f64 {
    if kind == "pls" {
        4.0
    } else {
        8.0
    }
}

/// Refill belts of one item: the refills in line order filled into belts of `cap` one after the
/// other; a refill may take the rest of one belt and the start of the next (one pile sorter per
/// tap, report §2.9: a tower belt may feed several refill points). Returns per belt (amount, taps).
fn refill_belts(feeds: &[(f64, u32)], cap: f64) -> Vec<(f64, Vec<u32>)> {
    let mut belts: Vec<(f64, Vec<u32>)> = vec![];
    for &(a, pos) in feeds {
        let mut left = a;
        while left > EPS {
            if belts.last().is_none_or(|b| b.0 >= cap - EPS) {
                belts.push((0.0, vec![]));
            }
            let b = belts.last_mut().unwrap();
            let take = left.min(cap - b.0);
            b.0 += take;
            if !b.1.contains(&pos) {
                b.1.push(pos);
            }
            left -= take;
        }
    }
    belts
}

/// 笑笑 runs 4.5 full belts from a 4-belt planetary station when the goods come from nearby
/// (BV1fXkxYDEbW 50:08): the throughput limit allows 4.5 / 4 of the nominal belts.
pub const THROUGHPUT_TOL: f64 = 4.5 / 4.0;

/// Machines of one line laid out over stations; longer lines repeat the pattern (keeps the page fast).
pub const STATION_MACHINES_MAX: u32 = 2000;

struct Ctx<'a> {
    lay: &'a RecipeLayout,
    /// Items/s a product belt carries (stacked by pile sorters near the station).
    out_cap: f64,
    stack: f64,
    capacity: f64,
    slots: u32,
    limit: f64,
    sprayed: bool,
}

impl Ctx<'_> {
    fn port(&self, dir: &str, i: usize, role: &str, items: f64, serves: Vec<u32>) -> Port {
        let it = &self.lay.inputs[i];
        Port { slot: 0, dir: dir.into(), item: it.item.clone(), name: it.name.clone(), role: role.into(), items, serves }
    }

    /// Scheme A: a station holding machines `first..=last` of a line (control unit at machine 1).
    fn station_a(&self, idx: u32, first: u32, last: u32) -> Station {
        let d = self.lay.two.as_ref().unwrap();
        let mut ports = vec![];
        let mut items = 0.0;
        if first == 1 {
            for i in 0..2 {
                ports.push(self.port("in", i, "control unit", d.shares[i] * self.stack, vec![0]));
                items += d.shares[i] * self.stack;
            }
        }
        // refills after machines first..=last (the one after the last machine is still at this station)
        for i in 0..2 {
            let feeds: Vec<(f64, u32)> = d.feeds.iter().filter(|f| f.item == i && f.pos >= first && f.pos <= last).map(|f| (f.amount, f.pos)).collect();
            for (a, taps) in refill_belts(&feeds, self.capacity) {
                items += a * self.stack;
                ports.push(self.port("in", i, "refill", a * self.stack, taps));
            }
        }
        self.finish(idx, first, last, ports, items)
    }

    /// Scheme B: a station holding ladder groups `groups` (first machine, size) in line order.
    fn station_b(&self, idx: u32, groups: &[(u32, u32)]) -> Station {
        let g = self.lay.ladder.as_ref().unwrap();
        let mut ports = vec![];
        let mut items = 0.0;
        for &(start, size) in groups {
            for f in &g.feeds {
                ports.push(self.port("in", f.item, "feed", f.rate * self.stack, vec![start]));
                items += f.rate * self.stack;
                // what the group does not use comes back at its recovery block
                let back = (f.rate - size as f64 * g.c[f.item]).max(0.0) * self.stack;
                ports.push(self.port("out", f.item, "recovery", back, vec![start + size - 1]));
            }
        }
        let first = groups[0].0;
        let last = groups.last().map(|(s, n)| s + n - 1).unwrap_or(first);
        self.finish(idx, first, last, ports, items)
    }

    /// Products, proliferator and slot numbers (both schemes).
    fn finish(&self, idx: u32, first: u32, last: u32, mut ports: Vec<Port>, mut items: f64) -> Station {
        let n = last + 1 - first;
        for o in &self.lay.outputs {
            let total = o.items * n as f64;
            let belts = ((total / self.out_cap) - EPS).ceil().max(1.0) as u32;
            // machines split evenly over the product's belts, in line order
            for b in 0..belts {
                let lo = first + (n * b) / belts;
                let hi = first + (n * (b + 1)) / belts;
                let serves: Vec<u32> = (lo..hi).collect();
                ports.push(Port { slot: 0, dir: "out".into(), item: o.item.clone(), name: o.name.clone(), role: "product".into(), items: o.items * serves.len() as f64, serves });
            }
        }
        if let (true, Some(c)) = (self.sprayed, self.lay.coaters.as_ref()) {
            let share = n as f64 / self.lay.machines.max(1) as f64;
            ports.push(Port { slot: 0, dir: "in".into(), item: String::new(), name: c.proliferator.clone(), role: "proliferator".into(), items: c.proliferator_per_s * share, serves: vec![] });
            items += c.proliferator_per_s * share;
        }
        for (k, p) in ports.iter_mut().enumerate() {
            p.slot = k as u32 + 1;
        }
        Station { index: idx, first, last, ports, items, limit: self.limit }
    }

    /// One belt per slot, and the station's supply within its throughput (with `THROUGHPUT_TOL`).
    fn fits(&self, s: &Station) -> bool {
        s.ports.len() as u32 <= self.slots && s.items <= s.limit * THROUGHPUT_TOL + EPS
    }
}

/// Stations of a scheme-A line: each takes the longest run of machines that fits.
fn split_a(ctx: &Ctx, line_len: u32, notes: &mut Vec<String>) -> Vec<Station> {
    let mut out: Vec<Station> = vec![];
    let mut warned = false;
    let mut first = 1;
    while first <= line_len {
        let idx = out.len() as u32 + 1;
        let mut best = None;
        for last in first..=line_len {
            let s = ctx.station_a(idx, first, last);
            if ctx.fits(&s) {
                best = Some(s);
            } else if best.is_some() {
                break;
            }
        }
        let s = best.unwrap_or_else(|| {
            if !warned {
                warned = true;
                notes.push(tr(
                    &format!("one machine with its belts needs more than a station's {} slots or throughput", ctx.slots),
                    &format!("一台机器连同它的传送带需要的槽位或吞吐超过一座物流塔的 {} 个槽位", ctx.slots),
                ));
            }
            ctx.station_a(idx, first, first)
        });
        first = s.last + 1;
        out.push(s);
    }
    out
}

pub fn plan_stations(lay: &RecipeLayout, ls: &BeltLayoutSettings, capacity: f64, stack: f64, sprayed: bool) -> Option<StationPlan> {
    let kind = if ls.station == "pls" { "pls" } else { "ils" };
    let slots = ls.station_slots.clamp(1, 12);
    let ctx = Ctx { lay, out_cap: capacity * stack, stack, capacity, slots, limit: station_belts(kind) * capacity * stack, sprayed };
    let mut plan = StationPlan { kind: kind.into(), slots, ..Default::default() };
    let mut notes = vec![];
    // (first machine, size) of each ladder group, for the input pile sorters
    let mut group_ends: Vec<u32> = vec![];
    let mut ladder_warned = false;
    if let Some(d) = &lay.two {
        plan.lines = d.units;
        let len = d.machines_per_unit.min(STATION_MACHINES_MAX);
        if len < d.machines_per_unit {
            notes.push(tr(
                &format!("the first {len} of {} machines are laid out; the rest repeat the same stations", d.machines_per_unit),
                &format!("只画出前 {len} 台（共 {} 台）；其余重复同样的物流塔", d.machines_per_unit),
            ));
        }
        plan.stations = split_a(&ctx, len, &mut notes);
        group_ends.push(d.machines_per_unit);
        for s in &plan.stations {
            for i in 0..2 {
                let item = &lay.inputs[i].item;
                for f in d.feeds.iter().filter(|f| f.item == i && f.pos >= s.first && f.pos <= s.last) {
                    let slots: Vec<u32> = s.ports.iter().filter(|p| p.role == "refill" && &p.item == item && p.serves.contains(&f.pos)).map(|p| p.slot).collect();
                    plan.refills.push(Refill { after: f.pos, item: item.clone(), name: lay.inputs[i].name.clone(), amount: f.amount, station: s.index, slots });
                }
            }
        }
        plan.refills.sort_by_key(|r| r.after);
    } else if let Some(g) = &lay.ladder {
        // all groups of the recipe in line order, as many per station as fit
        plan.lines = 1;
        let shown = g.groups.min((STATION_MACHINES_MAX / g.group.max(1)).max(1));
        if shown < g.groups {
            notes.push(tr(
                &format!("the first {shown} of {} groups are laid out; the rest repeat the same stations", g.groups),
                &format!("只画出前 {shown} 组（共 {} 组）；其余重复同样的物流塔", g.groups),
            ));
        }
        let groups: Vec<(u32, u32)> =
            (0..shown).map(|k| (1 + k * g.group, if k + 1 == g.groups { g.last_group } else { g.group })).collect();
        group_ends = groups.iter().map(|(s, n)| s + n - 1).collect();
        let mut k = 0;
        while k < groups.len() {
            let idx = plan.stations.len() as u32 + 1;
            let mut take = 1;
            while k + take < groups.len() && ctx.fits(&ctx.station_b(idx, &groups[k..k + take + 1])) {
                take += 1;
            }
            let s = ctx.station_b(idx, &groups[k..k + take]);
            if !ctx.fits(&s) && !ladder_warned {
                ladder_warned = true;
                notes.push(tr(
                    &format!("one ladder group needs {} slots, more than the {slots} of a station", s.ports.len()),
                    &format!("一组梯形线需要 {} 个槽位，超过一座物流塔的 {slots} 个", s.ports.len()),
                ));
            }
            plan.stations.push(s);
            k += take;
        }
    } else {
        return None;
    }
    plan.stations_total = (plan.stations.len() as u32).saturating_mul(plan.lines.max(1));
    if let Some(g) = &lay.ladder {
        // groups beyond the laid-out ones repeat at the same groups per station
        let laid: u32 = plan.stations.iter().map(|s| s.ports.iter().filter(|p| p.role == "feed").count() as u32).sum::<u32>() / g.feeds.len().max(1) as u32;
        if laid > 0 && laid < g.groups {
            plan.stations_total = ((g.groups as u64 * plan.stations.len() as u64).div_ceil(laid as u64)).min(u32::MAX as u64) as u32;
        }
    }
    // machines: sorters and sides
    let out_tiers: Vec<(String, u32)> = lay.sorters.outputs.iter().map(|o| (o.tier.clone(), o.full_at)).collect();
    for s in &plan.stations {
        for n in s.first..=s.last {
            // last machines before the control unit / the group's recovery take pile sorters
            let end = group_ends.iter().copied().find(|e| *e >= n).unwrap_or(n);
            let input = if n + lay.sorters.end_pile > end { "pile".to_string() } else { lay.sorters.input.clone() };
            let mut outputs = vec![];
            for (k, o) in lay.outputs.iter().enumerate() {
                let Some(p) = s.ports.iter().find(|p| p.role == "product" && p.item == o.item && p.serves.contains(&n)) else { continue };
                // the product belt runs from the line's far end toward the station: a machine with
                // `full_at` or more machines upstream of it (and itself) stacks with a pile sorter,
                // one machine early (switch before the belt is full, BV1iQrKYoEoy 34:12)
                let upstream = p.serves.iter().filter(|&&m| m >= n).count() as u32;
                let (tier, full_at) = out_tiers.get(k).cloned().unwrap_or(("pile".into(), 0));
                let tier = if upstream + 1 > full_at { "pile".to_string() } else { tier };
                outputs.push((p.slot, tier));
            }
            let (pos, side) = if ls.rows >= 2 { ((n - s.first) / 2 + 1, if (n - s.first) % 2 == 0 { "L" } else { "R" }) } else { (n - s.first + 1, "L") };
            plan.machines.push(MachineSlot { n, station: s.index, pos, side: side.into(), input, outputs });
        }
    }
    // the step list
    let st = |x: f64| format!("{:.2}", x);
    let slot_list = |v: &[u32]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("+");
    for s in &plan.stations {
        let mut parts = vec![];
        if let (Some(d), 1) = (&lay.two, s.first) {
            let (a, b) = (&lay.inputs[0].name, &lay.inputs[1].name);
            let (sa, sb) = (st(d.shares[0]), st(d.shares[1]));
            let (pa, pb) = (s.ports[0].slot, s.ports[1].slot);
            parts.push(tr(
                &format!("control unit (slot {pa} {a} {sa}, slot {pb} {b} {sb} stacks/s)"),
                &format!("控制单元（槽位 {pa} {a} {sa}，槽位 {pb} {b} {sb} 组/秒）"),
            ));
        }
        let mut last = s.first - 1;
        if let Some(g) = &lay.ladder {
            // per group: its feeds (each followed by a buffer cell), its machines, its recoveries
            let mut starts: Vec<u32> = s.ports.iter().filter(|p| p.role == "feed").map(|p| p.serves[0]).collect();
            starts.dedup();
            for start in starts {
                for p in s.ports.iter().filter(|p| p.role == "feed" && p.serves[0] == start) {
                    let f = g.feeds.iter().find(|f| lay.inputs[f.item].item == p.item).unwrap();
                    parts.push(tr(
                        &format!("feed {} {} ({}) from slot {} + buffer cell", p.name, st(f.rate), f.combo, p.slot),
                        &format!("补料 {} {}（{}）取自槽位 {} + 缓冲格", p.name, st(f.rate), f.combo, p.slot),
                    ));
                }
                let end = s.ports.iter().find(|p| p.role == "recovery" && p.serves[0] >= start).map(|p| p.serves[0]).unwrap_or(s.last);
                parts.push(tr(&format!("{} machines", end + 1 - start), &format!("{} 台机器", end + 1 - start)));
                last = end;
                let rec: Vec<String> = s.ports.iter().filter(|p| p.role == "recovery" && p.serves[0] == end).map(|p| tr(&format!("{} → slot {}", p.name, p.slot), &format!("{} → 槽位 {}", p.name, p.slot))).collect();
                parts.push(tr(&format!("recover {}", rec.join(", ")), &format!("回收 {}", rec.join("，"))));
            }
        }
        for r in plan.refills.iter().filter(|r| r.station == s.index) {
            if r.after > last {
                parts.push(tr(&format!("{} machines", r.after - last), &format!("{} 台机器", r.after - last)));
                last = r.after;
            }
            parts.push(tr(
                &format!("refill {} {} from slot {}", r.name, st(r.amount), slot_list(&r.slots)),
                &format!("补料 {} {} 取自槽位 {}", r.name, st(r.amount), slot_list(&r.slots)),
            ));
        }
        if s.last > last {
            parts.push(tr(&format!("{} machines", s.last - last), &format!("{} 台机器", s.last - last)));
        }
        if lay.two.is_some() && s.last == lay.two.as_ref().unwrap().machines_per_unit {
            parts.push(tr("back to the control unit", "回到控制单元"));
        }
        plan.sequence.push(tr(&format!("Station {}: {}", s.index, parts.join(" → ")), &format!("物流塔 {}：{}", s.index, parts.join(" → "))));
    }
    for s in plan.stations.iter().filter(|s| s.items > s.limit + EPS) {
        notes.push(tr(
            &format!(
                "station {} supplies {:.0} items/s, {:.1} full belts: above the {} belts of a {} station, within the 4.5 / 4 笑笑 runs when the goods come from nearby (BV1fXkxYDEbW 50:08)",
                s.index,
                s.items,
                s.items / ctx.out_cap,
                station_belts(kind),
                if kind == "pls" { "planetary" } else { "interstellar" }
            ),
            &format!(
                "物流塔 {} 供应 {:.0} 个/秒，约 {:.1} 条满带：超过{}物流塔的 {} 条带；货物来自附近时在笑笑的 4.5 / 4 条带范围内 (BV1fXkxYDEbW 50:08)",
                s.index,
                s.items,
                s.items / ctx.out_cap,
                if kind == "pls" { "行星" } else { "星际" },
                station_belts(kind)
            ),
        ));
    }
    if plan.lines > 1 {
        notes.push(tr(
            &format!("{} modules like this one: {} stations in all", plan.lines, plan.stations_total),
            &format!("共 {} 个这样的模块：合计 {} 座物流塔", plan.lines, plan.stations_total),
        ));
    }
    if slots > 9 {
        notes.push(tr(
            "笑笑 leaves the three back slots of a station unused; set 9 slots to do the same",
            "笑笑把物流塔背面的三个槽位空着；设为 9 个槽位即可照做",
        ));
    }
    if plan.refills.iter().any(|r| r.slots.len() > 1) || plan.stations.iter().any(|s| s.ports.iter().any(|p| p.role == "refill" && p.serves.len() > 1)) {
        notes.push(tr(
            "a refill belt shared by several refill points takes a pile sorter per point, with a buffer belt before each (BV1fXkxYDEbW 39:00, BV1iQrKYoEoy 35:33)",
            "几个补料点共用一条补料带时，每个补料点要一个集装分拣器（白爪），前面各接一段缓冲带 (BV1fXkxYDEbW 39:00, BV1iQrKYoEoy 35:33)",
        ));
    }
    let pile = PILE[(ls.pile_level as usize).min(2)].0;
    if plan.machines.iter().any(|m| m.input == "pile" || m.outputs.iter().any(|o| o.1 == "pile")) {
        notes.push(tr(
            &format!("pile sorters at {pile}: upgrade if a pile sorter cannot keep up"),
            &format!("集装分拣器（白爪）按 {pile} 计：集装分拣器速度不够时请升级"),
        ));
    }
    plan.notes = notes;
    Some(plan)
}
