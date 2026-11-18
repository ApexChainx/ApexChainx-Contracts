//! Tests for `reporting::settlement_summary`.

use crate::sla_operations::reporting::settlement_summary::{
    build_settlement_summary, Settlement,
};
use alloc::string::String;

fn s(id: &str, amount: i128) -> Settlement {
    Settlement {
        outage_id: String::from(id),
        amount,
    }
}

#[test]
fn empty_input_is_all_zeros() {
    let sum = build_settlement_summary(&[]);
    assert_eq!(
        (sum.rewards_total, sum.penalties_total, sum.net),
        (0, 0, 0)
    );
    assert_eq!(
        (sum.reward_count, sum.penalty_count, sum.zero_count),
        (0, 0, 0)
    );
}

#[test]
fn rewards_and_penalties_are_split_with_positive_penalty_magnitudes() {
    let sum = build_settlement_summary(&[s("a", 750), s("b", -3_000), s("c", 600), s("d", -500)]);
    assert_eq!(sum.rewards_total, 1_350);
    assert_eq!(sum.penalties_total, 3_500);
    assert_eq!(sum.net, -2_150);
    assert_eq!((sum.reward_count, sum.penalty_count), (2, 2));
}

#[test]
fn zero_amounts_are_counted_separately_and_do_not_move_totals() {
    let sum = build_settlement_summary(&[s("a", 0), s("b", 0), s("c", 10)]);
    assert_eq!(sum.zero_count, 2);
    assert_eq!(sum.reward_count, 1);
    assert_eq!(sum.net, 10);
}

#[test]
fn net_is_zero_when_rewards_and_penalties_cancel() {
    let sum = build_settlement_summary(&[s("a", 400), s("b", -400)]);
    assert_eq!(sum.net, 0);
}

#[test]
fn net_always_equals_rewards_minus_penalties() {
    let sum = build_settlement_summary(&[s("a", 5), s("b", -9), s("c", 20), s("d", -1)]);
    assert_eq!(sum.net, sum.rewards_total - sum.penalties_total);
}

#[test]
fn counts_add_up_to_the_input_length() {
    let input = [s("a", 1), s("b", -1), s("c", 0), s("d", 3)];
    let sum = build_settlement_summary(&input);
    assert_eq!(
        sum.reward_count + sum.penalty_count + sum.zero_count,
        input.len() as u32
    );
}

#[test]
fn extreme_values_saturate_without_panicking() {
    let sum = build_settlement_summary(&[s("a", i128::MIN), s("b", i128::MAX)]);
    // i128::MIN has no positive counterpart, so the penalty saturates at MAX.
    assert_eq!(sum.penalties_total, i128::MAX);
    assert_eq!(sum.rewards_total, i128::MAX);
    assert_eq!(sum.net, 0);

    let big = build_settlement_summary(&[s("a", i128::MAX), s("b", i128::MAX)]);
    assert_eq!(big.rewards_total, i128::MAX);
}

#[test]
fn same_input_gives_same_output() {
    let input = [s("a", 7), s("b", -3)];
    assert_eq!(
        build_settlement_summary(&input),
        build_settlement_summary(&input)
    );
}
