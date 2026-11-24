// ── Aliases ─────────────────────────────────────────────────────────────────
use core::{
    default::Default,
    fmt::{self, Debug, Display},
    hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};

// ── `enum Status` Definition ────────────────────────────────────────────────
/// Whether an item counts toward its category's grade, and if not, why not.
///
/// Only `Excluded` is the user's choice (the "Excl." checkbox); the engine
/// keeps it as given. The other three follow from the item's score and its
/// category's drop rules, so grading always recomputes them, whatever was
/// stored or sent in.
#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Status {
    /// Graded and counted as part of the category's grade.
    Counted,

    /// Graded, but removed by a drop-lowest / drop-highest rule.
    Dropped,

    /// Marked as not counting toward the grade, graded or not.
    Excluded,

    /// No grade yet.
    #[default]
    Ungraded,
}

// ── `Status` Implementation ─────────────────────────────────────────────────
impl Status {
    // ── Methods ─────────────────────────────────────────────────────────────
    /// Returns `true` if the status is `Counted` and `false` otherwise.
    #[inline]
    pub const fn is_counted(self) -> bool {
        matches!(self, Self::Counted)
    }

    /// Returns `true` if the status is `Dropped` and `false` otherwise.
    #[inline]
    pub const fn is_dropped(self) -> bool {
        matches!(self, Self::Dropped)
    }

    /// Returns `true` if the status is `Excluded` and `false` otherwise.
    #[inline]
    pub const fn is_excluded(self) -> bool {
        matches!(self, Self::Excluded)
    }

    /// Returns `true` if the status is `Ungraded` and `false` otherwise.
    #[inline]
    pub const fn is_ungraded(self) -> bool {
        matches!(self, Self::Ungraded)
    }
}

// ── `Display for Status` Implementation ─────────────────────────────────────
impl Display for Status {
    // ── Methods ─────────────────────────────────────────────────────────────
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Counted => write!(f, "Counted"),
            Status::Dropped => write!(f, "Dropped"),
            Status::Excluded => write!(f, "Excluded"),
            Status::Ungraded => write!(f, "Ungraded"),
        }
    }
}

// ── `Debug for Status` Implementation ───────────────────────────────────────
impl Debug for Status {
    // ── Methods ─────────────────────────────────────────────────────────────
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

// ── `Hash for Status` Implementation ────────────────────────────────────────
impl Hash for Status {
    // ── Methods ─────────────────────────────────────────────────────────────
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self as u8).hash(state);
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests;
