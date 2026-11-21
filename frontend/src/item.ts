/**
 * Item: one graded thing (a quiz, an exam, a lab), as `earned` out of
 * `possible` points. Mirrors backend/src/item/mod.rs (camelCase JSON); keep
 * the two in sync.
 *
 * An item's points also set how much it counts in its category: a 10-point
 * quiz counts a tenth as much as a 100-point exam. `status` is computed by
 * the engine (`engine.evaluate`); it is absent on the items the browser
 * stores and exports, and the engine ignores it on input.
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

/** `earned / possible` as a percentage; null if ungraded or extra credit (0 possible). */
export function percentOf(item: Item): number | null {
  if (item.earned === null || item.possible <= 0) return null
  return round((item.earned / item.possible) * 100, 6)
}

/** An item as the engine returns it: no id (ids belong to the front-end). */
export type EngineItem = Omit<Item, 'id'>

/** Give an item from the engine an id: `id` if given (e.g. its saved id), otherwise a new one. */
export function withItemId(item: EngineItem, id: string = newId('item')): Item {
  return { id, ...item }
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
 * the first item in a category is out of 100.
 */
export function newItem(name: string, previous?: Item): Item {
  return { id: newId('item'), name, earned: null, possible: previous?.possible ?? 100 }
}
