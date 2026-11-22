// Map (地理) view for Satisfactory plans (report docs/research/satisfactory-geo/00-最终报告.md, step G6): the plan's
// targets are demanded at a base site; the siting MILP (geo/model.ts, a port of tools/geo_solve.py) chooses where to
// mine and process, belts / pipes per grid link, and rail lines between candidate hubs; a p sweep over the number of
// open hubs runs in a worker on highs-js. Drawn as a plain vector map from node coordinates (no game map images).
import { useEffect, useMemo, useRef, useState } from 'react'
import { fmt, type GameData, type Plan } from './solver'
import { Icon } from './Icon'
import { haulCosts, stackOf, thresholdBelts, type GeoNode, type SitesDoc, type Summary } from './geo/model.ts'
import type { SweepEvent, SweepMeta, SweepOptions, SweepRow } from './geo/sweep.ts'
import './geo/map.css'
import sitesJson from '../../data/satisfactory-geo/sites.json'
import nodesJson from '../../data/satisfactory-geo/nodes.json'

const SITES = sitesJson as unknown as SitesDoc
const NODES = nodesJson.nodes as GeoNode[]
const SITE = new Map(SITES.sites.map(s => [s.id, s]))
const GRID = SITES.sites.filter(s => s.kind === 'grid').sort((a, b) => (a.id < b.id ? -1 : 1))
// world extent (satisfactory.wiki.gg/wiki/World): x −3246…4253 m, y −3750…3750 m; y grows south, so north is up
const VB = { x: -3300, y: -3800, w: 7600, h: 7600 }

// one hue per resource, readable on light and dark paper
const RES_COLOR: Record<string, string> = {
  Desc_OreIron_C: 'oklch(0.58 0.06 40)', Desc_OreCopper_C: 'oklch(0.66 0.13 55)', Desc_Stone_C: 'oklch(0.7 0.01 240)',
  Desc_Coal_C: 'oklch(0.45 0.01 240)', Desc_OreGold_C: 'oklch(0.8 0.14 95)', Desc_RawQuartz_C: 'oklch(0.68 0.13 340)',
  Desc_Sulfur_C: 'oklch(0.82 0.15 115)', Desc_OreBauxite_C: 'oklch(0.58 0.15 25)', Desc_OreUranium_C: 'oklch(0.68 0.17 145)',
  Desc_SAM_C: 'oklch(0.55 0.15 300)', Desc_LiquidOil_C: 'oklch(0.35 0.03 280)', Desc_Water_C: 'oklch(0.62 0.12 240)',
  Desc_NitrogenGas_C: 'oklch(0.72 0.1 200)', Desc_GeothermalEnergy_C: 'oklch(0.7 0.15 60)',
}

interface GeoPrefs { base: string; tier: number; workKm: number; pmax: number; timeLimit: number; recipes: 'plan' | 'base' }
// kept for the session, so switching tabs does not lose the inputs or the last sweep
let prefs: GeoPrefs = { base: SITES.hubs.find(h => h.reason === 'base')!.site, tier: 8, workKm: 0, pmax: 4, timeLimit: 30, recipes: 'plan' }
interface RunState {
  key: string; rows: SweepRow[]; sols: Map<string, Summary | null>; meta: SweepMeta | null
  solving: { label: string; step: { fmult: number; p: number } | null; since: number; limit: number; bound: number | null; best: number | null } | null
  done: boolean; stopped: boolean; error: string | null; ms: number
}
let last: RunState | null = null
const solKey = (fmult: number, p: number) => `${fmult}|${p}`

const statusText = (s: string) => (s === 'Optimal' ? 'proven optimal' : s === 'timeLimit' || s === 'interrupted' ? 'time limit' : s === 'stuck' ? 'solver stuck, skipped'
  : s === 'rejected' ? 'solution failed the re-check, dropped' : s)
const km = (m: number) => `${fmt(m / 1000, 1)} km`
const oe = (x: number | null | undefined) => (x === null || x === undefined ? '—' : fmt(x, 0))

export default function MapView({ data, plan }: { data: GameData; plan: Plan }) {
  const [p, setP] = useState<GeoPrefs>(prefs)
  useEffect(() => { prefs = p }, [p])
  const [run, setRun] = useState<RunState | null>(last)
  useEffect(() => { last = run }, [run])
  const [pick, setPick] = useState<string | null>(null) // chosen sweep point (solKey); null = best
  const [site, setSite] = useState<string | null>(null)
  const worker = useRef<Worker | null>(null)
  const lastMsg = useRef(0)
  const [, tick] = useState(0)

  const targets = useMemo(() => Object.fromEntries(plan.targets.filter(t => t.rate > 0).map(t => [t.item, t.rate])), [plan])
  // sorted, so the same recipe set in another order (the planner still refining) is the same input
  const recipeIds = useMemo(() => [...new Set(plan.recipes.map(r => r.id))].sort(), [plan])
  // the plan's supplied inputs are taken at the base site, free, up to the rate the plan uses
  const suppliedIn = useMemo(() => Object.fromEntries(plan.supplied.filter(t => t.rate > 0).map(t => [t.item, t.rate * (1 + 1e-6)])), [plan])
  const opts: SweepOptions = {
    targets, base: p.base, tier: p.tier, workKm: p.workKm, pmax: p.pmax, timeLimit: p.timeLimit, recipeIds: p.recipes === 'plan' ? recipeIds : null,
    supplied: p.recipes === 'plan' && Object.keys(suppliedIn).length ? suppliedIn : null,
  }
  const key = JSON.stringify(opts)
  const itemName = (id: string) => data.items.find(i => i.id === id)?.name ?? id
  const recipeName = (id: string) => data.recipes.find(r => r.id === id)?.name ?? id

  const runRef = useRef(run)
  runRef.current = run
  const restarts = useRef(0)
  const stop = (why: string | null = null) => {
    worker.current?.terminate()
    worker.current = null
    setRun(r => (r ? { ...r, solving: null, stopped: !r.done, error: why ?? r.error } : r))
  }
  // leaving the tab ends the sweep: the worker dies with the view, so the kept state must say "stopped", or the
  // tab would come back showing a solve that no worker is running
  useEffect(() => () => {
    if (worker.current) {
      worker.current.terminate(); worker.current = null
      if (last?.solving) last = { ...last, solving: null, stopped: !last.done }
    }
  }, [])
  // Watchdog. HiGHS stops itself at the time limit (and our interrupt callback 2 s later), but in the browser it
  // was seen to hang now and then where neither is reached. A solve running past its limit + 20 s is killed with
  // its worker: the point is recorded as not solved and the sweep goes on in a new worker. A worker silent for the
  // limit + 60 s (no step, no progress) is stopped.
  // (one interval while solving, reading state through refs: an effect re-created on every render would be cleared
  // before it fires whenever the page re-renders more often than once a second)
  const active = !!run?.solving
  const watch = useRef<() => void>(() => {})
  watch.current = () => {
    tick(n => n + 1)
    const r = runRef.current, s = r?.solving
    if (!worker.current || !r || !s) return
    const now = performance.now()
    if (s.step && now - s.since > (s.limit + 20) * 1000) skipStuck(r, s.step)
    else if (now - lastMsg.current > (s.limit + 60) * 1000) stop('The solver stopped responding and was stopped.')
  }
  useEffect(() => {
    if (!active) return
    const t = setInterval(() => watch.current(), 1000)
    return () => clearInterval(t)
  }, [active])

  const skipStuck = (r: RunState, step: { fmult: number; p: number }) => {
    worker.current?.terminate()
    worker.current = null
    if (++restarts.current > 3) { stop('The solver got stuck repeatedly; the sweep was stopped.'); return }
    const envOf = (rows: SweepRow[]) => rows.filter(x => x.fmult === 1 && x.obj !== null).reduce<number | null>((m, x) => (m === null ? x.obj! : Math.min(m, x.obj!)), null)
    const stuck: SweepRow = {
      fmult: step.fmult, F: (r.meta?.F ?? 0) * step.fmult, p: step.p, obj: null, bound: null, gap: Infinity, status: 'stuck',
      time_s: r.solving!.limit + 20, envelope: step.fmult === 1 ? envOf(r.rows) : null, hubs: [], n_lines: 0, rail_flow: 0, belts: 0,
    }
    const rows = [...r.rows, stuck, ...(step.fmult === 1 && step.p === 0 ? [{ ...stuck, p: 1, copied: true }] : [])]
    const sols = new Map(r.sols)
    for (const x of rows) if (!sols.has(solKey(x.fmult, x.p))) sols.set(solKey(x.fmult, x.p), null)
    const next: RunState = { ...r, rows, sols, solving: { label: 'restarting', step: null, since: performance.now(), limit: r.solving!.limit, bound: null, best: null } }
    setRun(next)
    launch({ ...JSON.parse(r.key) as SweepOptions, skip: rows.map(x => solKey(x.fmult, x.p)), envStart: envOf(rows) })
  }

  const start = () => {
    restarts.current = 0
    setPick(null); setSite(null)
    setRun({ key, rows: [], sols: new Map(), meta: null, solving: { label: 'loading HiGHS', step: null, since: performance.now(), limit: p.timeLimit, bound: null, best: null }, done: false, stopped: false, error: null, ms: 0 })
    launch(opts)
  }

  const launch = (o: SweepOptions) => {
    worker.current?.terminate()
    const w = new Worker(new URL('./geo/geoWorker.ts', import.meta.url), { type: 'module' })
    worker.current = w
    lastMsg.current = performance.now()
    w.onmessage = (ev: MessageEvent<SweepEvent>) => {
      const now = performance.now()
      lastMsg.current = now
      const e = ev.data
      // decide now whether this worker is the live one: the updater runs later, at render time, when the 'done'
      // below may already have cleared worker.current (the last row and 'done' often arrive in one batch)
      if (worker.current !== w) return
      setRun(r => {
        if (!r) return r
        switch (e.type) {
          case 'meta': return { ...r, meta: e.meta }
          case 'solving': return { ...r, solving: { label: e.label, step: { fmult: e.fmult, p: e.p }, since: now, limit: e.limit, bound: null, best: null } }
          case 'progress': return r.solving ? { ...r, solving: { ...r.solving, bound: e.bound, best: e.best } } : r
          case 'row': {
            const sols = new Map(r.sols); sols.set(solKey(e.row.fmult, e.row.p), e.summary)
            return { ...r, rows: [...r.rows, e.row], sols }
          }
          case 'done': return { ...r, solving: null, done: true, ms: e.ms }
          case 'error': return { ...r, solving: null, done: true, error: e.message }
          default: return r
        }
      })
      if (e.type === 'done' || e.type === 'error') { w.terminate(); if (worker.current === w) worker.current = null }
    }
    w.onerror = ev => {
      w.terminate()
      if (worker.current !== w) return
      worker.current = null
      setRun(r => (r ? { ...r, solving: null, done: true, error: ev.message ? `The solver worker failed: ${ev.message}` : "The solver worker could not be loaded (offline, or the page is older than the site: reload it)." } : r))
    }
    w.postMessage({
      game: {
        items: data.items.map(i => ({ id: i.id, liquid: i.liquid })),
        recipes: data.recipes.map(r => ({ id: r.id, alt: r.alt, building: r.building, in: r.in, out: r.out })),
        resources: data.resources.map(r => ({ item: r.item })),
      },
      opts: { ...o, names: Object.fromEntries(data.items.map(i => [i.id, i.name])) },
    })
  }

  const current = run && run.key === key
  const f1 = run?.rows.filter(r => r.fmult === 1) ?? []
  // best plan at F 1×: the lowest cost, fewest hubs on ties (the envelope's p*)
  const best = f1.reduce<SweepRow | null>((b, r) => (r.obj !== null && !r.copied && (!b || r.obj < b.obj! - 1e-6 * Math.abs(b.obj!)) ? r : b), null)
  const shownKey = pick ?? (best ? solKey(1, best.p) : null)
  const shownRow = run?.rows.find(r => solKey(r.fmult, r.p) === shownKey) ?? null
  const sol = shownKey ? run?.sols.get(shownKey) ?? null : null
  // belts-only reference: the cheapest solved plan without a hub at any p (a time-limited p = 0 solve is often beaten
  // by a later point that still opens no hub; comparing with p = 0 alone would call that difference a rail saving)
  const beltOnly = f1.reduce<SweepRow | null>((b, r) => (r.obj !== null && !r.copied && r.hubs.length === 0 && (!b || r.obj < b.obj!) ? r : b), null)
  const meta = run?.meta ?? null
  // number of hubs of the cheapest solved plan; null while nothing is solved (a stuck or dropped point has no plan)
  const pStar = (rows: SweepRow[]) => { const b = rows.reduce<SweepRow | null>((b, r) => (r.obj !== null && (!b || r.obj < b.obj! - 1e-6 * Math.abs(b.obj!)) ? r : b), null); return b ? b.hubs.length : null }
  const unsolved = (run?.rows ?? []).filter(r => r.obj === null && !r.copied)
  const noRail = !!meta && !meta.rail
  const sens = [0.5, 1, 2].map(fm => {
    const rows = run?.rows.filter(r => r.fmult === fm && !r.copied) ?? []
    return { fm, p: pStar(rows), proven: rows.length > 0 && rows.every(r => r.status === 'Optimal'), n: rows.length, solved: rows.some(r => r.obj !== null) }
  })

  const noTargets = !Object.keys(targets).length
  const supplied = Object.keys(suppliedIn).length > 0 && p.recipes === 'plan'

  return (
    <div className="geo">
      <p className="hint">Where to mine and process this plan on the Satisfactory map: its targets are needed at the base site, ore comes from the nodes of 39 one-km grid sites, products travel by belt or pipe between neighbouring sites or by train between candidate hubs. The solver tries 0 … p hubs and keeps the cheapest plan (one-time build cost in raw-ore equivalents, OE). Machines are fractional here; the Plan tab rounds them.</p>
      <div className="geoctl">
        <label>Base site
          <select aria-label="Base site" value={p.base} onChange={e => setP({ ...p, base: e.target.value })}>
            {GRID.map(s => <option key={s.id} value={s.id}>{s.id}{SITES.hubs.some(h => h.site === s.id) ? ' (hub)' : ''}</option>)}
          </select></label>
        <label>Tier
          <select aria-label="Tier" value={p.tier} onChange={e => setP({ ...p, tier: +e.target.value })}>
            {[0, 2, 4, 5, 6, 7, 8, 9].map(t => <option key={t} value={t}>T{t}{t < 6 ? ' (no rail)' : ''}</option>)}
          </select></label>
        <label>Hubs up to
          <select aria-label="Hubs up to" value={p.pmax} onChange={e => setP({ ...p, pmax: +e.target.value })}>
            {[2, 3, 4, 6, 8, 12].map(n => <option key={n} value={n}>{n}</option>)}
          </select></label>
        <label>Player work
          <select aria-label="Player work" value={p.workKm} onChange={e => setP({ ...p, workKm: +e.target.value })}>
            {[0, 250, 1000].map(n => <option key={n} value={n}>{n} OE/km</option>)}
          </select></label>
        <label>Time per solve
          <select aria-label="Time per solve" value={p.timeLimit} onChange={e => setP({ ...p, timeLimit: +e.target.value })}>
            {[10, 30, 60, 120].map(n => <option key={n} value={n}>{n} s</option>)}
          </select></label>
        <label>Recipes
          <select aria-label="Recipes" value={p.recipes} onChange={e => setP({ ...p, recipes: e.target.value as GeoPrefs['recipes'] })}>
            <option value="plan">this plan's</option><option value="base">base recipes</option>
          </select></label>
        {run?.solving ? <button onClick={() => stop()}>Stop</button>
          : <button className="primary" disabled={noTargets} onClick={start}>{current && run?.rows.length ? 'Solve again' : 'Plan sites'}</button>}
      </div>
      {noTargets && <p className="muted">Add a target to the plan first.</p>}
      {supplied && <p className="note small">Supplied inputs ({Object.keys(suppliedIn).map(itemName).join(', ')}) are taken at the base site, free, up to the rate the plan uses. Choose “base recipes” to make everything from ore.</p>}
      {run && !current && run.rows.length > 0 && <p className="note small">The result below is for other inputs (plan or map settings changed). Use “Plan sites” to solve again.</p>}
      {run?.solving && <p className="solving" role="status">Solving {run.solving.label}: {fmt((performance.now() - run.solving.since) / 1000, 0)} s of ≤ {run.solving.limit} s{run.solving.best !== null ? ` · best ${oe(run.solving.best)}` : ''}{run.solving.bound !== null ? ` · bound ${oe(run.solving.bound)}` : ''}</p>}
      {run?.error && <p className="warn small">{run.error}</p>}
      {run?.stopped && <p className="muted small">Stopped; the points solved so far are kept.</p>}
      {unsolved.length > 0 && <p className="warn small">Not solved: {unsolved.map(r => `${r.fmult === 1 ? '' : `F ${r.fmult}×, `}p ≤ ${r.p} (${statusText(r.status)})`).join('; ')}. The best plan is the best of the solved points only; a longer time per solve may find more.</p>}

      <div className="geogrid">
        <div className="geomap">
          <WorldMap sol={sol} hubs={meta?.hubs ?? SITES.hubs.map(h => h.site)} base={meta?.base ?? p.base} site={site} setSite={setSite} />
          <Legend sol={sol} itemName={itemName} />
        </div>
        <div className="geoside">
          {!sol && !run?.solving && !run?.stopped && <p className="muted">{run?.done && pick ? 'No plan at this point.' : current && run?.done ? 'No plan found.' : 'Choose the base site and press “Plan sites”.'}</p>}
          {sol && shownRow && meta && <SummaryPanel sol={sol} row={shownRow} beltOnly={beltOnly ?? null} meta={meta} itemName={itemName} />}
          {f1.length > 0 && <>
            <h2>Cost by number of hubs</h2>
            <SweepChart rows={f1} shown={shownKey} onPick={r => setPick(solKey(r.fmult, r.p))} />
            {noRail ? <p className="hint">Trains are not unlocked at tier {meta!.tier}: there is one plan, by belt and pipe.</p>
              : <p className="hint">Envelope cost(p) = cheapest plan with at most p open hubs; filled dots are proven optimal, hollow ones hit the time limit; the band is HiGHS's lower bound. p = 1 equals p = 0 (one hub cannot run a line). Tap a dot to show that plan.</p>}
            {!noRail && <table className="small"><thead><tr><th>Hub cost F</th><th className="num">best p*</th><th>proof</th></tr></thead>
              <tbody>{sens.map(s => (
                <tr key={s.fm} className={pick === solKey(s.fm, p.pmax) && s.fm !== 1 ? 'on' : ''}>
                  <td>{s.fm}× {meta ? `(${fmt(meta.F * s.fm, 0)} OE)` : ''}{s.fm !== 1 && s.solved && <button className="linkbtn" onClick={() => setPick(solKey(s.fm, p.pmax))}>show</button>}</td>
                  <td className="num">{s.p ?? (s.n && !run?.solving ? '—' : '…')}</td><td>{s.n ? (s.proven ? 'proven' : s.solved ? 'not proven' : 'not solved') : run?.solving ? 'pending' : '—'}</td>
                </tr>))}</tbody></table>}
          </>}
          {sol && site && <SiteDetail id={site} sol={sol} itemName={itemName} recipeName={recipeName} onBase={() => setP({ ...p, base: site })} isBase={site === (meta?.base ?? p.base)} />}
          {sol && meta && <Links sol={sol} meta={meta} itemName={itemName} />}
          {run?.done && meta && <p className="hint">Model: {fmt(meta.n_vars, 0)} variables ({meta.n_int} integer), {fmt(meta.n_rows, 0)} rows, {meta.recipes.length} recipes; {meta.hubs.length} candidate hubs; Mk.{meta.belt_tier} belts ({meta.belt_cap}/min, {fmt(meta.belt_oe_km, 0)} OE/km), double track {fmt(meta.track_oe_km, 0)} OE/km. Sweep {fmt(run.ms / 1000, 0)} s.</p>}
        </div>
      </div>
    </div>
  )
}

function SummaryPanel({ sol, row, beltOnly, meta, itemName }: { sol: Summary; row: SweepRow; beltOnly: SweepRow | null; meta: SweepMeta; itemName: (id: string) => string }) {
  const beltLen = sol.belts.reduce((s, b) => s + b.n * b.length_m, 0)
  const pipeLen = sol.pipes.reduce((s, b) => s + b.n * b.length_m, 0)
  const track = sol.lines.reduce((s, l) => s + l.length_m, 0)
  const dirs = sol.lines.flatMap(l => Object.values(l.dirs)).filter(d => d.cars + d.fluid_cars > 0)
  const cars = dirs.reduce((s, d) => s + d.cars + d.fluid_cars, 0)
  const machines = Object.values(sol.machines_total).reduce((s, n) => s + n, 0)
  const saving = beltOnly?.obj && row.obj !== null ? (beltOnly.obj - row.obj) / beltOnly.obj : null
  const cp = sol.cost_parts
  const parts: [string, number | undefined][] = [['Machines', cp.x], ['Mining + feed belts', cp.e], ['Belts', cp.b], ['Pipes', cp.bp], ['Hubs (F)', cp.h], ['Track', cp.u], ['Locomotives', cp.z], ['Cars + platforms', (cp.c ?? 0) + (cp.cf ?? 0)]]
  return (
    <div className="geosum">
      <div className="cards">
        <div className="card"><span className="big">{sol.hubs.length}</span><span className="muted">train hubs</span></div>
        <div className="card"><span className="big">{fmt(beltLen / 1000, 1)}<span className="unit"> km</span></span><span className="muted">belts ({sol.belt_count})</span></div>
        <div className="card"><span className="big">{fmt(track / 1000, 1)}<span className="unit"> km</span></span><span className="muted">rail ({cars} cars)</span></div>
        <div className="card"><span className="big">{fmt((row.obj ?? 0) / 1000, 1)}<span className="unit">k OE</span></span><span className="muted">build cost</span></div>
      </div>
      <p className="small">
        {row.fmult === 1 ? `p ≤ ${row.p}` : `F ${row.fmult}×, p ≤ ${row.p}`}: <b>{statusText(row.status)}</b>{row.status !== 'Optimal' && row.gap < 1 ? `, within ${fmt(row.gap * 100, 1)} % of the bound` : ''}.
        {row.fmult === 1 && sol.hubs.length === 0 && row.p > 0 && ' No train pays off in this plan: it uses belts and pipes only.'}
        {saving !== null && sol.hubs.length > 0 && (saving > 1e-6 ? ` Rail saves ${fmt(saving * 100, 1)} % against the best belts-only plan found (${oe(beltOnly!.obj)} OE${beltOnly!.status === 'Optimal' ? '' : ', not proven optimal'}).`
          : ` Not cheaper than the best belts-only plan found (${oe(beltOnly!.obj)} OE).`)}
        {sol.hubs.length > 0 && ` Hubs ${sol.hubs.join(', ')}.`} {fmt(machines, 1)} machines, {fmt(sol.rail_flow_items_per_min, 0)}/min by rail{pipeLen > 0 ? `, ${fmt(pipeLen / 1000, 1)} km of pipes (${sol.pipes.reduce((s, b) => s + b.n, 0)})` : ''}.
      </p>
      {sol.cbar_binding && sol.cbar_binding.length > 0 && <p className="warn small">The car bound is reached on {sol.cbar_binding.join(', ')}; the plan may need more cars than the model allowed.</p>}
      <table className="small"><tbody>
        {parts.filter(([, v]) => v).map(([n, v]) => <tr key={n}><td>{n}</td><td className="num">{oe(v)}</td></tr>)}
        <tr><td><b>Total</b></td><td className="num"><b>{oe(row.obj)}</b></td></tr>
      </tbody></table>
      {Object.keys(sol.extraction).length > 0 && <p className="hint">Mined: {Object.entries(totalsByItem(sol.extraction)).map(([k, v]) => `${itemName(k)} ${fmt(v, 0)}`).join(' · ')} /min. Tier {meta.tier}.</p>}
    </div>
  )
}
const totalsByItem = (by: Record<string, Record<string, number>>) => {
  const t: Record<string, number> = {}
  for (const m of Object.values(by)) for (const [k, v] of Object.entries(m)) t[k] = (t[k] ?? 0) + v
  return t
}

/** Every belt / pipe / rail link with its flow, and the closed-form break-even of report §8.5 for that length. */
function Links({ sol, meta, itemName }: { sol: Summary; meta: SweepMeta; itemName: (id: string) => string }) {
  const flowText = (f: Record<string, number>) => Object.entries(f).sort((a, b) => b[1] - a[1]).map(([k, v]) => `${itemName(k)} ${fmt(v, 0)}`).join(', ')
  const effStack = (f: Record<string, number>) => { const q = Object.values(f).reduce((s, v) => s + v, 0); const st = Object.entries(f).reduce((s, [k, v]) => s + v / stackOf(k), 0); return st > 0 ? q / st : 100 }
  const rows = [
    ...sol.lines.flatMap(l => Object.entries(l.dirs).filter(([, d]) => d.cars + d.fluid_cars > 0).map(([dir, d]) => {
      const q = Object.values(d.flow).reduce((s, v) => s + v, 0)
      const [belt] = haulCosts(meta, q, effStack(d.flow), l.length_m)
      return { key: dir, mode: 'rail', dir: dir.replace('->', ' → '), n: `${d.cars + d.fluid_cars} cars`, len: l.length_m, q, flow: d.flow, note: `belts would cost ${oe(belt)} OE` }
    })),
    ...sol.belts.map(b => {
      const q = Object.values(b.flow).reduce((s, v) => s + v, 0)
      const th = thresholdBelts(meta, effStack(b.flow), b.length_m)
      return { key: `b${b.a}${b.b}`, mode: 'belt', dir: `${b.a} → ${b.b}`, n: `${b.n} × Mk.${meta.belt_tier}`, len: b.length_m, q, flow: b.flow, note: th ? `rail pays from ${fmt(th * meta.belt_cap, 0)}/min` : 'rail never pays' }
    }),
    ...sol.pipes.map(b => ({ key: `p${b.a}${b.b}`, mode: 'pipe', dir: `${b.a} → ${b.b}`, n: `${b.n} pipes`, len: b.length_m, q: Object.values(b.flow).reduce((s, v) => s + v, 0), flow: b.flow, note: '' })),
  ]
  if (!rows.length) return <p className="hint">No transport between sites: everything is made where it is needed.</p>
  return (
    <>
      <h2>Links</h2>
      <div className="tablewrap"><table className="small geolinks"><thead><tr><th>Link</th><th>Mode</th><th className="num">/min</th><th className="num">Length</th></tr></thead>
        <tbody>{rows.map(r => (
          <tr key={r.key}>
            <td><div>{r.dir}</div><div className="muted">{flowText(r.flow)}</div></td>
            <td><div className={`mode ${r.mode}`}>{r.n}</div>{r.note && <div className="muted">{r.note}</div>}</td>
            <td className="num">{fmt(r.q, 0)}</td><td className="num">{km(r.len)}</td>
          </tr>))}</tbody></table></div>
      <p className="hint">Break-even per belt link: the flow above which one rail line of that length would be cheaper on its own (two hubs, track, train, cars), same costs as the solver (report §8.5).</p>
    </>
  )
}

function SiteDetail({ id, sol, itemName, recipeName, onBase, isBase }: { id: string; sol: Summary; itemName: (id: string) => string; recipeName: (id: string) => string; onBase: () => void; isBase: boolean }) {
  const s = SITE.get(id)!
  const mach = sol.machines[id] ?? {}, ext = sol.extraction[id] ?? {}
  const inn: Record<string, number> = {}, out: Record<string, number> = {}
  const add = (t: Record<string, number>, f: Record<string, number>) => { for (const [k, v] of Object.entries(f)) t[k] = (t[k] ?? 0) + v }
  for (const b of [...sol.belts, ...sol.pipes]) { if (b.b === id) add(inn, b.flow); if (b.a === id) add(out, b.flow) }
  for (const l of sol.lines) for (const [dir, d] of Object.entries(l.dirs)) { const [o, t] = dir.split('->'); if (t === id) add(inn, d.flow); if (o === id) add(out, d.flow) }
  const list = (t: Record<string, number>) => Object.entries(t).sort((a, b) => b[1] - a[1]).map(([k, v]) => <li key={k}><Icon id={k} size={16} />{itemName(k)} <b>{fmt(v, 1)}</b></li>)
  return (
    <section className="geosite">
      <h2>Site {id}{isBase ? ' · base' : ''}</h2>
      <p className="small muted">{s.nodes.length} nodes, mean feed belt {km(s.feed_dist_mean_m)}{sol.hubs.includes(id) ? ', train hub' : ''}. {!isBase && <button className="linkbtn" onClick={onBase}>Make this the base</button>}</p>
      {Object.keys(mach).length > 0 && <><b className="small">Machines</b><ul className="chips small">{Object.entries(mach).map(([r, n]) => <li key={r}>{recipeName(r)} <b>{fmt(n, 2)}</b></li>)}</ul></>}
      {Object.keys(ext).length > 0 && <><b className="small">Mined /min</b><ul className="chips small">{list(ext)}</ul></>}
      {Object.keys(inn).length > 0 && <><b className="small">In /min</b><ul className="chips small">{list(inn)}</ul></>}
      {Object.keys(out).length > 0 && <><b className="small">Out /min</b><ul className="chips small">{list(out)}</ul></>}
    </section>
  )
}

function SweepChart({ rows, shown, onPick }: { rows: SweepRow[]; shown: string | null; onPick: (r: SweepRow) => void }) {
  const W = 320, H = 130, L = 52, R = 10, T = 10, B = 24
  const pts = rows.filter(r => r.envelope !== null)
  if (!pts.length) return null
  const ps = rows.map(r => r.p), pmax = Math.max(...ps, 1)
  const vals = pts.flatMap(r => [r.envelope!, ...(r.bound !== null ? [Math.min(r.bound, r.envelope!)] : [])])
  let lo = Math.min(...vals), hi = Math.max(...vals)
  const span = Math.max(hi - lo, 0.02 * hi)
  const mid = (hi + lo) / 2; lo = mid - 0.6 * span; hi = mid + 0.6 * span
  const X = (p: number) => L + (p / pmax) * (W - L - R)
  const Y = (v: number) => T + (1 - (v - lo) / (hi - lo)) * (H - T - B)
  const line = pts.map((r, i) => `${i ? 'L' : 'M'}${X(r.p)},${Y(r.envelope!)}`).join(' ')
  const band = [...pts.map(r => `${X(r.p)},${Y(r.envelope!)}`), ...pts.slice().reverse().map(r => `${X(r.p)},${Y(Math.min(r.bound ?? r.envelope!, r.envelope!))}`)].join(' ')
  return (
    <svg className="sweep" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Build cost by maximum number of hubs">
      <polygon points={band} className="band" />
      {[lo + 0.1 * (hi - lo), (lo + hi) / 2, hi - 0.1 * (hi - lo)].map(v => <g key={v}><line x1={L} x2={W - R} y1={Y(v)} y2={Y(v)} className="gridl" /><text x={L - 4} y={Y(v) + 4} textAnchor="end">{fmt(v / 1000, 1)}k</text></g>)}
      <path d={line} className="env" />
      {pts.map(r => {
        const k = `${r.fmult}|${r.p}`
        return <g key={k} className="pt" onClick={() => onPick(r)}>
          <circle cx={X(r.p)} cy={Y(r.envelope!)} r={k === shown ? 6 : 4} className={r.status === 'Optimal' ? 'proven' : 'open'} />
          <circle cx={X(r.p)} cy={Y(r.envelope!)} r={12} fill="transparent" />
          <title>{r.obj === null ? `p ≤ ${r.p}: ${statusText(r.status)}` : `p ≤ ${r.p}: ${fmt(r.obj, 0)} OE, ${r.hubs.length} hubs, ${statusText(r.status)}${r.copied ? ' (same as p = 0)' : ''}`}</title>
        </g>
      })}
      {ps.map(p => <text key={p} x={X(p)} y={H - 6} textAnchor="middle">{p}</text>)}
    </svg>
  )
}

function WorldMap({ sol, hubs, base, site, setSite }: { sol: Summary | null; hubs: string[]; base: string; site: string | null; setSite: (s: string | null) => void }) {
  const used = new Set(sol ? Object.values(sol.extraction).flatMap(e => Object.keys(e)) : [])
  const machines = (id: string) => (sol?.machines[id] ? Object.values(sol.machines[id]).reduce((s, n) => s + n, 0) : 0)
  const grid: number[] = []
  for (let g = -3000; g <= 4000; g += 1000) grid.push(g)
  const seg = (a: string, b: string, shrink = 140) => {
    const A = SITE.get(a)!, B = SITE.get(b)!
    const dx = B.x - A.x, dy = B.y - A.y, d = Math.hypot(dx, dy) || 1
    return { x1: A.x + dx / d * shrink, y1: A.y + dy / d * shrink, x2: B.x - dx / d * shrink, y2: B.y - dy / d * shrink }
  }
  return (
    <svg className="worldmap" viewBox={`${VB.x} ${VB.y} ${VB.w} ${VB.h}`} role="img" aria-label="Map of nodes, sites, belts and rail lines"
      onClick={e => { if (e.target === e.currentTarget) setSite(null) }}>
      <defs>
        <marker id="arrow-belt" viewBox="0 0 10 10" refX="8" refY="5" markerUnits="userSpaceOnUse" markerWidth="170" markerHeight="170" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" className="m-belt" /></marker>
        <marker id="arrow-pipe" viewBox="0 0 10 10" refX="8" refY="5" markerUnits="userSpaceOnUse" markerWidth="170" markerHeight="170" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" className="m-pipe" /></marker>
        <marker id="arrow-rail" viewBox="0 0 10 10" refX="8" refY="5" markerUnits="userSpaceOnUse" markerWidth="220" markerHeight="220" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" className="m-rail" /></marker>
      </defs>
      <rect x={-3246} y={-3750} width={7499} height={7500} className="world" />
      {grid.map(g => <line key={`v${g}`} x1={g} x2={g} y1={-3750} y2={3750} className="gridl" />)}
      {grid.filter(g => g <= 3000).map(g => <line key={`h${g}`} y1={g} y2={g} x1={-3246} x2={4253} className="gridl" />)}
      {grid.map(g => <text key={`tx${g}`} x={g} y={-3600} className="axis" textAnchor="middle">{g / 1000}</text>)}
      {grid.filter(g => g <= 3000).map(g => <text key={`ty${g}`} x={-3180} y={g + 40} className="axis">{g / 1000}</text>)}
      <text x={4200} y={3680} className="axis" textAnchor="end">km · north up</text>
      {NODES.filter(n => n.type !== 'deposit').map(n => (
        <circle key={n.id} cx={n.x} cy={n.y} r={used.has(n.item) ? 34 : 22} fill={RES_COLOR[n.item] ?? 'gray'} className={used.size && !used.has(n.item) ? 'node dim' : 'node'} />
      ))}
      {sol?.belts.map(b => <line key={`b${b.a}${b.b}`} {...seg(b.a, b.b)} className="belt" strokeWidth={Math.min(1.5 + 0.9 * b.n, 9)} markerEnd="url(#arrow-belt)" />)}
      {sol?.pipes.map(b => <line key={`p${b.a}${b.b}`} {...seg(b.a, b.b)} className="pipe" strokeWidth={Math.min(1.5 + 0.9 * b.n, 9)} markerEnd="url(#arrow-pipe)" />)}
      {sol?.lines.map(l => Object.entries(l.dirs).filter(([, d]) => d.cars + d.fluid_cars > 0).map(([dir]) => {
        const [o, t] = dir.split('->')
        return <line key={`r${dir}`} {...seg(o, t, 220)} className="rail" markerEnd="url(#arrow-rail)" />
      }))}
      {hubs.map(h => { const s = SITE.get(h)!; return <circle key={`h${h}`} cx={s.x} cy={s.y} r={150} className={sol?.hubs.includes(h) ? 'hub open' : 'hub'} /> })}
      {SITES.sites.map(s => {
        const n = machines(s.id), active = n > 0 || !!sol?.extraction[s.id]
        return <g key={s.id} className="site" onClick={() => setSite(s.id === site ? null : s.id)}>
          <rect x={s.x - 45} y={s.y - 45} width={90} height={90} className={active ? 'sq act' : 'sq'} />
          {n > 0 && <circle cx={s.x} cy={s.y} r={Math.min(60 + 12 * Math.sqrt(n), 420)} className="mach" />}
          <circle cx={s.x} cy={s.y} r={260} fill="transparent" />
          <title>{`${s.id}${n ? `: ${fmt(n, 1)} machines` : ''}`}</title>
        </g>
      })}
      {(() => { const s = SITE.get(base); return s ? <path d={star(s.x, s.y, 190, 80)} className="base" /> : null })()}
      {sol?.hubs.map(h => { const s = SITE.get(h)!; return <text key={`l${h}`} x={s.x + 180} y={s.y - 170} className="lbl">{h}</text> })}
      {site && (() => { const s = SITE.get(site)!; return <circle cx={s.x} cy={s.y} r={230} className="sel" /> })()}
    </svg>
  )
}
const star = (x: number, y: number, R: number, r: number) =>
  Array.from({ length: 10 }, (_, i) => { const a = -Math.PI / 2 + i * Math.PI / 5, q = i % 2 ? r : R; return `${i ? 'L' : 'M'}${x + q * Math.cos(a)},${y + q * Math.sin(a)}` }).join(' ') + 'Z'

function Legend({ sol, itemName }: { sol: Summary | null; itemName: (id: string) => string }) {
  const used = sol ? Object.keys(totalsByItem(sol.extraction)) : []
  return (
    <ul className="chips small geolegend">
      <li><svg width="18" height="10"><line x1="1" y1="5" x2="17" y2="5" className="belt" strokeWidth="3" /></svg>belt</li>
      <li><svg width="18" height="10"><line x1="1" y1="5" x2="17" y2="5" className="pipe" strokeWidth="3" /></svg>pipe</li>
      <li><svg width="18" height="10"><line x1="1" y1="5" x2="17" y2="5" className="rail" /></svg>rail</li>
      <li><svg width="12" height="12"><circle cx="6" cy="6" r="5" className="hub open" /></svg>open hub</li>
      <li><svg width="12" height="12"><circle cx="6" cy="6" r="5" className="hub" /></svg>candidate hub</li>
      <li><svg width="12" height="12" viewBox="-6 -6 12 12"><path d={star(0, 0, 6, 2.6)} className="base" /></svg>base</li>
      <li><svg width="12" height="12"><circle cx="6" cy="6" r="5" className="mach" /></svg>machines</li>
      {used.map(k => <li key={k}><svg width="10" height="10"><circle cx="5" cy="5" r="4" fill={RES_COLOR[k] ?? 'gray'} /></svg>{itemName(k)}</li>)}
    </ul>
  )
}
