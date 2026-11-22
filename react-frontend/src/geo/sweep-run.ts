// Node driver for the p sweep (same code as the Map tab's worker): node src/geo/sweep-run.ts ITEM=RATE[,ITEM=RATE] BASE [pmax] [limit] [plan|base]
// Prints one line per event, so long runs show progress.
import fs from 'node:fs'
import highsLoader from 'highs'
import { highsSolver } from './highsLp.ts'
import { runSweep } from './sweep.ts'

const R = (p: string) => JSON.parse(fs.readFileSync(new URL('../../../' + p, import.meta.url), 'utf8'))
const [spec, base = 'S0_m1', pmax = '4', limit = '30'] = process.argv.slice(2)
const targets = Object.fromEntries(spec.split(',').map(s => { const [k, v] = s.split('='); return [k, +v] }))
const solve = highsSolver(await highsLoader())
let last = 0
runSweep(R('data/game-data.json'), { sitesDoc: R('data/satisfactory-geo/sites.json'), nodes: R('data/satisfactory-geo/nodes.json').nodes, transport: R('data/satisfactory-geo/transport.json') },
  solve, { targets, base, tier: 8, workKm: 0, pmax: +pmax, timeLimit: +limit, recipeIds: process.env.RECIPES ? process.env.RECIPES.split(',') : null }, e => {
    if (e.type === 'progress') { if (performance.now() - last < 5000) return; last = performance.now() }
    if (e.type === 'row') console.log('row', JSON.stringify(e.row))
    else if (e.type === 'meta') console.log('meta', e.meta.n_vars, e.meta.hubs.join(' '))
    else console.log(e.type, JSON.stringify(e))
  })
