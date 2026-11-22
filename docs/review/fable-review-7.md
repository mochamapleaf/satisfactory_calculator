# Review 7 — whole-building Factorio quality plans (machines shared across tiers), machine quality

Date 2026-11-22. Scope: commit 0372822 (`lp.rs` shared-machine MILP, `quality.rs` / `rules.rs` machine quality,
`Results.tsx` grouped rows, `solver.ts` race, converter `quality_speed`). File:line references are to 0372822.
Spec: `docs/research/factorio-quality/00-最终报告.md` §2.1 / §5 step 6; step log `_过程文件/progress-int.md`.

## What was run

| Check | Result |
|---|---|
| `cargo test -q --release` at 0372822 | 92 passed, 5 ignored |
| `cargo test -q --release` / `cargo test -q` after the fixes | 95 passed, 5 ignored / 94 passed, 6 ignored |
| `npm run build:all` (wasm-pack release, `tsc -b`, vite build), eslint on the changed files | clean; dist rebuilt from the fixed tree |
| New Rust tests | 3: `plant_harvest_is_normal`, `integer_shared_engine_slack_is_not_shown`, `integer_shared_beacons_at_whole_machines`. The last two were run against the 0372822 `lp.rs` and fail there (messages quoted below) |
| Bench `quality_integer_bench` (4 s, native microlp) | before and after, table in H1 / M3 |
| Live browser, after the fixes | Playwright Chromium on `vite preview` :4173 (build of the fixed tree): `docs/review/scripts/live_test_review7.py`, shots + texts + `live-log.txt` in `docs/review/shots-review7/`. Flows listed in the Resolution |
| Game rules | wiki Quality page (machine quality), forum + mod portal (seed quality), FactorioLab data in `data/factorio-*.json` |

Counts: **Critical 0 · High 1 · Medium 3 · Low 4.**

The shared-machine formulation itself is sound: the group key (base recipe, machine, loadout, beacon count) only
joins columns with the same speed, power and held modules; the capacity row is in machines (x / speed ≤ n_c,
Σ n_c ≤ N); HiGHS, the sliced microlp solve, Stop / Keep solving and the row check all handle the continuous
shares. The defects are one wrong game rule that the new bench case leaned on, and what the plan shows around the
solved point.

---

## High

### H1. Plants pass the seed's quality to the harvest: a legendary seed loop gives free legendary fruit
`src/planner/quality.rs:175-181` (`expand`), data `yumako-tree`, `jellystem`, `tree-plant` (building `agricultural-tower`).

Every recipe with a solid input is expanded to `r@i` with tier-i inputs and tier-i products. For the three plant
recipes this makes `yumako-tree@4`: legendary seed → 50 legendary yumako. In the game a planted seed's quality is
lost and the harvest is always normal (the forum suggestion "Have quality seeds grow quality plants",
https://forums.factorio.com/viewtopic.php?p=644983, and the "Quality Seeds" mod exist because of it; the wiki pages
for the tower, the seed and the tree say nothing about quality). Since processing legendary fruit returns
legendary seeds, the model had a self-feeding legendary fruit loop with no quality module anywhere.

Failure scenario: legendary plastic 60/min with Gleba allowed (the bench case "plastic@4 all planets" and the live
Stop / Keep solving case of 0372822): the plan was 7–8 buildings, objective 0.08, raw cost ≈ 0 — legendary bioplastic
from legendary jelly and yumako grown from legendary seeds (`shots-int/lds-highs-kept.txt`: "Yumako tree 2 tiers,
normal 0.07, legendary 0.93"). The "weak bound, 79 % gap" of progress-int.md §4 was this case: an unphysical plan
whose only cost is a handful of buildings. Any quality target reachable through Gleba fruit was affected, with
whole buildings or as an LP.

## Medium

### M1. The plan shows the solver's share variables, which cost nothing and can sit anywhere in a group's spare capacity
`src/planner/lp.rs:461-466` (`point_from`), `:422` (`offer`), `:767-812` and `:866-882` (`finish`).

Each tier column has a continuous share n_c at cost 0 with x_c ≤ speed·n_c and Σ n_c ≤ N. Any n_c between
x_c / speed and x_c / speed + (N − Σ) is optimal, and when a group's machine costs nothing (Buildings weight 0, no
module cost) so is any N. `point_from` stored the solver's n_c, `finish` showed it as the tier's machines, computed
"% busy" and the building counts from Σ n_c, and never read the solved N. Which vertex comes back depends on the
engine. The tests of 0372822 pass only because microlp happened to return n_c = x_c / speed.

Failure scenario (new test, fails on 0372822 with "widget@4: share 1 is not its work"): the proven optimum of the
widget loop is handed back through `offer_values` (the HiGHS path) with one group's spare capacity moved onto a
share. That tier's row then reads 1 machine at a clock below 100 %, the group shows 100 % busy, and with free
machines the building count would follow the inflated shares.

### M2. Beacons and their power are counted on the fractional share, the objective charges them per whole machine
`src/planner/lp.rs:785` (`+ f.beacon_power * n`), `:810` (`beacons: n * f.beacons`), against `:366-371` (N carries
the module, beacon-module and beacon MW cost).

With shared machines `n` in `finish` is a tier's share. A group of 2 furnaces doing 1.98 machines of work showed
15.87 beacons (2 × 8 = 16 stand there) and the idle fraction's beacon MW was missing from the plan's power, while
the objective had paid for both. Test `integer_shared_beacons_at_whole_machines` fails on 0372822:
`("iron-plate", "electric-furnace|q0p2b8"): 15.87 beacons for 2 machines (1.98 of work)`. The total happened to
round up to the right whole number in that case; with several beaconed groups the Buildings card loses beacons.

### M3. "Can the formulation be tightened cheaply?" — tried, no
Asked in the brief for the weak-bound legendary plastic case. After H1 that case is no longer weak: LP 4.5345, best
4.7312, gap 4.2 % at 4 s (234 buildings), not 79 %. Cover cuts were implemented and measured (an item that must be
made — a target, or an input of every column making such an item — needs Σ N_g ≥ 1 over its makers' groups, and
≥ ⌈rate / best output per machine⌉ for a target), then removed:

| bench row (4 s) | without cuts | with cuts |
|---|---|---|
| quality module 1@4 | proven 2673 ms | proven 1764 ms |
| processing unit@4 | proven 1519 ms | proven 1924 ms |
| plastic@4 all planets | gap 4.16 %, 4.0 s | gap 4.06 %, 5.5 s (over the limit) |
| gear, circuit, LDS chain | proven at the first plan | same |

No consistent gain: the remaining gap is the rounding of ~240 groups that each need more than one machine, which
"at least one machine" cuts do not touch. Not kept. What would help that case is a faster dual bound (HiGHS in the
browser already gets further than microlp), not more rows.

## Low

### L1. A link with a quality tier that is not 0 … 4 blanks the plan
`react-frontend/src/urlState.ts:39` (`upgrade`). `machine_quality` (and the older `module_quality`,
`beacon_quality`) pass `sanitize` as any finite number; the solver parses them as `u8`, so `2.5` or `-1` in a
hand-edited link fails with "bad settings" and no plan. An old link without the field is fine (default 0, checked live).

### L2. Machine graph: one node per recipe id showed only its last loadout, and a tier's fractional share with no hint
`react-frontend/src/FlowGraph.tsx:17`. `new Map(plan.recipes.map(r => [r.id, r]))` keeps one row per id; quality
plans can run one tier on two loadouts (the LP mixes loadouts per tier), and the node's machines and MW were then
those of one of them. In a whole-building plan the node read "0.16× Assembling machine 3" with nothing saying it is a share
of 12 shared machines.

### L3. Shared rows lose the "alt" tag
`react-frontend/src/Results.tsx` (`sharedRow`).

### L4. "optimal" tolerates several buildings (not changed)
`src/planner/lp.rs:48` (`MIP_GAP = 1e-4`, relative). Under the default objective a building weighs about 0.01
against resources: legendary gear 10/min is 1032.28, so the plan is called proven when within 0.10 ≈ 10 buildings
of the bound, and the rounded LP is accepted at once (50 buildings, 12 ms). The objective is met to its tolerance,
but a reader of "optimal" on a plan whose point is the building count may expect the fewest buildings. Same rule
as every other game's whole-building plans (review 4); a tighter rule would leave most quality plans "not proven"
at the time limit. Left for the owner.

### Left as they are
- Recyclers and furnaces take any item without a recipe change, so in the game they could be shared across
  different recipes too; the model keeps one group per base recipe (never too few machines, sometimes one too many
  per recipe). Assemblers need a circuit-set recipe to switch tiers and lose the productivity bar on a switch
  (report §1.6 notes it). Design scope of the report, not changed.
- `drop_dominated` / `detier` are off for shared plans (a per-column move is not exact there): a few more columns,
  no wrong result.
- Item flow tab shows a tier's share ("0.16× Assembling machine 3"); it sums loadouts correctly.

## Checked, no finding
- **Groups that must not share.** Key = base recipe + mode key (`machine|q{k}p{m}[b{count}]`). The legendary tier
  has no quality loadouts, so it only joins the 0Q groups, with the same modules. Different machines, loadouts or
  beacon counts never share. Research and built-in productivity are per base recipe and machine, equal across
  the tiers of a group.
- **Capacity units.** x is work in machine-minutes at speed 1 per minute, speed is the mode's (machine quality ×
  modules × beacons), n_c and N are machines. Hand check in the beacons shot: AM3 4P, 0.70 + 0.05 of work on
  1 machine = 74.5 % busy.
- **Machine quality.** Wiki Quality page: crafting machines gain crafting speed (+30 % per level, legendary ×2.5) and
  health only; AM3 row 1.25 / 1.625 / 2 / 2.375 / 3.125 equals `quality_speed`. No energy row, module slots
  unchanged, so MW per machine unchanged and MW per craft falls. Beacons (−1/6 power per level, effectivity) have
  their own setting. Every building in 2x1 / 2q1 / spa has `quality_speed` except the agricultural tower (no speed
  effect). Burner fuel per craft follows the new speed (`fuel_io`). Live: legendary machines 50 → 21 buildings.
- **Reductions.** Linear power is exact here (quality loadouts never overclock, exponent 1); beacon MW sit on N.
- **HiGHS / validation.** `General` holds only N; `offer_values` rounds only integer columns; `rows_hold` passes
  with continuous shares; Auto races both engines. Live: gear and quality module plans agree across engines.
- **Old links.** Without `machine_quality`: default Normal, same plan as before.

---

## Resolution

Fixed (uncommitted work at the time of writing is in the commit that adds this file):

| # | Fix | Test |
|---|---|---|
| H1 | `quality.rs`: recipes of the agricultural tower yield normal products at every seed tier (`PLANT_BUILDING`, `normal_products`); quality seeds can still be planted | `plant_harvest_is_normal`; bench plastic row now LP 4.53 / plan 4.73, 234 buildings; live: 0 legendary fruit lines |
| M1 | `lp.rs`: `point_from` takes a shared tier's share as x / speed; `offer` sets every group to ⌈Σ shares⌉ (`group_machines`, tolerance `WHOLE_TOL` 1e-6, the one `finish` uses) so objective, counts and rows agree | `integer_shared_engine_slack_is_not_shown` (fails before) |
| M2 | `lp.rs` `finish`: each tier row carries its part of the group's whole machines for beacons and beacon MW | `integer_shared_beacons_at_whole_machines` (fails before); live: 2 furnaces → 16 beacons |
| M3 | measured, not kept | bench table above |
| L1 | `urlState.ts`: the three quality tiers rounded and clamped to 0 … 4 | live: links with 2.5 / −1 / 9 show a plan, select reads Epic / Normal / Legendary |
| L2 | `FlowGraph.tsx`: node sums its loadouts; "share of N machines" on a tier that shares | live: graph tab, light and dark |
| L3 | `Results.tsx`: alt tag on shared rows | build |
| L4 | not changed | — |

Live flows tested after the fixes (all pass, no console or page errors; `shots-review7/live-log.txt`):
1. Legendary gear 10/min × microlp / HiGHS / Auto: 50 buildings, optimal; every shared row checked by script
   (whole count is whole, tier split fits it, busy % = split / whole).
2. Same with speed beacons and Buildings weight 50 (HiGHS): 40 buildings incl. 16 beacons, 3 shared rows.
3. Legendary quality module 1/min × microlp / HiGHS: 37 buildings, 6 shared rows checked; Item flow and Machine graph tabs.
4. Legendary plastic, all planets, Auto, 8 s: Stop at 3 s keeps the plan (234 buildings, 4.2 % gap), Keep solving
   resumes and ends; no legendary fruit.
5. Machine quality: link with legendary (21 buildings), old link without the field (50), bad values, select changed by hand (rare: 33).
6. 390 px, dark, and 390 px dark: no page overflow (table scrolls in its box), shared rows present, both flow tabs render.

Not tested live: the Chinese UI on these rows. Not done: quality mining (report §5 step 3),
group-level dominance, cross-recipe recycler sharing.
