//! [`Item`]: one graded thing (a quiz, an exam, a lab).

use serde::{Deserialize, Serialize};

use crate::is_false;

/// Whether an item counts toward its category's grade, and if not, why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Graded and part of the category's grade.
    Counted,
    /// Graded, but removed by a drop-lowest / drop-highest rule.
    Dropped,
    /// Marked as not counting toward the grade.
    Excluded,
    /// No score yet.
    Ungraded,
}

/// One graded thing. Its raw result is `score / out_of`; `worth` says how much
/// it counts inside its category (defaults to `out_of`, i.e. plain points).
///
/// Blackboard-style "this 0–100 report is worth 10 course points" is
/// `out_of: 100, worth: 10`. A syllabus rule like "each exam counts equally"
/// gives the items equal worth regardless of their raw point totals.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// Raw score, or `None` when the item has not been graded yet.
    pub score: Option<f64>,
    /// Raw points possible. 0 makes the item pure extra credit.
    pub out_of: f64,
    /// How much the item counts inside its category. Defaults to `out_of`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worth: Option<f64>,
    /// Not counted toward the grade at all (Canvas "does not count toward final grade").
    #[serde(default, skip_serializing_if = "is_false")]
    pub excluded: bool,
    /// Never removed by drop-lowest / drop-highest rules.
    #[serde(default, skip_serializing_if = "is_false")]
    pub never_drop: bool,
    /// Free-form note shown in the UI (e.g. due date).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    // Computed when the item's category is graded (see the crate docs).
    /// Whether the item counts toward its category's grade, and if not, why not.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    /// Contribution to the category: score / outOf * worth (raw score for
    /// extra credit). 0 unless the status is `Counted` or `Dropped`.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub earned: Option<f64>,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub possible: Option<f64>,
}

impl Item {
    /// Effective worth of the item inside its category.
    pub fn worth(&self) -> f64 {
        self.worth.unwrap_or(self.out_of)
    }

    /// Ungraded, not excluded, and still has points available.
    pub fn is_remaining(&self) -> bool {
        self.score.is_none() && !self.excluded && self.out_of > 0.0
    }

    /// Points the item contributes to its category for a given score:
    /// `(earned, possible)`.
    pub fn contribution(&self, score: f64) -> (f64, f64) {
        let worth = self.worth();
        if self.out_of > 0.0 {
            (score / self.out_of * worth, worth)
        } else {
            // Zero-point items are pure extra credit: the raw score is added on top.
            (score, 0.0)
        }
    }

    /// Grade the item on its own, before any drop rule: fills in `status`
    /// (`Counted`, `Excluded` or `Ungraded`), `earned` and `possible`.
    /// [`crate::Category::grade`] calls this for every item and then applies
    /// its drop rules, which can change `Counted` to `Dropped`.
    pub fn grade(&mut self) {
        let (status, (earned, possible)) = match self.score {
            _ if self.excluded => (Status::Excluded, (0.0, 0.0)),
            None => (Status::Ungraded, (0.0, 0.0)),
            Some(score) => (Status::Counted, self.contribution(score)),
        };
        self.status = Some(status);
        self.earned = Some(earned);
        self.possible = Some(possible);
    }

    /// Part of its category's grade (as of the last `grade`).
    pub fn is_counted(&self) -> bool {
        self.status == Some(Status::Counted)
    }

    /// Mark a counted item as removed by a drop rule. It keeps its `earned`
    /// and `possible`, so the UI can still show what the dropped score was.
    pub(crate) fn drop_from_grade(&mut self) {
        self.status = Some(Status::Dropped);
    }
}

#[cfg(test)]
mod tests;
