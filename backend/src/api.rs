//! The operations the front-end needs, as plain Rust. [`crate::wasm`] exposes
//! them to JavaScript; keeping them here means they are tested natively.

use serde_json::Value;

use crate::category::Category;
use crate::course::Course;

/// The course graded, with "what do I need?" solved for its targets (see
/// [`Course::evaluate`]).
pub fn evaluate(course: &Course) -> Course {
    course.evaluated()
}

/// Each course's overall percentage (for the course picker), in the same
/// order; `None` when nothing is graded.
pub fn summaries(courses: Vec<Course>) -> Vec<Option<f64>> {
    courses
        .into_iter()
        .map(|mut c| {
            c.grade();
            c.percent
        })
        .collect()
}

pub const NOT_A_COURSE: &str = "That file is not a course exported from this calculator.";

/// Validate exported JSON (one course or a list of them). Entries that
/// aren't courses are skipped; it is an error if none are.
pub fn import_courses(data: Value) -> Result<Vec<Course>, &'static str> {
    let courses = parse_courses(data);
    if courses.is_empty() {
        Err(NOT_A_COURSE)
    } else {
        Ok(courses)
    }
}

/// The courses in `data` (one course or a list), skipping anything that
/// isn't one. Each item's status is recomputed from its score, so only an
/// `"excluded"` status is taken from the input.
fn parse_courses(data: Value) -> Vec<Course> {
    let list = match data {
        Value::Array(list) => list,
        other => vec![other],
    };
    list.into_iter()
        .filter_map(|v| serde_json::from_value::<Course>(v).ok())
        .map(|mut course| {
            course
                .categories
                .iter_mut()
                .for_each(Category::reset_statuses);
            course
        })
        .collect()
}
