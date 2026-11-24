import { lazy, Suspense, useMemo, useEffect, useRef, useState } from 'react'
import { AUTO_SOLVE_COLUMNS, cancelSolve, keepSolving, MAX_TIME_MS, fmt, GAMES, loadSolver, solveStream, type Game, type GameData, type Plan, type Settings, type SolveUpdate } from './solver'
import { loadGame, loadSettings, saveSettings } from './urlState'
import Sidebar from './Sidebar'
import Results from './Results'
import TimeInput from './TimeInput'
import FlowGraph from './FlowGraph'
import ItemValues from './ItemValues'
import ItemFlow from './ItemFlow'
import MixedBelt from './MixedBelt'
import { ICON_STYLES, iconStyleName, IconProvider, loadIconStyle, saveIconStyle, type IconStyle } from './Icon'
import { lang, setLang, t, type Lang } from './i18n'

// the map view (Satisfactory only) and its geo data load on first use
const MapView = lazy(() => import('./MapView'))
// the DSP galaxy tab (seed generator + its worker) loads on first use too
const GalaxyTab = lazy(() => import('./Galaxy'))

type Tab = 'table' | 'flow' | 'graph' | 'values' | 'mixed' | 'galaxy' | 'map'
const tabLabel = (id: Tab): string => ({ table: t('Plan', '规划'), flow: t('Item flow', '物流'), graph: t('Machine graph', '机器图'), values: t('Item values', '物品价值'), mixed: t('Mixed belts', '混带'), galaxy: t('Galaxy', '星区'), map: t('Map', '地图') })[id]
const TABS: Tab[] = ['table', 'flow', 'graph', 'values', 'mixed', 'galaxy', 'map']
const gameName = (id: Game, en: string): string => ({ satisfactory: t('Satisfactory', '幸福工厂'), dsp: t('Dyson Sphere Program', '戴森球计划'), factorio: t('Factorio', '异星工厂') } as Record<string, string>)[id] ?? en

export default function App() {
  const [data, setData] = useState<GameData | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [settings, setSettings] = useState<Settings>(loadSettings)
  const [tabSel, setTab] = useState<Tab>('table')
  const [iconStyle, setIconStyle] = useState<IconStyle>(loadIconStyle)
  // interface language: the page under `key={lng}` remounts so every `t(...)` reads the new one
  const [lng, setLng] = useState<Lang>(lang)
  // names in the plan (mode labels, notes) come from the solver: solve again in the new language,
  // unless that would drop a large model the user started by hand (review: a minutes-long solve)
  const switchLang = (l: Lang) => {
    setLang(l)
    setLng(l)
    if (approved !== key) setRerun(n => n + 1)
  }
  useEffect(() => saveIconStyle(iconStyle), [iconStyle])

  useEffect(() => {
    setData(null)
    show(null, '')
    loadSolver(settings.game, settings.dataset).then(setData).catch(e => setError(String(e)))
  }, [settings.game, settings.dataset])
  const switchGame = (g: Game) => setSettings(() => loadGame(g))
  useEffect(() => saveSettings(settings), [settings])
  // a plan link pasted into an open tab (saveSettings uses replaceState, which does not fire this)
  useEffect(() => {
    const onHash = () => { if (location.hash.length > 1) setSettings(loadSettings()) }
    window.addEventListener('hashchange', onHash)
    return () => window.removeEventListener('hashchange', onHash)
  }, [])

  const [plan, setPlan] = useState<Plan | null>(null)
  // The plan on screen and the model it solves (the settings without time limit and engine, which
  // change the search, not the model). Read by the stream handlers, which must decide at once.
  const cur = useRef<{ plan: Plan | null; model: string }>({ plan: null, model: '' })
  const show = (p: Plan | null, model = cur.current.model) => { cur.current = { plan: p, model }; setPlan(p) }
  // newest solve request: updates still arriving from an older one are ignored (review 4 M2)
  const live = useRef(0)
  const [planKey, setPlanKey] = useState('') // settings the shown plan was solved for
  const [solving, setSolving] = useState(false)
  // Large models (e.g. Factorio "recycle everything", ~16 000 columns, minutes in WASM) are not
  // solved automatically, also when they come from saved settings or a link: the solver returns
  // their size and the user starts the solve (`approved` = the settings it was started for).
  // the mixed-belt tab's settings are not part of the solve: changing them only re-runs the layout
  const solveSettings: Partial<Settings> = { ...settings }
  delete solveSettings.belt_layout
  const key = JSON.stringify(solveSettings)
  const modelKey = JSON.stringify({ ...solveSettings, time_limit_ms: 0, engine: 'auto' })
  const [big, setBig] = useState<{ columns: number; rows: number; key: string } | null>(null)
  const [approved, setApproved] = useState<string | null>(null)
  const [stopped, setStopped] = useState(false)
  const [started, setStarted] = useState(0)
  const [now, setNow] = useState(0)
  const [rerun, setRerun] = useState(0) // bumped by "Solve again" to re-run unchanged settings
  const [extra, setExtra] = useState(0) // extra ms for the next run ("Keep solving" when the search is gone)
  const [runId, setRunId] = useState(0)
  // streamed progress of the current / last whole-building solve
  const [progress, setProgress] = useState<SolveUpdate | null>(null)
  const bigRun = solving && approved === key
  useEffect(() => {
    if (!solving) return
    const t = setInterval(() => setNow(Date.now()), 250)
    return () => clearInterval(t)
  }, [solving])
  /** The plan's gap to a (valid, same-model) lower bound, when that is smaller. */
  const tighten = (p: Plan | null, bound: number | undefined): Plan | null => {
    if (!p || p.proven_optimal || p.status !== 'optimal' || bound === undefined || !Number.isFinite(bound)) return p
    const g = Math.max(0, (p.objective - bound) / Math.max(Math.abs(p.objective), 1e-10))
    if (g >= (p.gap ?? Infinity)) return p
    // the gap in buildings shrinks with the gap (same plan, same cheapest building)
    const gb = p.gap_buildings != null && p.gap ? p.gap_buildings * g / p.gap : p.gap_buildings
    return { ...p, gap: g, gap_buildings: gb }
  }
  // one handler per solve: plans and progress as they stream in
  const onUpdate = (k: string, model: string, more: number, token: number) => (u: SolveUpdate) => {
    if (token !== live.current) return
    if (u.gone) { setExtra(more); setRerun(n => n + 1); return }
    if (u.plan) {
      const p = u.plan
      if (p.status === 'too_large') {
        setBig({ columns: p.columns ?? 0, rows: p.rows ?? 0, key: k })
        show(null, '')
      } else {
        setBig(null)
        // A search started again for the same model (Keep solving after its worker was gone, a new
        // time limit, another engine) never shows a dearer plan than the one in hand.
        const prev = cur.current.model === model ? cur.current.plan : null
        const keep = !!prev && prev.status === 'optimal' && p.status === 'optimal' && !p.proven_optimal
          && prev.objective < p.objective - 1e-9 * Math.max(1, Math.abs(p.objective))
        show(keep ? tighten(prev, u.bound) : p, model)
        setPlanKey(k)
      }
    } else if (u.bound !== undefined) {
      // the bound moved: same plan, smaller gap
      show(tighten(cur.current.plan, u.bound))
    }
    setProgress(u)
    if (u.done) { setSolving(false); setNow(Date.now()) }
  }
  useEffect(() => {
    const token = ++live.current
    if (!data || data.game !== settings.game) return
    if (data.game === 'factorio' && data.dataset !== (settings.dataset || '2.0')) return
    setSolving(true)
    setStopped(false)
    setProgress(null)
    setStarted(Date.now()); setNow(Date.now())
    const run = approved === key
    // the extra time belongs to this request only (not to settings changed during the delay below)
    const more = extra
    if (more) setExtra(0)
    const t = setTimeout(() => {
      setStarted(Date.now()); setNow(Date.now())
      setRunId(solveStream({ ...settings, time_limit_ms: settings.time_limit_ms + more, max_columns: run ? 0 : AUTO_SOLVE_COLUMNS }, onUpdate(key, modelKey, settings.time_limit_ms, token)))
    }, run ? 0 : 150)
    return () => clearTimeout(t)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data, approved, key, rerun])
  const cancel = () => {
    cancelSolve()
    setSolving(false)
    setApproved(null)
    setStopped(true)
  }
  /** Continue the stopped / timed-out search for another time limit's worth. */
  const keepGoing = () => {
    setStopped(false)
    setSolving(true)
    setStarted(Date.now() - (progress?.elapsed_ms ?? 0)); setNow(Date.now())
    const token = ++live.current
    if (!keepSolving(runId, settings.time_limit_ms, onUpdate(key, modelKey, settings.time_limit_ms, token))) {
      setExtra(settings.time_limit_ms); setRerun(n => n + 1)
    }
  }
  const showBig = big !== null && big.key === key
  const current = !!plan && planKey === key
  // the settings the shown plan was solved for (the mixed-belt tab lays out that plan, not the next one)
  const planSettings = useMemo(() => {
    try { return planKey ? (JSON.parse(planKey) as Settings) : null } catch { return null }
  }, [planKey])
  const limited = current && settings.integer && plan!.warnings.some(w => w.startsWith('Time limit reached'))
  // a whole-building plan in hand that is not proven optimal (timed out, stopped, or still solving)
  const unproven = current && settings.integer && plan!.status === 'optimal' && !plan!.proven_optimal
  // a budget plan that has no whole-building plan yet while the search still runs: not "infeasible"
  const pending = solving && current && settings.integer && plan!.status === 'infeasible'
  const gapPct = (g: number | null | undefined) => fmt(100 * (g ?? 0), (g ?? 0) < 0.01 ? 2 : 1)
  const elapsed = Math.max(0, Math.round((now - started) / 1000))

  if (error) return <div className="loading">{t(`The solver did not load: ${error}. Reload the page to try again.`, `求解器未能加载：${error}。请刷新页面重试。`)}</div>
  if (!data) return <div className="loading">{t('Loading solver…', '正在加载求解器…')}</div>
  // tabs that exist for this game; a tab chosen in another game (Galaxy, Map, Mixed belts) falls back to the plan
  const visibleTabs = TABS.map((id): [Tab, string] => [id, tabLabel(id)]).filter(([id]) => (id !== 'mixed' && id !== 'galaxy' || data.game === 'dsp') && (id !== 'map' || data.game === 'satisfactory'))
  const tab: Tab = visibleTabs.some(([id]) => id === tabSel) ? tabSel : 'table'

  return (
    <IconProvider game={data.game} style={iconStyle}>
    <div className="app">
      <header>
        <h1>{t('Factory Planner', '工厂规划器')}</h1>
        <div className="seg gamesel" role="group" aria-label={t('Game', '游戏')}>
          {GAMES.map(g => (
            <button key={g.id} className={settings.game === g.id ? 'on' : ''} aria-pressed={settings.game === g.id} onClick={() => switchGame(g.id)}>{gameName(g.id, g.name)}</button>
          ))}
        </div>
        <span className="meta" title={t('Mixed-integer solver written in Rust, running in your browser as WebAssembly', '用 Rust 编写的混合整数求解器，以 WebAssembly 在浏览器中运行')}>{t('Game data', '游戏数据')} {data.version}</span>
        <span className="spacer" />
        <div className="seg langsel" role="group" aria-label="Language / 语言">
          <button className={lng === 'zh' ? 'on' : ''} aria-pressed={lng === 'zh'} onClick={() => switchLang('zh')}>中文</button>
          <button className={lng === 'en' ? 'on' : ''} aria-pressed={lng === 'en'} onClick={() => switchLang('en')}>EN</button>
        </div>
        <label className="iconsel">{t('Icons', '图标')}
          <select value={iconStyle} onChange={e => setIconStyle(e.target.value as IconStyle)}>
            {ICON_STYLES.map(o => <option key={o.id} value={o.id}>{iconStyleName(o.id)}</option>)}
          </select>
        </label>
      </header>
      <div className="layout" key={lng}>
        <Sidebar data={data} settings={settings} setSettings={setSettings} />
        <main>
          <div className="tabs" role="tablist">
            {visibleTabs.map(([id, name]) => (
              <button key={id} role="tab" aria-selected={tab === id} className={tab === id ? 'on' : ''} onClick={() => setTab(id)}>{name}</button>
            ))}
            {solving && <span className="solving" role="status">{t('Solving', '求解中')}{elapsed >= 1 ? ` ${elapsed} s` : ''}{settings.integer && !bigRun ? t(` of ≤ ${fmt((progress?.deadline_ms ?? settings.time_limit_ms) / 1000, 1)} s`, `，上限 ${fmt((progress?.deadline_ms ?? settings.time_limit_ms) / 1000, 1)} s`) : ''}{unproven && plan ? t(` · best plan so far, within ${gapPct(plan.gap)} % of the bound`, ` · 目前最佳规划，与下界相差 ${gapPct(plan.gap)} %`) : ''}</span>}
            {solving && !bigRun && (elapsed >= 1 || unproven) && <button className="tabbtn" title={unproven ? t('Stop and keep the best plan found so far', '停止并保留目前找到的最佳规划') : t('Stop solving', '停止求解')} onClick={cancel}>{unproven ? t('Stop', '停止') : t('Cancel', '取消')}</button>}
            {!solving && data && <button className="tabbtn" title={t('Run the solver again with the current settings', '用当前设置重新求解')} onClick={() => setRerun(n => n + 1)}>{t('Solve again', '重新求解')}</button>}
          </div>
          {showBig && big && (
            <div className="warn bigmodel" role="status">
              {bigRun ? <>
                <p><b>{t('Solving a large model', '正在求解大型模型')}</b>{t(`: ${fmt(big.columns, 0)} columns × ${fmt(big.rows, 0)} items, ${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, '0')} so far. In the browser this takes a few minutes; changing any setting also stops it.`, `：${fmt(big.columns, 0)} 列 × ${fmt(big.rows, 0)} 个物品，已用 ${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, '0')}。在浏览器中需要几分钟；更改任何设置也会中止求解。`)}</p>
                <button onClick={cancel}>{t('Cancel', '取消')}</button>
              </> : <>
                <p><b>{t('Large model, not solved yet', '大型模型，尚未求解')}</b>{t(`: ${fmt(big.columns, 0)} columns × ${fmt(big.rows, 0)} items (automatic solving stops at ${fmt(AUTO_SOLVE_COLUMNS, 0)}). `, `：${fmt(big.columns, 0)} 列 × ${fmt(big.rows, 0)} 个物品（超过 ${fmt(AUTO_SOLVE_COLUMNS, 0)} 列不自动求解）。`)}{stopped ? t('The solve was cancelled. ', '求解已取消。') : ''}{t('In the browser this can take a few minutes; you can cancel it.', '在浏览器中可能需要几分钟，可随时取消。')}{settings.recycling === 'all' ? t(' The recycling scope "Everything" makes it this large; "Target chain" solves in well under a second.', ' 回收范围“全部”使模型这么大；“目标链”不到一秒即可求解。') : ''}</p>
                <span className="row">
                  <button onClick={() => setApproved(key)}>{t('Solve anyway', '仍然求解')}</button>
                  {settings.recycling === 'all' && <button onClick={() => setSettings(x => ({ ...x, recycling: 'chain' }))}>{t('Use Target chain', '改用目标链')}</button>}
                </span>
              </>}
            </div>
          )}
          {unproven && !solving && (
            <div className="note bigmodel" role="status">
              <p><b>{t('Best plan found', '已找到最佳规划')}{stopped ? t(' (stopped)', '（已停止）') : ''}</b>{t(` in ${fmt((stopped ? Math.max(progress?.elapsed_ms ?? 0, now - started) : progress?.elapsed_ms ?? plan!.solve_ms) / 1000, 1)} s: within ${gapPct(plan!.gap)} % of the lower bound, not proven optimal. More time can improve it or prove it.`, `，用时 ${fmt((stopped ? Math.max(progress?.elapsed_ms ?? 0, now - started) : progress?.elapsed_ms ?? plan!.solve_ms) / 1000, 1)} s：与下界相差 ${gapPct(plan!.gap)} %，未证明最优。更多时间可改进或证明最优。`)}{/^(load )?failed/.test(progress?.highs_status ?? '') ? t(' HiGHS could not be loaded or run (offline?), so the built-in solver worked alone.', ' HiGHS 无法加载或运行（离线？），仅使用内置求解器。') : ''}</p>
              <span className="row">
                <button onClick={keepGoing}>{t('Keep solving', '继续求解')} (+{fmt(settings.time_limit_ms / 1000, 1)} s)</button>
                <label className="row small">{t('Time limit', '时间限制')}
                  <TimeInput ms={settings.time_limit_ms} onChange={time_limit_ms => setSettings(x => ({ ...x, time_limit_ms }))} /> s</label>
                <button onClick={() => setSettings(x => ({ ...x, integer: false }))}>{t('Use fractional buildings', '改用小数建筑')}</button>
              </span>
            </div>
          )}
          {limited && !unproven && !solving && !stopped && (
            <div className="warn bigmodel" role="status">
              <p><b>{t('Time limit reached', '已达时间限制')}</b>{t(` after ${fmt(settings.time_limit_ms / 1000, 1)} s. No whole-building plan was found in time.`, `（${fmt(settings.time_limit_ms / 1000, 1)} s）。未能在时限内找到整数建筑规划。`)}</p>
              <span className="row">
                {settings.time_limit_ms < MAX_TIME_MS && <button onClick={() => setSettings(x => ({ ...x, time_limit_ms: Math.min(MAX_TIME_MS, x.time_limit_ms * 3) }))}>{t(`Try again with ${fmt(Math.min(MAX_TIME_MS, settings.time_limit_ms * 3) / 1000, 1)} s`, `用 ${fmt(Math.min(MAX_TIME_MS, settings.time_limit_ms * 3) / 1000, 1)} s 重试`)}</button>}
                <label className="row small">{t('Time limit', '时间限制')}
                  <TimeInput ms={settings.time_limit_ms} onChange={time_limit_ms => setSettings(x => ({ ...x, time_limit_ms }))} /> s</label>
                <button onClick={() => setSettings(x => ({ ...x, integer: false }))}>{t('Use fractional buildings', '改用小数建筑')}</button>
              </span>
            </div>
          )}
          {stopped && !solving && !showBig && plan && !current && <p className="warn">{t('The solve was cancelled; the plan shown is from the previous settings. Use “Solve again” to finish it.', '求解已取消；当前显示的是上一组设置的规划。点击“重新求解”完成。')}</p>}
          {!plan && !showBig && <p className="muted">{stopped && !solving ? t('Solve cancelled. Use “Solve again” to run it.', '求解已取消。点击“重新求解”运行。') : t('Working out the first plan…', '正在计算首个规划…')}</p>}
          {pending && <p className="muted">{t('No whole-building plan yet; still searching…', '尚无整数建筑规划；仍在搜索…')}</p>}
          {plan && !pending && tab === 'table' && <Results data={data} plan={plan} settings={settings} setSettings={setSettings} />}
          {plan && tab === 'flow' && <ItemFlow data={data} plan={plan} />}
          {plan && tab === 'graph' && <FlowGraph data={data} plan={plan} />}
          {plan && tab === 'values' && <ItemValues data={data} plan={plan} settings={settings} />}
          {!pending && tab === 'mixed' && data.game === 'dsp' && <MixedBelt plan={plan} planSettings={planSettings} current={current} settings={settings} setSettings={setSettings} />}
          {tab === 'galaxy' && data.game === 'dsp' && <Suspense fallback={<p className="muted">{t('Loading the galaxy planner…', '正在加载星区规划器…')}</p>}><GalaxyTab data={data} settings={settings} /></Suspense>}
          {plan && tab === 'map' && data.game === 'satisfactory' && <Suspense fallback={<p className="muted">{t('Loading the map…', '正在加载地图…')}</p>}><MapView data={data} plan={plan} /></Suspense>}
        </main>
      </div>
    </div>
    </IconProvider>
  )
}
