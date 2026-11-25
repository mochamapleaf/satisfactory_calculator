import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Icon, QualityPip, RemoveMark } from './Icon'
import ItemPicker from './ItemPicker'
import TimeInput from './TimeInput'
import { elevatorTargets } from './presets'
import { lang, t } from './i18n'
import { DATASETS, defaultSettings, withDataset, dn, fmt, GENERATORS, itemName as qItemName, qualityNames, preset, qsplit, scarcity, withQ, type CostWeights, type Generator, type GameData, type ItemRate, type Objective, type Settings, type Engine } from './solver'

interface Props {
  data: GameData
  settings: Settings
  setSettings: (f: (s: Settings) => Settings) => void
}

/** Quality tier select (Factorio data sets with quality). */
function QualitySelect({ data, value, onChange, label }: { data: GameData; value: number; onChange: (q: number) => void; label: string }) {
  const names = qualityNames(data)
  return (
    <span className="qselwrap"><QualityPip q={value} />
      <select className="qsel" value={value} aria-label={label} title={label} onChange={e => onChange(+e.target.value)}>
        {names.map((n, q) => <option key={q} value={q}>{n}</option>)}
      </select>
    </span>
  )
}

function ItemRates({ data, list, onChange, producibleOnly, quality }: {
  data: GameData; list: ItemRate[]; onChange: (l: ItemRate[]) => void; producibleOnly?: boolean
  /** show a quality select per row (Factorio data with quality) */
  quality?: boolean
}) {
  const produced = useMemo(() => new Set(data.recipes.flatMap(r => r.out.map(o => o[0]))), [data])
  const options = useMemo(() => data.items.filter(i => !producibleOnly || produced.has(i.id) || i.raw), [data, produced, producibleOnly])
  // which picker is open: a row index (replace that item) or 'add'
  const [open, setOpen] = useState<number | 'add' | null>(null)
  const scope = useRef<HTMLDivElement>(null)
  // after a pick or Esc, focus goes back to the button that opened the picker (it may have been re-rendered)
  const refocus = useRef<number | 'add' | null>(null)
  useEffect(() => {
    if (refocus.current === null) return
    scope.current?.querySelector<HTMLElement>(`[data-opener="${refocus.current}"]`)?.focus()
    refocus.current = null
  })
  // Picker ids are base items; a row keeps its quality when its item changes. An item is taken
  // at the quality the pick would use (new rows are normal), so with quality the same item can
  // be listed at several qualities (normal + legendary gears).
  const liquid = (id: string) => data.items.find(i => i.id === qsplit(id)[0])?.liquid ?? false
  const atQ = (id: string, q: number) => (liquid(id) ? qsplit(id)[0] : withQ(id, q))
  const pickQ = open === null || open === 'add' ? 0 : qsplit(list[open]?.item ?? '')[1]
  const listed = new Set(list.map(t => t.item))
  const taken = new Set(list.map(t => qsplit(t.item)[0]).filter(b => listed.has(atQ(b, pickQ))))
  const name = (id: string) => qItemName(data, id)
  const pick = (id: string) => {
    // items already in the list are disabled in the picker, so a pick never duplicates or drops a row
    if (open === 'add') {
      if (!taken.has(id)) onChange([...list, { item: id, rate: 10 }])
    } else if (open !== null && (!taken.has(id) || qsplit(list[open].item)[0] === id)) {
      onChange(list.map((t, j) => (j === open ? { ...t, item: atQ(id, qsplit(t.item)[1]) } : t)))
    }
    refocus.current = open
    setOpen(null)
  }
  const close = useCallback((how: 'esc' | 'away') => {
    setOpen(o => { if (how === 'esc') refocus.current = o; return null })
  }, [])
  const picker = (current?: string) => (
    <ItemPicker data={data} items={options} current={current} taken={taken} scope={scope} onPick={pick} onClose={close} />
  )
  return (
    <div ref={scope}>
      {list.map((it, i) => (
        <div key={it.item}>
          <div className="row">
            <button type="button" className="swap grow" title={t(`Change ${name(it.item)}`, `更换 ${name(it.item)}`)} data-opener={i}
              aria-expanded={open === i} onClick={() => setOpen(open === i ? null : i)}>
              <Icon id={it.item} />{name(it.item)}
            </button>
            <input type="number" min={0} step="any" value={it.rate} aria-label={t(`${name(it.item)} per minute`, `${name(it.item)} 每分钟`)}
              onChange={e => onChange(list.map((x, j) => j === i ? { ...x, rate: Math.max(0, +e.target.value) } : x))} />
            <span className="muted">{it.item === 'Electricity' ? 'MW' : t('/min', '/分钟')}</span>
            <button className="x" title={t(`Remove ${name(it.item)}`, `移除 ${name(it.item)}`)} aria-label={t(`Remove ${name(it.item)}`, `移除 ${name(it.item)}`)} onClick={() => onChange(list.filter((_, j) => j !== i))}><RemoveMark /></button>
          </div>
          {quality && !liquid(it.item) && (
            <div className="row small qrow">
              <span className="grow muted">{t('Quality', '品质')}</span>
              <QualitySelect data={data} value={qsplit(it.item)[1]} label={t(`Quality of ${name(it.item)}`, `${name(it.item)} 的品质`)}
                onChange={q => {
                  // a quality that is already listed merges into that row
                  const item = withQ(it.item, q)
                  const k = list.findIndex((x, j) => j !== i && x.item === item)
                  onChange(k < 0 ? list.map((x, j) => j === i ? { ...x, item } : x)
                    : list.flatMap((x, j) => j === i ? [] : j === k ? [{ ...x, rate: x.rate + it.rate }] : [x]))
                }} />
            </div>
          )}
          {open === i && picker(qsplit(it.item)[0])}
        </div>
      ))}
      <button type="button" className="addpick" aria-expanded={open === 'add'} data-opener="add"
        onClick={() => setOpen(open === 'add' ? null : 'add')}>{t('+ Add an item', '+ 添加物品')}</button>
      {open === 'add' && picker()}
    </div>
  )
}

function Num({ value, onChange, placeholder }: { value: number | undefined; onChange: (v: number | undefined) => void; placeholder?: string }) {
  return <input type="number" min={0} step="any" value={value ?? ''} placeholder={placeholder}
    onChange={e => onChange(e.target.value === '' ? undefined : Math.max(0, +e.target.value))} />
}

function CostEditor({ data, cost, onChange }: { data: GameData; cost: CostWeights; onChange: (c: CostWeights) => void }) {
  const set = (patch: Partial<CostWeights>) => onChange({ ...cost, ...patch })
  const terms: [keyof CostWeights, string, string][] = [
    ['resources', t('Raw resources', '原矿'), t('× Σ weight · unit/min', '× Σ 权重 · 个/分钟')],
    ['buildings', t('Buildings', '建筑'), t('× Σ weight · building', '× Σ 权重 · 建筑')],
    ['power', t('Power', '电力'), t('per MW', '每 MW')],
    ['shards', t('Power shards', '能量碎片'), t('per shard', '每个')],
    ['surplus', t('Byproducts', '副产品'), t('per unit/min dumped', '每丢弃 1 个/分钟')],
    ...(data.game === 'dsp' ? [
      ['sorter', t('Sorters', '分拣器'), t('per port of each building', '每个建筑的每个端口')],
      ['per_machine', t('Belts + stations', '传送带 + 物流站'), t('per building', '每个建筑')],
      ['coater', t('Spray coaters', '喷涂机'), t('per sprayed building', '每个喷涂的建筑')],
    ] as [keyof CostWeights, string, string][] : []),
  ]
  const perMap = (key: 'resource_weights' | 'building_weights', id: string, v: number | undefined) => {
    const m = { ...cost[key] }
    if (v === undefined) delete m[id]
    else m[id] = v
    set({ [key]: m } as Partial<CostWeights>)
  }
  const usedBuildings = data.buildings.filter(b => data.recipes.some(r => r.building === b.id || r.producers?.includes(b.id)))
  return (
    <div className="costedit">
      {terms.map(([k, label, unit]) => (
        <div className="row small" key={k}>
          <span className="grow">{label} <span className="muted">{unit}</span></span>
          <Num value={cost[k] as number} onChange={v => set({ [k]: v ?? 0 } as Partial<CostWeights>)} />
        </div>
      ))}
      <details>
        <summary>{t('Weight per raw resource', '每种原矿的权重')}</summary>
        {data.resources.map(r => (
          <div className="row small" key={r.item}>
            <span className="grow"><Icon id={r.item} size={16} />{dn(data.items.find(i => i.id === r.item), data.game, r.item)}</span>
            <Num value={cost.resource_weights[r.item]} placeholder={fmt((r as { weight?: number }).weight ?? scarcity(r.limit), 3)}
              onChange={v => perMap('resource_weights', r.item, v)} />
          </div>
        ))}
        <p className="hint">{t('Blank = scarcity default (92 100 / world maximum).', '留空 = 默认稀缺度权重（92 100 / 世界最大产量）。')}</p>
      </details>
      <details>
        <summary>{t('Weight per building type', '每种建筑的权重')}</summary>
        {usedBuildings.map(b => (
          <div className="row small" key={b.id}>
            <span className="grow">{dn(b, data.game)}</span>
            <Num value={cost.building_weights[b.id]} placeholder="1" onChange={v => perMap('building_weights', b.id, v)} />
          </div>
        ))}
        <p className="hint">{t('E.g. make Manufacturers cost 4 to prefer simpler buildings.', '例如把制造器的权重设为 4，以偏向更简单的建筑。')}</p>
      </details>
      <a href="#" onClick={e => { e.preventDefault(); onChange(preset('resources')) }}>{t('Reset weights', '重置权重')}</a>
    </div>
  )
}

/** Items whose recycling is allowed beyond the scope (chips + item picker). */
function AllowList({ data, list, onChange }: { data: GameData; list: string[]; onChange: (l: string[]) => void }) {
  const [open, setOpen] = useState(false)
  const scope = useRef<HTMLDivElement>(null)
  const recyclable = useMemo(() => {
    const ids = new Set(data.recipes.filter(r => r.recycling).map(r => r.in[0]?.[0]))
    return data.items.filter(i => ids.has(i.id))
  }, [data])
  return (
    <div ref={scope}>
      {list.length > 0 && <ul className="chips">
        {list.map(id => (
          <li key={id}><Icon id={id} size={16} />{qItemName(data, id)}
            <button className="x" aria-label={t(`Stop recycling ${qItemName(data, id)}`, `停止回收 ${qItemName(data, id)}`)} title={t('Remove', '移除')} onClick={() => onChange(list.filter(x => x !== id))}><RemoveMark /></button>
          </li>
        ))}
      </ul>}
      <button type="button" className="addpick" aria-expanded={open} onClick={() => setOpen(!open)}>{t('+ Allow recycling of an item', '+ 允许回收物品')}</button>
      {open && <ItemPicker data={data} items={recyclable} taken={new Set(list)} scope={scope}
        onPick={id => { if (!list.includes(id)) onChange([...list, id]); setOpen(false) }} onClose={() => setOpen(false)} />}
    </div>
  )
}

function QualitySettings({ data, s, set }: { data: GameData; s: Settings; set: (p: Partial<Settings>) => void }) {
  const mods = data.effects ?? []
  const qmods = mods.filter(e => (e.by_quality?.[0]?.quality ?? e.quality ?? 0) > 0)
  const pmods = mods.filter(e => e.kind === 'module' && e.productivity > 0)
  const tier = (id: string) => mods.find(e => e.id === id)?.by_quality?.[s.module_quality]
  const qv = tier(s.quality_module)?.quality ?? 0
  const pv = tier(s.prod_module)?.productivity ?? 0
  const active = s.targets.some(t => qsplit(t.item)[1] > 0) || s.supplies.some(t => qsplit(t.item)[1] > 0)
  return (
    <section>
      <h2>{t('Quality', '品质')}</h2>
      <p className="hint">
        {active
          ? t('Every recipe on the chain runs per input quality; the solver picks quality / productivity modules per machine and where to recycle. ', '链上每个配方按输入品质分别运行；求解器为每台机器选择品质 / 产能插件，并决定在哪里回收。') + (s.integer ? t('Whole buildings: the tiers of a recipe with the same machine and modules share machines that switch recipes.', '整数建筑：同一配方中机器和插件相同的各品质档共用机器，机器在配方间切换。') : t('Solved as an LP (fractional machines).', '按线性规划求解（机器数可为小数）。'))
          : t('Pick a quality above normal on a target to plan an upcycling loop.', '给目标选一个高于普通的品质，即可规划升级回收循环。')}
      </p>
      <div className="row small">
        <span className="grow">{t('Module quality', '插件品质')}</span>
        <QualitySelect data={data} value={s.module_quality} label={t('Quality of all modules', '所有插件的品质')} onChange={module_quality => set({ module_quality })} />
      </div>
      <div className="row small" title={t('Crafting speed of every machine at this quality (from the game data, e.g. assembling machine 3: 1.25 → 3.125 legendary); power per machine unchanged', '所有机器在该品质下的制作速度（来自游戏数据，例如组装机 3：1.25 → 传说 3.125）；单台机器功率不变')}>
        <span className="grow">{t('Machine quality', '机器品质')}</span>
        <QualitySelect data={data} value={s.machine_quality ?? 0} label={t('Quality of all machines', '所有机器的品质')} onChange={machine_quality => set({ machine_quality })} />
      </div>
      <div className="row small">
        <span className="grow">{t('Quality slots', '品质插槽')}</span>
        <select value={s.quality_module} aria-label={t('Quality module', '品质插件')} onChange={e => set({ quality_module: e.target.value })}>
          <option value="">{t('none', '无')}</option>
          {qmods.map(e => <option key={e.id} value={e.id}>{dn(e, data.game)}</option>)}
        </select>
        <span className="muted">{qv ? `+${fmt(100 * qv, 2)}%` : ''}</span>
      </div>
      <div className="row small">
        <span className="grow">{t('Other slots', '其余插槽')}</span>
        <select value={s.prod_module} aria-label={t('Productivity module', '产能插件')} onChange={e => set({ prod_module: e.target.value })}>
          <option value="">{t('empty', '空')}</option>
          {pmods.map(e => <option key={e.id} value={e.id}>{dn(e, data.game)}</option>)}
        </select>
        <span className="muted">{pv ? `+${fmt(100 * pv, 2)}%` : ''}</span>
      </div>
      <div className="row small"><span className="grow">{t('Recycle', '回收')}</span></div>
      <div className="seg" role="group" aria-label={t('Recycling scope', '回收范围')}>
        {([['chain', t('Target chain', '目标链')], ['list', t('Only listed', '仅列表')], ['all', t('Everything', '全部')]] as const).map(([id, label]) => (
          <button key={id} className={s.recycling === id ? 'on' : ''} aria-pressed={s.recycling === id} onClick={() => set({ recycling: id })}>{label}</button>
        ))}
      </div>
      <p className="hint">
        {s.recycling === 'chain' && t('Recyclers may take the targets and the items on their ingredient chain, plus the items listed below.', '回收机可以回收目标、目标原料链上的物品，以及下方列出的物品。')}
        {s.recycling === 'list' && t('Recyclers only take the items listed below.', '回收机只回收下方列出的物品。')}
        {s.recycling === 'all' && t('Every recycling recipe: finds detours through other items, but the model grows to ~16 000 columns and takes minutes, so the planner asks before solving it (and you can cancel).', '所有回收配方：能找到经由其他物品的迂回路线，但模型会增长到约 16 000 列、需要几分钟，所以求解前会先询问（可取消）。')}
      </p>
      <AllowList data={data} list={s.recycle_allow} onChange={recycle_allow => set({ recycle_allow })} />
      <p className="hint">{t('Quality mining is not modelled yet: ores come in normal quality. Inputs can carry a quality (set it on the row). Catalyst recipes (Kovarex, pentapod eggs) roll quality on their whole output, catalyst included; not yet checked in game.', '尚未模拟品质采矿：矿石都是普通品质。输入物品可以带品质（在该行设置）。催化剂配方（Kovarex、五足虫卵）会对整个产出（含催化剂）随机品质；尚未在游戏中核实。')}</p>
    </section>
  )
}

/** Factorio: optional objective cost per held module (machine slots and beacons), by quality. */
function ModuleCosts({ data, s, set, hasQuality }: { data: GameData; s: Settings; set: (p: Partial<Settings>) => void; hasQuality: boolean }) {
  const on = s.module_costs.length > 0
  const tiers = hasQuality ? qualityNames(data) : ['Module']
  return (
    <>
      <label className="row small">
        <input type="checkbox" checked={on} onChange={e => set({ module_costs: e.target.checked ? (hasQuality ? [1, 3, 10, 30, 100] : [1]) : [] })} />
        <span>{t('Charge for modules', '计入插件成本')}</span>
      </label>
      {on && tiers.map((n, q) => (
        <div className="row small" key={n}>
          <span className="grow">{hasQuality && <QualityPip q={q} />}{hasQuality ? n : t('Per module', '每个插件')}</span>
          <input type="number" min={0} step="any" value={s.module_costs[q] ?? s.module_costs[s.module_costs.length - 1] ?? 0}
            aria-label={t(`Cost of one ${hasQuality ? n.toLowerCase() + ' ' : ''}module`, `一个${hasQuality ? n + '品质' : ''}插件的成本`)}
            onChange={e => {
              const c = tiers.map((_, j) => s.module_costs[j] ?? s.module_costs[s.module_costs.length - 1] ?? 0)
              c[q] = Math.max(0, +e.target.value || 0)
              set({ module_costs: c })
            }} />
        </div>
      ))}
      {on && <p className="hint">{t('Each module a machine holds (beacon modules included) adds this to the cost function, in its units (Raw: 1 = one ore per minute), so loadouts are not free. A module is bought once while the plan is per minute: a cost c spreads a module worth c × T ore over T minutes. Defaults are a rough guess; quality plans price modules at the module quality.', '机器持有的每个插件（含插件塔内的插件）都会按其单位计入成本函数（原矿：1 = 每分钟 1 个矿），所以插件配置不是免费的。插件只买一次，而方案按每分钟计：成本 c 把价值 c × T 个矿的插件摊到 T 分钟。默认值只是粗略估计；品质方案按插件品质计价。')}</p>}
    </>
  )
}

/** Beacon counts offered to the solver (rules::MAX_BEACON_COUNTS). */
const MAX_BEACON_COUNTS = 4

/** Factorio beacons: module, counts per machine (each count is a solver option), sharing, quality. */
function BeaconSettings({ data, s, set, hasQuality }: { data: GameData; s: Settings; set: (p: Partial<Settings>) => void; hasQuality: boolean }) {
  const b = data.beacons![0]
  const mods = (data.effects ?? []).filter(e => e.kind === 'module' && e.productivity <= 0 && (e.quality ?? 0) <= 0)
  // at most 4 counts: each one multiplies the solver's columns (quality plans: tiers × loadouts × counts)
  const counts = (t: string) => [...new Set(t.split(/[ ,;]+/).map(x => Math.round(+x)).filter(n => n > 0 && n <= 64))].sort((a, c) => a - c).slice(0, MAX_BEACON_COUNTS)
  return (
    <>
      <div className="row small">
        <span className="grow"><Icon id={b.id} />{dn(b, data.game)} {t('module', '插件')}</span>
        <select value={s.beacon_module} aria-label={t('Beacon module', '插件塔插件')} onChange={e => set({ beacon_module: e.target.value })}>
          <option value="">{t('no beacons', '不用插件塔')}</option>
          {mods.map(e => <option key={e.id} value={e.id}>{dn(e, data.game)} ×{b.modules}</option>)}
        </select>
      </div>
      {s.beacon_module && <>
        <div className="row small">
          <span className="grow">{t('Beacons per machine', '每台机器的插件塔数')}</span>
          <input type="text" inputMode="numeric" key={s.beacon_counts.join(',')} defaultValue={s.beacon_counts.join(', ')}
            aria-label={t('Beacons per machine (list)', '每台机器的插件塔数（列表）')} style={{ width: '7em' }}
            onBlur={e => { const c = counts(e.target.value); set({ beacon_counts: c.length ? c : [8] }) }}
            onKeyDown={e => { if (e.key === 'Enter') (e.target as HTMLInputElement).blur() }} />
        </div>
        <div className="row small">
          <span className="grow">{t('Machines sharing a beacon', '共用一个插件塔的机器数')}</span>
          <input type="number" min={1} max={8} step="any" value={s.beacon_share} aria-label={t('Machines sharing a beacon', '共用一个插件塔的机器数')}
            onChange={e => set({ beacon_share: Math.max(1, +e.target.value || 1) })} />
        </div>
        {hasQuality && <div className="row small">
          <span className="grow">{t('Beacon quality', '插件塔品质')}</span>
          <QualitySelect data={data} value={s.beacon_quality} label={t('Beacon quality', '插件塔品质')} onChange={beacon_quality => set({ beacon_quality })} />
        </div>}
      </>}
      <p className="hint">{s.beacon_module
        ? t(`Each count is an option per recipe (none is always one). n beacons give n × ${fmt(b.effectivity)} × 1/√n (FactorioLab profile) × their modules' effect, speed modules' quality penalty included; each costs ${fmt(1000 * b.power)} kW and one building, divided by the sharing. Up to ${MAX_BEACON_COUNTS} counts; geometry is not checked: usual layouts reach 8 beacons per machine in rows and 12 around a 3×3 machine.`, `每个数量都是该配方的一个选项（不用插件塔始终是其中之一）。n 个插件塔提供 n × ${fmt(b.effectivity)} × 1/√n（FactorioLab 曲线）× 其插件效果，已计入速度插件的品质惩罚；每个耗电 ${fmt(1000 * b.power)} kW 并算一个建筑，按共用数分摊。最多 ${MAX_BEACON_COUNTS} 个数量；不检查布局几何：常见布局每台机器排成行可达 8 个插件塔，围绕 3×3 机器可放 12 个。`)
        : t('Beacons: pick a speed or efficiency module to let the solver add beacons around machines.', '插件塔：选择速度或节能插件，求解器就会在机器周围加插件塔。')}</p>
    </>
  )
}

export default function Sidebar({ data, settings: s, setSettings }: Props) {
  const set = (patch: Partial<Settings>) => setSettings(x => ({ ...x, ...patch }))
  const [altFilter, setAltFilter] = useState('')
  const alts = data.recipes.filter(r => r.alt && r.name.toLowerCase().includes(altFilter.toLowerCase()))
  const altEnabled = (id: string) => s.allow_all_alternates || s.enabled_alternates.includes(id)
  const toggleAlt = (id: string) => {
    if (s.allow_all_alternates) {
      // switch to explicit list with everything but this one
      set({ allow_all_alternates: false, enabled_alternates: data.recipes.filter(r => r.alt && r.id !== id).map(r => r.id) })
    } else {
      set({ enabled_alternates: s.enabled_alternates.includes(id) ? s.enabled_alternates.filter(x => x !== id) : [...s.enabled_alternates, id] })
    }
  }
  const itemName = (id: string) => qItemName(data, id)
  const sf = data.game === 'satisfactory'
  const hasQuality = data.game === 'factorio' && (data.qualities?.length ?? 0) > 0
  const dsp = data.game === 'dsp'
  // buildings that compete for some recipe (machine tiers)
  const tiers = data.buildings.filter(b => data.recipes.some(r => (r.producers?.length ?? 0) > 1 && r.producers!.includes(b.id)))
  const toggle = (list: string[], id: string) => (list.includes(id) ? list.filter(x => x !== id) : [...list, id])
  const limitOf = (item: string, dflt: number | null) => (item in s.resource_limits ? s.resource_limits[item] : dflt)

  return (
    <aside>
      {data.game === 'factorio' && (
        <section>
          <h2>{t('Game data', '游戏数据')}</h2>
          <div className="row small">
            <span className="grow">{t('Factorio version', '异星工厂版本')}</span>
            <select value={s.dataset || '2.0'} aria-label={t('Factorio data set', '异星工厂数据集')} onChange={e => { const id = e.target.value; setSettings(x => withDataset(x, id)) }}>
              {DATASETS.map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
            </select>
          </div>
          {!hasQuality && <p className="hint">{t('Quality (legendary items, recyclers) needs a 2.1.25 or Space Age data set.', '品质（传说物品、回收机）需要 2.1.25 或太空时代数据集。')}</p>}
          <p className="hint">{t('Burner machines burn their default fuel as an input (boilers, stone / steel furnaces, heating towers: coal; reactors: uranium cells; biochambers: nutrients); heat exchangers use reactor heat (MJ). Generators and electricity production are not modelled.', '燃烧类机器把默认燃料当作输入（锅炉、石炉 / 钢炉、供热塔：煤；反应堆：铀燃料棒；生物反应室：营养素）；热交换器使用反应堆热量（MJ）。不模拟发电机和发电。')}</p>
          {(data.locations?.length ?? 0) > 1 && <>
            <div className="row small"><span className="grow">{t('Planets', '星球')}</span></div>
            <div className="chips" role="group" aria-label={t('Planets', '星球')}>
              {data.locations!.map(l => (
                <label key={l.id} className="chip">
                  <input type="checkbox" checked={s.planets.includes(l.id)}
                    onChange={() => set({ planets: toggle(s.planets, l.id) })} />{dn(l, data.game)}
                </label>
              ))}
            </div>
            <p className="hint">{t('Recipes and resources of other planets are off (Gleba plants, Vulcanus calcite, asteroid chunks on the space platform). Items from elsewhere can be added under Available inputs.', '其他星球的配方和资源默认关闭（句芒星的植物、祝融星的方解石、太空平台上的星块）。其他地方来的物品可以加在“可用输入”里。')}</p>
          </>}
        </section>
      )}

      <section>
        <h2>{t('Targets', '目标')}</h2>
        <ItemRates data={data} list={s.targets} onChange={targets => set({ targets })} producibleOnly quality={hasQuality} />
      </section>

      {sf && <SpaceElevator set={set} />}

      {hasQuality && <QualitySettings data={data} s={s} set={set} />}

      <section>
        <h2>{t('Optimise for', '优化目标')}</h2>
        <div className="seg">
          {(['resources', 'buildings', 'power', ...(dsp ? ['compute'] : []), 'custom'] as Objective[]).map(o => (
            <button key={o} className={s.objective === o ? 'on' : ''} title={o === 'compute' ? t('算力 / UPS: minimise compute per logic tick', '算力 / UPS：最小化每逻辑帧的算力') : undefined}
              onClick={() => set(o === 'custom' && s.objective !== 'custom'
                ? { objective: o, cost: s.objective === 'compute' ? preset('compute', s.generator)
                    : { ...preset(s.objective), resource_weights: s.cost.resource_weights, building_weights: s.cost.building_weights } }
                : { objective: o })}>
              {o === 'resources' ? t('Raw', '原矿') : o === 'buildings' ? t('Buildings', '建筑') : o === 'power' ? t('Power', '电力') : o === 'compute' ? t('Compute', '算力') : t('Custom', '自定义')}
            </button>
          ))}
        </div>
        <p className="hint">
          {s.objective === 'resources' && (sf ? t('Raw resources weighted by scarcity (Iron Ore = 1, uranium ≈ 44); buildings break ties.', '原矿按稀缺度加权（铁矿 = 1，铀 ≈ 44）；建筑数用于打破平局。') : data.game === 'factorio' && hasQuality ? t('Raw resources (ores 1, water 0.01, oil 0.2, drops 10, asteroid chunks 100); buildings break ties.', '原矿（矿石 1，水 0.01，原油 0.2，掉落物 10，星块 100）；建筑数用于打破平局。') : t('Raw resources (ores 1, water 0.01, oil 0.2, drops 10); buildings break ties.', '原矿（矿石 1，水 0.01，原油 0.2，掉落物 10）；建筑数用于打破平局。'))}
          {s.objective === 'buildings' && t('Fewest whole buildings; raw resources break ties.', '整数建筑数最少；原矿用于打破平局。')}
          {s.objective === 'power' && t('MW (production + extraction) + 1 per building, so it underclocks where that pays.', 'MW（生产 + 开采）+ 每个建筑 1，所以划算时会降频。')}
          {s.objective === 'compute' && t('CPU per logic tick: every building costs the same whether working or idle, plus its sorters, belts, stations, coaters and the generators for its power. The solver picks tier and proliferator mode per recipe.', '每逻辑帧的 CPU：每个建筑无论工作还是空闲成本相同，再加上它的分拣器、传送带、物流站、喷涂机和为它供电的发电机。求解器为每个配方选择机器档位和增产剂模式。')}
          {s.objective === 'custom' && t('Minimise the weighted sum below. Edit any weight; the plan re-solves.', '最小化下方的加权和。修改任一权重，方案会重新求解。')}
        </p>
        {sf && data.items.some(i => i.id === 'Electricity') && <>
          <label className="row small">
            <input type="checkbox" checked={s.self_power} onChange={e => set({ self_power: e.target.checked })} />
            <span>{t("Self-powered: build generators for the plan's own power", '自供电：为方案自身的用电建造发电机')}</span>
          </label>
          <p className="hint">{t('Generators are recipes: fuel (+ water) in, Electricity (MW) out. Add Electricity as a target to plan a power plant on its own.', '发电机也是配方：输入燃料（+ 水），输出电力（MW）。把电力加为目标，即可单独规划一座电厂。')}</p>
        </>}
        {dsp && <>
          <div className="row small">
            <span className="grow">{t('Power from', '电力来源')}</span>
            <select value={s.generator} onChange={e => set({ generator: e.target.value as Generator })} aria-label={t('Generator', '发电机')}>
              {GENERATORS.map(g => <option key={g.id} value={g.id}>{t(g.name_en, g.name)}</option>)}
            </select>
          </div>
          <label className="row small">
            <input type="checkbox" checked={s.mixed_belts} onChange={e => set({ mixed_belts: e.target.checked })} />
            <span>{t('Mixed input belts (one input sorter per building)', '混合输入传送带（每个建筑一个输入分拣器）')}</span>
          </label>
          {s.objective !== 'compute' && (
            <p className="hint">{t('Generator and belts set how the compute card counts this plan', '发电机和传送带决定算力卡如何统计本方案')}{s.objective === 'custom' && s.cost.sorter > 0 ? t(' and, through the sorter weight, the plan itself', '，并通过分拣器权重影响方案本身') : ''}{t('.', '。')}</p>
          )}
          {s.objective === 'compute' && !s.effects.some(e => e.startsWith('proliferator')) && (
            <p className="hint">{t('No proliferator mode is enabled, so every recipe runs unsprayed.', '没有启用任何增产剂模式，所以所有配方都不喷涂。')}{' '}
              <a href="#" onClick={e => { e.preventDefault(); set({ effects: [...s.effects, 'proliferator-3-speed', 'proliferator-3-products'] }) }}>{t('Enable Mk.III (both modes)', '启用 Mk.III（两种模式）')}</a>
            </p>
          )}
          {s.objective === 'compute' && <p className="hint">{t('Coefficients: dsp-ups research model (machine = 1, sorter 0.12, belts + station 0.13 per building; estimates, low confidence). Mining machines, orbital collectors and ray receivers are not counted.', '系数：dsp-ups 研究模型（机器 = 1，分拣器 0.12，传送带 + 物流站每建筑 0.13；为估计值，可信度低）。采矿机、轨道采集器和射线接收站不计入。')}</p>}
        </>}
        {s.objective === 'custom' && <CostEditor data={data} cost={s.cost} onChange={cost => set({ cost })} />}
      </section>

      <section>
        <h2>{t('Solver', '求解器')}</h2>
        <label className="row">
          <input type="checkbox" checked={s.integer} onChange={e => set({ integer: e.target.checked })} />
          <span>{t('Whole buildings and shards (integer program)', '整数建筑和碎片（整数规划）')}</span>
        </label>
        {s.integer && (
          <div className="row small">
            <span className="grow">{t('Time limit', '时间限制')}</span>
            <TimeInput ms={s.time_limit_ms} onChange={time_limit_ms => set({ time_limit_ms })} />
            <span className="muted">s</span>
          </div>
        )}
        {s.integer && (
          <label className="row small" title={t('microlp is built in; HiGHS (open source, MIT) is downloaded on first use, about 1.2 MB', 'microlp 内置；HiGHS（开源，MIT）首次使用时下载，约 1.2 MB')}>
            <span className="grow">{t('Solver engine', '求解引擎')}</span>
            <select value={s.engine ?? 'auto'} onChange={e => set({ engine: e.target.value as Engine })}>
              <option value="auto">{t('Auto', '自动')}</option>
              <option value="microlp">microlp</option>
              <option value="highs">HiGHS</option>
            </select>
          </label>
        )}
        <p className="hint">
          {s.integer
            ? t('Branch-and-bound over whole buildings; each building can be underclocked freely. On timeout the best plan found is shown with its gap to the lower bound.', '对整数建筑做分支定界；每个建筑可自由降频。超时则显示已找到的最优方案及其与下界的差距。')
            : t('Continuous LP: fractional buildings (the old "solve, then round up" view).', '连续线性规划：建筑数可为小数（旧的“先求解、再向上取整”视图）。')}
        </p>
        {sf && <>
        <div className="row">
          <span>{t('Max clock', '最大时钟')}</span>
          <input type="range" min={1} max={250} value={s.max_clock_percent} className="grow" aria-label={t('Max clock', '最大时钟')}
            onChange={e => set({ max_clock_percent: +e.target.value })} />
          <input type="number" min={1} max={250} value={s.max_clock_percent}
            onChange={e => set({ max_clock_percent: Math.min(250, Math.max(1, +e.target.value)) })} />
          <span className="muted">%</span>
        </div>
        {Object.keys(s.recipe_clocks).length > 0 && (
          <div className="row small">
            <span className="grow">{t(`Per-recipe max clock: ${Object.keys(s.recipe_clocks).length} set (in the recipe table)`, `按配方的最大时钟：已设置 ${Object.keys(s.recipe_clocks).length} 个（在配方表中）`)}</span>
            <button type="button" className="tabbtn" onClick={() => set({ recipe_clocks: {} })}>{t('Clear', '清除')}</button>
          </div>
        )}
        {(s.max_clock_percent > 100 || Object.values(s.recipe_clocks).some(c => c > 100)) && (
          <div className="row small">
            <span className="grow">{t('Power shards available', '可用能量碎片')}</span>
            <input type="number" min={0} placeholder="∞" value={s.shard_budget ?? ''}
              onChange={e => set({ shard_budget: e.target.value === '' ? null : Math.max(0, Math.round(+e.target.value)) })} />
          </div>
        )}
        <p className="hint">{t('Above 100 % a building needs one shard per started 50 %. Power ∝ clock', '超过 100 % 后，每多 50 %（不足也算）需要一个能量碎片。电力 ∝ 时钟')}<sup>1.32</sup>{t('.', '。')}</p>
        <label className="row">
          <input type="checkbox" checked={s.auto_somersloops} onChange={e => set({ auto_somersloops: e.target.checked })} />
          <span>{t('Place Somersloops automatically', '自动放置索莫晶体')}</span>
        </label>
        <div className="row small">
          <span className="grow">{t('Somersloops available', '可用索莫晶体')}</span>
          <input type="number" min={0} placeholder="∞" value={s.somersloop_budget ?? ''}
            onChange={e => set({ somersloop_budget: e.target.value === '' ? null : Math.max(0, Math.round(+e.target.value)) })} />
        </div>
        <p className="hint">{t("Each doubles a building's output at 4× its power. Fix a recipe's count in the plan table; the rest are placed within the budget.", '每个让建筑产出翻倍，耗电变为 4 倍。可在方案表中固定某配方的数量；其余在预算内自动放置。')}</p>
        </>}
      </section>

      {!sf && (
        <section>
          <h2>{t('Machines', '机器')}</h2>
          <p className="hint">{t('Tiers the solver may use; it picks per recipe by the cost function.', '求解器可使用的机器档位；按成本函数为每个配方选择。')}</p>
          <div className="scroll">
            {tiers.map(b => (
              <label className="row small" key={b.id}>
                <input type="checkbox" checked={!s.disabled_buildings.includes(b.id)}
                  onChange={() => set({ disabled_buildings: toggle(s.disabled_buildings, b.id) })} />
                <span className="grow"><Icon id={b.id} />{dn(b, data.game)}</span>
                <span className="muted">×{b.speed ?? 1}</span>
              </label>
            ))}
          </div>
        </section>
      )}

      {!sf && (data.effects?.length ?? 0) > 0 && (
        <section>
          <h2>{data.game === 'dsp' ? t('Proliferator', '增产剂') : t('Modules', '插件')}</h2>
          <p className="hint">
            {data.game === 'dsp'
              ? t('Enabled modes become options per recipe; spray use (one per input item) is added as an input, so the proliferator chain is planned too. Add it under Available inputs to treat it as free.', '启用的模式会成为每个配方的选项；喷涂消耗（每个输入物品一个）作为输入计入，所以增产剂产业链也会一并规划。把它加到“可用输入”里即可视为免费。')
              : t(`Each enabled module fills all slots of a machine${hasQuality ? ' (at the module quality set under Quality; quality plans mix quality and productivity modules per machine instead)' : ''}. The foundry / EM plant / biochamber built-in +50 % applies to every recipe they run; productivity modules and research only where the recipe allows them; the total is capped at +300 %.`, `每个启用的插件会填满机器的所有插槽${hasQuality ? '（品质为“品质”部分设置的插件品质；品质方案则改为每台机器混合使用品质插件和产能插件）' : ''}。铸造厂 / 电磁工厂 / 生物反应室自带的 +50 % 适用于它们运行的所有配方；产能插件和研究只在配方允许时生效；总计上限 +300 %。`)}
          </p>
          {data.effects!.map(e => (
            <label className="row small" key={e.id}>
              <input type="checkbox" checked={s.effects.includes(e.id)} onChange={() => set({ effects: toggle(s.effects, e.id) })} />
              <span className="grow"><Icon id={e.item || e.id} />{dn(e, data.game)}</span>
              <span className="muted">
                {e.productivity ? `+${fmt(100 * e.productivity)}% ${t('out', '产出')} ` : ''}{e.speed ? `${e.speed > 0 ? '+' : ''}${fmt(100 * e.speed)}% ${t('speed', '速度')} ` : ''}
              </span>
            </label>
          ))}
          {(data.beacons?.length ?? 0) > 0 && <BeaconSettings data={data} s={s} set={set} hasQuality={hasQuality} />}
          {data.game === 'factorio' && <ModuleCosts data={data} s={s} set={set} hasQuality={hasQuality} />}
        </section>
      )}

      {(data.research?.length ?? 0) > 0 && (
        <section>
          <h2>{t('Research productivity', '研究增产')}</h2>
          <p className="hint">{t('Levels of the infinite productivity technologies (+10 % each), counted with modules and built-in productivity against the +300 % cap. Research changes the recipe itself, so it also applies where productivity modules are refused (scrap recycling in the recycler).', '无限增产科技的等级（每级 +10 %），与插件和自带增产一起计入 +300 % 上限。研究改变的是配方本身，所以在不接受产能插件的地方（回收机里的废料回收）同样生效。')}</p>
          {data.research!.map(f => {
            // the family's recipes are set together; show the highest if a link set them apart
            const level = Math.round(Math.max(0, ...f.recipes.map(r => s.research_productivity[r] ?? 0)) / f.step)
            const setLevel = (n: number) => {
              const rp = { ...s.research_productivity }
              for (const r of f.recipes) { if (n > 0) rp[r] = +(n * f.step).toFixed(6); else delete rp[r] }
              set({ research_productivity: rp })
            }
            return (
              <div className="row small" key={f.id} title={f.recipes.map(id => dn(data.recipes.find(r => r.id === id), data.game, id)).join(', ')}>
                <span className="grow">{lang() === 'zh' && f.name_zh ? f.name_zh : f.name.replace(/ productivity$/, '')}</span>
                <input type="number" min={0} max={300} step={1} value={level} aria-label={t(`${f.name} level`, `${dn(f, data.game)} 等级`)}
                  onChange={e => setLevel(Math.min(300, Math.max(0, Math.round(+e.target.value || 0))))} />
                <span className="muted">+{fmt(100 * level * f.step, 0)}%</span>
              </div>
            )
          })}
        </section>
      )}

      <section>
        <h2>{t('Alternate recipes', '替代配方')}</h2>
        <label className="row">
          <input type="checkbox" checked={s.allow_all_alternates}
            onChange={e => set({ allow_all_alternates: e.target.checked, enabled_alternates: [] })} />
          <span>{t(`Allow all ${data.recipes.filter(r => r.alt).length} alternates`, `允许全部 ${data.recipes.filter(r => r.alt).length} 个替代配方`)}</span>
        </label>
        {sf && <label className="row">
          <input type="checkbox" checked={s.allow_ore_conversion} onChange={e => set({ allow_ore_conversion: e.target.checked })} />
          <span>{t('Converter ore conversion (e.g. Bauxite from Copper)', '转化器矿石转换（例如铜矿转铝土矿）')}</span>
        </label>}
        <details>
          <summary>{t(`Choose individually (${s.allow_all_alternates ? 'all' : s.enabled_alternates.length} on)`, `逐个选择（已开 ${s.allow_all_alternates ? '全部' : s.enabled_alternates.length}）`)}</summary>
          <input className="wide" placeholder={t('Filter by name', '按名称筛选')} value={altFilter} onChange={e => setAltFilter(e.target.value)} />
          <div className="scroll">
            {alts.map(r => (
              <label className="row small" key={r.id}>
                <input type="checkbox" checked={altEnabled(r.id)} onChange={() => toggleAlt(r.id)} />
                <span>{dn(r, data.game)}</span>
              </label>
            ))}
          </div>
        </details>
        {s.disabled_recipes.length > 0 && (
          <p className="hint">
            {t(`${s.disabled_recipes.length} recipe(s) excluded in the plan table.`, `方案表中已排除 ${s.disabled_recipes.length} 个配方。`)}{' '}
            <a href="#" onClick={e => { e.preventDefault(); set({ disabled_recipes: [] }) }}>{t('Re-enable all', '全部重新启用')}</a>
          </p>
        )}
      </section>

      <section>
        <h2>{t('Available inputs', '可用输入')}</h2>
        <p className="hint">{t('Items you already produce elsewhere (free, up to the rate).', '你在别处已经生产的物品（免费，上限为设定速率）。')}</p>
        <ItemRates data={data} list={s.supplies} onChange={supplies => set({ supplies })} quality={hasQuality} />

      </section>

      {sf && <section>
        <h2>{t('Resource limits', '资源上限')}</h2>
        <label className="row">
          <input type="checkbox" checked={s.ignore_limits} onChange={e => set({ ignore_limits: e.target.checked })} />
          <span>{t('Ignore limits', '忽略上限')}</span>
        </label>
        <details>
          <summary>{t('World maximum per minute (editable)', '全图每分钟最大产量（可编辑）')}</summary>
          {data.resources.map(r => {
            const v = limitOf(r.item, r.limit)
            return (
              <div className="row small" key={r.item}>
                <span className="grow">{itemName(r.item)}</span>
                <input type="number" min={0} placeholder="∞" value={v ?? ''}
                  onChange={e => set({ resource_limits: { ...s.resource_limits, [r.item]: e.target.value === '' ? null : +e.target.value } })} />
              </div>
            )
          })}
          <a href="#" onClick={e => { e.preventDefault(); set({ resource_limits: {} }) }}>{t('Reset to world maxima', '重置为全图最大值')}</a>
        </details>
      </section>}

      <section>
        <button className="wide" onClick={() => setSettings(() => defaultSettings(s.game))}>{t('Reset all settings for this game', '重置本游戏的所有设置')}</button>
        <p className="hint">{t('The page address holds the whole plan. Copy it to share or bookmark this plan.', '页面地址包含整个方案。复制即可分享或收藏。')}</p>
      </section>
    </aside>
  )
}

/** Satisfactory: Space Elevator phase as targets per minute, delivered in a given time (memo step 16:
 *  「最后的4种太空电梯材料按时间的规划」). */
function SpaceElevator({ set }: { set: (p: Partial<Settings>) => void }) {
  const [phase, setPhase] = useState(5)
  // typed freely, read on "Set as targets" (review: snapping to 1 made 0.5 untypeable)
  const [hours, setHours] = useState('8')
  const [mult, setMult] = useState(1)
  const h = Number(hours) > 0 ? Number(hours) : 8
  return (
    <details className="sidesection">
      <summary>{t('Space Elevator by time', '太空电梯按时间规划')}</summary>
      <div className="row small">
        <span className="grow">{t('Phase', '阶段')}</span>
        <select value={phase} onChange={e => setPhase(+e.target.value)} aria-label={t('Phase', '阶段')}>
          {[1, 2, 3, 4, 5].map(p => <option key={p} value={p}>{t(`Phase ${p}`, `第 ${p} 阶段`)}</option>)}
        </select>
      </div>
      {phase === 5 && <p className="hint">{t('Phase 5 = the four final parts: Nuclear Pasta, Biochemical Sculptor, Ballistic Warp Drive, AI Expansion Server.', '第 5 阶段 = 最后四种：核意面、生化塑造器、弹道跃迁引擎、AI拓展伺服器。')}</p>}
      <div className="row small">
        <span className="grow">{t('Deliver in (hours)', '交付用时（小时）')}</span>
        <input type="number" min={0.1} step={0.5} value={hours} aria-label={t('Hours', '小时')} onChange={e => setHours(e.target.value)} />
      </div>
      <div className="row small">
        <span className="grow">{t('Cost multiplier (game setting)', '成本倍率（开档设置）')}</span>
        <select value={mult} onChange={e => setMult(+e.target.value)} aria-label={t('Cost multiplier', '成本倍率')}>
          {[0.25, 0.5, 0.75, 1, 2, 5, 10, 25, 50, 100].map(m => <option key={m} value={m}>{m}×</option>)}
        </select>
      </div>
      <button type="button" onClick={() => set({ targets: elevatorTargets(phase, h, mult) })}>{t('Set as targets', '设为目标')}</button>
      <p className="hint">{t('Delivery amounts from the Satisfactory wiki (Project Assembly, 1.0); the rate is the amount ÷ the time, so the factory delivers the phase in that time once it runs.', '交付数量取自幸福工厂 wiki（太空电梯，1.0）；速率 = 数量 ÷ 用时，工厂建好运行后在这段时间内交齐。')}</p>
    </details>
  )
}
