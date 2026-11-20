import type { Course } from './types'

const KEY = 'grade-calculator:v1'

export interface SavedState {
  courses: Course[]
  selectedId: string | null
}

/** Courses live in the browser; the server only calculates and holds no user data. */
export function loadState(): SavedState | null {
  try {
    const raw = localStorage.getItem(KEY)
    if (!raw) return null
    const data = JSON.parse(raw) as SavedState
    if (!Array.isArray(data.courses)) return null
    return data
  } catch {
    return null
  }
}

export function saveState(state: SavedState): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(state))
  } catch {
    // Storage can be unavailable (private mode, quota); the app still works in memory.
  }
}
