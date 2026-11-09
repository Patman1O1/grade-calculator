/**
 * Data model for a course gradebook.
 *
 * The model is deliberately small but general enough to express the grading
 * policies used by Canvas, Blackboard and typical syllabi:
 *
 *  - An Item is one graded thing (a quiz, an exam, a lab). Its raw result is
 *    `score / outOf`. `worth` says how much the item counts inside its category
 *    (defaults to `outOf`, i.e. plain points). Blackboard-style "this 0–100 report
 *    is worth 10 course points" is expressed as `outOf: 100, worth: 10`; a syllabus
 *    rule like "each exam counts equally" is expressed by giving the items equal
 *    worth regardless of their raw point totals.
 *  - A Category groups items, may drop the lowest/highest results, and carries a
 *    weight when the course is weighted.
 *  - A Course combines categories either by total points or by weights.
 */

export type Scheme =
  /** Sum of points earned / sum of points possible across all categories. */
  | 'points'
  /**
   * Each category's percentage multiplied by its weight. Categories with no
   * graded work are left out and the remaining weights are renormalised
   * (this is what Canvas and most syllabi do mid-semester).
   */
  | 'weighted'

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
}

export interface Category {
  id: string
  name: string
  /** Relative weight; only used by the 'weighted' scheme. */
  weight: number
  dropLowest?: number
  dropHighest?: number
  /**
   * Weighted scheme only: the category's weighted score is added on top of the
   * grade, but its weight is not part of the denominator.
   */
  extraCredit?: boolean
  items: Item[]
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
}

export interface ItemResult {
  item: Item
  /** Item is graded, not excluded and not dropped. */
  counted: boolean
  dropped: boolean
  /** Contribution to the category: score / outOf * worth (raw score for extra credit). */
  earned: number
  possible: number
}

export interface CategoryResult {
  category: Category
  items: ItemResult[]
  earned: number
  possible: number
  /** earned / possible as a percentage, or null when nothing is graded. */
  percent: number | null
}

export interface CourseResult {
  course: Course
  categories: CategoryResult[]
  /** Points scheme: totals across categories. Weighted scheme: weighted points. */
  earned: number
  possible: number
  /** Overall percentage, or null when nothing is graded. */
  percent: number | null
  letter: string | null
}
