//! Tests for `validation::invariants`.

use crate::sla_operations::validation::invariants::{
    check_bps, check_counts_add_up, check_net, check_shares_within_whole,
    check_strictly_ascending, InvariantError,
};

#[test]
fn bps_accepts_zero_through_ten_thousand() {
    assert_eq!(check_bps(0), Ok(()));
    assert_eq!(check_bps(10_000), Ok(()));
    assert_eq!(check_bps(10_001), Err(InvariantError::BpsOutOfRange));
    assert_eq!(check_bps(u32::MAX), Err(InvariantError::BpsOutOfRange));
}

#[test]
fn counts_must_add_up_exactly() {
    assert_eq!(check_counts_add_up(&[1, 2, 3], 6), Ok(()));
    assert_eq!(check_counts_add_up(&[], 0), Ok(()));
    assert_eq!(check_counts_add_up(&[1, 2, 3], 5), Err(InvariantError::CountMismatch));
    assert_eq!(check_counts_add_up(&[1, 2, 3], 7), Err(InvariantError::CountMismatch));
    assert_eq!(check_counts_add_up(&[], 1), Err(InvariantError::CountMismatch));
}

#[test]
fn counts_are_summed_without_overflow() {
    // Two u32::MAX parts exceed u32 but must be compared as u64, not wrap to a match.
    assert_eq!(
        check_counts_add_up(&[u32::MAX, u32::MAX], u32::MAX),
        Err(InvariantError::CountMismatch)
    );
    assert_eq!(check_counts_add_up(&[u32::MAX], u32::MAX), Ok(()));
}

#[test]
fn shares_may_total_less_than_the_whole_but_not_more() {
    assert_eq!(check_shares_within_whole(&[]), Ok(()));
    assert_eq!(check_shares_within_whole(&[3_333, 3_333, 3_333]), Ok(()));
    assert_eq!(check_shares_within_whole(&[5_000, 5_000]), Ok(()));
    assert_eq!(
        check_shares_within_whole(&[5_000, 5_001]),
        Err(InvariantError::SharesExceedWhole)
    );
    assert_eq!(
        check_shares_within_whole(&[u32::MAX, u32::MAX]),
        Err(InvariantError::SharesExceedWhole)
    );
}

#[test]
fn net_must_equal_rewards_minus_penalties() {
    assert_eq!(check_net(100, 40, 60), Ok(()));
    assert_eq!(check_net(0, 0, 0), Ok(()));
    assert_eq!(check_net(40, 100, -60), Ok(()));
    assert_eq!(check_net(100, 40, 61), Err(InvariantError::NetMismatch));
}

#[test]
fn net_overflow_is_a_mismatch_not_a_panic() {
    assert_eq!(check_net(i128::MAX, -1, i128::MAX), Err(InvariantError::NetMismatch));
    assert_eq!(check_net(i128::MIN, 1, i128::MIN), Err(InvariantError::NetMismatch));
}

#[test]
fn strictly_ascending_accepts_empty_single_and_increasing() {
    assert_eq!(check_strictly_ascending(&[]), Ok(()));
    assert_eq!(check_strictly_ascending(&[5]), Ok(()));
    assert_eq!(check_strictly_ascending(&[1, 2, 10]), Ok(()));
}

#[test]
fn strictly_ascending_rejects_duplicates_and_decreases() {
    assert_eq!(
        check_strictly_ascending(&[1, 1]),
        Err(InvariantError::NotStrictlyAscending)
    );
    assert_eq!(
        check_strictly_ascending(&[1, 3, 2]),
        Err(InvariantError::NotStrictlyAscending)
    );
    assert_eq!(
        check_strictly_ascending(&[2, 1]),
        Err(InvariantError::NotStrictlyAscending)
    );
}
