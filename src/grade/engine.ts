import type {
  Category,
  CategoryResult,
  Course,
  CourseResult,
  Cutoff,
  Item,
  ItemResult,
} from './types'

/** Effective worth of an item inside its category. */
export function itemWorth(item: Item): number {
  return item.worth ?? item.outOf
}

/** Points an item contributes to its category, given a score. */
function contribution(item: Item, score: number): { earned: number; possible: number } {
  const worth = itemWorth(item)
  if (item.outOf > 0) {
    return { earned: (score / item.outOf) * worth, possible: worth }
  }
  // Zero-point items are pure extra credit: the raw score is added on top.
  return { earned: score, possible: 0 }
}

interface Candidate {
  index: number
  earned: number
  possible: number
}

/**
 * Choose which `keep` candidates to keep so that
 *   (fixedEarned + Σ earned) / (fixedPossible + Σ possible)
 * is maximised (mode 'max', i.e. dropping the lowest) or minimised
 * (mode 'min', i.e. dropping the highest).
 *
 * When items are worth different amounts, "drop the lowest percentage" is not
 * always optimal. Canvas solves this exactly with a parametric search on the
 * resulting ratio q: a ratio q is achievable iff the best `keep` values of
 * (earned − q·possible) sum to at least −(fixedEarned − q·fixedPossible).
 * We do the same, so results match Canvas for any mix of point values.
 */
function chooseKept(
  candidates: Candidate[],
  keep: number,
  fixedEarned: number,
  fixedPossible: number,
  mode: 'max' | 'min',
): Set<number> {
  if (keep >= candidates.length) return new Set(candidates.map((c) => c.index))
  if (keep <= 0) return new Set()

  const sign = mode === 'max' ? 1 : -1
  const pick = (q: number) =>
    [...candidates]
      .map((c) => ({ c, v: sign * (c.earned - q * c.possible) }))
      // Stable tie-break: prefer keeping earlier items.
      .sort((a, b) => b.v - a.v || a.c.index - b.c.index)
      .slice(0, keep)
  const feasible = (q: number) =>
    sign * (fixedEarned - q * fixedPossible) + pick(q).reduce((s, x) => s + x.v, 0) >= 0

  const minPositive = Math.min(
    ...[fixedPossible, ...candidates.map((c) => c.possible)].filter((p) => p > 0),
  )
  if (!Number.isFinite(minPositive)) {
    // Nothing has a denominator: only extra credit, so keep the largest (or smallest) scores.
    return new Set(pick(0).map((x) => x.c.index))
  }

  const totalEarned = fixedEarned + candidates.reduce((s, c) => s + Math.max(0, c.earned), 0)
  let lo = 0
  let hi = totalEarned / minPositive + 1
  if (mode === 'max') {
    // Largest achievable q.
    for (let i = 0; i < 200; i++) {
      const mid = (lo + hi) / 2
      if (feasible(mid)) lo = mid
      else hi = mid
    }
    return new Set(pick(lo).map((x) => x.c.index))
  }
  // Smallest achievable q (feasible means "ratio ≤ q is reachable").
  for (let i = 0; i < 200; i++) {
    const mid = (lo + hi) / 2
    if (feasible(mid)) hi = mid
    else lo = mid
  }
  return new Set(pick(hi).map((x) => x.c.index))
}

export function computeCategory(category: Category): CategoryResult {
  const items: ItemResult[] = category.items.map((item) => {
    const graded = item.score !== null && !item.excluded
    const c = graded ? contribution(item, item.score as number) : { earned: 0, possible: 0 }
    return { item, counted: graded, dropped: false, ...c }
  })

  const dropLowest = Math.max(0, Math.floor(category.dropLowest ?? 0))
  const dropHighest = Math.max(0, Math.floor(category.dropHighest ?? 0))
  if (dropLowest > 0 || dropHighest > 0) {
    const fixed = items.filter((r) => r.counted && r.item.neverDrop)
    let pool: Candidate[] = items
      .map((r, index) => ({ r, index }))
      .filter(({ r }) => r.counted && !r.item.neverDrop)
      .map(({ r, index }) => ({ index, earned: r.earned, possible: r.possible }))
    const fixedEarned = fixed.reduce((s, r) => s + r.earned, 0)
    const fixedPossible = fixed.reduce((s, r) => s + r.possible, 0)
    // Like Canvas, never drop every item: at least one stays.
    const minKeep = fixed.length > 0 ? 0 : 1

    const lowDrops = Math.min(dropLowest, Math.max(0, pool.length - minKeep))
    if (lowDrops > 0) {
      const kept = chooseKept(pool, pool.length - lowDrops, fixedEarned, fixedPossible, 'max')
      pool = pool.filter((c) => kept.has(c.index))
    }
    const highDrops = Math.min(dropHighest, Math.max(0, pool.length - minKeep))
    if (highDrops > 0) {
      const kept = chooseKept(pool, pool.length - highDrops, fixedEarned, fixedPossible, 'min')
      pool = pool.filter((c) => kept.has(c.index))
    }

    const keptIndexes = new Set(pool.map((c) => c.index))
    items.forEach((r, index) => {
      if (r.counted && !r.item.neverDrop && !keptIndexes.has(index)) {
        r.counted = false
        r.dropped = true
      }
    })
  }

  const counted = items.filter((r) => r.counted)
  const earned = counted.reduce((s, r) => s + r.earned, 0)
  const possible = counted.reduce((s, r) => s + r.possible, 0)
  return {
    category,
    items,
    earned,
    possible,
    percent: possible > 0 ? (earned / possible) * 100 : null,
  }
}

export function letterFor(percent: number | null, scale: Cutoff[]): string | null {
  if (percent === null || scale.length === 0) return null
  const sorted = [...scale].sort((a, b) => b.min - a.min)
  // Small tolerance so 89.99999999 from floating point still earns the 90 cutoff.
  const hit = sorted.find((c) => percent + 1e-9 >= c.min)
  return (hit ?? sorted[sorted.length - 1]).letter
}

export function computeCourse(course: Course): CourseResult {
  const categories = course.categories.map(computeCategory)
  let earned = 0
  let possible = 0

  if (course.scheme === 'points') {
    for (const c of categories) {
      earned += c.earned
      possible += c.possible
    }
  } else {
    for (const c of categories) {
      const w = c.category.weight
      if (!(w > 0) || c.percent === null) continue
      earned += (w * c.percent) / 100
      if (!c.category.extraCredit) possible += w
    }
  }

  const percent = possible > 0 ? (earned / possible) * 100 : null
  return { course, categories, earned, possible, percent, letter: letterFor(percent, course.scale) }
}

/** Every ungraded, non-excluded item that still has points available. */
function remainingItems(course: Course): Item[] {
  return course.categories.flatMap((c) =>
    c.items.filter((i) => i.score === null && !i.excluded && i.outOf > 0),
  )
}

/** Course with every remaining item scored at `fraction` of its points. */
export function withRemainingAt(course: Course, fraction: number): Course {
  return {
    ...course,
    categories: course.categories.map((c) => ({
      ...c,
      items: c.items.map((i) =>
        i.score === null && !i.excluded && i.outOf > 0 ? { ...i, score: i.outOf * fraction } : i,
      ),
    })),
  }
}

export type NeededResult =
  | { kind: 'no-remaining' }
  | { kind: 'secured' }
  | { kind: 'impossible'; best: number | null }
  | { kind: 'needs'; fraction: number }

/**
 * Average score (as a fraction of points) needed on every remaining item to
 * finish at or above `target` percent. "Impossible" means even 100% everywhere
 * falls short.
 */
export function neededOnRemaining(course: Course, target: number): NeededResult {
  if (remainingItems(course).length === 0) return { kind: 'no-remaining' }
  const at = (f: number) => computeCourse(withRemainingAt(course, f)).percent ?? 0
  if (at(0) + 1e-9 >= target) return { kind: 'secured' }
  if (at(1) + 1e-9 < target) return { kind: 'impossible', best: at(1) }
  let lo = 0
  let hi = 1
  for (let i = 0; i < 60; i++) {
    const mid = (lo + hi) / 2
    if (at(mid) + 1e-9 >= target) hi = mid
    else lo = mid
  }
  return { kind: 'needs', fraction: hi }
}

/** Round half away from zero, immune to binary noise like 52.549999999. */
export function round(x: number, decimals: number): number {
  const f = 10 ** decimals
  const clean = Number(x.toFixed(10))
  return (Math.sign(clean) * Math.round(Math.abs(clean) * f)) / f
}

/** Format like Canvas: up to `decimals` places, trailing zeros trimmed. */
export function formatPercent(x: number | null, decimals = 2): string {
  if (x === null) return 'N/A'
  return `${round(x, decimals)}%`
}

export function formatNumber(x: number, decimals = 2): string {
  return round(x, decimals).toFixed(decimals)
}
