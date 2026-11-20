import { useEffect, useState } from 'react'
import { GAMES, latestRequest, loadSolver, solveAsync, type Game, type GameData, type Plan, type Settings } from './solver'
import { loadGame, loadSettings, saveSettings } from './urlState'
import Sidebar from './Sidebar'
import Results from './Results'
import FlowGraph from './FlowGraph'
import ItemValues from './ItemValues'
import { ICON_STYLES, IconProvider, loadIconStyle, saveIconStyle, type IconStyle } from './Icon'

type Tab = 'table' | 'graph' | 'values'

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
    loadSolver(settings.game).then(setData).catch(e => setError(String(e)))
  }, [settings.game])
  const switchGame = (g: Game) => setSettings(() => loadGame(g))
  useEffect(() => saveSettings(settings), [settings])

  const [plan, setPlan] = useState<Plan | null>(null)
  const [solving, setSolving] = useState(false)
  useEffect(() => {
    if (!data || data.game !== settings.game) return
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

  if (error) return <div className="loading">Failed to load solver: {error}</div>
  if (!data) return <div className="loading">Loading solver…</div>

  return (
    <IconProvider game={data.game} style={iconStyle}>
    <div className="app">
      <header>
        <h1>Factory Planner</h1>
        <div className="seg gamesel">
          {GAMES.map(g => (
            <button key={g.id} className={settings.game === g.id ? 'on' : ''} onClick={() => switchGame(g.id)}>{g.name}</button>
          ))}
        </div>
        <span className="muted">MILP solver (Rust → WASM) · game data {data.version}</span>
        {solving && <span className="solving">solving…</span>}
        <label className="iconsel small">Icons{' '}
          <select value={iconStyle} onChange={e => setIconStyle(e.target.value as IconStyle)}>
            {ICON_STYLES.map(o => <option key={o.id} value={o.id}>{o.name}</option>)}
          </select>
        </label>
      </header>
      <div className="layout">
        <Sidebar data={data} settings={settings} setSettings={setSettings} />
        <main>
          <div className="tabs">
            <button className={tab === 'table' ? 'on' : ''} onClick={() => setTab('table')}>Plan</button>
            <button className={tab === 'graph' ? 'on' : ''} onClick={() => setTab('graph')}>Flow graph</button>
            <button className={tab === 'values' ? 'on' : ''} onClick={() => setTab('values')}>Item values</button>
          </div>
          {!plan && <p className="muted">Solving…</p>}
          {plan && tab === 'table' && <Results data={data} plan={plan} settings={settings} setSettings={setSettings} />}
          {plan && tab === 'graph' && <FlowGraph data={data} plan={plan} />}
          {plan && tab === 'values' && <ItemValues plan={plan} settings={settings} />}
        </main>
      </div>
    </div>
    </IconProvider>
  )
}
