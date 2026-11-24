//! Index of reusable fixture modules for the SLA operations workspace.

/// Description of a fixture module exposed by the workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixtureModule {
    /// Module name used by callers.
    pub name: &'static str,
    /// Repository-relative path to the module.
    pub path: &'static str,
    /// Short description of the fixture data provided by the module.
    pub purpose: &'static str,
}

/// Known fixture modules in the SLA operations workspace.
pub const FIXTURE_MODULES: &[FixtureModule] = &[
    FixtureModule {
        name: "golden_vectors",
        path: "fixtures/golden_vectors.rs",
        purpose: "Expected and observed value pairs for deterministic comparisons",
    },
    FixtureModule {
        name: "policy_snapshots",
        path: "fixtures/policy_snapshots.rs",
        purpose: "Sample policy and configuration snapshots",
    },
    FixtureModule {
        name: "pagination_cases",
        path: "fixtures/pagination_cases.rs",
        purpose: "Pagination and query-bound scenarios",
    },
    FixtureModule {
        name: "anomaly_scenarios",
        path: "fixtures/anomaly_scenarios.rs",
        purpose: "Anomalous SLA metric patterns",
    },
    FixtureModule {
        name: "report_expectations",
        path: "fixtures/report_expectations.rs",
        purpose: "Expected report and formatter outputs",
    },
];

/// Looks up a fixture module by name.
#[must_use]
pub fn find_fixture_module(name: &str) -> Option<&'static FixtureModule> {
    FIXTURE_MODULES.iter().find(|module| module.name == name)
}

/// Returns whether a fixture module with `name` is registered.
#[must_use]
pub fn has_fixture_module(name: &str) -> bool {
    find_fixture_module(name).is_some()
}

pub mod sla_inputs;

pub mod anomaly_scenarios;
pub mod pagination_cases;

pub mod history_records;
pub mod settlement_scenarios;
pub mod telemetry_samples;
pub mod threshold_configs;

pub mod boundary_cases;
pub mod invalid_inputs;
pub mod severity_distributions;
pub mod time_series;
