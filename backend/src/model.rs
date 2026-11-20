//! Data model for a course gradebook.
//!
//! The model is deliberately small but general enough to express the grading
//! policies used by Canvas, Blackboard and typical syllabi:
//!
//!  - An [`Item`] is one graded thing (a quiz, an exam, a lab). Its raw result is
//!    `score / out_of`. `worth` says how much the item counts inside its category
//!    (defaults to `out_of`, i.e. plain points). Blackboard-style "this 0–100
//!    report is worth 10 course points" is `out_of: 100, worth: 10`; a syllabus
//!    rule like "each exam counts equally" gives the items equal worth regardless
//!    of their raw point totals.
//!  - A [`Category`] groups items, may drop the lowest/highest results, and
//!    carries a weight when the course is weighted.
//!  - A [`Course`] combines categories either by total points or by weights.
//!
//! Everything serialises as camelCase JSON, the same shape the front-end stores
//! and that older JSON exports of the TypeScript app used, so those still import.
//!
//! The grade itself lives on the same types: [`crate::engine::compute_course`]
//! fills in each item's `counted`/`dropped`/`earned`/`possible`, each category's
//! `earned`/`possible`/`percent`, and the course's `earned`/`possible`/`percent`/
//! `letter`. These computed fields are `None` (and left out of the JSON) until
//! the course is graded, and they are ignored when a course is deserialised, so
//! a client can never send in results; they are always recomputed.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    /// Sum of points earned / sum of points possible across all categories.
    Points,
    /// Each category's percentage multiplied by its weight. Categories with no
    /// graded work are left out and the remaining weights are renormalised
    /// (what Canvas and most syllabi do mid-semester).
    #[default]
    Weighted,
}

fn is_false(b: &bool) -> bool {
    !*b
}

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

    // Computed by the engine (see the module docs).
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

    /// Ungraded, counted, and still has points available.
    pub fn is_remaining(&self) -> bool {
        self.score.is_none() && !self.excluded && self.out_of > 0.0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// Relative weight; only used by the weighted scheme.
    pub weight: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_lowest: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_highest: Option<f64>,
    /// Weighted scheme only: the category's weighted score is added on top of
    /// the grade, but its weight is not part of the denominator.
    #[serde(default, skip_serializing_if = "is_false")]
    pub extra_credit: bool,
    pub items: Vec<Item>,

    // Computed by the engine (see the module docs).
    /// Sum over counted items.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub earned: Option<f64>,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub possible: Option<f64>,
    /// earned / possible as a percentage; stays `None` when nothing is graded.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cutoff {
    pub letter: String,
    /// Minimum percentage (0–100) needed for this letter.
    pub min: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub scheme: Scheme,
    pub categories: Vec<Category>,
    /// Letter-grade cutoffs; order does not matter.
    pub scale: Vec<Cutoff>,
    /// Decimal places the course's own gradebook shows for percentages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_decimals: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    // Computed by the engine (see the module docs).
    /// Points scheme: totals across categories. Weighted scheme: weighted points.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub earned: Option<f64>,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub possible: Option<f64>,
    /// Overall percentage; stays `None` when nothing is graded.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub letter: Option<String>,
}

impl Course {
    /// Every ungraded, non-excluded item that still has points available.
    pub fn remaining_count(&self) -> usize {
        self.categories
            .iter()
            .flat_map(|c| &c.items)
            .filter(|i| i.is_remaining())
            .count()
    }
}

/// Average needed on the remaining work to reach a target.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Needed {
    NoRemaining,
    Secured,
    Impossible { best: Option<f64> },
    Needs { fraction: f64 },
}
