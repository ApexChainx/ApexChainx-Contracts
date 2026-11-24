//! End-to-end composition tests for the SLA operations workspace.
//!
//! Each test drives several workspace modules from the same fixture data and
//! checks that their independently computed results agree. Nothing here touches
//! contract storage, entrypoints, or events; every input is a deterministic
//! fixture owned by the test.

use crate::sla_operations::compliance_score::compliance_score_bps;
use crate::sla_operations::counter_deltas::{counter_delta, summarize_counters, CounterMovement};
use crate::sla_operations::duration_utils::{
    clamp_duration, normalize_duration, total_duration, DurationWindow,
};
use crate::sla_operations::fixtures::anomaly_scenarios::{scenarios as anomaly_scenarios, AnomalyPattern};
use crate::sla_operations::fixtures::boundary_cases::{mttr_cases, ratio_cases};
use crate::sla_operations::fixtures::history_records::mixed_history;
use crate::sla_operations::fixtures::pagination_cases::page_bounds;
use crate::sla_operations::fixtures::sla_inputs::sample_inputs;
use crate::sla_operations::fixtures::threshold_configs::{config_for, default_configs};
use crate::sla_operations::mttr_classification::{classify_mttr, meets_mttr_threshold, MttrBand};
use crate::sla_operations::policy_delta::{
    compare_policy, policy_changes, PolicyDelta, PolicyField, PolicySnapshot,
};
use crate::sla_operations::reporting::compliance_report::{build_report, worst_first, ComplianceEntry};
use crate::sla_operations::reporting::health_indicators::{derive, HealthLevel, SlaMetrics};
use crate::sla_operations::reporting::history_export::to_export_rows_page;
use crate::sla_operations::reporting::severity_distribution::{
    build_distribution, total_count, SeverityCount,
};
use crate::sla_operations::reporting::sla_summary::{build_summary, SlaOutcome};
use crate::sla_operations::rolling_windows::{
    aggregate_window, window_average, window_max, window_min, window_sum,
};
use crate::sla_operations::severity_weighting::{
    average_weight_bps, total_weight_bps, weighted_score_bps, Severity,
};
use crate::sla_operations::violation_rate::{compliance_rate_bps, violation_rate_bps};
use alloc::{string::String, vec, vec::Vec};

fn threshold_for(severity: &str) -> u32 {
    let configs = default_configs();
    config_for(&configs, severity)
        .expect("sample input severity is configured")
        .threshold_minutes
}

fn entries_from_sample_inputs() -> Vec<ComplianceEntry> {
    sample_inputs()
        .into_iter()
        .map(|input| ComplianceEntry {
            outage_id: input.outage_id,
            severity: input.severity.clone(),
            mttr_minutes: input.mttr_minutes,
            threshold_minutes: threshold_for(&input.severity),
        })
        .collect()
}

fn outcomes_from_entries(entries: &[ComplianceEntry]) -> Vec<SlaOutcome> {
    entries
        .iter()
        .map(|entry| SlaOutcome {
            severity: entry.severity.clone(),
            met: entry.mttr_minutes <= entry.threshold_minutes,
            mttr_minutes: entry.mttr_minutes,
        })
        .collect()
}

#[test]
fn sla_inputs_feed_consistent_summary_compliance_and_health_reports() {
    let entries = entries_from_sample_inputs();
    let outcomes = outcomes_from_entries(&entries);
    let report = build_report(&entries);
    let summary = build_summary(&outcomes);

    assert_eq!(entries.len(), 8);
    assert_eq!(report.checked, 8);
    assert_eq!((report.compliant, report.violations.len()), (6, 2));
    assert_eq!(report.compliance_bps, 7_500);
    assert_eq!((summary.total, summary.met, summary.violated), (8, 6, 2));
    assert_eq!(summary.compliance_bps, report.compliance_bps);
    assert_eq!((summary.avg_mttr_minutes, summary.worst_mttr_minutes), (35, 90));
    assert_eq!(summary.total_mttr_minutes, 285);

    let health = derive(SlaMetrics {
        total: summary.total,
        violations: summary.violated,
        total_mttr_minutes: summary.total_mttr_minutes,
    });
    assert_eq!(health.level, HealthLevel::Critical);
    assert_eq!(health.compliance_bps, summary.compliance_bps);
    assert_eq!(health.avg_mttr_minutes, summary.avg_mttr_minutes);
    assert_eq!(health.violation_bps, 2_500);
    assert_eq!(
        compliance_score_bps(u64::from(report.compliant), 2),
        Some(report.compliance_bps)
    );
    assert_eq!(violation_rate_bps(2, 8), Some(health.violation_bps));
    assert_eq!(compliance_rate_bps(2, 8), Some(health.compliance_bps));

    let ranked = worst_first(&report);
    assert_eq!(
        ranked
            .iter()
            .map(|line| (line.outage_id.as_str(), line.overage_minutes))
            .collect::<Vec<_>>(),
        vec![("critical-2", 30), ("medium-1", 30)]
    );
}

#[test]
fn boundary_cases_agree_with_compliance_summary_and_classification() {
    for case in mttr_cases() {
        let entries = [ComplianceEntry {
            outage_id: String::from(case.name),
            severity: String::from("critical"),
            mttr_minutes: case.mttr_minutes,
            threshold_minutes: case.threshold_minutes,
        }];
        let report = build_report(&entries);
        let summary = build_summary(&outcomes_from_entries(&entries));
        let classification = classify_mttr(
            u64::from(case.mttr_minutes),
            u64::from(case.threshold_minutes),
            u64::from(case.threshold_minutes),
        )
        .expect("warning and breach thresholds are equal and non-zero except max");

        assert_eq!(report.compliant == 1, case.expect_met, "{}", case.name);
        assert_eq!(summary.met == 1, case.expect_met, "{}", case.name);
        assert_eq!(
            report.compliance_bps,
            if case.expect_met { 10_000 } else { 0 },
            "{}",
            case.name
        );
        assert_eq!(
            meets_mttr_threshold(u64::from(case.mttr_minutes), u64::from(case.threshold_minutes)),
            case.expect_met,
            "{}",
            case.name
        );
        assert_eq!(classification.exceeded_breach, !case.expect_met, "{}", case.name);
        assert_eq!(
            classification.band,
            if case.expect_met {
                MttrBand::Healthy
            } else {
                MttrBand::Breached
            },
            "{}",
            case.name
        );
    }

    for (numerator, denominator, expected_bps) in ratio_cases() {
        let rate = violation_rate_bps(u64::from(numerator), u64::from(denominator));

        if denominator == 0 {
            assert_eq!(rate, None, "{numerator}/{denominator}");
            assert_eq!(compliance_rate_bps(u64::from(numerator), 0), None);
        } else {
            assert_eq!(rate, Some(expected_bps), "{numerator}/{denominator}");
            assert_eq!(
                compliance_rate_bps(u64::from(numerator), u64::from(denominator)),
                Some(10_000 - expected_bps)
            );
        }
    }
}

#[test]
fn history_pagination_matches_resolved_page_bounds() {
    let records = mixed_history();
    let total = records.len() as u32;

    assert_eq!(total, 3);
    // (offset, page_size, expected_len, expected_has_more)
    let requests = [
        (0_u32, 1_u32, 1_u32, true),
        (0, 3, 3, false),
        (1, 1, 1, true),
        (2, 2, 1, false),
        (3, 2, 0, false),
        (0, 0, 0, false),
        (0, 5, 3, false),
        (4, 5, 0, false),
    ];

    for (offset, page_size, expected_len, expected_has_more) in requests {
        let bounds = page_bounds(total, offset, page_size);
        let page = to_export_rows_page(&records, offset, page_size);

        assert_eq!(bounds.len, expected_len, "offset={offset}");
        assert_eq!(bounds.has_more, expected_has_more, "offset={offset}");
        assert_eq!(page.len() as u32, bounds.len, "offset={offset}");
        assert_eq!(page.len() as u32, expected_len, "offset={offset}");

        if let Some(first) = page.first() {
            assert_eq!(first.index, bounds.start, "offset={offset}");
        }
        if let Some(last) = page.last() {
            assert_eq!(last.index + 1, bounds.end, "offset={offset}");
        }

        let start = (offset as usize).min(records.len());
        let expected: Vec<&str> = records[start..start + expected_len as usize]
            .iter()
            .map(|record| record.outage_id.as_str())
            .collect();
        let rendered: Vec<&str> = page.iter().map(|row| row.record.outage_id.as_str()).collect();
        assert_eq!(rendered, expected, "offset={offset}");
    }
}

#[test]
fn anomaly_scenarios_agree_with_health_indicators_and_rates() {
    for scenario in anomaly_scenarios() {
        let total = u64::from(scenario.total);
        let violations = u64::from(scenario.violations);
        let met = u64::from(scenario.met());
        let metrics = SlaMetrics {
            total: scenario.total,
            violations: scenario.violations,
            total_mttr_minutes: u64::from(scenario.mttr_minutes) * total,
        };
        let health = derive(metrics);

        assert_eq!(scenario.is_anomalous(), scenario.expect_anomalous);

        if scenario.total == 0 {
            assert_eq!(health.level, HealthLevel::NoData, "{}", scenario.name);
            assert_eq!(violation_rate_bps(0, 0), None);
            assert_eq!(health.compliance_bps, 0);
        } else {
            assert_eq!(
                health.violation_bps,
                violation_rate_bps(violations, total).expect("valid scenario counts"),
                "{}",
                scenario.name
            );
            assert_eq!(
                health.compliance_bps,
                compliance_score_bps(met, violations).expect("non-empty scenario"),
                "{}",
                scenario.name
            );
        }

        if scenario.pattern == AnomalyPattern::CompleteViolation {
            assert_eq!(health.level, HealthLevel::Critical, "{}", scenario.name);
            assert_eq!(health.violation_bps, 10_000);
        }
    }
}

#[test]
fn severity_distribution_agrees_with_weighted_severity_helpers() {
    let inputs = sample_inputs();
    let mut counts: Vec<SeverityCount> = Vec::new();
    for input in &inputs {
        match counts.iter_mut().find(|count| count.severity == input.severity) {
            Some(count) => count.count += 1,
            None => counts.push(SeverityCount {
                severity: input.severity.clone(),
                count: 1,
            }),
        }
    }

    let rows = build_distribution(&counts);
    assert_eq!(total_count(&rows), 8);
    assert_eq!(
        rows.iter()
            .map(|row| (row.severity.as_str(), row.count, row.share_bps))
            .collect::<Vec<_>>(),
        vec![
            ("critical", 3, 3_750),
            ("high", 2, 2_500),
            ("low", 1, 1_250),
            ("medium", 2, 2_500),
        ]
    );

    let severities = [
        Severity::Critical,
        Severity::Critical,
        Severity::Critical,
        Severity::High,
        Severity::High,
        Severity::Medium,
        Severity::Medium,
        Severity::Low,
    ];
    assert_eq!(total_weight_bps(&severities), 57_500);
    assert_eq!(average_weight_bps(&severities), Some(7_187));
    assert_eq!(weighted_score_bps(Severity::Critical, 7_500), 7_500);
    assert_eq!(weighted_score_bps(Severity::Low, 7_500), 1_875);
}

#[test]
fn policy_changes_and_counter_deltas_describe_the_same_movement() {
    let previous = PolicySnapshot {
        threshold_secs: 3_600,
        payout_bps: 500,
        enabled: true,
    };
    let current = PolicySnapshot {
        threshold_secs: 3_000,
        payout_bps: 750,
        enabled: false,
    };

    let summary = compare_policy(previous, current);
    assert_eq!(summary.change_count(), 3);
    assert!(!summary.is_unchanged());
    assert!(summary.threshold_changed && summary.payout_changed && summary.enabled_changed);

    let mut deltas = [PolicyDelta {
        field: PolicyField::ThresholdSecs,
        previous: 0,
        current: 0,
    }; 3];
    let written = policy_changes(previous, current, &mut deltas);
    assert_eq!(written, 3);
    assert_eq!(deltas[0].field, PolicyField::ThresholdSecs);
    assert_eq!((deltas[0].previous, deltas[0].current), (3_600, 3_000));
    assert_eq!(deltas[1].field, PolicyField::PayoutBps);
    assert_eq!((deltas[1].previous, deltas[1].current), (500, 750));
    assert_eq!(deltas[2].field, PolicyField::Enabled);
    assert_eq!((deltas[2].previous, deltas[2].current), (1, 0));

    let samples = [
        (previous.threshold_secs, current.threshold_secs),
        (u64::from(previous.payout_bps), u64::from(current.payout_bps)),
        (previous.enabled as u64, current.enabled as u64),
    ];
    let counters = summarize_counters(&samples);
    assert_eq!(
        (counters.increases, counters.decreases, counters.unchanged),
        (1, 2, 0)
    );
    assert_eq!(counters.growth_total, 250);
    assert_eq!(counters.decline_total, 601);
    assert_eq!(
        counter_delta(current.threshold_secs, previous.threshold_secs).movement,
        CounterMovement::Decreased
    );
}

#[test]
fn rolling_windows_and_duration_helpers_agree_on_totals_and_bounds() {
    let durations = [10_u64, 20, 30, 40];

    assert_eq!(total_duration(&durations), 100);
    let aggregate = aggregate_window(&durations, 0, 4).expect("full window");
    assert_eq!(aggregate.sum, total_duration(&durations));
    assert_eq!(aggregate.count, 4);
    assert_eq!(aggregate.average, Some(25));

    assert_eq!(window_sum(&durations, 1, 2), Some(50));
    assert_eq!(window_average(&durations, 1, 2), Some(25));
    assert_eq!(window_max(&durations, 0, 3), Some(30));
    assert_eq!(window_min(&durations, 1, 3), Some(20));
    assert_eq!(window_sum(&durations, 3, 2), None);
    assert_eq!(window_sum(&durations, 0, 0), None);
    assert_eq!(aggregate_window(&durations, 4, 0), None);

    let window = DurationWindow::new(10, 40).expect("ordered bounds");
    assert_eq!(window.duration(), 30);
    assert!(window.overlaps(DurationWindow::new(39, 50).expect("ordered bounds")));
    assert!(!window.overlaps(DurationWindow::new(40, 50).expect("ordered bounds")));
    assert_eq!(normalize_duration(10, 40), Some(30));
    assert_eq!(normalize_duration(40, 10), None);
    assert_eq!(clamp_duration(100, 60), 60);
    assert_eq!(clamp_duration(50, 60), 50);
}
