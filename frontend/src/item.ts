/**
 * Item: one graded thing (a quiz, an exam, a lab), as `earned` out of
 * `possible` points. Mirrors backend/src/item/mod.rs (camelCase JSON); keep
 * the two in sync.
 *
 * An item's points also set how much it counts in its category: a 10-point
 * quiz counts a tenth as much as a 100-point exam. Its `status` is stored;
 * only 'excluded' is read by the engine, which recomputes the rest (see
 * status.ts).
 */
import { newId, round } from './format'
import { forScore, type Status } from './status'

export interface Item {
  id: string
  name: string
  /** Points earned, or null when the item has not been graded yet. */
  earned: number | null
  /** Points possible. 0 makes the item pure extra credit. */
  possible: number
  /** Never removed by drop-lowest / drop-highest rules. */
  neverDrop?: boolean
  /** Free-form note shown in the UI (e.g. due date). */
  note?: string
  /**
   * Whether the item counts toward its category's grade, and if not, why not.
   * 'excluded' is the user's choice; the engine recomputes the rest.
   */
  status: Status
}

/** `earned / possible` as a percentage; null if ungraded or extra credit (0 possible). */
export function percentOf(item: Item): number | null {
  if (item.earned === null || item.possible <= 0) return null
  return round((item.earned / item.possible) * 100, 6)
}

/** An item as the engine returns it: no id (ids belong to the front-end). */
export type EngineItem = Omit<Item, 'id'>

/** Give an item from the engine a new id. */
export function withItemId(item: EngineItem): Item {
  return { id: newId('item'), ...item }
}

/** Set an item's score, keeping its stored status in step ('counted' / 'ungraded'). */
export function setEarned(item: Item, earned: number | null): void {
  item.earned = earned
  item.status = forScore(item.status, earned)
}

/** Mark an item as excluded from the grade, or include it again. */
export function setExcluded(item: Item, excluded: boolean): void {
  item.status = excluded ? 'excluded' : forScore('ungraded', item.earned)
}

/**
 * A new, ungraded item named `name`. It copies `possible` from `previous`
 * (the category's last item) so a run of similar items is quick to enter;
 * the first item in a category is out of 100.
 */
export function newItem(name: string, previous?: Item): Item {
  return { id: newId('item'), name, earned: null, possible: previous?.possible ?? 100, status: 'ungraded' }
}
