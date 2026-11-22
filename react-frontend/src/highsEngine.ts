// HiGHS (highs-js, MIT, WebAssembly) as the whole-building MILP engine, loaded only when a
// whole-building solve needs it (docs/research/solver-speed/00-最终报告.md §6 step 5). microlp
// keeps the LP, the first plans and the heuristics; HiGHS gets the reduced model as LP text,
// starts from the best plan in hand, and every incumbent it finds goes back to microlp's run,
// which keeps it only when it fits the budgets and is cheaper.
import type { Highs } from 'highs'
import highsWasmUrl from 'highs/runtime?url'

let loading: Promise<Highs> | null = null
/** ms the last load took (download + compile), 0 before. */
export let highsLoadMs = 0

export function loadHighs(): Promise<Highs> {
  if (!loading) {
    const t0 = performance.now()
    loading = import('highs')
      .then(m => m.default({ locateFile: () => highsWasmUrl }))
      .then(h => { highsLoadMs = performance.now() - t0; return h })
    loading.catch(() => { loading = null })
  }
  return loading
}

/** Forget the loaded instance (after a failed run): the next `loadHighs` starts a new one. */
export function resetHighs() {
  loading = null
}

export interface HighsRun {
  /** HiGHS model status name: optimal, timeLimit, interrupted, infeasible, … */
  status: string
  proven: boolean
  /** Final incumbent in model column order (v0, v1, …), or null. */
  values: Float64Array | null
  bound: number | null
  nodes: number
  ms: number
}

/** Solve the LP text (columns v0..v{n-1}) from `start`, within `seconds`. `improving` gets each
 *  new incumbent (model column order) and the current bound; `progress` the bound and gap. */
export function runHighs(
  highs: Highs, lp: string, start: Float64Array, seconds: number,
  improving: (values: Float64Array, bound: number | null) => void,
  progress: (bound: number | null, nodes: number) => void,
): HighsRun {
  const t0 = performance.now()
  const model = highs.createModel({ format: 'lp', data: lp })
  try {
    const n = start.length
    // the LP reader numbers columns by first appearance: map them back to v0..v{n-1}
    const col = new Int32Array(n)
    for (let j = 0; j < n; j++) col[j] = model.getColByName(`v${j}`)
    const toModel = (x: Float64Array) => { const y = new Float64Array(n); for (let j = 0; j < n; j++) y[col[j]] = x[j]; return y }
    const fromModel = (y: Float64Array) => { const x = new Float64Array(n); for (let j = 0; j < n; j++) x[j] = y[col[j]]; return x }
    // the relative gap alone decides (lp.rs MIP_GAP): HiGHS's default absolute gap of 1e-6 would call
    // plans with a small objective (DSP: 0.01 per building) optimal at a larger relative gap
    model.options.set({ output_flag: false, time_limit: Math.max(0.05, seconds), mip_rel_gap: 1e-4, mip_abs_gap: 1e-10 })
    if (start.length && start.some(v => v !== 0)) {
      try { model.setSolution({ colValue: toModel(start) }) } catch { /* start refused: solve without */ }
    }
    let lastLog = 0
    const cb = highs.constants.callbackType
    model.run({
      [cb.mipImprovingSolution](e) {
        const sol = e.data.mip_solution
        if (sol && sol.length === n) improving(fromModel(sol), finite(e.data.mip_dual_bound))
      },
      [cb.mipLogging](e) {
        const now = performance.now()
        if (now - lastLog < 200) return
        lastLog = now
        progress(finite(e.data.mip_dual_bound), Number(e.data.mip_node_count ?? 0))
      },
    })
    const st = model.getModelStatus()
    const name = (Object.entries(highs.constants.modelStatus).find(([, v]) => v === st)?.[0]) ?? String(st)
    let values: Float64Array | null = null
    try { const s = model.getSolution(); if (s.colValue?.length === n) values = fromModel(s.colValue) } catch { /* no solution */ }
    const bound = infoNumber(model, 'mip_dual_bound')
    const nodes = infoNumber(model, 'mip_node_count') ?? 0
    return { status: name, proven: st === highs.constants.modelStatus.optimal, values, bound, nodes, ms: performance.now() - t0 }
  } finally {
    model.dispose()
  }
}

function finite(x: number | undefined | null): number | null {
  return typeof x === 'number' && Number.isFinite(x) ? x : null
}
function infoNumber(model: { info: { get(name: string): unknown } }, name: string): number | null {
  try { const v = model.info.get(name); return finite(typeof v === 'bigint' ? Number(v) : (v as number)) } catch { return null }
}
