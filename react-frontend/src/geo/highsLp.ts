// highs-js (the same WASM package the planner's HiGHS engine loads) as the LpSolve of the siting model.
// The Model goes over as arrays (CSC matrix, rows sorted within each column), the way geo_solve.solve_lp hands
// it to highspy, with the same options (mip_rel_gap 1e-4, random_seed 0; highs-js is single-threaded like threads=1), optional MIP start and
// time limit.
import type { Highs } from 'highs'
import type { LpOptions, LpResult, LpSolve, Model } from './model.ts'

const finite = (v: unknown): number | null => {
  const x = typeof v === 'bigint' ? Number(v) : (v as number)
  return typeof x === 'number' && Number.isFinite(x) ? x : null
}

/** CSC arrays of the model's rows. */
function csc(m: Model) {
  const n = m.c.length
  const cols: [number, number][][] = Array.from({ length: n }, () => [])
  m.rows.forEach((row, i) => { for (const [j, a] of row) if (a !== 0) cols[j].push([i, a]) })
  const starts = new Int32Array(n + 1)
  let nz = 0
  for (let j = 0; j < n; j++) { starts[j] = nz; nz += cols[j].length }
  starts[n] = nz
  const indices = new Int32Array(nz), values = new Float64Array(nz)
  let k = 0
  for (let j = 0; j < n; j++) {
    cols[j].sort((p, q) => p[0] - q[0])
    for (const [i, a] of cols[j]) { indices[k] = i; values[k] = a; k++ }
  }
  return { starts, indices, values }
}

/** LpSolve on a loaded highs-js instance. */
export function highsSolver(highs: Highs): LpSolve {
  return (m: Model, opts: LpOptions): LpResult => {
    const t0 = performance.now()
    const n = m.c.length, nr = m.rows.length
    const inf = highs.infinity
    const cap = (v: number) => (v === Infinity ? inf : v === -Infinity ? -inf : v)
    const mip = m.integ.some(Boolean)
    const { starts, indices, values } = csc(m)
    const model = highs.createModel({
      numCols: n, numRows: nr,
      colCost: Float64Array.from(m.c), colLower: Float64Array.from(m.lb, cap), colUpper: Float64Array.from(m.ub, cap),
      rowLower: Float64Array.from(m.lo, cap), rowUpper: Float64Array.from(m.hi, cap),
      matrix: { format: 'csc', numRows: nr, numCols: n, starts, indices, values },
      ...(mip ? { integrality: Int32Array.from(m.integ, t => (t ? 1 : 0)) } : {}),
    })
    try {
      const o: Record<string, unknown> = { output_flag: false, random_seed: 0, mip_rel_gap: opts.gap ?? 1e-4 }
      if (opts.timeLimit) o.time_limit = opts.timeLimit
      model.options.set(o)
      if (mip && opts.warm) {
        try { model.setSolution({ colValue: opts.warm }) } catch { /* start refused: solve without */ }
      }
      const cb = highs.constants.callbackType
      let last = 0
      model.run(opts.progress ? {
        [cb.mipLogging](e) {
          const now = performance.now()
          if (now - last < 500) return
          last = now
          opts.progress!({ bound: finite(e.data.mip_dual_bound), best: finite(e.data.objective_function_value), nodes: Number(e.data.mip_node_count ?? 0) })
        },
      } : undefined)
      const st = model.getModelStatus()
      const names = highs.constants.modelStatus as unknown as Record<string, number>
      const name = Object.entries(names).find(([, v]) => v === st)?.[0] ?? String(st)
      const status = st === names.optimal ? 'Optimal' : name
      let x: Float64Array | null = null
      try {
        const pss = finite(model.info.get('primal_solution_status'))
        const s = model.getSolution()
        if ((pss === null || pss >= 2) && s.colValue?.length === n) x = Float64Array.from(s.colValue)
      } catch { /* no solution */ }
      const obj = x ? finite(model.info.get('objective_function_value')) : null
      const bound = mip ? finite(model.info.get('mip_dual_bound')) : obj
      const gap = mip ? finite(model.info.get('mip_gap')) ?? Infinity : 0
      return { x, obj, bound, gap, status, time: (performance.now() - t0) / 1000 }
    } finally {
      model.dispose()
    }
  }
}
