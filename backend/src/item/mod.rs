//! [`Item`]: one graded thing (a quiz, an exam, a lab).

use serde::{Deserialize, Serialize};

use crate::status::Status;
use crate::utils::is_false;

/// One graded thing, as `earned_points` out of `total_points`.
///
/// Its points are what it contributes to its category, so they also set how
/// much it counts there: a 10-point quiz counts a tenth as much as a 100-point
/// exam.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub name: String,
    /// Points earned, or `None` when the item has not been graded yet.
    #[serde(default)]
    pub earned_points: Option<f64>,
    /// Points the item is out of. 0 makes the item pure extra credit: its
    /// `earned_points` are added on top of the category.
    pub total_points: f64,
    /// Never removed by drop-lowest / drop-highest rules.
    #[serde(default, skip_serializing_if = "is_false")]
    pub never_drop: bool,
    /// Free-form note shown in the UI (e.g. due date).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Whether the item counts toward its category's grade, and if not, why
    /// not. `Excluded` is the user's choice and is kept; [`Item::grade`]
    /// recomputes the other three from the score (and the category's drop
    /// rules then set `Dropped`), so a status read from input is never
    /// trusted except `Excluded`.
    #[serde(default)]
    pub status: Status,
}

impl Item {
    /// Marked as not counting toward the grade.
    pub fn is_excluded(&self) -> bool {
        self.status == Status::Excluded
    }

    /// Ungraded, not excluded, and still has points available.
    pub fn is_remaining(&self) -> bool {
        self.earned_points.is_none() && !self.is_excluded() && self.total_points > 0.0
    }

    /// `earned_points / total_points` as a percentage; `None` if ungraded or
    /// extra credit.
    pub fn percent(&self) -> Option<f64> {
        let earned = self.earned_points?;
        (self.total_points > 0.0).then(|| earned / self.total_points * 100.0)
    }

    /// Grade the item on its own, before any drop rule: an excluded item
    /// stays `Excluded`; any other becomes `Counted` or `Ungraded` from its
    /// score. [`crate::category::Category::grade`] calls this for every item
    /// and then applies its drop rules, which can change `Counted` to `Dropped`.
    pub fn grade(&mut self) {
        self.status = self.status.for_score(self.earned_points);
    }

    /// Part of its category's grade (as of the last `grade`).
    pub fn is_counted(&self) -> bool {
        self.status.counts()
    }

    /// `(earned, possible)` points this item adds to its category when counted.
    pub(crate) fn points(&self) -> (f64, f64) {
        (self.earned_points.unwrap_or(0.0), self.total_points)
    }

    /// Mark a counted item as removed by a drop rule.
    pub(crate) fn drop_from_grade(&mut self) {
        self.status = Status::Dropped;
    }
}

#[cfg(test)]
mod tests;
