import { useState } from 'react'
import type { Course, CourseResult } from '../grade/types'
import { formatNumber, formatPercent, neededOnRemaining, round } from '../grade/engine'
import { NumberInput } from './NumberInput'

interface Props {
  course: Course
  result: CourseResult
}

function describeNeed(course: Course, target: number): string {
  const need = neededOnRemaining(course, target)
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

export function Summary({ course, result }: Props) {
  const [custom, setCustom] = useState<number | null>(null)
  const decimals = course.displayDecimals ?? 2
  const weighted = course.scheme === 'weighted'
  const remaining = course.categories.reduce(
    (n, c) => n + c.items.filter((i) => i.score === null && !i.excluded && i.outOf > 0).length,
    0,
  )
  const targets = [...course.scale].filter((c) => c.min > 0).sort((a, b) => b.min - a.min)

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
          {result.categories.map((c) => {
            const share =
              weighted && result.possible > 0 && c.percent !== null && c.category.weight > 0
                ? (c.category.weight / result.possible) * 100
                : null
            return (
              <li key={c.category.id} className={c.percent === null ? 'empty' : ''}>
                <div className="row">
                  <span className="cname">{c.category.name}</span>
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
                      weight {c.category.weight}
                      {c.category.extraCredit ? ' (EC)' : ''}
                      {share !== null && !c.category.extraCredit ? ` · ${round(share, 1)}% now` : ''}
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
                    <td>{describeNeed(course, t.min)}</td>
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
                  <td>{custom === null ? '' : describeNeed(course, custom)}</td>
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
