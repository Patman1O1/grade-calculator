import { useState } from 'react'

interface Props {
  value: number | null
  onChange: (value: number | null) => void
  /** When false, clearing the field yields 0 instead of null. */
  nullable?: boolean
  placeholder?: string
  min?: number
  className?: string
  'aria-label'?: string
}

const parse = (s: string): number | null => {
  if (s.trim() === '') return null
  const n = Number(s)
  return Number.isFinite(n) ? n : null
}

/**
 * Numeric text field that lets the user type intermediate states like "18." or
 * "-" without the value snapping back, while still reporting numbers upward.
 */
export function NumberInput({ value, onChange, nullable = true, placeholder, min, className, ...rest }: Props) {
  const [text, setText] = useState(value === null ? '' : String(value))
  const [prev, setPrev] = useState(value)

  // Re-sync when the value changes from outside (e.g. loading another course).
  if (value !== prev) {
    setPrev(value)
    if (parse(text) !== value) setText(value === null ? '' : String(value))
  }

  return (
    <input
      className={className}
      type="text"
      inputMode="decimal"
      value={text}
      placeholder={placeholder}
      aria-label={rest['aria-label']}
      onChange={(e) => {
        const s = e.target.value
        setText(s)
        let n = parse(s)
        if (n !== null && min !== undefined && n < min) n = min
        if (n === null && !nullable) n = 0
        if (s.trim() === '' || parse(s) !== null) {
          setPrev(n)
          onChange(n)
        }
      }}
    />
  )
}
