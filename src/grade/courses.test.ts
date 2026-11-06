/**
 * Every expected value below is copied from docs/<course>/grades-11-5-2026.pdf
 * (and rounded the same way the gradebook displays it).
 */
import { describe, expect, it } from 'vitest'
import { computeCourse, round } from './engine'
import { cs401, cs411, cs440, math218, math218Canvas } from './courses'
import type { Course, CourseResult } from './types'

function category(result: CourseResult, name: string) {
  const c = result.categories.find((x) => x.category.name === name)
  if (!c) throw new Error(`no category ${name}`)
  return {
    percent: c.percent === null ? null : round(c.percent, 2),
    earned: round(c.earned, 2),
    possible: round(c.possible, 2),
  }
}

describe('CS 401 (Canvas total points)', () => {
  const r = computeCourse(cs401())
  it('Total 56.95% (106.50 / 187.00)', () => {
    expect(round(r.percent!, 2)).toBe(56.95)
    expect(round(r.earned, 2)).toBe(106.5)
    expect(round(r.possible, 2)).toBe(187)
  })
  it('group rows', () => {
    expect(category(r, 'Homework')).toEqual({ percent: 97.92, earned: 70.5, possible: 72 })
    expect(category(r, 'Check-Ins')).toEqual({ percent: 66.67, earned: 10, possible: 15 })
    expect(category(r, 'Exams')).toEqual({ percent: 26, earned: 26, possible: 100 })
    // Late Days does not count: Canvas shows "N/A 0.00 / 0.00".
    expect(category(r, 'Assignments')).toEqual({ percent: null, earned: 0, possible: 0 })
  })
})

describe('CS 411 (Canvas weighted groups, drop lowest 2)', () => {
  const r = computeCourse(cs411())
  it('Total 79.53%', () => {
    expect(round(r.percent!, 2)).toBe(79.53)
  })
  it('group rows', () => {
    expect(category(r, 'Extra Credit Assignments')).toEqual({ percent: 76.67, earned: 6.9, possible: 9 })
    expect(category(r, 'Quiz')).toEqual({ percent: 93.33, earned: 28, possible: 30 })
    expect(category(r, 'Projects')).toEqual({ percent: 82.25, earned: 164.5, possible: 200 })
    expect(category(r, 'Labs')).toEqual({ percent: 95, earned: 57, possible: 60 })
    expect(category(r, 'Exams')).toEqual({ percent: 51, earned: 51, possible: 100 })
    expect(category(r, 'Final Exam').percent).toBeNull()
    expect(category(r, 'Make-up Project/Exam').percent).toBeNull()
  })
  it('drops the two lowest quizzes and labs', () => {
    const dropped = r.categories.flatMap((c) => c.items.filter((i) => i.dropped).map((i) => i.item.name))
    // Quiz 04 and Quiz 05 are both 8/10, so dropping either one is equivalent.
    expect(dropped.filter((d) => d !== 'Quiz 04' && d !== 'Quiz 05').sort()).toEqual(['Lab07', 'Lab08', 'Quiz 03'])
    expect(dropped.filter((d) => d === 'Quiz 04' || d === 'Quiz 05')).toHaveLength(1)
  })
})

describe('CS 440 (Blackboard scaled points)', () => {
  const r = computeCourse(cs440())
  it('Total (out of 67) 52.6', () => {
    expect(round(r.possible, 1)).toBe(67)
    expect(round(r.earned, 1)).toBe(52.6)
  })
  it('PPP (Based on % of 67) 78.4', () => {
    expect(round(r.percent!, 1)).toBe(78.4)
  })
})

describe('Math 218', () => {
  it('Canvas setup: Total 118.28% (241.29 / 204.00)', () => {
    const r = computeCourse(math218Canvas())
    expect(round(r.percent!, 2)).toBe(118.28)
    expect(round(r.earned, 2)).toBe(241.29)
    expect(round(r.possible, 2)).toBe(204)
    expect(category(r, 'Written Homework group')).toEqual({ percent: 96.3, earned: 52, possible: 54 })
    expect(category(r, 'MOM Homework group')).toEqual({ percent: 97.9, earned: 97.9, possible: 100 })
    expect(category(r, 'Midterm Exams')).toEqual({ percent: 52, earned: 26, possible: 50 })
    expect(category(r, 'Assignments').percent).toBeNull()
    // "Midsemester Score 65.39 / 0" → group row "N/A 65.39 / 0.00"
    expect(category(r, 'Midsemester Grade')).toEqual({ percent: null, earned: 65.39, possible: 0 })
  })

  it('syllabus weighting reproduces the posted Midsemester Score of 65.39', () => {
    // At mid-semester (after Exam #1 on Oct 5) the graded work was MOM HW 1–3
    // and Written HW 1–2, and no drops had been applied yet.
    const course: Course = math218()
    const gradedLater = new Set(['Homework 4 (Weeks 7-8)', 'Homework 5 (Weeks 9-10)', 'Written Homework #3'])
    for (const c of course.categories) {
      c.dropLowest = 0
      for (const i of c.items) if (gradedLater.has(i.name)) i.score = null
    }
    expect(round(computeCourse(course).percent!, 2)).toBe(65.39)
  })

  it('syllabus weighting today', () => {
    const r = computeCourse(math218())
    // MOM: drop 18.6 → 79.3/80; Written: drop 16 → 36/36; Exams: 52%.
    expect(category(r, 'MOM Homework (online)').percent).toBe(99.13)
    expect(category(r, 'Written Homework').percent).toBe(100)
    expect(category(r, 'Exams').percent).toBe(52)
    expect(round(r.percent!, 2)).toBe(round(15 * 0.99125 + 15 * 1 + 70 * 0.52, 2))
    expect(r.letter).toBe('C')
  })
})
