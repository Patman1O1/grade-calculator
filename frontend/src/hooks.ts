import { useEffect, useState } from 'react'

export interface Remote<T> {
  /** Latest successful response. Kept while a newer request is in flight so the UI doesn't flicker. */
  data: T | null
  error: string | null
  loading: boolean
}

/**
 * Re-run `fetcher` whenever `key` changes (after `delay` ms of quiet), aborting
 * the previous request. Pass `key = null` to skip.
 */
export function useRemote<T>(
  key: string | null,
  fetcher: (signal: AbortSignal) => Promise<T>,
  delay = 60,
): Remote<T> {
  const [state, setState] = useState<Remote<T>>({ data: null, error: null, loading: key !== null })

  useEffect(() => {
    if (key === null) return
    const ctrl = new AbortController()
    const timer = window.setTimeout(() => {
      setState((s) => ({ ...s, loading: true }))
      fetcher(ctrl.signal).then(
        (data) => setState({ data, error: null, loading: false }),
        (e: unknown) => {
          if (ctrl.signal.aborted) return
          setState((s) => ({ ...s, error: e instanceof Error ? e.message : String(e), loading: false }))
        },
      )
    }, delay)
    return () => {
      window.clearTimeout(timer)
      ctrl.abort()
    }
    // `fetcher` is a fresh closure every render; `key` captures everything it depends on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, delay])

  return state
}
