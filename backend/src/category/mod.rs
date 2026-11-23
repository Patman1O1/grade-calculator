//! [`Category`]: a group of items with optional drop rules and a weight.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use serde_json::Value;

use crate::item::{self, Item};
use crate::utils::is_false;

/// A group of items (homework, exams, …). It may drop the lowest/highest
/// results, and carries a weight when the course is weighted.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
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

/// Rewrite a category's JSON from older forms to the current one, in place
/// (see [`item::upgrade_json`]).
pub fn upgrade_json(category: &mut Value) {
    if let Some(items) = category.get_mut("items").and_then(Value::as_array_mut) {
        items.iter_mut().for_each(item::upgrade_json);
    }
}

impl Category {
    /// Grade the category in place: grades every item, applies the drop rules,
    /// and fills in the category's `earned`, `possible` and `percent`.
    pub fn grade(&mut self) {
        self.items.iter_mut().for_each(Item::grade);
        self.apply_drops();

        let (earned, possible) =
            self.items
                .iter()
                .filter(|i| i.is_counted())
                .fold((0.0, 0.0), |(e, p), i| {
                    let (earned, possible) = i.points();
                    (e + earned, p + possible)
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
            let (earned, possible) = item.points();
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
mod tests;
