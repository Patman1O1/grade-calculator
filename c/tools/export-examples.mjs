// Writes the example courses from src/grade/courses.ts to c/examples/*.json.
// These files are the shared fixtures for the C tests and can also be imported
// into the web app. Requires Node with TypeScript type stripping (22.18+).
//
//   node c/tools/export-examples.mjs
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { EXAMPLES } from '../../src/grade/courses.ts'

const outDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'examples')
mkdirSync(outDir, { recursive: true })

// Generated ids change on every run and are not needed by the engine.
const withoutIds = (value) =>
  JSON.parse(JSON.stringify(value, (key, v) => (key === 'id' ? undefined : v)))

for (const example of EXAMPLES) {
  const file = join(outDir, `${example.key}.json`)
  writeFileSync(file, `${JSON.stringify(withoutIds(example.build()), null, 2)}\n`)
  console.log(`wrote ${file}`)
}
