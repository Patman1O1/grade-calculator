import type { Course, CourseResult, Cutoff } from '../grade/types'
import { formatNumber, formatPercent } from '../grade/engine'
import { newId } from '../grade/courses'
import { NumberInput } from './NumberInput'

export type Mutate = (fn: (draft: Course) => void) => void

interface Props {
  course: Course
  result: CourseResult
  mutate: Mutate
}

export function CourseEditor({ course, result, mutate }: Props) {
  const weighted = course.scheme === 'weighted'
  const decimals = course.displayDecimals ?? 2
  const totalWeight = course.categories.reduce((s, c) => s + (c.extraCredit ? 0 : c.weight), 0)

  return (
    <div className="editor">
      <section className="panel course-settings">
        <label className="field grow">
          <span>Course name</span>
          <input value={course.name} onChange={(e) => mutate((d) => void (d.name = e.target.value))} />
        </label>
        <div className="field">
          <span>Calculation</span>
          <div className="segmented" role="radiogroup" aria-label="Calculation">
            {(['weighted', 'points'] as const).map((s) => (
              <button
                key={s}
                type="button"
                role="radio"
                aria-checked={course.scheme === s}
                className={course.scheme === s ? 'on' : ''}
                onClick={() => mutate((d) => void (d.scheme = s))}
              >
                {s === 'weighted' ? 'Weighted categories' : 'Total points'}
              </button>
            ))}
          </div>
        </div>
        {weighted && (
          <p className="hint">
            Category weights add up to <strong>{formatNumber(totalWeight, 2).replace(/\.?0+$/, '')}</strong>. Categories
            with nothing graded yet are left out and the rest are rescaled, like Canvas does.
          </p>
        )}
        {course.notes && <p className="hint notes">{course.notes}</p>}
      </section>

      {course.categories.map((cat, ci) => {
        const cr = result.categories[ci]
        const hasDrops = (cat.dropLowest ?? 0) > 0 || (cat.dropHighest ?? 0) > 0
        return (
          <section className="panel category" key={cat.id}>
            <header className="category-head">
              <input
                className="category-name"
                aria-label="Category name"
                value={cat.name}
                onChange={(e) => mutate((d) => void (d.categories[ci].name = e.target.value))}
              />
              <span className="category-score">
                {formatPercent(cr.percent, decimals)}
                <small>
                  {formatNumber(cr.earned, decimals)} / {formatNumber(cr.possible, decimals)}
                </small>
              </span>
              <button
                type="button"
                className="icon danger"
                title="Delete category"
                aria-label={`Delete category ${cat.name}`}
                onClick={() => mutate((d) => void d.categories.splice(ci, 1))}
              >
                ✕
              </button>
            </header>

            <div className="category-rules">
              {weighted && (
                <label className="field small">
                  <span>Weight</span>
                  <NumberInput
                    value={cat.weight}
                    nullable={false}
                    min={0}
                    onChange={(v) => mutate((d) => void (d.categories[ci].weight = v ?? 0))}
                  />
                </label>
              )}
              <label className="field small">
                <span>Drop lowest</span>
                <NumberInput
                  value={cat.dropLowest ?? 0}
                  nullable={false}
                  min={0}
                  onChange={(v) => mutate((d) => void (d.categories[ci].dropLowest = Math.floor(v ?? 0)))}
                />
              </label>
              <label className="field small">
                <span>Drop highest</span>
                <NumberInput
                  value={cat.dropHighest ?? 0}
                  nullable={false}
                  min={0}
                  onChange={(v) => mutate((d) => void (d.categories[ci].dropHighest = Math.floor(v ?? 0)))}
                />
              </label>
              {weighted && (
                <label className="check">
                  <input
                    type="checkbox"
                    checked={!!cat.extraCredit}
                    onChange={(e) => mutate((d) => void (d.categories[ci].extraCredit = e.target.checked))}
                  />
                  Extra credit category
                </label>
              )}
            </div>

            <table className="items">
              <thead>
                <tr>
                  <th className="name">Item</th>
                  <th>Score</th>
                  <th>Out of</th>
                  <th title="How much the item counts in this category. Blank = same as 'Out of'.">Worth</th>
                  <th title="Contribution to this category after scaling">Counts</th>
                  <th title="Excluded from the grade">Excl.</th>
                  {hasDrops && <th title="Never drop this item">Keep</th>}
                  <th aria-label="Actions" />
                </tr>
              </thead>
              <tbody>
                {cat.items.map((item, ii) => {
                  const ir = cr.items[ii]
                  const state = item.excluded
                    ? 'excluded'
                    : ir.dropped
                      ? 'dropped'
                      : item.score === null
                        ? 'pending'
                        : ''
                  return (
                    <tr key={item.id} className={state}>
                      <td className="name">
                        <input
                          aria-label="Item name"
                          value={item.name}
                          onChange={(e) => mutate((d) => void (d.categories[ci].items[ii].name = e.target.value))}
                        />
                        {item.note && <small className="note">{item.note}</small>}
                      </td>
                      <td>
                        <NumberInput
                          aria-label={`${item.name} score`}
                          className="num"
                          placeholder="–"
                          value={item.score}
                          onChange={(v) => mutate((d) => void (d.categories[ci].items[ii].score = v))}
                        />
                      </td>
                      <td>
                        <NumberInput
                          aria-label={`${item.name} out of`}
                          className="num"
                          nullable={false}
                          min={0}
                          value={item.outOf}
                          onChange={(v) => mutate((d) => void (d.categories[ci].items[ii].outOf = v ?? 0))}
                        />
                      </td>
                      <td>
                        <NumberInput
                          aria-label={`${item.name} worth`}
                          className="num"
                          min={0}
                          placeholder={String(item.outOf)}
                          value={item.worth ?? null}
                          onChange={(v) =>
                            mutate((d) => {
                              const it = d.categories[ci].items[ii]
                              if (v === null) delete it.worth
                              else it.worth = v
                            })
                          }
                        />
                      </td>
                      <td className="counts">
                        {state === 'dropped' ? (
                          <span className="tag">dropped</span>
                        ) : state === 'excluded' ? (
                          <span className="tag">excluded</span>
                        ) : state === 'pending' ? (
                          <span className="muted">—</span>
                        ) : (
                          `${formatNumber(ir.earned, decimals)} / ${formatNumber(ir.possible, decimals)}`
                        )}
                      </td>
                      <td>
                        <input
                          type="checkbox"
                          aria-label={`Exclude ${item.name}`}
                          checked={!!item.excluded}
                          onChange={(e) =>
                            mutate((d) => void (d.categories[ci].items[ii].excluded = e.target.checked))
                          }
                        />
                      </td>
                      {hasDrops && (
                        <td>
                          <input
                            type="checkbox"
                            aria-label={`Never drop ${item.name}`}
                            checked={!!item.neverDrop}
                            onChange={(e) =>
                              mutate((d) => void (d.categories[ci].items[ii].neverDrop = e.target.checked))
                            }
                          />
                        </td>
                      )}
                      <td>
                        <button
                          type="button"
                          className="icon danger"
                          aria-label={`Delete ${item.name}`}
                          title="Delete item"
                          onClick={() => mutate((d) => void d.categories[ci].items.splice(ii, 1))}
                        >
                          ✕
                        </button>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
            <button
              type="button"
              className="link"
              onClick={() =>
                mutate((d) => {
                  const items = d.categories[ci].items
                  const last = items[items.length - 1]
                  items.push({
                    id: newId('item'),
                    name: `${d.categories[ci].name} ${items.length + 1}`,
                    score: null,
                    outOf: last?.outOf ?? 10,
                    ...(last?.worth !== undefined ? { worth: last.worth } : {}),
                  })
                })
              }
            >
              + Add item
            </button>
          </section>
        )
      })}

      <button
        type="button"
        className="add-category"
        onClick={() =>
          mutate((d) =>
            void d.categories.push({
              id: newId('cat'),
              name: 'New category',
              weight: 0,
              items: [],
            }),
          )
        }
      >
        + Add category
      </button>

      <ScaleEditor course={course} mutate={mutate} />
    </div>
  )
}

function ScaleEditor({ course, mutate }: { course: Course; mutate: Mutate }) {
  const rows: Cutoff[] = course.scale
  return (
    <section className="panel scale">
      <header className="category-head">
        <h3>Letter grade scale</h3>
      </header>
      <table className="items">
        <thead>
          <tr>
            <th className="name">Letter</th>
            <th>Minimum %</th>
            <th aria-label="Actions" />
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i}>
              <td className="name">
                <input
                  aria-label="Letter"
                  value={row.letter}
                  onChange={(e) => mutate((d) => void (d.scale[i].letter = e.target.value))}
                />
              </td>
              <td>
                <NumberInput
                  aria-label={`${row.letter} minimum`}
                  className="num"
                  nullable={false}
                  value={Number(row.min.toFixed(4))}
                  onChange={(v) => mutate((d) => void (d.scale[i].min = v ?? 0))}
                />
              </td>
              <td>
                <button
                  type="button"
                  className="icon danger"
                  aria-label={`Delete ${row.letter}`}
                  onClick={() => mutate((d) => void d.scale.splice(i, 1))}
                >
                  ✕
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <button
        type="button"
        className="link"
        onClick={() => mutate((d) => void d.scale.push({ letter: '?', min: 0 }))}
      >
        + Add letter
      </button>
    </section>
  )
}
