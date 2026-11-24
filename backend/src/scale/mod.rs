//! [`Scale`]: a course's letter grade cutoffs.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// The minimum percentage (0–100) for each letter grade.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scale {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub f: f64,
}

impl Scale {
    /// The usual 90/80/70/60 scale.
    pub const STANDARD: Scale = Scale {
        a: 90.0,
        b: 80.0,
        c: 70.0,
        d: 60.0,
        f: 0.0,
    };

    /// Each letter with its cutoff, from A to F.
    pub const fn cutoffs(&self) -> [(&'static str, f64); 5] {
        [
            ("A", self.a),
            ("B", self.b),
            ("C", self.c),
            ("D", self.d),
            ("F", self.f),
        ]
    }

    /// Letter for a percentage: the letter with the highest cutoff it
    /// reaches, or the lowest letter if it reaches none. `None` without a
    /// percentage.
    pub fn letter_for(&self, percent: Option<f64>) -> Option<&'static str> {
        let percent = percent?;
        let mut sorted = self.cutoffs();
        // Highest cutoff first (stable, so equal cutoffs keep A→F order), in
        // case the cutoffs were set out of order.
        sorted.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(Ordering::Equal));
        // Small tolerance so 89.99999999 from floating point still earns the 90 cutoff.
        sorted
            .iter()
            .find(|(_, min)| percent + 1e-9 >= *min)
            .or(sorted.last())
            .map(|(letter, _)| *letter)
    }
}

impl Default for Scale {
    fn default() -> Self {
        Scale::STANDARD
    }
}

#[cfg(test)]
mod tests;
