/**
 * Data model for a course gradebook, and the shapes the Rust engine returns.
 * These mirror backend/src/{item,category,course}/mod.rs and backend/src/api.rs
 * (camelCase JSON); keep the two in sync.
 *
 *  - An Item is one graded thing (a quiz, an exam, a lab). Its raw result is
 *    `score / outOf`. `worth` says how much the item counts inside its category
 *    (defaults to `outOf`, i.e. plain points).
 *  - A Category groups items, may drop the lowest/highest results, and carries a
 *    weight when the course is weighted.
 *  - A Course combines categories either by total points or by weights.
 *
 * The grade lives on the same types. Fields marked "computed" are filled in
 * by the engine (`engine.evaluate` returns the graded course); they are
 * absent on the courses the browser stores and exports, and the engine
 * ignores them on input.
 */

export type Scheme = 'points' | 'weighted'

/** Whether an item counts toward its category's grade, and if not, why not. */
export type Status =
  /** Graded and part of the category's grade. */
  | 'counted'
  /** Graded, but removed by a drop-lowest / drop-highest rule. */
  | 'dropped'
  /** Marked as not counting toward the grade. */
  | 'excluded'
  /** No score yet. */
  | 'ungraded'

export interface Item {
  id: string
  name: string
  /** Raw score, or null when the item has not been graded yet. */
  score: number | null
  /** Raw points possible. 0 makes the item pure extra credit. */
  outOf: number
  /** How much the item counts inside its category. Defaults to `outOf`. */
  worth?: number
  /** Not counted toward the grade at all (Canvas "does not count toward final grade"). */
  excluded?: boolean
  /** Never removed by drop-lowest / drop-highest rules. */
  neverDrop?: boolean
  /** Free-form note shown in the UI (e.g. due date). */
  note?: string

  /** Computed: whether the item counts toward its category's grade, and if not, why not. */
  status?: Status
  /** Computed: contribution to the category, score / outOf * worth (raw score for extra credit). */
  earned?: number
  /** Computed. */
  possible?: number
}

export interface Category {
  id: string
  name: string
  /** Relative weight; only used by the 'weighted' scheme. */
  weight: number
  dropLowest?: number
  dropHighest?: number
  /** Weighted scheme only: adds to the grade without adding to the denominator. */
  extraCredit?: boolean
  items: Item[]

  /** Computed: sum over items whose status is 'counted'. */
  earned?: number
  /** Computed. */
  possible?: number
  /** Computed: earned / possible as a percentage; absent when nothing is graded. */
  percent?: number
}

export interface Cutoff {
  letter: string
  /** Minimum percentage (0–100) needed for this letter. */
  min: number
}

export interface Course {
  id: string
  name: string
  scheme: Scheme
  categories: Category[]
  /** Letter-grade cutoffs; order does not matter. */
  scale: Cutoff[]
  /** Decimal places the course's own gradebook shows for percentages. */
  displayDecimals?: number
  notes?: string

  /** Computed: points scheme, totals across categories; weighted scheme, weighted points. */
  earned?: number
  /** Computed. */
  possible?: number
  /** Computed: overall percentage; absent when nothing is graded. */
  percent?: number
  /** Computed. */
  letter?: string
}

// ---------------------------------------------------------------------------
// Engine results
// ---------------------------------------------------------------------------

export type Needed =
  | { kind: 'no-remaining' }
  | { kind: 'secured' }
  | { kind: 'impossible'; best: number | null }
  | { kind: 'needs'; fraction: number }

export interface Evaluation {
  /** The course with its computed fields filled in. */
  course: Course
  /** Ungraded items that still have points available. */
  remaining: number
  /** One entry per requested target, in request order. */
  needs: { target: number; needed: Needed }[]
}

export interface CourseSummary {
  id: string
  percent: number | null
  letter: string | null
}

export interface ExampleInfo {
  key: string
  label: string
}
