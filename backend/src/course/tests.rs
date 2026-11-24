//! Unit tests for [`super`] (`course/mod.rs`).

use super::*;
use crate::item::Item;
use crate::utils::round;

fn item(earned_points: Option<f64>, total_points: f64) -> Item {
    Item {
        earned_points,
        total_points,
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
        name: "x".into(),
        scheme,
        categories,
        scale: Scale::STANDARD,
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
fn maps_percentages_to_letters_on_its_scale() {
    let mut c = course(Scheme::Points, vec![]);
    assert_eq!(c.letter_for(Some(89.99)).as_deref(), Some("B"));
    assert_eq!(c.letter_for(Some(90.0)).as_deref(), Some("A"));
    assert_eq!(c.letter_for(Some(-5.0)).as_deref(), Some("F"));
    assert_eq!(c.letter_for(None), None);
    c.scale.a = 95.0;
    assert_eq!(c.letter_for(Some(92.0)).as_deref(), Some("B"));
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
