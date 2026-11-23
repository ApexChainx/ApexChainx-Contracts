//! Severity / count distribution fixtures.

use crate::sla_operations::reporting::severity_distribution::SeverityCount;
use alloc::{string::String, vec::Vec};

/// Build one count.
pub fn count(severity: &str, count: u32) -> SeverityCount {
    SeverityCount {
        severity: String::from(severity),
        count,
    }
}

/// 10 / 20 / 30 / 40 across the four canonical severities (total 100).
pub fn even_hundred() -> Vec<SeverityCount> {
    alloc::vec![
        count("critical", 10),
        count("high", 20),
        count("medium", 30),
        count("low", 40),
    ]
}

/// Everything in one severity.
pub fn single_severity() -> Vec<SeverityCount> {
    alloc::vec![count("high", 7)]
}

/// Counts that do not divide evenly: three severities with one each.
pub fn thirds() -> Vec<SeverityCount> {
    alloc::vec![count("a", 1), count("b", 1), count("c", 1)]
}

/// The same severity listed twice, to exercise merging.
pub fn duplicated_severity() -> Vec<SeverityCount> {
    alloc::vec![count("high", 3), count("low", 1), count("high", 4)]
}

/// All counts zero.
pub fn all_zero() -> Vec<SeverityCount> {
    alloc::vec![count("critical", 0), count("low", 0)]
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn fixtures_have_the_documented_totals() {
        let total = |v: &[SeverityCount]| v.iter().map(|c| c.count).sum::<u32>();
        assert_eq!(total(&even_hundred()), 100);
        assert_eq!(total(&single_severity()), 7);
        assert_eq!(total(&thirds()), 3);
        assert_eq!(total(&duplicated_severity()), 8);
        assert_eq!(total(&all_zero()), 0);
    }
}
