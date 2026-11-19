import { useState, useEffect } from 'react'
import { initWasm, WasmModule } from '../wasm'
import './App.css'

function App() {
  const [wasmModule, setWasmModule] = useState<WasmModule | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    initWasm()
      .then((module) => {
        setWasmModule(module)
        setLoading(false)
      })
      .catch((err) => {
        setError(err.message)
        setLoading(false)
      })
  }, [])

  const handleGreet = () => {
    if (wasmModule) {
      wasmModule.greet('React + WASM')
    }
  }

  if (loading) {
    return <div>Loading WASM module...</div>
  }

  if (error) {
    return <div>Error loading WASM: {error}</div>
  }

  return (
    <div className="App">
      <header className="App-header">
        <h1>Satisfactory Production Calculator</h1>
        <p>React + TypeScript + WASM</p>
        <button onClick={handleGreet}>
          Test WASM Integration
        </button>
      </header>
    </div>
  )
}

export default App