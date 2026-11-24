// DSP mixed-belt (混带) tab: layout per recipe from src/planner/mixed_belt.rs
// (docs/research/dsp-mixed-belt/00-最终报告.md §3). Settings live in Settings.belt_layout and do not re-solve.
import { useMemo, useState } from 'react'
import { Icon } from './Icon'
import { StationDiagram } from './StationDiagram'
import { t as tr } from './i18n'
import {
  cleanBeltLayout, defaultBeltLayout, fmt, mixedBelt, type BeltLayoutSettings, type Plan, type RecipeLayout, type Settings,
  type TwoOverride,
} from './solver'

interface Props {
  plan: Plan | null
  /** The settings `plan` was solved for (null: unknown, use the current ones). */
  planSettings: Settings | null
  /** `plan` is the plan of the current settings (false: a new solve is running, was stopped or failed). */
  current: boolean
  settings: Settings
  setSettings: (f: (s: Settings) => Settings) => void
}

const beltNames = () => [tr('Yellow belt Mk.I', '黄带 Mk.I'), tr('Green belt Mk.II', '绿带 Mk.II'), tr('Blue belt Mk.III', '蓝带 Mk.III')]
const sorterNames = () => [tr('Yellow sorter Mk.I', '黄爪 Mk.I'), tr('Green sorter Mk.II', '绿爪 Mk.II'), tr('Blue sorter Mk.III', '蓝爪 Mk.III')]
const pileNames = () => ['Lv0', 'Lv5', tr('max', '满级')]
const tierName = (k: string): string => (
  k === 'yellow' ? tr('yellow sorter', '黄爪') : k === 'green' ? tr('green sorter', '绿爪') : k === 'blue' ? tr('blue sorter', '蓝爪') : k === 'pile' ? tr('pile sorter', '白爪') : k
)
const schemeName = (k: string): string => (
  k === 'A' ? tr('A: control unit + direct refill', 'A 控制单元 + 直怼') : k === 'B' ? tr('B: ladder group', 'B 梯口机组') : k === 'none' ? tr('not mixable', '不可混带') : k
)
const unitName = (k: string): string => (
  k === 'standard' ? tr('standard', '标准') : k === 'double' ? tr('two green belts', '双绿带') : k === 'unknown' ? tr('beyond known structures', '超出已知结构') : k
)
const SERIES = ['var(--accent)', 'var(--raw)', 'var(--sup)', 'var(--liquid)']
const f2 = (x: number) => fmt(x, 3)

/** Scheme-A order along the belt as copyable text (report §3 item 4). */
function sequenceText(l: RecipeLayout): string {
  const t = l.two
  if (!t) return ''
  const nm = (i: number) => l.inputs[i].name
  const lim = 1 - t.jump
  const parts = [tr(`control unit (${nm(lim)} on the limiter lane ${Math.min(t.sigma5, 5 - t.sigma5)}/5, ${nm(t.jump)} on the jump lane ${Math.max(t.sigma5, 5 - t.sigma5)}/5)`, `控制单元(${nm(lim)} 走限流格 ${Math.min(t.sigma5, 5 - t.sigma5)}/5,${nm(t.jump)} 跳格 ${Math.max(t.sigma5, 5 - t.sigma5)}/5)`)]
  const segs: string[] = []
  let at = 0
  for (const f of t.feeds) {
    segs.push(tr(`${f.pos - at} machines → refill ${nm(f.item)} ${fmt(f.amount, 3)}`, `${f.pos - at} 台 → 补${nm(f.item)} ${fmt(f.amount, 3)}`))
    at = f.pos
  }
  // compress repeats of a two-refill pattern
  const out: string[] = []
  for (let i = 0; i < segs.length;) {
    let n = 1
    while (i + 2 * n + 1 < segs.length && segs[i + 2 * n] === segs[i] && segs[i + 2 * n + 1] === segs[i + 1]) n++
    if (n >= 2) { out.push(`[${segs[i]} → ${segs[i + 1]}] × ${n}`); i += 2 * n } else { out.push(segs[i]); i++ }
  }
  parts.push(...out)
  const tail = t.machines_per_unit - at
  const r = t.returns
  const ok = (i: number) => (r[i] <= t.shares[i] + 1e-9 ? '✓' : '✗')
  const tot = r[0] + r[1]
  parts.push(tr(`${tail} machines (remainder)`, `${tail} 台(零头)`))
  const unit = t.unit === 'standard' ? tr('≤ 12 → standard unit', '≤ 12 → 标准单元') : t.unit === 'double' ? tr('> 12 → two-green-belt unit', '> 12 → 双绿带单元') : tr('> 12, and beyond the two-green-belt unit (jump item ≤ 24, limiter item ≤ 12) → no known structure', '> 12,且超出双绿带单元(跳格物品 ≤ 24、限流物品 ≤ 12)→ 无已知结构')
  parts.push(tr(`back to the control unit (return: ${nm(0)} ${fmt(r[0], 3)} ≤ ${fmt(t.shares[0], 3)} ${ok(0)}, ${nm(1)} ${fmt(r[1], 3)} ≤ ${fmt(t.shares[1], 3)} ${ok(1)}, total ${fmt(tot, 3)} ${unit})`, `回控制单元(回流:${nm(0)} ${fmt(r[0], 3)} ≤ ${fmt(t.shares[0], 3)} ${ok(0)},${nm(1)} ${fmt(r[1], 3)} ≤ ${fmt(t.shares[1], 3)} ${ok(1)},合计 ${fmt(tot, 3)} ${unit})`))
  return `${parts.join(' → ')}${t.units > 1 ? tr(`; ${t.units} control units in all`, `;共 ${t.units} 个控制单元`) : ''}`
}

function ladderText(l: RecipeLayout): string {
  const g = l.ladder
  if (!g) return ''
  const feeds = g.feeds.map(f => tr(`feed ${l.inputs[f.item].name} ${fmt(f.rate, 3)} (${f.combo}) + 1 buffer lane`, `投${l.inputs[f.item].name} ${fmt(f.rate, 3)}(${f.combo})+ 1 格缓存`)).join(' → ')
  return tr(`${feeds} → ${g.group} machines → recovery × ${g.feeds.length}; ${g.groups} groups${g.last_group !== g.group ? ` (last group ${g.last_group} machines)` : ''}`, `${feeds} → ${g.group} 台 → 回收 × ${g.feeds.length};${g.groups} 组${g.last_group !== g.group ? `(最后一组 ${g.last_group} 台)` : ''}`)
}

/** The 1D belt diagram: belt strip with machines and feeds, flow lines below (report §3). */
function BeltDiagram({ l, cap, margin, endPile }: { l: RecipeLayout; cap: number; margin: number; endPile: number }) {
  const pts = l.trace.points
  const n = pts.filter(p => p.kind === 'machine').length
  if (!n) return null
  const px = n <= 60 ? 14 : n <= 200 ? 8 : 5
  const left = 64
  const w = left + n * px + 40
  const stripY = 30, stripH = 16, chartTop = 78, chartH = 150
  const y = (v: number) => chartTop + chartH - (Math.max(-cap * 0.1, Math.min(cap, v)) / cap) * chartH
  const x = (pos: number) => left + pos * px
  const items = l.inputs.length
  const input = l.sorters.input
  // flow lines: one point per event at its position (machines step down, feeds jump up)
  const paths = Array.from({ length: items }, (_, i) => {
    let d = `M${x(0)},${y(pts[0]?.flows[i] ?? 0)}`
    for (const p of pts) d += ` L${x(p.pos)},${y(p.flows[i])}`
    return d
  })
  // segments under the margin
  const low: { x1: number; x2: number; i: number }[] = []
  for (let i = 0; i < (l.scheme === 'A' ? items : 0); i++) {
    for (let j = 1; j < pts.length; j++) {
      if (pts[j].kind === 'machine' && pts[j].flows[i] < margin - 1e-9) low.push({ x1: x(pts[j].pos - 1), x2: x(pts[j].pos), i })
    }
  }
  const feeds = pts.filter(p => (p.kind === 'fill' || p.kind === 'feed') && p.item !== null)
  const recover = pts.filter(p => p.kind === 'recover')
  const tick = n <= 60 ? 5 : n <= 200 ? 10 : 50
  return (
    <div className="beltwrap">
      <svg className="belt" width={w} height={chartTop + chartH + 28} role="img" aria-label={`Belt diagram of ${l.name}`}>
        {/* control unit / feed block */}
        {l.scheme === 'A' ? (
          <g>
            {[0, 1, 2, 3].map(c => (
              <rect key={c} x={8 + c * 12} y={stripY} width={11} height={stripH} className={c < 2 ? 'cu-green' : 'cu-blue'} />
            ))}
            <text x={8} y={stripY - 6} className="blabel">{tr('control unit', '控制单元')}</text>
          </g>
        ) : (
          <text x={8} y={stripY + 12} className="blabel">{tr('feed', '投料')}</text>
        )}
        <rect x={left} y={stripY + stripH / 2 - 2} width={n * px} height={4} className="beltline" />
        {Array.from({ length: n }, (_, m) => {
          const pile = m >= n - endPile
          return <rect key={m} x={x(m) + 1} y={stripY} width={Math.max(2, px - 2)} height={stripH} className={`mach tier-${pile ? 'pile' : input}`}>
            <title>{`#${m + 1} · ${tierName(pile ? 'pile' : input)}`}</title>
          </rect>
        })}
        {feeds.map((p, j) => (
          <g key={j}>
            <path d={`M${x(p.pos)},${stripY - 14} L${x(p.pos)},${stripY - 2}`} className="feedarrow" style={{ stroke: SERIES[p.item! % 4] }} markerEnd="" />
            <path d={`M${x(p.pos) - 3},${stripY - 6} L${x(p.pos)},${stripY - 1} L${x(p.pos) + 3},${stripY - 6}`} className="feedhead" style={{ stroke: SERIES[p.item! % 4] }} />
            {(l.scheme === 'A' ? px >= 8 || j % 2 === 0 : j === 0) && <text x={x(p.pos) + 3} y={stripY - 16} className="flabel">+{fmt(l.scheme === 'A' ? p.amount : feeds.reduce((a, f) => a + f.amount, 0), 2)}</text>}
            <title>{tr(`after #${p.pos}: ${l.scheme === 'A' ? 'refill' : 'feed'} ${l.inputs[p.item!].name} ${fmt(p.amount, 3)} stacks/s`, `第 ${p.pos} 台后: ${l.scheme === 'A' ? '补' : '投'}${l.inputs[p.item!].name} ${fmt(p.amount, 3)} 叠/秒`)}</title>
          </g>
        ))}
        {recover.length > 0 && <text x={x(n) + 4} y={stripY + 12} className="blabel">{tr('recovery', '回收')}</text>}
        {/* flow chart */}
        <line x1={left} x2={x(n)} y1={y(cap)} y2={y(cap)} className="refline" />
        <text x={left - 4} y={y(cap) + 4} className="axlabel" textAnchor="end">C {fmt(cap)}</text>
        {l.scheme === 'A' && <>
          <line x1={left} x2={x(n)} y1={y(margin)} y2={y(margin)} className="refline margin" />
          <text x={x(n) + 4} y={y(margin) + 4} className="axlabel">m {fmt(margin)}</text>
        </>}
        <line x1={left} x2={x(n)} y1={y(0)} y2={y(0)} className="axis" />
        <text x={left - 4} y={y(0) + 4} className="axlabel" textAnchor="end">0</text>
        {Array.from({ length: Math.floor(n / tick) + 1 }, (_, i) => i * tick).map(t => (
          <text key={t} x={x(t)} y={chartTop + chartH + 16} className="axlabel" textAnchor="middle">{t}</text>
        ))}
        {low.map((s, j) => <line key={j} x1={s.x1} x2={s.x2} y1={y(margin)} y2={y(margin)} className="lowseg" />)}
        {paths.map((d, i) => <path key={i} d={d} className="flowline" style={{ stroke: SERIES[i % 4] }} />)}
      </svg>
    </div>
  )
}

/** Copies text; where the clipboard API is missing (plain http) it falls back to a selection copy. */
async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard) { await navigator.clipboard.writeText(text); return true }
  } catch { /* fall through */ }
  try {
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'; ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    const ok = document.execCommand('copy')
    ta.remove()
    return ok
  } catch { return false }
}

function Detail({ l, ls, cap, margin, setOverride }: {
  l: RecipeLayout; ls: BeltLayoutSettings; cap: number; margin: number
  setOverride: (key: string, o: TwoOverride | null) => void
}) {
  const t = l.two
  const g = l.ladder
  const ov = ls.overrides[l.key] ?? {}
  const seq = l.scheme === 'A' ? sequenceText(l) : ladderText(l)
  const set = (patch: TwoOverride) => setOverride(l.key, { ...ov, ...patch })
  const endPile = l.sorters.end_pile
  const [copied, setCopied] = useState('')
  // periods offered by hand: multiples of p + q that one refill can carry, plus the recommended and the chosen one
  const periods = t ? [...new Set([
    ...Array.from({ length: 60 }, (_, i) => (i + 1) * (t.p + t.q)).filter(p => p * Math.max(...t.c) <= cap + 1e-9),
    ...(t.recommended ? [t.recommended[0]] : []), ...(ov.period ? [ov.period] : []),
  ])].sort((a, b) => a - b) : []
  return (
    <div className="mbdetail">
      <ul className="chips small">
        {l.inputs.map((r, i) => (
          <li key={r.item}><span className="swatch" style={{ background: SERIES[i % 4] }} /><Icon id={r.item} size={18} />{r.name} <b>{fmt(r.items, 3)}</b>/s = <b>{fmt(r.stacks, 3)}</b> {tr('stacks/s per machine', '叠/秒(每台)')}</li>
        ))}
      </ul>
      <BeltDiagram l={l} cap={cap} margin={margin} endPile={endPile} />
      {l.trace_omitted > 0 && t && (
        <p className="hint">{tr(`Long line: the diagram omits ${l.trace_omitted} machines in the middle (${l.trace_omitted / t.period} whole periods, laid out like the periods before); add ${l.trace_omitted} to the machine numbers after the gap. The start section, remainder and return are the same as for the whole line.`, `长线:图中省略了中间 ${l.trace_omitted} 台(${l.trace_omitted / t.period} 个整周期,排法与前面的周期相同);省略处之后的机器序号要加上 ${l.trace_omitted}。起始段、零头和回流与整条线相同。`)}</p>
      )}
      <p className="hint">
        {tr('x axis: machine number', '横轴:机器序号')}{ls.rows >= 2 ? tr(' (one belt feeds two rows; refill positions only at even machines)', '(一条带供两排,补料位置只取偶数台)') : ''}{tr('; above: the input sorter of each machine (colour = yellow/green/blue/pile sorter), ', ';上方:每台机器的进料爪(颜色 = 黄/绿/蓝/白爪)、')}
        {l.scheme === 'A' ? tr('refill points (↓ refill amount, stacks/s)', '补料点(↓ 补进量,叠/秒)') : tr('feed points', '投料')}{tr('; below: the flow of each item on the belt (stacks/s), ', ';下方:带上各物品流量(叠/秒),')}{l.scheme === 'A' ? tr(`dashed line = margin m = ${fmt(margin)}, stretches below m are marked red, `, `虚线为余量 m = ${fmt(margin)},低于 m 的段标红,`) : ''}{tr('solid line = belt capacity C.', '实线为带容量 C。')}
      </p>
      <div className="seqbox">
        <div className="row small"><b>{tr('Sequence', '序列')}</b><button className="tabbtn" onClick={() => { void copyText(seq).then(ok => { setCopied(ok ? tr('Copied', '已复制') : tr('Copy failed: select the text', '复制失败:请手动选中文字')); setTimeout(() => setCopied(''), 2000) }) }}>{copied || tr('Copy', '复制')}</button></div>
        <p className="seq">{seq}</p>
      </div>
      {t && (
        <div className="cols mbcols">
          <div>
            <h4>{tr('Period', '周期')}</h4>
            <table className="small">
              <tbody>
                <tr><td>{tr('Consumption ratio a : b', '耗量比 a : b')}</td><td className="num">{t.p} : {t.q}{t.ratio_dev > 0 ? ` (≈, ${fmt(100 * t.ratio_dev, 1)} %)` : ''}</td></tr>
                <tr><td>{tr(`Period P · before refilling ${l.inputs[0].name} / before refilling ${l.inputs[1].name}`, `周期 P · 补 ${l.inputs[0].name} 前 / 补 ${l.inputs[1].name} 前`)}</td><td className="num">{t.period} · {t.n[0]} / {t.n[1]} {tr('machines', '台')}</td></tr>
                <tr><td>S(P) = C − P(a²+ab+b²)/(a+b)</td><td className="num">{f2(t.sum)}</td></tr>
                <tr><td>{tr('Steady-state minimum flow (A / B)', '最低流量(稳态)(A / B)')}</td><td className="num">{f2(t.best_mins[0])} / {f2(t.best_mins[1])}</td></tr>
                <tr><td>{tr('Minimum flow on this line (A / B)', '本线最低流量 (A / B)')}</td><td className="num">{f2(t.line_mins[0])} / {f2(t.line_mins[1])}</td></tr>
                <tr><td>{tr('Return (A / B) · shares', '回流 (A / B) · 份额')}</td><td className="num">{f2(t.returns[0])} / {f2(t.returns[1])} · {f2(t.shares[0])} / {f2(t.shares[1])}</td></tr>
                <tr><td>{tr('Control unit', '控制单元')}</td><td className="num">{unitName(t.unit)}</td></tr>
                <tr><td>{tr('Limiter lane', '限流格')} ({l.inputs[1 - t.jump].name})</td><td className="num">{t.limiter_lane}</td></tr>
                <tr><td>{tr('Station output belts (A / B)', '塔出货带 (A / B)')}</td><td className="num">{t.tower_belts[0]} / {t.tower_belts[1]}</td></tr>
                {t.complete_add > 0 && <tr><td>{tr('Machines short of a full period', '补满周期还差')}</td><td className="num">{t.complete_add}</td></tr>}
              </tbody>
            </table>
          </div>
          <div>
            <h4>{tr('Manual layout', '手动覆盖')}</h4>
            <div className="ovgrid small">
              <label>{tr('Period P', '周期 P')}
                <select value={ov.period ?? ''} onChange={e => set({ period: e.target.value ? +e.target.value : null })}>
                  <option value="">{tr('auto', '推荐')} ({t.recommended?.[0] ?? '–'})</option>
                  {periods.map(p => <option key={p} value={p}>{p}{p % (t.p + t.q) ? tr(' (not a multiple)', '(非倍数)') : ''}</option>)}
                </select></label>
              <label>{tr('Refill first', '先补')}
                <select value={ov.first ?? ''} onChange={e => set({ first: e.target.value === '' ? null : +e.target.value })}>
                  <option value="">{tr('auto', '自动')}</option>
                  {l.inputs.map((r, i) => <option key={i} value={i}>{r.name}</option>)}
                </select></label>
              <label>{tr('First refill position k', '第一次补料位置 k')}
                <input type="number" min={1} value={ov.k ?? ''} placeholder={String(t.k)} onChange={e => set({ k: e.target.value && Number.isFinite(+e.target.value) ? Math.min(1000000, Math.max(1, Math.round(+e.target.value))) : null })} /></label>
              <label>{tr('Shake σ', '摇匀 σ')} ({l.inputs[0].name})
                <select value={ov.sigma5 ?? ''} onChange={e => set({ sigma5: e.target.value ? +e.target.value : null })}>
                  <option value="">{tr('auto', '自动')}</option>
                  {[1, 2, 3, 4].map(s => <option key={s} value={s}>{s}/5</option>)}
                </select></label>
            </div>
            {(t.overridden || Object.keys(ov).length > 0) && (
              <p className="hint">
                {t.recommended
                  ? <>{tr('Recommended', '推荐')}: P = {t.recommended[0]}, σ = {t.recommended[1]}/5, {tr('refill first', '先补')} {l.inputs[t.recommended[2]].name}, k = {t.recommended[3]}, {tr('minimum', '最低')} {f2(t.recommended[4])};
                    {' '}{tr('current', '当前')}: {tr('minimum', '最低')} {f2(t.best)}{t.recommended[4] > t.best + 1e-9 ? ` (${f2(t.best - t.recommended[4])})` : ''}.{' '}</>
                  : <>{tr('No feasible recommendation', '没有可行的推荐')}; {tr('current', '当前')}: {tr('minimum', '最低')} {f2(t.best)}.{' '}</>}
                <a href="#" onClick={e => { e.preventDefault(); setOverride(l.key, null) }}>{tr('Reset', '重置')}</a></p>
            )}
            {!t.overridden && <p className="hint">{tr("auto: the largest feasible period (whole line ≥ m, return passes the check); among equally good layouts, the one whose two minima are closer and whose return is smaller (this order is set by this tool, not 笑笑's rule).", 'auto: 可行(全线 ≥ m,回流过检查)的最大周期;同样好的排法里取两个最低值更接近、回流更小的(这个次序是本工具定的,不是笑笑的规则)。')}</p>}
            <h4>{tr('Candidate periods', '候选周期')}</h4>
            <table className="small">
              <thead><tr><th className="num">P</th><th className="num">S</th><th className="num">{tr('best min', '最优最低')}</th><th>{tr('status', '状态')}</th></tr></thead>
              <tbody>
                {[...t.candidates].reverse().map(c => (
                  <tr key={c.period} className={c.period === t.period ? 'sel' : ''}>
                    <td className="num">{c.period}</td><td className="num">{f2(c.sum)}</td><td className="num">{c.best === null ? '–' : f2(c.best)}</td>
                    <td>{c.feasible ? tr('✓ feasible', '✓ 可行') : c.reason || (c.best === null ? tr('not evaluated (a larger P works)', '未评估(更大的 P 可行)') : '')}{c.unit ? ` · lcm ${c.unit}` : ''}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
      {g && (
        <div className="tablewrap"><table className="small">
          <thead><tr><th>{tr('Ingredient', '原料')}</th><th className="num">{tr('Need', '需求')}</th><th className="num">{tr('Feed', '投料')}</th><th>{tr('Combination', '组合')}</th><th className="num">{tr('Recovery sorters', '回收爪')}</th></tr></thead>
          <tbody>
            {g.feeds.map((f, i) => (
              <tr key={i}><td><Icon id={l.inputs[f.item].item} size={18} />{l.inputs[f.item].name}</td><td className="num">{f2(f.need)}</td><td className="num">{f2(f.rate)}</td><td>{f.combo}</td><td className="num">{g.recover_sorters[i]}</td></tr>
            ))}
            <tr><td>{tr('Total', '合计')} ≤ C {fmt(cap)}</td><td /><td className="num">{f2(g.total)}</td><td>{tr(`${g.group} machines per group × ${g.groups} groups`, `每组 ${g.group} 台 × ${g.groups} 组`)}{g.group > ls.ladder_short ? tr(`, surplus ρ = ${fmt(100 * g.rho, 1)} %`, `,余量 ρ = ${fmt(100 * g.rho, 1)} %`) : ''}</td><td /></tr>
          </tbody>
        </table></div>
      )}
      {l.stations && <>
        <h4>{tr('Stations and belts', '塔与传送带')}</h4>
        <p className="hint">
          {tr('One belt per port: at least one port per ingredient (control unit or refill) and at least one per product. ', '每个口一条带:每种原料至少一个口(控制单元或补料),每种产物至少一个口。')}{ls.station_ring ? tr("When a station's ports or throughput run out, open the next station; the big ring belt passes through all stations and refills come from the station they sit at (笑笑's way). ", '一个塔的口或运力用完就开下一个塔,大环带穿过各塔,补料由所在塔供(笑笑的做法)。') : tr('If one station is not enough, split the line into several modules, one control unit per station. ', '一个塔装不下就把线拆成多个模块,每塔一个控制单元。')}
          {tr(`${l.stations.stations.length} ${l.stations.kind === 'pls' ? 'planetary' : 'interstellar'} stations × ${l.stations.lines} ${ls.station_ring ? 'control units' : 'modules'} = ${l.stations.stations_total} stations. `, `${l.stations.stations.length} 个${l.stations.kind === 'pls' ? '行星' : '星际'}塔 × ${l.stations.lines} 个${ls.station_ring ? '控制单元' : '模块'} = ${l.stations.stations_total} 个塔。`)}
          {tr('Diagram: stations and port numbers on the left; the product belts above (dot above a machine = output sorter), the mixed belt in the middle (machine colour = input sorter), the refill belts from the ports below, joining the mixed belt at the arrows.', '图:左边是塔和口号;上方是产物带(机器上方的点 = 出料爪),中间是混带(机器颜色 = 进料爪),下方是从各口引出的补料带,箭头处并入混带。')}
        </p>
        <StationDiagram key={l.key} l={l} sp={l.stations} />
        <div className="seqbox"><div className="seq">{l.stations.sequence.map((q, i) => <div key={i}>{q}</div>)}</div></div>
        {l.stations.notes.length > 0 && <ul className="plain small notes">{l.stations.notes.map((n, i) => <li key={i} className="nl-info"><span className="tag">{tr('Note', '提示')}</span> {n}</li>)}</ul>}
      </>}
      <h4>{tr('Sorters', '分拣器')}</h4>
      <ul className="plain small">
        <li>{tr('Input', '进料')}: {tierName(l.sorters.input)} × {l.machines} {tr(`(${fmt(l.sorters.input_items, 2)} items/s per machine, must be strictly above)`, `(每台 ${fmt(l.sorters.input_items, 2)} 个/秒,须严格大于)`)}{endPile > 0 ? tr(`; the last ${endPile} machines before ${l.scheme === 'A' ? 'the control unit' : 'the recovery port'} use pile sorters`, `;${l.scheme === 'A' ? '回控制单元前' : '回收口前'}最后 ${endPile} 台用白爪`) : ''}</li>
        {l.sorters.outputs.map(o => (
          <li key={o.item}>{tr('Output', '出料')} {o.name}: {tierName(o.tier)} {tr(`(${fmt(o.rate, 3)} items/s per machine${Math.abs(o.sized_for - o.rate) > 1e-9 ? `, sorter chosen for ${fmt(o.sized_for, 3)} (proliferated output comes in batches)` : ''}); single-stack output belt full at ${o.full_at} machines`, `(每台 ${fmt(o.rate, 3)} 个/秒${Math.abs(o.sized_for - o.rate) > 1e-9 ? `,按 ${fmt(o.sized_for, 3)} 选爪(增产成批出)` : ''});单叠出货带 ${o.full_at} 台满带`)}</li>
        ))}
        <li>{tr('Sorter count', '爪数')}: {tr('mixed belt', '混带')} <b>{l.sorters.mixed}</b> · {tr('separate belts', '不混带')} {l.sorters.separate} · {tr('saved', '省')} {l.sorters.separate - l.sorters.mixed} · {tr('solver estimate', '求解器估算')} {fmt(l.sorters.lp_estimate, 0)}</li>
        {l.coaters && <li>{tr('Coaters', '喷涂机')}: {l.coaters.coaters} {tr('(on the refill branch)', '(装在补料支线上)')}, {fmt(l.coaters.sprays_per_s, 2)} {tr('sprays/s', '喷/秒')} ≈ {fmt(l.coaters.proliferator_per_s, 3)} {l.coaters.proliferator}{tr('/s', '/秒')}</li>}
      </ul>
      {l.notes.length > 0 && <>
        <h4>{tr('Checks', '检查')}</h4>
        <ul className="plain small notes">
          {l.notes.map((n, i) => (
            <li key={i} className={`nl-${n.level}`}><span className="tag">{n.level === 'fail' ? tr('Fail', '不通过') : n.level === 'warn' ? tr('Warning', '注意') : tr('Note', '提示')}</span> {n.text}{n.cite && <span className="muted"> [{n.cite}]</span>}</li>
          ))}
        </ul>
      </>}
    </div>
  )
}

const numOr = (v: string, d: number | null) => (v.trim() === '' ? d : Number.isFinite(+v) ? +v : d)

export default function MixedBelt({ plan, planSettings, current, settings, setSettings }: Props) {
  const ls = settings.belt_layout ?? defaultBeltLayout()
  const solved = !!plan && plan.status === 'optimal'
  // the plan is laid out with the settings it was solved for (its machines and modes), and the tab's own settings
  const rep = useMemo(
    () => (solved ? mixedBelt({ ...(planSettings ?? settings), belt_layout: ls }, plan) : null),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [solved, plan, planSettings, planSettings ? null : settings, ls],
  )
  const [open, setOpen] = useState<Record<string, boolean>>({})
  // every write goes through cleanBeltLayout: whole numbers in range, so a typed 7.5 or −1 never reaches the solver
  const setL = (patch: Partial<BeltLayoutSettings>) => setSettings(s => ({ ...s, belt_layout: cleanBeltLayout({ ...(s.belt_layout ?? defaultBeltLayout()), ...patch }) }))
  const setOverride = (key: string, o: TwoOverride | null) => setSettings(s => {
    const bl = s.belt_layout ?? defaultBeltLayout()
    const overrides = { ...bl.overrides }
    const clean = o ? Object.fromEntries(Object.entries(o).filter(([, v]) => v !== null && v !== undefined)) : {}
    if (Object.keys(clean).length) overrides[key] = clean
    else delete overrides[key]
    return { ...s, belt_layout: cleanBeltLayout({ ...bl, overrides }) }
  })
  const isOpen = (k: string) => open[k] ?? settings.mixed_belts
  const cap = rep?.capacity ?? 30
  return (
    <div className="results mixedbelt">
      <p className="hint">
        <b>{tr('Mixed belts', '混带')}</b>{tr(': one mixed input belt per line, laid out per recipe after 笑笑meow (ep19/20). Two ingredients: control unit + direct refills (scheme A, closed form S = C − P(a²+ab+b²)/(a+b), period, σ and first refill searched); three or more: fixed-rate ladder groups (scheme B). Flows in stacks/s. Statistical average-flow model: the margin m, the ladder surplus ρ and short-group length, the repeat-unit cap 4R, the restart hint and the order among equally good layouts are ', ':每条线一条混合进料带,按配方排布,依据笑笑meow(ep19/20)。两种原料:控制单元 + 直怼补料(方案 A,闭式 S = C − P(a²+ab+b²)/(a+b),搜索周期、σ 和先补哪种);三种及以上:固定速率梯口机组(方案 B)。流量单位叠/秒。统计平均流量模型:余量 m、梯口余量 ρ 与短组长度、重复单元上限 4R、重启提示以及同样好的排法之间的取舍次序均为')}
        <b>{tr(' fitted to his lines or chosen here, not measured', '按他的线拟合或在此选定,并非实测')}</b>{tr(' — test outage of each input and a full output in a sandbox and confirm a full-speed restart before building [BV1fXkxYDEbW 80:40].', ' —— 建造前请在沙盒里测试任一进料断货和产物堆满,并确认能重启到满速 [BV1fXkxYDEbW 80:40]。')}
      </p>
      {!settings.mixed_belts && <p className="note small">{tr('The solver did not cost sorters as mixed belts (Sidebar → mixed input belts); this tab works either way.', '求解器没有按混带计算分拣器成本(侧栏 → 混合进料带);本页两种情况都可用。')}</p>}
      <div className="mbsettings small">
        <label>{tr('Belt', '带')}<select value={ls.belt} onChange={e => setL({ belt: +e.target.value })}>{beltNames().map((n, i) => <option key={i} value={i}>{n}</option>)}</select></label>
        <label>{tr('Stack', '叠层')}<select value={ls.stack} onChange={e => setL({ stack: +e.target.value })}>{[1, 2, 3, 4].map(n => <option key={n} value={n}>{n}</option>)}</select></label>
        <label>{tr('Rows per belt', '每带排数')}<select value={ls.rows} onChange={e => setL({ rows: +e.target.value })}><option value={1}>1</option><option value={2}>2</option></select></label>
        <label>{tr('Machines per row R', '每排台数 R')}<input type="number" min={1} placeholder="–" value={ls.row_len ?? ''} onChange={e => setL({ row_len: numOr(e.target.value, null) })} /></label>
        <label>{tr('Margin m', '余量 m')}<input type="number" min={0} step={0.25} value={ls.margin} onChange={e => setL({ margin: numOr(e.target.value, 1.5) ?? 1.5 })} /></label>
        <label>{tr('Station', '塔')}<select value={ls.station} onChange={e => setL({ station: e.target.value as 'ils' | 'pls' })}><option value="ils">{tr('Interstellar ILS', '星际塔 ILS')}</option><option value="pls">{tr('Planetary PLS', '行星塔 PLS')}</option></select></label>
        <label>{tr('Multi-station layout', '多塔')}<select value={ls.station_ring ? 'ring' : 'module'} onChange={e => setL({ station_ring: e.target.value === 'ring' })}><option value="module">{tr('one module per station', '每塔一个模块')}</option><option value="ring">{tr('big ring belt through all stations', '大环带穿过多塔')}</option></select></label>
        <label>{tr('Ports used per station', '每塔用口')}<input type="number" min={1} max={12} value={ls.station_slots} onChange={e => setL({ station_slots: Math.min(12, Math.max(1, Math.round(numOr(e.target.value, 12) ?? 12))) })} /></label>
        <label>{tr('Best sorter', '最高爪')}<select value={ls.sorter_max} onChange={e => setL({ sorter_max: +e.target.value })}>{sorterNames().map((n, i) => <option key={i} value={i}>{n}</option>)}</select></label>
        <details>
          <summary>{tr('More', '更多')}</summary>
          <div className="mbsettings">
            <label>{tr('Pile sorter', '白爪')}<select value={ls.pile_level} onChange={e => setL({ pile_level: +e.target.value })}>{pileNames().map((n, i) => <option key={i} value={i}>{n}</option>)}</select></label>
            <label>{tr('Unit limit lcm(P, rows·R) ≤', '单元上限 lcm(P, 排数·R) ≤')}<input type="number" min={1} placeholder={ls.row_len ? String(4 * ls.row_len) : '4R'} value={ls.unit_max ?? ''} onChange={e => setL({ unit_max: numOr(e.target.value, null) })} /></label>
            <label>{tr('Machine cap per control unit', '单控制单元台数上限')}<input type="number" min={1} placeholder={tr('none', '不设')} value={ls.unit_cap ?? ''} onChange={e => setL({ unit_cap: numOr(e.target.value, null) })} /></label>
            <label>{tr('Ladder surplus ρ %', '梯口余量 ρ %')}<input type="number" min={0} step={0.1} value={+(100 * ls.ladder_rho).toFixed(3)} onChange={e => setL({ ladder_rho: (numOr(e.target.value, 8.333) ?? 8.333) / 100 })} /></label>
            <label>{tr('Short group ≤', '短组 ≤')}<input type="number" min={1} value={ls.ladder_short} onChange={e => setL({ ladder_short: numOr(e.target.value, 12) ?? 12 })} /></label>
            <label>{tr('Ladder group length cap', '梯口组长上限')}<input type="number" min={1} value={ls.ladder_cap} onChange={e => setL({ ladder_cap: numOr(e.target.value, 24) ?? 24 })} /></label>
            <label>{tr('Restart hint: below items/s', '重启提示 < 个/秒')}<input type="number" min={0} value={ls.hint_items} onChange={e => setL({ hint_items: numOr(e.target.value, 12) ?? 12 })} /></label>
            <label>{tr('and above machines', '且 > 台')}<input type="number" min={1} value={ls.hint_machines} onChange={e => setL({ hint_machines: numOr(e.target.value, 60) ?? 60 })} /></label>
            <button onClick={() => setL({ ...defaultBeltLayout(), overrides: ls.overrides })}>{tr('Defaults', '默认')}</button>
          </div>
        </details>
      </div>
      {!rep && <p className="muted">{tr('Solve a plan first.', '请先求解一个方案。')}</p>}
      {rep && !current && <p className="note small" role="status"><b>{tr('Earlier plan.', '较早的方案。')}</b> {tr('The settings changed and no plan for them is in hand yet (still solving, stopped, or no solution): the layouts below are for the plan solved before.', '设置已更改,但还没有对应的方案(仍在求解、已停止或无解):下面的排布对应之前求解的方案。')}</p>}
      {rep?.error && <p className="warn">{rep.error}</p>}
      {rep?.warnings.map((w, i) => <p key={i} className="warn">{w}</p>)}
      {rep && !rep.error && rep.recipes.length === 0 && <p className="muted">{tr('No recipe in this plan has two or more ingredients.', '此方案中没有配方有两种及以上原料。')}</p>}
      {rep && rep.recipes.length > 0 && (
        <div className="tablewrap"><table className="mbtable">
          <thead><tr>
            <th>{tr('Recipe', '配方')}</th><th className="num">{tr('Machines', '台数')}</th><th>{tr('Scheme', '方案')}</th><th className="num">{tr('Period / group', '周期 / 组')}</th><th className="num">{tr('Min', '最低')}</th>
            <th>{tr('Control unit', '控制单元')}</th><th className="num">{tr('Sorters mixed / separate', '爪 混带 / 不混带')}</th><th>{tr('Status', '状态')}</th>
          </tr></thead>
          <tbody>
            {rep.recipes.map(l => (
              <tr key={l.key} className="clickable" aria-expanded={isOpen(l.key)} onClick={() => setOpen(o => ({ ...o, [l.key]: !isOpen(l.key) }))}>
                <td><span className="caret">{isOpen(l.key) ? '▾' : '▸'}</span><Icon id={l.outputs[0]?.item} />{l.name}{l.mode_label && <span className="muted small"> · {l.mode_label}</span>}</td>
                <td className="num">{l.machines}</td>
                <td>{schemeName(l.scheme)}</td>
                <td className="num">{l.two ? `P ${l.two.period}` : l.ladder ? `${l.ladder.group} × ${l.ladder.groups}` : '–'}</td>
                <td className="num">{l.two ? f2(Math.min(l.two.line_mins[0], l.two.line_mins[1])) : l.ladder ? `${f2(l.ladder.total)} / ${fmt(cap)}` : '–'}</td>
                <td>{l.two ? unitName(l.two.unit) : '–'}</td>
                <td className="num">{l.sorters.mixed} / {l.sorters.separate}</td>
                <td className={`st-${l.status}`}>{l.status === 'ok' ? tr('Pass', '通过') : l.status === 'warn' ? tr('Warning', '注意') : tr('Fail', '不通过')}</td>
              </tr>
            ))}
          </tbody>
        </table></div>
      )}
      {rep?.recipes.filter(l => isOpen(l.key)).map(l => (
        <section key={l.key} className="mbrecipe">
          <h3><Icon id={l.outputs[0]?.item} />{l.name}{l.mode_label && <span className="muted small"> · {l.mode_label}</span>} <span className="tag">{schemeName(l.scheme)}</span></h3>
          <Detail l={l} ls={ls} cap={cap} margin={rep.margin} setOverride={setOverride} />
        </section>
      ))}
      {rep && rep.single.length > 0 && <p className="hint">{tr('Single-ingredient recipes are not mixed, one item per station', '单原料配方不混带,一物一塔')} [BV1fXkxYDEbW 03:44]: {rep.single.join(tr(', ', '、'))}</p>}
      <p className="hint">
        {tr('Statistical flow model (average flow; it cannot see logic-frame collisions or the unevenness of slow recipes). Before going live, test in a sandbox: any input running out, and the output backing up, and confirm each restarts to full speed (笑笑 [BV1fXkxYDEbW 80:40]). The fitted parameters (margin m, ladder ρ / short group 12 / group length 24, unit cap 4R, restart hint 12 items/s · 60 machines, 2 / 3 pile sorters at the end) must be calibrated in game.', '统计流量模型(平均流量,看不到逻辑帧碰撞和慢配方的不均匀)。上线前在沙盒测三种情况:任一进料断货、产物堆满,并确认都能重启到满速(笑笑 [BV1fXkxYDEbW 80:40])。拟合参数(余量 m、梯口 ρ / 短组 12 / 组长 24、单元上限 4R、重启提示 12 个/秒 · 60 台、末端白爪 2 / 3 台)需在游戏中校准。')}
      </p>
    </div>
  )
}
