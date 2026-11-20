# UML diagrams

These diagrams cover the Rust server in `backend/src/` and the React front-end in `frontend/src/`. Neither side uses classes in the object-oriented sense, so the diagrams use these conventions:

| Notation | Meaning in the code |
|---|---|
| `«struct»` / `«enum»` | a Rust `struct` / `enum` (field names shown as they appear in JSON, camelCase) |
| `«variant»` | one variant of a Rust enum that serialises with a `kind` tag |
| `«interface»` / `«type»` | a TypeScript `interface` / `type` alias |
| `«module»` | a source file; its free functions and constants appear as static (underlined) members |
| `«component»` | a React function component; its `useState` values appear as attributes |
| `+` / `-` | public (`pub` / `export`) / private |
| `?` | optional (`Option<T>` in Rust, `?` in TypeScript) |
| `/` before a name | computed by `grade()`, not entered by the user (`Option<T>`, omitted from the JSON until the course is graded) |

## 1. Architecture

The browser keeps the courses; the server does every calculation and holds no user data.

```mermaid
flowchart LR
  subgraph browser["frontend/ (React + TypeScript)"]
    App["App.tsx"]
    CourseEditor["components/CourseEditor.tsx"]
    Summary["components/Summary.tsx"]
    NumberInput["components/NumberInput.tsx"]
    hooks["hooks.ts<br/>useRemote"]
    apits["api.ts"]
    storage["storage.ts<br/>localStorage"]
    format["format.ts"]
  end
  subgraph server["backend/ (Rust, axum)"]
    main["main.rs<br/>binary"]
    lib["lib.rs<br/>app()"]
    api["api.rs<br/>/api routes"]
    item["item.rs<br/>Item"]
    category["category.rs<br/>Category + drop search"]
    course["course.rs<br/>Course, Scheme, Cutoff, Needed"]
    examples["examples.rs"]
    static["ServeDir<br/>frontend/dist"]
  end

  App --> hooks --> apits
  App --> storage
  App -->|renders| CourseEditor
  App -->|renders| Summary
  CourseEditor --> NumberInput
  Summary --> NumberInput
  CourseEditor --> format
  Summary --> format
  apits -->|"HTTP JSON /api/*"| api
  main --> lib --> api
  lib --> static
  api --> course
  api --> examples
  examples --> course
  course -->|grades| category
  category -->|grades| item
```

## 2. Domain model and grading (`backend/src/item.rs`, `category.rs`, `course.rs`; mirrored in `frontend/src/types.ts`)

A `Course` owns its categories and letter scale, and each `Category` owns its items. Each type lives in its own module together with the logic that grades it, and the grade lives on the same types: the fields marked `/` are filled in by `grade()`. `Course::grade` grades each category, and `Category::grade` grades each item and then applies its drop rules.

```mermaid
classDiagram
  direction LR
  class Scheme {
    <<enum>>
    Points = "points"
    Weighted = "weighted"
  }
  class Course {
    <<struct>>
    +id: String
    +name: String
    +scheme: Scheme
    +categories: Vec~Category~
    +scale: Vec~Cutoff~
    +displayDecimals?: u32
    +notes?: String
    +/earned?: f64
    +/possible?: f64
    +/percent?: f64
    +/letter?: String
    +grade(&mut self)
    +graded(&self) Course
    +letter_for(&self, percent: Option~f64~) Option~String~
    +remaining_count(&self) usize
    +with_remaining_at(&self, fraction: f64) Course
    +needed_on_remaining(&self, target: f64) Needed
  }
  class Category {
    <<struct>>
    +id: String
    +name: String
    +weight: f64
    +dropLowest?: f64
    +dropHighest?: f64
    +extraCredit: bool
    +items: Vec~Item~
    +/earned?: f64
    +/possible?: f64
    +/percent?: f64
    +grade(&mut self)
    -apply_drops(&mut self)
  }
  class Item {
    <<struct>>
    +id: String
    +name: String
    +score?: f64
    +outOf: f64
    +worth?: f64
    +excluded: bool
    +neverDrop: bool
    +note?: String
    +/status?: Status
    +/earned?: f64
    +/possible?: f64
    +worth(&self) f64
    +is_remaining(&self) bool
    +contribution(&self, score: f64) (f64, f64)
    +grade(&mut self)
  }
  class Cutoff {
    <<struct>>
    +letter: String
    +min: f64
  }
  class Status {
    <<enum>>
    Counted = "counted"
    Dropped = "dropped"
    Excluded = "excluded"
    Ungraded = "ungraded"
  }
  Course "1" *-- "0..*" Category : categories
  Category "1" *-- "0..*" Item : items
  Course "1" *-- "0..*" Cutoff : scale
  Course --> "1" Scheme : scheme
  Item --> "0..1" Status : status
  Course ..> Needed : needed_on_remaining
```

- `excluded`, `neverDrop` and `extraCredit` default to `false` and are left out of the JSON when false, so the JSON matches what the TypeScript-only version of the app exported.
- The computed fields are `None` and left out of the JSON until the course is graded, so the courses the browser stores and exports never contain them. They are also ignored when a course is deserialised: a client can't send in a grade, it is always recomputed.
- On a graded course, a category's or the course's `percent` (and the course's `letter`) stays absent when nothing is graded yet; `earned` and `possible` are always present.
- Every node keeps its `id`, so the front-end matches graded values to rows by id. An evaluation that is one keystroke behind never lands on the wrong row.
- `Item::grade` sets each item's `Status` to `Excluded` (if marked excluded, even once it has a score), `Ungraded` (no score) or `Counted`; `Category::apply_drops` then changes some `Counted` items to `Dropped`. The category sums only `Counted` items, and the front-end styles each row by its status. A dropped item keeps its `earned` and `possible`.
- `Course::needed_on_remaining` bisects on the score given to every remaining item (via `with_remaining_at`), regrading a copy of the course each step.
- `round` (display rounding, used by tests) and the serde helper `is_false` are free functions in `lib.rs`, which also re-exports all six types at the crate root.

## 3. "What do I need?" (`backend/src/course.rs`)

`Needed` is what `Course::needed_on_remaining` returns for one target percentage.

```mermaid
classDiagram
  direction LR
  class Needed {
    <<enum>>
    +kind: String
  }
  class NoRemaining {
    <<variant>>
    +kind = "no-remaining"
  }
  class Secured {
    <<variant>>
    +kind = "secured"
  }
  class Impossible {
    <<variant>>
    +kind = "impossible"
    +best?: f64
  }
  class Needs {
    <<variant>>
    +kind = "needs"
    +fraction: f64
  }
  Needed <|-- NoRemaining
  Needed <|-- Secured
  Needed <|-- Impossible
  Needed <|-- Needs
```

## 4. Drop search (`backend/src/category.rs`, private)

```mermaid
classDiagram
  direction LR
  class category {
    <<module>>
    -drop_count(n: Option~f64~)$ usize
    -choose_kept(candidates: &[Candidate], keep: usize, fixed_earned: f64, fixed_possible: f64, mode: Mode)$ Vec~usize~
  }
  class Candidate {
    <<struct>>
    -index: usize
    -earned: f64
    -possible: f64
  }
  class Mode {
    <<enum>>
    Max
    Min
  }
  class Category {
    <<struct>>
    -apply_drops(&mut self)
  }
  Category ..> category : apply_drops
  category ..> Candidate : drop search
  category ..> Mode : drop lowest / highest
```

`Category::apply_drops` collects the graded, droppable items as `Candidate`s and calls `choose_kept`, which runs the Canvas-exact parametric search for drop-lowest (`Max`) and drop-highest (`Min`). Items marked `neverDrop` are held fixed, and at least one item always stays.

## 5. Example courses (`backend/src/examples.rs`)

```mermaid
classDiagram
  direction LR
  class examples {
    <<module>>
    +EXAMPLES: [Example; 5]$
    +new_id(prefix: &str)$ String
    +re_id(course: Course)$ Course
    +standard_scale()$ Vec~Cutoff~
    +cs401()$ Course
    +cs411()$ Course
    +cs440()$ Course
    +math218()$ Course
    +math218_canvas()$ Course
    +example(key: &str)$ Option~Course~
    +blank_course()$ Course
    -item(name, score, out_of)$ Item
    -g(name, score, out_of)$ Item
    -u(name, out_of)$ Item
    -category(name, weight, items)$ Category
    -math218_scale()$ Vec~Cutoff~
    -math218_mom()$ Vec~Item~
    -math218_written()$ Vec~Item~
  }
  class Example {
    <<struct>>
    +key: &str
    +label: &str
    +build: fn() Course
  }
  examples "1" *-- "5" Example : EXAMPLES
  Example ..> Course : build()
```

`g` builds a graded item and `u` an ungraded one; private builder methods (`with_worth`, `exclude`, `with_note`, `drop_lowest`) fill in the optional fields. Server ids contain `-s<n>` and front-end ids contain `-c<n>`, so the two never collide.

## 6. HTTP API (`backend/src/api.rs`)

```mermaid
classDiagram
  direction LR
  class api {
    <<module>>
    +MAX_TARGETS: usize = 32$
    +MAX_COURSES: usize = 200$
    +router()$ Router
    +evaluate_course(req: EvaluateRequest)$ EvaluateResponse
    -health()$ Json
    -list_examples()$ Json
    -get_example(key: String)$ Json~Course~
    -get_blank()$ Json~Course~
    -evaluate(req: EvaluateRequest)$ Json~EvaluateResponse~
    -summaries(req: SummariesRequest)$ Json
    -import(data: Value)$ Json
  }
  class EvaluateRequest {
    <<struct>>
    +course: Course
    +targets: Vec~f64~
  }
  class EvaluateResponse {
    <<struct>>
    +course: Course
    +remaining: usize
    +needs: Vec~TargetNeed~
  }
  class TargetNeed {
    <<struct>>
    +target: f64
    +needed: Needed
  }
  class SummariesRequest {
    <<struct>>
    +courses: Vec~Course~
  }
  class CourseSummary {
    <<struct>>
    +id: String
    +percent?: f64
    +letter?: String
  }
  class ApiError {
    <<struct>>
    -status: StatusCode
    -message: String
  }
  api ..> EvaluateRequest
  api ..> EvaluateResponse
  EvaluateResponse *-- TargetNeed
  api ..> SummariesRequest
  api ..> CourseSummary
  api ..> ApiError : errors
```

Each private handler serves the route of the same name in the README's API table. `lib.rs` mounts `api::router()` under `/api`, serves the built front-end for every other path (falling back to `index.html`), and adds CORS, gzip and request logging. Grading runs on `spawn_blocking` threads.

### Editing a score

```mermaid
sequenceDiagram
  actor User
  participant Editor as CourseEditor
  participant App as App / CourseView
  participant Hook as useRemote
  participant Server as Rust API
  User->>Editor: types a score
  Editor->>App: mutate(draft => …)
  App->>App: new course state, saved to localStorage
  App->>Hook: key = JSON of course + targets
  Hook->>Hook: wait 60 ms, abort the previous request
  Hook->>Server: POST /api/evaluate { course, targets }
  Server-->>Hook: { course (graded), remaining, needs }
  Hook-->>App: data
  App->>Editor: graded course (looked up by id)
  App->>Summary: evaluation
```

## 7. Front-end (`frontend/src/`)

```mermaid
classDiagram
  direction TB
  class App {
    <<component>>
    -state: SavedState or null
    -examples: ExampleInfo[]
    -startupError: string or null
    -summaries: Remote~CourseSummary[]~
    -mutate: Mutate
  }
  class CourseView {
    <<component>>
    -custom: number or null
    -evaluation: Remote~Evaluation~
  }
  class CourseEditor {
    <<component>>
    +course: Course
    +graded: Course or null
    +mutate: Mutate
  }
  class ScaleEditor {
    <<component>>
  }
  class Summary {
    <<component>>
    +course: Course
    +evaluation: Evaluation or null
    +custom: number or null
    +onCustom(value) void
  }
  class NumberInput {
    <<component>>
    -text: string
    -prev: number or null
  }
  class ServerBanner {
    <<component>>
    +message: string
  }
  class api {
    <<module>>
    +examples()$ Promise
    +example(key)$ Promise
    +blank()$ Promise
    +evaluate(course, targets, signal)$ Promise
    +summaries(courses, signal)$ Promise
    +import(data)$ Promise
  }
  class hooks {
    <<module>>
    +useRemote(key, fetcher, delay)$ Remote
  }
  App *-- "0..1" CourseView : selected course
  App *-- "0..1" ServerBanner
  CourseView *-- CourseEditor
  CourseView *-- Summary
  CourseView *-- "0..1" ServerBanner
  CourseEditor *-- ScaleEditor
  CourseEditor *-- "0..*" NumberInput
  ScaleEditor *-- "0..*" NumberInput
  Summary *-- NumberInput : custom target
  App ..> api
  App ..> hooks
  CourseView ..> hooks
```

`CourseView` is keyed by course id, so switching courses starts with a fresh evaluation. `Mutate` is `(fn: (draft: Course) => void) => void`: `App` clones the selected course, lets `fn` edit the draft, and stores the result. `useRemote` keeps the last good response while a newer one is in flight, so numbers don't flicker while typing.
