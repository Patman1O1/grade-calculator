//! Unit tests for [`super`] (`item/mod.rs`).

use super::*;

fn item(score: Option<f64>, out_of: f64) -> Item {
    Item {
        score,
        out_of,
        ..Default::default()
    }
}

#[test]
fn scales_a_raw_score_to_its_worth() {
    let mut i = item(Some(85.0), 100.0);
    i.worth = Some(10.0);
    i.grade();
    assert!((i.earned.unwrap() - 8.5).abs() < 1e-9);
    assert_eq!(i.possible, Some(10.0));
    assert_eq!(i.status, Some(Status::Counted));
}

#[test]
fn zero_point_items_are_extra_credit() {
    let mut i = item(Some(2.0), 0.0);
    i.grade();
    assert_eq!((i.earned, i.possible), (Some(2.0), Some(0.0)));
}

#[test]
fn ungraded_and_excluded_items_do_not_count() {
    let mut ungraded = item(None, 10.0);
    ungraded.grade();
    assert_eq!(ungraded.status, Some(Status::Ungraded));
    assert!(!ungraded.is_counted());
    assert!(ungraded.is_remaining());

    let mut excluded = item(Some(5.0), 10.0);
    excluded.excluded = true;
    excluded.grade();
    assert_eq!(
        (excluded.status, excluded.earned),
        (Some(Status::Excluded), Some(0.0))
    );
    assert!(!excluded.is_remaining());

    // Excluded wins over ungraded: it won't count even once it has a score.
    let mut excluded_blank = item(None, 10.0);
    excluded_blank.excluded = true;
    excluded_blank.grade();
    assert_eq!(excluded_blank.status, Some(Status::Excluded));
}

#[test]
fn status_serialises_in_lowercase_and_is_ignored_on_input() {
    let mut i = item(Some(1.0), 2.0);
    i.grade();
    let json = serde_json::to_value(&i).unwrap();
    assert_eq!(json["status"], "counted");
    let back: Item = serde_json::from_value(json).unwrap();
    assert_eq!(back.status, None);
}
