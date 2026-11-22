import { DATASETS, defaultBeltLayout, defaultSettings, withDataset, type BeltLayoutSettings, type Game, type Settings } from './solver'

// Settings live in the URL hash (shareable links) and localStorage (last session).
const KEY = 'sf-planner-settings'
const GAME_KEY = 'sf-planner-game'
const keyFor = (g: Game) => (g === 'satisfactory' ? KEY : `${KEY}-${g}`)

const isRecord = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)
const isRate = (v: unknown) => isRecord(v) && typeof v.item === 'string' && typeof v.rate === 'number' && Number.isFinite(v.rate)

/** Drop values of the wrong type (a hand-edited or corrupted link must not blank the app):
 *  each field must have the type of its default; arrays keep only elements of the default's
 *  element type (targets / supplies: {item, rate} rows), records only finite numbers or null. */
function sanitize(def: Settings, raw: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(raw)) {
    const d = (def as unknown as Record<string, unknown>)[k]
    if (d === undefined) { out[k] = v; continue } // optional fields, older names (clock_percent)
    if (d === null) { if (v === null || (typeof v === 'number' && Number.isFinite(v))) out[k] = v; continue } // budgets
    if (Array.isArray(d)) {
      if (!Array.isArray(v)) continue
      if (k === 'targets' || k === 'supplies') out[k] = v.filter(isRate)
      else if (k === 'beacon_counts' || k === 'module_costs') out[k] = v.filter(x => typeof x === 'number' && Number.isFinite(x))
      else out[k] = v.filter(x => typeof x === 'string')
    } else if (isRecord(d)) {
      if (!isRecord(v)) continue
      if (k === 'cost' || k === 'belt_layout') { out[k] = v; continue } // merged with the defaults below
      out[k] = Object.fromEntries(Object.entries(v).filter(([, x]) => x === null || (typeof x === 'number' && Number.isFinite(x))))
    } else if (typeof d === 'number') {
      if (typeof v === 'number' && Number.isFinite(v)) out[k] = v
    } else if (typeof v === typeof d) out[k] = v
  }
  return out
}

/** Fill defaults and migrate settings saved by older versions. */
function upgrade(input: Record<string, unknown>): Settings {
  const game: Game = (['satisfactory', 'dsp', 'factorio'] as const).find(g => g === input.game) ?? 'satisfactory'
  const raw = isRecord(input) ? sanitize(defaultSettings(game), { ...input, game }) : { game }
  const s = { ...defaultSettings(game), ...raw } as Settings & { clock_percent?: number }
  if (s.clock_percent !== undefined) {
    if (raw.max_clock_percent === undefined) s.max_clock_percent = s.clock_percent
    delete s.clock_percent
  }
  s.cost = { ...defaultSettings(game).cost, ...(s.cost ?? {}) }
  {
    // mixed-belt settings: defaults for missing or mistyped fields
    const def = defaultBeltLayout() as unknown as Record<string, unknown>
    const bl: Record<string, unknown> = { ...def }
    for (const [k, v] of Object.entries((s.belt_layout ?? {}) as unknown as Record<string, unknown>)) {
      if (k === 'overrides') { if (isRecord(v)) bl[k] = v; continue }
      if (!(k in def)) continue
      if (typeof v === 'number' && Number.isFinite(v)) bl[k] = v
      else if (v === null && def[k] === null) bl[k] = null
    }
    s.belt_layout = bl as unknown as BeltLayoutSettings
  }
  if (s.objective === 'compute' && game !== 'dsp') s.objective = 'buildings' // DSP-only preset
  delete s.max_columns // a per-request guard, never taken from a link
  if (game === 'factorio') {
    // an unknown data set id (typo, or a set removed later) falls back to base 2.0
    const id = DATASETS.some(d => d.id === s.dataset) ? s.dataset : '2.0'
    return withDataset(s, id)
  }
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
