//! Pagination and query-bound fixtures for the SLA operations workspace.
//!
//! A page request is an `(offset, page_size)` pair over a result set of
//! `total_items`. The fixtures use the same semantics as
//! `reporting::history_export::to_export_rows_page`:
//!
//! - a page size of zero yields an empty page,
//! - an offset at or past the end yields an empty page,
//! - the returned length never exceeds the page size, and
//! - the last page reports `has_more == false`.
//!
//! Nothing here reads contract state; fixtures are plain deterministic data.

/// The resolved bounds of one page request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageBounds {
    /// Total number of items in the queried result set.
    pub total_items: u32,
    /// Requested offset, preserved verbatim.
    pub offset: u32,
    /// Requested page size, preserved verbatim.
    pub page_size: u32,
    /// Index of the first returned item, clamped to `total_items`.
    pub start: u32,
    /// Number of returned items; zero for an empty page.
    pub len: u32,
    /// Index one past the last returned item.
    pub end: u32,
    /// Whether at least one item follows the returned page.
    pub has_more: bool,
}

/// One documented pagination scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaginationCase {
    /// Stable scenario name.
    pub name: &'static str,
    /// Total number of items in the result set.
    pub total_items: u32,
    /// Requested offset.
    pub offset: u32,
    /// Requested page size.
    pub page_size: u32,
    /// Expected index of the first returned item.
    pub expect_start: u32,
    /// Expected number of returned items.
    pub expect_len: u32,
    /// Expected `has_more` flag.
    pub expect_has_more: bool,
}

/// Resolves the bounds of a page request.
///
/// Out-of-range offsets and zero page sizes are treated as empty pages rather
/// than errors; the offset is still reported and `start` is clamped to
/// `total_items`. The computation uses only `u32` differences that are known to
/// be in range, so it cannot overflow at `u32::MAX`.
#[must_use]
pub const fn page_bounds(total_items: u32, offset: u32, page_size: u32) -> PageBounds {
    let start = if offset > total_items { total_items } else { offset };

    if page_size == 0 || offset >= total_items {
        return PageBounds {
            total_items,
            offset,
            page_size,
            start,
            len: 0,
            end: start,
            has_more: false,
        };
    }

    let remaining = total_items - offset;
    let len = if page_size < remaining {
        page_size
    } else {
        remaining
    };
    let end = offset + len;

    PageBounds {
        total_items,
        offset,
        page_size,
        start: offset,
        len,
        end,
        has_more: end < total_items,
    }
}

const fn case(
    name: &'static str,
    total_items: u32,
    offset: u32,
    page_size: u32,
    expect_start: u32,
    expect_len: u32,
    expect_has_more: bool,
) -> PaginationCase {
    PaginationCase {
        name,
        total_items,
        offset,
        page_size,
        expect_start,
        expect_len,
        expect_has_more,
    }
}

/// Documented pagination cases, from the first page through the limits.
#[must_use]
pub fn cases() -> alloc::vec::Vec<PaginationCase> {
    alloc::vec![
        case("first_page", 10, 0, 3, 0, 3, true),
        case("middle_page", 10, 3, 3, 3, 3, true),
        case("last_page_exact", 9, 6, 3, 6, 3, false),
        case("last_page_partial", 10, 9, 3, 9, 1, false),
        case("exactly_at_end", 10, 10, 3, 10, 0, false),
        case("offset_past_end", 10, 11, 3, 10, 0, false),
        case("zero_page_size", 10, 0, 0, 0, 0, false),
        case("single_item", 1, 0, 1, 0, 1, false),
        case("full_set", 10, 0, 10, 0, 10, false),
        case("page_larger_than_set", 10, 0, 100, 0, 10, false),
        case("empty_set", 0, 0, 5, 0, 0, false),
        case("empty_set_offset", 0, 1, 5, 0, 0, false),
        case("max_set_first_page", u32::MAX, 0, 1, 0, 1, true),
        case(
            "max_set_last_item",
            u32::MAX,
            u32::MAX - 1,
            1,
            u32::MAX - 1,
            1,
            false,
        ),
        case("max_page_size", u32::MAX, 0, u32::MAX, 0, u32::MAX, false),
    ]
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn fixtures_are_deterministic_and_bounds_hold() {
        assert_eq!(cases(), cases());

        for c in cases() {
            let b = page_bounds(c.total_items, c.offset, c.page_size);
            assert_eq!(
                (b.start, b.len, b.has_more),
                (c.expect_start, c.expect_len, c.expect_has_more)
            );
            assert!(b.end <= c.total_items, "{}", c.name);
            assert!(b.start + b.len == b.end, "{}", c.name);
            assert!(b.len <= c.page_size, "{}", c.name);
        }
    }
}
