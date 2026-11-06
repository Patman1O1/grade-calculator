import { useEffect, useMemo, useRef, useState } from 'react'
import { computeCourse, formatPercent } from './grade/engine'
import { EXAMPLES, blankCourse, newId } from './grade/courses'
import type { Course } from './grade/types'
import { isCourse, loadState, saveState } from './storage'
import { CourseEditor, type Mutate } from './components/CourseEditor'
import { Summary } from './components/Summary'

function initialState() {
  const saved = loadState()
  if (saved && saved.courses.length > 0) return saved
  const courses = EXAMPLES.map((e) => e.build())
  return { courses, selectedId: courses[0].id }
}

/** Give a course (and everything in it) fresh ids so copies never collide. */
function reId(course: Course): Course {
  return {
    ...course,
    id: newId('course'),
    categories: course.categories.map((c) => ({
      ...c,
      id: newId('cat'),
      items: c.items.map((i) => ({ ...i, id: newId('item') })),
    })),
  }
}

export default function App() {
  const [state, setState] = useState(initialState)
  const fileInput = useRef<HTMLInputElement>(null)
  const { courses } = state
  const course = courses.find((c) => c.id === state.selectedId) ?? courses[0] ?? null
  const result = useMemo(() => (course ? computeCourse(course) : null), [course])

  useEffect(() => saveState(state), [state])

  const select = (id: string) => setState((s) => ({ ...s, selectedId: id }))
  const addCourse = (c: Course) => setState((s) => ({ courses: [...s.courses, c], selectedId: c.id }))

  const mutate: Mutate = (fn) =>
    setState((s) => ({
      ...s,
      courses: s.courses.map((c) => {
        if (c.id !== course?.id) return c
        const draft = structuredClone(c)
        fn(draft)
        return draft
      }),
    }))

  const removeCourse = () => {
    if (!course || !window.confirm(`Delete “${course.name}”?`)) return
    setState((s) => {
      const rest = s.courses.filter((c) => c.id !== course.id)
      return { courses: rest, selectedId: rest[0]?.id ?? null }
    })
  }

  const exportCourse = () => {
    if (!course) return
    const blob = new Blob([JSON.stringify(course, null, 2)], { type: 'application/json' })
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = `${course.name.replace(/[^\w.-]+/g, '-').replace(/^-+|-+$/g, '') || 'course'}.json`
    a.click()
    URL.revokeObjectURL(a.href)
  }

  const importCourse = async (file: File) => {
    try {
      const data: unknown = JSON.parse(await file.text())
      const list = Array.isArray(data) ? data : [data]
      const valid = list.filter(isCourse).map(reId)
      if (valid.length === 0) throw new Error('not a course')
      setState((s) => ({ courses: [...s.courses, ...valid], selectedId: valid[0].id }))
    } catch {
      window.alert('That file is not a course exported from this calculator.')
    }
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
              const p = computeCourse(c).percent
              return (
                <option key={c.id} value={c.id}>
                  {c.name} — {formatPercent(p, c.displayDecimals ?? 2)}
                </option>
              )
            })}
          </select>
          <button type="button" onClick={() => addCourse(blankCourse())}>
            New
          </button>
          <select
            aria-label="Load example"
            value=""
            onChange={(e) => {
              const ex = EXAMPLES.find((x) => x.key === e.target.value)
              if (ex) addCourse(ex.build())
            }}
          >
            <option value="">Add example…</option>
            {EXAMPLES.map((e) => (
              <option key={e.key} value={e.key}>
                {e.label}
              </option>
            ))}
          </select>
          {course && (
            <button type="button" onClick={() => addCourse({ ...reId(course), name: `${course.name} (copy)` })}>
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

      {course && result ? (
        <main className="layout">
          <CourseEditor course={course} result={result} mutate={mutate} />
          <Summary course={course} result={result} />
        </main>
      ) : (
        <main className="empty-state">
          <p>No courses yet.</p>
          <button type="button" onClick={() => addCourse(blankCourse())}>
            Create a course
          </button>
        </main>
      )}
    </div>
  )
}
