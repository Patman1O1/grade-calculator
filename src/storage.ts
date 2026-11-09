import type { Category, Course, Item } from './grade/types'

const KEY = 'grade-calculator:v1'

export interface SavedState {
  courses: Course[]
  selectedId: string | null
}

const isNum = (x: unknown): x is number => typeof x === 'number' && Number.isFinite(x)

function isItem(x: unknown): x is Item {
  const i = x as Item
  return !!i && typeof i.name === 'string' && (i.score === null || isNum(i.score)) && isNum(i.outOf)
}

function isCategory(x: unknown): x is Category {
  const c = x as Category
  return !!c && typeof c.name === 'string' && isNum(c.weight) && Array.isArray(c.items) && c.items.every(isItem)
}

export function isCourse(x: unknown): x is Course {
  const c = x as Course
  return (
    !!c &&
    typeof c.name === 'string' &&
    (c.scheme === 'points' || c.scheme === 'weighted') &&
    Array.isArray(c.categories) &&
    c.categories.every(isCategory) &&
    Array.isArray(c.scale)
  )
}

export function loadState(): SavedState | null {
  try {
    const raw = localStorage.getItem(KEY)
    if (!raw) return null
    const data = JSON.parse(raw) as SavedState
    if (!Array.isArray(data.courses) || !data.courses.every(isCourse)) return null
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
