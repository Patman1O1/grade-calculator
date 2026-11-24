/**
 * Scale: a course's letter grade cutoffs. Mirrors backend/src/scale/mod.rs;
 * keep the two in sync.
 */

/** The minimum percentage (0–100) for each letter grade. */
export interface Scale {
  a: number
  b: number
  c: number
  d: number
  f: number
}

/** The scale's fields, from A to F. */
export type Grade = keyof Scale

export const GRADES: readonly Grade[] = ['a', 'b', 'c', 'd', 'f']

/** One letter of a scale with its cutoff. */
export interface Cutoff {
  grade: Grade
  /** Display letter, e.g. 'A'. */
  letter: string
  min: number
}

/** Each letter with its cutoff, from A to F. */
export function cutoffs(scale: Scale): Cutoff[] {
  return GRADES.map((grade) => ({ grade, letter: grade.toUpperCase(), min: scale[grade] }))
}
