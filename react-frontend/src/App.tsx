import { useEffect, useState } from 'react'
import { GAMES, latestRequest, loadSolver, solveAsync, type Game, type GameData, type Plan, type Settings } from './solver'
import { loadGame, loadSettings, saveSettings } from './urlState'
import Sidebar from './Sidebar'
import Results from './Results'
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
  const [solving, setSolving] = useState(false)
  useEffect(() => {
    if (!data || data.game !== settings.game) return
    if (data.game === 'factorio' && data.dataset !== (settings.dataset || '2.0')) return
    setSolving(true)
    const t = setTimeout(() => {
      const req = solveAsync(settings)
      const id = latestRequest()
      req.then(p => {
        setPlan(p)
        if (id === latestRequest()) setSolving(false)
      })
    }, 150)
    return () => clearTimeout(t)
  }, [data, settings])

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
            {solving && <span className="solving" role="status">Solving</span>}
          </div>
          {!plan && <p className="muted">Working out the first plan…</p>}
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
