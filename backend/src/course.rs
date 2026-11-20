//! [`Course`]: categories combined by total points or by weights, with a
//! letter scale and the "what do I need?" solver.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::category::Category;

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

impl Course {
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
                item.score = Some(item.out_of * fraction);
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
mod tests {
    use super::*;
    use crate::examples::standard_scale;
    use crate::item::Item;
    use crate::round;

    fn item(score: Option<f64>, out_of: f64) -> Item {
        Item {
            score,
            out_of,
            ..Default::default()
        }
    }

    fn cat(weight: f64, items: Vec<Item>) -> Category {
        Category {
            weight,
            items,
            ..Default::default()
        }
    }

    fn course(scheme: Scheme, categories: Vec<Category>) -> Course {
        Course {
            id: "x".into(),
            name: "x".into(),
            scheme,
            categories,
            scale: standard_scale(),
            ..Default::default()
        }
    }

    #[test]
    fn points_scheme_sums_every_category() {
        let r = course(
            Scheme::Points,
            vec![
                cat(0.0, vec![item(Some(9.0), 10.0)]),
                cat(0.0, vec![item(Some(70.0), 100.0)]),
            ],
        )
        .graded();
        assert_eq!((r.earned, r.possible), (Some(79.0), Some(110.0)));
        assert_eq!(r.letter.as_deref(), Some("C"));
    }

    #[test]
    fn weighted_renormalises_over_categories_with_graded_work() {
        let r = course(
            Scheme::Weighted,
            vec![
                cat(30.0, vec![item(Some(9.0), 10.0)]),
                cat(20.0, vec![item(Some(70.0), 100.0)]),
                cat(50.0, vec![item(None, 100.0)]),
            ],
        )
        .graded();
        assert_eq!(
            round(r.percent.unwrap(), 4),
            round((30.0 * 0.9 + 20.0 * 0.7) / 50.0 * 100.0, 4)
        );
    }

    #[test]
    fn extra_credit_categories_add_without_raising_the_denominator() {
        let mut ec = cat(5.0, vec![item(Some(1.0), 1.0)]);
        ec.extra_credit = true;
        let r = course(
            Scheme::Weighted,
            vec![cat(100.0, vec![item(Some(80.0), 100.0)]), ec],
        )
        .graded();
        assert!((r.percent.unwrap() - 85.0).abs() < 1e-9);
    }

    #[test]
    fn nothing_graded_has_no_percent_or_letter() {
        let r = course(Scheme::Points, vec![cat(0.0, vec![item(None, 10.0)])]).graded();
        assert_eq!((r.percent, r.letter), (None, None));
        assert_eq!(r.earned, Some(0.0));
    }

    #[test]
    fn maps_percentages_to_letters() {
        let c = course(Scheme::Points, vec![]);
        assert_eq!(c.letter_for(Some(89.99)).as_deref(), Some("B"));
        assert_eq!(c.letter_for(Some(90.0)).as_deref(), Some("A"));
        assert_eq!(c.letter_for(Some(-5.0)).as_deref(), Some("F"));
        assert_eq!(c.letter_for(None), None);
        let no_scale = Course::default();
        assert_eq!(no_scale.letter_for(Some(50.0)), None);
    }

    #[test]
    fn finds_the_average_needed_on_remaining_work() {
        // 60/100 so far, 100 points left; 80% overall needs 100/100.
        let c = course(
            Scheme::Points,
            vec![cat(0.0, vec![item(Some(60.0), 100.0), item(None, 100.0)])],
        );
        match c.needed_on_remaining(80.0) {
            Needed::Needs { fraction } => assert!((fraction - 1.0).abs() < 1e-6),
            other => panic!("expected needs, got {other:?}"),
        }
        assert!(matches!(c.needed_on_remaining(70.0), Needed::Needs { .. }));
        assert!(matches!(
            c.needed_on_remaining(81.0),
            Needed::Impossible { .. }
        ));
        assert_eq!(c.needed_on_remaining(30.0), Needed::Secured);

        let done = course(
            Scheme::Points,
            vec![cat(0.0, vec![item(Some(60.0), 100.0)])],
        );
        assert_eq!(done.needed_on_remaining(80.0), Needed::NoRemaining);
    }
}
