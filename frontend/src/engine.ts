/**
 * Typed wrapper around the Rust grading engine (backend/src/wasm.rs), compiled
 * to WebAssembly by `npm run wasm`. Call `loadEngine()` once before using
 * anything else; after that every function is synchronous.
 */
import init, * as wasm from './wasm/engine.js'
import { withIds, type Course, type CourseSummary, type EngineCourse, type Needed } from './course'

// Engine results (mirror backend/src/api.rs).

export interface Evaluation {
  /**
   * The course with its computed fields filled in. It has no ids: its
   * categories and items are in the same order as the course that was graded.
   */
  course: EngineCourse
  /** Ungraded items that still have points available. */
  remaining: number
  /** One entry per requested target, in request order. */
  needs: { target: number; needed: Needed }[]
}

/** Fetch and instantiate the .wasm module. */
export async function loadEngine(): Promise<void> {
  await init()
}

/**
 * Summaries already worked out, by course object. Courses are never edited in
 * place (`updateCourse` replaces the edited one with a copy), so a course
 * object's summary never goes stale, and on each edit only the edited course
 * needs grading again.
 */
const summaryCache = new WeakMap<Course, CourseSummary>()

/** Each course's summary, asking the engine only about courses not seen before. */
function summaries(courses: Course[]): CourseSummary[] {
  const missing = courses.filter((c) => !summaryCache.has(c))
  if (missing.length > 0) {
    const results: CourseSummary[] = wasm.summaries(missing)
    missing.forEach((c, i) => summaryCache.set(c, results[i]))
  }
  return courses.map((c) => summaryCache.get(c)!)
}

/**
 * The engine works on plain course data and knows nothing about ids, so
 * every course coming back from it gets its ids here.
 */
export const engine = {
  /** A new course to start from. */
  blankCourse: (): Course => withIds(wasm.blankCourse()),
  /** Grade a course and solve "what do I need?" for each target percentage. */
  evaluate: (course: Course, targets: number[]): Evaluation => wasm.evaluate(course, targets),
  /** Each course's overall grade, in the same order as `courses`. Cached per course object. */
  summaries,
  /** Validate parsed JSON (one course or a list) and get its courses with new ids. Throws if it has no course. */
  importCourses: (data: unknown): Course[] => (wasm.importCourses(data) as EngineCourse[]).map(withIds),
}
