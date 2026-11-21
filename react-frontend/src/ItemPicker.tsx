import { useEffect, useMemo, useRef, useState } from 'react'
import { Icon, useIconsOn } from './Icon'
import { dn, type GameData, type Item } from './solver'

// Compact icon grid for choosing an item: group tabs (the game's own crafting-menu tabs for DSP and
// Factorio; derived classes for Satisfactory), one line per menu row, and a search box that filters
// across all groups. Hover shows the name; Enter takes the first match; Esc closes.

const GROUPS: Record<string, [string, string][]> = {
  satisfactory: [
    ['raw', 'Raw'], ['ingots', 'Ingots'], ['basic', 'Parts'], ['intermediate', 'Assembled'],
    ['advanced', 'Advanced'], ['exotic', 'Exotic'], ['fluids', 'Fluids'], ['project', 'Project'], ['other', 'Found'],
  ],
  dsp: [['components', '物品'], ['buildings', '建筑']],
  factorio: [
    ['intermediate-products', 'Intermediates'], ['production', 'Production'], ['logistics', 'Logistics'],
    ['combat', 'Combat'], ['fluids', 'Fluids'],
  ],
}

export default function ItemPicker({ data, items, current, onPick, onClose }: {
  data: GameData; items: Item[]; current?: string; onPick: (id: string) => void; onClose: () => void
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

  useEffect(() => {
    const away = (e: MouseEvent) => {
      const t = e.target as HTMLElement
      // the buttons that open pickers toggle them on click; closing here first would shift the layout under the pointer
      if (box.current && !box.current.contains(t) && !t.closest?.('button[aria-expanded]')) onClose()
    }
    document.addEventListener('mousedown', away)
    return () => document.removeEventListener('mousedown', away)
  }, [onClose])

  const name = (i: Item) => dn(i, data.game)
  const query = q.trim().toLowerCase()
  const matches = useMemo(() => !query ? [] : items.filter(i =>
    name(i).toLowerCase().includes(query) || i.name.toLowerCase().includes(query) || i.id.toLowerCase().includes(query)),
  // eslint-disable-next-line react-hooks/exhaustive-deps
  [items, query])
  const rows = useMemo(() => {
    const m = new Map<number, Item[]>()
    for (const i of items) if ((i.group ?? 'other') === group) {
      const r = i.row ?? 0
      if (!m.has(r)) m.set(r, [])
      m.get(r)!.push(i)
    }
    return [...m.entries()].sort((a, b) => a[0] - b[0]).map(([, l]) => l.sort((a, b) => (a.order ?? 0) - (b.order ?? 0)))
  }, [items, group])

  const cell = (i: Item) => (
    <button key={i.id} type="button" className={`pick${i.id === current ? ' on' : ''}${iconsOn ? '' : ' text'}`}
      title={name(i)} aria-label={name(i)}
      onMouseEnter={() => setHover(i)} onFocus={() => setHover(i)}
      onClick={() => onPick(i.id)}>
      {iconsOn ? <Icon id={i.id} size={26} /> : name(i)}
    </button>
  )

  return (
    <div className="picker" ref={box} role="dialog" aria-label="Choose an item"
      onKeyDown={e => { if (e.key === 'Escape') onClose() }}>
      <input className="wide" autoFocus placeholder="Search…" value={q} onChange={e => setQ(e.target.value)}
        onKeyDown={e => { if (e.key === 'Enter' && matches[0]) onPick(matches[0].id) }} />
      {!query && (
        <div className="pickgroups" role="tablist">
          {groups.map(([g, label]) => (
            <button key={g} type="button" role="tab" aria-selected={g === group}
              className={g === group ? 'on' : ''} onClick={() => setGroup(g)}>{label}</button>
          ))}
        </div>
      )}
      <div className="pickgrid">
        {query
          ? (matches.length ? <div className="pickrow">{matches.slice(0, 120).map(cell)}</div>
            : <p className="muted small">No item matches “{q}”.</p>)
          : rows.map((r, n) => <div className="pickrow" key={n}>{r.map(cell)}</div>)}
      </div>
      <div className="pickname">{hover ? name(hover) : ' '}</div>
    </div>
  )
}
