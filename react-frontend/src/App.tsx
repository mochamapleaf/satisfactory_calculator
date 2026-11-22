import { useEffect, useState } from 'react'
import { AUTO_SOLVE_COLUMNS, cancelSolve, keepSolving, MAX_TIME_MS, fmt, GAMES, loadSolver, solveStream, type Game, type GameData, type Plan, type Settings, type SolveUpdate } from './solver'
import { loadGame, loadSettings, saveSettings } from './urlState'
import Sidebar from './Sidebar'
import Results from './Results'
import TimeInput from './TimeInput'
import FlowGraph from './FlowGraph'
import ItemValues from './ItemValues'
import ItemFlow from './ItemFlow'
import { ICON_STYLES, IconProvider, loadIconStyle, saveIconStyle, type IconStyle } from './Icon'

type Tab = 'table' | 'flow' | 'graph' | 'values'
const TABS: [Tab, string][] = [['table', 'Plan'], ['flow', 'Item flow'], ['graph', 'Machine graph'], ['values', 'Item values']]

export default function App() {
  const [data, setData] = useState<GameData | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [settings, setSettings] = useState<Settings>(loadSettings)
  const [tab, setTab] = useState<Tab>('table')
  const [iconStyle, setIconStyle] = useState<IconStyle>(loadIconStyle)
  useEffect(() => saveIconStyle(iconStyle), [iconStyle])

  useEffect(() => {
    setData(null)
    setPlan(null)
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
  const [planKey, setPlanKey] = useState('') // settings the shown plan was solved for
  const [solving, setSolving] = useState(false)
  // Large models (e.g. Factorio "recycle everything", ~16 000 columns, minutes in WASM) are not
  // solved automatically, also when they come from saved settings or a link: the solver returns
  // their size and the user starts the solve (`approved` = the settings it was started for).
  const key = JSON.stringify(settings)
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
  // one handler per solve: plans and progress as they stream in
  const onUpdate = (k: string, more: number) => (u: SolveUpdate) => {
    if (u.gone) { setExtra(more); setRerun(n => n + 1); return }
    if (u.plan) {
      const p = u.plan
      if (p.status === 'too_large') {
        setBig({ columns: p.columns ?? 0, rows: p.rows ?? 0, key: k })
        setPlan(null)
      } else {
        setBig(null)
        setPlan(p)
        setPlanKey(k)
      }
    } else if (u.gap !== undefined) {
      // the bound moved: same plan, smaller gap
      setPlan(p => (p && !p.proven_optimal ? { ...p, gap: u.gap ?? null } : p))
    }
    setProgress(u)
    if (u.done) { setSolving(false); setNow(Date.now()) }
  }
  useEffect(() => {
    if (!data || data.game !== settings.game) return
    if (data.game === 'factorio' && data.dataset !== (settings.dataset || '2.0')) return
    setSolving(true)
    setStopped(false)
    setProgress(null)
    setStarted(Date.now()); setNow(Date.now())
    const run = approved === key
    const more = extra
    const t = setTimeout(() => {
      setStarted(Date.now()); setNow(Date.now())
      if (more) setExtra(0)
      setRunId(solveStream({ ...settings, time_limit_ms: settings.time_limit_ms + more, max_columns: run ? 0 : AUTO_SOLVE_COLUMNS }, onUpdate(key, settings.time_limit_ms)))
    }, run ? 0 : 150)
    return () => clearTimeout(t)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data, settings, approved, key, rerun])
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
    if (!keepSolving(runId, settings.time_limit_ms, onUpdate(key, settings.time_limit_ms))) {
      setExtra(settings.time_limit_ms); setRerun(n => n + 1)
    }
  }
  const showBig = big !== null && big.key === key
  const current = !!plan && planKey === key
  const limited = current && settings.integer && plan!.warnings.some(w => w.startsWith('Time limit reached'))
  // a whole-building plan in hand that is not proven optimal (timed out, stopped, or still solving)
  const unproven = current && settings.integer && plan!.status === 'optimal' && !plan!.proven_optimal
  const gapPct = (g: number | null | undefined) => fmt(100 * (g ?? 0), (g ?? 0) < 0.01 ? 2 : 1)
  const elapsed = Math.max(0, Math.round((now - started) / 1000))

  if (error) return <div className="loading">The solver did not load: {error}. Reload the page to try again.</div>
  if (!data) return <div className="loading">Loading solver…</div>

  return (
    <IconProvider game={data.game} style={iconStyle}>
    <div className="app">
      <header>
        <h1>Factory Planner</h1>
        <div className="seg gamesel" role="group" aria-label="Game">
          {GAMES.map(g => (
            <button key={g.id} className={settings.game === g.id ? 'on' : ''} aria-pressed={settings.game === g.id} onClick={() => switchGame(g.id)}>{g.name}</button>
          ))}
        </div>
        <span className="meta" title="Mixed-integer solver written in Rust, running in your browser as WebAssembly">Game data {data.version}</span>
        <span className="spacer" />
        <label className="iconsel">Icons
          <select value={iconStyle} onChange={e => setIconStyle(e.target.value as IconStyle)}>
            {ICON_STYLES.map(o => <option key={o.id} value={o.id}>{o.name}</option>)}
          </select>
        </label>
      </header>
      <div className="layout">
        <Sidebar data={data} settings={settings} setSettings={setSettings} />
        <main>
          <div className="tabs" role="tablist">
            {TABS.map(([id, name]) => (
              <button key={id} role="tab" aria-selected={tab === id} className={tab === id ? 'on' : ''} onClick={() => setTab(id)}>{name}</button>
            ))}
            {solving && <span className="solving" role="status">Solving{elapsed >= 1 ? ` ${elapsed} s` : ''}{settings.integer && !bigRun ? ` of ≤ ${fmt((progress?.deadline_ms ?? settings.time_limit_ms) / 1000, 1)} s` : ''}{unproven && plan ? ` · best plan so far, within ${gapPct(plan.gap)} % of the bound` : ''}</span>}
            {solving && !bigRun && (elapsed >= 1 || unproven) && <button className="tabbtn" title={unproven ? 'Stop and keep the best plan found so far' : 'Stop solving'} onClick={cancel}>{unproven ? 'Stop' : 'Cancel'}</button>}
            {!solving && data && <button className="tabbtn" title="Run the solver again with the current settings" onClick={() => setRerun(n => n + 1)}>Solve again</button>}
          </div>
          {showBig && big && (
            <div className="warn bigmodel" role="status">
              {bigRun ? <>
                <p><b>Solving a large model</b>: {fmt(big.columns, 0)} columns × {fmt(big.rows, 0)} items, {Math.floor(elapsed / 60)}:{String(elapsed % 60).padStart(2, '0')} so far. In the browser this takes a few minutes; changing any setting also stops it.</p>
                <button onClick={cancel}>Cancel</button>
              </> : <>
                <p><b>Large model, not solved yet</b>: {fmt(big.columns, 0)} columns × {fmt(big.rows, 0)} items (automatic solving stops at {fmt(AUTO_SOLVE_COLUMNS, 0)}). {stopped ? 'The solve was cancelled. ' : ''}In the browser this can take a few minutes; you can cancel it.{settings.recycling === 'all' ? ' The recycling scope "Everything" makes it this large; "Target chain" solves in well under a second.' : ''}</p>
                <span className="row">
                  <button onClick={() => setApproved(key)}>Solve anyway</button>
                  {settings.recycling === 'all' && <button onClick={() => setSettings(x => ({ ...x, recycling: 'chain' }))}>Use Target chain</button>}
                </span>
              </>}
            </div>
          )}
          {unproven && !solving && (
            <div className="note bigmodel" role="status">
              <p><b>Best plan found{stopped ? ' (stopped)' : ''}</b> in {fmt((progress?.elapsed_ms ?? plan!.solve_ms) / 1000, 1)} s: within {gapPct(plan!.gap)} % of the lower bound, not proven optimal. More time can improve it or prove it.</p>
              <span className="row">
                <button onClick={keepGoing}>Keep solving (+{fmt(settings.time_limit_ms / 1000, 1)} s)</button>
                <label className="row small">Time limit
                  <TimeInput ms={settings.time_limit_ms} onChange={time_limit_ms => setSettings(x => ({ ...x, time_limit_ms }))} /> s</label>
                <button onClick={() => setSettings(x => ({ ...x, integer: false }))}>Use fractional buildings</button>
              </span>
            </div>
          )}
          {limited && !unproven && !solving && !stopped && (
            <div className="warn bigmodel" role="status">
              <p><b>Time limit reached</b> after {fmt(settings.time_limit_ms / 1000, 1)} s. No whole-building plan was found in time.</p>
              <span className="row">
                {settings.time_limit_ms < MAX_TIME_MS && <button onClick={() => setSettings(x => ({ ...x, time_limit_ms: Math.min(MAX_TIME_MS, x.time_limit_ms * 3) }))}>Try again with {fmt(Math.min(MAX_TIME_MS, settings.time_limit_ms * 3) / 1000, 1)} s</button>}
                <label className="row small">Time limit
                  <TimeInput ms={settings.time_limit_ms} onChange={time_limit_ms => setSettings(x => ({ ...x, time_limit_ms }))} /> s</label>
                <button onClick={() => setSettings(x => ({ ...x, integer: false }))}>Use fractional buildings</button>
              </span>
            </div>
          )}
          {stopped && !solving && !showBig && plan && !current && <p className="warn">The solve was cancelled; the plan shown is from the previous settings. Use “Solve again” to finish it.</p>}
          {!plan && !showBig && <p className="muted">{stopped && !solving ? 'Solve cancelled. Use “Solve again” to run it.' : 'Working out the first plan…'}</p>}
          {plan && tab === 'table' && <Results data={data} plan={plan} settings={settings} setSettings={setSettings} />}
          {plan && tab === 'flow' && <ItemFlow data={data} plan={plan} />}
          {plan && tab === 'graph' && <FlowGraph data={data} plan={plan} />}
          {plan && tab === 'values' && <ItemValues plan={plan} settings={settings} />}
        </main>
      </div>
    </div>
    </IconProvider>
  )
}
