// Map (地理) view for Satisfactory plans (report docs/research/satisfactory-geo/00-最终报告.md, step G6): the plan's
// targets are demanded at a base site; the siting MILP (geo/model.ts, a port of tools/geo_solve.py) chooses where to
// mine and process, belts / pipes per grid link, and rail lines between candidate hubs; a p sweep over the number of
// open hubs runs in a worker on highs-js. Drawn as a plain vector map from node coordinates (no game map images).
import { useEffect, useMemo, useRef, useState } from 'react'
import { dn, fmt, tierRecipes, type GameData, type Plan } from './solver'
import { t } from './i18n'
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

const statusText = (s: string) => (s === 'Optimal' ? t('proven optimal', '已证最优') : s === 'timeLimit' || s === 'interrupted' ? t('time limit', '达到时限') : s === 'stuck' ? t('solver stuck, skipped', '求解卡住,已跳过')
  : s === 'rejected' ? t('solution failed the re-check, dropped', '解未通过复核,已丢弃') : s)
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
  // recipes unlocked at the chosen tier (report G9)
  const atTier = useMemo(() => tierRecipes(p.tier), [p.tier])
  const opts: SweepOptions = {
    targets, base: p.base, tier: p.tier, workKm: p.workKm, pmax: p.pmax, timeLimit: p.timeLimit, recipeIds: p.recipes === 'plan' ? recipeIds : null,
    supplied: p.recipes === 'plan' && Object.keys(suppliedIn).length ? suppliedIn : null,
    tierRecipes: atTier,
  }
  const key = JSON.stringify(opts)
  const itemName = (id: string) => dn(data.items.find(i => i.id === id), data.game, id)
  const recipeName = (id: string) => dn(data.recipes.find(r => r.id === id), data.game, id)

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
    else if (now - lastMsg.current > (s.limit + 60) * 1000) stop(t('The solver stopped responding and was stopped.', '求解器无响应,已停止。'))
  }
  useEffect(() => {
    if (!active) return
    const t = setInterval(() => watch.current(), 1000)
    return () => clearInterval(t)
  }, [active])

  const skipStuck = (r: RunState, step: { fmult: number; p: number }) => {
    worker.current?.terminate()
    worker.current = null
    if (++restarts.current > 3) { stop(t('The solver got stuck repeatedly; the sweep was stopped.', '求解器多次卡住,扫描已停止。')); return }
    const envOf = (rows: SweepRow[]) => rows.filter(x => x.fmult === 1 && x.obj !== null).reduce<number | null>((m, x) => (m === null ? x.obj! : Math.min(m, x.obj!)), null)
    const stuck: SweepRow = {
      fmult: step.fmult, F: (r.meta?.F ?? 0) * step.fmult, p: step.p, obj: null, bound: null, gap: Infinity, status: 'stuck',
      time_s: r.solving!.limit + 20, envelope: step.fmult === 1 ? envOf(r.rows) : null, hubs: [], n_lines: 0, rail_flow: 0, belts: 0,
    }
    const rows = [...r.rows, stuck, ...(step.fmult === 1 && step.p === 0 ? [{ ...stuck, p: 1, copied: true }] : [])]
    const sols = new Map(r.sols)
    for (const x of rows) if (!sols.has(solKey(x.fmult, x.p))) sols.set(solKey(x.fmult, x.p), null)
    const next: RunState = { ...r, rows, sols, solving: { label: t('restarting', '重启'), step: null, since: performance.now(), limit: r.solving!.limit, bound: null, best: null } }
    setRun(next)
    launch({ ...JSON.parse(r.key) as SweepOptions, skip: rows.map(x => solKey(x.fmult, x.p)), envStart: envOf(rows) })
  }

  const start = () => {
    restarts.current = 0
    setPick(null); setSite(null)
    setRun({ key, rows: [], sols: new Map(), meta: null, solving: { label: t('loading HiGHS', '加载 HiGHS'), step: null, since: performance.now(), limit: p.timeLimit, bound: null, best: null }, done: false, stopped: false, error: null, ms: 0 })
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
      setRun(r => (r ? { ...r, solving: null, done: true, error: ev.message ? t(`The solver worker failed: ${ev.message}`, `求解线程出错:${ev.message}`) : t("The solver worker could not be loaded (offline, or the page is older than the site: reload it).", '求解线程无法加载 (离线,或页面比站点旧:请刷新)。') } : r))
    }
    w.postMessage({
      game: {
        items: data.items.map(i => ({ id: i.id, liquid: i.liquid })),
        recipes: data.recipes.map(r => ({ id: r.id, alt: r.alt, building: r.building, in: r.in, out: r.out })),
        resources: data.resources.map(r => ({ item: r.item })),
      },
      opts: { ...o, names: Object.fromEntries(data.items.map(i => [i.id, dn(i, data.game, i.name)])) },
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
      <p className="hint">{t('Where to mine and process this plan on the Satisfactory map: its targets are needed at the base site, ore comes from the nodes of 39 one-km grid sites, products travel by belt or pipe between neighbouring sites or by train between candidate hubs. The solver tries 0 … p hubs and keeps the cheapest plan (one-time build cost in raw-ore equivalents, OE). Machines are fractional here; the Plan tab rounds them.', '在幸福工厂地图上,这个方案该在哪里开采和加工:目标产物送到基地站点,原矿来自 39 个 1 km 网格站点的矿点,产品在相邻站点间走传送带或管道,在候选枢纽间走火车。求解器尝试 0 … p 个枢纽,保留成本最低的方案 (一次性建造成本,以原矿当量 OE 计)。这里的机器数是小数,方案页会取整。')}</p>
      <div className="geoctl">
        <label>{t('Base site', '基地站点')}
          <select aria-label={t('Base site', '基地站点')} value={p.base} onChange={e => setP({ ...p, base: e.target.value })}>
            {GRID.map(s => <option key={s.id} value={s.id}>{s.id}{SITES.hubs.some(h => h.site === s.id) ? t(' (hub)', ' (枢纽)') : ''}</option>)}
          </select></label>
        <label>{t('Tier', '层级')}
          <select aria-label={t('Tier', '层级')} value={p.tier} onChange={e => setP({ ...p, tier: +e.target.value })}>
            {[0, 2, 4, 5, 6, 7, 8, 9].map(tt => <option key={tt} value={tt}>T{tt}{tt < 6 ? t(' (no rail)', ' (无铁路)') : ''}</option>)}
          </select></label>
        <label>{t('Hubs up to', '枢纽上限')}
          <select aria-label={t('Hubs up to', '枢纽上限')} value={p.pmax} onChange={e => setP({ ...p, pmax: +e.target.value })}>
            {[2, 3, 4, 6, 8, 12].map(n => <option key={n} value={n}>{n}</option>)}
          </select></label>
        <label>{t('Player work', '玩家人工')}
          <select aria-label={t('Player work', '玩家人工')} value={p.workKm} onChange={e => setP({ ...p, workKm: +e.target.value })}>
            {[0, 250, 1000].map(n => <option key={n} value={n}>{n} OE/km</option>)}
          </select></label>
        <label>{t('Time per solve', '每次求解时间')}
          <select aria-label={t('Time per solve', '每次求解时间')} value={p.timeLimit} onChange={e => setP({ ...p, timeLimit: +e.target.value })}>
            {[10, 30, 60, 120].map(n => <option key={n} value={n}>{n} s</option>)}
          </select></label>
        <label>{t('Recipes', '配方')}
          <select aria-label={t('Recipes', '配方')} value={p.recipes} onChange={e => setP({ ...p, recipes: e.target.value as GeoPrefs['recipes'] })}>
            <option value="plan">{t("this plan's", '本方案的')}</option><option value="base">{t('base recipes', '基础配方')}</option>
          </select></label>
        {run?.solving ? <button onClick={() => stop()}>{t('Stop', '停止')}</button>
          : <button className="primary" disabled={noTargets} onClick={start}>{current && run?.rows.length ? t('Solve again', '重新求解') : t('Plan sites', '规划选址')}</button>}
      </div>
      {noTargets && <p className="muted">{t('Add a target to the plan first.', '请先给方案添加一个目标。')}</p>}
      {supplied && <p className="note small">{t(`Supplied inputs (${Object.keys(suppliedIn).map(itemName).join(', ')}) are taken at the base site, free, up to the rate the plan uses. Choose “base recipes” to make everything from ore.`, `外部供应的输入 (${Object.keys(suppliedIn).map(itemName).join('、')}) 在基地站点免费取用,上限为方案用量。选“基础配方”则全部从原矿做起。`)}</p>}
      {run && !current && run.rows.length > 0 && <p className="note small">{t('The result below is for other inputs (plan or map settings changed). Use “Plan sites” to solve again.', '下面的结果对应其他输入 (方案或地图设置已更改)。点“规划选址”重新求解。')}</p>}
      {run?.solving && <p className="solving" role="status">{t(`Solving ${run.solving.label}: ${fmt((performance.now() - run.solving.since) / 1000, 0)} s of ≤ ${run.solving.limit} s`, `求解中 ${run.solving.label}:${fmt((performance.now() - run.solving.since) / 1000, 0)} 秒 / 上限 ${run.solving.limit} 秒`)}{run.solving.best !== null ? t(` · best ${oe(run.solving.best)}`, ` · 当前最优 ${oe(run.solving.best)}`) : ''}{run.solving.bound !== null ? t(` · bound ${oe(run.solving.bound)}`, ` · 下界 ${oe(run.solving.bound)}`) : ''}</p>}
      {run?.error && <p className="warn small">{run.error}</p>}
      {run?.stopped && <p className="muted small">{t('Stopped; the points solved so far are kept.', '已停止;已求解的点会保留。')}</p>}
      {unsolved.length > 0 && <p className="warn small">{t(`Not solved: ${unsolved.map(r => `${r.fmult === 1 ? '' : `F ${r.fmult}×, `}p ≤ ${r.p} (${statusText(r.status)})`).join('; ')}. The best plan is the best of the solved points only; a longer time per solve may find more.`, `未求解:${unsolved.map(r => `${r.fmult === 1 ? '' : `F ${r.fmult}×, `}p ≤ ${r.p} (${statusText(r.status)})`).join(';')}。最优方案只取自已求解的点;加长每次求解时间可能找到更多。`)}</p>}

      <div className="geogrid">
        <div className="geomap">
          <WorldMap sol={sol} hubs={meta?.hubs ?? SITES.hubs.map(h => h.site)} base={meta?.base ?? p.base} site={site} setSite={setSite} />
          <Legend sol={sol} itemName={itemName} />
        </div>
        <div className="geoside">
          {!sol && !run?.solving && !run?.stopped && <p className="muted">{run?.done && pick ? t('No plan at this point.', '这个点没有方案。') : current && run?.done ? t('No plan found.', '没有找到方案。') : t('Choose the base site and press “Plan sites”.', '选好基地站点,再点“规划选址”。')}</p>}
          {sol && shownRow && meta && <SummaryPanel sol={sol} row={shownRow} beltOnly={beltOnly ?? null} meta={meta} itemName={itemName} />}
          {f1.length > 0 && <>
            <h2>{t('Cost by number of hubs', '按枢纽数的成本')}</h2>
            <SweepChart rows={f1} shown={shownKey} onPick={r => setPick(solKey(r.fmult, r.p))} />
            {noRail ? <p className="hint">{t(`Trains are not unlocked at tier ${meta!.tier}: there is one plan, by belt and pipe.`, `层级 ${meta!.tier} 还没解锁火车:只有一种方案,用传送带和管道。`)}</p>
              : <p className="hint">{t("Envelope cost(p) = cheapest plan with at most p open hubs; filled dots are proven optimal, hollow ones hit the time limit; the band is HiGHS's lower bound. p = 1 equals p = 0 (one hub cannot run a line). Tap a dot to show that plan.", '包络线 cost(p) = 最多开 p 个枢纽时的最便宜方案;实心点已证最优,空心点达到时限;色带是 HiGHS 的下界。p = 1 等同 p = 0 (一个枢纽跑不了一条线)。点一个点查看对应方案。')}</p>}
            {!noRail && <table className="small"><thead><tr><th>{t('Hub cost F', '枢纽成本 F')}</th><th className="num">{t('best p*', '最优 p*')}</th><th>{t('proof', '证明')}</th></tr></thead>
              <tbody>{sens.map(s => (
                <tr key={s.fm} className={pick === solKey(s.fm, p.pmax) && s.fm !== 1 ? 'on' : ''}>
                  <td>{s.fm}× {meta ? `(${fmt(meta.F * s.fm, 0)} OE)` : ''}{s.fm !== 1 && s.solved && <button className="linkbtn" onClick={() => setPick(solKey(s.fm, p.pmax))}>{t('show', '查看')}</button>}</td>
                  <td className="num">{s.p ?? (s.n && !run?.solving ? '—' : '…')}</td><td>{s.n ? (s.proven ? t('proven', '已证明') : s.solved ? t('not proven', '未证明') : t('not solved', '未求解')) : run?.solving ? t('pending', '待求解') : '—'}</td>
                </tr>))}</tbody></table>}
          </>}
          {sol && site && <SiteDetail id={site} sol={sol} itemName={itemName} recipeName={recipeName} onBase={() => setP({ ...p, base: site })} isBase={site === (meta?.base ?? p.base)} />}
          {sol && meta && <Links sol={sol} meta={meta} itemName={itemName} />}
          <TierPlan data={data} opts={opts} />
          {run?.done && meta && <p className="hint">{t(`Model: ${fmt(meta.n_vars, 0)} variables (${meta.n_int} integer), ${fmt(meta.n_rows, 0)} rows, ${meta.recipes.length} recipes; ${meta.hubs.length} candidate hubs; Mk.${meta.belt_tier} belts (${meta.belt_cap}/min, ${fmt(meta.belt_oe_km, 0)} OE/km), double track ${fmt(meta.track_oe_km, 0)} OE/km. Sweep ${fmt(run.ms / 1000, 0)} s.`, `模型:${fmt(meta.n_vars, 0)} 个变量 (${meta.n_int} 个整数),${fmt(meta.n_rows, 0)} 行,${meta.recipes.length} 个配方;${meta.hubs.length} 个候选枢纽;Mk.${meta.belt_tier} 传送带 (${meta.belt_cap}/min,${fmt(meta.belt_oe_km, 0)} OE/km),双轨 ${fmt(meta.track_oe_km, 0)} OE/km。扫描耗时 ${fmt(run.ms / 1000, 0)} 秒。`)}</p>}
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
  const parts: [string, number | undefined][] = [[t('Machines', '机器'), cp.x], [t('Mining + feed belts', '开采 + 进料传送带'), cp.e], [t('Belts', '传送带'), cp.b], [t('Pipes', '管道'), cp.bp], [t('Hubs (F)', '枢纽 (F)'), cp.h], [t('Track', '轨道'), cp.u], [t('Locomotives', '机车'), cp.z], [t('Cars + platforms', '车厢 + 站台'), (cp.c ?? 0) + (cp.cf ?? 0)]]
  return (
    <div className="geosum">
      <div className="cards">
        <div className="card"><span className="big">{sol.hubs.length}</span><span className="muted">{t('train hubs', '火车枢纽')}</span></div>
        <div className="card"><span className="big">{fmt(beltLen / 1000, 1)}<span className="unit"> km</span></span><span className="muted">{t(`belts (${sol.belt_count})`, `传送带 (${sol.belt_count})`)}</span></div>
        <div className="card"><span className="big">{fmt(track / 1000, 1)}<span className="unit"> km</span></span><span className="muted">{t(`rail (${cars} cars)`, `铁路 (${cars} 节车厢)`)}</span></div>
        <div className="card"><span className="big">{fmt((row.obj ?? 0) / 1000, 1)}<span className="unit">k OE</span></span><span className="muted">{t('build cost', '建造成本')}</span></div>
      </div>
      <p className="small">
        {row.fmult === 1 ? `p ≤ ${row.p}` : `F ${row.fmult}×, p ≤ ${row.p}`}: <b>{statusText(row.status)}</b>{row.status !== 'Optimal' && row.gap < 1 ? t(`, within ${fmt(row.gap * 100, 1)} % of the bound`, `,与下界相差 ${fmt(row.gap * 100, 1)} %`) : ''}{t('.', '。')}
        {row.fmult === 1 && sol.hubs.length === 0 && row.p > 0 && t(' No train pays off in this plan: it uses belts and pipes only.', ' 这个方案里火车不划算:只用传送带和管道。')}
        {saving !== null && sol.hubs.length > 0 && (saving > 1e-6 ? t(` Rail saves ${fmt(saving * 100, 1)} % against the best belts-only plan found (${oe(beltOnly!.obj)} OE${beltOnly!.status === 'Optimal' ? '' : ', not proven optimal'}).`, ` 铁路比找到的最佳纯传送带方案省 ${fmt(saving * 100, 1)} % (${oe(beltOnly!.obj)} OE${beltOnly!.status === 'Optimal' ? '' : ',未证最优'})。`)
          : t(` Not cheaper than the best belts-only plan found (${oe(beltOnly!.obj)} OE).`, ` 不比找到的最佳纯传送带方案便宜 (${oe(beltOnly!.obj)} OE)。`))}
        {sol.hubs.length > 0 && t(` Hubs ${sol.hubs.join(', ')}.`, ` 枢纽 ${sol.hubs.join('、')}。`)} {t(`${fmt(machines, 1)} machines, ${fmt(sol.rail_flow_items_per_min, 0)}/min by rail${pipeLen > 0 ? `, ${fmt(pipeLen / 1000, 1)} km of pipes (${sol.pipes.reduce((s, b) => s + b.n, 0)})` : ''}.`, `${fmt(machines, 1)} 台机器,铁路运量 ${fmt(sol.rail_flow_items_per_min, 0)}/min${pipeLen > 0 ? `,管道 ${fmt(pipeLen / 1000, 1)} km (${sol.pipes.reduce((s, b) => s + b.n, 0)} 段)` : ''}。`)}
      </p>
      {sol.cbar_binding && sol.cbar_binding.length > 0 && <p className="warn small">{t(`The car bound is reached on ${sol.cbar_binding.join(', ')}; the plan may need more cars than the model allowed.`, `${sol.cbar_binding.join('、')} 的车厢数达到上限;实际方案可能需要比模型允许的更多车厢。`)}</p>}
      <table className="small"><tbody>
        {parts.filter(([, v]) => v).map(([n, v]) => <tr key={n}><td>{n}</td><td className="num">{oe(v)}</td></tr>)}
        <tr><td><b>{t('Total', '合计')}</b></td><td className="num"><b>{oe(row.obj)}</b></td></tr>
      </tbody></table>
      {Object.keys(sol.extraction).length > 0 && <p className="hint">{t(`Mined: ${Object.entries(totalsByItem(sol.extraction)).map(([k, v]) => `${itemName(k)} ${fmt(v, 0)}`).join(' · ')} /min. Tier ${meta.tier}.`, `开采:${Object.entries(totalsByItem(sol.extraction)).map(([k, v]) => `${itemName(k)} ${fmt(v, 0)}`).join(' · ')} /min。层级 ${meta.tier}。`)}</p>}
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
      return { key: dir, mode: 'rail', dir: dir.replace('->', ' → '), n: t(`${d.cars + d.fluid_cars} cars`, `${d.cars + d.fluid_cars} 节车厢`), len: l.length_m, q, flow: d.flow, note: t(`belts would cost ${oe(belt)} OE`, `用传送带要 ${oe(belt)} OE`) }
    })),
    ...sol.belts.map(b => {
      const q = Object.values(b.flow).reduce((s, v) => s + v, 0)
      const th = thresholdBelts(meta, effStack(b.flow), b.length_m)
      return { key: `b${b.a}${b.b}`, mode: 'belt', dir: `${b.a} → ${b.b}`, n: `${b.n} × Mk.${meta.belt_tier}`, len: b.length_m, q, flow: b.flow, note: th ? t(`rail pays from ${fmt(th * meta.belt_cap, 0)}/min`, `流量 ≥ ${fmt(th * meta.belt_cap, 0)}/min 时铁路划算`) : t('rail never pays', '铁路始终不划算') }
    }),
    ...sol.pipes.map(b => ({ key: `p${b.a}${b.b}`, mode: 'pipe', dir: `${b.a} → ${b.b}`, n: t(`${b.n} pipes`, `${b.n} 段管道`), len: b.length_m, q: Object.values(b.flow).reduce((s, v) => s + v, 0), flow: b.flow, note: '' })),
  ]
  if (!rows.length) return <p className="hint">{t('No transport between sites: everything is made where it is needed.', '站点之间没有运输:所有东西都在用到的地方就地生产。')}</p>
  return (
    <>
      <h2>{t('Links', '连接')}</h2>
      <div className="tablewrap"><table className="small geolinks"><thead><tr><th>{t('Link', '连接')}</th><th>{t('Mode', '方式')}</th><th className="num">/min</th><th className="num">{t('Length', '长度')}</th></tr></thead>
        <tbody>{rows.map(r => (
          <tr key={r.key}>
            <td><div>{r.dir}</div><div className="muted">{flowText(r.flow)}</div></td>
            <td><div className={`mode ${r.mode}`}>{r.n}</div>{r.note && <div className="muted">{r.note}</div>}</td>
            <td className="num">{fmt(r.q, 0)}</td><td className="num">{km(r.len)}</td>
          </tr>))}</tbody></table></div>
      <p className="hint">{t('Break-even per belt link: the flow above which one rail line of that length would be cheaper on its own (two hubs, track, train, cars), same costs as the solver (report §8.5).', '每条传送带连接的盈亏平衡点:流量高于它时,同样长度的一条独立铁路线 (两个枢纽、轨道、火车、车厢) 更便宜,成本与求解器一致 (报告 §8.5)。')}</p>
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
      <h2>{t('Site', '站点')} {id}{isBase ? t(' · base', ' · 基地') : ''}</h2>
      <p className="small muted">{t(`${s.nodes.length} nodes, mean feed belt ${km(s.feed_dist_mean_m)}${sol.hubs.includes(id) ? ', train hub' : ''}.`, `${s.nodes.length} 个矿点,平均进料传送带 ${km(s.feed_dist_mean_m)}${sol.hubs.includes(id) ? ',火车枢纽' : ''}。`)} {!isBase && <button className="linkbtn" onClick={onBase}>{t('Make this the base', '设为基地')}</button>}</p>
      {Object.keys(mach).length > 0 && <><b className="small">{t('Machines', '机器')}</b><ul className="chips small">{Object.entries(mach).map(([r, n]) => <li key={r}>{recipeName(r)} <b>{fmt(n, 2)}</b></li>)}</ul></>}
      {Object.keys(ext).length > 0 && <><b className="small">{t('Mined /min', '开采 /min')}</b><ul className="chips small">{list(ext)}</ul></>}
      {Object.keys(inn).length > 0 && <><b className="small">{t('In /min', '输入 /min')}</b><ul className="chips small">{list(inn)}</ul></>}
      {Object.keys(out).length > 0 && <><b className="small">{t('Out /min', '输出 /min')}</b><ul className="chips small">{list(out)}</ul></>}
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
    <svg className="sweep" viewBox={`0 0 ${W} ${H}`} role="img" aria-label={t('Build cost by maximum number of hubs', '按枢纽数上限的建造成本')}>
      <polygon points={band} className="band" />
      {[lo + 0.1 * (hi - lo), (lo + hi) / 2, hi - 0.1 * (hi - lo)].map(v => <g key={v}><line x1={L} x2={W - R} y1={Y(v)} y2={Y(v)} className="gridl" /><text x={L - 4} y={Y(v) + 4} textAnchor="end">{fmt(v / 1000, 1)}k</text></g>)}
      <path d={line} className="env" />
      {pts.map(r => {
        const k = `${r.fmult}|${r.p}`
        return <g key={k} className="pt" onClick={() => onPick(r)}>
          <circle cx={X(r.p)} cy={Y(r.envelope!)} r={k === shown ? 6 : 4} className={r.status === 'Optimal' ? 'proven' : 'open'} />
          <circle cx={X(r.p)} cy={Y(r.envelope!)} r={12} fill="transparent" />
          <title>{r.obj === null ? `p ≤ ${r.p}: ${statusText(r.status)}` : `p ≤ ${r.p}: ${fmt(r.obj, 0)} OE, ${r.hubs.length} ${t('hubs', '个枢纽')}, ${statusText(r.status)}${r.copied ? t(' (same as p = 0)', ' (同 p = 0)') : ''}`}</title>
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
    <svg className="worldmap" viewBox={`${VB.x} ${VB.y} ${VB.w} ${VB.h}`} role="img" aria-label={t('Map of nodes, sites, belts and rail lines', '矿点、站点、传送带和铁路线地图')}
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
      <text x={4200} y={3680} className="axis" textAnchor="end">{t('km · north up', 'km · 上为北')}</text>
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
          <title>{`${s.id}${n ? t(`: ${fmt(n, 1)} machines`, `:${fmt(n, 1)} 台机器`) : ''}`}</title>
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
      <li><svg width="18" height="10"><line x1="1" y1="5" x2="17" y2="5" className="belt" strokeWidth="3" /></svg>{t('belt', '传送带')}</li>
      <li><svg width="18" height="10"><line x1="1" y1="5" x2="17" y2="5" className="pipe" strokeWidth="3" /></svg>{t('pipe', '管道')}</li>
      <li><svg width="18" height="10"><line x1="1" y1="5" x2="17" y2="5" className="rail" /></svg>{t('rail', '铁路')}</li>
      <li><svg width="12" height="12"><circle cx="6" cy="6" r="5" className="hub open" /></svg>{t('open hub', '启用枢纽')}</li>
      <li><svg width="12" height="12"><circle cx="6" cy="6" r="5" className="hub" /></svg>{t('candidate hub', '候选枢纽')}</li>
      <li><svg width="12" height="12" viewBox="-6 -6 12 12"><path d={star(0, 0, 6, 2.6)} className="base" /></svg>{t('base', '基地')}</li>
      <li><svg width="12" height="12"><circle cx="6" cy="6" r="5" className="mach" /></svg>{t('machines', '机器')}</li>
      {used.map(k => <li key={k}><svg width="10" height="10"><circle cx="5" cy="5" r="4" fill={RES_COLOR[k] ?? 'gray'} /></svg>{itemName(k)}</li>)}
    </ul>
  )
}

/** Multi-period build-out (report G9): the plan grows tier by tier; each period reuses what earlier periods built
 *  (free up to that amount, `BuildOptions.existing`) and pays only for the new part. One solve per period at the
 *  hub limit (no F sensitivity), the targets a share of the plan's. */
function TierPlan({ data, opts }: { data: GameData; opts: SweepOptions }) {
  const [tiers, setTiers] = useState('4, 6, 8')
  const [shares, setShares] = useState('25, 50, 100')
  type Period = { tier: number; share: number; newOe: number | null; total: number | null; sum: Summary | null; error: string | null; ms: number; status: string | null; gap: number | null }
  const [periods, setPeriods] = useState<Period[]>([])
  const [busy, setBusy] = useState(false)
  // the running period's worker and its promise's resolve (Stop ends both; review: a live watchdog resumed an old run)
  const stopRef = useRef<{ w: Worker; done: () => void } | null>(null)
  const runId = useRef(0)
  useEffect(() => () => { stopRef.current?.w.terminate(); stopRef.current?.done(); runId.current++ }, [])
  const nums = (x: string) => x.split(/[\s,，]+/).map(Number).filter(v => Number.isFinite(v) && v > 0)
  const ts = nums(tiers).map(v => Math.min(9, Math.max(0, Math.round(v))))
  const ss = nums(shares)
  const n = Math.min(ts.length, ss.length)
  const game = () => ({
    items: data.items.map(i => ({ id: i.id, liquid: i.liquid })),
    recipes: data.recipes.map(r => ({ id: r.id, alt: r.alt, building: r.building, in: r.in, out: r.out })),
    resources: data.resources.map(r => ({ item: r.item })),
  })
  type One = { best: Record<string, number> | null; obj: number | null; sum: Summary | null; error: string | null; ms: number; status: string | null; gap: number | null }
  const one = (o: SweepOptions) => new Promise<One>(resolve => {
    const w = new Worker(new URL('./geo/geoWorker.ts', import.meta.url), { type: 'module' })
    let settled = false
    const fail = (error: string): One => ({ best: null, obj: null, sum: null, error, ms: 0, status: null, gap: null })
    const res = (r: One) => { if (settled) return; settled = true; clearTimeout(dog); w.terminate(); stopRef.current = null; resolve(r) }
    stopRef.current = { w, done: () => res(fail(t('Stopped.', '已停止。'))) }
    // the solves stop themselves at the time limit; a worker silent far past it is stopped
    const dog = setTimeout(() => res(fail(t('The solver stopped responding.', '求解器无响应。'))), (o.timeLimit * (o.pmax + 1) + 120) * 1000)
    w.onmessage = (ev: MessageEvent<SweepEvent>) => {
      const e = ev.data
      if (e.type === 'done') res(e.best ? { best: e.best, obj: e.bestObj ?? null, sum: e.bestSummary ?? null, error: null, ms: e.ms, status: e.bestStatus ?? null, gap: e.bestGap ?? null }
        : fail(t('No plan was found within the time limit.', '时限内没有找到方案。')))
      if (e.type === 'error') res(fail(e.message))
    }
    w.onerror = ev => res(fail(ev.message || 'worker failed'))
    w.postMessage({ game: game(), opts: { ...o, names: Object.fromEntries(data.items.map(i => [i.id, dn(i, data.game, i.name)])) } })
  })
  const go = async () => {
    const id = ++runId.current
    setBusy(true)
    setPeriods([])
    let existing: Record<string, number> = {}
    let total = 0
    const out: Period[] = []
    for (let i = 0; i < n; i++) {
      const tier = ts[i], share = ss[i] / 100
      const o: SweepOptions = {
        ...opts, tier, fmults: [], existing: Object.keys(existing).length ? existing : null,
        targets: Object.fromEntries(Object.entries(opts.targets).map(([k, v]) => [k, v * share])),
        tierRecipes: tierRecipes(tier),
        supplied: opts.supplied ? Object.fromEntries(Object.entries(opts.supplied).map(([k, v]) => [k, v * share])) : null,
      }
      const r = await one(o)
      if (id !== runId.current) return // stopped, or a newer run started
      if (r.obj !== null) total += r.obj
      out.push({ tier, share: ss[i], newOe: r.obj, total: r.obj !== null ? total : null, sum: r.sum, error: r.error, ms: r.ms, status: r.status, gap: r.gap })
      setPeriods([...out])
      if (!r.best) break
      // what stands after this period: the larger of before and now, per variable
      const next = { ...existing }
      for (const [k, v] of Object.entries(r.best)) next[k] = Math.max(next[k] ?? 0, v)
      existing = next
    }
    if (id === runId.current) setBusy(false)
  }
  const stop = () => { runId.current++; stopRef.current?.done(); setBusy(false) }
  return (
    <details className="tierplan">
      <summary>{t('Build out tier by tier', '按科技层逐期扩张')}</summary>
      <p className="hint">{t('The plan grows with the tech tiers: each period plans a share of the targets with the recipes and transport of its tier, reuses what earlier periods built for free (old belts and pipes count as the capacity they carry), and pays only for the new part (ore equivalents; mining counts per item/min as in the single plan). Periods are solved one after another, each for its own targets (not looking ahead), at the hub limit, with the time limit per solve. Leaving old infrastructure idle costs nothing; dismantling (materials come back) and its work are not counted. With “this plan’s recipes”, an early tier fails when the plan uses a later recipe: choose base recipes.',
        '方案随科技层级逐期扩大：每一期按该层级的配方和运输方式规划目标的一部分，前几期建好的东西免费沿用（旧传送带和管道按它们实际的运力算），只为新增部分付费（原矿当量；开采按每分钟物品数计，与单期方案一致）。各期依次求解，每期只看自己的目标（不往后看），按枢纽上限、每次求解的时限。旧设施闲置不收费；拆除（材料返还）和拆除工作量不计。选「本方案的配方」时，若方案用到更高层级的配方，早期会失败：请改选基础配方。')}</p>
      <div className="row small">
        <label>{t('Tiers', '各期层级')} <input value={tiers} onChange={e => setTiers(e.target.value)} aria-label={t('Tiers of the periods', '各期层级')} /></label>
        <label>{t('Share of targets %', '目标比例 %')} <input value={shares} onChange={e => setShares(e.target.value)} aria-label={t('Share of the targets per period', '各期目标比例')} /></label>
        {!busy && <button className="tabbtn" disabled={n === 0} onClick={() => void go()}>{t(`Plan ${n} period${n === 1 ? '' : 's'}`, `规划 ${n} 期`)}</button>}
        {busy && <button className="tabbtn" onClick={stop}>{t('Stop', '停止')}</button>}
      </div>
      {periods.length > 0 && <div className="tablewrap"><table className="small">
        <thead><tr><th>{t('Period', '期')}</th><th>{t('Tier', '层级')}</th><th className="num">{t('Targets', '目标')}</th><th className="num">{t('New build OE', '新建 OE')}</th><th className="num">{t('Total OE', '累计 OE')}</th><th>{t('Hubs', '枢纽')}</th><th className="num">{t('Belts', '传送带')}</th></tr></thead>
        <tbody>{periods.map((q, i) => <tr key={i}>
          <td>{i + 1}</td><td>T{q.tier}</td><td className="num">{q.share}%</td>
          <td className="num" title={q.status && q.status !== 'Optimal' && q.gap !== null ? t(`not proven optimal: within ${fmt(q.gap * 100, 1)} % of the bound`, `未证最优：与下界相差 ${fmt(q.gap * 100, 1)} %`) : undefined}>
            {q.newOe !== null ? fmt(q.newOe, 0) : '—'}{q.status && q.status !== 'Optimal' && q.gap !== null ? ` (±${fmt(q.gap * 100, 0)}%)` : ''}</td>
          <td className="num">{q.total !== null ? fmt(q.total, 0) : '—'}</td>
          <td className="small">{q.error ?? (q.sum?.hubs.join(t(', ', '、')) || '—')}</td><td className="num">{q.sum ? q.sum.belt_count : '—'}</td>
        </tr>)}</tbody>
      </table></div>}
      {busy && <p className="solving" role="status">{t(`Solving period ${periods.length + 1} of ${n}…`, `正在求第 ${periods.length + 1}/${n} 期…`)}</p>}
    </details>
  )
}
