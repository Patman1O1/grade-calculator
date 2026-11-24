//! [`Status`]: whether an item counts toward its category's grade, and if
//! not, why not.

use serde::{Deserialize, Serialize};

/// Whether an item counts toward its category's grade, and if not, why not.
///
/// Only `Excluded` is the user's choice (the "Excl." checkbox); the engine
/// keeps it as given. The other three follow from the item's score and its
/// category's drop rules, so grading always recomputes them, whatever was
/// stored or sent in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Graded and part of the category's grade.
    Counted,
    /// Graded, but removed by a drop-lowest / drop-highest rule.
    Dropped,
    /// Marked as not counting toward the grade, graded or not.
    Excluded,
    /// No score yet.
    #[default]
    Ungraded,
}

impl Status {
    /// The status an item has before any drop rule: `Excluded` stays
    /// `Excluded`; otherwise `Counted` if it has a score, `Ungraded` if not.
    pub fn for_score(self, earned_points: Option<f64>) -> Status {
        match (self, earned_points) {
            (Status::Excluded, _) => Status::Excluded,
            (_, None) => Status::Ungraded,
            (_, Some(_)) => Status::Counted,
        }
    }

    /// Part of its category's grade.
    pub fn counts(self) -> bool {
        self == Status::Counted
    }
}

#[cfg(test)]
mod tests;
