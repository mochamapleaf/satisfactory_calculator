import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent, type RefObject } from 'react'
import { Icon, useIconsOn } from './Icon'
import { dn, type GameData, type Item } from './solver'

// Compact icon grid for choosing an item: group tabs (the game's own crafting-menu tabs for DSP and
// Factorio; derived classes for Satisfactory), one line per menu row, and a search box that filters
// across all groups. Hover shows the name; Enter takes the first match (outlined); Esc closes.
// Items in `taken` (already in the list) are shown but cannot be picked.

const GROUPS: Record<string, [string, string][]> = {
  satisfactory: [
    ['raw', 'Raw'], ['ingots', 'Ingots'], ['basic', 'Parts'], ['intermediate', 'Assembled'],
    ['advanced', 'Advanced'], ['exotic', 'Exotic'], ['refined', 'Refined'], ['fluids', 'Fluids'],
    ['project', 'Project'], ['other', 'Found'],
  ],
  dsp: [['components', '物品'], ['buildings', '建筑']],
  factorio: [
    ['intermediate-products', 'Intermediates'], ['production', 'Production'], ['logistics', 'Logistics'],
    ['combat', 'Combat'], ['fluids', 'Fluids'],
  ],
}
const MAX_RESULTS = 120

export default function ItemPicker({ data, items, current, taken, scope, onPick, onClose }: {
  data: GameData; items: Item[]; current?: string; taken?: Set<string>
  /** the list that owns this picker: its own opener buttons toggle the picker, so clicks on them are not "outside" */
  scope?: RefObject<HTMLElement | null>
  onPick: (id: string) => void; onClose: (how: 'esc' | 'away') => void
}) {
  const iconsOn = useIconsOn()
  const groups = useMemo(() => {
    const known = GROUPS[data.game] ?? []
    const present = new Set(items.map(i => i.group ?? 'other'))
    const list = known.filter(([g]) => present.has(g))
    for (const g of present) if (!list.some(([k]) => k === g)) list.push([g, g])
    return list
  }, [data.game, items])
  const curGroup = items.find(i => i.id === current)?.group
  const [group, setGroup] = useState(curGroup ?? groups[0]?.[0] ?? 'other')
  const [q, setQ] = useState('')
  const [hover, setHover] = useState<Item | null>(null)
  const box = useRef<HTMLDivElement>(null)
  const uid = useId()

  useEffect(() => {
    const away = (e: MouseEvent) => {
      const t = e.target as HTMLElement
      // this list's opener buttons toggle the picker on click; closing here first would shift the layout under
      // the pointer. Openers of other lists do not know about this picker, so a click on them closes it.
      const opener = t.closest?.('button[aria-expanded]')
      if (box.current && !box.current.contains(t) && !(opener && scope?.current?.contains(opener))) onClose('away')
    }
    document.addEventListener('mousedown', away)
    return () => document.removeEventListener('mousedown', away)
  }, [onClose, scope])

  const name = (i: Item) => dn(i, data.game)
  const isTaken = (i: Item) => !!taken?.has(i.id) && i.id !== current
  const query = q.trim().toLowerCase()
  const matches = useMemo(() => !query ? [] : items.filter(i =>
    name(i).toLowerCase().includes(query) || i.name.toLowerCase().includes(query) || i.id.toLowerCase().includes(query)),
  // eslint-disable-next-line react-hooks/exhaustive-deps
  [items, query])
  const first = matches.find(i => !isTaken(i))
  const rows = useMemo(() => {
    const m = new Map<number, Item[]>()
    for (const i of items) if ((i.group ?? 'other') === group) {
      const r = i.row ?? 0
      if (!m.has(r)) m.set(r, [])
      m.get(r)!.push(i)
    }
    return [...m.entries()].sort((a, b) => a[0] - b[0]).map(([, l]) => l.sort((a, b) => (a.order ?? 0) - (b.order ?? 0)))
  }, [items, group])

  // arrow keys move between group tabs (roving tabindex)
  const tabKeys = (e: KeyboardEvent) => {
    const i = groups.findIndex(([g]) => g === group)
    const j = e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowLeft' ? i - 1 : e.key === 'Home' ? 0 : e.key === 'End' ? groups.length - 1 : null
    if (j === null) return
    e.preventDefault()
    const g = groups[(j + groups.length) % groups.length][0]
    setGroup(g)
    document.getElementById(`${uid}-tab-${g}`)?.focus()
  }

  const cell = (i: Item) => (
    <button key={i.id} type="button"
      className={`pick${i.id === current ? ' on' : ''}${iconsOn ? '' : ' text'}${query && i === first ? ' first' : ''}`}
      title={isTaken(i) ? `${name(i)} (already in the list)` : name(i)} aria-label={name(i)} disabled={isTaken(i)}
      onMouseEnter={() => setHover(i)} onFocus={() => setHover(i)}
      onClick={() => onPick(i.id)}>
      {iconsOn ? <Icon id={i.id} size={26} /> : name(i)}
    </button>
  )

  return (
    <div className="picker" ref={box} role="dialog" aria-label="Choose an item"
      onKeyDown={e => { if (e.key === 'Escape') { e.stopPropagation(); onClose('esc') } }}>
      <input className="wide" autoFocus placeholder="Search…" aria-label="Search items" value={q} onChange={e => setQ(e.target.value)}
        onKeyDown={e => { if (e.key === 'Enter' && !e.nativeEvent.isComposing && first) onPick(first.id) }} />
      {!query && (
        <div className="pickgroups" role="tablist" onKeyDown={tabKeys}>
          {groups.map(([g, label]) => (
            <button key={g} type="button" role="tab" id={`${uid}-tab-${g}`} aria-selected={g === group}
              aria-controls={`${uid}-panel`} tabIndex={g === group ? 0 : -1}
              className={g === group ? 'on' : ''} onClick={() => setGroup(g)}>{label}</button>
          ))}
        </div>
      )}
      <div className="pickgrid" id={`${uid}-panel`} role={query ? undefined : 'tabpanel'}
        aria-labelledby={query ? undefined : `${uid}-tab-${group}`}>
        {query
          ? (matches.length ? <>
              <div className="pickrow">{matches.slice(0, MAX_RESULTS).map(cell)}</div>
              {matches.length > MAX_RESULTS && <p className="muted small">Showing {MAX_RESULTS} of {matches.length}; type more to narrow the search.</p>}
            </>
            : <p className="muted small">No item matches “{q}”.</p>)
          : rows.map((r, n) => <div className="pickrow" key={n}>{r.map(cell)}</div>)}
      </div>
      <div className="pickname">{hover ? name(hover) : query && first ? `Enter picks ${name(first)}` : ' '}</div>
    </div>
  )
}
