use crate::sla_operations::rolling_windows::{
    aggregate_window, window_average, window_max, window_min, window_sum, WindowAggregate,
};

#[test]
fn window_sum_and_average_use_the_requested_range() {
    let samples = [10_u64, 20, 30, 40, 50];

    assert_eq!(window_sum(&samples, 1, 3), Some(90));
    assert_eq!(window_average(&samples, 1, 3), Some(30));
    assert_eq!(window_sum(&samples, 4, 1), Some(50));
}

#[test]
fn empty_and_out_of_range_windows_are_rejected() {
    let samples = [10_u64, 20, 30];

    assert_eq!(window_sum(&samples, 0, 0), None);
    assert_eq!(window_average(&samples, 0, 0), None);
    assert_eq!(window_sum(&samples, 2, 2), None);
    assert_eq!(window_average(&samples, 4, 1), None);
}

#[test]
fn window_sums_saturate_instead_of_wrapping() {
    let samples = [u64::MAX, 1];

    assert_eq!(window_sum(&samples, 0, 2), Some(u64::MAX));
}

#[test]
fn window_extremes_report_the_expected_values() {
    let samples = [7_u64, 3, 9, 1, 5];

    assert_eq!(window_max(&samples, 0, 5), Some(9));
    assert_eq!(window_min(&samples, 0, 5), Some(1));
    assert_eq!(window_max(&samples, 3, 2), Some(5));
    assert_eq!(window_min(&samples, 3, 2), Some(1));
}

#[test]
fn aggregate_window_combines_all_fields() {
    let samples = [2_u64, 4, 6, 8];

    assert_eq!(
        aggregate_window(&samples, 1, 3),
        Some(WindowAggregate {
            sum: 18,
            count: 3,
            average: Some(6),
        })
    );
    assert_eq!(aggregate_window(&samples, 0, 0), None);
}
