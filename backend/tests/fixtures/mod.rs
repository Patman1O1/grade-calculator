//! The four courses in `docs/`, transcribed from each
//! `docs/<course>/grades-11-5-2026.pdf` and configured from its
//! `syllabus.pdf`. Test data only: `tests/courses.rs` checks that the engine
//! reproduces every total printed in those gradebooks.

#![allow(dead_code)] // each test file uses a different subset

use grade_calculator::category::Category;
use grade_calculator::course::standard_scale;
use grade_calculator::course::{Course, Cutoff, Scheme};
use grade_calculator::item::Item;

/// An item scored `score` out of `out_of` points.
fn item(name: &str, score: Option<f64>, out_of: f64) -> Item {
    Item {
        name: name.to_owned(),
        earned: score,
        possible: out_of,
        ..Default::default()
    }
}

/// A graded item.
fn g(name: &str, score: f64, out_of: f64) -> Item {
    item(name, Some(score), out_of)
}

/// An item that has not been graded yet.
fn u(name: &str, out_of: f64) -> Item {
    item(name, None, out_of)
}

/// Builder methods for the course data below.
trait ItemExt {
    fn with_worth(self, worth: f64) -> Self;
    fn exclude(self) -> Self;
    fn with_note(self, note: &str) -> Self;
}

impl ItemExt for Item {
    /// Rescale the item to count `worth` points (a 0–100 report worth 10
    /// points: 85/100 becomes 8.5/10), as the syllabus weights it.
    fn with_worth(mut self, worth: f64) -> Self {
        if self.possible > 0.0 {
            self.earned = self.earned.map(|e| e / self.possible * worth);
            self.possible = worth;
        }
        self
    }
    fn exclude(mut self) -> Self {
        self.excluded = true;
        self
    }
    fn with_note(mut self, note: &str) -> Self {
        self.note = Some(note.to_owned());
        self
    }
}

fn category(name: &str, weight: f64, items: Vec<Item>) -> Category {
    Category {
        name: name.to_owned(),
        weight,
        drop_lowest: None,
        items,
        ..Default::default()
    }
}

trait CategoryExt {
    fn drop_lowest(self, n: u32) -> Self;
}

impl CategoryExt for Category {
    fn drop_lowest(mut self, n: u32) -> Self {
        self.drop_lowest = Some(n.into());
        self
    }
}

fn scale(cutoffs: &[(&str, f64)]) -> Vec<Cutoff> {
    cutoffs
        .iter()
        .map(|(letter, min)| Cutoff {
            letter: (*letter).to_owned(),
            min: *min,
        })
        .collect()
}

fn course(
    name: &str,
    scheme: Scheme,
    decimals: u32,
    scale: Vec<Cutoff>,
    notes: &str,
    categories: Vec<Category>,
) -> Course {
    Course {
        name: name.to_owned(),
        scheme,
        categories,
        scale,
        display_decimals: Some(decimals),
        notes: Some(notes.to_owned()),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// CS 401 – Computer Algorithms (Canvas, total points)
// Syllabus: grade is purely total points; 360/320/280/200 of 410 guarantee A/B/C/D.
// 410 = 14 check-ins × 3 + 7 homeworks × 24 + 2 exams × 100.
// ---------------------------------------------------------------------------
pub fn cs401() -> Course {
    let pct = |points: f64| points / 410.0 * 100.0;
    let syllabus = "From syllabus (not yet on Canvas)";
    let mut check_ins = vec![
        g("Check-In 1", 2.0, 3.0),
        g("Check-In 3", 2.0, 3.0),
        g("Check-In 4", 3.0, 3.0),
        g("Check-In 5", 3.0, 3.0),
        g("Check-In 6", 0.0, 3.0),
    ];
    check_ins.extend((7..=14).map(|n| u(&format!("Check-In {n}"), 3.0)));
    course(
        "CS 401 – Computer Algorithms",
        Scheme::Points,
        2,
        scale(&[
            ("A", pct(360.0)),
            ("B", pct(320.0)),
            ("C", pct(280.0)),
            ("D", pct(200.0)),
            ("F", 0.0),
        ]),
        "Total-points course. Letter cutoffs are the syllabus point thresholds (360/320/280/200) expressed as a share of the 410 planned points. Late Days is tracked on Canvas but does not count toward the grade. Homework 6, Homework 7 and the Final Exam are planned in the syllabus but not yet on Canvas.",
        vec![
            category("Check-Ins", 0.0, check_ins),
            category(
                "Homework",
                0.0,
                vec![
                    u("Homework 0 (Optional)", 0.0),
                    g("Homework 1", 23.0, 24.0),
                    g("Homework 2", 23.5, 24.0),
                    g("Homework 3", 24.0, 24.0),
                    u("Homework 4", 24.0),
                    u("Homework 5", 24.0),
                    u("Homework 6", 24.0).with_note(syllabus),
                    u("Homework 7", 24.0).with_note(syllabus),
                ],
            ),
            category(
                "Assignments",
                0.0,
                vec![
                    g("Late Days", 4.0, 5.0)
                        .exclude()
                        .with_note("Late days remaining; not graded"),
                ],
            ),
            category(
                "Exams",
                0.0,
                vec![
                    g("Midterm Exam", 26.0, 100.0),
                    u("Final Exam", 100.0).with_note(syllabus),
                ],
            ),
        ],
    )
}

// ---------------------------------------------------------------------------
// CS 411 – Artificial Intelligence (Canvas, weighted groups)
// Syllabus: Quizzes 18% (drop 2), Labs 24% (drop 2), Exams 1–3 8% each,
// Final 10%, Projects 24%. Extra-credit group carries 0% on Canvas.
// ---------------------------------------------------------------------------
pub fn cs411() -> Course {
    let quizzes = [
        Some(10.0),
        Some(10.0),
        Some(3.0),
        Some(8.0),
        Some(8.0),
        None,
        None,
        None,
    ];
    let labs = [9.0, 10.0, 8.0, 10.0, 10.0, 10.0, 0.0, 7.0]
        .map(Some)
        .into_iter()
        .chain([None, None]);
    course(
        "CS 411 – Artificial Intelligence",
        Scheme::Weighted,
        2,
        standard_scale(),
        "Exams 1–3 are 8% each, so Exam 03 (out of 100 on Canvas) is entered as 50 points, like the other two exams; enter its score in points out of 50. Canvas pools exam points instead, which will diverge from the syllabus once Exam 03 is graded. The cumulative make-up exam (replaces one or two of Exams 1–3) is not modelled; edit the scores directly to try it. No letter scale in the syllabus, so 90/80/70/60 is used.",
        vec![
            category(
                "Quiz",
                18.0,
                quizzes
                    .iter()
                    .enumerate()
                    .map(|(i, s)| item(&format!("Quiz {:02}", i + 1), *s, 10.0))
                    .collect(),
            )
            .drop_lowest(2),
            category(
                "Labs",
                24.0,
                labs.enumerate()
                    .map(|(i, s)| item(&format!("Lab{:02}", i + 1), s, 10.0))
                    .collect(),
            )
            .drop_lowest(2),
            category(
                "Exams",
                24.0,
                vec![
                    g("Exam 01", 34.0, 50.0),
                    g("Exam 02", 17.0, 50.0),
                    u("Exam 03", 100.0)
                        .with_worth(50.0)
                        .with_note("Out of 100 on Canvas; counts 50 points, like Exams 01 and 02"),
                ],
            ),
            category("Final Exam", 10.0, vec![u("Final Exam", 100.0)]),
            category(
                "Projects",
                24.0,
                vec![
                    g("Project 01", 97.0, 100.0),
                    g("Project 02", 67.5, 100.0),
                    u("Project 03", 100.0),
                ],
            ),
            category(
                "Extra Credit Assignments",
                0.0,
                vec![
                    g("In-class-Activity-01", 1.0, 1.0),
                    g("In-class-Activity-02", 0.5, 0.5),
                    g("In-class-Activity-03", 1.5, 1.5),
                    g("In-class-Activity-04", 0.0, 1.0),
                    g("In-class-Activity-05", 0.9, 1.0),
                    g("In-class-Activity-06", 0.0, 1.0),
                    g("In-class-Activity-07", 3.0, 3.0),
                    u("In-class-Activity-08", 1.5),
                    u("In-class-Activity-09", 1.0),
                ],
            ),
            category("Make-up Project/Exam", 0.0, vec![]),
        ],
    )
}

// ---------------------------------------------------------------------------
// CS 440 – Intro to Software Engineering (Blackboard, scaled points)
// Syllabus: each component is worth a fixed number of points; PPP =
// points accumulated to date / possible points to date. Raw 0–100 report scores
// are scaled to their point value (e.g. Description Report 85/100 → 8.5 of 10).
// ---------------------------------------------------------------------------
pub fn cs440() -> Course {
    let later = "From syllabus (not yet graded)";
    course(
        "CS 440 – Intro to Software Engineering",
        Scheme::Points,
        1,
        standard_scale(),
        "Each Blackboard column is entered as its syllabus point value (a 0–100 report worth 10 points scoring 85 is 8.5 of 10). The 12 points for Weeklies, Reviews, Grammatical, OCL and Class Participation are all at full credit; the PDF pins down their total (67 possible) but not the exact split between Weeklies and Reviews, which doesn't change the grade. Attendance and the midterm/review sub-scores are informational only. Syllabus says letter breaks will be close to 90/80/70/60.",
        vec![
            category(
                "Individual Work",
                0.0,
                vec![
                    g("Attendance %", 100.0, 100.0)
                        .exclude()
                        .with_note("Informational"),
                    g("Midterm MC", 32.0, 100.0)
                        .exclude()
                        .with_note("Part of Midterm"),
                    g("Midterm Non MC", 19.0, 100.0)
                        .exclude()
                        .with_note("Part of Midterm"),
                    g("Midterm", 51.0, 100.0).with_worth(15.0),
                    u("Final Exam", 100.0).with_worth(20.0).with_note(later),
                    g("Brainstorming", 70.0, 100.0).with_worth(5.0),
                    g("Rev1", 5.0, 5.0).exclude().with_note("Part of Reviews"),
                    g("Rev2", 5.0, 5.0).exclude().with_note("Part of Reviews"),
                    g("Rev3", 5.0, 5.0).exclude().with_note("Part of Reviews"),
                    g("Reviews", 5.0, 5.0)
                        .with_worth(5.0)
                        .with_note("Presentation evaluations"),
                    g("Weeklies", 10.0, 10.0)
                        .with_worth(3.0)
                        .with_note("Weekly progress reports to date"),
                    g("Grammatical", 1.0, 1.0),
                    g("OCL", 1.0, 1.0),
                    g("Class Participation (2)", 2.0, 2.0),
                ],
            ),
            category(
                "Design Project",
                0.0,
                vec![
                    g("Description Report", 85.0, 100.0).with_worth(10.0),
                    g("Description Presentation", 4.2, 5.0),
                    u("Requirements Report", 100.0)
                        .with_worth(10.0)
                        .with_note(later),
                    u("Final Design Report", 100.0)
                        .with_worth(20.0)
                        .with_note(later),
                    u("Final Design Presentation", 5.0).with_note(later),
                ],
            ),
            category(
                "Implementation Project",
                0.0,
                vec![
                    g("First Scenario", 85.0, 100.0).with_worth(5.0),
                    g("Second Scenario", 89.0, 100.0).with_worth(5.0),
                    g("Demo 1", 80.0, 100.0).with_worth(10.0),
                    u("Demo 2", 100.0).with_worth(10.0).with_note(later),
                    u("Final Oral Presentation & Demo", 5.0).with_note(later),
                    u("Final Implementation Report", 100.0)
                        .with_worth(10.0)
                        .with_note(later),
                    u("CASE tool usage (Jira)", 5.0).with_note(later),
                ],
            ),
        ],
    )
}

// ---------------------------------------------------------------------------
// Math 218 – Applied Linear Algebra
// ---------------------------------------------------------------------------
fn math218_scale() -> Vec<Cutoff> {
    scale(&[
        ("A", 87.0),
        ("B", 75.0),
        ("C", 60.0),
        ("D", 45.0),
        ("F", 0.0),
    ])
}

fn math218_mom() -> Vec<Item> {
    vec![
        g("Homework 1 (Weeks 1-2)", 19.5, 20.0),
        g("Homework 2 (Weeks 3-4)", 19.8, 20.0),
        g("Homework 3 (Weeks 5-6)", 20.0, 20.0),
        g("Homework 4 (Weeks 7-8)", 18.6, 20.0),
        g("Homework 5 (Weeks 9-10)", 20.0, 20.0),
        u("Homework 6 (Weeks 11-12)", 20.0),
    ]
}

fn math218_written() -> Vec<Item> {
    vec![
        g("Written Homework #1", 18.0, 18.0),
        g("Written Homework #2", 16.0, 18.0),
        g("Written Homework #3", 18.0, 18.0),
        u("Written Homework #4", 18.0),
        u("Written Homework #5", 18.0),
        u("Written Homework #6", 18.0),
    ]
}

/// Syllabus policy: Online HW 15%, Written HW 15% (lowest of each dropped),
/// Exam #1 20%, Exam #2 20%, Final 30%. The exams share one 70% category with
/// items worth 20/20/30 points, which is exactly how the instructor's posted
/// "Midsemester Score" (65.39) was computed — see `tests/courses.rs`.
pub fn math218() -> Course {
    course(
        "Math 218 – Applied Linear Algebra",
        Scheme::Weighted,
        2,
        math218_scale(),
        "Weighted per the syllabus (Canvas itself is set to total points, which counts the 65.39/0 Midsemester Score as extra points and shows 118.28%). Exams form one 70% category with items worth 20/20/30 points, so missing exams are projected at your exam average — the same method behind the posted Midsemester Score. Good attendance adds 5 points to the final exam score.",
        vec![
            category("MOM Homework (online)", 15.0, math218_mom()).drop_lowest(1),
            category("Written Homework", 15.0, math218_written()).drop_lowest(1),
            category(
                "Exams",
                70.0,
                vec![
                    g("Exam #1", 26.0, 50.0).with_worth(20.0),
                    u("Exam #2", 50.0).with_worth(20.0),
                    u("Final Exam", 100.0)
                        .with_worth(30.0)
                        .with_note("Worth 30 points; enter your score in points out of 30"),
                ],
            ),
            category(
                "Midsemester Grade",
                0.0,
                vec![
                    g("Midsemester Score", 65.39, 0.0)
                        .exclude()
                        .with_note("Instructor-posted snapshot"),
                ],
            ),
        ],
    )
}

/// Math 218 exactly as its Canvas gradebook is configured (unweighted total points).
pub fn math218_canvas() -> Course {
    course(
        "Math 218 – as Canvas computes it",
        Scheme::Points,
        2,
        math218_scale(),
        "Mirror of the Canvas gradebook setup: no weights, no drops, and the 0-point Midsemester Score is added as extra points. This reproduces the 118.28% Canvas shows, which is not a real course grade; see the syllabus-weighted Math 218 course for that.",
        vec![
            category("Written Homework group", 0.0, math218_written()),
            category("Assignments", 0.0, vec![]),
            category("MOM Homework group", 0.0, math218_mom()),
            category("Midterm Exams", 0.0, vec![g("Exam #1", 26.0, 50.0)]),
            category(
                "Midsemester Grade",
                0.0,
                vec![g("Midsemester Score", 65.39, 0.0)],
            ),
        ],
    )
}
