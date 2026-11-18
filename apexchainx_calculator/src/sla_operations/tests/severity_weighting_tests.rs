use crate::sla_operations::severity_weighting::{
    average_weight_bps, total_weight_bps, weighted_score_bps, Severity,
};

#[test]
fn severity_weights_are_ordered() {
    assert_eq!(Severity::Low.weight_bps(), 2_500);
    assert_eq!(Severity::Medium.weight_bps(), 5_000);
    assert_eq!(Severity::High.weight_bps(), 7_500);
    assert_eq!(Severity::Critical.weight_bps(), 10_000);
}

#[test]
fn severity_labels_are_stable() {
    assert_eq!(Severity::Low.label(), "low");
    assert_eq!(Severity::Medium.label(), "medium");
    assert_eq!(Severity::High.label(), "high");
    assert_eq!(Severity::Critical.label(), "critical");
}

#[test]
fn weighted_scores_apply_basis_points() {
    assert_eq!(weighted_score_bps(Severity::Low, 8_000), 2_000);
    assert_eq!(weighted_score_bps(Severity::High, 8_000), 6_000);
    assert_eq!(weighted_score_bps(Severity::Critical, 8_000), 8_000);
    assert_eq!(weighted_score_bps(Severity::Critical, 0), 0);
}

#[test]
fn total_weight_sums_all_severities() {
    let severities = [Severity::Low, Severity::Medium, Severity::High, Severity::Critical];
    assert_eq!(total_weight_bps(&severities), 25_000);
    assert_eq!(average_weight_bps(&severities), Some(6_250));
}

#[test]
fn empty_severity_sequence_has_no_average() {
    assert_eq!(total_weight_bps(&[]), 0);
    assert_eq!(average_weight_bps(&[]), None);
}
