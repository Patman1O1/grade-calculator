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

    /// Average score (as a fraction of points) needed on every remaining item
    /// to finish at or above `target` percent. "Impossible" means even 100%
    /// everywhere falls short.
    pub fn needed_on_remaining(&self, target: f64) -> Needed {
        let mut needs = self.needs(&[target]);
        needs.pop().unwrap_or(Needed::NoRemaining)
    }

    /// [`Course::needed_on_remaining`] for several targets at once, sharing
    /// the work between them. One result per target, in the same order.
    pub fn needs(&self, targets: &[f64]) -> Vec<Needed> {
        let Some(mut solver) = Solver::new(self) else {
            return vec![Needed::NoRemaining; targets.len()];
        };
        let at_zero = solver.percent_at(0.0);
        let at_full = solver.percent_at(1.0);
        targets
            .iter()
            .map(|&target| {
                // The tolerance lets 89.99999999 from floating point count as 90.
                let goal = target - TOLERANCE;
                if at_zero >= goal {
                    Needed::Secured
                } else if at_full < goal {
                    Needed::Impossible {
                        best: Some(at_full),
                    }
                } else if solver.linear {
                    Needed::Needs {
                        fraction: ((goal - at_zero) / (at_full - at_zero)).clamp(0.0, 1.0),
                    }
                } else {
                    // The smallest fraction that reaches the goal (the grade
                    // never falls as the fraction rises).
                    let (mut lo, mut hi) = (0.0, 1.0);
                    for _ in 0..BISECTION_STEPS {
                        let mid = (lo + hi) / 2.0;
                        if solver.percent_at(mid) >= goal {
                            hi = mid;
                        } else {
                            lo = mid;
                        }
                    }
                    Needed::Needs { fraction: hi }
                }
            })
            .collect()
    }
}

/// How far below a target a grade may fall and still count as reaching it.
const TOLERANCE: f64 = 1e-9;

/// Halvings of [0, 1] when solving with drop rules: 2⁻⁴⁰ ≈ 1e-12, far below
/// anything displayed.
const BISECTION_STEPS: u32 = 40;

/// The course's grade as a function of the fraction scored on the remaining
/// work, for [`Course::needs`]. Only the categories with remaining work change
/// with the fraction, so only they are regraded.
struct Solver {
    /// Working copy of the course, every category graded.
    course: Course,
    /// Each category with remaining work, and the indexes of those items.
    affected: Vec<(usize, Vec<usize>)>,
    /// No affected category has drop rules, so the grade is linear in the
    /// fraction: each affected category's percentage is
    /// `(earned + fraction · remaining) / possible` with a fixed `possible`,
    /// and the set of graded categories doesn't change.
    linear: bool,
}

impl Solver {
    /// `None` when nothing is left to grade.
    fn new(course: &Course) -> Option<Solver> {
        let affected: Vec<(usize, Vec<usize>)> = course
            .categories
            .iter()
            .enumerate()
            .filter_map(|(c, category)| {
                let items: Vec<usize> = (category.items.iter().enumerate())
                    .filter(|(_, item)| is_remaining(item))
                    .map(|(i, _)| i)
                    .collect();
                (!items.is_empty()).then_some((c, items))
            })
            .collect();
        if affected.is_empty() {
            return None;
        }
        let linear = !affected
            .iter()
            .any(|&(c, _)| course.categories[c].has_drop_rules());
        let mut course = course.clone();
        course.categories.iter_mut().for_each(Category::grade);
        Some(Solver {
            course,
            affected,
            linear,
        })
    }

    /// Course percentage (0 when undefined) with every remaining item scored at
    /// `fraction` of its points.
    fn percent_at(&mut self, fraction: f64) -> f64 {
        for (c, items) in &self.affected {
            let category = &mut self.course.categories[*c];
            for &i in items {
                let item = &mut category.items[i];
                item.earned_points = Some(item.total_points * fraction);
            }
            category.grade();
        }
        let (earned, possible) = self.course.totals();
        if possible > 0.0 {
            earned / possible * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests;
