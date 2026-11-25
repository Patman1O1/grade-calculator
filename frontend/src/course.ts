/**
 * Course: categories combined by total points or by weights, with a letter
 * scale and the "what do I need?" results. Mirrors backend/src/course/mod.rs
 * (camelCase JSON); keep the two in sync.
 */
import { withCategoryIds, type Category, type EngineCategory } from './category'
import { formatPercent, newId, round } from './format'
import { cutoffs, type Cutoff, type Scale } from './scale'

/** How a course combines its categories. */
export type Scheme = 'points' | 'weighted'

export interface Course {
  id: string
  name: string
  scheme: Scheme
  categories: Category[]
  /** Letter-grade cutoffs. */
  scale: Scale
  /** Decimal places the course's own gradebook shows for percentages. */
  displayDecimals?: number
  notes?: string
  /** The user's own target percentage for "what do I need?", solved after the letter cutoffs. */
  target?: number

  /** Computed: points scheme, totals across categories; weighted scheme, weighted points. */
  earned?: number
  /** Computed. */
  possible?: number
  /** Computed: overall percentage; absent when nothing is graded. */
  percent?: number
  /** Computed. */
  letter?: string

  /** Computed by `engine.evaluate`: items still to be graded. */
  remaining?: number
  /** Computed by `engine.evaluate`: the grade with full marks on all remaining work; absent if undefined. */
  best?: number
  /**
   * Computed by `engine.evaluate`, one per letter in `letterTargets` order and
   * then one for `target` if set: the average needed on the remaining work as
   * a fraction of its points (0 when already secured), or null when out of
   * reach even with full marks.
   */
  needs?: (number | null)[]
}

/** A course as the engine returns it: no ids (ids belong to the front-end). */
export type EngineCourse = Omit<Course, 'id' | 'categories'> & { categories: EngineCategory[] }

/** Give a course from the engine, and everything in it, new ids. */
export function withIds(course: EngineCourse): Course {
  return { id: newId('course'), ...course, categories: course.categories.map(withCategoryIds) }
}

/** Edit the selected course: `fn` receives a draft copy to change in place. */
export type Mutate = (fn: (draft: Course) => void) => void

/** Decimal places to show this course's percentages with. */
export function decimals(course: Course): number {
  return course.displayDecimals ?? 2
}

/** Letters with a minimum above 0, highest first: the targets the engine solves for, before `target`. */
export function letterTargets(course: Course): Cutoff[] {
  return cutoffs(course.scale)
    .filter((c) => c.min > 0)
    .sort((a, b) => b.min - a.min)
}

/** Sum of the weights of the non-extra-credit categories. */
export function totalWeight(course: Course): number {
  return course.categories.reduce((sum, c) => sum + (c.extraCredit ? 0 : c.weight), 0)
}

/** Return `courses` with the course `id` replaced by an edited copy (`fn` edits a draft). */
export function updateCourse(courses: Course[], id: string, fn: (draft: Course) => void): Course[] {
  return courses.map((c) => {
    if (c.id !== id) return c
    const draft = structuredClone(c)
    fn(draft)
    return draft
  })
}

/** Each course's overall percentage by course id, for the course picker. `percents` is in course order. */
export function percentsById(courses: Course[], percents: (number | null)[]): Map<string, number | null> {
  return new Map(courses.map((c, i) => [c.id, percents[i] ?? null]))
}

/** File name for exporting a course as JSON ("CS-411-Artificial-Intelligence.json"). */
export function exportFileName(course: Course): string {
  return `${course.name.replace(/[^\w.-]+/g, '-').replace(/^-+|-+$/g, '') || 'course'}.json`
}

/** Human-readable "what do I need?" result for one target; `best` is the graded course's `best`. */
export function describeNeed(need: number | null | undefined, best: number | undefined): string {
  if (need === undefined) return '…'
  if (need === null) return best === undefined ? 'out of reach' : `out of reach (max ${formatPercent(best, 2)})`
  if (need === 0) return 'secured'
  return `${round(need * 100, 1)}% avg`
}
