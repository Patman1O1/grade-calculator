import { decimals as decimalsOf, describeNeed, letterTargets, type Course } from '../course'
import type { Evaluation } from '../engine'
import { formatNumber, formatPercent, round } from '../format'
import { NumberInput } from './NumberInput'

interface Props {
  course: Course
  /** The engine's evaluation; null if the course couldn't be graded. */
  evaluation: Evaluation | null
  custom: number | null
  onCustom: (value: number | null) => void
}

export function Summary({ course, evaluation, custom, onCustom }: Props) {
  const decimals = decimalsOf(course)
  const weighted = course.scheme === 'weighted'
  const targets = letterTargets(course)

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

  const { course: graded, remaining, needs } = evaluation
  const gradedPossible = graded.possible ?? 0
  const needFor = (target: number) => needs.find((n) => n.target === target)?.needed

  return (
    <aside className="summary">
      <section className="panel total">
        <span className="label">Current grade</span>
        <div className="big">
          <span className="pct">{formatPercent(graded.percent, decimals)}</span>
          {graded.letter && <span className="letter">{graded.letter}</span>}
        </div>
        {course.scheme === 'points' && (
          <span className="sub">
            {formatNumber(graded.earned ?? 0, decimals)} / {formatNumber(gradedPossible, decimals)} points
          </span>
        )}
        {weighted && gradedPossible > 0 && (
          <span className="sub">
            Based on {formatNumber(gradedPossible, 2).replace(/\.?0+$/, '')}
            {' '}weight of graded categories
          </span>
        )}
      </section>

      <section className="panel breakdown">
        <h3>Breakdown</h3>
        <ul>
          {course.categories.map((category) => {
            const c = graded.categories.find((g) => g.id === category.id)
            if (!c) return null
            const share =
              weighted && gradedPossible > 0 && c.percent !== undefined && category.weight > 0
                ? (category.weight / gradedPossible) * 100
                : null
            return (
              <li key={category.id} className={c.percent === undefined ? 'empty' : ''}>
                <div className="row">
                  <span className="cname">{category.name}</span>
                  <span className="cpct">{formatPercent(c.percent, decimals)}</span>
                </div>
                <div className="bar" aria-hidden="true">
                  <span style={{ width: `${Math.max(0, Math.min(100, c.percent ?? 0))}%` }} />
                </div>
                <div className="row meta">
                  <span>
                    {formatNumber(c.earned ?? 0, decimals)} / {formatNumber(c.possible ?? 0, decimals)}
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
