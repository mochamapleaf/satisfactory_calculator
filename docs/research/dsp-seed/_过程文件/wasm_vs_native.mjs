// Compare the WASM generator with the native prototype sample, planet by planet.
import { readFileSync } from 'node:fs'
const pkg = process.argv[2], sample = process.argv[3]
const m = await import(`file:///${pkg}/satisfactory_calculator.js`)
m.initSync({ module: readFileSync(`${pkg}/satisfactory_calculator_bg.wasm`) })
const g = JSON.parse(m.dsp_galaxy_json(JSON.stringify({ seed: 12345678, stars: 32, resource_multiplier: 0.1 })))
const lines = readFileSync(sample, 'utf8').split(/\r?\n/)
const NAMES = { '铁': 'iron-ore', '铜': 'copper-ore', '硅': 'silicon-ore', '钛': 'titanium-ore', '石': 'stone', '煤': 'coal', '可燃冰': 'fire-ice', '金伯利': 'kimberlite-ore', '分形硅': 'fractal-silicon', '有机晶体': 'organic-crystal', '光栅石': 'optical-grating-crystal', '刺笋': 'spiniform-stalagmite-crystal', '单极磁石': 'unipolar-magnet' }
let star = -1, diffs = 0, compared = 0
for (const l of lines) {
  const s = l.match(/^★(\d+)/)
  if (s) { star = +s[1]; continue }
  const p = l.match(/^\s+P(\d+)\s/)
  if (!p) continue
  const pl = g.stars[star].planets[+p[1]]
  const want = {}
  for (const [, k, v] of l.matchAll(/(铁|铜|硅|钛|石|煤|可燃冰|金伯利|分形硅|有机晶体|光栅石|刺笋|单极磁石) (\d+)/g)) want[NAMES[k]] = +v
  const oil = l.match(/原油涌泉 ([\d.]+)\/s/)
  if (oil) want['crude-oil'] = +oil[1]
  for (const [k, v] of Object.entries(want)) {
    compared++
    const got = pl.veins.find(x => x.item === k)
    const gv = got ? (k === 'crude-oil' ? +(got.amount * 4e-5).toFixed(2) : got.amount) : 0
    if (gv !== v) { diffs++; console.log(`star ${star} P${p[1]} ${k}: native ${v} wasm ${gv}`) }
  }
}
console.log(`${compared} values compared, ${diffs} differ; wasm oil total ${g.oil_rate.toFixed(2)}/s`)
