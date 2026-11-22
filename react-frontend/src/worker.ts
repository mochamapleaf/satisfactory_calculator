// Runs the WASM solver off the main thread and streams its progress (see solver.ts).
//   in:  {type:'solve', id, settings} | {type:'stop', id} | {type:'resume', id, ms}
//   out: {id, update}  (update = JSON of a SolveUpdate)
// Whole-building plans: microlp gives the LP, the first plans and the heuristics; with the HiGHS
// engine the full search then goes to HiGHS (loaded on first use, in parallel with microlp's
// first steps), whose incumbents go back through microlp's run (budget and row check).
import init, {
  solve_best_values, solve_engine_gap, solve_extend, solve_finish, solve_heuristics_done, solve_model_lp, solve_offer, solve_start, solve_step,
} from './wasm-pkg/satisfactory_calculator.js'
import { highsLoadMs, loadHighs, resetHighs, runHighs } from './highsEngine'
import type { Highs } from 'highs'

/** Length of one solver step (ms): the plan on screen is at most this old. */
const SLICE_MS = 150
const ready = init()
let cur = 0 // solve the WASM state belongs to
let gen = 0 // bumped by every message: a step loop from an older one stops
let wantHighs = false // the solve asked for HiGHS
let useHighs = false // … and HiGHS is (still) usable
let raceLane = false // Auto's HiGHS lane: another worker runs microlp, so this one never falls back to it
let highsTried = false
let highs: Highs | null = null
let highsStatus = ''
const post = (id: number, update: string, extra?: Record<string, unknown>) => {
  if (extra) update = JSON.stringify({ ...JSON.parse(update), ...extra })
  ;(self as unknown as Worker).postMessage({ id, update })
}
const failed = (err: unknown) => JSON.stringify({ done: true, plan: { status: 'error', warnings: [String(err)] } })
const engineInfo = () => (wantHighs ? { engine: useHighs && highsTried ? 'highs' : 'microlp', highs_load_ms: Math.round(highsLoadMs), highs_status: highsStatus } : { engine: 'microlp' })

/** The full search on HiGHS for the time left. Returns the final update, or null when HiGHS
 *  failed (out of memory, an abort): the plan in hand stays and microlp goes on (review 4 M3). */
function highsSearch(id: number, h: Highs, seconds: number): string | null {
  highsTried = true
  const lp = solve_model_lp()
  const busy = { engine_busy: true, ...engineInfo() }
  // tell the page before blocking: a stop now has to terminate the worker
  post(id, solve_offer(new Float64Array(), NaN, false), busy)
  const t0 = performance.now()
  let out = ''
  try {
    // HiGHS stops at the relative gap of the plan in hand when it starts (lp.rs engine_gap); a
    // better plan found on the way can need a tighter one, so a proof the run does not accept
    // (still within MIP_GAP, not under one building) starts HiGHS again from that plan
    let prevGap = NaN
    for (let round = 0; round < 4; round++) {
      const left = seconds - (performance.now() - t0) / 1000
      const gap = solve_engine_gap()
      // refused for another reason (row or budget check): the same gap would repeat the same solve
      if (round > 0 && (left <= 0.05 || gap === prevGap)) break
      prevGap = gap
      const r = runHighs(h, lp, new Float64Array(solve_best_values()), left, gap,
        (values, bound) => post(id, solve_offer(values, bound ?? NaN, false), busy),
        bound => { if (bound !== null) post(id, solve_offer(new Float64Array(), bound, false), busy) })
      highsStatus = r.status
      out = solve_offer(r.values ?? new Float64Array(), r.bound ?? NaN, r.proven)
      if (JSON.parse(out).done || !r.proven) break
    }
  } catch (err) {
    // the instance may be unusable after an abort: the next solve loads a new one
    highs = null
    resetHighs()
    useHighs = false
    highsStatus = `failed: ${err}`
    return null
  }
  return JSON.parse(out).done ? out : solve_finish()
}

async function loop(id: number, g: number) {
  for (;;) {
    // yield, so a stop / resume / new solve is seen between steps
    await new Promise(r => setTimeout(r, 0))
    if (g !== gen) return
    let out: string
    try {
      if (raceLane && wantHighs && !useHighs) {
        // HiGHS is not available (offline, load or run failure): the microlp lane has this search
        out = solve_finish()
      } else if (useHighs && !highsTried && highs && solve_heuristics_done()) {
        const left = ((last.deadline_ms ?? 0) - (last.elapsed_ms ?? 0)) / 1000
        out = (left > 0.05 ? highsSearch(id, highs, left) : null) ?? solve_step(SLICE_MS)
      } else out = solve_step(SLICE_MS)
    } catch (err) { out = failed(err) }
    const u = JSON.parse(out) as { done: boolean; elapsed_ms?: number; deadline_ms?: number }
    if (u.elapsed_ms !== undefined) last = u
    post(id, out, engineInfo())
    if (u.done) return
  }
}
let last: { elapsed_ms?: number; deadline_ms?: number } = {}

self.onmessage = async (e: MessageEvent<{ type: string; id: number; settings?: string; ms?: number; race?: boolean }>) => {
  await ready
  const m = e.data
  const g = ++gen
  if (m.type === 'stop') return
  const run = (id: number) => loop(id, g)
  if (m.type === 'resume') {
    if (m.id !== cur || !solve_extend(m.ms ?? 4000)) { post(m.id, JSON.stringify({ done: true, gone: true, plan: null })); return }
    highsTried = false
    last = JSON.parse(solve_offer(new Float64Array(), NaN, false))
    run(m.id)
    return
  }
  cur = m.id
  const settings = JSON.parse(m.settings ?? '{}') as { integer?: boolean; engine?: string }
  // 'auto' is resolved by the page (solver.ts races a microlp and a HiGHS worker)
  wantHighs = useHighs = !!settings.integer && settings.engine === 'highs'
  raceLane = !!m.race
  highsTried = false
  highsStatus = ''
  let out: string
  try { out = solve_start(m.settings ?? '{}') } catch (err) { out = failed(err) }
  const u = JSON.parse(out) as { done: boolean; elapsed_ms?: number; deadline_ms?: number }
  last = u
  post(m.id, out, engineInfo())
  if (u.done) return
  if (useHighs && !highs) loadHighs().then(h => { highs = h }).catch(err => { useHighs = false; highsStatus = `load failed: ${err}` })
  run(m.id)
}
