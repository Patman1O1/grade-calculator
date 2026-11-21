import { useState } from 'react'
import { defaultGradeMode, percentOf, type GradeMode, type Item } from '../item'
import { NumberInput } from './NumberInput'

interface Props {
  item: Item
  /** Grade entered as a percentage: store `percent` out of 100. */
  onPercent: (percent: number | null) => void
  /** Grade entered as points. */
  onPoints: (earned: number | null, possible: number) => void
  /** Switching to percentage entry: convert the current score (8/10 → 80/100). */
  onSwitchToPercent: () => void
}

/**
 * The Grade cell: a dropdown picks how the grade is entered, as one
 * percentage ("Score (%)") or as points earned / possible ("Points").
 */
export function GradeInput({ item, onPercent, onPoints, onSwitchToPercent }: Props) {
  // Which input is showing is UI state only; the item just stores earned / possible.
  const [mode, setMode] = useState<GradeMode>(() => defaultGradeMode(item))

  return (
    <div className="grade-input">
      <select
        aria-label={`${item.name} grade type`}
        title="How to enter this grade. A score in % is stored as that many points out of 100."
        value={mode}
        onChange={(e) => {
          const next = e.target.value as GradeMode
          if (next === 'percent' && item.possible !== 100) onSwitchToPercent()
          setMode(next)
        }}
      >
        <option value="percent">Score (%)</option>
        <option value="points">Points</option>
      </select>

      {mode === 'percent' ? (
        <label className="percent-field" title="Enter the score as a percentage, e.g. 85 for 85%">
          <NumberInput
            aria-label={`${item.name} score (%)`}
            className="num"
            placeholder="0–100"
            min={0}
            value={percentOf(item)}
            onChange={onPercent}
          />
          <span className="unit">%</span>
        </label>
      ) : (
        <span className="points-field">
          <NumberInput
            aria-label={`${item.name} points earned`}
            className="num"
            placeholder="–"
            value={item.earned}
            onChange={(v) => onPoints(v, item.possible)}
          />
          <span className="sep">/</span>
          <NumberInput
            aria-label={`${item.name} points possible`}
            className="num"
            placeholder="–"
            nullable={false}
            min={0}
            value={item.possible}
            onChange={(v) => onPoints(item.earned, v ?? 0)}
          />
        </span>
      )}
    </div>
  )
}
