//! JSON-friendly report models.
//!
//! A tiny dependency-free value type with a deterministic serialiser. Objects
//! keep insertion order (no hash maps), so the same input always produces the
//! same bytes. Strings are escaped per RFC 8259.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

/// A JSON value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Int(i128),
    Str(String),
    Array(Vec<JsonValue>),
    /// Key/value pairs, serialised in the order given.
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// Convenience constructor for a string value.
    pub fn str(value: &str) -> JsonValue {
        JsonValue::Str(String::from(value))
    }

    /// Build an object from `(key, value)` pairs, preserving order.
    pub fn object(pairs: Vec<(&str, JsonValue)>) -> JsonValue {
        JsonValue::Object(
            pairs
                .into_iter()
                .map(|(k, v)| (String::from(k), v))
                .collect(),
        )
    }

    /// Serialise to a compact JSON string (no whitespace).
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        write_value(self, &mut out);
        out
    }
}

/// Escape `value` as a JSON string literal, including the surrounding quotes.
pub fn escape_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str("\\u00");
                let n = c as u32;
                out.push(hex_digit(n >> 4));
                out.push(hex_digit(n & 0xf));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn hex_digit(n: u32) -> char {
    match n {
        0..=9 => (b'0' + n as u8) as char,
        _ => (b'a' + (n as u8 - 10)) as char,
    }
}

fn write_value(value: &JsonValue, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Int(n) => out.push_str(&n.to_string()),
        JsonValue::Str(s) => out.push_str(&escape_string(s)),
        JsonValue::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out);
            }
            out.push(']');
        }
        JsonValue::Object(pairs) => {
            out.push('{');
            for (i, (key, item)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&escape_string(key));
                out.push(':');
                write_value(item, out);
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;

    #[test]
    fn serialises_scalars_and_nesting_in_order() {
        let v = JsonValue::object(alloc::vec![
            ("b", JsonValue::Int(-5)),
            ("a", JsonValue::Array(alloc::vec![JsonValue::Bool(true), JsonValue::Null])),
        ]);
        assert_eq!(v.to_json_string(), "{\"b\":-5,\"a\":[true,null]}");
    }

    #[test]
    fn escapes_special_characters() {
        assert_eq!(escape_string("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
        assert_eq!(escape_string("\u{1}"), "\"\\u0001\"");
    }
}
