//! Every expected value below is copied from docs/<course>/grades-11-5-2026.pdf
//! (and rounded the same way the gradebook displays it).

mod fixtures;

use fixtures::{cs401, cs411, cs440, math218, math218_canvas};
use grade_calculator::course::Course;
use grade_calculator::status::Status;
use grade_calculator::utils::round;

/// `(percent, earned, possible)` of a category, rounded to 2 places.
fn category(r: &Course, name: &str) -> (Option<f64>, f64, f64) {
    let c = r
        .categories
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no category {name}"));
    (
        c.percent.map(|p| round(p, 2)),
        round(c.earned.unwrap(), 2),
        round(c.possible.unwrap(), 2),
    )
}

#[test]
fn cs401_canvas_total_points() {
    let r = cs401().graded();
    // Total 56.95% (106.50 / 187.00)
    assert_eq!(round(r.percent.unwrap(), 2), 56.95);
    assert_eq!(round(r.earned.unwrap(), 2), 106.5);
    assert_eq!(round(r.possible.unwrap(), 2), 187.0);

    assert_eq!(category(&r, "Homework"), (Some(97.92), 70.5, 72.0));
    assert_eq!(category(&r, "Check-Ins"), (Some(66.67), 10.0, 15.0));
    assert_eq!(category(&r, "Exams"), (Some(26.0), 26.0, 100.0));
    // Late Days does not count: Canvas shows "N/A 0.00 / 0.00".
    assert_eq!(category(&r, "Assignments"), (None, 0.0, 0.0));
}

#[test]
fn cs411_canvas_weighted_drop_lowest_two() {
    let r = cs411().graded();
    // Total 79.53%
    assert_eq!(round(r.percent.unwrap(), 2), 79.53);

    assert_eq!(
        category(&r, "Extra Credit Assignments"),
        (Some(76.67), 6.9, 9.0)
    );
    assert_eq!(category(&r, "Quiz"), (Some(93.33), 28.0, 30.0));
    assert_eq!(category(&r, "Projects"), (Some(82.25), 164.5, 200.0));
    assert_eq!(category(&r, "Labs"), (Some(95.0), 57.0, 60.0));
    assert_eq!(category(&r, "Exams"), (Some(51.0), 51.0, 100.0));
    assert_eq!(category(&r, "Final Exam").0, None);
    assert_eq!(category(&r, "Make-up Project/Exam").0, None);

    // Drops the two lowest quizzes and labs. Quiz 04 and Quiz 05 are both 8/10,
    // so dropping either one is equivalent.
    let mut dropped: Vec<&str> = r
        .categories
        .iter()
        .flat_map(|c| &c.items)
        .filter(|i| i.status == Status::Dropped)
        .map(|i| i.name.as_str())
        .collect();
    dropped.sort();
    let tied = |d: &&str| *d == "Quiz 04" || *d == "Quiz 05";
    assert_eq!(
        dropped
            .iter()
            .filter(|d| !tied(d))
            .copied()
            .collect::<Vec<_>>(),
        ["Lab07", "Lab08", "Quiz 03"]
    );
    assert_eq!(dropped.iter().filter(|d| tied(d)).count(), 1);
}

#[test]
fn cs440_blackboard_scaled_points() {
    let r = cs440().graded();
    // Total (out of 67) 52.6
    assert_eq!(round(r.possible.unwrap(), 1), 67.0);
    assert_eq!(round(r.earned.unwrap(), 1), 52.6);
    // PPP (Based on % of 67) 78.4
    assert_eq!(round(r.percent.unwrap(), 1), 78.4);
}

#[test]
fn math218_canvas_total_points() {
    let r = math218_canvas().graded();
    // Total 118.28% (241.29 / 204.00)
    assert_eq!(round(r.percent.unwrap(), 2), 118.28);
    assert_eq!(round(r.earned.unwrap(), 2), 241.29);
    assert_eq!(round(r.possible.unwrap(), 2), 204.0);
    assert_eq!(
        category(&r, "Written Homework group"),
        (Some(96.3), 52.0, 54.0)
    );
    assert_eq!(
        category(&r, "MOM Homework group"),
        (Some(97.9), 97.9, 100.0)
    );
    assert_eq!(category(&r, "Midterm Exams"), (Some(52.0), 26.0, 50.0));
    assert_eq!(category(&r, "Assignments").0, None);
    // "Midsemester Score 65.39 / 0" → group row "N/A 65.39 / 0.00"
    assert_eq!(category(&r, "Midsemester Grade"), (None, 65.39, 0.0));
}

#[test]
fn math218_syllabus_weighting_reproduces_posted_midsemester_score() {
    // At mid-semester (after Exam #1 on Oct 5) the graded work was MOM HW 1–3
    // and Written HW 1–2, and no drops had been applied yet.
    let mut course = math218();
    let graded_later = [
        "Homework 4 (Weeks 7-8)",
        "Homework 5 (Weeks 9-10)",
        "Written Homework #3",
    ];
    for c in &mut course.categories {
        c.drop_lowest = None;
        for i in &mut c.items {
            if graded_later.contains(&i.name.as_str()) {
                i.earned_points = None;
            }
        }
    }
    assert_eq!(round(course.graded().percent.unwrap(), 2), 65.39);
}

#[test]
fn math218_syllabus_weighting_today() {
    let r = math218().graded();
    // MOM: drop 18.6 → 79.3/80; Written: drop 16 → 36/36; Exams: 52%.
    assert_eq!(category(&r, "MOM Homework (online)").0, Some(99.13));
    assert_eq!(category(&r, "Written Homework").0, Some(100.0));
    assert_eq!(category(&r, "Exams").0, Some(52.0));
    assert_eq!(
        round(r.percent.unwrap(), 2),
        round(15.0 * 0.99125 + 15.0 * 1.0 + 70.0 * 0.52, 2)
    );
    assert_eq!(r.letter.as_deref(), Some("C"));
}
