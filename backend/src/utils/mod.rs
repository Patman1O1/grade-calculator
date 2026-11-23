//! Small helpers shared across the crate.

#[cfg(test)]
mod tests;

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
