//! [`Course`]: categories combined by total points or by weights, with a
//! letter scale and the "what do I need?" solver.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::category::Category;
use crate::item::Item;
use crate::scale::Scale;
use crate::status::Status;

/// How a course combines its categories.
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

/// Average needed on the remaining work to reach a target.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Needed {
    /// Nothing left to grade.
    NoRemaining,
    /// Reached even with zeros on everything left.
    Secured,
    /// Out of reach even with 100% on everything left; `best` is that 100% grade.
    Impossible { best: Option<f64> },
    /// Average needed on the remaining work, as a fraction of its points.
    Needs { fraction: f64 },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub name: String,
    pub scheme: Scheme,
    pub categories: Vec<Category>,
    /// Letter-grade cutoffs.
    pub scale: Scale,
    /// Decimal places the course's own gradebook shows for percentages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_decimals: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    // Computed by `grade` (see the crate docs).
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

/// Ungraded, not excluded, and still has points available: the work
/// "what do I need?" solves for.
fn is_remaining(item: &Item) -> bool {
    item.earned_points.is_none() && !item.status.is_excluded() && item.total_points > 0.0
}

impl Course {
    /// Starting point for a new course: weighted, a Homework and an Exams
    /// category with one ungraded item each, and the standard letter scale.
    pub fn blank() -> Course {
        let category = |name: &str, weight: f64, item: &str, total_points: f64| Category {
            name: name.to_owned(),
            weight,
            items: vec![Item::new(
                item,
                None,
                total_points,
                false,
                None,
                Status::Ungraded,
            )],
            ..Default::default()
        };
        Course {
            name: "New course".to_owned(),
            scheme: Scheme::Weighted,
            categories: vec![
                category("Homework", 40.0, "Homework 1", 10.0),
                category("Exams", 60.0, "Midterm", 100.0),
            ],
            scale: Scale::STANDARD,
            display_decimals: Some(2),
            ..Default::default()
        }
    }

    /// Grade the course in place: grades every category (and item), then fills
    /// in the course's `earned`, `possible`, `percent` and `letter`.
    pub fn grade(&mut self) {
        self.categories.iter_mut().for_each(Category::grade);
        let mut earned = 0.0;
        let mut possible = 0.0;

        match self.scheme {
            Scheme::Points => {
                for c in &self.categories {
                    earned += c.earned.unwrap_or(0.0);
                    possible += c.possible.unwrap_or(0.0);
                }
            }
            Scheme::Weighted => {
                for c in &self.categories {
                    let w = c.weight;
                    let Some(percent) = c.percent else { continue };
                    // Skips zero, negative and NaN weights alike.
                    if w.partial_cmp(&0.0) != Some(Ordering::Greater) {
                        continue;
                    }
                    earned += w * percent / 100.0;
                    if !c.extra_credit {
                        possible += w;
                    }
                }
            }
        }

        let percent = (possible > 0.0).then(|| earned / possible * 100.0);
        self.earned = Some(earned);
        self.possible = Some(possible);
        self.percent = percent;
        self.letter = self.letter_for(percent);
    }

    /// A graded copy of the course.
    pub fn graded(&self) -> Course {
        let mut c = self.clone();
        c.grade();
        c
    }

    /// Letter for a percentage on this course's scale (see
    /// [`Scale::letter_for`]). `None` without a percentage.
    pub fn letter_for(&self, percent: Option<f64>) -> Option<String> {
        self.scale.letter_for(percent).map(str::to_owned)
    }

    /// Ungraded, non-excluded items that still have points available.
    pub fn remaining_count(&self) -> usize {
        self.categories
            .iter()
            .flat_map(|c| &c.items)
            .filter(|i| is_remaining(i))
            .count()
    }

    /// Copy of the course with every remaining item scored at `fraction` of its points.
    pub fn with_remaining_at(&self, fraction: f64) -> Course {
        let mut c = self.clone();
        for item in c.categories.iter_mut().flat_map(|c| c.items.iter_mut()) {
            if is_remaining(item) {
                item.earned_points = Some(item.total_points * fraction);
            }
        }
        c
    }

    /// Average score (as a fraction of points) needed on every remaining item
    /// to finish at or above `target` percent. "Impossible" means even 100%
    /// everywhere falls short.
    pub fn needed_on_remaining(&self, target: f64) -> Needed {
        if self.remaining_count() == 0 {
            return Needed::NoRemaining;
        }
        let at = |f: f64| self.with_remaining_at(f).graded().percent.unwrap_or(0.0);
        if at(0.0) + 1e-9 >= target {
            return Needed::Secured;
        }
        let best = at(1.0);
        if best + 1e-9 < target {
            return Needed::Impossible { best: Some(best) };
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..60 {
            let mid = (lo + hi) / 2.0;
            if at(mid) + 1e-9 >= target {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        Needed::Needs { fraction: hi }
    }
}

#[cfg(test)]
mod tests;
