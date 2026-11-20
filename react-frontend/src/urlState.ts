import { defaultSettings, type Game, type Settings } from './solver'

// Settings live in the URL hash (shareable links) and localStorage (last session).
const KEY = 'sf-planner-settings'
const GAME_KEY = 'sf-planner-game'
const keyFor = (g: Game) => (g === 'satisfactory' ? KEY : `${KEY}-${g}`)

/** Fill defaults and migrate settings saved by older versions. */
function upgrade(raw: Record<string, unknown>): Settings {
  const game = (raw.game as Game) ?? 'satisfactory'
  const s = { ...defaultSettings(game), ...raw } as Settings & { clock_percent?: number }
  if (s.clock_percent !== undefined) {
    if (raw.max_clock_percent === undefined) s.max_clock_percent = s.clock_percent
    delete s.clock_percent
  }
  s.cost = { ...defaultSettings(game).cost, ...(s.cost ?? {}) }
  return s
}

/** Last saved settings for `game` (switching games). */
export function loadGame(game: Game): Settings {
  try {
    const s = localStorage.getItem(keyFor(game))
    if (s) return upgrade({ ...JSON.parse(s), game })
  } catch { /* storage unavailable */ }
  return defaultSettings(game)
}

export function loadSettings(): Settings {
  try {
    const h = location.hash.slice(1)
    if (h) return upgrade(JSON.parse(decodeURIComponent(atob(h))))
  } catch { /* ignore bad hash */ }
  try {
    const g = (localStorage.getItem(GAME_KEY) as Game | null) ?? 'satisfactory'
    const s = localStorage.getItem(keyFor(g))
    if (s) return upgrade({ ...JSON.parse(s), game: g })
  } catch { /* storage unavailable */ }
  return defaultSettings()
}

export function saveSettings(s: Settings) {
  const json = JSON.stringify(s)
  try {
    localStorage.setItem(keyFor(s.game), json)
    localStorage.setItem(GAME_KEY, s.game)
  } catch { /* ignore */ }
  history.replaceState(null, '', '#' + btoa(encodeURIComponent(json)))
}
