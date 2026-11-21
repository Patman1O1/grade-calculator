import { addItem, hasDropRules, newCategory } from '../category'
import {
  decimals as decimalsOf,
  newCutoff,
  totalWeight as totalWeightOf,
  type Course,
  type EngineCourse,
  type Cutoff,
  type Mutate,
} from '../course'
import { formatNumber, formatPercent } from '../format'
import { percentOf, rowClass } from '../item'
import { NumberInput } from './NumberInput'

interface Props {
  course: Course
  /**
   * Graded copy of the course from the engine; null if it couldn't be graded.
   * Its categories and items are in the same order as `course`'s.
   */
  graded: EngineCourse | null
  mutate: Mutate
}

const PENDING = '…'

export function CourseEditor({ course, graded, mutate }: Props) {
  const weighted = course.scheme === 'weighted'
  const decimals = decimalsOf(course)
  const totalWeight = totalWeightOf(course)

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
        const gc = graded?.categories[ci]
        const hasDrops = hasDropRules(cat)
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
                {gc ? formatPercent(gc.percent, decimals) : PENDING}
                <small>
                  {gc ? `${formatNumber(gc.earned ?? 0, decimals)} / ${formatNumber(gc.possible ?? 0, decimals)}` : ''}
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
                  <th title="Points earned on this item. Leave blank if it hasn't been graded yet.">Earned Points</th>
                  <th title="Points the item is out of. This also sets how much it counts in its category; 0 makes it extra credit.">
                    Total Points
                  </th>
                  <th title="The item's percentage, or why it doesn't count">Counts</th>
                  <th title="Excluded from the grade">Excl.</th>
                  {hasDrops && <th title="Never drop this item">Keep</th>}
                  <th aria-label="Actions" />
                </tr>
              </thead>
              <tbody>
                {cat.items.map((item, ii) => {
                  const gi = gc?.items[ii]
                  const status = gi?.status
                  return (
                    <tr key={item.id} className={rowClass(status)}>
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
                          aria-label={`${item.name} earned points`}
                          className="num"
                          placeholder="–"
                          value={item.earned}
                          onChange={(v) => mutate((d) => void (d.categories[ci].items[ii].earned = v))}
                        />
                      </td>
                      <td>
                        <NumberInput
                          aria-label={`${item.name} total points`}
                          className="num"
                          nullable={false}
                          min={0}
                          value={item.possible}
                          onChange={(v) => mutate((d) => void (d.categories[ci].items[ii].possible = v ?? 0))}
                        />
                      </td>
                      <td className="counts">
                        {status === 'counted' ? (
                          item.possible > 0 ? (
                            formatPercent(percentOf(item), decimals)
                          ) : (
                            `+${formatNumber(item.earned ?? 0, decimals)} extra`
                          )
                        ) : status === 'dropped' || status === 'excluded' ? (
                          <span className="tag">{status}</span>
                        ) : status === 'ungraded' ? (
                          <span className="muted">—</span>
                        ) : (
                          <span className="muted">{PENDING}</span>
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
              onClick={() => mutate((d) => addItem(d.categories[ci]))}
            >
              + Add item
            </button>
          </section>
        )
      })}

      <button
        type="button"
        className="add-category"
        onClick={() => mutate((d) => void d.categories.push(newCategory()))}
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
        onClick={() => mutate((d) => void d.scale.push(newCutoff()))}
      >
        + Add letter
      </button>
    </section>
  )
}
