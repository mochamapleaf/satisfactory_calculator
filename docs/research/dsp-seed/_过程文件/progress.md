# progress dsp-seed
- 2026-11-21 step0: read survey/PROGRESS; transcripts present. next: grep transcripts, web survey seed gen.
- 2026-11-21 transcript hints: video 12 (BV1QF24YDE7H) + 16 (BV1BxDQYyEfU) read; hints: done (partial ok)
- 2026-11-21 transcript hints: videos 17 (BV1sQBtYxEsm), 18 (BV1wxikYXEHp), 19 (BV1fXkxYDEbW), 20 (BV1iQrKYoEoy) read; hints: done (partial ok)
- 2026-11-21 transcript hints: other transcripts grepped (seed/threat/drops), xiaoxiao-hints.md written; hints: done (partial ok)
- step1 (web): candidates found via GitHub API: DoubleUTH/DSP-Seed-Finder (Rust, Apache-2.0, real vein amounts), botany233/dsp_search_seed (C++/py GPL-3), w1w218/dsp-seed-atlas (C#, GPL-3, has planner), soarqin/DSPSeedCalc (MIT C++), crazyyao0 (Apache, 0.9.26), Xinyuell/DspFindSeed (GPL-3), AndrewLuebke/dsp-galaxy-db (1.8TB dataset, tooling only). next: clone DoubleUTH, build prototype.
- step1b: prototype seed_planets.py works (DoubleUTH native ws server, cargo build 37s, generate 64 stars actual veins <1s). samples saved. next: mechanics research (vein utilization, dark fog drops, orbital collectors).
- step2: mechanics: vein util 0.94^n (Saros readme, biligame L1-6 costs, 390/1681 zero-level claims 存疑); oil half-life & 0.1/s floor (biligame 原油涌泉 via search); orbital collector net formula (biligame); DF drop table by level, no rates published. Verified xiaoxiao S=815448 = sum_{k>=1} 4000k*0.94^(k+4). Planner run (scratch crate, rawpercube_*): raw per white cube scenarios A-G in rawpercube_output.txt; DF normal-item drops not modeled in data. next: write report.
- step3: S(L)=sum_{n>=L} 4000(n-5)0.94^(n-1): L6 815449, 20 630964, 40 302425, 49 204069, 60 122368, 80 45547, 85 35270, 100 16128. magnet thresholds @10/cube: B=0.18M ->L98, 142k ->L103. Report 00-最终报告.md written. next: critic (Opus) -> review.md -> fix.
critic: started review 00:36
critic: verified 7 GitHub repos/licences, BWIKI x4, Thunderstore x3, dsp-wiki, S(L) recomputed, transcripts checked; writing review 00:46
critic: done — review written to _过程文件/00-最终报告.review.md (0 critical, 5 major, 16 minor) 00:48
- step4: critic review applied to 00-最终报告.md (fixed framing of end-state bound, phi correction, quota view, +4%/level DF, cost law verified, units). DONE.
