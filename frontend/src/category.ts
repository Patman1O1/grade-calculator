/**
 * Category: a group of items with optional drop rules and a weight. Mirrors
 * backend/src/category/mod.rs (camelCase JSON); keep the two in sync.
 */
import { newId } from './format'
import { newItem, type Item } from './item'

export interface Category {
  id: string
  name: string
  /** Relative weight; only used by the 'weighted' scheme. */
  weight: number
  dropLowest?: number
  dropHighest?: number
  /** Weighted scheme only: adds to the grade without adding to the denominator. */
  extraCredit?: boolean
  items: Item[]

  /** Computed: sum over items whose status is 'counted'. */
  earned?: number
  /** Computed. */
  possible?: number
  /** Computed: earned / possible as a percentage; absent when nothing is graded. */
  percent?: number
}

/** A new, empty category. */
export function newCategory(): Category {
  return { id: newId('cat'), name: 'New category', weight: 0, items: [] }
}

/** Add an ungraded item named after the category ("Quiz 9"), like the one before it. */
export function addItem(category: Category): void {
  const { items } = category
  items.push(newItem(`${category.name} ${items.length + 1}`, items[items.length - 1]))
}

/** Whether the category drops any items (so the "Keep" column matters). */
export function hasDropRules(category: Category): boolean {
  return (category.dropLowest ?? 0) > 0 || (category.dropHighest ?? 0) > 0
}

/**
 * The graded copies of a category's items, by id. Looking up by id (not
 * position) keeps results on the right row even if the graded copy is
 * from a slightly older version of the category.
 */
export function gradedItems(graded: Category | undefined): Map<string, Item> {
  return new Map(graded?.items.map((i) => [i.id, i]))
}
