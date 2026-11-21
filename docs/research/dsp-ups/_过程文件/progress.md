# DSP UPS model — progress

## 2026-11-21 step 1: local context
- Done: read docs/PROGRESS.md, calculators-survey.md (DSP §1.3/§2.1), data/dsp.json building ids.
- Transcripts read: ep18 BV1wxikYXEHp 06:20–40:00 (优化技巧), ep19 BV1fXkxYDEbW (混带), ep20 BV1iQrKYoEoy.
  Key hints: logic frame 60/s? (she says default 30 logic : 60 render — 存疑, game is 60 UPS); blocked/idle machines
  cost more than running ones (claim); white (pile/Mk4) sorters waste CPU vs matched-speed sorters; mixed belts save
  sorters + belt length; 四向分流器 heavier than sorter splitting; in-system interplanetary (星系内) ILS flights
  costliest (orbit calc), fewer ILS better; PLS cheap; dark fog farm + dyson swarm big.
- Local game: DSP folder on G: has no DSPGAME_Data (uninstalled) → no local decompile; rely on mod sources.
- Next: web — DSPOptimizations, SampleAndHoldSim, BlackBox, multithreading devlog, benchmarks.

## step 2: mod sources
- Selsion/DSPMods DSPOptimizations README (sphere/swarm/station-belt MT; planned belt/splitter/lab work). Current repo has no inserter/assembler opt.
- Weaver (Loom, https://github.com/TheAIBot/DSP_Weaver, master pushed 2026-10-18): readme + OptimizedInserter.cs,
  OptimizedAssembler.cs, InserterExecutor.cs downloaded to scratchpad src/weaver. Findings:
  - vanilla-like inserter stage machine Picking/Sending/Inserting/Returning; Sending/Returning = time+= only;
    Picking polls belt every tick unless careNeeds (then idleTick=9/10 backoff checking needs); Inserting backs off idleTick=10.
  - assembler update when blocked = few compares (output>limit or served<require) — O(1) like running.
  - Weaver adds Inactive states (vanilla lacks them) → idle cheap only with Weaver. Weaver readme: memory bandwidth/large structs dominate.
- Next: DSP official patch notes on perf (multithreading 0.9.27?, 0.10.x optimizations), benchmarks, belts/station/splitter code.

## step 3: official + aggregate data
- GPUOpen/devlog (2026-02-12; devtrackers 81e22586): sorters ~3.6 ms of 22 ms logic frame (old MT), 11.7 ms new;
  assembler "few hundred to few thousand ns"; 100K matrix save: production-line logic 80 ms; labs still main thread;
  monitor/spray/station output 0.6 ms -> <0.1 ms.
- Patch 0.10.34.28281 (2026-01-22, eprison mirror): cache-hit/memory-bandwidth optimisation 5–30 %, better data access for assemblers/labs/sorters, power cheaper. No idle-sleep mentioned.
- Weaver station executor: vanilla StationComponent.InternalTickLocal + InternalTickRemote each tick per station; UpdateNeeds each tick.
- Weaver belt: CargoPath.Update returns early when jammed; memmove of moving region otherwise (∝ length).
- Rough count: 10k white/min, no prolif, Mk3/plane/quantum tiers ≈ 35k machines (scratchpad count.py).
- 60 UPS confirmed (SampleAndHoldSim page, reddit); xiaoxiao's "30 logic fps default" contradicted.
- Next: write 00-最终报告.md + model.json, then critic.

## step 4: deliverables drafted
- Wrote docs/research/dsp-ups/00-最终报告.md and model.json (ids checked against data/dsp.json).
- Next: Opus critic -> _过程文件/00-最终报告.review.md, then apply fixes.

## step 5: critic done -> 00-最终报告.review.md (sorter 0.6 inconsistent, calibration version mix, station timeGene %60). Next: apply fixes.

## step 6: fixes applied to report + model.json per review (sorter 0.12, stations 3/5, belt 0.1/path, calibration 0.25us ~10x, SAHS runtime, k_r cap). Done.
