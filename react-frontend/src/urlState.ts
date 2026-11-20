import { defaultSettings, type Settings } from './solver'

// Settings live in the URL hash (shareable links) and localStorage (last session).
const KEY = 'sf-planner-settings'

/** Fill defaults and migrate settings saved by older versions. */
function upgrade(raw: Record<string, unknown>): Settings {
  const s = { ...defaultSettings(), ...raw } as Settings & { clock_percent?: number }
  if (s.clock_percent !== undefined) {
    if (raw.max_clock_percent === undefined) s.max_clock_percent = s.clock_percent
    delete s.clock_percent
  }
  s.cost = { ...defaultSettings().cost, ...(s.cost ?? {}) }
  return s
}

export function loadSettings(): Settings {
  try {
    const h = location.hash.slice(1)
    if (h) return upgrade(JSON.parse(decodeURIComponent(atob(h))))
  } catch { /* ignore bad hash */ }
  try {
    const s = localStorage.getItem(KEY)
    if (s) return upgrade(JSON.parse(s))
  } catch { /* storage unavailable */ }
  return defaultSettings()
}

export function saveSettings(s: Settings) {
  const json = JSON.stringify(s)
  try { localStorage.setItem(KEY, json) } catch { /* ignore */ }
  history.replaceState(null, '', '#' + btoa(encodeURIComponent(json)))
}
