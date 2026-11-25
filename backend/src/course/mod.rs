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
    /// A target percentage of the user's own for "what do I need?", solved
    /// after the letter cutoffs (see [`Course::targets`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,

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

    // Computed by `evaluate` only ("what do I need?").
    /// Number of items still to be graded (see [`Course::remaining_count`]).
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub remaining: Option<usize>,
    /// The grade with full marks on all remaining work: the best still
    /// possible. The current grade when nothing remains; `None` if undefined.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub best: Option<f64>,
    /// One entry per target in [`Course::targets`], in that order: the
    /// average score needed on the remaining work, as a fraction of its
    /// points (`0` when the target is already secured), or `None` when it is
    /// out of reach even with full marks (see [`Course::needs`]).
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub needs: Option<Vec<Option<f64>>>,
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
        let (earned, possible) = self.totals();
        let percent = (possible > 0.0).then(|| earned / possible * 100.0);
        self.earned = Some(earned);
        self.possible = Some(possible);
        self.percent = percent;
        self.letter = self.letter_for(percent);
    }

    /// The course's `(earned, possible)` from its already-graded categories.
    fn totals(&self) -> (f64, f64) {
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
        (earned, possible)
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

    /// The targets "what do I need?" solves for: each letter's cutoff above 0,
    /// highest first, then the course's own `target` if it has one.
    pub fn targets(&self) -> Vec<f64> {
        let mut targets: Vec<f64> = (self.scale.cutoffs().into_iter())
            .map(|(_, min)| min)
            .filter(|&min| min > 0.0)
            .collect();
        targets.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));
        targets.extend(self.target);
        targets
    }

    /// Grade the course in place (see [`Course::grade`]) and also solve "what
    /// do I need?": fills in `remaining`, `best` and `needs` for
    /// [`Course::targets`].
    pub fn evaluate(&mut self) {
        let (best, needs) = self.solve(&self.targets());
        self.remaining = Some(self.remaining_count());
        self.best = best;
        self.needs = Some(needs);
        self.grade();
    }

    /// An evaluated copy of the course (see [`Course::evaluate`]).
    pub fn evaluated(&self) -> Course {
        let mut c = self.clone();
        c.evaluate();
        c
    }

    /// Average score (as a fraction of points) needed on every remaining item
    /// to finish at or above `target` percent: `Some(0.0)` when it's already
    /// secured, `None` when even full marks fall short.
    pub fn needed_on_remaining(&self, target: f64) -> Option<f64> {
        self.needs(&[target]).pop().flatten()
    }

    /// [`Course::needed_on_remaining`] for several targets at once, sharing
    /// the work between them. One result per target, in the same order.
    pub fn needs(&self, targets: &[f64]) -> Vec<Option<f64>> {
        self.solve(targets).1
    }

    /// The best grade still possible, and [`Course::needs`] for `targets`.
    ///
    /// Only the categories with remaining work change with the score given
    /// to it, so the solver grades one working copy of the course and then
    /// regrades only those. If none of them has drop rules, the grade is a
    /// straight line in that score (each such category's percentage is
    /// `(earned + fraction · remaining) / possible`, with a fixed `possible`
    /// and a fixed set of graded categories), so it's solved exactly from the
    /// grades at 0% and 100%. Otherwise it bisects; the grade never falls as
    /// the score rises.
    fn solve(&self, targets: &[f64]) -> (Option<f64>, Vec<Option<f64>>) {
        // Each category with remaining work, and the indexes of those items.
        let affected: Vec<(usize, Vec<usize>)> = (self.categories.iter().enumerate())
            .filter_map(|(c, category)| {
                let items: Vec<usize> = (category.items.iter().enumerate())
                    .filter(|(_, item)| is_remaining(item))
                    .map(|(i, _)| i)
                    .collect();
                (!items.is_empty()).then_some((c, items))
            })
            .collect();

        let mut course = self.clone();
        course.categories.iter_mut().for_each(Category::grade);
        let linear = !(affected.iter()).any(|&(c, _)| course.categories[c].has_drop_rules());
        // Course percentage with every remaining item at `fraction` of its points.
        let mut percent_at = |fraction: f64| -> Option<f64> {
            for (c, items) in &affected {
                let category = &mut course.categories[*c];
                for &i in items {
                    let item = &mut category.items[i];
                    item.earned_points = Some(item.total_points * fraction);
                }
                category.grade();
            }
            let (earned, possible) = course.totals();
            (possible > 0.0).then(|| earned / possible * 100.0)
        };

        let at_zero = percent_at(0.0).unwrap_or(0.0);
        let best = percent_at(1.0);
        let at_full = best.unwrap_or(0.0);
        let mut needs = Vec::with_capacity(targets.len());
        for &target in targets {
            // The tolerance lets 89.99999999 from floating point count as 90.
            let goal = target - TOLERANCE;
            needs.push(if at_zero >= goal {
                Some(0.0)
            } else if at_full < goal {
                None
            } else if linear {
                Some(((goal - at_zero) / (at_full - at_zero)).clamp(0.0, 1.0))
            } else {
                // The smallest fraction that reaches the goal.
                let (mut lo, mut hi) = (0.0, 1.0);
                for _ in 0..BISECTION_STEPS {
                    let mid = (lo + hi) / 2.0;
                    if percent_at(mid).unwrap_or(0.0) >= goal {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                Some(hi)
            });
        }
        (best, needs)
    }
}

/// How far below a target a grade may fall and still count as reaching it.
const TOLERANCE: f64 = 1e-9;

/// Halvings of [0, 1] when solving with drop rules: 2⁻⁴⁰ ≈ 1e-12, far below
/// anything displayed.
const BISECTION_STEPS: u32 = 40;

#[cfg(test)]
mod tests;
