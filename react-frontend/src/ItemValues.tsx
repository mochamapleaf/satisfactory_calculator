import { Icon } from './Icon'
import { fmt, type Plan, type Settings } from './solver'

// Shadow prices of the continuous relaxation: d(cost) / d(one more unit/min of the item).
export default function ItemValues({ plan, settings }: { plan: Plan; settings: Settings }) {
  if (plan.status !== 'optimal') return <p className="muted">Solve a plan first.</p>
  const unit = settings.objective === 'custom' ? 'cost units' : settings.objective === 'resources'
    ? 'Iron-Ore equivalents' : settings.objective === 'buildings' ? 'buildings' : 'MW-equivalents'
  const max = Math.max(1e-9, ...plan.item_values.map(v => Math.abs(v.value)))
  return (
    <div className="results">
      <p className="hint">
        Marginal cost of one more item per minute under the current cost function, in {unit}{' '}
        (shadow prices of the continuous relaxation, i.e. with fractional buildings). A raw resource
        at its limit shows how much the plan would gain per extra unit/min; a byproduct with value 0
        is free to dump.
      </p>
      <table>
        <thead><tr><th>Item</th><th className="num">Value</th><th></th></tr></thead>
        <tbody>
          {plan.item_values.map(v => (
            <tr key={v.item}>
              <td><Icon id={v.item} />{v.name}</td>
              <td className="num">{fmt(v.value, 4)}</td>
              <td style={{ width: '40%' }}>
                <div className="bar" style={{ width: `${(100 * Math.abs(v.value)) / max}%`, opacity: v.value < 0 ? 0.4 : 1 }} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="hint">Continuous lower bound on the cost: {fmt(plan.lp_bound, 3)} · whole-building plan: {fmt(plan.objective, 3)}</p>
    </div>
  )
}
