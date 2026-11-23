//! Unit tests for [`super`] (`utils/mod.rs`).

use super::*;

#[test]
fn rounds_half_away_from_zero_without_binary_noise() {
    // 52.55 is stored as 52.5499999999999971578… in binary.
    assert_eq!(round(52.55, 1), 52.6);
    assert_eq!(round(78.36, 1), 78.4);
    assert_eq!(round(-2.5, 0), -3.0);
}

#[test]
fn is_false_only_for_false() {
    assert!(is_false(&false));
    assert!(!is_false(&true));
}
