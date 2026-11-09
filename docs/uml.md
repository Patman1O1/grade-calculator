# UML diagrams

These class diagrams cover every type, interface, union and module in `src/`. The code is functional TypeScript and has no `class` declarations, so the diagrams use these conventions:

| Notation | Meaning in the code |
|---|---|
| `«interface»` | a TypeScript `interface` |
| `«enumeration»` | a string-literal union used as an enum (`Scheme`) |
| `«union»` / `«variant»` | a discriminated union and its members, told apart by `kind` |
| `«type»` | a `type` alias |
| `«module»` | a source file; its top-level functions and constants appear as static (underlined) members |
| `«component»` | a React function component; its `useState` values appear as attributes |
| `+` / `-` | exported / module-private |
| `?` | optional property |

## 1. Module overview

```mermaid
flowchart LR
  subgraph grade["src/grade"]
    types["types.ts<br/>«data model»"]
    engine["engine.ts<br/>«module»"]
    courses["courses.ts<br/>«module»"]
  end
  subgraph components["src/components"]
    CourseEditor["CourseEditor.tsx"]
    Summary["Summary.tsx"]
    NumberInput["NumberInput.tsx"]
  end
  subgraph tests["tests (vitest)"]
    engineTest["engine.test.ts"]
    coursesTest["courses.test.ts"]
  end
  main["main.tsx"] -->|renders| App["App.tsx"]
  storage["storage.ts<br/>«module»"]

  engine -->|imports| types
  courses -->|imports| types
  storage -->|imports| types
  App -->|imports| engine
  App -->|imports| courses
  App -->|imports| storage
  App -->|renders| CourseEditor
  App -->|renders| Summary
  CourseEditor -->|imports| engine
  CourseEditor -->|newId| courses
  CourseEditor -->|renders| NumberInput
  Summary -->|imports| engine
  Summary -->|renders| NumberInput
  engineTest -.->|tests| engine
  coursesTest -.->|tests| engine
  coursesTest -.->|tests| courses
```

## 2. Domain model (`src/grade/types.ts`)

A `Course` owns its categories and letter scale, and each `Category` owns its items.

```mermaid
classDiagram
  direction LR
  class Scheme {
    <<enumeration>>
    points
    weighted
  }
  class Course {
    <<interface>>
    +id: string
    +name: string
    +scheme: Scheme
    +categories: Category[]
    +scale: Cutoff[]
    +displayDecimals?: number
    +notes?: string
  }
  class Category {
    <<interface>>
    +id: string
    +name: string
    +weight: number
    +dropLowest?: number
    +dropHighest?: number
    +extraCredit?: boolean
    +items: Item[]
  }
  class Item {
    <<interface>>
    +id: string
    +name: string
    +score: number or null
    +outOf: number
    +worth?: number
    +excluded?: boolean
    +neverDrop?: boolean
    +note?: string
  }
  class Cutoff {
    <<interface>>
    +letter: string
    +min: number
  }
  Course "1" *-- "0..*" Category : categories
  Category "1" *-- "0..*" Item : items
  Course "1" *-- "0..*" Cutoff : scale
  Course --> "1" Scheme : scheme
```

## 3. Results (`types.ts` and `engine.ts`)

The engine returns a result tree that mirrors the course tree. Each result node points back to the input node it was computed from. `NeededResult` is the discriminated union that `neededOnRemaining` returns.

```mermaid
classDiagram
  direction LR
  class CourseResult {
    <<interface>>
    +course: Course
    +categories: CategoryResult[]
    +earned: number
    +possible: number
    +percent: number or null
    +letter: string or null
  }
  class CategoryResult {
    <<interface>>
    +category: Category
    +items: ItemResult[]
    +earned: number
    +possible: number
    +percent: number or null
  }
  class ItemResult {
    <<interface>>
    +item: Item
    +counted: boolean
    +dropped: boolean
    +earned: number
    +possible: number
  }
  class Course {
    <<interface>>
  }
  class Category {
    <<interface>>
  }
  class Item {
    <<interface>>
  }
  class NeededResult {
    <<union>>
    +kind: string
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
    +best: number or null
  }
  class Needs {
    <<variant>>
    +kind = "needs"
    +fraction: number
  }
  CourseResult "1" *-- "0..*" CategoryResult : categories
  CategoryResult "1" *-- "0..*" ItemResult : items
  CourseResult --> "1" Course : course
  CategoryResult --> "1" Category : category
  ItemResult --> "1" Item : item
  NeededResult <|-- NoRemaining
  NeededResult <|-- Secured
  NeededResult <|-- Impossible
  NeededResult <|-- Needs
```

## 4. Grading engine (`src/grade/engine.ts`)

```mermaid
classDiagram
  direction LR
  class engine {
    <<module>>
    +itemWorth(item: Item)$ number
    -contribution(item: Item, score: number)$ EarnedPossible
    -chooseKept(candidates: Candidate[], keep: number, fixedEarned: number, fixedPossible: number, mode: max or min)$ Set~number~
    +computeCategory(category: Category)$ CategoryResult
    +letterFor(percent: number or null, scale: Cutoff[])$ string or null
    +computeCourse(course: Course)$ CourseResult
    -remainingItems(course: Course)$ Item[]
    +withRemainingAt(course: Course, fraction: number)$ Course
    +neededOnRemaining(course: Course, target: number)$ NeededResult
    +round(x: number, decimals: number)$ number
    +formatPercent(x: number or null, decimals = 2)$ string
    +formatNumber(x: number, decimals = 2)$ string
  }
  class Candidate {
    <<interface>>
    +index: number
    +earned: number
    +possible: number
  }
  class Course {
    <<interface>>
  }
  class Category {
    <<interface>>
  }
  class CourseResult {
    <<interface>>
  }
  class CategoryResult {
    <<interface>>
  }
  class NeededResult {
    <<union>>
  }
  engine ..> Candidate : drop search
  engine ..> Course : reads
  engine ..> Category : reads
  engine ..> CourseResult : creates
  engine ..> CategoryResult : creates
  engine ..> NeededResult : returns
```

`Candidate` is module-private. `chooseKept` works on candidates for the Canvas-exact drop-lowest/highest search. `EarnedPossible` stands for the inline return type `{ earned: number; possible: number }`.

## 5. Course data and storage (`src/grade/courses.ts`, `src/storage.ts`)

```mermaid
classDiagram
  direction LR
  class courses {
    <<module>>
    -nextId: number$
    +STANDARD_SCALE: Cutoff[]$
    -MATH218_SCALE: Cutoff[]$
    +EXAMPLES: Example[]$
    +newId(prefix = "id")$ string
    -item(name: string, score: number or null, outOf: number, opts: ItemOpts)$ Item
    -category(name: string, weight: number, items: Item[], opts)$ Category
    +cs401()$ Course
    +cs411()$ Course
    +cs440()$ Course
    +math218()$ Course
    +math218Canvas()$ Course
    -math218Mom()$ Item[]
    -math218Written()$ Item[]
    +blankCourse()$ Course
  }
  class ItemOpts {
    <<type>>
    +worth?: number
    +excluded?: boolean
    +neverDrop?: boolean
    +note?: string
  }
  class Example {
    <<type>>
    +key: string
    +label: string
    +build() Course
  }
  class storage {
    <<module>>
    -KEY: string = "grade-calculator:v1"$
    -isNum(x: unknown)$ boolean
    -isItem(x: unknown)$ boolean
    -isCategory(x: unknown)$ boolean
    +isCourse(x: unknown)$ boolean
    +loadState()$ SavedState or null
    +saveState(state: SavedState)$ void
  }
  class SavedState {
    <<interface>>
    +courses: Course[]
    +selectedId: string or null
  }
  class Course {
    <<interface>>
  }
  class Item {
    <<interface>>
  }
  class Cutoff {
    <<interface>>
  }
  courses "1" *-- "5" Example : EXAMPLES
  courses ..> ItemOpts : uses
  courses ..> Course : builds
  courses ..> Cutoff : defines scales
  ItemOpts ..> Item : Partial of Pick of Item
  Example ..> Course : build()
  storage ..> SavedState : loads and saves
  SavedState "1" o-- "0..*" Course : courses
```

`ItemOpts` is `Partial<Pick<Item, 'worth' | 'excluded' | 'neverDrop' | 'note'>>`. `Example` is the inline element type of `EXAMPLES`. The `is*` guards are TypeScript type predicates (`x is Course`, and so on).

## 6. UI components (`src/App.tsx`, `src/components/`)

### 6a. Component tree

```mermaid
classDiagram
  direction TB
  class App {
    <<component>>
    -state: SavedState
    -fileInput: Ref~HTMLInputElement~
    -mutate: Mutate
    -initialState()$ SavedState
    -reId(course: Course)$ Course
    -select(id: string) void
    -addCourse(c: Course) void
    -removeCourse() void
    -exportCourse() void
    -importCourse(file: File) Promise~void~
  }
  class CourseEditor {
    <<component>>
  }
  class ScaleEditor {
    <<component>>
  }
  class Summary {
    <<component>>
    -custom: number or null
    -describeNeed(course: Course, target: number)$ string
  }
  class NumberInput {
    <<component>>
    -text: string
    -prev: number or null
    -parse(s: string)$ number or null
  }
  App *-- "1" CourseEditor : renders
  App *-- "1" Summary : renders
  CourseEditor *-- "1" ScaleEditor : renders
  CourseEditor *-- "0..*" NumberInput : renders
  ScaleEditor *-- "0..*" NumberInput : renders
  Summary *-- "1" NumberInput : custom target
```

`App` keeps the `SavedState` in `useState`. `initialState`, `reId`, `describeNeed` and `parse` are module-level helpers in their component's file, so they are drawn as static members.

### 6b. Props and the `Mutate` callback

```mermaid
classDiagram
  direction LR
  class App {
    <<component>>
  }
  class CourseEditor {
    <<component>>
  }
  class ScaleEditor {
    <<component>>
  }
  class Summary {
    <<component>>
  }
  class NumberInput {
    <<component>>
  }
  class CourseEditorProps {
    <<interface>>
    +course: Course
    +result: CourseResult
    +mutate: Mutate
  }
  class ScaleEditorProps {
    <<type>>
    +course: Course
    +mutate: Mutate
  }
  class SummaryProps {
    <<interface>>
    +course: Course
    +result: CourseResult
  }
  class NumberInputProps {
    <<interface>>
    +value: number or null
    +nullable?: boolean
    +placeholder?: string
    +min?: number
    +className?: string
    +aria-label?: string
    +onChange(value: number or null) void
  }
  class Mutate {
    <<type>>
    +call(fn: Course to void) void
  }
  CourseEditor ..> CourseEditorProps : props
  ScaleEditor ..> ScaleEditorProps : props
  Summary ..> SummaryProps : props
  NumberInput ..> NumberInputProps : props
  CourseEditorProps --> Mutate : mutate
  ScaleEditorProps --> Mutate : mutate
  App ..> Mutate : provides
```

In the source, all three props interfaces are module-private and named `Props`. They are renamed here to tell them apart. `ScaleEditor` (private to `CourseEditor.tsx`) takes an inline props type. `Mutate` is `(fn: (draft: Course) => void) => void`: App clones the selected course, lets `fn` edit the draft, and stores the result.
