import { useEffect, useState } from 'react'
import { t } from './i18n'
import { clampTime, MAX_TIME_MS } from './solver'

/** Seconds field for the solver time limit: typing is free, the value is applied on Enter or blur. */
export default function TimeInput({ ms, onChange }: { ms: number; onChange: (ms: number) => void }) {
  const [draft, setDraft] = useState(String(ms / 1000))
  useEffect(() => setDraft(String(ms / 1000)), [ms])
  const commit = () => {
    const v = parseFloat(draft)
    if (!Number.isFinite(v)) { setDraft(String(ms / 1000)); return }
    const next = clampTime(v)
    setDraft(String(next / 1000))
    if (next !== ms) onChange(next)
  }
  return (
    <input type="number" min={0.2} max={MAX_TIME_MS / 1000} step={0.5} value={draft} aria-label={t('Time limit in seconds', '时间限制（秒）')}
      onChange={e => setDraft(e.target.value)} onBlur={commit}
      onKeyDown={e => { if (e.key === 'Enter') commit() }} />
  )
}
