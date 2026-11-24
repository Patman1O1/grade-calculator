//! Unit tests for [`super`] (`scale/mod.rs`).

use super::*;
use serde_json::json;

#[test]
fn default_is_the_standard_scale() {
    assert_eq!(Scale::default(), Scale::STANDARD);
    assert_eq!(
        Scale::STANDARD.cutoffs(),
        [
            ("A", 90.0),
            ("B", 80.0),
            ("C", 70.0),
            ("D", 60.0),
            ("F", 0.0)
        ]
    );
}

#[test]
fn maps_percentages_to_letters() {
    let s = Scale::STANDARD;
    assert_eq!(s.letter_for(Some(90.0)), Some("A"));
    assert_eq!(s.letter_for(Some(89.99)), Some("B"));
    assert_eq!(s.letter_for(Some(60.0)), Some("D"));
    assert_eq!(s.letter_for(Some(59.0)), Some("F"));
    assert_eq!(s.letter_for(Some(-5.0)), Some("F"));
    assert_eq!(s.letter_for(Some(130.0)), Some("A"));
    assert_eq!(s.letter_for(None), None);
}

#[test]
fn tolerates_floating_point_noise_at_a_cutoff() {
    assert_eq!(
        Scale::STANDARD.letter_for(Some(89.999_999_999_9)),
        Some("A")
    );
}

#[test]
fn uses_the_highest_cutoff_reached_even_when_out_of_order() {
    // B's cutoff set above A's: 95% reaches both and B's is higher; 90% reaches only A's.
    let s = Scale {
        a: 88.0,
        b: 92.0,
        ..Scale::STANDARD
    };
    assert_eq!(s.letter_for(Some(95.0)), Some("B"));
    assert_eq!(s.letter_for(Some(90.0)), Some("A"));
    // A lowest letter above 0 still catches everything below every cutoff.
    let s = Scale {
        f: 10.0,
        ..Scale::STANDARD
    };
    assert_eq!(s.letter_for(Some(5.0)), Some("F"));
}

#[test]
fn serialises_as_one_field_per_letter() {
    let v = serde_json::to_value(Scale::STANDARD).unwrap();
    assert_eq!(
        v,
        json!({ "a": 90.0, "b": 80.0, "c": 70.0, "d": 60.0, "f": 0.0 })
    );
    let back: Scale = serde_json::from_value(v).unwrap();
    assert_eq!(back, Scale::STANDARD);
    assert!(serde_json::from_value::<Scale>(json!({ "a": 90 })).is_err());
}
