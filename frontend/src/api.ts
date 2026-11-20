/**
 * Client for the Rust grade-calculator API (backend/src/api.rs).
 *
 * By default requests go to the same origin (`/api/...`), which is how the Rust
 * server serves the built app, and how the Vite dev server proxies it. Set
 * VITE_API_URL at build time to call an API hosted elsewhere.
 */
import type { Course, CourseSummary, Evaluation, ExampleInfo } from './types'

const BASE = (import.meta.env.VITE_API_URL ?? '').replace(/\/+$/, '')

export class ApiError extends Error {}

async function request<T>(path: string, init?: RequestInit & { json?: unknown }): Promise<T> {
  const { json, ...rest } = init ?? {}
  let res: Response
  try {
    res = await fetch(`${BASE}/api${path}`, {
      ...rest,
      ...(json !== undefined
        ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(json) }
        : {}),
    })
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') throw e
    throw new ApiError("Can't reach the grade server.")
  }
  if (!res.ok) {
    const body = (await res.json().catch(() => null)) as { error?: string } | null
    throw new ApiError(body?.error ?? `Server error (${res.status})`)
  }
  return (await res.json()) as T
}

export const api = {
  examples: () => request<ExampleInfo[]>('/examples'),
  example: (key: string) => request<Course>(`/examples/${encodeURIComponent(key)}`),
  blank: () => request<Course>('/courses/blank'),
  evaluate: (course: Course, targets: number[], signal?: AbortSignal) =>
    request<Evaluation>('/evaluate', { json: { course, targets }, signal }),
  summaries: (courses: Course[], signal?: AbortSignal) =>
    request<CourseSummary[]>('/summaries', { json: { courses }, signal }),
  /** Validate exported JSON (one course or a list) and get copies with fresh ids. */
  import: (data: unknown) => request<{ courses: Course[] }>('/import', { json: data }).then((r) => r.courses),
}
