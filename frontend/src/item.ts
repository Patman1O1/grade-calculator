/**
 * Item: one graded thing (a quiz, an exam, a lab), as `earned` out of
 * `possible` points. Mirrors backend/src/item/mod.rs (camelCase JSON); keep
 * the two in sync.
 *
 * An item's points also set how much it counts in its category: a 10-point
 * quiz counts a tenth as much as a 100-point exam. A score entered as a
 * percentage is stored as `percent / 100`. `status` is computed by the engine
 * (`engine.evaluate`); it is absent on the items the browser stores and
 * exports, and the engine ignores it on input.
 */
import { newId, round } from './format'

/** Whether an item counts toward its category's grade, and if not, why not. */
export type Status =
  /** Graded and part of the category's grade. */
  | 'counted'
  /** Graded, but removed by a drop-lowest / drop-highest rule. */
  | 'dropped'
  /** Marked as not counting toward the grade. */
  | 'excluded'
  /** No score yet. */
  | 'ungraded'

export interface Item {
  id: string
  name: string
  /** Points earned, or null when the item has not been graded yet. */
  earned: number | null
  /** Points possible. 0 makes the item pure extra credit. */
  possible: number
  /** Not counted toward the grade at all (Canvas "does not count toward final grade"). */
  excluded?: boolean
  /** Never removed by drop-lowest / drop-highest rules. */
  neverDrop?: boolean
  /** Free-form note shown in the UI (e.g. due date). */
  note?: string

  /** Computed: whether the item counts toward its category's grade, and if not, why not. */
  status?: Status
}

/** How the grade is entered: one percentage (stored as x/100) or points earned / possible. */
export type GradeMode = 'percent' | 'points'

/** The input an item opens with: a percentage if it is out of 100, points otherwise. */
export function defaultGradeMode(item: Item): GradeMode {
  return item.possible === 100 ? 'percent' : 'points'
}

/** `earned / possible` as a percentage; null if ungraded or extra credit (0 possible). */
export function percentOf(item: Item): number | null {
  if (item.earned === null || item.possible <= 0) return null
  // Out of 100, the earned points are the percentage; avoid float noise like 85.00000000000001.
  return item.possible === 100 ? item.earned : round((item.earned / item.possible) * 100, 6)
}

/** Grade entered as a percentage: stored as `percent` out of 100 points. */
export function setPercent(item: Item, percent: number | null): void {
  item.earned = percent
  item.possible = 100
}

/** Switch an item to percentage entry, keeping its score: 8/10 becomes 80/100. */
export function toPercent(item: Item): void {
  setPercent(item, item.possible > 0 ? percentOf(item) : item.earned)
}

/** Table-row class for each status (see index.css). */
const ROW_CLASS: Record<Status, string> = {
  counted: '',
  dropped: 'dropped',
  excluded: 'excluded',
  ungraded: 'pending',
}

/** Table-row class for an item's status; '' when not graded yet. */
export function rowClass(status: Status | undefined): string {
  return status ? ROW_CLASS[status] : ''
}

/**
 * A new, ungraded item named `name`. It copies `possible` from `previous`
 * (the category's last item) so a run of similar items is quick to enter;
 * the first item in a category is out of 100, i.e. a percentage.
 */
export function newItem(name: string, previous?: Item): Item {
  return { id: newId('item'), name, earned: null, possible: previous?.possible ?? 100 }
}
