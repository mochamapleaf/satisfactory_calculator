// DSP mixed belt: station module drawing (src/planner/mixed_station.rs). Schematic: the station on
// the left with one lane per slot; the mixed belt runs right past the machines; refill lanes come
// from their slot and join the belt after their machine; product lanes take each machine's output
// back to their slot. Machine colour = input sorter tier, dot above = output sorter tier.
import { useState } from 'react'
import { fmt, type RecipeLayout, type StationPlan } from './solver'
import { t } from './i18n'

const tierName = (k: string): string => {
  switch (k) {
    case 'yellow': return t('yellow sorter', '黄爪')
    case 'green': return t('green sorter', '绿爪')
    case 'blue': return t('blue sorter', '蓝爪')
    case 'pile': return t('pile sorter', '白爪')
    default: return k
  }
}
const roleName = (k: string): string => {
  switch (k) {
    case 'control unit': return t('control unit', '控制单元')
    case 'refill': return t('refill', '补料')
    case 'feed': return t('feed', '投料')
    case 'recovery': return t('recovery', '回收')
    case 'product': return t('product', '产物')
    case 'proliferator': return t('proliferator', '增产剂')
    default: return k
  }
}
const SERIES = ['var(--accent)', 'var(--raw)', 'var(--sup)', 'var(--liquid)']

export function StationDiagram({ l, sp }: { l: RecipeLayout; sp: StationPlan }) {
  const [pick, setSi] = useState(0)
  const si = Math.min(pick, sp.stations.length - 1)
  const s = sp.stations[si]
  if (!s) return null
  const color = (item: string) => {
    const i = l.inputs.findIndex(x => x.item === item)
    return i >= 0 ? SERIES[i % 4] : 'var(--muted)'
  }
  const two = sp.machines.some(m => m.side === 'R')
  const machines = sp.machines.filter(m => m.station === s.index)
  const positions = Math.max(1, ...machines.map(m => m.pos))
  const px = 26, mw = 20, mh = 18
  const left = 150
  const outs = s.ports.filter(p => p.dir === 'out')
  const ins = s.ports.filter(p => p.dir === 'in')
  const lane = 15
  const top = 22
  const yOut = (k: number) => top + k * lane
  const yMachL = top + outs.length * lane + 16
  const yBelt = yMachL + mh + 10
  const yMachR = yBelt + 10
  const yInStart = (two ? yMachR + mh : yBelt) + 26
  const yIn = (k: number) => yInStart + k * lane
  const h = yIn(ins.length) + 10
  const x = (pos: number) => left + 30 + (pos - 1) * px
  const w = x(positions) + px + 70
  const slotY = new Map<number, number>()
  outs.forEach((p, k) => slotY.set(p.slot, yOut(k)))
  ins.forEach((p, k) => slotY.set(p.slot, yIn(k)))
  const byN = new Map(machines.map(m => [m.n, m]))
  const mx = (n: number) => { const m = byN.get(n); return m ? x(m.pos) + mw / 2 : left + 30 }
  const isA = l.scheme === 'A'
  return (
    <div className="stationbox">
      {sp.stations.length > 1 && (
        <div className="row small stationtabs" role="tablist">
          {sp.stations.map((st, i) => (
            <button key={st.index} role="tab" aria-selected={i === si} className={`tabbtn${i === si ? ' on' : ''}`} onClick={() => setSi(i)}>
              {t('Station', '塔')} {st.index} · #{st.first}–{st.last}
            </button>
          ))}
        </div>
      )}
      <div className="beltwrap">
        <svg className="belt station" width={w} height={h} role="img" aria-label={`Station ${s.index} of ${l.name}`}>
          {/* station */}
          <rect x={8} y={top - 12} width={left - 76} height={h - top} rx={6} className="stationrect" />
          <text x={14} y={top + 2} className="blabel">{sp.kind === 'pls' ? t('Planetary station', '行星塔') : t('Interstellar station', '星际塔')} {s.index}</text>
          <text x={14} y={top + 16} className="axlabel">{s.ports.length}/{sp.slots} {t('ports', '口')}</text>
          <text x={14} y={top + 30} className="axlabel">{t('supply', '供')} {fmt(s.items, 0)}/s</text>
          {/* product lanes: from the served machines back to the slot */}
          {outs.map(p => {
            const y = slotY.get(p.slot)!
            const xr = Math.max(...p.serves.map(mx))
            return (
              <g key={p.slot}>
                <line x1={left - 52} x2={xr} y1={y} y2={y} className="lane out" />
                <path d={`M${left - 44},${y - 4} L${left - 50},${y} L${left - 44},${y + 4}`} className="lanehead" />
                <text x={left - 50} y={y - 3} className="slotlabel" textAnchor="end">{p.slot}</text>
                <text x={xr + 6} y={y + 4} className="axlabel">{p.name} {fmt(p.items, 1)}/s</text>
                <title>{`${t('Port', '口')} ${p.slot} ${t('out', '出')} ${p.name} ${fmt(p.items, 2)}/s ← #${p.serves[0]}–#${p.serves[p.serves.length - 1]}`}</title>
              </g>
            )
          })}
          {/* mixed belt */}
          <line x1={left - 52} x2={x(positions) + px} y1={yBelt} y2={yBelt} className="lane mixed" />
          <text x={x(positions) + px + 4} y={yBelt + 4} className="axlabel">{t('mixed belt', '混带')} →</text>
          {isA && s.first === 1 && <g>
            {[0, 1, 2, 3].map(c => <rect key={c} x={left - 44 + c * 9} y={yBelt - 5} width={8} height={10} className={c < 2 ? 'cu-green' : 'cu-blue'} />)}
            <text x={left - 46} y={yBelt - 9} className="axlabel">{t('control unit', '控制单元')}</text>
            <title>{`${t('Control unit: first two lanes green belt (limiter), last two blue belt', '控制单元:前两格绿带(限流),后两格蓝带')}; ${l.two ? `${l.inputs[0].name} ${fmt(l.two.shares[0], 2)} / ${l.inputs[1].name} ${fmt(l.two.shares[1], 2)} ${t('stacks/s', '叠/秒')}` : ''}`}</title>
          </g>}
          {/* machines */}
          {machines.map(m => {
            const y = m.side === 'R' ? yMachR : yMachL
            return (
              <g key={m.n}>
                <rect x={x(m.pos)} y={y} width={mw} height={mh} rx={2} className={`mach tier-${m.input}`} />
                <text x={x(m.pos) + mw / 2} y={y + 12} className="mnum" textAnchor="middle">{m.n}</text>
                {m.outputs.map((o, k) => {
                  const ly = slotY.get(o[0]) ?? top
                  const cx = x(m.pos) + 4 + k * 6
                  return <g key={k}>
                    <line x1={cx} x2={cx} y1={m.side === 'L' ? y : y + mh} y2={m.side === 'L' ? ly : y + mh + 8} className="tick" />
                    <circle cx={cx} cy={m.side === 'L' ? y - 3 : y + mh + 3} r={2.6} className={`dot tier-${o[1]}`} />
                  </g>
                })}
                <title>{`#${m.n} · ${t('in', '进料')} ${tierName(m.input)} (${t('mixed belt', '混带')}) · ${t('out', '出料')} ${m.outputs.map(o => `${tierName(o[1])} → ${t('port', '口')} ${o[0]}`).join(', ')}`}</title>
              </g>
            )
          })}
          {/* input lanes: control unit at the belt start, refills after their machine */}
          {ins.map(p => {
            const y = slotY.get(p.slot)!
            const xs = p.role === 'control unit' || p.role === 'feed' ? [left - 30] : p.role === 'proliferator' ? [] : p.serves.map(n => mx(n) + px / 2)
            const xr = xs.length ? Math.max(...xs) : left
            return (
              <g key={p.slot}>
                <line x1={left - 52} x2={xr} y1={y} y2={y} className="lane in" style={{ stroke: color(p.item) }} />
                <text x={left - 50} y={y - 3} className="slotlabel" textAnchor="end">{p.slot}</text>
                {xs.map((xx, k) => (
                  <g key={k}>
                    <line x1={xx} x2={xx} y1={y} y2={yBelt + 3} className="lane in up" style={{ stroke: color(p.item) }} />
                    <path d={`M${xx - 3},${yBelt + 9} L${xx},${yBelt + 3} L${xx + 3},${yBelt + 9}`} className="lanehead" style={{ stroke: color(p.item) }} />
                  </g>
                ))}
                <text x={xr + 6} y={y + 4} className="axlabel">{roleName(p.role)} {p.name} {fmt(p.items, 1)}/s</text>
                <title>{`${t('Port', '口')} ${p.slot} ${t('supplies', '供')} ${p.name} ${fmt(p.items, 2)}/s (${roleName(p.role)}${p.role === 'refill' ? `, ${t('after machine', '第')} ${p.serves.join(t(', ', '、'))}${t('', ' 台后')}` : ''})`}</title>
              </g>
            )
          })}
        </svg>
      </div>
      <div className="tablewrap">
        <table className="mbtable small">
          <thead><tr><th className="num">{t('Port', '口')}</th><th>{t('Direction', '方向')}</th><th>{t('Use', '用途')}</th><th>{t('Item', '物品')}</th><th className="num">{t('items/s', '个/秒')}</th><th>{t('Position', '位置')}</th></tr></thead>
          <tbody>
            {s.ports.map(p => (
              <tr key={p.slot}>
                <td className="num">{p.slot}</td><td>{p.dir === 'in' ? t('station → belt', '塔 → 带') : t('belt → station', '带 → 塔')}</td><td>{roleName(p.role)}</td>
                <td><span className="swatch" style={{ background: p.dir === 'in' ? color(p.item) : 'var(--muted)' }} />{p.name}</td>
                <td className="num">{fmt(p.items, 2)}</td>
                <td className="small">{p.role === 'refill' ? t(`joins the mixed belt after machine ${p.serves.join(', ')}`, `第 ${p.serves.join('、')} 台后并入混带`) : p.role === 'product' ? t(`takes the product of #${p.serves[0]}–#${p.serves[p.serves.length - 1]}`, `收 #${p.serves[0]}–#${p.serves[p.serves.length - 1]} 的产物`) : p.role === 'control unit' ? t('control unit (belt start)', '控制单元(带起点)') : p.role === 'feed' ? t(`fed before machine ${p.serves[0]}`, `第 ${p.serves[0]} 台前投料`) : p.role === 'recovery' ? t(`recovered after machine ${p.serves[0]}`, `第 ${p.serves[0]} 台后回收`) : t('coater (on the refill branch)', '喷涂机(补料支线上)')}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <StationTable sp={sp} station={s.index} />
    </div>
  )
}

/** Per-machine table: sorter and the belt it takes from / gives to. */
function StationTable({ sp, station }: { sp: StationPlan; station: number }) {
  const [all, setAll] = useState(false)
  const here = sp.machines.filter(m => m.station === station)
  const rows = all ? here : here.slice(0, 40)
  return (
    <div className="tablewrap">
      <table className="mbtable small">
        <thead><tr><th className="num">#</th><th>{t('Station', '塔')}</th><th>{t('Position', '位置')}</th><th>{t('Input sorter (mixed belt)', '进料爪 (混带)')}</th><th>{t('Output sorter → port', '出料爪 → 口')}</th></tr></thead>
        <tbody>
          {rows.map(m => (
            <tr key={m.n}>
              <td className="num">{m.n}</td><td>{m.station}</td><td>{m.pos}{m.side === 'R' ? t(' right', ' 右') : sp.machines.some(q => q.side === 'R') ? t(' left', ' 左') : ''}</td>
              <td><span className={`tierchip tier-${m.input}`} />{tierName(m.input)}</td>
              <td>{m.outputs.map((o, k) => <span key={k}><span className={`tierchip tier-${o[1]}`} />{tierName(o[1])} → {t('port', '口')} {o[0]} </span>)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {here.length > 40 && <button className="tabbtn" onClick={() => setAll(!all)}>{all ? t('First 40 machines only', '只看前 40 台') : t(`All ${here.length} machines`, `全部 ${here.length} 台`)}</button>}
    </div>
  )
}
