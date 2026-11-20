//! [`Item`]: one graded thing (a quiz, an exam, a lab).

use serde::{Deserialize, Serialize};

use crate::is_false;

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
    /// Item is graded, not excluded and not dropped.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub counted: Option<bool>,
    /// Removed by a drop-lowest / drop-highest rule.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub dropped: Option<bool>,
    /// Contribution to the category: score / outOf * worth (raw score for extra credit).
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

    /// Grade the item on its own, before any drop rule: fills in `counted`,
    /// `dropped` (false), `earned` and `possible`. [`crate::Category::grade`]
    /// calls this for every item and then applies its drop rules.
    pub fn grade(&mut self) {
        let (counted, (earned, possible)) = match self.score {
            Some(score) if !self.excluded => (true, self.contribution(score)),
            _ => (false, (0.0, 0.0)),
        };
        self.counted = Some(counted);
        self.dropped = Some(false);
        self.earned = Some(earned);
        self.possible = Some(possible);
    }

    /// Graded and not excluded: a candidate for counting or dropping.
    pub(crate) fn is_counted(&self) -> bool {
        self.counted == Some(true)
    }

    /// Mark the item as removed by a drop rule.
    pub(crate) fn drop_from_grade(&mut self) {
        self.counted = Some(false);
        self.dropped = Some(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(score: Option<f64>, out_of: f64) -> Item {
        Item {
            score,
            out_of,
            ..Default::default()
        }
    }

    #[test]
    fn scales_a_raw_score_to_its_worth() {
        let mut i = item(Some(85.0), 100.0);
        i.worth = Some(10.0);
        i.grade();
        assert!((i.earned.unwrap() - 8.5).abs() < 1e-9);
        assert_eq!(i.possible, Some(10.0));
        assert_eq!(i.counted, Some(true));
    }

    #[test]
    fn zero_point_items_are_extra_credit() {
        let mut i = item(Some(2.0), 0.0);
        i.grade();
        assert_eq!((i.earned, i.possible), (Some(2.0), Some(0.0)));
    }

    #[test]
    fn ungraded_and_excluded_items_do_not_count() {
        let mut ungraded = item(None, 10.0);
        ungraded.grade();
        assert_eq!(ungraded.counted, Some(false));
        assert!(ungraded.is_remaining());

        let mut excluded = item(Some(5.0), 10.0);
        excluded.excluded = true;
        excluded.grade();
        assert_eq!(
            (excluded.counted, excluded.earned),
            (Some(false), Some(0.0))
        );
        assert!(!excluded.is_remaining());
    }
}
