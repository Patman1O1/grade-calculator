import { decimals as decimalsOf, describeNeed, letterTargets, type Course, type EngineCourse, type Mutate } from '../course'
import { formatNumber, formatPercent, round } from '../format'
import { NumberInput } from './NumberInput'

interface Props {
  course: Course
  /** The course as `engine.evaluate` graded it; null if it couldn't be graded. */
  graded: EngineCourse | null
  mutate: Mutate
}

export function Summary({ course, graded, mutate }: Props) {
  const decimals = decimalsOf(course)
  const weighted = course.scheme === 'weighted'
  const targets = letterTargets(course)

  if (!graded) {
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

  const gradedPossible = graded.possible ?? 0
  const remaining = graded.remaining ?? 0
  // One per letter target, then the course's own target.
  const needs = graded.needs ?? []
  const custom = course.target ?? null
  const setCustom = (value: number | null) =>
    mutate((d) => {
      if (value === null) delete d.target
      else d.target = value
    })

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
          {course.categories.map((category, ci) => {
            const c = graded.categories[ci]
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
                {targets.map((t, i) => (
                  <tr key={t.grade}>
                    <th>
                      {t.letter} <small>≥ {round(t.min, 2)}%</small>
                    </th>
                    <td>{describeNeed(needs[i], graded.best)}</td>
                  </tr>
                ))}
                <tr>
                  <th>
                    <NumberInput
                      className="num"
                      aria-label="Custom target percent"
                      placeholder="Target %"
                      value={custom}
                      onChange={setCustom}
                    />
                  </th>
                  <td>{custom === null ? '' : describeNeed(needs[targets.length], graded.best)}</td>
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
