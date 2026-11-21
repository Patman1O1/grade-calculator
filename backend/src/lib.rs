//! Grade calculator engine, compiled to WebAssembly for the React front-end.
//!
//! The data model is small but general enough to express the grading policies
//! used by Canvas, Blackboard and typical syllabi. Each type lives in its own
//! module together with the logic that grades it:
//!
//! - [`item`]: an [`Item`] is one graded thing, `earned` out of `possible`
//!   points; its [`Status`] says whether it counts and, if not, why not.
//! - [`category`]: a [`Category`] groups items, may drop the lowest/highest
//!   results (the Canvas-exact drop search lives here), and carries a weight.
//! - [`course`]: a [`Course`] combines categories by total points or by weights
//!   ([`Scheme`]), maps percentages to letters ([`Cutoff`]), and solves "what do
//!   I need on the remaining work?" ([`Needed`]).
//! - [`examples`]: example courses transcribed from the PDFs in `docs/`.
//! - [`api`]: the operations the front-end needs (evaluate, summaries, import…).
//! - [`wasm`]: the `wasm-bindgen` JavaScript interface over [`api`].
//!
//! Everything serialises as camelCase JSON, the same shape the front-end stores
//! and that older JSON exports of the TypeScript app used, so those still import.
//!
//! The grade lives on the same types: [`Course::grade`] fills in each item's
//! `status`, each category's
//! `earned`/`possible`/`percent`, and the course's `earned`/`possible`/`percent`/
//! `letter`. These computed fields are `None` (and left out of the JSON) until
//! the course is graded, and they are ignored when a course is deserialised, so
//! the front-end can never send in results; they are always recomputed.

pub mod api;
pub mod category;
pub mod course;
pub mod examples;
pub mod item;
pub mod wasm;

pub use category::Category;
pub use course::{Course, Cutoff, Needed, Scheme};
pub use item::{Item, Status};

/// Round half away from zero, immune to binary noise like 52.549999999.
pub fn round(x: f64, decimals: u32) -> f64 {
    let f = 10f64.powi(decimals as i32);
    let clean: f64 = format!("{x:.10}").parse().unwrap_or(x);
    clean.signum() * (clean.abs() * f).round() / f
}

/// `skip_serializing_if` helper for flags that default to false.
pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

#[cfg(test)]
mod tests {
    use super::round;

    #[test]
    fn rounds_half_away_from_zero_without_binary_noise() {
        // 52.55 is stored as 52.5499999999999971578… in binary.
        assert_eq!(round(52.55, 1), 52.6);
        assert_eq!(round(78.36, 1), 78.4);
        assert_eq!(round(-2.5, 0), -3.0);
    }
}
