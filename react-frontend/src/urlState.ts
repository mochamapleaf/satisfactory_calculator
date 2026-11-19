import { defaultSettings, type Settings } from './solver'

// Settings live in the URL hash (shareable links) and localStorage (last session).
const KEY = 'sf-planner-settings'

export function loadSettings(): Settings {
  try {
    const h = location.hash.slice(1)
    if (h) return { ...defaultSettings(), ...JSON.parse(decodeURIComponent(atob(h))) }
  } catch { /* ignore bad hash */ }
  try {
    const s = localStorage.getItem(KEY)
    if (s) return { ...defaultSettings(), ...JSON.parse(s) }
  } catch { /* storage unavailable */ }
  return defaultSettings()
}

export function saveSettings(s: Settings) {
  const json = JSON.stringify(s)
  try { localStorage.setItem(KEY, json) } catch { /* ignore */ }
  history.replaceState(null, '', '#' + btoa(encodeURIComponent(json)))
}
