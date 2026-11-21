//! Ids for courses, categories and items.

use std::sync::atomic::{AtomicU64, Ordering};

/// A fresh id: a counter plus the current time, so ids stay unique across
/// page loads (courses saved in an earlier visit keep their ids). Engine ids
/// carry an `s` marker so they never collide with the `c`-marked ids the
/// front-end generates for items it adds itself.
pub fn new_id(prefix: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed) + 1;
    format!("{prefix}-{:x}-s{n}", now_millis())
}

/// Milliseconds since the Unix epoch. `std::time::SystemTime` panics in the
/// browser (wasm32-unknown-unknown), so there we ask JavaScript instead.
#[cfg(target_arch = "wasm32")]
fn now_millis() -> u64 {
    js_sys::Date::now() as u64
}

#[cfg(not(target_arch = "wasm32"))]
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
