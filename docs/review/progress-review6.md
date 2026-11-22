# Review 6 progress (geo Map tab; range a243811..36fd943)

## step 0 (2026-11-22)
- Read model.ts, sweep.ts, highsLp.ts, geoWorker.ts, MapView.tsx, map.css, geo.test.ts, geo_solve.py, progress-g6.md.
- Next: baseline test:geo / tsc / build, live browser run, write fable-review-6.md, fix, retest.

## step 1: baseline + findings (done)
- Baseline at 36fd943: `npm run test:geo` 25/25 pass (226 s), `tsc -b` clean, `vite build` ok. Preview :4174 (this worktree's dist).
- Live (Playwright) before fixes: 12k plate OK (95 124, 4.3 %, p* 2/2/0). Found: fmt(x,0) strips integer zeros (timer " s"/"1 s"
  at 10 s); tab switch mid-solve leaves "Solving…" forever with no worker; supplied-input plan -> "aggregate LP: infeasible";
  late-game 7-target plan: 29 352 vars, every solve hits 30 s, 181 s sweep, gap 16.8 %, works.
## step 2: fixes written (not yet committed)
- model.ts: `supplied` option (free at base), unmakeable(), rawShortfalls(). sweep.ts: setup inside try, named infeasibility
  messages, stop on infeasible, rejected rows instead of aborting, no-rail = one solve, meta.F always 1x, sorted recipes.
- MapView.tsx: unmount marks state stopped, onerror guard, pStar null, unsolved-points warning, legend, supplied note.
  fmt moved to src/fmt.ts and fixed. map.css: phone font sizes.
- Tests: 6 new in geo.test.ts, pass. Live script: docs/research/satisfactory-geo/_过程文件/scripts/live_test_review6.py
- Next: full test:geo, build, live run, write fable-review-6.md, commit.

## step 3: tests + live after fixes
- `npm run test:geo`: 31/31 pass (227 s). tsc, eslint, vite build clean.
- Live found one more (M5): "Rail saves 11.2 %" on a late-game plan with 0 hubs; fixed (belts-only reference = cheapest
  hub-less plan; no saving claimed without hubs).
- docs/review/fable-review-6.md written with Resolution. Final live run of live_test_review6.py in progress; then commit.

## step 4: done
- Final live run on the fixed build: 20/20 checks pass (plate, tab switch, stop, supplied, tier 4, shortfall, late game 390 dark).
- Fixed H1, H2, M1-M6, L1-L7; left L8-L11 (see fable-review-6.md). Committed on worktree-agent-ac4b893018efffcdc, not pushed.
