//! Defines the contract-wide policy for resolving optional values.

/// Generic helper for resolving optional storage.
///
/// Public read-only methods should use this helper instead of
/// calling `unwrap()` directly.
pub fn resolve_or_default<T: Clone>(
    value: Option<T>,
    default: T,
) -> T {
    match value {
        Some(value) => value,
        None => default,
    }
}

/// Returns whether a fallback value was used.
pub fn used_default<T>(
    value: &Option<T>,
) -> bool {
    value.is_none()
}