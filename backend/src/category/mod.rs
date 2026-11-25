//! [`Category`]: a group of items with optional drop rules and a weight.

use serde::{Deserialize, Serialize};

use crate::item::Item;
use crate::status::Status;
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

impl Category {
    /// Grade the category in place: resets every item's status from its
    /// score, applies the drop rules, and fills in the category's `earned`,
    /// `possible` and `percent`.
    pub fn grade(&mut self) {
        self.reset_statuses();
        self.apply_drops();

        let (earned, possible) =
            self.items
                .iter()
                .filter(|i| i.status.is_counted())
                .fold((0.0, 0.0), |(e, p), i| {
                    let (earned, possible) = points(i);
                    (e + earned, p + possible)
                });
        self.earned = Some(earned);
        self.possible = Some(possible);
        self.percent = (possible > 0.0).then(|| earned / possible * 100.0);
    }

    /// Set every item's status from its score, before any drop rule: an
    /// excluded item stays `Excluded` (the user's choice); any other becomes
    /// `Counted` if it has a score and `Ungraded` if not. So a status read
    /// from input is never trusted except `Excluded`.
    pub fn reset_statuses(&mut self) {
        for item in &mut self.items {
            if item.status.is_excluded() {
                continue;
            }
            match item.earned_points {
                Some(earned) => item.grade(earned, item.total_points),
                None => item.status = Status::Ungraded,
            }
        }
    }

    /// Whether the category has a drop-lowest or drop-highest rule that drops anything.
    pub fn has_drop_rules(&self) -> bool {
        drop_count(self.drop_lowest) > 0 || drop_count(self.drop_highest) > 0
    }

    /// Apply drop-lowest, then drop-highest, to the graded items.
    fn apply_drops(&mut self) {
        let drop_lowest = drop_count(self.drop_lowest);
        let drop_highest = drop_count(self.drop_highest);
        if drop_lowest == 0 && drop_highest == 0 {
            return;
        }

        // Indexes (into `items`) of the graded, droppable items.
        let mut pool: Vec<usize> = Vec::new();
        let (mut fixed_earned, mut fixed_possible, mut fixed_count) = (0.0, 0.0, 0usize);
        for (index, item) in self.items.iter().enumerate() {
            if !item.status.is_counted() {
                continue;
            }
            if item.never_drop {
                let (earned, possible) = points(item);
                fixed_earned += earned;
                fixed_possible += possible;
                fixed_count += 1;
            } else {
                pool.push(index);
            }
        }
        // Like Canvas, never drop every item: at least one stays.
        let min_keep = if fixed_count > 0 { 0 } else { 1 };

        let mut scratch = Vec::with_capacity(pool.len());
        // Drop-lowest keeps the best set (maximises), drop-highest the worst.
        for (drops, maximise) in [(drop_lowest, true), (drop_highest, false)] {
            let drops = drops.min(pool.len().saturating_sub(min_keep));
            if drops > 0 {
                let keep = pool.len() - drops;
                let fixed = (fixed_earned, fixed_possible);
                choose_kept(&self.items, &mut pool, keep, fixed, maximise, &mut scratch);
            }
        }

        // `pool` is now the kept items, in ascending order.
        for (index, item) in self.items.iter_mut().enumerate() {
            if item.status.is_counted() && pool.binary_search(&index).is_err() {
                // `drop` refuses never-drop items, which are never in the pool anyway.
                item.drop();
            }
        }
    }
}

/// `(earned, possible)` points an item adds to its category when counted.
fn points(item: &Item) -> (f64, f64) {
    (item.earned_points.unwrap_or(0.0), item.total_points)
}

/// Number of items a drop rule asks for (negative and fractional values are floored to 0..).
fn drop_count(n: Option<f64>) -> usize {
    match n {
        Some(n) if n.is_finite() && n > 0.0 => n.floor() as usize,
        _ => 0,
    }
}

/// Narrow `pool` (indexes into `items`) to the `keep` items whose
/// `(fixed_earned + Σ earned) / (fixed_possible + Σ possible)` is the highest
/// (`maximise`, for drop-lowest) or lowest (for drop-highest), leaving them in
/// ascending order.
///
/// When items are worth different amounts, "drop the lowest percentage" is not
/// always optimal. For a ratio q, the best set is the `keep` items with the
/// largest `earned − q·possible`, and q is optimal exactly when that set's
/// own ratio is q. Dinkelbach's method finds it: start from any q, take the
/// best set for q, move q to that set's ratio, and repeat until q stops
/// improving (usually 2–5 rounds). This is the same optimum Canvas finds, so
/// results match Canvas for any mix of point values. Among sets that tie for
/// the best ratio, the one keeping the earlier items wins.
///
/// `scratch` is a reusable buffer for the scores.
fn choose_kept(
    items: &[Item],
    pool: &mut Vec<usize>,
    keep: usize,
    (fixed_earned, fixed_possible): (f64, f64),
    maximise: bool,
    scratch: &mut Vec<(usize, f64)>,
) {
    if keep >= pool.len() {
        return;
    }
    if keep == 0 {
        pool.clear();
        return;
    }

    let sign = if maximise { 1.0 } else { -1.0 };
    // The `keep` best items for ratio q (largest `sign·(earned − q·possible)`
    // first, ties keeping the earlier item), with their total points.
    let mut pick = |q: f64, picked: &mut Vec<usize>| -> (f64, f64) {
        scratch.clear();
        scratch.extend(pool.iter().map(|&i| {
            let (earned, possible) = points(&items[i]);
            (i, sign * (earned - q * possible))
        }));
        let order = |a: &(usize, f64), b: &(usize, f64)| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0));
        scratch.select_nth_unstable_by(keep - 1, order);
        picked.clear();
        picked.extend(scratch[..keep].iter().map(|&(i, _)| i));
        picked
            .iter()
            .fold((fixed_earned, fixed_possible), |(e, p), &i| {
                let (earned, possible) = points(&items[i]);
                (e + earned, p + possible)
            })
    };

    let all_possible = fixed_possible + pool.iter().map(|&i| items[i].total_points).sum::<f64>();
    if all_possible <= 0.0 {
        // Nothing has a denominator: only extra credit, so keep the largest (or smallest) scores.
        let mut picked = Vec::with_capacity(keep);
        pick(0.0, &mut picked);
        *pool = picked;
        pool.sort_unstable();
        return;
    }

    // Start from the ratio with every item kept.
    let all_earned = fixed_earned + pool.iter().map(|&i| points(&items[i]).0).sum::<f64>();
    let mut q = all_earned / all_possible;
    let mut best: Vec<usize> = Vec::with_capacity(keep);
    let mut next: Vec<usize> = Vec::with_capacity(keep);
    for round in 0..64 {
        let (earned, possible) = pick(q, &mut next);
        if possible <= 0.0 {
            // Only zero-point items picked, so the ratio is undefined: take the
            // set if it's the first, or if it earned points while maximising
            // (an unbeatable ratio), and stop.
            if round == 0 || (maximise && earned > 0.0) {
                std::mem::swap(&mut best, &mut next);
            }
            break;
        }
        let ratio = earned / possible;
        let slack = 1e-12 * q.abs().max(1.0);
        let improved = if maximise {
            ratio > q + slack
        } else {
            ratio < q - slack
        };
        if round > 0 && !improved {
            break;
        }
        std::mem::swap(&mut best, &mut next);
        q = ratio;
    }

    // Several sets can tie for the best ratio (keeping 5/10 or 10/20 is 50%
    // either way) yet count different points, which matters to a
    // points-scheme course. The set picked at exactly the best ratio breaks
    // such ties by keeping the earlier items, so use it if it still reaches
    // that ratio.
    let (earned, possible) = pick(q, &mut next);
    if possible > 0.0 {
        let ratio = earned / possible;
        let slack = 1e-12 * q.abs().max(1.0);
        let reaches = if maximise {
            ratio >= q - slack
        } else {
            ratio <= q + slack
        };
        if reaches {
            std::mem::swap(&mut best, &mut next);
        }
    }
    best.sort_unstable();
    *pool = best;
}

#[cfg(test)]
mod tests;
