/**
 * Item: one graded thing (a quiz, an exam, a lab). Mirrors
 * backend/src/item/mod.rs (camelCase JSON); keep the two in sync.
 *
 * Its raw result is `score / outOf`. `worth` says how much the item counts
 * inside its category (defaults to `outOf`, i.e. plain points). Fields marked
 * "computed" are filled in by the engine (`engine.evaluate`); they are absent
 * on the items the browser stores and exports, and the engine ignores them on
 * input.
 */
import { newId } from './format'

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
  /** Raw score, or null when the item has not been graded yet. */
  score: number | null
  /** Raw points possible. 0 makes the item pure extra credit. */
  outOf: number
  /** How much the item counts inside its category. Defaults to `outOf`. */
  worth?: number
  /** Not counted toward the grade at all (Canvas "does not count toward final grade"). */
  excluded?: boolean
  /** Never removed by drop-lowest / drop-highest rules. */
  neverDrop?: boolean
  /** Free-form note shown in the UI (e.g. due date). */
  note?: string

  /** Computed: whether the item counts toward its category's grade, and if not, why not. */
  status?: Status
  /** Computed: contribution to the category, score / outOf * worth (raw score for extra credit). */
  earned?: number
  /** Computed. */
  possible?: number
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
 * A new, ungraded item named `name`. It copies `outOf` and `worth` from
 * `previous` (the category's last item) so a run of similar items is quick
 * to enter.
 */
export function newItem(name: string, previous?: Item): Item {
  return {
    id: newId('item'),
    name,
    score: null,
    outOf: previous?.outOf ?? 10,
    ...(previous?.worth !== undefined ? { worth: previous.worth } : {}),
  }
}

/** Set an item's worth; null clears it so it falls back to `outOf`. */
export function setWorth(item: Item, worth: number | null): void {
  if (worth === null) delete item.worth
  else item.worth = worth
}
