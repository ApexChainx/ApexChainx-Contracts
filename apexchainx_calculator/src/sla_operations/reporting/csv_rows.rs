//! CSV-style row formatting helpers.
//!
//! Pure string helpers following RFC 4180 quoting: a field is wrapped in double
//! quotes when it contains a comma, a double quote, a carriage return or a line
//! feed, and embedded double quotes are doubled. Rows are joined with `,` and
//! every row in a table ends with `\n`, so the output for the same input is
//! byte-for-byte identical.

use alloc::{string::String, vec::Vec};

/// Escape a single field for CSV output.
///
/// Fields that need no quoting are returned unchanged (as an owned `String`).
pub fn escape_field(field: &str) -> String {
    let needs_quotes = field
        .chars()
        .any(|c| c == ',' || c == '"' || c == '\n' || c == '\r');
    if !needs_quotes {
        return String::from(field);
    }
    let mut out = String::with_capacity(field.len() + 2);
    out.push('"');
    for c in field.chars() {
        if c == '"' {
            out.push('"');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Join already-owned fields into one CSV row (no trailing newline).
pub fn format_row(fields: &[String]) -> String {
    let mut out = String::new();
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&escape_field(field));
    }
    out
}

/// Format a header plus rows into a CSV document. Every line, including the
/// last, ends with `\n`.
pub fn format_table(header: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    for (i, name) in header.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&escape_field(name));
    }
    out.push('\n');
    for row in rows {
        out.push_str(&format_row(row));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod csv_smoke_tests {
    use super::*;

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(escape_field("plain"), "plain");
        assert_eq!(escape_field("a,b"), "\"a,b\"");
        assert_eq!(escape_field("q\"q"), "\"q\"\"q\"");
    }

    #[test]
    fn table_ends_every_line_with_a_newline() {
        let rows = alloc::vec![alloc::vec![String::from("1"), String::from("x,y")]];
        assert_eq!(format_table(&["id", "note"], &rows), "id,note\n1,\"x,y\"\n");
    }
}
