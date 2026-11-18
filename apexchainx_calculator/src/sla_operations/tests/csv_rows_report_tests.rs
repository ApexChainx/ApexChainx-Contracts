//! Tests for `reporting::csv_rows`.

use crate::sla_operations::reporting::csv_rows::{escape_field, format_row, format_table};
use alloc::{string::String, vec::Vec};

fn row(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|f| String::from(*f)).collect()
}

#[test]
fn plain_fields_are_left_alone() {
    assert_eq!(escape_field("critical"), "critical");
    assert_eq!(escape_field(""), "");
    assert_eq!(escape_field("with space"), "with space");
    assert_eq!(escape_field("-3000"), "-3000");
}

#[test]
fn fields_with_commas_are_quoted() {
    assert_eq!(escape_field("a,b"), "\"a,b\"");
}

#[test]
fn embedded_quotes_are_doubled_and_the_field_is_quoted() {
    assert_eq!(escape_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    assert_eq!(escape_field("\""), "\"\"\"\"");
}

#[test]
fn newlines_and_carriage_returns_force_quoting() {
    assert_eq!(escape_field("a\nb"), "\"a\nb\"");
    assert_eq!(escape_field("a\rb"), "\"a\rb\"");
}

#[test]
fn unicode_passes_through_unchanged() {
    assert_eq!(escape_field("résumé"), "résumé");
    assert_eq!(escape_field("a,é"), "\"a,é\"");
}

#[test]
fn format_row_joins_with_commas_and_no_trailing_newline() {
    assert_eq!(format_row(&row(&["a", "b", "c"])), "a,b,c");
    assert_eq!(format_row(&row(&["only"])), "only");
}

#[test]
fn format_row_escapes_each_field_independently() {
    assert_eq!(
        format_row(&row(&["a,b", "plain", "q\"q"])),
        "\"a,b\",plain,\"q\"\"q\""
    );
}

#[test]
fn an_empty_row_is_the_empty_string() {
    assert_eq!(format_row(&[]), "");
}

#[test]
fn empty_fields_keep_their_columns() {
    assert_eq!(format_row(&row(&["", "x", ""])), ",x,");
}

#[test]
fn table_has_a_header_and_every_line_ends_with_a_newline() {
    let out = format_table(&["id", "note"], &[row(&["1", "ok"]), row(&["2", "a,b"])]);
    assert_eq!(out, "id,note\n1,ok\n2,\"a,b\"\n");
}

#[test]
fn a_table_with_no_rows_is_just_the_header_line() {
    assert_eq!(format_table(&["a", "b"], &[]), "a,b\n");
}

#[test]
fn header_names_are_escaped_too() {
    assert_eq!(format_table(&["a,b", "c"], &[]), "\"a,b\",c\n");
}

#[test]
fn output_is_deterministic() {
    let rows = [row(&["1", "x,y"])];
    assert_eq!(format_table(&["h1", "h2"], &rows), format_table(&["h1", "h2"], &rows));
}
