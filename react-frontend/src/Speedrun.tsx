// Satisfactory speedrun plan (memo step 17): climb the tech tree stage by stage, building factories as
// they are needed; model in src/planner/speedrun.rs. Runs in the galaxy worker (a plain WASM worker).
import { useEffect, useMemo, useRef, useState } from 'react'
import { Icon } from './Icon'
import { lang, t } from './i18n'
import { fmt, itemName, type GameData, type ItemRate } from './solver'

interface Stage {
  kind: 'milestone' | 'phase'; id: string; name: string; tier: number; cost: ItemRate[]
  buildings: number; miners: number; build: { building: string; count: number }[]
  build_min: number; produce_min: number; wait_min: number; start_min: number; end_min: number
  total_buildings: number; unlocks: string[]; note: string
}
interface Report { status: string; stages: Stage[]; total_min: number; total_buildings: number; done_in_hours: number | null; solve_ms: number; warnings: string[]; error?: string }
interface Opts { up_to_tier: number; place_s: number; wire_s: number; elevator: boolean; hours: number }
const DEFAULTS: Opts = { up_to_tier: 9, place_s: 25, wire_s: 20, elevator: true, hours: 8 }
const KEY = 'speedrun.v1'

let worker: Worker | null = null
let nextId = 1
const waiting = new Map<number, (s: string) => void>()
function run(payload: unknown): Promise<Report> {
  if (!worker) {
    worker = new Worker(new URL('./galaxyWorker.ts', import.meta.url), { type: 'module' })
    worker.onmessage = (e: MessageEvent<{ id: number; out: string }>) => { waiting.get(e.data.id)?.(e.data.out); waiting.delete(e.data.id) }
    worker.onerror = e => {
      const out = JSON.stringify({ error: `${e.message || 'worker failed'}` })
      waiting.forEach(res => res(out)); waiting.clear(); worker?.terminate(); worker = null
    }
  }
  const id = nextId++
  return new Promise(res => { waiting.set(id, out => res(JSON.parse(out) as Report)); worker!.postMessage({ id, fn: 'speedrun', json: JSON.stringify(payload) }) })
}

/** 1:05 h, or 25 min. */
const hm = (min: number) => (min >= 60 ? `${Math.floor(min / 60)}:${String(Math.round(min % 60)).padStart(2, '0')} h` : `${fmt(min, 0)} min`)

export default function Speedrun({ data }: { data: GameData }) {
  const [o, setO] = useState<Opts>(() => {
    try { return { ...DEFAULTS, ...(JSON.parse(localStorage.getItem(KEY) ?? 'null') ?? {}) } } catch { return DEFAULTS }
  })
  useEffect(() => { try { localStorage.setItem(KEY, JSON.stringify(o)) } catch { /* ignore */ } }, [o])
  const set = (p: Partial<Opts>) => setO(x => ({ ...x, ...p }))
  const [r, setR] = useState<Report | null>(null)
  const [busy, setBusy] = useState(false)
  const [sentKey, setSentKey] = useState('')
  const [open, setOpen] = useState<string | null>(null)
  const payload = useMemo(() => JSON.stringify({ ...o, lang: lang() }), [o])
  const gen = useRef(0)
  const go = () => {
    const g = ++gen.current, sent = payload
    setBusy(true)
    void run(JSON.parse(sent)).then(x => { if (g !== gen.current) return; setBusy(false); setR(x); setSentKey(sent) })
  }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { go() }, [])
  const stale = !!r && !r.error && sentKey !== payload
  const limit = o.hours > 0 ? o.hours * 60 : Infinity
  const num = (x: number, d = 0) => fmt(x, d)

  return (
    <div className="mixedbelt speedrun">
      <p className="hint">
        {t('Speedrun plan: climb the tech tree milestone by milestone (cheapest one that can be made first in each tier, Space Elevator phases where they gate the next tiers). For each step the plan adds factories sized so that building them plus producing the cost takes the least time; factories built earlier keep running and feed later steps. Building time = buildings × (place + wiring seconds).',
          '速通规划：按里程碑一步步爬科技树（每一阶先做能做出来的最便宜的里程碑，太空电梯阶段放在它解锁下一阶之前）。每一步新建工厂，工厂规模取「建造时间 + 生产时间」最短；之前建好的工厂继续运转，产出给后面用。建造时间 = 建筑数 × （摆放 + 拉线秒数）。')}
      </p>
      <div className="mbsettings">
        <label>{t('Up to tier', '爬到第几阶')}<input type="number" min={1} max={9} value={o.up_to_tier} onChange={e => set({ up_to_tier: Math.min(9, Math.max(1, Math.round(+e.target.value || 9))) })} /></label>
        <label>{t('Seconds to place a building', '每个建筑摆放秒数')}<input type="number" min={0} max={600} value={o.place_s} onChange={e => set({ place_s: Math.max(0, +e.target.value || 0) })} /></label>
        <label>{t('Seconds of belts / power per building', '每个建筑拉线秒数')}<input type="number" min={0} max={600} value={o.wire_s} onChange={e => set({ wire_s: Math.max(0, +e.target.value || 0) })} /></label>
        <label className="check"><input type="checkbox" checked={o.elevator} onChange={e => set({ elevator: e.target.checked })} /> {t('Space Elevator phases', '太空电梯阶段')}</label>
        <label>{t('Show what fits in (hours)', '看几小时内能爬到哪')}<input type="number" min={0} max={1000} step={1} value={o.hours} onChange={e => set({ hours: Math.max(0, +e.target.value || 0) })} /></label>
        <button onClick={go} disabled={busy}>{busy ? t('Planning…', '规划中…') : t('Plan the climb', '规划爬科技')}</button>
      </div>
      {r?.error && <p className="warn">{r.error}</p>}
      {stale && <p className="warn stale" role="status"><b>{t('Out of date', '结果已过期')}</b>{t(': settings changed. Press Plan the climb.', '：设置改了，按「规划爬科技」重新计算。')}</p>}
      {r && !r.error && <div className={stale ? 'staleresult' : undefined}>
        <div className="cards">
          <div className="card"><span className="big">{hm(r.total_min)}</span><span className="muted">{t(`to finish (${r.stages.length} steps)`, `爬完（${r.stages.length} 步）`)}</span></div>
          <div className="card"><span className="big">{num(r.total_buildings)}</span><span className="muted">{t('buildings at the end', '最后的建筑数')}</span></div>
          {r.done_in_hours !== null && <div className="card"><span className="big">{r.done_in_hours}</span><span className="muted">{t(`steps done in ${o.hours} h`, `${o.hours} 小时内完成的步数`)}</span></div>}
        </div>
        {r.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
        <div className="tablewrap">
          <table className="mbtable">
            <thead><tr>
              <th>{t('Step', '步骤')}</th><th className="num">{t('Tier', '阶')}</th><th className="num">{t('New buildings', '新建筑')}</th>
              <th className="num">{t('Build', '建造')}</th><th className="num">{t('Produce', '生产')}</th><th className="num">{t('Done at', '完成时间')}</th><th className="num">{t('Total buildings', '累计建筑')}</th>
            </tr></thead>
            <tbody>
              {r.stages.map(s => [
                <tr key={s.id} className={`clickable${s.end_min > limit ? ' muted' : ''}`} onClick={() => setOpen(open === s.id ? null : s.id)}>
                  <td><span className="caret">{open === s.id ? '▾' : '▸'}</span>{s.kind === 'phase' ? <b>{s.name}</b> : s.name}</td>
                  <td className="num">{s.kind === 'phase' ? '—' : s.tier}</td>
                  <td className="num">{num(s.buildings)}{s.miners > 0 ? ` + ${num(s.miners)}` : ''}</td>
                  <td className="num">{hm(s.build_min)}</td>
                  <td className="num">{hm(s.produce_min)}</td>
                  <td className="num">{hm(s.end_min)}</td>
                  <td className="num">{num(s.total_buildings)}</td>
                </tr>,
                open === s.id && <tr key={`${s.id}-d`}><td colSpan={7} className="small">
                  <div><b>{t('Cost', '花费')}</b>{t(': ', '：')}{s.cost.map(c => <span key={c.item} className="chipitem"><Icon id={c.item} size={16} />{num(c.rate)} {itemName(data, c.item)} </span>)}</div>
                  <div><b>{t('Build', '新建')}</b>{t(': ', '：')}{s.build.map(b => `${b.building} ×${num(b.count)}`).join(' · ') || t('nothing new (earlier factories make it)', '不用新建（之前的工厂就能做）')}</div>
                  {s.unlocks.length > 0 && <div><b>{t('Unlocks', '解锁')}</b>{t(': ', '：')}{s.unlocks.join('、')}</div>}
                  {s.wait_min > 0.05 && <div>{t(`Waits ${hm(s.wait_min)} for the HUB ship.`, `等 HUB 飞船 ${hm(s.wait_min)}。`)}</div>}
                </td></tr>,
              ])}
            </tbody>
          </table>
        </div>
        <p className="small muted">
          {t('Not modelled yet: power plants and power, hand-crafting at the start, walking and exploring, node purity and distance, MAM research cost, alternate recipes (hard drives). Miners count as Mk.1 / Mk.2 / Mk.3 on normal nodes as they unlock; the parts of each step\'s buildings are made by that step\'s factories. ',
            '暂未建模：发电厂和电力、开局手搓、跑图探索、矿点纯度和距离、MAM 研究花费、替代配方（硬盘）。采矿机按解锁情况算一/二/三级、普通矿点；每一步新建筑的材料由这一步的工厂生产。')}
          {fmt(r.solve_ms / 1000, 1)} s.
        </p>
      </div>}
    </div>
  )
}
