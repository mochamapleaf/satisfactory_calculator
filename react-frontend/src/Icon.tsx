import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'
import { t } from './i18n'
import { qsplit, type Game } from './solver'

// Local item/building icons: one sprite sheet per game and style, built by tools/build_icons.py.
export type IconStyle = 'original' | 'flat' | 'mono' | 'off'
export const ICON_STYLES: { id: IconStyle; name: string }[] = [
  { id: 'original', name: 'Game' },
  { id: 'flat', name: 'Flat 2D' },
  { id: 'mono', name: 'Minimal' },
  { id: 'off', name: 'Off' },
]
/** Style name in the interface language (call at render time). */
export const iconStyleName = (id: IconStyle): string =>
  ({ original: t('Game', '游戏原图'), flat: t('Flat 2D', '扁平 2D'), mono: t('Minimal', '极简'), off: t('Off', '关闭') })[id]
const STYLE_KEY = 'factory-planner:icon-style'

interface Meta { size: number; cols: number; rows: number; index: Record<string, number> }
interface Ctx { game: Game; style: IconStyle; meta: Meta | null }
const IconCtx = createContext<Ctx>({ game: 'satisfactory', style: 'off', meta: null })
const metaCache = new Map<Game, Promise<Meta | null>>()

function loadMeta(game: Game) {
  if (!metaCache.has(game)) {
    metaCache.set(game, fetch(`./icons/${game}.json`).then(r => (r.ok ? r.json() : null)).catch(() => null))
  }
  return metaCache.get(game)!
}

export function loadIconStyle(): IconStyle {
  try {
    const s = localStorage.getItem(STYLE_KEY)
    if (s === 'original' || s === 'flat' || s === 'mono' || s === 'off') return s
  } catch { /* storage unavailable */ }
  return 'original'
}
export function saveIconStyle(s: IconStyle) {
  try { localStorage.setItem(STYLE_KEY, s) } catch { /* ignore */ }
}

export function IconProvider({ game, style, children }: { game: Game; style: IconStyle; children: ReactNode }) {
  const [meta, setMeta] = useState<{ game: Game; meta: Meta | null } | null>(null)
  useEffect(() => {
    let live = true
    loadMeta(game).then(m => live && setMeta({ game, meta: m }))
    return () => { live = false }
  }, [game])
  const m = meta?.game === game ? meta.meta : null
  return <IconCtx.Provider value={{ game, style, meta: m }}>{children}</IconCtx.Provider>
}

/** True when icons are being drawn (a style is chosen and the sheet has loaded). */
export function useIconsOn() {
  const { style, meta } = useContext(IconCtx)
  return style !== 'off' && !!meta
}

/** Factorio quality pip (uncommon … legendary), drawn on the icon's corner or on its own. */
export function QualityPip({ q }: { q: number }) {
  if (!q) return null
  return <span className={`qpip q${q}`} aria-hidden="true" />
}

/** Item or building icon; renders nothing when icons are off or the id has none. Quality ids
 *  (`item@q`) draw the base icon with a coloured pip; without an icon only the pip shows. */
export function Icon({ id, size = 20, title }: { id: string | undefined; size?: number; title?: string }) {
  const { game, style, meta } = useContext(IconCtx)
  if (!id) return null
  const [base, q] = qsplit(id)
  if (style === 'off' || !meta) return q ? <QualityPip q={q} /> : null
  const n = meta.index[base]
  if (n === undefined) return q ? <QualityPip q={q} /> : null
  const x = (n % meta.cols) * size
  const y = Math.floor(n / meta.cols) * size
  return (
    <span
      className={`icon icon-${style}`}
      title={title}
      style={{
        width: size,
        height: size,
        backgroundImage: `url(./icons/${game}-${style}.webp)`,
        backgroundSize: `${meta.cols * size}px ${meta.rows * size}px`,
        backgroundPosition: `-${x}px -${y}px`,
      }}
    >{q > 0 && <QualityPip q={q} />}</span>
  )
}

/** Small drawn "remove" mark for icon buttons (instead of a × glyph). */
export function RemoveMark() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
      <path d="M3 3l6 6M9 3l-6 6" />
    </svg>
  )
}
