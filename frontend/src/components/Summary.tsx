import type { Course, Evaluation, Needed } from '../types'
import { formatNumber, formatPercent, round } from '../format'
import { NumberInput } from './NumberInput'

interface Props {
  course: Course
  /** Latest evaluation from the server; null until the first one arrives. */
  evaluation: Evaluation | null
  custom: number | null
  onCustom: (value: number | null) => void
}

function describeNeed(need: Needed | undefined): string {
  if (!need) return '…'
  switch (need.kind) {
    case 'no-remaining':
      return 'nothing left to grade'
    case 'secured':
      return 'secured'
    case 'impossible':
      return need.best === null ? 'out of reach' : `out of reach (max ${formatPercent(need.best, 2)})`
    case 'needs':
      return `${round(need.fraction * 100, 1)}% avg`
  }
}

export function Summary({ course, evaluation, custom, onCustom }: Props) {
  const decimals = course.displayDecimals ?? 2
  const weighted = course.scheme === 'weighted'
  const targets = [...course.scale].filter((c) => c.min > 0).sort((a, b) => b.min - a.min)

  if (!evaluation) {
    return (
      <aside className="summary">
        <section className="panel total">
          <span className="label">Current grade</span>
          <div className="big">
            <span className="pct muted">…</span>
          </div>
        </section>
      </aside>
    )
  }

  const { result, remaining, needs } = evaluation
  const needFor = (target: number) => needs.find((n) => n.target === target)?.needed

  return (
    <aside className="summary">
      <section className="panel total">
        <span className="label">Current grade</span>
        <div className="big">
          <span className="pct">{formatPercent(result.percent, decimals)}</span>
          {result.letter && <span className="letter">{result.letter}</span>}
        </div>
        {course.scheme === 'points' && (
          <span className="sub">
            {formatNumber(result.earned, decimals)} / {formatNumber(result.possible, decimals)} points
          </span>
        )}
        {weighted && result.possible > 0 && (
          <span className="sub">
            Based on {formatNumber(result.possible, 2).replace(/\.?0+$/, '')}
            {' '}weight of graded categories
          </span>
        )}
      </section>

      <section className="panel breakdown">
        <h3>Breakdown</h3>
        <ul>
          {course.categories.map((category) => {
            const c = result.categories.find((r) => r.id === category.id)
            if (!c) return null
            const share =
              weighted && result.possible > 0 && c.percent !== null && category.weight > 0
                ? (category.weight / result.possible) * 100
                : null
            return (
              <li key={category.id} className={c.percent === null ? 'empty' : ''}>
                <div className="row">
                  <span className="cname">{category.name}</span>
                  <span className="cpct">{formatPercent(c.percent, decimals)}</span>
                </div>
                <div className="bar" aria-hidden="true">
                  <span style={{ width: `${Math.max(0, Math.min(100, c.percent ?? 0))}%` }} />
                </div>
                <div className="row meta">
                  <span>
                    {formatNumber(c.earned, decimals)} / {formatNumber(c.possible, decimals)}
                  </span>
                  {weighted && (
                    <span>
                      weight {category.weight}
                      {category.extraCredit ? ' (EC)' : ''}
                      {share !== null && !category.extraCredit ? ` · ${round(share, 1)}% now` : ''}
                    </span>
                  )}
                </div>
              </li>
            )
          })}
        </ul>
      </section>

      <section className="panel need">
        <h3>What do I need?</h3>
        {remaining === 0 ? (
          <p className="hint">Add ungraded items (leave the score blank) to see what you need on them.</p>
        ) : (
          <>
            <p className="hint">
              Average needed on the {remaining} remaining item{remaining === 1 ? '' : 's'} to finish with:
            </p>
            <table className="needs">
              <tbody>
                {targets.map((t) => (
                  <tr key={t.letter + t.min}>
                    <th>
                      {t.letter} <small>≥ {round(t.min, 2)}%</small>
                    </th>
                    <td>{describeNeed(needFor(t.min))}</td>
                  </tr>
                ))}
                <tr>
                  <th>
                    <NumberInput
                      className="num"
                      aria-label="Custom target percent"
                      placeholder="Target %"
                      value={custom}
                      onChange={onCustom}
                    />
                  </th>
                  <td>{custom === null ? '' : describeNeed(needFor(custom))}</td>
                </tr>
              </tbody>
            </table>
            <p className="hint">Tip: type hypothetical scores into ungraded items to try out scenarios.</p>
          </>
        )}
      </section>
    </aside>
  )
}
