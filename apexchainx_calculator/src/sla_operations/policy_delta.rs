//! Deterministic policy snapshot delta helpers for the SLA operations workspace.

/// A policy snapshot used for delta comparisons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicySnapshot {
    /// Threshold duration in seconds.
    pub threshold_secs: u64,
    /// Payout amount in basis points.
    pub payout_bps: u32,
    /// Whether the policy is enabled.
    pub enabled: bool,
}

/// Policy field that changed between two snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyField {
    /// Threshold duration in seconds.
    ThresholdSecs,
    /// Payout amount in basis points.
    PayoutBps,
    /// Enabled flag.
    Enabled,
}

/// A single changed policy field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyDelta {
    /// Field that changed.
    pub field: PolicyField,
    /// Previous value, with booleans encoded as zero or one.
    pub previous: u64,
    /// Current value, with booleans encoded as zero or one.
    pub current: u64,
}

/// Summary of which policy fields changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyDeltaSummary {
    /// Whether the threshold changed.
    pub threshold_changed: bool,
    /// Whether the payout changed.
    pub payout_changed: bool,
    /// Whether the enabled flag changed.
    pub enabled_changed: bool,
}

impl PolicyDeltaSummary {
    /// Returns the number of changed fields.
    #[must_use]
    pub const fn change_count(&self) -> u8 {
        self.threshold_changed as u8 + self.payout_changed as u8 + self.enabled_changed as u8
    }

    /// Returns whether no fields changed.
    #[must_use]
    pub const fn is_unchanged(&self) -> bool {
        self.change_count() == 0
    }
}

/// Compares two policy snapshots and reports which fields changed.
#[must_use]
pub const fn compare_policy(
    previous: PolicySnapshot,
    current: PolicySnapshot,
) -> PolicyDeltaSummary {
    PolicyDeltaSummary {
        threshold_changed: previous.threshold_secs != current.threshold_secs,
        payout_changed: previous.payout_bps != current.payout_bps,
        enabled_changed: previous.enabled != current.enabled,
    }
}

/// Writes changed-field descriptors into `out` and returns the number written.
#[must_use]
pub fn policy_changes(
    previous: PolicySnapshot,
    current: PolicySnapshot,
    out: &mut [PolicyDelta],
) -> usize {
    let mut written = 0;

    if previous.threshold_secs != current.threshold_secs && written < out.len() {
        out[written] = PolicyDelta {
            field: PolicyField::ThresholdSecs,
            previous: previous.threshold_secs,
            current: current.threshold_secs,
        };
        written += 1;
    }

    if previous.payout_bps != current.payout_bps && written < out.len() {
        out[written] = PolicyDelta {
            field: PolicyField::PayoutBps,
            previous: previous.payout_bps as u64,
            current: current.payout_bps as u64,
        };
        written += 1;
    }

    if previous.enabled != current.enabled && written < out.len() {
        out[written] = PolicyDelta {
            field: PolicyField::Enabled,
            previous: previous.enabled as u64,
            current: current.enabled as u64,
        };
        written += 1;
    }

    written
}
