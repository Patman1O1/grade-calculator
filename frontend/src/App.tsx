import { useEffect, useMemo, useRef, useState } from 'react'
import { api } from './api'
import { formatPercent } from './format'
import { useRemote } from './hooks'
import type { Course, ExampleInfo } from './types'
import { loadState, saveState, type SavedState } from './storage'
import { CourseEditor, type Mutate } from './components/CourseEditor'
import { Summary } from './components/Summary'

/** Letter targets (highest first) followed by the user's custom target, if any. */
function targetsFor(course: Course, custom: number | null): number[] {
  const letters = course.scale
    .map((c) => c.min)
    .filter((m) => m > 0)
    .sort((a, b) => b - a)
  return custom === null ? letters : [...letters, custom]
}

/** Editor + summary for one course, evaluated by the server as it changes. */
function CourseView({ course, mutate }: { course: Course; mutate: Mutate }) {
  const [custom, setCustom] = useState<number | null>(null)
  const targets = targetsFor(course, custom)
  const evaluation = useRemote(JSON.stringify([course, targets]), (signal) =>
    api.evaluate(course, targets, signal),
  )
  const data = evaluation.data

  return (
    <>
      {evaluation.error && <ServerBanner message={evaluation.error} />}
      <main className="layout">
        <CourseEditor course={course} result={data?.result ?? null} mutate={mutate} />
        <Summary course={course} evaluation={data} custom={custom} onCustom={setCustom} />
      </main>
    </>
  )
}

function ServerBanner({ message }: { message: string }) {
  return (
    <p className="banner" role="alert">
      {message} Your courses are saved in this browser; the numbers will update once the server is reachable.
    </p>
  )
}

export default function App() {
  const [state, setState] = useState<SavedState | null>(loadState)
  const [examples, setExamples] = useState<ExampleInfo[]>([])
  const [startupError, setStartupError] = useState<string | null>(null)
  const fileInput = useRef<HTMLInputElement>(null)

  // Load the example list; on first visit, seed the app with every example.
  useEffect(() => {
    let cancelled = false
    api
      .examples()
      .then(async (list) => {
        if (cancelled) return
        setExamples(list)
        if (loadState()?.courses.length) return
        const courses = await Promise.all(list.map((e) => api.example(e.key)))
        if (!cancelled) setState({ courses, selectedId: courses[0]?.id ?? null })
      })
      .catch((e: unknown) => {
        if (cancelled) return
        setStartupError(e instanceof Error ? e.message : String(e))
        setState((s) => s ?? { courses: [], selectedId: null })
      })
    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    if (state) saveState(state)
  }, [state])

  const courses = useMemo(() => state?.courses ?? [], [state])
  const course = courses.find((c) => c.id === state?.selectedId) ?? courses[0] ?? null
  const summaries = useRemote(
    courses.length ? JSON.stringify(courses) : null,
    (signal) => api.summaries(courses, signal),
    250,
  )
  const percentOf = (id: string) => summaries.data?.find((s) => s.id === id)?.percent

  const run = (task: () => Promise<void>) =>
    task().catch((e: unknown) => window.alert(e instanceof Error ? e.message : String(e)))

  const select = (id: string) => setState((s) => s && { ...s, selectedId: id })
  const addCourses = (added: Course[]) => {
    if (added.length === 0) return
    setState((s) => ({ courses: [...(s?.courses ?? []), ...added], selectedId: added[0].id }))
  }

  const mutate: Mutate = (fn) =>
    setState(
      (s) =>
        s && {
          ...s,
          courses: s.courses.map((c) => {
            if (c.id !== course?.id) return c
            const draft = structuredClone(c)
            fn(draft)
            return draft
          }),
        },
    )

  const removeCourse = () => {
    if (!course || !window.confirm(`Delete “${course.name}”?`)) return
    setState((s) => {
      const rest = (s?.courses ?? []).filter((c) => c.id !== course.id)
      return { courses: rest, selectedId: rest[0]?.id ?? null }
    })
  }

  const newCourse = () => run(async () => addCourses([await api.blank()]))

  const duplicate = () =>
    run(async () => {
      if (!course) return
      const [copy] = await api.import(course)
      addCourses([{ ...copy, name: `${course.name} (copy)` }])
    })

  const exportCourse = () => {
    if (!course) return
    const blob = new Blob([JSON.stringify(course, null, 2)], { type: 'application/json' })
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = `${course.name.replace(/[^\w.-]+/g, '-').replace(/^-+|-+$/g, '') || 'course'}.json`
    a.click()
    URL.revokeObjectURL(a.href)
  }

  const importCourse = (file: File) =>
    run(async () => {
      let data: unknown
      try {
        data = JSON.parse(await file.text())
      } catch {
        throw new Error('That file is not valid JSON.')
      }
      addCourses(await api.import(data))
    })

  if (!state) {
    return (
      <div className="app">
        <main className="empty-state">
          <p>Loading…</p>
        </main>
      </div>
    )
  }

  return (
    <div className="app">
      <header className="topbar">
        <h1>Grade Calculator</h1>
        <div className="course-picker">
          <select
            aria-label="Course"
            value={course?.id ?? ''}
            onChange={(e) => select(e.target.value)}
            disabled={courses.length === 0}
          >
            {courses.map((c) => {
              const p = percentOf(c.id)
              return (
                <option key={c.id} value={c.id}>
                  {c.name}
                  {p === undefined ? '' : ` — ${formatPercent(p, c.displayDecimals ?? 2)}`}
                </option>
              )
            })}
          </select>
          <button type="button" onClick={newCourse}>
            New
          </button>
          <select
            aria-label="Load example"
            value=""
            disabled={examples.length === 0}
            onChange={(e) => {
              const key = e.target.value
              if (key) run(async () => addCourses([await api.example(key)]))
            }}
          >
            <option value="">Add example…</option>
            {examples.map((e) => (
              <option key={e.key} value={e.key}>
                {e.label}
              </option>
            ))}
          </select>
          {course && (
            <button type="button" onClick={duplicate}>
              Duplicate
            </button>
          )}
          <button type="button" onClick={exportCourse} disabled={!course}>
            Export
          </button>
          <button type="button" onClick={() => fileInput.current?.click()}>
            Import
          </button>
          <input
            ref={fileInput}
            type="file"
            accept="application/json,.json"
            hidden
            onChange={(e) => {
              const f = e.target.files?.[0]
              if (f) void importCourse(f)
              e.target.value = ''
            }}
          />
          <button type="button" className="danger" onClick={removeCourse} disabled={!course}>
            Delete
          </button>
        </div>
      </header>

      {course ? (
        <CourseView key={course.id} course={course} mutate={mutate} />
      ) : (
        <>
          {startupError && <ServerBanner message={startupError} />}
          <main className="empty-state">
            <p>No courses yet.</p>
            <button type="button" onClick={newCourse}>
              Create a course
            </button>
          </main>
        </>
      )}
    </div>
  )
}
