//! The operations the front-end needs, as plain Rust. [`crate::wasm`] exposes
//! them to JavaScript; keeping them here means they are tested natively.

use serde::Serialize;
use serde_json::Value;

use crate::examples::{EXAMPLES, re_id};
use crate::{Course, Needed};

/// A target percentage and what it takes to reach it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetNeed {
    pub target: f64,
    pub needed: Needed,
}

/// Everything the editor shows for one course.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evaluation {
    /// The course with its computed fields (grade, category and item results) filled in.
    pub course: Course,
    /// Ungraded items that still have points available.
    pub remaining: usize,
    /// One entry per requested target, in the same order.
    pub needs: Vec<TargetNeed>,
}

/// Grade a course and solve "what do I need?" for each target percentage.
pub fn evaluate(course: &Course, targets: &[f64]) -> Evaluation {
    Evaluation {
        course: course.graded(),
        remaining: course.remaining_count(),
        needs: targets
            .iter()
            .map(|&target| TargetNeed {
                target,
                needed: course.needed_on_remaining(target),
            })
            .collect(),
    }
}

/// A course's overall grade, for the course picker.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseSummary {
    pub id: String,
    pub percent: Option<f64>,
    pub letter: Option<String>,
}

/// Grade every course (for the course picker).
pub fn summaries(courses: Vec<Course>) -> Vec<CourseSummary> {
    courses
        .into_iter()
        .map(|mut c| {
            c.grade();
            CourseSummary {
                id: c.id,
                percent: c.percent,
                letter: c.letter,
            }
        })
        .collect()
}

/// An example course the front-end can load by key.
#[derive(Debug, Serialize)]
pub struct ExampleInfo {
    pub key: &'static str,
    pub label: &'static str,
}

pub fn examples() -> Vec<ExampleInfo> {
    EXAMPLES
        .iter()
        .map(|e| ExampleInfo {
            key: e.key,
            label: e.label,
        })
        .collect()
}

pub const NOT_A_COURSE: &str = "That file is not a course exported from this calculator.";

/// Validate exported JSON (one course or a list of them) and give every
/// course fresh ids so it can be added next to existing ones. Entries that
/// aren't courses are skipped; it is an error if none are. Also used to
/// duplicate a course.
pub fn import_courses(data: Value) -> Result<Vec<Course>, &'static str> {
    let list = match data {
        Value::Array(list) => list,
        other => vec![other],
    };
    let courses: Vec<Course> = list
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Course>(v).ok())
        .map(re_id)
        .collect();
    if courses.is_empty() {
        Err(NOT_A_COURSE)
    } else {
        Ok(courses)
    }
}
