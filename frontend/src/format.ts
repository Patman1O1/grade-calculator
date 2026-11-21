/** Display helpers. All grade math happens in the Rust engine (WebAssembly). */

/** Round half away from zero, immune to binary noise like 52.549999999. */
export function round(x: number, decimals: number): number {
  const f = 10 ** decimals
  const clean = Number(x.toFixed(10))
  return (Math.sign(clean) * Math.round(Math.abs(clean) * f)) / f
}

/** Format like Canvas: up to `decimals` places, trailing zeros trimmed. */
export function formatPercent(x: number | null | undefined, decimals = 2): string {
  if (x === null || x === undefined) return 'N/A'
  return `${round(x, decimals)}%`
}

export function formatNumber(x: number, decimals = 2): string {
  return round(x, decimals).toFixed(decimals)
}

let nextId = 0
/**
 * A fresh id. Ids belong to the front-end (React keys, the selected course,
 * which course an edit applies to); the engine has none. The time keeps ids
 * unique across page loads, so they never collide with saved ones.
 */
export function newId(prefix = 'id'): string {
  nextId += 1
  return `${prefix}-${Date.now().toString(36)}-${nextId}`
}
