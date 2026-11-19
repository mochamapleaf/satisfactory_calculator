import { useEffect, useMemo, useState } from 'react'
import { loadSolver, solve, type GameData, type Settings } from './solver'
import { loadSettings, saveSettings } from './urlState'
import Sidebar from './Sidebar'
import Results from './Results'
import FlowGraph from './FlowGraph'

type Tab = 'table' | 'graph'

export default function App() {
  const [data, setData] = useState<GameData | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [settings, setSettings] = useState<Settings>(loadSettings)
  const [tab, setTab] = useState<Tab>('table')

  useEffect(() => {
    loadSolver().then(setData).catch(e => setError(String(e)))
  }, [])
  useEffect(() => saveSettings(settings), [settings])

  const plan = useMemo(() => (data ? solve(settings) : null), [data, settings])

  if (error) return <div className="loading">Failed to load solver: {error}</div>
  if (!data || !plan) return <div className="loading">Loading solver…</div>

  return (
    <div className="app">
      <header>
        <h1>Satisfactory Planner</h1>
        <span className="muted">LP solver (Rust → WASM) · game data {data.version}</span>
      </header>
      <div className="layout">
        <Sidebar data={data} settings={settings} setSettings={setSettings} />
        <main>
          <div className="tabs">
            <button className={tab === 'table' ? 'on' : ''} onClick={() => setTab('table')}>Plan</button>
            <button className={tab === 'graph' ? 'on' : ''} onClick={() => setTab('graph')}>Flow graph</button>
          </div>
          {tab === 'table'
            ? <Results data={data} plan={plan} settings={settings} setSettings={setSettings} />
            : <FlowGraph data={data} plan={plan} />}
        </main>
      </div>
    </div>
  )
}
