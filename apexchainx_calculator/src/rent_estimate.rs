//! Shared estimate struct carrying an approximation disclaimer (Issue #647).
//!
//! `get_rent_estimate` returns a proxy value; without a machine-readable flag
//! dashboards cannot distinguish a real network figure from a relative proxy.
//! This struct carries `is_approximation` so the disclaimer crosses the
//! contract boundary alongside the value.

use soroban_sdk::contracttype;

/// A storage or rent estimate value with an explicit approximation flag.
///
/// Both `get_rent_estimate` and footprint estimate endpoints return this
/// shape so consumers can tell whether the figure is a real network value
/// or a relative proxy without reading the source.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RentEstimate {
    /// The estimated value (units depend on the endpoint).
    pub value: u64,
    /// `true` when this figure is a heuristic proxy rather than a real
    /// network-derived rent cost. Dashboards must surface this to operators.
    pub is_approximation: bool,
    /// Human-readable description of the formula used to produce the value.
    pub formula: soroban_sdk::String,
}