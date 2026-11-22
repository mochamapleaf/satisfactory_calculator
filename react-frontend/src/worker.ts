// Runs the WASM solver off the main thread and streams its progress (see solver.ts).
//   in:  {type:'solve', id, settings} | {type:'stop', id} | {type:'resume', id, ms}
//   out: {id, update}  (update = JSON of a SolveUpdate)
import init, { solve_extend, solve_start, solve_step } from './wasm-pkg/satisfactory_calculator.js'

/** Length of one solver step (ms): the plan on screen is at most this old. */
const SLICE_MS = 150
const ready = init()
let cur = 0 // solve the WASM state belongs to
let gen = 0 // bumped by every message: a step loop from an older one stops
const post = (id: number, update: string) => (self as unknown as Worker).postMessage({ id, update })
const failed = (err: unknown) => JSON.stringify({ done: true, plan: { status: 'error', warnings: [String(err)] } })

async function loop(id: number, g: number) {
  for (;;) {
    // yield, so a stop / resume / new solve is seen between steps
    await new Promise(r => setTimeout(r, 0))
    if (g !== gen) return
    let out: string
    try { out = solve_step(SLICE_MS) } catch (err) { out = failed(err) }
    post(id, out)
    if ((JSON.parse(out) as { done: boolean }).done) return
  }
}

self.onmessage = async (e: MessageEvent<{ type: string; id: number; settings?: string; ms?: number }>) => {
  await ready
  const m = e.data
  const g = ++gen
  if (m.type === 'stop') return
  if (m.type === 'resume') {
    if (m.id !== cur || !solve_extend(m.ms ?? 4000)) { post(m.id, JSON.stringify({ done: true, gone: true, plan: null })); return }
    loop(m.id, g)
    return
  }
  cur = m.id
  let out: string
  try { out = solve_start(m.settings ?? '{}') } catch (err) { out = failed(err) }
  post(m.id, out)
  if (!(JSON.parse(out) as { done: boolean }).done) loop(m.id, g)
}
