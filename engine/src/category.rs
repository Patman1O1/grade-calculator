//! [`Category`]: a group of items with optional drop rules and a weight.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::is_false;
use crate::item::Item;

/// A group of items (homework, exams, …). It may drop the lowest/highest
/// results, and carries a weight when the course is weighted.
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

    // Computed by `grade` (see the crate docs).
    /// Sum over items whose status is `Counted`.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub earned: Option<f64>,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub possible: Option<f64>,
    /// earned / possible as a percentage; stays `None` when nothing is graded.
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
}

impl Category {
    /// Grade the category in place: grades every item, applies the drop rules,
    /// and fills in the category's `earned`, `possible` and `percent`.
    pub fn grade(&mut self) {
        self.items.iter_mut().for_each(Item::grade);
        self.apply_drops();

        let (earned, possible) = self
            .items
            .iter()
            .filter(|i| i.is_counted())
            .fold((0.0, 0.0), |(e, p), i| {
                (e + i.earned.unwrap_or(0.0), p + i.possible.unwrap_or(0.0))
            });
        self.earned = Some(earned);
        self.possible = Some(possible);
        self.percent = (possible > 0.0).then(|| earned / possible * 100.0);
    }

    /// Apply drop-lowest, then drop-highest, to the graded items.
    fn apply_drops(&mut self) {
        let drop_lowest = drop_count(self.drop_lowest);
        let drop_highest = drop_count(self.drop_highest);
        if drop_lowest == 0 && drop_highest == 0 {
            return;
        }

        let (mut fixed_earned, mut fixed_possible, mut fixed_count) = (0.0, 0.0, 0usize);
        let mut pool: Vec<Candidate> = Vec::new();
        for (index, item) in self.items.iter().enumerate() {
            if !item.is_counted() {
                continue;
            }
            let (earned, possible) = (item.earned.unwrap_or(0.0), item.possible.unwrap_or(0.0));
            if item.never_drop {
                fixed_earned += earned;
                fixed_possible += possible;
                fixed_count += 1;
            } else {
                pool.push(Candidate {
                    index,
                    earned,
                    possible,
                });
            }
        }
        // Like Canvas, never drop every item: at least one stays.
        let min_keep = if fixed_count > 0 { 0 } else { 1 };

        for (drops, mode) in [(drop_lowest, Mode::Max), (drop_highest, Mode::Min)] {
            let drops = drops.min(pool.len().saturating_sub(min_keep));
            if drops > 0 {
                let kept = choose_kept(
                    &pool,
                    pool.len() - drops,
                    fixed_earned,
                    fixed_possible,
                    mode,
                );
                pool.retain(|c| kept.contains(&c.index));
            }
        }

        for (index, item) in self.items.iter_mut().enumerate() {
            if item.is_counted() && !item.never_drop && !pool.iter().any(|c| c.index == index) {
                item.drop_from_grade();
            }
        }
    }
}

/// Number of items a drop rule asks for (negative and fractional values are floored to 0..).
fn drop_count(n: Option<f64>) -> usize {
    match n {
        Some(n) if n.is_finite() && n > 0.0 => n.floor() as usize,
        _ => 0,
    }
}

/// A graded, droppable item in the drop search.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    /// Index into `Category::items`.
    index: usize,
    earned: f64,
    possible: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Keep the set with the highest ratio (i.e. drop the lowest).
    Max,
    /// Keep the set with the lowest ratio (i.e. drop the highest).
    Min,
}

/// Choose which `keep` candidates to keep so that
/// `(fixed_earned + Σ earned) / (fixed_possible + Σ possible)`
/// is maximised ([`Mode::Max`]) or minimised ([`Mode::Min`]).
///
/// When items are worth different amounts, "drop the lowest percentage" is not
/// always optimal. Canvas solves this exactly with a parametric search on the
/// resulting ratio q: a ratio q is achievable iff the best `keep` values of
/// `earned − q·possible` sum to at least `−(fixed_earned − q·fixed_possible)`.
/// We do the same, so results match Canvas for any mix of point values.
///
/// Returns the indexes (into the item list) of the kept candidates.
fn choose_kept(
    candidates: &[Candidate],
    keep: usize,
    fixed_earned: f64,
    fixed_possible: f64,
    mode: Mode,
) -> Vec<usize> {
    if keep >= candidates.len() {
        return candidates.iter().map(|c| c.index).collect();
    }
    if keep == 0 {
        return Vec::new();
    }

    let sign = if mode == Mode::Max { 1.0 } else { -1.0 };
    // The `keep` best candidates for a given q, with their values.
    let pick = |q: f64| -> Vec<(usize, f64)> {
        let mut scored: Vec<(usize, f64)> = candidates
            .iter()
            .map(|c| (c.index, sign * (c.earned - q * c.possible)))
            .collect();
        // Highest value first; stable tie-break prefers keeping earlier items.
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        scored.truncate(keep);
        scored
    };
    let feasible = |q: f64| {
        let picked: f64 = pick(q).iter().map(|(_, v)| v).sum();
        sign * (fixed_earned - q * fixed_possible) + picked >= 0.0
    };
    let indexes = |picked: Vec<(usize, f64)>| picked.into_iter().map(|(i, _)| i).collect();

    let min_positive = std::iter::once(fixed_possible)
        .chain(candidates.iter().map(|c| c.possible))
        .filter(|p| *p > 0.0)
        .fold(f64::INFINITY, f64::min);
    if !min_positive.is_finite() {
        // Nothing has a denominator: only extra credit, so keep the largest (or smallest) scores.
        return indexes(pick(0.0));
    }

    let total_earned = fixed_earned + candidates.iter().map(|c| c.earned.max(0.0)).sum::<f64>();
    let mut lo = 0.0;
    let mut hi = total_earned / min_positive + 1.0;
    match mode {
        Mode::Max => {
            // Largest achievable q.
            for _ in 0..200 {
                let mid = (lo + hi) / 2.0;
                if feasible(mid) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            indexes(pick(lo))
        }
        Mode::Min => {
            // Smallest achievable q (feasible means "ratio ≤ q is reachable").
            for _ in 0..200 {
                let mid = (lo + hi) / 2.0;
                if feasible(mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            indexes(pick(hi))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Status;
    use crate::round;

    fn item(score: Option<f64>, out_of: f64) -> Item {
        Item {
            score,
            out_of,
            ..Default::default()
        }
    }

    fn graded(items: Vec<Item>, drop_lowest: f64, drop_highest: f64) -> Category {
        let mut c = Category {
            items,
            drop_lowest: Some(drop_lowest),
            drop_highest: Some(drop_highest),
            ..Default::default()
        };
        c.grade();
        c
    }

    fn dropped(c: &Category) -> Vec<bool> {
        c.items
            .iter()
            .map(|i| i.status == Some(Status::Dropped))
            .collect()
    }

    #[test]
    fn drops_the_item_that_maximises_the_result_not_just_the_lowest_percentage() {
        // Dropping the 0/1 gives 40/110 = 36%; dropping the 30/100 gives 10/11 = 91%.
        let c = graded(
            vec![
                item(Some(0.0), 1.0),
                item(Some(30.0), 100.0),
                item(Some(10.0), 10.0),
            ],
            1.0,
            0.0,
        );
        assert_eq!(dropped(&c), [false, true, false]);
        assert_eq!(round(c.percent.unwrap(), 2), 90.91);
    }

    #[test]
    fn drop_highest_removes_the_best_result() {
        let c = graded(
            vec![
                item(Some(10.0), 10.0),
                item(Some(5.0), 10.0),
                item(Some(7.0), 10.0),
            ],
            0.0,
            1.0,
        );
        assert_eq!(dropped(&c), [true, false, false]);
        assert_eq!(c.percent, Some(60.0));
    }

    #[test]
    fn never_drops_never_drop_items_and_always_keeps_one() {
        let mut keep = item(Some(0.0), 10.0);
        keep.never_drop = true;
        let c = graded(vec![keep, item(Some(5.0), 10.0)], 1.0, 0.0);
        assert_eq!(dropped(&c), [false, true]);

        let single = graded(vec![item(Some(2.0), 10.0)], 3.0, 0.0);
        assert_eq!(single.percent, Some(20.0));
    }

    #[test]
    fn ignores_ungraded_and_excluded_items_when_dropping() {
        let mut excluded = item(Some(1.0), 10.0);
        excluded.excluded = true;
        let c = graded(
            vec![
                item(None, 10.0),
                excluded,
                item(Some(4.0), 10.0),
                item(Some(8.0), 10.0),
            ],
            1.0,
            0.0,
        );
        assert_eq!(c.percent, Some(80.0));
        let statuses: Vec<_> = c.items.iter().map(|i| i.status.unwrap()).collect();
        assert_eq!(
            statuses,
            [
                Status::Ungraded,
                Status::Excluded,
                Status::Dropped,
                Status::Counted
            ]
        );
    }

    #[test]
    fn sums_counted_items_including_extra_credit() {
        let c = graded(vec![item(Some(9.0), 10.0), item(Some(2.0), 0.0)], 0.0, 0.0);
        assert_eq!((c.earned, c.possible), (Some(11.0), Some(10.0)));
        let empty = graded(vec![item(None, 10.0)], 0.0, 0.0);
        assert_eq!((empty.earned, empty.percent), (Some(0.0), None));
    }
}
