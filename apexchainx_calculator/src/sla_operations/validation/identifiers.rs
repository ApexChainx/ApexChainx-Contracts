//! Validation helpers for outage and record identifiers.

/// Errors produced while validating workspace identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentifierError {
    /// The identifier is empty.
    Empty,
    /// The identifier exceeds the allowed length.
    TooLong,
    /// The identifier contains a character outside the allowed set.
    InvalidCharacter,
}

/// Validates an identifier against a maximum byte length.
///
/// Identifiers must be non-empty, at most `max_len` bytes, and contain only
/// ASCII letters, digits, underscores, or hyphens.
#[must_use]
pub fn validate_identifier(value: &str, max_len: usize) -> Result<(), IdentifierError> {
    let bytes = value.as_bytes();

    if bytes.is_empty() {
        return Err(IdentifierError::Empty);
    }

    if bytes.len() > max_len {
        return Err(IdentifierError::TooLong);
    }

    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-') {
            return Err(IdentifierError::InvalidCharacter);
        }
        index += 1;
    }

    Ok(())
}

/// Returns whether `value` is a valid identifier for `max_len`.
#[must_use]
pub fn is_valid_identifier(value: &str, max_len: usize) -> bool {
    validate_identifier(value, max_len).is_ok()
}

/// Validates that `value` uses only lowercase ASCII characters.
#[must_use]
pub fn validate_lowercase_identifier(value: &str, max_len: usize) -> Result<(), IdentifierError> {
    validate_identifier(value, max_len)?;

    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_uppercase() {
            return Err(IdentifierError::InvalidCharacter);
        }
        index += 1;
    }

    Ok(())
}
