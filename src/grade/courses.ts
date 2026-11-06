/**
 * Example courses, transcribed from docs/<course>/grades-11-5-2026.pdf and
 * configured from docs/<course>/syllabus.pdf. src/grade/courses.test.ts checks
 * that each one reproduces the numbers printed in its gradebook PDF.
 */
import type { Category, Course, Cutoff, Item } from './types'

let nextId = 0
export function newId(prefix = 'id'): string {
  nextId += 1
  return `${prefix}-${Date.now().toString(36)}-${nextId}`
}

type ItemOpts = Partial<Pick<Item, 'worth' | 'excluded' | 'neverDrop' | 'note'>>

function item(name: string, score: number | null, outOf: number, opts: ItemOpts = {}): Item {
  return { id: newId('item'), name, score, outOf, ...opts }
}

function category(
  name: string,
  weight: number,
  items: Item[],
  opts: Partial<Pick<Category, 'dropLowest' | 'dropHighest' | 'extraCredit'>> = {},
): Category {
  return { id: newId('cat'), name, weight, items, ...opts }
}

export const STANDARD_SCALE: Cutoff[] = [
  { letter: 'A', min: 90 },
  { letter: 'B', min: 80 },
  { letter: 'C', min: 70 },
  { letter: 'D', min: 60 },
  { letter: 'F', min: 0 },
]

// ---------------------------------------------------------------------------
// CS 401 – Computer Algorithms (Canvas, total points)
// Syllabus: grade is purely total points; 360/320/280/200 of 410 guarantee A/B/C/D.
// 410 = 14 check-ins × 3 + 7 homeworks × 24 + 2 exams × 100.
// ---------------------------------------------------------------------------
export function cs401(): Course {
  const total = 410
  const pct = (points: number) => (points / total) * 100
  return {
    id: newId('course'),
    name: 'CS 401 – Computer Algorithms',
    scheme: 'points',
    displayDecimals: 2,
    scale: [
      { letter: 'A', min: pct(360) },
      { letter: 'B', min: pct(320) },
      { letter: 'C', min: pct(280) },
      { letter: 'D', min: pct(200) },
      { letter: 'F', min: 0 },
    ],
    notes:
      'Total-points course. Letter cutoffs are the syllabus point thresholds (360/320/280/200) expressed as a share of the 410 planned points. Late Days is tracked on Canvas but does not count toward the grade. Homework 6, Homework 7 and the Final Exam are planned in the syllabus but not yet on Canvas.',
    categories: [
      category('Check-Ins', 0, [
        item('Check-In 1', 2, 3),
        item('Check-In 3', 2, 3),
        item('Check-In 4', 3, 3),
        item('Check-In 5', 3, 3),
        item('Check-In 6', 0, 3),
        item('Check-In 7', null, 3),
        item('Check-In 8', null, 3),
        item('Check-In 9', null, 3),
        item('Check-In 10', null, 3),
        item('Check-In 11', null, 3),
        item('Check-In 12', null, 3),
        item('Check-In 13', null, 3),
        item('Check-In 14', null, 3),
      ]),
      category('Homework', 0, [
        item('Homework 0 (Optional)', null, 0),
        item('Homework 1', 23, 24),
        item('Homework 2', 23.5, 24),
        item('Homework 3', 24, 24),
        item('Homework 4', null, 24),
        item('Homework 5', null, 24),
        item('Homework 6', null, 24, { note: 'From syllabus (not yet on Canvas)' }),
        item('Homework 7', null, 24, { note: 'From syllabus (not yet on Canvas)' }),
      ]),
      category('Assignments', 0, [
        item('Late Days', 4, 5, { excluded: true, note: 'Late days remaining; not graded' }),
      ]),
      category('Exams', 0, [
        item('Midterm Exam', 26, 100),
        item('Final Exam', null, 100, { note: 'From syllabus (not yet on Canvas)' }),
      ]),
    ],
  }
}

// ---------------------------------------------------------------------------
// CS 411 – Artificial Intelligence (Canvas, weighted groups)
// Syllabus: Quizzes 18% (drop 2), Labs 24% (drop 2), Exams 1–3 8% each,
// Final 10%, Projects 24%. Extra-credit group carries 0% on Canvas.
// ---------------------------------------------------------------------------
export function cs411(): Course {
  return {
    id: newId('course'),
    name: 'CS 411 – Artificial Intelligence',
    scheme: 'weighted',
    displayDecimals: 2,
    scale: STANDARD_SCALE,
    notes:
      'Exams 1–3 are 8% each, so Exam 03 (out of 100) is given the same worth as the 50-point exams. Canvas pools exam points instead, which will diverge from the syllabus once Exam 03 is graded. The cumulative make-up exam (replaces one or two of Exams 1–3) is not modelled; edit the scores directly to try it. No letter scale in the syllabus, so 90/80/70/60 is used.',
    categories: [
      category(
        'Quiz',
        18,
        [
          item('Quiz 01', 10, 10),
          item('Quiz 02', 10, 10),
          item('Quiz 03', 3, 10),
          item('Quiz 04', 8, 10),
          item('Quiz 05', 8, 10),
          item('Quiz 06', null, 10),
          item('Quiz 07', null, 10),
          item('Quiz 08', null, 10),
        ],
        { dropLowest: 2 },
      ),
      category(
        'Labs',
        24,
        [
          item('Lab01', 9, 10),
          item('Lab02', 10, 10),
          item('Lab03', 8, 10),
          item('Lab04', 10, 10),
          item('Lab05', 10, 10),
          item('Lab06', 10, 10),
          item('Lab07', 0, 10),
          item('Lab08', 7, 10),
          item('Lab09', null, 10),
          item('Lab10', null, 10),
        ],
        { dropLowest: 2 },
      ),
      category('Exams', 24, [
        item('Exam 01', 34, 50),
        item('Exam 02', 17, 50),
        item('Exam 03', null, 100, { worth: 50, note: 'Counts the same as Exams 01 and 02' }),
      ]),
      category('Final Exam', 10, [item('Final Exam', null, 100)]),
      category('Projects', 24, [
        item('Project 01', 97, 100),
        item('Project 02', 67.5, 100),
        item('Project 03', null, 100),
      ]),
      category('Extra Credit Assignments', 0, [
        item('In-class-Activity-01', 1, 1),
        item('In-class-Activity-02', 0.5, 0.5),
        item('In-class-Activity-03', 1.5, 1.5),
        item('In-class-Activity-04', 0, 1),
        item('In-class-Activity-05', 0.9, 1),
        item('In-class-Activity-06', 0, 1),
        item('In-class-Activity-07', 3, 3),
        item('In-class-Activity-08', null, 1.5),
        item('In-class-Activity-09', null, 1),
      ]),
      category('Make-up Project/Exam', 0, []),
    ],
  }
}

// ---------------------------------------------------------------------------
// CS 440 – Intro to Software Engineering (Blackboard, scaled points)
// Syllabus: each component is worth a fixed number of points; PPP =
// points accumulated to date / possible points to date. Raw 0–100 report scores
// are scaled to their point value (e.g. Description Report 85/100 → 8.5 of 10).
// ---------------------------------------------------------------------------
export function cs440(): Course {
  const later = { note: 'From syllabus (not yet graded)' }
  return {
    id: newId('course'),
    name: 'CS 440 – Intro to Software Engineering',
    scheme: 'points',
    displayDecimals: 1,
    scale: STANDARD_SCALE,
    notes:
      "Each Blackboard column is scaled to its syllabus point value. The 12 points for Weeklies, Reviews, Grammatical, OCL and Class Participation are all at full credit; the PDF pins down their total (67 possible) but not the exact split between Weeklies and Reviews, which doesn't change the grade. Attendance and the midterm/review sub-scores are informational only. Syllabus says letter breaks will be close to 90/80/70/60.",
    categories: [
      category('Individual Work', 0, [
        item('Attendance %', 100, 100, { excluded: true, note: 'Informational' }),
        item('Midterm MC', 32, 100, { excluded: true, note: 'Part of Midterm' }),
        item('Midterm Non MC', 19, 100, { excluded: true, note: 'Part of Midterm' }),
        item('Midterm', 51, 100, { worth: 15 }),
        item('Final Exam', null, 100, { worth: 20, ...later }),
        item('Brainstorming', 70, 100, { worth: 5 }),
        item('Rev1', 5, 5, { excluded: true, note: 'Part of Reviews' }),
        item('Rev2', 5, 5, { excluded: true, note: 'Part of Reviews' }),
        item('Rev3', 5, 5, { excluded: true, note: 'Part of Reviews' }),
        item('Reviews', 5, 5, { worth: 5, note: 'Presentation evaluations' }),
        item('Weeklies', 10, 10, { worth: 3, note: 'Weekly progress reports to date' }),
        item('Grammatical', 1, 1),
        item('OCL', 1, 1),
        item('Class Participation (2)', 2, 2),
      ]),
      category('Design Project', 0, [
        item('Description Report', 85, 100, { worth: 10 }),
        item('Description Presentation', 4.2, 5),
        item('Requirements Report', null, 100, { worth: 10, ...later }),
        item('Final Design Report', null, 100, { worth: 20, ...later }),
        item('Final Design Presentation', null, 5, later),
      ]),
      category('Implementation Project', 0, [
        item('First Scenario', 85, 100, { worth: 5 }),
        item('Second Scenario', 89, 100, { worth: 5 }),
        item('Demo 1', 80, 100, { worth: 10 }),
        item('Demo 2', null, 100, { worth: 10, ...later }),
        item('Final Oral Presentation & Demo', null, 5, later),
        item('Final Implementation Report', null, 100, { worth: 10, ...later }),
        item('CASE tool usage (Jira)', null, 5, later),
      ]),
    ],
  }
}

// ---------------------------------------------------------------------------
// Math 218 – Applied Linear Algebra
// ---------------------------------------------------------------------------
const MATH218_SCALE: Cutoff[] = [
  { letter: 'A', min: 87 },
  { letter: 'B', min: 75 },
  { letter: 'C', min: 60 },
  { letter: 'D', min: 45 },
  { letter: 'F', min: 0 },
]

function math218Mom(): Item[] {
  return [
    item('Homework 1 (Weeks 1-2)', 19.5, 20),
    item('Homework 2 (Weeks 3-4)', 19.8, 20),
    item('Homework 3 (Weeks 5-6)', 20, 20),
    item('Homework 4 (Weeks 7-8)', 18.6, 20),
    item('Homework 5 (Weeks 9-10)', 20, 20),
    item('Homework 6 (Weeks 11-12)', null, 20),
  ]
}

function math218Written(): Item[] {
  return [
    item('Written Homework #1', 18, 18),
    item('Written Homework #2', 16, 18),
    item('Written Homework #3', 18, 18),
    item('Written Homework #4', null, 18),
    item('Written Homework #5', null, 18),
    item('Written Homework #6', null, 18),
  ]
}

/**
 * Syllabus policy: Online HW 15%, Written HW 15% (lowest of each dropped),
 * Exam #1 20%, Exam #2 20%, Final 30%. The exams share one 70% category with
 * item worths 20/20/30, which is exactly how the instructor's posted
 * "Midsemester Score" (65.39) was computed — see courses.test.ts.
 */
export function math218(): Course {
  return {
    id: newId('course'),
    name: 'Math 218 – Applied Linear Algebra',
    scheme: 'weighted',
    displayDecimals: 2,
    scale: MATH218_SCALE,
    notes:
      'Weighted per the syllabus (Canvas itself is set to total points, which counts the 65.39/0 Midsemester Score as extra points and shows 118.28%). Exams form one 70% category with worths 20/20/30, so missing exams are projected at your exam average — the same method behind the posted Midsemester Score. Good attendance adds 5 points to the final exam score.',
    categories: [
      category('MOM Homework (online)', 15, math218Mom(), { dropLowest: 1 }),
      category('Written Homework', 15, math218Written(), { dropLowest: 1 }),
      category('Exams', 70, [
        item('Exam #1', 26, 50, { worth: 20 }),
        item('Exam #2', null, 50, { worth: 20 }),
        item('Final Exam', null, 100, { worth: 30, note: 'Points possible assumed; only the percentage matters' }),
      ]),
      category('Midsemester Grade', 0, [
        item('Midsemester Score', 65.39, 0, { excluded: true, note: 'Instructor-posted snapshot' }),
      ]),
    ],
  }
}

/** Math 218 exactly as its Canvas gradebook is configured (unweighted total points). */
export function math218Canvas(): Course {
  return {
    id: newId('course'),
    name: 'Math 218 – as Canvas computes it',
    scheme: 'points',
    displayDecimals: 2,
    scale: MATH218_SCALE,
    notes:
      'Mirror of the Canvas gradebook setup: no weights, no drops, and the 0-point Midsemester Score is added as extra points. This reproduces the 118.28% Canvas shows, which is not a real course grade; see the syllabus-weighted Math 218 example for that.',
    categories: [
      category('Written Homework group', 0, math218Written()),
      category('Assignments', 0, []),
      category('MOM Homework group', 0, math218Mom()),
      category('Midterm Exams', 0, [item('Exam #1', 26, 50)]),
      category('Midsemester Grade', 0, [item('Midsemester Score', 65.39, 0)]),
    ],
  }
}

export const EXAMPLES: { key: string; label: string; build: () => Course }[] = [
  { key: 'cs401', label: 'CS 401 – Computer Algorithms', build: cs401 },
  { key: 'cs411', label: 'CS 411 – Artificial Intelligence', build: cs411 },
  { key: 'cs440', label: 'CS 440 – Intro to Software Engineering', build: cs440 },
  { key: 'math218', label: 'Math 218 – Applied Linear Algebra', build: math218 },
  { key: 'math218-canvas', label: 'Math 218 – as Canvas computes it', build: math218Canvas },
]

export function blankCourse(): Course {
  return {
    id: newId('course'),
    name: 'New course',
    scheme: 'weighted',
    displayDecimals: 2,
    scale: STANDARD_SCALE,
    categories: [
      category('Homework', 40, [item('Homework 1', null, 10)]),
      category('Exams', 60, [item('Midterm', null, 100)]),
    ],
  }
}
