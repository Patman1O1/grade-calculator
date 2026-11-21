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

/// One graded thing, as `earned` out of `possible` points.
///
/// Its points are what it contributes to its category, so they also set how
/// much it counts there: a 10-point quiz counts a tenth as much as a 100-point
/// exam. A score entered as a percentage is stored as `percent / 100`.
///
/// Deserialising also accepts the older `score` / `outOf` / `worth` form and
/// converts it (see `RawItem`), so saved courses and old exports still load.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "RawItem")]
pub struct Item {
    pub id: String,
    pub name: String,
    /// Points earned, or `None` when the item has not been graded yet.
    pub earned: Option<f64>,
    /// Points possible. 0 makes the item pure extra credit: its `earned`
    /// points are added on top of the category.
    pub possible: f64,
    /// Not counted toward the grade at all (Canvas "does not count toward final grade").
    #[serde(skip_serializing_if = "is_false")]
    pub excluded: bool,
    /// Never removed by drop-lowest / drop-highest rules.
    #[serde(skip_serializing_if = "is_false")]
    pub never_drop: bool,
    /// Free-form note shown in the UI (e.g. due date).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    /// Computed when the item's category is graded (see the crate docs):
    /// whether the item counts toward its category's grade, and if not, why not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
}

/// What an item looks like in JSON: the current `earned` / `possible` form,
/// or the older `score` / `outOf` / `worth` form. A computed `status` in the
/// input is ignored.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawItem {
    #[serde(default)]
    id: String,
    name: String,
    #[serde(default)]
    earned: Option<f64>,
    #[serde(default)]
    possible: Option<f64>,
    /// Older form: raw score out of `outOf`, optionally rescaled to `worth`.
    #[serde(default)]
    score: Option<f64>,
    #[serde(default)]
    out_of: Option<f64>,
    #[serde(default)]
    worth: Option<f64>,
    #[serde(default)]
    excluded: bool,
    #[serde(default)]
    never_drop: bool,
    #[serde(default)]
    note: Option<String>,
}

impl TryFrom<RawItem> for Item {
    type Error = String;

    fn try_from(raw: RawItem) -> Result<Self, Self::Error> {
        let (earned, possible) = match (raw.out_of, raw.possible) {
            // Older form. Converts exactly as the old engine scored it:
            // score / outOf * worth, out of worth (raw score for extra credit).
            (Some(out_of), _) => {
                let worth = raw.worth.unwrap_or(out_of);
                if out_of > 0.0 {
                    (raw.score.map(|s| s / out_of * worth), worth)
                } else {
                    (raw.score, 0.0)
                }
            }
            (None, Some(possible)) => (raw.earned, possible),
            (None, None) => return Err(format!("item {:?} has no points possible", raw.name)),
        };
        Ok(Item {
            id: raw.id,
            name: raw.name,
            earned,
            possible,
            excluded: raw.excluded,
            never_drop: raw.never_drop,
            note: raw.note,
            status: None,
        })
    }
}

impl Item {
    /// Ungraded, not excluded, and still has points available.
    pub fn is_remaining(&self) -> bool {
        self.earned.is_none() && !self.excluded && self.possible > 0.0
    }

    /// `earned / possible` as a percentage; `None` if ungraded or extra credit.
    pub fn percent(&self) -> Option<f64> {
        let earned = self.earned?;
        (self.possible > 0.0).then(|| earned / self.possible * 100.0)
    }

    /// Grade the item on its own, before any drop rule: sets `status` to
    /// `Counted`, `Excluded` or `Ungraded`. [`crate::Category::grade`] calls
    /// this for every item and then applies its drop rules, which can change
    /// `Counted` to `Dropped`.
    pub fn grade(&mut self) {
        self.status = Some(if self.excluded {
            Status::Excluded
        } else if self.earned.is_none() {
            Status::Ungraded
        } else {
            Status::Counted
        });
    }

    /// Part of its category's grade (as of the last `grade`).
    pub fn is_counted(&self) -> bool {
        self.status == Some(Status::Counted)
    }

    /// `(earned, possible)` this item adds to its category when counted.
    pub(crate) fn points(&self) -> (f64, f64) {
        (self.earned.unwrap_or(0.0), self.possible)
    }

    /// Mark a counted item as removed by a drop rule.
    pub(crate) fn drop_from_grade(&mut self) {
        self.status = Some(Status::Dropped);
    }
}

#[cfg(test)]
mod tests;
