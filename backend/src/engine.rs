//! The grading engine: pure functions over the [`crate::model`] types.

use std::cmp::Ordering;

use crate::model::{
    Category, CategoryResult, Course, CourseResult, Cutoff, Item, ItemResult, Needed, Scheme,
};

/// Points an item contributes to its category, given a score: `(earned, possible)`.
fn contribution(item: &Item, score: f64) -> (f64, f64) {
    let worth = item.worth();
    if item.out_of > 0.0 {
        (score / item.out_of * worth, worth)
    } else {
        // Zero-point items are pure extra credit: the raw score is added on top.
        (score, 0.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
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

/// Number of items a drop rule asks for (negative and fractional values are floored to 0..).
fn drop_count(n: Option<f64>) -> usize {
    match n {
        Some(n) if n.is_finite() && n > 0.0 => n.floor() as usize,
        _ => 0,
    }
}

pub fn compute_category(category: &Category) -> CategoryResult {
    let mut items: Vec<ItemResult> = category
        .items
        .iter()
        .map(|item| {
            let (counted, (earned, possible)) = match item.score {
                Some(score) if !item.excluded => (true, contribution(item, score)),
                _ => (false, (0.0, 0.0)),
            };
            ItemResult {
                id: item.id.clone(),
                counted,
                dropped: false,
                earned,
                possible,
            }
        })
        .collect();

    let drop_lowest = drop_count(category.drop_lowest);
    let drop_highest = drop_count(category.drop_highest);
    if drop_lowest > 0 || drop_highest > 0 {
        let never_drop = |i: usize| category.items[i].never_drop;
        let (mut fixed_earned, mut fixed_possible, mut fixed_count) = (0.0, 0.0, 0usize);
        let mut pool: Vec<Candidate> = Vec::new();
        for (index, r) in items.iter().enumerate().filter(|(_, r)| r.counted) {
            if never_drop(index) {
                fixed_earned += r.earned;
                fixed_possible += r.possible;
                fixed_count += 1;
            } else {
                pool.push(Candidate {
                    index,
                    earned: r.earned,
                    possible: r.possible,
                });
            }
        }
        // Like Canvas, never drop every item: at least one stays.
        let min_keep = if fixed_count > 0 { 0 } else { 1 };

        let low_drops = drop_lowest.min(pool.len().saturating_sub(min_keep));
        if low_drops > 0 {
            let kept = choose_kept(
                &pool,
                pool.len() - low_drops,
                fixed_earned,
                fixed_possible,
                Mode::Max,
            );
            pool.retain(|c| kept.contains(&c.index));
        }
        let high_drops = drop_highest.min(pool.len().saturating_sub(min_keep));
        if high_drops > 0 {
            let kept = choose_kept(
                &pool,
                pool.len() - high_drops,
                fixed_earned,
                fixed_possible,
                Mode::Min,
            );
            pool.retain(|c| kept.contains(&c.index));
        }

        for (index, r) in items.iter_mut().enumerate() {
            if r.counted && !never_drop(index) && !pool.iter().any(|c| c.index == index) {
                r.counted = false;
                r.dropped = true;
            }
        }
    }

    let (earned, possible) = items
        .iter()
        .filter(|r| r.counted)
        .fold((0.0, 0.0), |(e, p), r| (e + r.earned, p + r.possible));
    CategoryResult {
        id: category.id.clone(),
        name: category.name.clone(),
        items,
        earned,
        possible,
        percent: (possible > 0.0).then(|| earned / possible * 100.0),
    }
}

/// Letter for a percentage: the highest cutoff it reaches, or the lowest letter.
pub fn letter_for(percent: Option<f64>, scale: &[Cutoff]) -> Option<String> {
    let percent = percent?;
    let mut sorted: Vec<&Cutoff> = scale.iter().collect();
    sorted.sort_by(|a, b| b.min.partial_cmp(&a.min).unwrap_or(Ordering::Equal));
    // Small tolerance so 89.99999999 from floating point still earns the 90 cutoff.
    sorted
        .iter()
        .find(|c| percent + 1e-9 >= c.min)
        .or(sorted.last())
        .map(|c| c.letter.clone())
}

pub fn compute_course(course: &Course) -> CourseResult {
    let categories: Vec<CategoryResult> = course.categories.iter().map(compute_category).collect();
    let mut earned = 0.0;
    let mut possible = 0.0;

    match course.scheme {
        Scheme::Points => {
            for c in &categories {
                earned += c.earned;
                possible += c.possible;
            }
        }
        Scheme::Weighted => {
            for (cat, c) in course.categories.iter().zip(&categories) {
                let w = cat.weight;
                let Some(percent) = c.percent else { continue };
                // Skips zero, negative and NaN weights alike.
                if w.partial_cmp(&0.0) != Some(Ordering::Greater) {
                    continue;
                }
                earned += w * percent / 100.0;
                if !cat.extra_credit {
                    possible += w;
                }
            }
        }
    }

    let percent = (possible > 0.0).then(|| earned / possible * 100.0);
    CourseResult {
        categories,
        earned,
        possible,
        percent,
        letter: letter_for(percent, &course.scale),
    }
}

/// Copy of the course with every remaining item scored at `fraction` of its points.
pub fn with_remaining_at(course: &Course, fraction: f64) -> Course {
    let mut c = course.clone();
    for item in c.categories.iter_mut().flat_map(|c| c.items.iter_mut()) {
        if item.is_remaining() {
            item.score = Some(item.out_of * fraction);
        }
    }
    c
}

/// Average score (as a fraction of points) needed on every remaining item to
/// finish at or above `target` percent. "Impossible" means even 100% everywhere
/// falls short.
pub fn needed_on_remaining(course: &Course, target: f64) -> Needed {
    if course.remaining_count() == 0 {
        return Needed::NoRemaining;
    }
    let at = |f: f64| {
        compute_course(&with_remaining_at(course, f))
            .percent
            .unwrap_or(0.0)
    };
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

/// Round half away from zero, immune to binary noise like 52.549999999.
pub fn round(x: f64, decimals: u32) -> f64 {
    let f = 10f64.powi(decimals as i32);
    let clean: f64 = format!("{x:.10}").parse().unwrap_or(x);
    clean.signum() * (clean.abs() * f).round() / f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples::standard_scale;
    use crate::model::Category;

    fn item(score: Option<f64>, out_of: f64) -> Item {
        Item {
            id: String::new(),
            name: String::new(),
            score,
            out_of,
            worth: None,
            excluded: false,
            never_drop: false,
            note: None,
        }
    }

    fn cat(items: Vec<Item>) -> Category {
        Category {
            id: String::new(),
            name: String::new(),
            weight: 0.0,
            drop_lowest: None,
            drop_highest: None,
            extra_credit: false,
            items,
        }
    }

    fn course(scheme: Scheme, categories: Vec<Category>) -> Course {
        Course {
            id: "x".into(),
            name: "x".into(),
            scheme,
            categories,
            scale: standard_scale(),
            display_decimals: None,
            notes: None,
        }
    }

    fn dropped(r: &CategoryResult) -> Vec<bool> {
        r.items.iter().map(|i| i.dropped).collect()
    }

    #[test]
    fn drops_the_item_that_maximises_the_result_not_just_the_lowest_percentage() {
        // Dropping the 0/1 gives 40/110 = 36%; dropping the 30/100 gives 10/11 = 91%.
        let mut c = cat(vec![
            item(Some(0.0), 1.0),
            item(Some(30.0), 100.0),
            item(Some(10.0), 10.0),
        ]);
        c.drop_lowest = Some(1.0);
        let r = compute_category(&c);
        assert_eq!(dropped(&r), [false, true, false]);
        assert_eq!(round(r.percent.unwrap(), 2), 90.91);
    }

    #[test]
    fn drop_highest_removes_the_best_result() {
        let mut c = cat(vec![
            item(Some(10.0), 10.0),
            item(Some(5.0), 10.0),
            item(Some(7.0), 10.0),
        ]);
        c.drop_highest = Some(1.0);
        let r = compute_category(&c);
        assert_eq!(dropped(&r), [true, false, false]);
        assert_eq!(r.percent, Some(60.0));
    }

    #[test]
    fn never_drops_never_drop_items_and_always_keeps_one() {
        let mut keep = item(Some(0.0), 10.0);
        keep.never_drop = true;
        let mut c = cat(vec![keep, item(Some(5.0), 10.0)]);
        c.drop_lowest = Some(1.0);
        assert_eq!(dropped(&compute_category(&c)), [false, true]);

        let mut single = cat(vec![item(Some(2.0), 10.0)]);
        single.drop_lowest = Some(3.0);
        assert_eq!(compute_category(&single).percent, Some(20.0));
    }

    #[test]
    fn ignores_ungraded_and_excluded_items_when_dropping() {
        let mut excluded = item(Some(1.0), 10.0);
        excluded.excluded = true;
        let mut c = cat(vec![
            item(None, 10.0),
            excluded,
            item(Some(4.0), 10.0),
            item(Some(8.0), 10.0),
        ]);
        c.drop_lowest = Some(1.0);
        assert_eq!(compute_category(&c).percent, Some(80.0));
    }

    #[test]
    fn scales_a_raw_score_to_its_worth() {
        let mut i = item(Some(85.0), 100.0);
        i.worth = Some(10.0);
        let r = compute_category(&cat(vec![i]));
        assert!((r.earned - 8.5).abs() < 1e-9);
        assert_eq!(r.possible, 10.0);
    }

    #[test]
    fn treats_zero_point_items_as_extra_credit() {
        let r = compute_category(&cat(vec![item(Some(9.0), 10.0), item(Some(2.0), 0.0)]));
        assert_eq!(r.earned, 11.0);
        assert_eq!(r.possible, 10.0);
    }

    #[test]
    fn weighted_renormalises_over_categories_with_graded_work() {
        let mut a = cat(vec![item(Some(9.0), 10.0)]);
        a.weight = 30.0;
        let mut b = cat(vec![item(Some(70.0), 100.0)]);
        b.weight = 20.0;
        let mut c = cat(vec![item(None, 100.0)]);
        c.weight = 50.0;
        let r = compute_course(&course(Scheme::Weighted, vec![a, b, c]));
        assert_eq!(
            round(r.percent.unwrap(), 4),
            round((30.0 * 0.9 + 20.0 * 0.7) / 50.0 * 100.0, 4)
        );
    }

    #[test]
    fn extra_credit_categories_add_without_raising_the_denominator() {
        let mut a = cat(vec![item(Some(80.0), 100.0)]);
        a.weight = 100.0;
        let mut ec = cat(vec![item(Some(1.0), 1.0)]);
        ec.weight = 5.0;
        ec.extra_credit = true;
        let r = compute_course(&course(Scheme::Weighted, vec![a, ec]));
        assert!((r.percent.unwrap() - 85.0).abs() < 1e-9);
    }

    #[test]
    fn maps_percentages_to_letters() {
        let scale = standard_scale();
        assert_eq!(letter_for(Some(89.99), &scale).as_deref(), Some("B"));
        assert_eq!(letter_for(Some(90.0), &scale).as_deref(), Some("A"));
        assert_eq!(letter_for(None, &scale), None);
        assert_eq!(letter_for(Some(50.0), &[]), None);
    }

    #[test]
    fn finds_the_average_needed_on_remaining_work() {
        // 60/100 so far, 100 points left; 80% overall needs 100/100.
        let c = course(
            Scheme::Points,
            vec![cat(vec![item(Some(60.0), 100.0), item(None, 100.0)])],
        );
        match needed_on_remaining(&c, 80.0) {
            Needed::Needs { fraction } => assert!((fraction - 1.0).abs() < 1e-6),
            other => panic!("expected needs, got {other:?}"),
        }
        assert!(matches!(
            needed_on_remaining(&c, 70.0),
            Needed::Needs { .. }
        ));
        assert!(matches!(
            needed_on_remaining(&c, 81.0),
            Needed::Impossible { .. }
        ));
        assert_eq!(needed_on_remaining(&c, 30.0), Needed::Secured);
        let done = course(Scheme::Points, vec![cat(vec![item(Some(60.0), 100.0)])]);
        assert_eq!(needed_on_remaining(&done, 80.0), Needed::NoRemaining);
    }

    #[test]
    fn rounds_half_away_from_zero_without_binary_noise() {
        // 52.55 is stored as 52.5499999999999971578… in binary.
        assert_eq!(round(52.55, 1), 52.6);
        assert_eq!(round(78.36, 1), 78.4);
    }
}
