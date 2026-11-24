import { Icon, RemoveMark } from './Icon'
import { t, trWarn } from './i18n'
import { carrier, dn, fmt, isLiquid, itemName, qsplit, qualityNames, type GameData, type MachineGroup, type Plan, type PlanRecipe, type RateLine, type Settings } from './solver'

/** CPU time per tick: µs, or ms from 1000 µs. */
const fmtUs = (us: number) => (us >= 1000 ? `${fmt(us / 1000, 1)} ms` : `${fmt(us, us < 10 ? 1 : 0)} µs`)

const groupText = (g: MachineGroup) =>
  `${fmt(g.count)} × ${fmt(g.clock_percent, 2)}%${g.shards ? ` (${g.shards} ${t(g.shards > 1 ? 'shards' : 'shard', '能量碎片')})` : ''}`

interface Props {
  data: GameData
  plan: Plan
  settings: Settings
  setSettings: (f: (s: Settings) => Settings) => void
}

export default function Results({ data, plan, settings: s, setSettings }: Props) {
  if (plan.status === 'empty') return <p className="muted">{t('Add a target item to start a plan.', '添加目标物品以开始规划。')}</p>
  if (plan.status !== 'optimal') {
    return (
      <div className="warn">
        <b>{plan.status === 'infeasible' ? t('No feasible plan', '无可行规划') : plan.status === 'unbounded' ? t('Unbounded', '无上限') : plan.status === 'cancelled' ? t('Cancelled', '已取消') : plan.status === 'error' ? t('Error', '出错') : plan.status}</b>
        {plan.warnings.map(w => <p key={w}>{trWarn(w)}</p>)}
      </div>
    )
  }
  const liquid = (id: string) => isLiquid(data, id)
  const recipeById = new Map(data.recipes.map(r => [r.id, r]))
  const bld = (name: string) => dn(data.buildings.find(b => b.name === name || b.id === name), data.game, name)
  const rn = (r: PlanRecipe) => {
    const [base, q] = qsplit(r.id)
    const x = recipeById.get(r.id) ?? recipeById.get(base)
    return x ? dn(x, data.game, r.name) + (q ? ` ★${tierName(q)}` : '') : r.name
  }
  const bid = (name: string) => data.buildings.find(b => dn(b, data.game) === name || b.name === name)?.id
  const slots = (rid: string) => {
    const b = recipeById.get(rid)?.building
    return data.buildings.find(x => x.id === b)?.somersloop_slots ?? 0
  }
  const setSloops = (rid: string, n: number) =>
    setSettings(x => {
      const somersloops = { ...x.somersloops }
      // n < 0: let the solver choose (auto mode); 0 is a fixed "none" only in auto mode
      if (n > 0 || (n === 0 && x.auto_somersloops)) somersloops[rid] = n
      else delete somersloops[rid]
      return { ...x, somersloops }
    })
  // per-recipe highest clock (memo step 8); '' = the plan's max clock
  const setClock = (rid: string, v: string) =>
    setSettings(x => {
      const recipe_clocks = { ...x.recipe_clocks }
      if (v === '') delete recipe_clocks[rid]
      else recipe_clocks[rid] = +v
      return { ...x, recipe_clocks }
    })
  const CLOCKS = [25, 50, 75, 100, 150, 200, 250]
  // quality plans: the normal-tier row excludes only that tier ("id@0"); the bare id would drop every tier
  const exclude = (rid: string) => {
    const key = plan.quality && qsplit(rid)[1] === 0 ? `${rid}@0` : rid
    setSettings(x => ({ ...x, disabled_recipes: [...x.disabled_recipes, key] }))
  }
  const totalSloops = plan.somersloops
  const whole = s.integer
  // quality plans: one row per shared group (base recipe × machine × loadout); the tiers it runs
  // switch recipes on the group's machines, listed as a tier split
  const rowGroups: PlanRecipe[][] = []
  {
    const at = new Map<string, PlanRecipe[]>()
    for (const r of plan.recipes) {
      const k = plan.quality ? `${qsplit(r.id)[0]}#${r.mode}` : `${r.id}#${r.mode}`
      const g = at.get(k)
      if (g) g.push(r)
      else { const n = [r]; at.set(k, n); rowGroups.push(n) }
    }
    for (const g of rowGroups) g.sort((a, b) => qsplit(a.id)[1] - qsplit(b.id)[1])
  }
  const tierName = (q: number) => (qualityNames(data)[q] ?? ['normal', 'uncommon', 'rare', 'epic', 'legendary'][q] ?? '?').toLowerCase()
  const sumLines = (ls: RateLine[]) => {
    const m = new Map<string, RateLine>()
    for (const l of ls) { const o = m.get(l.item); if (o) o.rate += l.rate; else m.set(l.item, { ...l }) }
    return [...m.values()]
  }

  // group buildings by type
  // (quality plans: tiers of a recipe share machines, so the solver rounds up per base recipe and loadout)
  const byBuilding = new Map<string, number>()
  if (plan.building_counts?.length) for (const b of plan.building_counts) byBuilding.set(b.building, b.count)
  else for (const r of plan.recipes) byBuilding.set(r.building, (byBuilding.get(r.building) ?? 0) + (whole ? r.machines : r.machines_ceil))

  const limit = (item: string) => {
    if (s.ignore_limits) return null
    if (item in s.resource_limits) return s.resource_limits[item]
    return data.resources.find(r => r.item === item)?.limit ?? null
  }

  const sharedRow = (g: PlanRecipe[]) => {
              const r = g[0]
              const share = g.reduce((a, x) => a + x.machines, 0)
              const n = r.group_machines ?? Math.ceil(share - 1e-6)
              const base = rn(r).replace(/ ★\S+$/, '')
              return (
                <tr key={qsplit(r.id)[0] + '#' + r.mode} className="shared">
                  <td><Icon id={qsplit(r.outputs[0]?.item ?? '')[0]} />{base}{r.alt && <span className="tag">{t('alt', '替代')}</span>}<span className="tag" title={t('These quality tiers run on the same machines, which switch recipes', '这些品质档共用同一批机器，机器会切换配方')}>{g.length} {t('tiers', '档')}</span>{r.mode_label && <div className="muted small">{r.mode_label}</div>}</td>
                  <td><Icon id={bid(r.building)} />{bld(r.building)}</td>
                  <td className="num" title={t(`shared by ${g.length} tiers: ${fmt(share, 4)} machines of work`, `${g.length} 档共用：工作量 ${fmt(share, 4)} 台机器`)}>
                    <b>{fmt(n, whole ? 0 : 2)}</b>{!whole && <span className="muted"> ({fmt(share)})</span>}
                    {g.map(x => <div key={x.id} className="muted small">{tierName(qsplit(x.id)[1])} {fmt(x.machines)}</div>)}
                  </td>
                  <td className="small">{n > 0 ? t(`${fmt((100 * share) / n, 1)}% busy`, `${fmt((100 * share) / n, 1)}% 负载`) : ''}</td>
                  <td><span className="muted">—</span></td>
                  <td className="num">{fmt(g.reduce((a, x) => a + x.power, 0), 1)}</td>
                  {plan.compute && <td className="num">{fmt((100 * g.reduce((a, x) => a + x.compute, 0)) / (plan.compute.total || 1), 1)}%</td>}
                  <td className="small">{sumLines(g.flatMap(x => x.inputs)).map(l => <div key={l.item}><Icon id={l.item} size={16} />{fmt(l.rate)} {itemName(data, l.item)}</div>)}</td>
                  <td className="small">{sumLines(g.flatMap(x => x.outputs)).map(l => <div key={l.item}><Icon id={l.item} size={16} />{fmt(l.rate)} {itemName(data, l.item)}</div>)}</td>
                  <td><button className="x" title={t('Exclude these tiers of the recipe and re-solve', '排除该配方的这些品质档并重新求解')} aria-label={t(`Exclude ${base}`, `排除 ${base}`)} onClick={() => setSettings(x => ({ ...x, disabled_recipes: [...x.disabled_recipes, ...g.map(y => qsplit(y.id)[1] === 0 ? `${y.id}@0` : y.id)] }))}><RemoveMark /></button></td>
                </tr>
              )
  }

  const generated = plan.recipes.reduce((a, r) => a + r.outputs.filter(o => o.item === 'Electricity').reduce((x, o) => x + o.rate, 0), 0)
  return (
    <div className="results">
      <div className="cards">
        <div className="card"><div className="big">{fmt(plan.buildings, 0)}</div><div className="muted">{t('production buildings', '生产建筑')}</div></div>
        <div className="card"><div className="big">{fmt(plan.production_power, 1)}<span className="unit">MW</span></div><div className="muted">{t('production power', '生产电力')}</div></div>
        <div className="card"><div className="big">{fmt(plan.extraction_power, 1)}<span className="unit">MW</span></div><div className="muted">{t('extraction, est.', '开采（估算）')}</div></div>
        <div className="card"><div className="big">{fmt(plan.production_power + plan.extraction_power, 1)}<span className="unit">MW</span></div><div className="muted">{t('total power', '总电力')}</div></div>
        {generated > 0 && <div className="card" title={t('Generators in this plan (fuel → Electricity). Self-powered plans cover their own production + extraction draw.', '本规划中的发电机（燃料 → 电力）。自供电规划覆盖自身的生产与开采用电。')}><div className="big">{fmt(generated, 1)}<span className="unit">MW</span></div><div className="muted">{t('generated', '发电')}</div></div>}
        {plan.shards > 0 && <div className="card"><div className="big">{fmt(plan.shards, 0)}</div><div className="muted">{t('power shards', '能量碎片')}</div></div>}
        {totalSloops > 0 && <div className="card"><div className="big">{fmt(totalSloops, 0)}</div><div className="muted">{t('Somersloops', '索莫晶体')}</div></div>}
        {plan.compute && (
          <div className="card" title={t(`Whole buildings, working or idle; mining machines, orbital collectors and ray receivers are not counted. machines ${fmt(plan.compute.machines, 1)} · sorters ${fmt(plan.compute.sorters, 1)} · belts + stations ${fmt(plan.compute.logistics, 1)} · coaters ${fmt(plan.compute.coaters, 1)} · generation ${fmt(plan.compute.generation, 1)} (${fmt(plan.compute.cost_per_mw, 4)}/MW)`, `整数建筑，无论工作或空闲；不计采矿机、轨道采集器和射线接收站。机器 ${fmt(plan.compute.machines, 1)} · 分拣器 ${fmt(plan.compute.sorters, 1)} · 传送带+物流站 ${fmt(plan.compute.logistics, 1)} · 喷涂机 ${fmt(plan.compute.coaters, 1)} · 发电 ${fmt(plan.compute.generation, 1)}（${fmt(plan.compute.cost_per_mw, 4)}/MW）`)}>
            <div className="big">{fmt(plan.compute.total, plan.compute.total < 100 ? 1 : 0)}</div><div className="muted">{t('compute units / tick', '算力单位 / tick')} <span className="nocase">{t('(excl. mining, collectors)', '（不含采矿、采集器）')}</span></div>
          </div>
        )}
        {plan.compute && (
          <div className="card" title={t('CPU time summed over threads; wall clock ≈ this ÷ effective threads. 60 UPS leaves 16 667 µs per tick. Order-of-magnitude estimate (report §2.1).', '各线程 CPU 时间之和；实际耗时 ≈ 此值 ÷ 有效线程数。60 UPS 每 tick 为 16 667 µs。数量级估算（报告 §2.1）。')}>
            <div className="big">≈ {fmtUs(plan.compute.us_per_tick)}</div>
            <div className="muted">{t('CPU / tick', 'CPU / tick')} <span className="nocase">({fmtUs(plan.compute.us_low)}–{fmtUs(plan.compute.us_high)})</span></div>
          </div>
        )}
        <div className="card" title={t(`cost ${fmt(plan.objective, 3)} · continuous bound ${fmt(plan.lp_bound, 3)}${whole && plan.proven_optimal ? ' · optimal: within 0.01 % of the bound, and when buildings are a main cost (Buildings, Power, Compute) also less than one of its cheapest buildings above it' : ''}${whole && !plan.proven_optimal ? ` · within ${fmt(100 * (plan.gap ?? 0), 3)} % of the bound${plan.gap_buildings != null ? `, about ${fmt(plan.gap_buildings, 1)}× the weight of its cheapest building (a better plan may also trade resources or power)` : ''}` : ''}`,
            `成本 ${fmt(plan.objective, 3)} · 连续下界 ${fmt(plan.lp_bound, 3)}${whole && plan.proven_optimal ? ' · 最优：与下界相差不到 0.01 %；当建筑是主要成本（建筑、电力、算力）时，还要比最便宜的建筑少一座以内' : ''}${whole && !plan.proven_optimal ? ` · 与下界相差 ${fmt(100 * (plan.gap ?? 0), 3)} %${plan.gap_buildings != null ? `，约为最便宜建筑权重的 ${fmt(plan.gap_buildings, 1)} 倍（更好的规划也可能在资源或电力上做取舍）` : ''}` : ''}`)}>
          <div className="big">{!whole ? 'LP' : plan.proven_optimal ? t('optimal', '最优') : plan.gap_buildings != null ? (plan.gap_buildings < 0.99 ? t('≈ optimal', '≈ 最优') : t(`gap ≈ ${fmt(plan.gap_buildings, plan.gap_buildings < 10 ? 1 : 0)} bldg`, `差距 ≈ ${fmt(plan.gap_buildings, plan.gap_buildings < 10 ? 1 : 0)} 座`)) : (plan.gap ?? 1) < 0.0005 ? t('≈ optimal', '≈ 最优') : t(`≤ ${fmt(100 * (plan.gap ?? 0), 1)}% gap`, `差距 ≤ ${fmt(100 * (plan.gap ?? 0), 1)}%`)}</div>
          <div className="muted" title={plan.columns ? t(`${plan.columns} solver columns (recipe × machine × modules), ${plan.rows} item rows`, `${plan.columns} 个求解器列（配方 × 机器 × 模块），${plan.rows} 个物品行`) : undefined}>
            {whole ? t('whole buildings', '整数建筑') : t('fractional buildings', '小数建筑')} · {fmt(plan.solve_ms, 0)} ms
          </div>

        </div>
      </div>
      {/* the time-limit warning is shown above the tabs with retry controls */}
      {plan.warnings.filter(w => !(s.integer && w.startsWith('Time limit reached'))).map(w => <p className="warn" key={w}>{trWarn(w)}</p>)}

      <h3>{t('Buildings', '建筑')}</h3>
      <ul className="chips">
        {[...byBuilding].map(([b, n]) => <li key={b}><Icon id={bid(b)} size={18} /><b>{n}×</b> {bld(b)}</li>)}
      </ul>

      <h3>{t('Recipes', '配方')}</h3>
      <div className="tablewrap">
        <table>
          <thead>
            <tr><th>{t('Recipe', '配方')}</th><th>{t('Building', '建筑')}</th><th className="num">{t('Buildings', '建筑数')}</th><th>{t('Clock', '时钟')}</th><th>{data.game === 'satisfactory' ? t('Somersloops', '索莫晶体') : ''}</th><th className="num">MW</th>{plan.compute && <th className="num" title={t("Share of the plan's compute (units per tick incl. power)", '占本规划算力的比例（每 tick 单位，含供电）')}>{t('Compute', '算力')}</th>}<th>{t('In (/min)', '输入 (/min)')}</th><th>{t('Out (/min)', '输出 (/min)')}</th><th></th></tr>
          </thead>
          <tbody>
            {rowGroups.map(g => g.length > 1 ? sharedRow(g) : [g[0]].map(r => (
              <tr key={r.id + '#' + r.mode}>
                <td><Icon id={r.outputs[0]?.item} />{rn(r)}{r.alt && <span className="tag">{t('alt', '替代')}</span>}{r.mode_label && <div className="muted small">{r.mode_label}</div>}</td>
                <td><Icon id={bid(r.building)} />{bld(r.building)}</td>
                <td className="num" title={t(`work = ${fmt(r.throughput, 4)} machines at 100 %`, `工作量 = ${fmt(r.throughput, 4)} 台机器（100 %）`)}>
                  {plan.quality && r.group_machines
                    ? (whole ? fmt(r.group_machines, 0) : <>{fmt(r.machines)} <span className="muted">→ {r.group_machines}</span></>)
                    : whole ? fmt(r.machines) : <>{fmt(r.machines)} <span className="muted">→ {r.machines_ceil}</span></>}
                </td>
                <td className="small">{plan.quality && r.group_machines
                  ? t(`${fmt((100 * r.machines) / r.group_machines, 1)}% busy`, `${fmt((100 * r.machines) / r.group_machines, 1)}% 负载`)
                  : r.groups.map((g, i) => <div key={i}>{groupText(g)}</div>)}
                  {data.game === 'satisfactory'
                    ? <select className="clocksel" title={t("Highest clock for this recipe (overrides the plan's max clock)", '此配方的最高时钟（覆盖规划的最高时钟）')} aria-label={t(`Max clock for ${rn(r)}`, `${rn(r)} 的最高时钟`)}
                        value={s.recipe_clocks[r.id] ?? ''} onChange={e => setClock(r.id, e.target.value)}>
                        <option value="">{t('max', '最高')} {fmt(s.max_clock_percent, 0)}% {t('(plan)', '（规划）')}</option>
                        {[...new Set([...CLOCKS.filter(c => c !== s.max_clock_percent), ...(s.recipe_clocks[r.id] !== undefined ? [s.recipe_clocks[r.id]] : [])])].sort((a, b) => a - b)
                          .map(c => <option key={c} value={c}>{t('max', '最高')} {c}%</option>)}
                      </select>
                    : null}</td>
                <td>
                  {data.game === 'satisfactory' && slots(r.id) > 0
                    ? <select value={r.id in s.somersloops ? r.somersloops : 'auto'} onChange={e => setSloops(r.id, e.target.value === 'auto' ? -1 : +e.target.value)}>
                        {s.auto_somersloops && <option value="auto">{t('auto', '自动')} ({r.somersloops})</option>}
                        {Array.from({ length: slots(r.id) + 1 }, (_, i) => <option key={i} value={i}>{i}/{slots(r.id)}</option>)}
                      </select>
                    : <span className="muted">—</span>}
                </td>
                <td className="num">{fmt(r.power, 1)}</td>
                {plan.compute && <td className="num" title={`${fmt(r.compute, 2)} units / tick`}>{fmt((100 * r.compute) / (plan.compute.total || 1), 1)}%</td>}
                <td className="small">{r.inputs.map(l => <div key={l.item}><Icon id={l.item} size={16} />{fmt(l.rate)} {itemName(data, l.item)}</div>)}</td>
                <td className="small">{r.outputs.map(l => <div key={l.item}><Icon id={l.item} size={16} />{fmt(l.rate)} {itemName(data, l.item)}</div>)}</td>
                <td><button className="x" title={t('Exclude this recipe and re-solve', '排除此配方并重新求解')} aria-label={t(`Exclude ${rn(r)}`, `排除 ${rn(r)}`)} onClick={() => exclude(r.id)}><RemoveMark /></button></td>
              </tr>
            ))[0])}
          </tbody>
        </table>
      </div>

      <div className="cols">
        <div>
          <h3>{t('Raw resources', '原矿')}</h3>
          <div className="tablewrap"><table>
            <thead><tr><th>{t('Resource', '资源')}</th><th className="num">/min</th><th className="num">{t('of world max', '占全图上限')}</th><th>{t('Carrier', '运输方式')}</th></tr></thead>
            <tbody>
              {plan.raw.map(l => {
                const lim = limit(l.item)
                return (
                  <tr key={l.item}>
                    <td><Icon id={l.item} />{itemName(data, l.item)}</td>
                    <td className="num">{fmt(l.rate)}</td>
                    <td className="num">{lim ? `${fmt((100 * l.rate) / lim, 1)}%` : '—'}</td>
                    <td className="small">{l.item === 'Electricity' ? t('power lines', '输电线') : carrier(l.rate, liquid(l.item), data)}</td>
                  </tr>
                )
              })}
              {plan.supplied.map(l => (
                <tr key={'s' + l.item}><td><Icon id={l.item} />{itemName(data, l.item)} <span className="tag">{t('input', '输入')}</span></td><td className="num">{fmt(l.rate)}</td><td /><td className="small">{l.item === 'Electricity' ? t('power lines', '输电线') : carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
            </tbody>
          </table></div>
        </div>
        <div>
          <h3>{t('Outputs', '产出')}</h3>
          <div className="tablewrap"><table>
            <thead><tr><th>{t('Item', '物品')}</th><th className="num">/min</th><th>{t('Carrier', '运输方式')}</th></tr></thead>
            <tbody>
              {plan.targets.map(l => (
                <tr key={l.item}><td><Icon id={l.item} />{itemName(data, l.item)}</td><td className="num">{fmt(l.rate)}</td><td className="small">{l.item === 'Electricity' ? t('power lines', '输电线') : carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
              {plan.surplus.map(l => (
                <tr key={'x' + l.item}><td><Icon id={l.item} />{itemName(data, l.item)} <span className="tag">{t('byproduct', '副产品')}</span></td><td className="num">{fmt(l.rate)}</td><td className="small">{l.item === 'Electricity' ? t('power lines', '输电线') : carrier(l.rate, liquid(l.item), data)}</td></tr>
              ))}
            </tbody>
          </table></div>
        </div>
      </div>
    </div>
  )
}
