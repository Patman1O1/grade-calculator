# grade-calculator

A course grade calculator that handles the grading schemes used by Canvas, Blackboard and typical syllabi. You can use it for any course: enter the categories and items, then see your current grade, letter, per-category breakdown, and the average you need on the remaining work for each letter.

```sh
npm install
npm run dev      # app at http://localhost:5173
npm test         # checks the calculator against the PDFs in docs/
npm run build
```

Courses are saved in the browser (localStorage) and can be exported or imported as JSON.

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

The engine lives in [`src/grade/engine.ts`](src/grade/engine.ts) and has no UI dependencies.

A C11 command-line version of the same engine lives in [`c/`](c/README.md). It reads the course JSON the app exports, and a differential test checks that it matches the TypeScript engine bit for bit:

```sh
cd c && make && build/grade -n examples/cs411.json
```

## Verification against `docs/`

[`src/grade/courses.ts`](src/grade/courses.ts) has the four courses in `docs/`, transcribed from each `grades-11-5-2026.pdf` and configured from each `syllabus.pdf`. [`src/grade/courses.test.ts`](src/grade/courses.test.ts) checks that the calculator reproduces every total and group row printed in those PDFs:

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
