import { Icon } from './Icon'
import { t } from './i18n'
import { fmt, itemName, type GameData, type Plan, type Settings } from './solver'

// Shadow prices of the continuous relaxation: d(cost) / d(one more unit/min of the item).
export default function ItemValues({ data, plan, settings }: { data: GameData; plan: Plan; settings: Settings }) {
  if (plan.status !== 'optimal') return <p className="muted">{t('Solve a plan first.', '请先求解一个规划。')}</p>
  const unit = settings.objective === 'custom' ? t('cost units', '成本单位') : settings.objective === 'resources'
    ? t('Iron-Ore equivalents', '铁矿当量') : settings.objective === 'buildings' ? t('buildings', '建筑') : t('MW-equivalents', 'MW 当量')
  const max = Math.max(1e-9, ...plan.item_values.map(v => Math.abs(v.value)))
  return (
    <div className="results">
      <p className="hint">
        {t(`Marginal cost of one more item per minute under the current cost function, in ${unit} (shadow prices of the continuous relaxation, i.e. with fractional buildings). A raw resource at its limit shows how much the plan would gain per extra unit/min; a byproduct with value 0 is free to dump.`,
          `在当前成本函数下，每分钟多产 1 个物品的边际成本，单位：${unit}（连续松弛的影子价格，即允许小数建筑）。达到上限的原矿显示每多 1 个/分钟规划能获得的收益；价值为 0 的副产品可以随意丢弃。`)}
      </p>
      <div className="tablewrap"><table>
        <thead><tr><th>{t('Item', '物品')}</th><th className="num">{t('Value', '价值')}</th><th>{t('Relative', '相对')}</th></tr></thead>
        <tbody>
          {plan.item_values.map(v => (
            <tr key={v.item}>
              <td><Icon id={v.item} />{itemName(data, v.item)}</td>
              <td className="num">{fmt(v.value, 4)}</td>
              <td style={{ width: '40%', verticalAlign: 'middle' }}>
                <div className="bar" style={{ width: `${(100 * Math.abs(v.value)) / max}%`, opacity: v.value < 0 ? 0.4 : 1 }} />
              </td>
            </tr>
          ))}
        </tbody>
      </table></div>
      <p className="hint">{t('Continuous lower bound on the cost', '成本的连续下界')}: {fmt(plan.lp_bound, 3)} · {t('whole-building plan', '整数建筑规划')}: {fmt(plan.objective, 3)}</p>
    </div>
  )
}
