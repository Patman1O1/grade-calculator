//! Unit tests for [`super`] (`item/mod.rs`).

use super::*;
use serde_json::json;

fn item(earned: Option<f64>, possible: f64) -> Item {
    Item {
        earned,
        possible,
        ..Default::default()
    }
}

fn parse(v: serde_json::Value) -> Item {
    serde_json::from_value(v).unwrap()
}

#[test]
fn graded_items_count() {
    let mut i = item(Some(8.5), 10.0);
    i.grade();
    assert_eq!(i.status, Some(Status::Counted));
    assert!(i.is_counted());
    assert_eq!(i.points(), (8.5, 10.0));
    assert_eq!(i.percent(), Some(85.0));
}

#[test]
fn zero_point_items_are_extra_credit() {
    let mut i = item(Some(2.0), 0.0);
    i.grade();
    assert_eq!(i.points(), (2.0, 0.0));
    assert_eq!(i.percent(), None);
    assert!(!i.is_remaining());
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
    assert_eq!(excluded.status, Some(Status::Excluded));
    assert!(!excluded.is_remaining());

    // Excluded wins over ungraded: it won't count even once it has a score.
    let mut excluded_blank = item(None, 10.0);
    excluded_blank.excluded = true;
    excluded_blank.grade();
    assert_eq!(excluded_blank.status, Some(Status::Excluded));
}

#[test]
fn serialises_earned_and_possible_and_ignores_status_on_input() {
    let mut i = item(Some(1.0), 2.0);
    i.grade();
    let v = serde_json::to_value(&i).unwrap();
    assert_eq!(v["earned"], 1.0);
    assert_eq!(v["possible"], 2.0);
    assert_eq!(v["status"], "counted");
    assert!(v.get("excluded").is_none());
    let back: Item = serde_json::from_value(v).unwrap();
    assert_eq!(back.status, None);
    assert_eq!((back.earned, back.possible), (Some(1.0), 2.0));

    // Ungraded is an explicit null, so it survives a round trip.
    let v = serde_json::to_value(item(None, 10.0)).unwrap();
    assert_eq!(v["earned"], serde_json::Value::Null);
}

#[test]
fn converts_the_older_score_out_of_worth_form() {
    // A 0–100 report worth 10 points: 85/100 → 8.5 of 10.
    let i = parse(json!({ "name": "Report", "score": 85, "outOf": 100, "worth": 10 }));
    assert_eq!((i.earned, i.possible), (Some(85.0 / 100.0 * 10.0), 10.0));
    // Without worth, the points are the raw score.
    let i = parse(json!({ "name": "Quiz", "score": 7, "outOf": 10, "neverDrop": true }));
    assert_eq!(
        (i.earned, i.possible, i.never_drop),
        (Some(7.0), 10.0, true)
    );
    // Ungraded keeps its (rescaled) points possible.
    let i = parse(json!({ "name": "Exam", "score": null, "outOf": 100, "worth": 50 }));
    assert_eq!((i.earned, i.possible), (None, 50.0));
    // Zero-point extra credit keeps its raw score.
    let i = parse(json!({ "name": "EC", "score": 65.39, "outOf": 0 }));
    assert_eq!((i.earned, i.possible), (Some(65.39), 0.0));
    // Old computed fields are ignored.
    let i =
        parse(json!({ "name": "X", "score": 0, "outOf": 10, "earned": 10, "status": "counted" }));
    assert_eq!((i.earned, i.possible, i.status), (Some(0.0), 10.0, None));
}

#[test]
fn rejects_items_without_points_possible() {
    assert!(serde_json::from_value::<Item>(json!({ "name": "?", "earned": 5 })).is_err());
}
