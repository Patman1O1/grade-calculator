//! Unit tests for [`super`] (`item/mod.rs`).

use super::*;
use crate::status::Status;
use serde_json::json;

fn item(earned: Option<f64>, possible: f64) -> Item {
    Item {
        earned,
        possible,
        ..Default::default()
    }
}

/// Read an item the way imported courses come in: deserialise, then grade,
/// which recomputes the status.
fn parse(v: serde_json::Value) -> Item {
    let mut item: Item = serde_json::from_value(v).unwrap();
    item.grade();
    item
}

#[test]
fn graded_items_count() {
    let mut i = item(Some(8.5), 10.0);
    i.grade();
    assert_eq!(i.status, Status::Counted);
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
    assert_eq!(ungraded.status, Status::Ungraded);
    assert!(!ungraded.is_counted());
    assert!(ungraded.is_remaining());

    let mut excluded = item(Some(5.0), 10.0);
    excluded.status = Status::Excluded;
    excluded.grade();
    assert_eq!(excluded.status, Status::Excluded);
    assert!(!excluded.is_remaining());

    // Excluded wins over ungraded: it won't count even once it has a score.
    let mut excluded_blank = item(None, 10.0);
    excluded_blank.status = Status::Excluded;
    excluded_blank.grade();
    assert_eq!(excluded_blank.status, Status::Excluded);
}

#[test]
fn serialises_status_and_only_trusts_excluded_on_input() {
    let mut i = item(Some(1.0), 2.0);
    i.grade();
    let v = serde_json::to_value(&i).unwrap();
    assert_eq!(v["earned"], 1.0);
    assert_eq!(v["possible"], 2.0);
    assert_eq!(v["status"], "counted");
    let back: Item = serde_json::from_value(v).unwrap();
    assert_eq!(
        (back.earned, back.possible, back.status),
        (Some(1.0), 2.0, Status::Counted)
    );

    // "excluded" is the user's choice and is kept, graded or not.
    let i = parse(json!({ "name": "A", "earned": 5, "possible": 10, "status": "excluded" }));
    assert_eq!(i.status, Status::Excluded);
    let i = parse(json!({ "name": "A", "earned": null, "possible": 10, "status": "excluded" }));
    assert_eq!(i.status, Status::Excluded);
    // Anything else is recomputed from the score, whatever was sent.
    let i = parse(json!({ "name": "A", "earned": null, "possible": 10, "status": "counted" }));
    assert_eq!(i.status, Status::Ungraded);
    let i = parse(json!({ "name": "A", "earned": 3, "possible": 10, "status": "dropped" }));
    assert_eq!(i.status, Status::Counted);
    let i = parse(json!({ "name": "A", "earned": 3, "possible": 10 }));
    assert_eq!(i.status, Status::Counted);

    // Ungraded is an explicit null, so it survives a round trip.
    let v = serde_json::to_value(item(None, 10.0)).unwrap();
    assert_eq!(v["earned"], serde_json::Value::Null);
    assert_eq!(v["status"], "ungraded");
}

#[test]
fn rejects_items_without_points_possible() {
    assert!(serde_json::from_value::<Item>(json!({ "name": "?", "earned": 5 })).is_err());
}

#[test]
fn a_missing_status_defaults_to_ungraded_until_graded() {
    let i: Item =
        serde_json::from_value(json!({ "name": "Q", "earned": 3, "possible": 10 })).unwrap();
    assert_eq!(i.status, Status::Ungraded);
    assert_eq!(
        parse(json!({ "name": "Q", "earned": 3, "possible": 10 })).status,
        Status::Counted
    );
}
