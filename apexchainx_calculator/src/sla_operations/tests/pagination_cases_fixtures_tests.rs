//! Tests for `fixtures::pagination_cases`.
//!
//! The fixtures describe page requests over a bounded result set. Expectations
//! are written literally so a change in the pagination rule fails here instead
//! of re-deriving itself from the implementation under test.

use crate::sla_operations::fixtures::pagination_cases::{cases, page_bounds, PageBounds, PaginationCase};
use alloc::{string::String, vec, vec::Vec};

const DOCUMENTED_CASES: [(&str, u32, u32, u32, u32, u32, bool); 15] = [
    ("first_page", 10, 0, 3, 0, 3, true),
    ("middle_page", 10, 3, 3, 3, 3, true),
    ("last_page_exact", 9, 6, 3, 6, 3, false),
    ("last_page_partial", 10, 9, 3, 9, 1, false),
    ("exactly_at_end", 10, 10, 3, 10, 0, false),
    ("offset_past_end", 10, 11, 3, 10, 0, false),
    ("zero_page_size", 10, 0, 0, 0, 0, false),
    ("single_item", 1, 0, 1, 0, 1, false),
    ("full_set", 10, 0, 10, 0, 10, false),
    ("page_larger_than_set", 10, 0, 100, 0, 10, false),
    ("empty_set", 0, 0, 5, 0, 0, false),
    ("empty_set_offset", 0, 1, 5, 0, 0, false),
    ("max_set_first_page", u32::MAX, 0, 1, 0, 1, true),
    (
        "max_set_last_item",
        u32::MAX,
        u32::MAX - 1,
        1,
        u32::MAX - 1,
        1,
        false,
    ),
    ("max_page_size", u32::MAX, 0, u32::MAX, 0, u32::MAX, false),
];

fn names() -> Vec<&'static str> {
    cases().iter().map(|c| c.name).collect()
}

fn bounds_for(case: PaginationCase) -> PageBounds {
    page_bounds(case.total_items, case.offset, case.page_size)
}

fn find_case(name: &str) -> PaginationCase {
    cases()
        .into_iter()
        .find(|c| c.name == name)
        .expect("documented pagination case")
}

#[test]
fn cases_have_the_documented_names_in_order() {
    assert_eq!(
        names(),
        vec![
            "first_page",
            "middle_page",
            "last_page_exact",
            "last_page_partial",
            "exactly_at_end",
            "offset_past_end",
            "zero_page_size",
            "single_item",
            "full_set",
            "page_larger_than_set",
            "empty_set",
            "empty_set_offset",
            "max_set_first_page",
            "max_set_last_item",
            "max_page_size",
        ]
    );
}

#[test]
fn cases_carry_the_documented_requests_and_expectations() {
    let all = cases();

    assert_eq!(all.len(), DOCUMENTED_CASES.len());
    for (case, (name, total, offset, page_size, start, len, has_more)) in all.iter().zip(DOCUMENTED_CASES) {
        assert_eq!(case.name, name);
        assert_eq!(case.total_items, total, "{name}");
        assert_eq!(case.offset, offset, "{name}");
        assert_eq!(case.page_size, page_size, "{name}");
        assert_eq!(case.expect_start, start, "{name}");
        assert_eq!(case.expect_len, len, "{name}");
        assert_eq!(case.expect_has_more, has_more, "{name}");
    }
}

#[test]
fn every_case_resolves_to_its_explicit_expectations() {
    for case in cases() {
        let bounds = bounds_for(case);

        assert_eq!(bounds.start, case.expect_start, "{}", case.name);
        assert_eq!(bounds.len, case.expect_len, "{}", case.name);
        assert_eq!(bounds.end, bounds.start + bounds.len, "{}", case.name);
        assert_eq!(bounds.has_more, case.expect_has_more, "{}", case.name);
    }
}

#[test]
fn bounds_never_exceed_the_page_size_or_the_result_set() {
    for case in cases() {
        let bounds = bounds_for(case);

        assert!(bounds.start <= bounds.total_items, "{}", case.name);
        assert!(bounds.end <= bounds.total_items, "{}", case.name);
        assert!(bounds.len <= case.page_size, "{}", case.name);
        assert!(bounds.len <= bounds.total_items, "{}", case.name);
    }
}

#[test]
fn has_more_is_true_exactly_when_items_follow_the_page() {
    for case in cases() {
        let bounds = bounds_for(case);
        let expected = bounds.len > 0 && bounds.end < bounds.total_items;

        assert_eq!(bounds.has_more, expected, "{}", case.name);
    }
}

#[test]
fn empty_pages_have_zero_length_and_no_following_items() {
    for case in cases().into_iter().filter(|c| c.expect_len == 0) {
        let bounds = bounds_for(case);

        assert_eq!(bounds.start, bounds.end, "{}", case.name);
        assert!(!bounds.has_more, "{}", case.name);
    }
}

#[test]
fn the_first_page_starts_at_zero_and_reports_more() {
    let bounds = bounds_for(find_case("first_page"));

    assert_eq!(bounds.start, 0);
    assert_eq!(bounds.len, 3);
    assert_eq!((bounds.end, bounds.has_more), (3, true));
}

#[test]
fn a_middle_page_starts_at_the_requested_offset() {
    let bounds = bounds_for(find_case("middle_page"));

    assert_eq!(bounds.start, bounds.offset);
    assert_eq!(bounds.len, 3);
    assert!(bounds.has_more);
}

#[test]
fn the_last_page_finishes_the_result_set_and_reports_no_more() {
    for name in ["last_page_exact", "last_page_partial"] {
        let bounds = bounds_for(find_case(name));

        assert_eq!(bounds.end, bounds.total_items, "{name}");
        assert!(!bounds.has_more, "{name}");
    }
}

#[test]
fn an_offset_at_or_past_the_end_is_an_empty_page() {
    for name in ["exactly_at_end", "offset_past_end"] {
        let case = find_case(name);
        let bounds = bounds_for(case);

        assert_eq!(bounds.len, 0, "{name}");
        assert_eq!(bounds.end, bounds.start, "{name}");
        assert_eq!(bounds.start, case.total_items, "{name}");
        assert!(!bounds.has_more, "{name}");
    }
}

#[test]
fn a_zero_page_size_is_an_empty_page() {
    let bounds = bounds_for(find_case("zero_page_size"));

    assert_eq!(bounds.start, 0);
    assert_eq!(bounds.len, 0);
    assert_eq!(bounds.end, 0);
    assert!(!bounds.has_more);
}

#[test]
fn a_page_larger_than_the_set_returns_the_whole_set() {
    for name in ["full_set", "page_larger_than_set"] {
        let bounds = bounds_for(find_case(name));

        assert_eq!(bounds.start, 0, "{name}");
        assert_eq!(bounds.len, bounds.total_items, "{name}");
        assert!(!bounds.has_more, "{name}");
    }
}

#[test]
fn an_empty_result_set_always_yields_an_empty_page() {
    for name in ["empty_set", "empty_set_offset"] {
        let bounds = bounds_for(find_case(name));

        assert_eq!(bounds.total_items, 0, "{name}");
        assert_eq!((bounds.start, bounds.len), (0, 0), "{name}");
        assert!(!bounds.has_more, "{name}");
    }
}

#[test]
fn maximum_values_do_not_overflow() {
    let first = bounds_for(find_case("max_set_first_page"));
    assert_eq!((first.start, first.len, first.end), (0, 1, 1));
    assert!(first.has_more);

    let last = bounds_for(find_case("max_set_last_item"));
    assert_eq!((last.start, last.len, last.end), (u32::MAX - 1, 1, u32::MAX));
    assert!(!last.has_more);

    let whole = bounds_for(find_case("max_page_size"));
    assert_eq!((whole.start, whole.len, whole.end), (0, u32::MAX, u32::MAX));
    assert!(!whole.has_more);
}

#[test]
fn fixtures_are_stable_and_owned_across_calls() {
    assert_eq!(cases(), cases());

    let mut first = cases();
    first.clear();
    assert_eq!(cases().len(), DOCUMENTED_CASES.len());
}

#[test]
fn case_names_are_unique_and_non_empty() {
    let mut seen: Vec<String> = Vec::new();

    for case in cases() {
        assert!(!case.name.is_empty());
        let name = String::from(case.name);
        assert!(!seen.contains(&name), "duplicate case name: {name}");
        seen.push(name);
    }
}
