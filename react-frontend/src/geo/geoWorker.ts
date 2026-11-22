// Map view worker: loads highs-js (shared loader with the planner's HiGHS engine) and runs the siting p sweep,
// posting every event (meta, solving, progress, row, done / error). The page terminates the worker to stop it,
// and also when it has been silent for longer than the time limit + 30 s (watchdog in MapView).
//   in:  { game: GameLite, opts: SweepOptions }
//   out: SweepEvent
import { loadHighs } from '../highsEngine'
import { highsSolver } from './highsLp.ts'
import { runSweep, type SweepEvent, type SweepOptions } from './sweep.ts'
import type { GameLite, GeoNode, SitesDoc } from './model.ts'
import sitesDoc from '../../../data/satisfactory-geo/sites.json'
import nodesDoc from '../../../data/satisfactory-geo/nodes.json'
import transport from '../../../data/satisfactory-geo/transport.json'

const post = (e: SweepEvent) => (self as unknown as Worker).postMessage(e)

self.onmessage = async (ev: MessageEvent<{ game: GameLite; opts: SweepOptions }>) => {
  let highs
  try { highs = await loadHighs() } catch (err) { post({ type: 'error', message: `HiGHS could not be loaded: ${err}` }); return }
  runSweep(ev.data.game, { sitesDoc: sitesDoc as unknown as SitesDoc, nodes: nodesDoc.nodes as GeoNode[], transport }, highsSolver(highs), ev.data.opts, post)
}
