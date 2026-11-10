# grade — C command-line version

A C11 port of the grading engine in [`src/grade`](../src/grade). It reads the same course JSON the web app exports, and it produces the same numbers as the TypeScript engine, bit for bit. It has no dependencies beyond the C standard library and libm.

```sh
cd c
make            # builds build/grade
make test       # C tests: the gradebook PDF figures + engine rules
build/grade examples/cs411.json
```

## Usage

```
grade [options] [FILE...]

  -j, --json          print one JSON object per course instead of text
  -s, --summary       text output: categories only, without items
  -n, --needed        show the average needed on remaining work for each letter
  -t, --target PCT    show the average needed to finish with at least PCT percent (repeatable)
  -h, --help
  -V, --version
```

`FILE` holds one course object or an array of them. With no `FILE`, or with `-`, the tool reads standard input. The exit status is 0 on success, 1 if any input was invalid (the message names the field, e.g. `categories[0].items[2].outOf: missing`), and 2 on usage errors.

```
$ build/grade -s -n examples/cs411.json
CS 411 – Artificial Intelligence  [weighted categories]

  Quiz (weight 18, drop lowest 2)                                28.00 / 30.00    93.33%
  Labs (weight 24, drop lowest 2)                                57.00 / 60.00       95%
  Exams (weight 24)                                             51.00 / 100.00       51%
  ...
  Current grade: 79.53% (C)
  Based on 90 of 100 category weight graded so far

  Average needed on the 10 remaining items to finish with:
    A  >= 90%         out of reach (max 87.82%)
    B  >= 80%         80.9% avg
    ...
```

To get a course into the tool, use the web app's **Export** button. `examples/*.json` are the app's built-in examples in the same format, and they can be imported back into the app.

### JSON output

`--json` prints one object per course per line (JSON Lines), so it works with `jq`:

```json
{"name":"…","scheme":"weighted","earned":71.58,"possible":90,"percent":79.533…,"letter":"C","remaining":10,
 "categories":[{"name":"Quiz","weight":18,"earned":28,"possible":30,"percent":93.33…,
   "items":[{"name":"Quiz 01","counted":true,"dropped":false,"excluded":false,"earned":10,"possible":10}, …]}, …],
 "needed":[{"letter":"A","target":90,"kind":"impossible","best":87.82…},
           {"letter":null,"target":85,"kind":"needs","fraction":0.931…}]}
```

Numbers are unrounded and print the shortest form that reads back exactly. `percent` is `null` when nothing is graded. `needed` appears only with `-n` or `-t`. Its `kind` is one of `no-remaining`, `secured`, `impossible` (with `best`) or `needs` (with `fraction`, from 0 to 1).

## Using the engine as a library

`include/grade.h` plus `src/grade.c` make up the whole engine. It is pure computation, with no I/O and no global state. The JSON layer (`src/json.c`, `src/course_json.c`) is optional.

```c
#include "grade.h"

grade_course_result r;
if (grade_compute_course(&course, &r) == 0) {
    if (r.has_percent)
        printf("%.2f%% %s\n", grade_round(r.percent, 2), r.letter ? r.letter : "");
    grade_course_result_free(&r);
}
```

| TypeScript (`engine.ts`) | C (`grade.h`) |
|---|---|
| `computeCourse` / `computeCategory` | `grade_compute_course` / `grade_compute_category` |
| `neededOnRemaining` | `grade_needed_on_remaining` |
| `letterFor`, `itemWorth`, `round` | `grade_letter_for`, `grade_item_worth`, `grade_round` |
| `score: number \| null` | `bool graded` + `double score` |
| `percent: number \| null` | `bool has_percent` + `double percent` |
| `chooseKept` (private) | `choose_kept` (static) |

## How it matches the TypeScript engine

- **Same operations, same order.** Sums, products and divisions follow `engine.ts` operation for operation, so the doubles come out identical.
- **No fused multiply-add.** `-ffp-contract=off` (set in the Makefile) stops the compiler from turning `a*b + c` into an FMA instruction, which JavaScript never does. Don't build with `-ffast-math`.
- **Rounding.** `grade_round` reproduces JavaScript's `Number(x.toFixed(10))`, rounding exact ties away from zero. glibc's `printf` would round them to even.
- **Drop rules.** These use the same exact Canvas-style search (bisection on the resulting ratio), with the same tie-breaking.

`make difftest` runs about 5,000 random courses (ties, extra credit, mixed worths, never-drop items, odd scales) plus the examples through both engines. It requires every number, flag and letter to be identical, and reports how many hard cases it covered. `make sanitize` runs the tests and the difftest under AddressSanitizer and UBSan.

## Differences from the web app's import

The tool is slightly stricter than the app. Optional fields must have the right JSON type if present: `worth` and `dropLowest` must be numbers, `excluded`, `neverDrop` and `extraCredit` must be booleans, and any of them may be `null`. Numbers must be finite. `\u0000` inside strings is rejected.

## Files

```
include/grade.h          engine API (model, results, functions)
src/grade.c              engine
src/json.[ch]            minimal strict JSON parser/writer
src/course_json.[ch]     course JSON -> structs, results -> JSON
src/main.c               command-line tool
tests/test_grade.c       C tests (PDF figures + engine rules)
examples/*.json          example courses, exported from src/grade/courses.ts
tools/export-examples.mjs  regenerates examples/ (make examples)
tools/difftest.mjs       TypeScript vs C differential test (make difftest)
```
