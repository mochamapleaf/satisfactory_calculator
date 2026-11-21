# progress — factorio-quality research (2026-11-21)

## Step 0 — read existing docs
- Read PROGRESS.md, calculators-survey.md §2.2.3, src/planner/rules.rs, data/factorio.json.
- Finding: data/factorio.json is **base 2.0.77, no Space Age** (FactorioLab `2.0`). No quality modules, no
  recycler, no EM plant/foundry/cryo plant in data. Need FactorioLab `spa` data for quality work.
- Modes today: one module type fills all slots; no beacons; Effect.quality field exists (speed modules −1%..).
- Next: web research on mechanics (wiki Quality, Recycler, Module, FFF), transcripts grep.

## Step 4 (done early) — 笑笑 transcripts
- Sonnet subagent wrote `xiaoxiao-quality-notes.md` (~40 KB): 星岩转转乐 asteroid loop (ep37/38), circuit
  recipe switching, foundry 产能保护, legendary prod3/quality3 routes (ep39/40/46), no speed beacons on
  quality machines, 2.1 removes quality modules from crusher reprocessing. ep36 missing, ep41 truncated 40:59.
- Next: web research mechanics (step 1).

## Step 1 — mechanics (web, 2026-11-21)
- Fetched wiki Quality, Tutorial:Quality_upcycling_math, Recycler, Module, Quality_module_3, Beacon, Productivity,
  EM plant, Foundry, Cryogenic plant, Crusher, Big mining drill, Casting LDS, Version_history/2.1.0,
  lua-api RecipePrototype + QualityPrototype (2.1.25), FFF-375, scottmsul/FactorioQualityOptimizer, quality-calc mod.
- Key: tier jump 0.9/0.09/0.009/0.001 (×q); recycler 25 %, speed 0.5, 4 slots, no prod, time 1/16;
  prod cap 300 % (maximum_productivity default 3.0); EM 2.0/5 slots/+50 %, Foundry 4/4/+50 %, Cryo 2/8/0;
  Q3 legendary 6.2 % (2.0) vs 6.25 % (wiki module page, 2.1.7 "resolution of 0.01%") -> version dependent;
  2.1.7: no quality modules in asteroid reprocessing (0.4 self + 0.2 + 0.2), asteroid crushing 20->30 % chunk return.
  Beacons: speed/efficiency only. Extra products share rolled quality.
- Next: LP validation script reproducing wiki 0.013 / 2.72 % EM-plant numbers.

## Step 2 — LP validation (quality_lp.py, scipy HiGHS)
- A. scottmsul gear example reproduced exactly: 79.8786 normal sets per legendary gear (2Q/2P all tiers),
  [3,2,2,2] -> 84.1518 (README 84.1518). Closed form (forward substitution) == LP.
- B. wiki 0.013 = EM plant, all 5 Q incl. legendary stage (p5=0.5), metric = legendary products per normal
  *product* entering the recycler (L = A·R + C). Reproduced 0.01299.
- Wiki "percentage yield" table: EM 2.722332 % and cryo 4.835199 % reproduced with integer loadouts (1,1,1,1)/(0,0,0,0);
  chem/AM3/foundry values need **fractional** module counts (1.6 Q etc.) -> wiki table is a continuous relaxation.
- D. pure self-recycle 4×Q3L: 1 legendary per 2726.9 normal; 2.0 asteroid casino 2×Q3L, 80 %: 0.0210.
- E. min machines (fractional) 15.53 machines / 96.31 plates/s vs input-optimal 19.99 machines / 79.88 plates/s.
- 2.1 values (Q3L 6.25 %, recycler 25 %): gear loop -> see report.
- Next: write 00-最终报告.md.
- 7:1 check: 4×Q3L all-quality assembler loop gives x0/(x1+x2+x3) = 7.09 -> matches 笑笑 ep41 "七个组装机做普通，一个做所有高品".
- 27:1 (笑笑 ep39/42) vs model self-recycle 2726.9:1 -> likely "两千七比一" mis-heard (存疑).
- Next: write report.

## Step 3/5 — report written
- `../00-最终报告.md`: mechanics + URLs, model/closed form/worked gear example, wiki/scottmsul comparison, objectives,
  笑笑 methods, 9-step implementation plan, 前置知识, 存疑 list.
- Next: critic subagent (Opus) -> `00-最终报告.review.md`, then apply fixes.

## Step 6 — critic done (review at 00-最终报告.review.md); applying fixes M1-M5 + minors.
- script: relabelled B, added F (wiki metric, integer vs fractional), G (7:1 sensitivity), 2.1 self-recycle 2682.3, asteroid 47.7:1, input-optimal 19.99 machines. Next: report edits.
- Report fixes applied: M1 (fluid-only recipes + quality mining routes), M2 (§2.4 numbers now from script F, wiki
  machine-mix argument), M3 (plan rewritten: recipe@q in data layer per survey, touched functions, supply-cost test,
  research-productivity step, spa data facts, removed false PROGRESS step-10 citation), M4 (27:1 wording), M5 (7:1
  sensitivity), minors (LDS premise/losses/source, mod-page inference, quality-calc 0.124 存疑, prior art, 47.7:1).
- DONE.

## Step 7 — second review (fable-review) applied, 2026-11-21
- Script: added H (wiki metric LP with loadout mixing; wiki fractional rows substituted), I (normal:higher
  ratio by loadout: 4.12/5.03/7.09/3.68), J (LDS shuffle by research level: 15 lv P3L & 25 lv Q3L both 1.000,
  14.9 vs 7.7 casts per 5 plastic, cost 1.31 M vs 75.75 M; <15 lv best is all-prod, 14 lv 0.696).
- Report §0 and §1.1/§1.2/§1.5 revised (catalyst, mining as 2nd stage, 2.1 API fields, 2x1 module values, crush loop 6350:1).
- Next: §1.6 LDS, §2, §3, §4, §5, §6/§7, sources.
- §1.6 LDS rewritten (table from J), §2.1 catalyst + shared-machine, §2.3 allowed-recipes, §2.4 H-section numbers, §2.5 ratio table, §3.1 scale, §3.3 recycle scope, §4 7:1 scope + ep46 4-tries. Next: §5-§7, sources.
- §5 rewritten (step 0 scale test, datasets spa/2x1/2q1, catalyst, mining choice, recycle scope, tests, shared machines, version switch), §6/§7/sources updated. Resolution appended to fable-review. DONE (not committed).
