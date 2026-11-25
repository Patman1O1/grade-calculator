import { useEffect, useMemo, useRef, useState } from 'react'
import { decimals, exportFileName, percentsById, updateCourse, type Course, type Mutate } from './course'
import { engine } from './engine'
import { formatPercent } from './format'
import { loadState, saveState, type SavedState } from './storage'
import { CourseEditor } from './components/CourseEditor'
import { Summary } from './components/Summary'

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

/** Editor + summary for one course, graded by the Rust engine on every change. */
function CourseView({ course, mutate }: { course: Course; mutate: Mutate }) {
  const evaluation = useMemo(() => {
    try {
      return { data: engine.evaluate(course), error: null }
    } catch (e) {
      return { data: null, error: message(e) }
    }
  }, [course])

  return (
    <>
      {evaluation.error && (
        <p className="banner" role="alert">
          This course couldn’t be graded: {evaluation.error}
        </p>
      )}
      <main className="layout">
        <CourseEditor course={course} graded={evaluation.data} mutate={mutate} />
        <Summary course={course} graded={evaluation.data} mutate={mutate} />
      </main>
    </>
  )
}

/** Saved courses, or none on a first visit. */
function initialState(): SavedState {
  return loadState() ?? { courses: [], selectedId: null }
}

export default function App() {
  const [state, setState] = useState<SavedState>(initialState)
  const fileInput = useRef<HTMLInputElement>(null)

  useEffect(() => saveState(state), [state])

  const { courses } = state
  const course = courses.find((c) => c.id === state.selectedId) ?? courses[0] ?? null
  const percents = useMemo(() => {
    try {
      return percentsById(courses, engine.summaries(courses))
    } catch {
      return new Map<string, number | null>()
    }
  }, [courses])

  const select = (id: string) => setState((s) => ({ ...s, selectedId: id }))
  const addCourses = (added: Course[]) => {
    if (added.length === 0) return
    setState((s) => ({ courses: [...s.courses, ...added], selectedId: added[0].id }))
  }

  const mutate: Mutate = (fn) => {
    if (course) setState((s) => ({ ...s, courses: updateCourse(s.courses, course.id, fn) }))
  }

  const removeCourse = () => {
    if (!course || !window.confirm(`Delete “${course.name}”?`)) return
    setState((s) => {
      const rest = s.courses.filter((c) => c.id !== course.id)
      return { courses: rest, selectedId: rest[0]?.id ?? null }
    })
  }

  const duplicate = () => {
    if (!course) return
    const [copy] = engine.importCourses(course)
    addCourses([{ ...copy, name: `${course.name} (copy)` }])
  }

  const exportCourse = () => {
    if (!course) return
    const blob = new Blob([JSON.stringify(course, null, 2)], { type: 'application/json' })
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = exportFileName(course)
    a.click()
    URL.revokeObjectURL(a.href)
  }

  const importCourse = async (file: File) => {
    try {
      addCourses(engine.importCourses(JSON.parse(await file.text())))
    } catch (e) {
      window.alert(e instanceof SyntaxError ? 'That file is not valid JSON.' : message(e))
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
            {courses.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name} — {formatPercent(percents.get(c.id), decimals(c))}
              </option>
            ))}
          </select>
          <button type="button" onClick={() => addCourses([engine.blankCourse()])}>
            New
          </button>
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
        <main className="empty-state">
          <p>No courses yet.</p>
          <button type="button" onClick={() => addCourses([engine.blankCourse()])}>
            Create a course
          </button>
        </main>
      )}
    </div>
  )
}
