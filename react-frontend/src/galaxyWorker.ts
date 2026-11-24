// DSP seed planning off the main thread (src/planner/dsp_galaxy.rs via WASM).
//   in:  {id, fn: 'galaxy' | 'thresholds' | 'plan' | 'total' | 'rate', json}
//   out: {id, out}  (out = the function's JSON, or {"error": …})
import init, { dsp_galaxy_json, dsp_galaxy_plan_json, dsp_galaxy_total_json, dsp_galaxy_rate_json, dsp_thresholds_json } from './wasm-pkg/satisfactory_calculator.js'

const ready = init()
const FNS: Record<string, (json: string) => string> = {
  galaxy: dsp_galaxy_json,
  thresholds: dsp_thresholds_json,
  plan: dsp_galaxy_plan_json,
  total: dsp_galaxy_total_json,
  rate: dsp_galaxy_rate_json,
}

self.onmessage = async (e: MessageEvent<{ id: number; fn: string; json: string }>) => {
  const { id, fn, json } = e.data
  let out: string
  try {
    await ready
    const f = FNS[fn]
    out = f ? f(json) : JSON.stringify({ error: `unknown function ${fn}` })
  } catch (err) {
    out = JSON.stringify({ error: String(err) })
  }
  ;(self as unknown as Worker).postMessage({ id, out })
}
