# UML diagrams

These diagrams cover the Rust engine in `backend/src/` (compiled to WebAssembly) and the React front-end in `frontend/src/`. Neither side uses classes in the object-oriented sense, so the diagrams use these conventions:

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

Everything runs in the browser. The Rust engine is compiled to a WebAssembly module that the React app loads once at startup and then calls synchronously; courses are kept in localStorage. There is no server: the built app is static files on GitHub Pages.

```mermaid
flowchart LR
  subgraph frontend["frontend/ (React + TypeScript)"]
    main["main.tsx<br/>loadEngine(), then render"]
    App["App.tsx"]
    CourseEditor["components/CourseEditor.tsx"]
    Summary["components/Summary.tsx"]
    NumberInput["components/NumberInput.tsx"]
    coursets["course.ts<br/>Course, Scheme, Needed, CourseSummary, Mutate"]
    scalets["scale.ts<br/>Scale, Cutoff"]
    categoryts["category.ts<br/>Category"]
    itemts["item.ts<br/>Item"]
    statusts["status.ts<br/>Status"]
    enginets["engine.ts<br/>typed wrapper + result types"]
    glue["wasm/engine.js<br/>generated glue"]
    storage["storage.ts<br/>localStorage"]
    format["format.ts"]
  end
  subgraph engine["backend/ (Rust → engine_bg.wasm)"]
    wasm["wasm.rs<br/>#[wasm_bindgen] exports"]
    api["api.rs<br/>evaluate, summaries, import"]
    item["item/mod.rs<br/>Item"]
    status["status/mod.rs<br/>Status"]
    scale["scale/mod.rs<br/>Scale"]
    category["category/mod.rs<br/>Category + drop search"]
    course["course/mod.rs<br/>Course, Scheme, Needed"]
  end

  main --> App
  App --> enginets --> glue
  App --> storage
  App -->|renders| CourseEditor
  App -->|renders| Summary
  CourseEditor --> NumberInput
  Summary --> NumberInput
  CourseEditor --> format
  Summary --> format
  App --> coursets
  CourseEditor --> coursets
  CourseEditor --> categoryts
  CourseEditor --> itemts
  Summary --> coursets
  coursets --> categoryts --> itemts --> statusts
  coursets --> scalets
  CourseEditor --> scalets
  CourseEditor --> statusts
  glue -->|"calls (JS objects via serde-wasm-bindgen)"| wasm
  wasm --> api
  api --> course
  course -->|grades| category
  course -->|letter| scale
  category -->|grades| item
  item --> status
```

## 2. Domain model and grading (`backend/src/item/mod.rs`, `category/mod.rs`, `course/mod.rs`, `scale/mod.rs`; mirrored in `frontend/src/item.ts`, `category.ts`, `course.ts`, `scale.ts`)

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
    +name: String
    +scheme: Scheme
    +categories: Vec~Category~
    +scale: Scale
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
    +needs(&self, targets: &[f64]) Vec~Needed~
  }
  class Category {
    <<struct>>
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
    +reset_statuses(&mut self)
    +has_drop_rules(&self) bool
    -apply_drops(&mut self)
  }
  class Item {
    <<struct>>
    +name: String
    +earnedPoints?: f64
    +totalPoints: f64
    +neverDrop: bool
    +note?: String
    +status: Status
    +new(name, earned_points, total_points, never_drop, note, status)$ Item
    +drop(&mut self) bool
    +exclude(&mut self)
    +ungrade(&mut self) (Option~f64~, f64)
    +grade(&mut self, earned_points: f64, total_points: f64)
    +status(&self) Status
    +percent(&self) Option~f64~
  }
  class Scale {
    <<struct>>
    +a: f64
    +b: f64
    +c: f64
    +d: f64
    +f: f64
    +STANDARD: Scale$
    +cutoffs(&self) [&str × f64; 5]
    +letter_for(&self, percent: Option~f64~) Option~&str~
  }
  class Status {
    <<enum>>
    Counted = "counted"
    Dropped = "dropped"
    Excluded = "excluded"
    Ungraded = "ungraded"
    +is_counted(self) bool
    +is_dropped(self) bool
    +is_excluded(self) bool
    +is_ungraded(self) bool
  }
  Course "1" *-- "0..*" Category : categories
  Category "1" *-- "0..*" Item : items
  Course "1" *-- "1" Scale : scale
  Course --> "1" Scheme : scheme
  Item --> "1" Status : status
  Course ..> Needed : needs
```

- `neverDrop` and `extraCredit` default to `false` and are left out of the JSON when false.
- The computed fields are `None` and left out of the JSON until the course is graded, so the courses the browser stores and exports never contain them. They are also ignored when a course is deserialised: a client can't send in a grade, it is always recomputed.
- On a graded course, a category's or the course's `percent` (and the course's `letter`) stays absent when nothing is graded yet; `earned` and `possible` are always present.
- The engine has no ids: they belong to the front-end (React keys, the selected course, which course an edit applies to), and serde ignores any it is given. A graded course keeps its categories and items in the same order, so the front-end matches results to rows by position.
- An item is `earned_points` out of `total_points` (`earnedPoints` / `totalPoints` in JSON); those points are also how much it counts in its category. Categories and courses keep `earned` / `possible` for their computed totals. `Item` uses serde's derived `Deserialize`; `api::parse_courses` (import) then grades each item's status, so only an `"excluded"` status is taken from the input.
- `Status` lives in its own module (`status/`) and is a stored field of every item. Only `Excluded` is the user's choice and is kept as given; `Category::reset_statuses` turns anything else into `Counted` (has a score, via `Item::grade`) or `Ungraded` (no score), and `Category::apply_drops` then changes some `Counted` items to `Dropped` with `Item::drop`, which refuses never-drop items. Import runs `reset_statuses` too. So a status sent in from outside is never trusted except for `Excluded`. The category sums only `Counted` items, and the front-end styles each row by the graded status. A dropped item keeps its `earned_points` and `total_points`, so the UI can still show the score.
- `Course::needs` solves "what do I need?" for all targets at once (`needed_on_remaining` is the one-target version). It grades one working copy of the course, then varies only the categories with remaining work. Without drop rules in those categories the grade is a straight line in the score on the remaining work, so it is solved exactly from 0% and 100%; with drop rules it bisects (40 steps).
- Each of `item/`, `category/`, `course/`, `scale/`, `status/` and `utils/` keeps its code in `mod.rs` and its unit tests in `tests.rs` (declared as `#[cfg(test)] mod tests;`).
- `round` (display rounding, used by tests) and the serde helper `is_false` live in `utils/` (`mod.rs`, with their tests in `tests.rs`). `lib.rs` only declares the modules (`api`, `category`, `course`, `item`, `scale`, `status`, `utils`, `wasm`); types are used by their full paths, e.g. `crate::course::Course`.

## 3. "What do I need?" (`backend/src/course/mod.rs`)

`Needed` is what `Course::needs` returns for each target percentage: the average score needed on every remaining item, as a fraction of its points.

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

## 4. Drop search (`backend/src/category/mod.rs`, private)

```mermaid
classDiagram
  direction LR
  class category {
    <<module>>
    -drop_count(n: Option~f64~)$ usize
    -choose_kept(items: &[Item], pool: &mut Vec~usize~, keep: usize, fixed: (f64, f64), mode: Mode, scratch)$
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
  category ..> Mode : drop lowest / highest
```

`Category::apply_drops` collects the indexes of the graded, droppable items into a pool and calls `choose_kept`, which narrows the pool to the items kept by drop-lowest (`Max`) and then drop-highest (`Min`). It finds the same best set Canvas does, using Dinkelbach's method: for a ratio q the best set is the items with the largest `earned − q·possible`, so it repeatedly moves q to the ratio of the best set for q until q stops improving, usually in 2–5 rounds. Ties between sets with the same ratio keep the earlier items. Items marked `neverDrop` are held fixed, and at least one item always stays.

## 5. The blank course (`backend/src/course/mod.rs`)

```mermaid
classDiagram
  direction LR
  class Course {
    <<struct>>
    +blank()$ Course
  }
  class Scale {
    <<struct>>
    +STANDARD: Scale$
  }
  Course ..> Scale : STANDARD
```

`Course::blank` is the starting point behind the app's **New** button: weighted, a Homework and an Exams category with one ungraded item each, and the standard 90/80/70/60 scale.

The four courses in `docs/` live in `backend/tests/fixtures/mod.rs` as test data only; `tests/courses.rs` checks that the engine reproduces every total in their gradebook PDFs.

## 6. Engine interface (`backend/src/api.rs`, `backend/src/wasm.rs`)

`api.rs` holds the operations the front-end needs as plain Rust, so they are tested natively. `wasm.rs` exports them to JavaScript with `#[wasm_bindgen]`, converting arguments and results with `serde-wasm-bindgen` and turning errors into thrown JS `Error`s.

```mermaid
classDiagram
  direction LR
  class wasm {
    <<module>>
    +blankCourse()$ Course
    +evaluate(course: JsValue, targets: JsValue)$ Evaluation
    +summaries(courses: JsValue)$ CourseSummary[]
    +importCourses(data: JsValue)$ Course[]
    -to_js(value)$ JsValue
    -from_js(value, what)$ T
  }
  class api {
    <<module>>
    +NOT_A_COURSE: &str$
    +evaluate(course: &Course, targets: &[f64])$ Evaluation
    +summaries(courses: Vec~Course~)$ Vec~CourseSummary~
    +import_courses(data: Value)$ Result~Vec~Course~, &str~
  }
  class Evaluation {
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
  class CourseSummary {
    <<struct>>
    +percent?: f64
    +letter?: String
  }
  wasm ..> api : wraps
  api ..> Evaluation
  Evaluation *-- TargetNeed
  api ..> CourseSummary
```

All `wasm` functions return `Result<JsValue, JsError>`; the types shown are what the JavaScript side receives.

### Editing a score

```mermaid
sequenceDiagram
  actor User
  participant Editor as CourseEditor
  participant App as App / CourseView
  participant Engine as engine.ts → WebAssembly
  User->>Editor: types a score
  Editor->>App: mutate(draft => …)
  App->>App: new course state, saved to localStorage
  App->>Engine: evaluate(course, targets)
  Engine-->>App: { course (graded), remaining, needs }
  App->>Editor: graded course (matched by position)
  App->>Summary: evaluation
```

Grading happens synchronously during the render, in a few milliseconds even for courses with drop rules.

## 7. Front-end (`frontend/src/`)

```mermaid
classDiagram
  direction TB
  class App {
    <<component>>
    -state: SavedState
    -percents: Map~string, number or null~
    -mutate: Mutate
    -initialState()$ SavedState
  }
  class CourseView {
    <<component>>
    -custom: number or null
    -evaluation: Evaluation or error
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
  class engine {
    <<module>>
    +loadEngine()$ Promise~void~
    +blankCourse()$ Course
    +evaluate(course, targets)$ Evaluation
    +summaries(courses)$ CourseSummary[]
    +importCourses(data)$ Course[]
  }
  App *-- "0..1" CourseView : selected course
  CourseView *-- CourseEditor
  CourseView *-- Summary
  CourseEditor *-- ScaleEditor
  CourseEditor *-- "0..*" NumberInput
  ScaleEditor *-- "0..*" NumberInput
  Summary *-- NumberInput : custom target
  App ..> engine
  CourseView ..> engine
  App ..> course
  CourseView ..> course
  CourseEditor ..> course
  CourseEditor ..> category
  CourseEditor ..> item
  Summary ..> course
```

The model and its front-end logic mirror the Rust modules: one file per type, holding the interface and the functions that work on it. All grading happens in the engine; these helpers only build, edit and display courses.

```mermaid
classDiagram
  direction LR
  class course {
    <<module>>
    +Course, Scheme, Needed, CourseSummary, Mutate
    +decimals(course)$ number
    +letterTargets(course)$ Cutoff[]
    +targets(course, custom)$ number[]
    +totalWeight(course)$ number
    +EngineCourse
    +withIds(course)$ Course
    +updateCourse(courses, id, fn)$ Course[]
    +percentsById(courses, summaries)$ Map~string, number or null~
    +exportFileName(course)$ string
    +describeNeed(need)$ string
  }
  class category {
    <<module>>
    +Category
    +newCategory()$ Category
    +addItem(category)$ void
    +hasDropRules(category)$ boolean
    +EngineCategory
    +withCategoryIds(category)$ Category
  }
  class item {
    <<module>>
    +Item
    +percentOf(item)$ number or null
    +setEarnedPoints(item, earnedPoints)$ void
    +setExcluded(item, excluded)$ void
    +newItem(name, previous)$ Item
    +EngineItem
    +withItemId(item)$ Item
  }
  class scale {
    <<module>>
    +Scale
    +Grade
    +GRADES$: Grade[]
    +Cutoff
    +cutoffs(scale)$ Cutoff[]
  }
  class status {
    <<module>>
    +Status
    +forScore(status, earnedPoints)$ Status
    -ROW_CLASS: Record~Status, string~$
    +rowClass(status)$ string
  }
  course ..> category : Category
  course ..> scale : Scale, cutoffs
  category ..> item : Item, newItem
  item ..> status : Status, forScore
```

`main.tsx` awaits `loadEngine()` (which fetches and instantiates the `.wasm`) before rendering `App`, so every `engine` call after that is synchronous. `CourseView` is keyed by course id and grades its course with `useMemo` whenever the course or the custom target changes; if the engine throws (for example on a malformed course from storage), it shows an error banner instead. `Mutate` (in `course.ts`) is `(fn: (draft: Course) => void) => void`: `App` passes `fn` to `updateCourse`, which clones the selected course, lets `fn` edit the draft, and returns the new course list. `engine.ts` gives every course the engine creates new ids with `withIds`, and also holds the engine's other result type (`Evaluation`), mirroring `api.rs`; `CourseSummary` and `percentsById`, which turns summaries into each course's percentage for the course picker, are in `course.ts`. `engine.summaries` caches each course object's summary: courses are never edited in place, so after an edit only the edited course is sent to the engine again.
