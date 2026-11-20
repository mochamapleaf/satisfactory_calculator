// Runs the WASM solver off the main thread (integer solves can take seconds).
import init, { solve_json } from './wasm-pkg/satisfactory_calculator.js'

const ready = init()
self.onmessage = async (e: MessageEvent<{ id: number; settings: string }>) => {
  await ready
  let plan: string
  try {
    plan = solve_json(e.data.settings)
  } catch (err) {
    plan = JSON.stringify({ status: 'error', warnings: [String(err)] })
  }
  ;(self as unknown as Worker).postMessage({ id: e.data.id, plan })
}
