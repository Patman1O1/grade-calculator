/**
 * Status: whether an item counts toward its category's grade, and if not,
 * why not. Mirrors backend/src/status/mod.rs; keep the two in sync.
 *
 * Only 'excluded' is the user's choice (the "Excl." checkbox). The engine
 * recomputes the other three from the item's score and its category's drop
 * rules on every evaluation, so the graded status from `engine.evaluate` is
 * the one to display.
 */
export type Status =
  /** Graded and part of the category's grade. */
  | 'counted'
  /** Graded, but removed by a drop-lowest / drop-highest rule. */
  | 'dropped'
  /** Marked as not counting toward the grade, graded or not. */
  | 'excluded'
  /** No score yet. */
  | 'ungraded'

/**
 * The status an item has before any drop rule (like `Status::for_score` in
 * Rust): 'excluded' stays 'excluded'; otherwise 'counted' with a score and
 * 'ungraded' without.
 */
export function forScore(status: Status, earnedPoints: number | null): Status {
  if (status === 'excluded') return 'excluded'
  return earnedPoints === null ? 'ungraded' : 'counted'
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
