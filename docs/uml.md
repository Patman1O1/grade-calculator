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
    engine["engine.rs"]
    examples["examples.rs"]
    model["model.rs"]
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
  api --> engine
  api --> examples
  engine --> model
  examples --> model
  api --> model
```

## 2. Domain model (`backend/src/model.rs`, mirrored in `frontend/src/types.ts`)

A `Course` owns its categories and letter scale, and each `Category` owns its items.

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
    +remaining_count() usize
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
    +worth() f64
    +is_remaining() bool
  }
  class Cutoff {
    <<struct>>
    +letter: String
    +min: f64
  }
  Course "1" *-- "0..*" Category : categories
  Category "1" *-- "0..*" Item : items
  Course "1" *-- "0..*" Cutoff : scale
  Course --> "1" Scheme : scheme
```

`excluded`, `neverDrop` and `extraCredit` default to `false` and are left out of the JSON when false, so the JSON matches what the TypeScript-only version of the app exported.

## 3. Results (`backend/src/model.rs`)

The engine returns a result tree that mirrors the course tree. Each result node carries the `id` of the input node it was computed from; the front-end looks results up by id, so a result that is one keystroke behind never lands on the wrong row. `Needed` is what the "what do I need?" solver returns.

```mermaid
classDiagram
  direction LR
  class CourseResult {
    <<struct>>
    +categories: Vec~CategoryResult~
    +earned: f64
    +possible: f64
    +percent?: f64
    +letter?: String
  }
  class CategoryResult {
    <<struct>>
    +id: String
    +name: String
    +items: Vec~ItemResult~
    +earned: f64
    +possible: f64
    +percent?: f64
  }
  class ItemResult {
    <<struct>>
    +id: String
    +counted: bool
    +dropped: bool
    +earned: f64
    +possible: f64
  }
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
  CourseResult "1" *-- "0..*" CategoryResult : categories
  CategoryResult "1" *-- "0..*" ItemResult : items
  Needed <|-- NoRemaining
  Needed <|-- Secured
  Needed <|-- Impossible
  Needed <|-- Needs
```

## 4. Grading engine (`backend/src/engine.rs`)

```mermaid
classDiagram
  direction LR
  class engine {
    <<module>>
    -contribution(item: &Item, score: f64)$ (f64, f64)
    -choose_kept(candidates: &[Candidate], keep: usize, fixed_earned: f64, fixed_possible: f64, mode: Mode)$ Vec~usize~
    -drop_count(n: Option~f64~)$ usize
    +compute_category(category: &Category)$ CategoryResult
    +letter_for(percent: Option~f64~, scale: &[Cutoff])$ Option~String~
    +compute_course(course: &Course)$ CourseResult
    +with_remaining_at(course: &Course, fraction: f64)$ Course
    +needed_on_remaining(course: &Course, target: f64)$ Needed
    +round(x: f64, decimals: u32)$ f64
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
  engine ..> Candidate : drop search
  engine ..> Mode : drop lowest / highest
  engine ..> Course : reads
  engine ..> CourseResult : creates
  engine ..> Needed : returns
```

`Candidate` and `Mode` are private. `choose_kept` runs the Canvas-exact parametric search for drop-lowest (`Max`) and drop-highest (`Min`). `needed_on_remaining` bisects on the score given to every remaining item, recomputing the course each step.

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
    +result: CourseResult
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
  Server-->>Hook: { result, remaining, needs }
  Hook-->>App: data
  App->>Editor: result (looked up by id)
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
    +result: CourseResult or null
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
