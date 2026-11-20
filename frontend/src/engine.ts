/**
 * Typed wrapper around the Rust grading engine (engine/src/wasm.rs), compiled
 * to WebAssembly by `npm run wasm`. Call `loadEngine()` once before using
 * anything else; after that every function is synchronous.
 */
import init, * as wasm from './wasm/engine.js'
import type { Course, CourseSummary, Evaluation, ExampleInfo } from './types'

/** Fetch and instantiate the .wasm module. */
export async function loadEngine(): Promise<void> {
  await init()
}

export const engine = {
  examples: (): ExampleInfo[] => wasm.examples(),
  /** A fresh copy of an example course. Throws for an unknown key. */
  example: (key: string): Course => wasm.example(key),
  blankCourse: (): Course => wasm.blankCourse(),
  /** Grade a course and solve "what do I need?" for each target percentage. */
  evaluate: (course: Course, targets: number[]): Evaluation => wasm.evaluate(course, targets),
  summaries: (courses: Course[]): CourseSummary[] => wasm.summaries(courses),
  /** Validate parsed JSON (one course or a list) and get copies with fresh ids. Throws if it has no course. */
  importCourses: (data: unknown): Course[] => wasm.importCourses(data),
}
