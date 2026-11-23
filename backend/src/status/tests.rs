//! Unit tests for [`super`] (`status/mod.rs`).

use super::*;

#[test]
fn for_score_keeps_excluded_and_recomputes_the_rest() {
    for status in [Status::Counted, Status::Dropped, Status::Ungraded] {
        assert_eq!(status.for_score(Some(5.0)), Status::Counted);
        assert_eq!(status.for_score(None), Status::Ungraded);
    }
    assert_eq!(Status::Excluded.for_score(Some(5.0)), Status::Excluded);
    assert_eq!(Status::Excluded.for_score(None), Status::Excluded);
}

#[test]
fn only_counted_counts() {
    assert!(Status::Counted.counts());
    for status in [Status::Dropped, Status::Excluded, Status::Ungraded] {
        assert!(!status.counts());
    }
}

#[test]
fn serialises_in_lowercase_and_defaults_to_ungraded() {
    assert_eq!(serde_json::to_value(Status::Excluded).unwrap(), "excluded");
    let s: Status = serde_json::from_value("dropped".into()).unwrap();
    assert_eq!(s, Status::Dropped);
    assert_eq!(Status::default(), Status::Ungraded);
}
