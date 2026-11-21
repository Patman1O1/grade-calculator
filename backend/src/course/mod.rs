//! [`Course`]: categories combined by total points or by weights, with a
//! letter scale and the "what do I need?" solver.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::category::Category;
use crate::ids::new_id;
use crate::item::Item;

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

/// One letter of a course's scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cutoff {
    pub letter: String,
    /// Minimum percentage (0–100) needed for this letter.
    pub min: f64,
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

/// The usual 90/80/70/60 letter scale.
pub fn standard_scale() -> Vec<Cutoff> {
    [
        ("A", 90.0),
        ("B", 80.0),
        ("C", 70.0),
        ("D", 60.0),
        ("F", 0.0),
    ]
    .into_iter()
    .map(|(letter, min)| Cutoff {
        letter: letter.to_owned(),
        min,
    })
    .collect()
}

impl Course {
    /// Starting point for a new course: weighted, a Homework and an Exams
    /// category with one ungraded item each, and the standard letter scale.
    pub fn blank() -> Course {
        let category = |name: &str, weight: f64, item: &str, possible: f64| Category {
            id: new_id("cat"),
            name: name.to_owned(),
            weight,
            items: vec![Item {
                id: new_id("item"),
                name: item.to_owned(),
                earned: None,
                possible,
                ..Default::default()
            }],
            ..Default::default()
        };
        Course {
            id: new_id("course"),
            name: "New course".to_owned(),
            scheme: Scheme::Weighted,
            categories: vec![
                category("Homework", 40.0, "Homework 1", 10.0),
                category("Exams", 60.0, "Midterm", 100.0),
            ],
            scale: standard_scale(),
            display_decimals: Some(2),
            ..Default::default()
        }
    }

    /// The same course with fresh ids for it and everything in it, so a copy
    /// (an import or a duplicate) never collides with the original.
    pub fn with_fresh_ids(mut self) -> Course {
        self.id = new_id("course");
        for c in &mut self.categories {
            c.id = new_id("cat");
            for i in &mut c.items {
                i.id = new_id("item");
            }
        }
        self
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

    /// Letter for a percentage on this course's scale: the highest cutoff it
    /// reaches, or the lowest letter. `None` without a percentage or a scale.
    pub fn letter_for(&self, percent: Option<f64>) -> Option<String> {
        let percent = percent?;
        let mut sorted: Vec<&Cutoff> = self.scale.iter().collect();
        sorted.sort_by(|a, b| b.min.partial_cmp(&a.min).unwrap_or(Ordering::Equal));
        // Small tolerance so 89.99999999 from floating point still earns the 90 cutoff.
        sorted
            .iter()
            .find(|c| percent + 1e-9 >= c.min)
            .or(sorted.last())
            .map(|c| c.letter.clone())
    }

    /// Ungraded, non-excluded items that still have points available.
    pub fn remaining_count(&self) -> usize {
        self.categories
            .iter()
            .flat_map(|c| &c.items)
            .filter(|i| i.is_remaining())
            .count()
    }

    /// Copy of the course with every remaining item scored at `fraction` of its points.
    pub fn with_remaining_at(&self, fraction: f64) -> Course {
        let mut c = self.clone();
        for item in c.categories.iter_mut().flat_map(|c| c.items.iter_mut()) {
            if item.is_remaining() {
                item.earned = Some(item.possible * fraction);
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
