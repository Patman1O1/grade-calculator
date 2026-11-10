// Differential test: runs random courses (plus the examples) through both the
// TypeScript engine (src/grade/engine.ts) and the C `grade` binary, and
// requires every number to be identical. Requires Node 22.18+.
//
//   node c/tools/difftest.mjs c/build/grade [count] [seed]
import { spawnSync } from 'node:child_process'
import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { computeCourse, neededOnRemaining } from '../../src/grade/engine.ts'

// Same rule as engine.ts's (private) remainingItems.
const remainingCount = (course) =>
  course.categories.reduce(
    (n, c) => n + c.items.filter((i) => i.score === null && !i.excluded && i.outOf > 0).length,
    0,
  )

const [binary, countArg = '5000', seedArg = '20261110'] = process.argv.slice(2)
if (!binary) {
  console.error('usage: node difftest.mjs PATH_TO_GRADE [count] [seed]')
  process.exit(2)
}
const COUNT = Number(countArg)
const TARGETS = [85, 72.5, 101, 0]

// Small deterministic PRNG (mulberry32) so failures are reproducible.
let state = Number(seedArg) >>> 0
const rand = () => {
  state = (state + 0x6d2b79f5) >>> 0
  let t = state
  t = Math.imul(t ^ (t >>> 15), t | 1)
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296
}
const pick = (xs) => xs[Math.floor(rand() * xs.length)]
const chance = (p) => rand() < p

const SCALES = [
  [{ letter: 'A', min: 90 }, { letter: 'B', min: 80 }, { letter: 'C', min: 70 }, { letter: 'D', min: 60 }, { letter: 'F', min: 0 }],
  [{ letter: 'C', min: 70 }, { letter: 'A', min: 87 }, { letter: 'F', min: 0 }, { letter: 'B', min: 75 }],
  [{ letter: 'A', min: (360 / 410) * 100 }, { letter: 'B', min: (320 / 410) * 100 }, { letter: 'F', min: 0 }],
  [{ letter: 'P', min: 50 }, { letter: 'P2', min: 50 }, { letter: 'NP', min: 0 }],
  [{ letter: 'A', min: 93 }, { letter: 'A-', min: 90 }],
  [],
]

function randomScore(outOf) {
  if (outOf === 0) return pick([0, 1, 2.5, 65.39])
  const r = rand()
  if (r < 0.15) return outOf // full marks (lots of ties)
  if (r < 0.25) return 0
  if (r < 0.6) return Math.round(rand() * outOf) // integers
  if (r < 0.65) return outOf * 1.1 // over 100%
  return Math.round(rand() * outOf * 100) / 100
}

function randomItem(i) {
  const outOf = pick([10, 10, 10, 20, 100, 50, 5, 1, 0.5, 3, 0, 2.5, 18])
  const item = { name: `Item ${i}`, score: chance(0.25) ? null : randomScore(outOf), outOf }
  if (chance(0.25)) item.worth = pick([1, 5, 8, 10, 15, 20, 0, 2.5])
  if (chance(0.08)) item.excluded = true
  if (chance(0.1)) item.neverDrop = true
  return item
}

function randomCourse(n) {
  const categories = []
  const catCount = Math.floor(rand() * 6)
  for (let c = 0; c < catCount; c++) {
    const items = []
    const itemCount = Math.floor(rand() * 10)
    for (let i = 0; i < itemCount; i++) items.push(randomItem(i))
    const cat = { name: `Cat ${c}`, weight: pick([0, 5, 10, 15, 15.5, 20, 24, 33.3, 70, -5]), items }
    if (chance(0.4)) cat.dropLowest = pick([1, 1, 2, 3, 0.7, 12])
    if (chance(0.2)) cat.dropHighest = pick([1, 2])
    if (chance(0.1)) cat.extraCredit = true
    categories.push(cat)
  }
  return {
    name: `Random ${n}`,
    scheme: chance(0.5) ? 'points' : 'weighted',
    categories,
    scale: pick(SCALES),
  }
}

// Expected output in the C binary's JSON shape, computed by the TS engine.
function expected(course) {
  const r = computeCourse(course)
  const letters = course.scale.filter((c) => c.min > 0).sort((a, b) => b.min - a.min)
  const needed = [
    ...letters.map((c) => ({ letter: c.letter, target: c.min })),
    ...TARGETS.map((t) => ({ letter: null, target: t })),
  ].map((q) => {
    const n = neededOnRemaining(course, q.target)
    const out = { ...q, kind: n.kind }
    if (n.kind === 'impossible') out.best = n.best
    if (n.kind === 'needs') out.fraction = n.fraction
    return out
  })
  return {
    name: course.name,
    scheme: course.scheme,
    earned: r.earned,
    possible: r.possible,
    percent: r.percent,
    letter: r.letter,
    remaining: remainingCount(course),
    categories: r.categories.map((c) => ({
      name: c.category.name,
      weight: c.category.weight,
      earned: c.earned,
      possible: c.possible,
      percent: c.percent,
      items: c.items.map((i) => ({
        name: i.item.name,
        counted: i.counted,
        dropped: i.dropped,
        excluded: !!i.item.excluded,
        earned: i.earned,
        possible: i.possible,
      })),
    })),
    needed,
  }
}

function diff(a, b, path, out) {
  if (out.length > 5) return
  if (typeof a === 'number' && typeof b === 'number') {
    if (a !== b) out.push(`${path}: ts=${a} c=${b}`)
  } else if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) out.push(`${path}: length ts=${a.length} c=${b.length}`)
    else a.forEach((x, i) => diff(x, b[i], `${path}[${i}]`, out))
  } else if (a && b && typeof a === 'object' && typeof b === 'object') {
    for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) diff(a[k], b[k], `${path}.${k}`, out)
  } else if (a !== b) {
    out.push(`${path}: ts=${JSON.stringify(a)} c=${JSON.stringify(b)}`)
  }
}

const examplesDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'examples')
const courses = readdirSync(examplesDir)
  .filter((f) => f.endsWith('.json'))
  .map((f) => JSON.parse(readFileSync(join(examplesDir, f), 'utf8')))
for (let n = 0; n < COUNT; n++) courses.push(randomCourse(n))

const args = ['--json', '--needed', ...TARGETS.flatMap((t) => ['--target', String(t)]), '-']
let mismatches = 0
const BATCH = 500
for (let start = 0; start < courses.length; start += BATCH) {
  const batch = courses.slice(start, start + BATCH)
  const run = spawnSync(binary, args, { input: JSON.stringify(batch), encoding: 'utf8', maxBuffer: 1 << 28 })
  if (run.status !== 0) {
    console.error(`grade exited with ${run.status}: ${run.stderr}`)
    process.exit(1)
  }
  const lines = run.stdout.trimEnd().split('\n')
  if (lines.length !== batch.length) {
    console.error(`expected ${batch.length} results, got ${lines.length}`)
    process.exit(1)
  }
  lines.forEach((line, i) => {
    const problems = []
    diff(expected(batch[i]), JSON.parse(line), 'result', problems)
    if (problems.length) {
      mismatches++
      if (mismatches <= 5) console.error(`mismatch for ${batch[i].name}:\n  ${problems.join('\n  ')}`)
    }
  })
}

if (mismatches) {
  console.error(`${mismatches} of ${courses.length} courses differ`)
  process.exit(1)
}

// Coverage summary, so a pass is known to have exercised the hard paths.
const stats = { dropped: 0, smarterThanNaive: 0, needs: 0, impossible: 0, secured: 0 }
for (const course of courses) {
  const r = computeCourse(course)
  for (const c of r.categories) {
    stats.dropped += c.items.filter((i) => i.dropped).length
    const k = Math.floor(c.category.dropLowest ?? 0)
    if (k > 0 && !c.category.dropHighest && c.percent !== null) {
      // Naive rule: drop the k lowest percentages among droppable graded items.
      const counted = c.items.filter((i) => i.counted || i.dropped)
      const pool = counted.filter((i) => !i.item.neverDrop)
      const keepCount = Math.max(pool.length - k, counted.length > pool.length ? 0 : 1)
      const pct = (i) => (i.possible > 0 ? i.earned / i.possible : Infinity)
      const kept = [...pool].sort((a, b) => pct(b) - pct(a)).slice(0, keepCount)
      const fixed = counted.filter((i) => i.item.neverDrop)
      const sum = (xs, f) => xs.reduce((s, x) => s + f(x), 0)
      const p = sum([...fixed, ...kept], (i) => i.possible)
      if (p > 0 && c.percent > (sum([...fixed, ...kept], (i) => i.earned) / p) * 100 + 1e-9) stats.smarterThanNaive++
    }
  }
  for (const t of TARGETS) {
    const kind = neededOnRemaining(course, t).kind
    if (kind in stats) stats[kind]++
  }
}
console.log(`difftest: ${courses.length} courses identical between TypeScript and C`)
console.log(
  `  covered: ${stats.dropped} dropped items, ${stats.smarterThanNaive} categories where the optimal drop beats ` +
    `"drop lowest %", needed-score answers: ${stats.needs} needs / ${stats.impossible} impossible / ${stats.secured} secured`,
)
