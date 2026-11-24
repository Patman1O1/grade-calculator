// ── Aliases ─────────────────────────────────────────────────────────────────
use serde::{Deserialize, Serialize};

use crate::{status::Status, utils::is_false};

// ── `struct Item` ───────────────────────────────────────────────────────────
/// One graded thing, as `earned` out of `possible` points.
///
/// Its points are what it contributes to its category, so they also set how
/// much it counts there: a 10-point quiz counts a tenth as much as a 100-point
/// exam.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// The name of the item.
    pub name: String,

    /// Points earned, or `None` when the item has not been graded yet.
    #[serde(default)]
    pub earned_points: Option<f64>,

    /// Points possible. 0 makes the item pure extra credit: its `earned`
    /// points are added on top of the category.
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

// ── `Item` Implementation ───────────────────────────────────────────────────
impl Item {
    // ── Functions ───────────────────────────────────────────────────────────
    pub fn new(
        name: impl Into<String>,
        earned_points: Option<f64>,
        total_points: f64,
        never_drop: bool,
        note: Option<String>,
        status: Status,
    ) -> Self {
        Self {
            name: name.into(),
            earned_points,
            total_points,
            never_drop,
            note,
            status,
        }
    }

    // ── Methods ─────────────────────────────────────────────────────────────
    #[inline]
    pub const fn drop(&mut self) -> bool {
        if !self.never_drop {
            self.status = Status::Dropped;
            true
        } else {
            false
        }
    }

    #[inline]
    pub const fn exclude(&mut self) {
        self.status = Status::Excluded;
    }

    #[inline]
    pub const fn ungrade(&mut self) -> (Option<f64>, f64) {
        let old_points = (self.earned_points, self.total_points);

        self.earned_points = None;
        self.total_points = 0.0;
        self.status = Status::Ungraded;

        old_points
    }

    #[inline]
    pub const fn grade(&mut self, earned_points: f64, total_points: f64) {
        self.earned_points = Some(earned_points);
        self.total_points = total_points;
        self.status = Status::Counted;
    }

    #[inline]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// `earned / possible` as a percentage; `None` if ungraded or extra credit.
    pub const fn percent(&self) -> Option<f64> {
        match self.earned_points {
            Some(earned) if self.total_points > 0.0 => Some(earned / self.total_points * 100.0),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
