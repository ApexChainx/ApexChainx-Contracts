use crate::sla_operations::policy_delta::{
    compare_policy, policy_changes, PolicyDelta, PolicyField, PolicySnapshot,
};

fn base_snapshot() -> PolicySnapshot {
    PolicySnapshot {
        threshold_secs: 3_600,
        payout_bps: 500,
        enabled: true,
    }
}

#[test]
fn identical_snapshots_have_no_changes() {
    let snapshot = base_snapshot();
    let summary = compare_policy(snapshot, snapshot);

    assert_eq!(summary.change_count(), 0);
    assert!(summary.is_unchanged());
    assert!(!summary.threshold_changed);
    assert!(!summary.payout_changed);
    assert!(!summary.enabled_changed);
}

#[test]
fn summary_reports_each_changed_field() {
    let previous = base_snapshot();
    let current = PolicySnapshot {
        threshold_secs: 7_200,
        payout_bps: 750,
        enabled: false,
    };

    let summary = compare_policy(previous, current);
    assert_eq!(summary.change_count(), 3);
    assert!(!summary.is_unchanged());
    assert!(summary.threshold_changed);
    assert!(summary.payout_changed);
    assert!(summary.enabled_changed);
}

#[test]
fn partial_changes_are_isolated() {
    let previous = base_snapshot();
    let current = PolicySnapshot {
        threshold_secs: 3_600,
        payout_bps: 900,
        enabled: true,
    };

    let summary = compare_policy(previous, current);
    assert_eq!(summary.change_count(), 1);
    assert!(!summary.threshold_changed);
    assert!(summary.payout_changed);
    assert!(!summary.enabled_changed);
}

#[test]
fn changed_fields_are_written_in_stable_order() {
    let previous = base_snapshot();
    let current = PolicySnapshot {
        threshold_secs: 7_200,
        payout_bps: 750,
        enabled: false,
    };
    let mut out = [PolicyDelta {
        field: PolicyField::ThresholdSecs,
        previous: 0,
        current: 0,
    }; 3];

    assert_eq!(policy_changes(previous, current, &mut out), 3);
    assert_eq!(
        out,
        [
            PolicyDelta {
                field: PolicyField::ThresholdSecs,
                previous: 3_600,
                current: 7_200,
            },
            PolicyDelta {
                field: PolicyField::PayoutBps,
                previous: 500,
                current: 750,
            },
            PolicyDelta {
                field: PolicyField::Enabled,
                previous: 1,
                current: 0,
            },
        ]
    );
}

#[test]
fn output_buffer_is_never_overrun() {
    let previous = base_snapshot();
    let current = PolicySnapshot {
        threshold_secs: 7_200,
        payout_bps: 750,
        enabled: false,
    };
    let mut out = [PolicyDelta {
        field: PolicyField::Enabled,
        previous: 9,
        current: 9,
    }; 1];

    assert_eq!(policy_changes(previous, current, &mut out), 1);
    assert_eq!(
        out[0],
        PolicyDelta {
            field: PolicyField::ThresholdSecs,
            previous: 3_600,
            current: 7_200,
        }
    );
}
