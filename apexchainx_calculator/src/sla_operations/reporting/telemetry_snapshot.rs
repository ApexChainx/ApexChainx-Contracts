//! Telemetry snapshot formatting.
//!
//! Turns per-severity counters into report rows with violation rates in basis
//! points. Rows are sorted by severity name. A `violations` counter larger than
//! `calculations` is clamped to `calculations`.

use alloc::{format, string::String, vec::Vec};

/// Raw counters for one severity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeverityCounters {
    pub severity: String,
    pub calculations: u32,
    pub violations: u32,
}

/// Report row for one severity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryRow {
    pub severity: String,
    pub calculations: u32,
    pub violations: u32,
    /// `violations * 10_000 / calculations`; `0` when `calculations == 0`.
    pub violation_rate_bps: u32,
}

/// A formatted snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryReport {
    pub rows: Vec<TelemetryRow>,
    pub total_calculations: u64,
    pub total_violations: u64,
    /// Overall rate across all rows, in bps; `0` when there are no calculations.
    pub overall_rate_bps: u32,
}

/// Format a basis-point value as a percentage with two decimals (`2050` -> `20.50%`).
pub fn format_bps(bps: u32) -> String {
    format!("{}.{:02}%", bps / 100, bps % 100)
}

/// Build the report for `counters`.
pub fn format_snapshot(counters: &[SeverityCounters]) -> TelemetryReport {
    let mut rows: Vec<TelemetryRow> = counters
        .iter()
        .map(|c| {
            let violations = c.violations.min(c.calculations);
            let rate = if c.calculations == 0 {
                0
            } else {
                (violations as u64 * 10_000 / c.calculations as u64) as u32
            };
            TelemetryRow {
                severity: c.severity.clone(),
                calculations: c.calculations,
                violations,
                violation_rate_bps: rate,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.severity.cmp(&b.severity));
    let total_calculations: u64 = rows.iter().map(|r| r.calculations as u64).sum();
    let total_violations: u64 = rows.iter().map(|r| r.violations as u64).sum();
    let overall_rate_bps = if total_calculations == 0 {
        0
    } else {
        (total_violations * 10_000 / total_calculations) as u32
    };
    TelemetryReport {
        rows,
        total_calculations,
        total_violations,
        overall_rate_bps,
    }
}

/// One human-readable line per row, e.g. `high: 10 calculations, 2 violations (20.00%)`.
pub fn render_lines(report: &TelemetryReport) -> Vec<String> {
    report
        .rows
        .iter()
        .map(|r| {
            format!(
                "{}: {} calculations, {} violations ({})",
                r.severity,
                r.calculations,
                r.violations,
                format_bps(r.violation_rate_bps)
            )
        })
        .collect()
}

#[cfg(test)]
mod telemetry_smoke_tests {
    use super::*;

    #[test]
    fn formats_rates_and_lines() {
        let report = format_snapshot(&[SeverityCounters {
            severity: String::from("high"),
            calculations: 10,
            violations: 2,
        }]);
        assert_eq!(report.rows[0].violation_rate_bps, 2_000);
        assert_eq!(format_bps(2_050), "20.50%");
        assert_eq!(render_lines(&report)[0], "high: 10 calculations, 2 violations (20.00%)");
    }
}
