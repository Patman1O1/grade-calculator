# grade-calculator

A course grade calculator that handles the grading schemes used by Canvas, Blackboard and typical syllabi. You can use it for any course: enter the categories and items, then see your current grade, letter, per-category breakdown, and the average you need on the remaining work for each letter.

It runs entirely in your browser and has two parts:

- **`backend/`**: the grading engine, in Rust. It owns all of the grade math (drop rules, weighting, letters, the "what do I need?" solver), the example courses, and import validation. It is compiled to WebAssembly with [wasm-bindgen](https://crates.io/crates/wasm-bindgen).
- **`frontend/`**: a React + TypeScript app (Vite). It is the editor UI and calls the engine directly, so every edit is graded instantly with no server. Courses are saved in your browser (localStorage) and can be exported or imported as JSON. The app starts with no courses; create one with **New**, or load one of the examples below from **Add example…**.

## Running locally

You need Rust (stable, 1.85 or newer) with the WebAssembly target, the `wasm-bindgen` CLI at the same version as `backend/Cargo.lock`, and Node 20 or newer:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked
```

Then:

```sh
cd frontend
npm install
npm run dev        # builds the engine, then serves http://localhost:5173/grade-calculator/
npm run build      # builds the engine and the app into frontend/dist
```

`npm run wasm` rebuilds just the engine (into `frontend/src/wasm/`, which is generated and not committed); run it again after changing Rust code while `npm run dev` is running.

Tests run natively, no browser needed:

```sh
cd engine && cargo test    # unit tests, the front-end interface, and the docs/ PDF checks
```

## Engine interface

[`backend/src/wasm.rs`](backend/src/wasm.rs) exports these functions to JavaScript; [`frontend/src/engine.ts`](frontend/src/engine.ts) wraps them with TypeScript types. They take and return plain objects in the same camelCase JSON shape the app stores and exports (see [`item/mod.rs`](backend/src/item/mod.rs), [`category/mod.rs`](backend/src/category/mod.rs) and [`course/mod.rs`](backend/src/course/mod.rs)). Errors are thrown as JS `Error`s.

| Function | Returns |
|---|---|
| `examples()` | `[{ key, label }]` |
| `example(key)` | a `Course` with fresh ids |
| `blankCourse()` | a starter `Course` |
| `evaluate(course, targets)` | `{ course, remaining, needs: [{ target, needed }] }`, where `course` is the graded copy |
| `summaries(courses)` | `[{ id, percent, letter }]` |
| `normalizeCourses(courses)` | saved courses converted to the current format, with their ids kept |
| `importCourses(data)` | the courses in `data` (one or a list), validated, with fresh ids |

A graded course has extra computed fields: `earned`, `possible`, `percent` and `letter` on the course; `earned`, `possible` and `percent` on each category; and `status` (`counted`, `dropped`, `excluded` or `ungraded`) on each item. They are left out of ungraded courses and ignored on input.

`normalizeCourses(courses)` converts courses saved by an earlier version of the app (items stored as `score` / `outOf` / `worth`) to the current format, keeping their ids; the app runs saved courses through it on startup. `importCourses` accepts that older format too.

## Deploying

The built app is static files (HTML, JS, CSS and one `.wasm`), so it is hosted for free on GitHub Pages at https://patman1o1.github.io/grade-calculator/. [The workflow](.github/workflows/deploy.yml) tests the engine and builds the app on every push, and deploys pushes to `main`. One-time setup: repo **Settings → Pages → Source: GitHub Actions**.

## How grades are computed

| Concept | Meaning |
|---|---|
| **Item** | `earned` out of `possible` points. A blank `earned` means the item is not graded yet and is ignored. `possible = 0` makes the item pure extra credit. |
| **Earned / Total Points** | Each item has two fields next to its name: **Earned Points** (leave blank until it's graded) and **Total Points**. To enter a percentage, use a total of 100. |
| **Points set weight** | Within a category, items count in proportion to their total points: a 10-point quiz counts a tenth as much as a 100-point exam. To make a 0–100 report count 10 points, enter it scaled (85/100 → 8.5 / 10). |
| **Excluded** | The item is shown but does not count (for example Canvas "does not count toward final grade" items, or sub-scores). |
| **Drop lowest / highest** | Drop rules are applied per category to graded items. When items are worth different amounts, the calculator drops whichever items maximize (or minimize) the result, the same way Canvas does, rather than just the lowest percentage. Items marked *Keep* are never dropped. |
| **Total points** | Course % = sum of earned / sum of possible over all categories. |
| **Weighted categories** | Course % = Σ weight × category % divided by Σ weight of categories that have graded work. Ungraded categories are rescaled out, as on Canvas. *Extra credit* categories add to the grade without adding to the denominator. |
| **Letter scale** | Each letter has a minimum %. |

Each type is graded by its own module in `backend/src/`, none of which depend on WebAssembly: [`item/mod.rs`](backend/src/item/mod.rs) scales scores, [`category/mod.rs`](backend/src/category/mod.rs) applies drop rules, and [`course/mod.rs`](backend/src/course/mod.rs) combines categories, assigns the letter and solves "what do I need?".

## Verification against `docs/`

[`backend/src/examples.rs`](backend/src/examples.rs) has the four courses in `docs/`, transcribed from each `grades-11-5-2026.pdf` and configured from each `syllabus.pdf`. [`backend/tests/courses.rs`](backend/tests/courses.rs) checks that the calculator reproduces every total and group row printed in those PDFs:

| Course | Gradebook | PDF shows | Calculator |
|---|---|---|---|
| CS 401 | Canvas, total points; Late Days excluded | 56.95% (106.50 / 187.00) | 56.95% (106.50 / 187.00) |
| CS 411 | Canvas, weighted 18/24/24/24 (+ Final 10, EC 0); drop 2 lowest quizzes and labs | 79.53% | 79.53% |
| CS 440 | Blackboard, each item entered as its syllabus point value | Total 52.6 of 67, PPP 78.4 | 52.6 of 67, 78.4 |
| Math 218 | Canvas, total points with a 65.39/0 "Midsemester Score" | 118.28% (241.29 / 204.00) | 118.28% (241.29 / 204.00) |

Math 218 also includes a syllabus-weighted version (online HW 15%, written HW 15%, exams 70% split 20/20/30, lowest homework of each type dropped). It reproduces the instructor's posted **Midsemester Score of 65.39** from the work graded at that point. That confirms the method: missing exams are projected at your exam average. Canvas's 118.28% is an artifact of how its gradebook is configured, not a real course grade, so the app includes both versions.

### Assumptions

- **CS 440:** Weeklies, Reviews, Grammatical, OCL and Class Participation are all at full credit and together make up 12 of the 67 possible points. The PDF determines that total but not how it splits between Weeklies and Reviews. The split doesn't change the grade.
- **CS 401:** letter cutoffs are the syllabus point thresholds (360/320/280/200) as a share of the 410 planned points.
- **CS 411 / CS 440:** neither syllabus gives a letter scale, so 90/80/70/60 is used.
