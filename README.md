# grade-calculator

A course grade calculator that handles the grading schemes used by Canvas, Blackboard and typical syllabi. You can use it for any course: enter the categories and items, then see your current grade, letter, per-category breakdown, and the average you need on the remaining work for each letter.

It has two parts:

- **`backend/`**: a Rust ([axum](https://github.com/tokio-rs/axum)) server. It owns all of the grade math (drop rules, weighting, letters, the "what do I need?" solver), the example courses, and import validation. It exposes them as a small JSON API and also serves the built front-end.
- **`frontend/`**: a React + TypeScript app (Vite). It is the editor UI. It sends the course you are editing to the API and shows the results. Courses are saved in your browser (localStorage), so the server holds no user data, and they can be exported or imported as JSON.

## Running locally

You need Rust (stable, 1.85 or newer) and Node 20 or newer.

```sh
# Terminal 1: the API on http://localhost:3000
cd backend
cargo run

# Terminal 2: the front-end on http://localhost:5173 (proxies /api to :3000)
cd frontend
npm install
npm run dev
```

To run it the way it is deployed (one server, one port):

```sh
cd frontend && npm run build && cd ../backend
cargo run --release        # serves ../frontend/dist and the API on http://localhost:3000
```

Tests:

```sh
cd backend && cargo test   # engine unit tests, API tests, and the docs/ PDF checks
```

### Server configuration

| Variable | Default | Meaning |
|---|---|---|
| `PORT` / `HOST` | `3000` / `0.0.0.0` | Where to listen |
| `STATIC_DIR` | `dist`, `../frontend/dist` or `frontend/dist`, whichever has an `index.html` | Built front-end to serve. Set to an empty string for API only. |
| `RUST_LOG` | `info` | Log filter |

The front-end calls the API on its own origin. To point a separately hosted front-end at an API somewhere else, build it with `VITE_API_URL=https://your-api.example.com` (and `VITE_BASE=/sub/path/` if it is served from a sub-path). The API allows cross-origin requests.

## API

All bodies are JSON in the same camelCase shape the front-end stores and exports (see [`backend/src/model.rs`](backend/src/model.rs)).

| Method | Path | Body | Response |
|---|---|---|---|
| GET | `/api/health` | | `{ "status": "ok" }` |
| GET | `/api/examples` | | `[{ key, label }]` |
| GET | `/api/examples/{key}` | | a `Course` with fresh ids |
| GET | `/api/courses/blank` | | a starter `Course` |
| POST | `/api/evaluate` | `{ course, targets: number[] }` | `{ result, remaining, needs: [{ target, needed }] }` |
| POST | `/api/summaries` | `{ courses }` | `[{ id, percent, letter }]` |
| POST | `/api/import` | a `Course` or a list of them | `{ courses }`, validated, with fresh ids |

Errors come back as `{ "error": "..." }` with a 4xx status.

## Deploying for free

GitHub Pages only hosts static files, so it can't run the Rust server. The repo includes a [`Dockerfile`](Dockerfile) that builds both halves into one small image, and a [`render.yaml`](render.yaml) for [Render](https://render.com)'s free web-service tier: in the Render dashboard choose **New → Blueprint** and pick this repo. Any host that runs a Docker image and sets `PORT` works the same way. Free Render services sleep when idle, so the first request after a quiet period takes a few seconds.

[CI](.github/workflows/ci.yml) runs the Rust tests, builds the front-end, and builds the Docker image on every push.

## How grades are computed

| Concept | Meaning |
|---|---|
| **Item** | `score / outOf`. A blank score means the item is not graded yet and is ignored. `outOf = 0` makes the item pure extra credit. |
| **Worth** | How much an item counts inside its category. It defaults to `outOf` (plain points). Set it to rescale an item, for example a report scored 0–100 that is worth 10 course points, or exams with different point totals that each count equally. |
| **Excluded** | The item is shown but does not count (for example Canvas "does not count toward final grade" items, or sub-scores). |
| **Drop lowest / highest** | Drop rules are applied per category to graded items. When items are worth different amounts, the calculator drops whichever items maximize (or minimize) the result, the same way Canvas does, rather than just the lowest percentage. Items marked *Keep* are never dropped. |
| **Total points** | Course % = sum of earned / sum of possible over all categories. |
| **Weighted categories** | Course % = Σ weight × category % divided by Σ weight of categories that have graded work. Ungraded categories are rescaled out, as on Canvas. *Extra credit* categories add to the grade without adding to the denominator. |
| **Letter scale** | Each letter has a minimum %. |

The engine lives in [`backend/src/engine.rs`](backend/src/engine.rs) and has no HTTP dependencies.

## Verification against `docs/`

[`backend/src/examples.rs`](backend/src/examples.rs) has the four courses in `docs/`, transcribed from each `grades-11-5-2026.pdf` and configured from each `syllabus.pdf`. [`backend/tests/courses.rs`](backend/tests/courses.rs) checks that the calculator reproduces every total and group row printed in those PDFs:

| Course | Gradebook | PDF shows | Calculator |
|---|---|---|---|
| CS 401 | Canvas, total points; Late Days excluded | 56.95% (106.50 / 187.00) | 56.95% (106.50 / 187.00) |
| CS 411 | Canvas, weighted 18/24/24/24 (+ Final 10, EC 0); drop 2 lowest quizzes and labs | 79.53% | 79.53% |
| CS 440 | Blackboard, each item scaled to its syllabus point value | Total 52.6 of 67, PPP 78.4 | 52.6 of 67, 78.4 |
| Math 218 | Canvas, total points with a 65.39/0 "Midsemester Score" | 118.28% (241.29 / 204.00) | 118.28% (241.29 / 204.00) |

Math 218 also includes a syllabus-weighted version (online HW 15%, written HW 15%, exams 70% split 20/20/30, lowest homework of each type dropped). It reproduces the instructor's posted **Midsemester Score of 65.39** from the work graded at that point. That confirms the method: missing exams are projected at your exam average. Canvas's 118.28% is an artifact of how its gradebook is configured, not a real course grade, so the app includes both versions.

### Assumptions

- **CS 440:** Weeklies, Reviews, Grammatical, OCL and Class Participation are all at full credit and together make up 12 of the 67 possible points. The PDF determines that total but not how it splits between Weeklies and Reviews. The split doesn't change the grade.
- **CS 401:** letter cutoffs are the syllabus point thresholds (360/320/280/200) as a share of the 410 planned points.
- **CS 411 / CS 440:** neither syllabus gives a letter scale, so 90/80/70/60 is used.
