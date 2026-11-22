/**
 * Course: categories combined by total points or by weights, with a letter
 * scale and the "what do I need?" results. Mirrors backend/src/course/mod.rs
 * (camelCase JSON); keep the two in sync.
 */
import { withCategoryIds, type Category, type EngineCategory } from './category'
import { formatPercent, newId, round } from './format'

/** How a course combines its categories. */
export type Scheme = 'points' | 'weighted'

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

/** Average needed on the remaining work to reach a target (from `engine.evaluate`). */
export type Needed =
  | { kind: 'no-remaining' }
  | { kind: 'secured' }
  | { kind: 'impossible'; best: number | null }
  | { kind: 'needs'; fraction: number }

/** A course as the engine returns it: no ids (ids belong to the front-end). */
export type EngineCourse = Omit<Course, 'id' | 'categories'> & { categories: EngineCategory[] }

/**
 * Give a course from the engine, and everything in it, ids: the ids of
 * `saved` (the same course as stored, matched by position) where there is
 * one, new ids otherwise.
 */
export function withIds(course: EngineCourse, saved?: Course): Course {
  return {
    id: saved?.id ?? newId('course'),
    ...course,
    categories: course.categories.map((c, i) => withCategoryIds(c, saved?.categories[i])),
  }
}

/** A course's overall grade, for the course picker (from `engine.summaries`, in course order). */
export interface CourseSummary {
  /** Overall percentage, or null when nothing is graded. */
  percent: number | null
  letter: string | null
}

/** Edit the selected course: `fn` receives a draft copy to change in place. */
export type Mutate = (fn: (draft: Course) => void) => void

/** Decimal places to show this course's percentages with. */
export function decimals(course: Course): number {
  return course.displayDecimals ?? 2
}

/** Letters with a minimum above 0, highest first: the targets worth solving for. */
export function letterTargets(course: Course): Cutoff[] {
  return course.scale.filter((c) => c.min > 0).sort((a, b) => b.min - a.min)
}

/** Target percentages for `engine.evaluate`: each letter's minimum, then `custom` if set. */
export function targets(course: Course, custom: number | null): number[] {
  const letters = letterTargets(course).map((c) => c.min)
  return custom === null ? letters : [...letters, custom]
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

/** Each course's overall percentage by course id, for the course picker. `summaries` is in course order. */
export function percentsById(courses: Course[], summaries: CourseSummary[]): Map<string, number | null> {
  return new Map(courses.map((c, i) => [c.id, summaries[i]?.percent ?? null]))
}

/** File name for exporting a course as JSON ("CS-411-Artificial-Intelligence.json"). */
export function exportFileName(course: Course): string {
  return `${course.name.replace(/[^\w.-]+/g, '-').replace(/^-+|-+$/g, '') || 'course'}.json`
}

/** Human-readable "what do I need?" result. */
export function describeNeed(need: Needed | undefined): string {
  if (!need) return '…'
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
