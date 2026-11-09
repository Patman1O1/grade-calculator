import { describe, expect, it } from 'vitest'
import { computeCategory, computeCourse, letterFor, neededOnRemaining, round } from './engine'
import { STANDARD_SCALE } from './courses'
import type { Category, Course, Item } from './types'

let n = 0
const it_ = (score: number | null, outOf: number, extra: Partial<Item> = {}): Item => ({
  id: `i${++n}`,
  name: `Item ${n}`,
  score,
  outOf,
  ...extra,
})
const cat = (items: Item[], extra: Partial<Category> = {}): Category => ({
  id: `c${++n}`,
  name: `Cat ${n}`,
  weight: 0,
  items,
  ...extra,
})
const course = (scheme: Course['scheme'], categories: Category[]): Course => ({
  id: 'x',
  name: 'x',
  scheme,
  categories,
  scale: STANDARD_SCALE,
})

describe('drop rules', () => {
  it('drops the item that maximises the result, not just the lowest percentage', () => {
    // Dropping the 0/1 gives 40/110 = 36%; dropping the 30/100 gives 10/11 = 91%.
    const r = computeCategory(cat([it_(0, 1), it_(30, 100), it_(10, 10)], { dropLowest: 1 }))
    expect(r.items.map((i) => i.dropped)).toEqual([false, true, false])
    expect(round(r.percent!, 2)).toBe(90.91)
  })

  it('drop highest removes the best result', () => {
    const r = computeCategory(cat([it_(10, 10), it_(5, 10), it_(7, 10)], { dropHighest: 1 }))
    expect(r.items.map((i) => i.dropped)).toEqual([true, false, false])
    expect(r.percent).toBe(60)
  })

  it('never drops neverDrop items and always keeps at least one item', () => {
    const r = computeCategory(cat([it_(0, 10, { neverDrop: true }), it_(5, 10)], { dropLowest: 1 }))
    expect(r.items.map((i) => i.dropped)).toEqual([false, true])
    const single = computeCategory(cat([it_(2, 10)], { dropLowest: 3 }))
    expect(single.percent).toBe(20)
  })

  it('ignores ungraded and excluded items when dropping', () => {
    const r = computeCategory(
      cat([it_(null, 10), it_(1, 10, { excluded: true }), it_(4, 10), it_(8, 10)], { dropLowest: 1 }),
    )
    expect(r.percent).toBe(80)
  })
})

describe('item worth', () => {
  it('scales a raw score to its worth', () => {
    const r = computeCategory(cat([it_(85, 100, { worth: 10 })]))
    expect(r.earned).toBeCloseTo(8.5)
    expect(r.possible).toBe(10)
  })
  it('treats zero-point items as extra credit', () => {
    const r = computeCategory(cat([it_(9, 10), it_(2, 0)]))
    expect(r.earned).toBe(11)
    expect(r.possible).toBe(10)
  })
})

describe('weighted scheme', () => {
  it('renormalises over categories that have graded work', () => {
    const r = computeCourse(
      course('weighted', [
        cat([it_(9, 10)], { weight: 30 }),
        cat([it_(70, 100)], { weight: 20 }),
        cat([it_(null, 100)], { weight: 50 }),
      ]),
    )
    expect(round(r.percent!, 4)).toBe(round(((30 * 0.9 + 20 * 0.7) / 50) * 100, 4))
  })
  it('extra-credit categories add without raising the denominator', () => {
    const r = computeCourse(
      course('weighted', [cat([it_(80, 100)], { weight: 100 }), cat([it_(1, 1)], { weight: 5, extraCredit: true })]),
    )
    expect(r.percent).toBeCloseTo(85)
  })
})

describe('letters and the needed-score solver', () => {
  it('maps percentages to letters', () => {
    expect(letterFor(89.99, STANDARD_SCALE)).toBe('B')
    expect(letterFor(90, STANDARD_SCALE)).toBe('A')
    expect(letterFor(null, STANDARD_SCALE)).toBeNull()
  })
  it('finds the average needed on remaining work', () => {
    // 60/100 so far, 100 points left; 80% overall needs 100/100.
    const c = course('points', [cat([it_(60, 100), it_(null, 100)])])
    const res = neededOnRemaining(c, 80)
    expect(res.kind).toBe('needs')
    if (res.kind === 'needs') expect(res.fraction).toBeCloseTo(1, 6)
    expect(neededOnRemaining(c, 70)).toMatchObject({ kind: 'needs' })
    expect(neededOnRemaining(c, 81).kind).toBe('impossible')
    expect(neededOnRemaining(c, 30).kind).toBe('secured')
  })
})
