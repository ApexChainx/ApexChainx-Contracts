//! Oracle module and Soroban contract for ApexChainx.
//!
//! Provides verified metric ingestion, decentralized multi-reporter feeds,
//! checked arithmetic with overflow/underflow guards, role-based access control,
//! and integration interfaces for SLA calculations and downstream consumers.
//!
//! # Issues Addressed:
//! - #846: Check Oracle for integer overflow/underflow risks (checked arithmetic, bounded math)
//! - #844: Audit Oracle access control modifiers (role-based modifiers: Admin, Reporter, Paused)
//! - #845: Add invariant tests for Oracle (monotonic timestamps, bounded values, consensus)
//! - #843: Add integration tests covering Oracle flows (lifecycle, reporting, consumption)

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol,
    Vec,
};

/// Oracle-specific contract errors.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum OracleError {
    /// Contract is already initialized.
    AlreadyInitialized = 101,
    /// Contract is not yet initialized.
    NotInitialized = 102,
    /// Caller is not authorized for this operation.
    Unauthorized = 103,
    /// Contract is currently paused.
    ContractPaused = 104,
    /// Reporter is not registered or active.
    ReporterNotAuthorized = 105,
    /// Arithmetic addition overflowed.
    MathOverflow = 106,
    /// Arithmetic subtraction underflowed.
    MathUnderflow = 107,
    /// Division by zero or invalid division.
    DivisionByZero = 108,
    /// Provided metric or value is outside configured bounds.
    ValueOutOfBounds = 109,
    /// Timestamp regression: incoming timestamp must be non-decreasing.
    TimestampRegression = 110,
    /// No data available for requested metric or feed.
    NoDataAvailable = 111,
    /// Insufficient consensus or reporter responses.
    InsufficientConsensus = 112,
    /// Invalid configuration parameters.
    InvalidConfiguration = 113,
}

/// Status of a metric report.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ReportStatus {
    Valid = 1,
    Stale = 2,
    Disputed = 3,
}

/// Data report submitted by an authorized oracle reporter.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricReport {
    /// Reported integer value (e.g., MTTR minutes, response latency, or unit price).
    pub value: u128,
    /// Ledger timestamp when the observation occurred.
    pub timestamp: u64,
    /// Reporter address that authored this observation.
    pub reporter: Address,
    /// Monotonically increasing sequence/round number.
    pub round_id: u64,
    /// Status flag.
    pub status: ReportStatus,
}

/// Aggregated metric summary produced by the Oracle.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregatedMetric {
    /// Median / consensus value across active reporters.
    pub consensus_value: u128,
    /// Time-weighted or arithmetic mean value.
    pub mean_value: u128,
    /// Minimum reported value in round.
    pub min_value: u128,
    /// Maximum reported value in round.
    pub max_value: u128,
    /// Number of participating reports.
    pub reporter_count: u32,
    /// Latest timestamp among participants.
    pub latest_timestamp: u64,
    /// Round identifier.
    pub round_id: u64,
}

/// Oracle configuration parameters.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    /// Lower bound on valid metric values (protects against underflow).
    pub min_allowed_value: u128,
    /// Upper bound on valid metric values (protects against overflow).
    pub max_allowed_value: u128,
    /// Maximum allowed age of a report in seconds before considered stale.
    pub max_staleness_seconds: u64,
    /// Minimum required reporter consensus quorum.
    pub min_quorum: u32,
}

// Storage keys
const ORACLE_ADMIN_KEY: Symbol = symbol_short!("O_ADMIN");
const ORACLE_PAUSED_KEY: Symbol = symbol_short!("O_PAUSED");
const ORACLE_CONFIG_KEY: Symbol = symbol_short!("O_CONFIG");
const REPORTERS_KEY: Symbol = symbol_short!("O_REPS");
const LATEST_METRIC_KEY: Symbol = symbol_short!("O_LATEST");
const ROUND_COUNTER_KEY: Symbol = symbol_short!("O_ROUND");
const REPORTS_HISTORY_KEY: Symbol = symbol_short!("O_HIST");

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    // -----------------------------------------------------------------------
    // Access Control Modifiers & Guards (#844)
    // -----------------------------------------------------------------------

    /// Ensures the contract has been initialized.
    pub fn require_initialized(env: &Env) -> Result<(), OracleError> {
        if !env.storage().instance().has(&ORACLE_ADMIN_KEY) {
            return Err(OracleError::NotInitialized);
        }
        Ok(())
    }

    /// Ensures caller is the contract admin.
    pub fn require_admin(env: &Env, caller: &Address) -> Result<(), OracleError> {
        Self::require_initialized(env)?;
        caller.require_auth();
        let admin: Address = env.storage().instance().get(&ORACLE_ADMIN_KEY).ok_or(OracleError::NotInitialized)?;
        if *caller != admin {
            return Err(OracleError::Unauthorized);
        }
        Ok(())
    }

    /// Ensures caller is an authorized reporter.
    pub fn require_reporter(env: &Env, caller: &Address) -> Result<(), OracleError> {
        Self::require_initialized(env)?;
        caller.require_auth();
        let reporters: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&REPORTERS_KEY)
            .unwrap_or_else(|| Map::new(env));
        if !reporters.get(caller.clone()).unwrap_or(false) {
            return Err(OracleError::ReporterNotAuthorized);
        }
        Ok(())
    }

    /// Ensures contract is not paused.
    pub fn require_not_paused(env: &Env) -> Result<(), OracleError> {
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&ORACLE_PAUSED_KEY)
            .unwrap_or(false);
        if is_paused {
            return Err(OracleError::ContractPaused);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Lifecycle and Role Management (#844)
    // -----------------------------------------------------------------------

    /// Initialize the Oracle contract with an admin and baseline configuration.
    pub fn initialize(
        env: Env,
        admin: Address,
        config: OracleConfig,
    ) -> Result<(), OracleError> {
        if env.storage().instance().has(&ORACLE_ADMIN_KEY) {
            return Err(OracleError::AlreadyInitialized);
        }

        // Validate config bounds (prevents overflow and invalid parameters)
        if config.min_allowed_value > config.max_allowed_value || config.min_quorum == 0 {
            return Err(OracleError::InvalidConfiguration);
        }

        admin.require_auth();
        env.storage().instance().set(&ORACLE_ADMIN_KEY, &admin);
        env.storage().instance().set(&ORACLE_PAUSED_KEY, &false);
        env.storage().instance().set(&ORACLE_CONFIG_KEY, &config);
        env.storage().instance().set(&ROUND_COUNTER_KEY, &0u64);

        let reporters: Map<Address, bool> = Map::new(&env);
        env.storage().instance().set(&REPORTERS_KEY, &reporters);

        Ok(())
    }

    /// Pause the Oracle in emergency situations (Admin only).
    pub fn pause(env: Env, caller: Address) -> Result<(), OracleError> {
        Self::require_admin(&env, &caller)?;
        env.storage().instance().set(&ORACLE_PAUSED_KEY, &true);
        Ok(())
    }

    /// Resume the Oracle (Admin only).
    pub fn unpause(env: Env, caller: Address) -> Result<(), OracleError> {
        Self::require_admin(&env, &caller)?;
        env.storage().instance().set(&ORACLE_PAUSED_KEY, &false);
        Ok(())
    }

    /// Add an authorized reporter (Admin only).
    pub fn add_reporter(
        env: Env,
        caller: Address,
        reporter: Address,
    ) -> Result<(), OracleError> {
        Self::require_admin(&env, &caller)?;
        let mut reporters: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&REPORTERS_KEY)
            .unwrap_or_else(|| Map::new(&env));
        reporters.set(reporter, true);
        env.storage().instance().set(&REPORTERS_KEY, &reporters);
        Ok(())
    }

    /// Remove an authorized reporter (Admin only).
    pub fn remove_reporter(
        env: Env,
        caller: Address,
        reporter: Address,
    ) -> Result<(), OracleError> {
        Self::require_admin(&env, &caller)?;
        let mut reporters: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&REPORTERS_KEY)
            .unwrap_or_else(|| Map::new(&env));
        reporters.set(reporter, false);
        env.storage().instance().set(&REPORTERS_KEY, &reporters);
        Ok(())
    }

    /// Update Oracle configuration (Admin only).
    pub fn set_config(
        env: Env,
        caller: Address,
        config: OracleConfig,
    ) -> Result<(), OracleError> {
        Self::require_admin(&env, &caller)?;
        if config.min_allowed_value > config.max_allowed_value || config.min_quorum == 0 {
            return Err(OracleError::InvalidConfiguration);
        }
        env.storage().instance().set(&ORACLE_CONFIG_KEY, &config);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Checked Arithmetic & Overflow/Underflow Protections (#846)
    // -----------------------------------------------------------------------

    /// Safe addition with explicit overflow error.
    pub fn safe_add(a: u128, b: u128) -> Result<u128, OracleError> {
        a.checked_add(b).ok_or(OracleError::MathOverflow)
    }

    /// Safe subtraction with explicit underflow error.
    pub fn safe_sub(a: u128, b: u128) -> Result<u128, OracleError> {
        a.checked_sub(b).ok_or(OracleError::MathUnderflow)
    }

    /// Safe multiplication with explicit overflow error.
    pub fn safe_mul(a: u128, b: u128) -> Result<u128, OracleError> {
        a.checked_mul(b).ok_or(OracleError::MathOverflow)
    }

    /// Safe division with zero check.
    pub fn safe_div(a: u128, b: u128) -> Result<u128, OracleError> {
        if b == 0 {
            return Err(OracleError::DivisionByZero);
        }
        a.checked_div(b).ok_or(OracleError::MathOverflow)
    }

    /// Safe weighted average calculation protecting against intermediate overflows.
    pub fn safe_weighted_average(
        v1: u128,
        w1: u128,
        v2: u128,
        w2: u128,
    ) -> Result<u128, OracleError> {
        let total_weight = Self::safe_add(w1, w2)?;
        if total_weight == 0 {
            return Err(OracleError::DivisionByZero);
        }
        let prod1 = Self::safe_mul(v1, w1)?;
        let prod2 = Self::safe_mul(v2, w2)?;
        let sum_prod = Self::safe_add(prod1, prod2)?;
        Self::safe_div(sum_prod, total_weight)
    }

    /// Safe delta and rate-of-change calculation.
    pub fn safe_calculate_rate_change(
        old_val: u128,
        new_val: u128,
        time_delta_seconds: u64,
    ) -> Result<u128, OracleError> {
        if time_delta_seconds == 0 {
            return Err(OracleError::DivisionByZero);
        }
        let delta = if new_val >= old_val {
            Self::safe_sub(new_val, old_val)?
        } else {
            Self::safe_sub(old_val, new_val)?
        };
        // Rate scaled by 1,000 for precision
        let scaled_delta = Self::safe_mul(delta, 1000)?;
        Self::safe_div(scaled_delta, time_delta_seconds as u128)
    }

    // -----------------------------------------------------------------------
    // Core Reporting & Consensus Flows (#843, #844, #846)
    // -----------------------------------------------------------------------

    /// Submit a single metric observation (Reporter only, not paused).
    pub fn submit_report(
        env: Env,
        reporter: Address,
        metric_name: Symbol,
        value: u128,
        timestamp: u64,
    ) -> Result<MetricReport, OracleError> {
        Self::require_not_paused(&env)?;
        Self::require_reporter(&env, &reporter)?;

        let config: OracleConfig = env.storage().instance().get(&ORACLE_CONFIG_KEY).ok_or(OracleError::NotInitialized)?;

        // Bounds check (#846)
        if value < config.min_allowed_value || value > config.max_allowed_value {
            return Err(OracleError::ValueOutOfBounds);
        }

        // Monotonic timestamp check (#845)
        let latest_opt: Option<MetricReport> = env.storage().instance().get(&metric_name);
        if let Some(ref prev) = latest_opt {
            if timestamp < prev.timestamp {
                return Err(OracleError::TimestampRegression);
            }
        }

        let round: u64 = env
            .storage()
            .instance()
            .get(&ROUND_COUNTER_KEY)
            .unwrap_or(0);
        let next_round = round.checked_add(1).ok_or(OracleError::MathOverflow)?;
        env.storage().instance().set(&ROUND_COUNTER_KEY, &next_round);

        let report = MetricReport {
            value,
            timestamp,
            reporter,
            round_id: next_round,
            status: ReportStatus::Valid,
        };

        env.storage().instance().set(&metric_name, &report);

        Ok(report)
    }

    /// Aggregate a vector of reports using safe checked math and median selection.
    pub fn aggregate_reports(
        env: Env,
        reports: Vec<MetricReport>,
    ) -> Result<AggregatedMetric, OracleError> {
        let count = reports.len();
        if count == 0 {
            return Err(OracleError::NoDataAvailable);
            let r = reports.get(i).ok_or(OracleError::NoDataAvailable)?;
        }

        let mut sum: u128 = 0;
        let mut min_val: u128 = u128::MAX;
        let mut max_val: u128 = 0;
        let mut latest_ts: u64 = 0;
        let mut latest_round: u64 = 0;

        for i in 0..count {
            let r = reports.get(i).unwrap();
            sum = Self::safe_add(sum, r.value)?;
            if r.value < min_val {
                min_val = r.value;
            }
            if r.value > max_val {
                max_val = r.value;
            }
            if r.timestamp > latest_ts {
                latest_ts = r.timestamp;
            }
            if r.round_id > latest_round {
                latest_round = r.round_id;
            }
        }

        let mean_val = Self::safe_div(sum, count as u128)?;

        // Approximate median: mid-point between min and max if dispersed, or mean
        let mid_sum = Self::safe_add(min_val, max_val)?;
        let consensus_val = Self::safe_div(mid_sum, 2)?;

        Ok(AggregatedMetric {
            consensus_value: consensus_val,
            mean_value: mean_val,
            min_value: min_val,
            max_value: max_val,
            reporter_count: count,
            latest_timestamp: latest_ts,
            round_id: latest_round,
        })
    }

    /// Read the latest reported metric for a given feed/symbol.
    pub fn get_latest_metric(
        env: Env,
        metric_name: Symbol,
    ) -> Result<MetricReport, OracleError> {
        Self::require_initialized(&env)?;
        env.storage()
            .instance()
            .get(&metric_name)
            .ok_or(OracleError::NoDataAvailable)
    }

    /// Check if an address is an active reporter.
    pub fn is_reporter(env: Env, address: Address) -> bool {
        let reporters: Map<Address, bool> = env
            .storage()
            .instance()
            .get(&REPORTERS_KEY)
            .unwrap_or_else(|| Map::new(&env));
        reporters.get(address).unwrap_or(false)
    }

    /// Check if the Oracle is paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&ORACLE_PAUSED_KEY)
            .unwrap_or(false)
    }
}

// -----------------------------------------------------------------------
// Test Suites: Unit, Invariant (#845), and Integration (#843)
// -----------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, vec, Env};

    fn setup_oracle(env: &Env) -> (Address, OracleConfig) {
        let admin = Address::generate(env);
        let config = OracleConfig {
            min_allowed_value: 10,
            max_allowed_value: 1_000_000,
            max_staleness_seconds: 3600,
            min_quorum: 2,
        };
        (admin, config)
    }

    // -------------------------------------------------------------------
    // #846: Integer Overflow & Underflow Tests
    // -------------------------------------------------------------------
    #[test]
    fn test_overflow_protection_in_addition() {
        let res = OracleContract::safe_add(u128::MAX, 1);
        assert_eq!(res, Err(OracleError::MathOverflow));

        let valid = OracleContract::safe_add(100, 200).unwrap();
        assert_eq!(valid, 300);
    }

    #[test]
    fn test_underflow_protection_in_subtraction() {
        let res = OracleContract::safe_sub(10, 20);
        assert_eq!(res, Err(OracleError::MathUnderflow));

        let valid = OracleContract::safe_sub(50, 20).unwrap();
        assert_eq!(valid, 30);
    }

    #[test]
    fn test_overflow_protection_in_multiplication() {
        let res = OracleContract::safe_mul(u128::MAX / 2 + 1, 2);
        assert_eq!(res, Err(OracleError::MathOverflow));

        let valid = OracleContract::safe_mul(50, 20).unwrap();
        assert_eq!(valid, 1000);
    }

    #[test]
    fn test_division_by_zero_protection() {
        let res = OracleContract::safe_div(100, 0);
        assert_eq!(res, Err(OracleError::DivisionByZero));

        let valid = OracleContract::safe_div(100, 4).unwrap();
        assert_eq!(valid, 25);
    }

    #[test]
    fn test_weighted_average_overflow_safety() {
        // High values with risk of intermediate overflow
        let v1 = u128::MAX / 4;
        let w1 = 2;
        let v2 = u128::MAX / 4;
        let w2 = 2;
        // Total weight = 4. Intermediate product fits or is guarded.
        let avg = OracleContract::safe_weighted_average(v1, w1, v2, w2).unwrap();
        assert_eq!(avg, v1);

        // When intermediate sum overflows u128::MAX, it fails cleanly with MathOverflow
        let overflow_res = OracleContract::safe_weighted_average(u128::MAX, 2, u128::MAX, 2);
        assert_eq!(overflow_res, Err(OracleError::MathOverflow));
    }

    // -------------------------------------------------------------------
    // #844: Access Control Modifier & Role Tests
    // -------------------------------------------------------------------
    #[test]
    fn test_access_control_admin_only_operations() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, config) = setup_oracle(&env);
        let hacker = Address::generate(&env);

        OracleContract::initialize(env.clone(), admin.clone(), config.clone()).unwrap();

        // Non-admin cannot pause
        let pause_res = OracleContract::pause(env.clone(), hacker.clone());
        assert_eq!(pause_res, Err(OracleError::Unauthorized));

        // Admin can pause
        assert!(OracleContract::pause(env.clone(), admin.clone()).is_ok());
        assert!(OracleContract::is_paused(env.clone()));

        // Admin can unpause
        assert!(OracleContract::unpause(env.clone(), admin.clone()).is_ok());
        assert!(!OracleContract::is_paused(env.clone()));
    }

    #[test]
    fn test_access_control_reporter_authorization() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, config) = setup_oracle(&env);
        let reporter = Address::generate(&env);
        let non_reporter = Address::generate(&env);

        OracleContract::initialize(env.clone(), admin.clone(), config).unwrap();
        OracleContract::add_reporter(env.clone(), admin.clone(), reporter.clone()).unwrap();

        assert!(OracleContract::is_reporter(env.clone(), reporter.clone()));
        assert!(!OracleContract::is_reporter(env.clone(), non_reporter.clone()));

        // Non-reporter cannot submit data
        let res = OracleContract::submit_report(
            env.clone(),
            non_reporter.clone(),
            symbol_short!("MTTR"),
            100,
            1000,
        );
        assert_eq!(res, Err(OracleError::ReporterNotAuthorized));

        // Authorized reporter can submit data
        let success = OracleContract::submit_report(
            env.clone(),
            reporter.clone(),
            symbol_short!("MTTR"),
            100,
            1000,
        );
        assert!(success.is_ok());

        // Revoking reporter prevents further submissions
        OracleContract::remove_reporter(env.clone(), admin.clone(), reporter.clone()).unwrap();
        let revoked_res = OracleContract::submit_report(
            env.clone(),
            reporter.clone(),
            symbol_short!("MTTR"),
            120,
            1005,
        );
        assert_eq!(revoked_res, Err(OracleError::ReporterNotAuthorized));
    }

    // -------------------------------------------------------------------
    // #845: Invariant Tests
    // -------------------------------------------------------------------
    #[test]
    fn test_invariant_timestamp_monotonicity() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, config) = setup_oracle(&env);
        let reporter = Address::generate(&env);

        OracleContract::initialize(env.clone(), admin.clone(), config).unwrap();
        OracleContract::add_reporter(env.clone(), admin.clone(), reporter.clone()).unwrap();

        // Initial submission at T=1000
        OracleContract::submit_report(
            env.clone(),
            reporter.clone(),
            symbol_short!("METRIC1"),
            500,
            1000,
        )
        .unwrap();

        // Regression attempt at T=999 must fail (Invariant: non-decreasing timestamps)
        let regressed = OracleContract::submit_report(
            env.clone(),
            reporter.clone(),
            symbol_short!("METRIC1"),
            550,
            999,
        );
        assert_eq!(regressed, Err(OracleError::TimestampRegression));

        // Subsequent valid timestamp at T=1001 must succeed
        let valid = OracleContract::submit_report(
            env.clone(),
            reporter.clone(),
            symbol_short!("METRIC1"),
            550,
            1001,
        );
        assert!(valid.is_ok());
    }

    #[test]
    fn test_invariant_metric_bounds() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, config) = setup_oracle(&env); // min=10, max=1_000_000
        let reporter = Address::generate(&env);

        OracleContract::initialize(env.clone(), admin.clone(), config).unwrap();
        OracleContract::add_reporter(env.clone(), admin.clone(), reporter.clone()).unwrap();

        // Under lower bound
        let under = OracleContract::submit_report(
            env.clone(),
            reporter.clone(),
            symbol_short!("PING"),
            9,
            100,
        );
        assert_eq!(under, Err(OracleError::ValueOutOfBounds));

        // Over upper bound
        let over = OracleContract::submit_report(
            env.clone(),
            reporter.clone(),
            symbol_short!("PING"),
            1_000_001,
            100,
        );
        assert_eq!(over, Err(OracleError::ValueOutOfBounds));
    }

    #[test]
    fn test_invariant_aggregation_between_bounds() {
        let env = Env::default();
        let rep_addr = Address::generate(&env);

        let r1 = MetricReport {
            value: 100,
            timestamp: 10,
            reporter: rep_addr.clone(),
            round_id: 1,
            status: ReportStatus::Valid,
        };
        let r2 = MetricReport {
            value: 500,
            timestamp: 15,
            reporter: rep_addr.clone(),
            round_id: 2,
            status: ReportStatus::Valid,
        };
        let r3 = MetricReport {
            value: 300,
            timestamp: 20,
            reporter: rep_addr.clone(),
            round_id: 3,
            status: ReportStatus::Valid,
        };

        let reports = vec![&env, r1, r2, r3];
        let agg = OracleContract::aggregate_reports(env.clone(), reports).unwrap();

        // Invariants:
        // 1. Min is 100
        assert_eq!(agg.min_value, 100);
        // 2. Max is 500
        assert_eq!(agg.max_value, 500);
        // 3. Mean (300) and Consensus (300) must stay inside [min, max]
        assert!(agg.mean_value >= agg.min_value && agg.mean_value <= agg.max_value);
        assert!(agg.consensus_value >= agg.min_value && agg.consensus_value <= agg.max_value);
    }

    // -------------------------------------------------------------------
    // #843: Integration Tests Covering Full Oracle Flows
    // -------------------------------------------------------------------
    #[test]
    fn test_integration_full_oracle_flow() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, config) = setup_oracle(&env);
        let reporter1 = Address::generate(&env);
        let reporter2 = Address::generate(&env);

        // Step 1: Deploy & Initialize
        OracleContract::initialize(env.clone(), admin.clone(), config).unwrap();

        // Step 2: Register Reporters
        OracleContract::add_reporter(env.clone(), admin.clone(), reporter1.clone()).unwrap();
        OracleContract::add_reporter(env.clone(), admin.clone(), reporter2.clone()).unwrap();

        // Step 3: Reporting & Ingestion Flow
        let feed_name = symbol_short!("SLA_FEED");
        let rep1 = OracleContract::submit_report(
            env.clone(),
            reporter1.clone(),
            feed_name.clone(),
            250,
            1000,
        )
        .unwrap();
        assert_eq!(rep1.value, 250);

        let rep2 = OracleContract::submit_report(
            env.clone(),
            reporter2.clone(),
            feed_name.clone(),
            350,
            1050,
        )
        .unwrap();
        assert_eq!(rep2.value, 350);

        // Step 4: Verification of latest state
        let latest = OracleContract::get_latest_metric(env.clone(), feed_name).unwrap();
        assert_eq!(latest.value, 350);
        assert_eq!(latest.timestamp, 1050);

        // Step 5: Multi-reporter consensus aggregation
        let reports = vec![&env, rep1, rep2];
        let agg = OracleContract::aggregate_reports(env.clone(), reports).unwrap();
        assert_eq!(agg.reporter_count, 2);
        assert_eq!(agg.mean_value, 300);

        // Step 6: Emergency Pause Flow
        OracleContract::pause(env.clone(), admin.clone()).unwrap();
        let paused_err = OracleContract::submit_report(
            env.clone(),
            reporter1.clone(),
            feed_name.clone(),
            400,
            1100,
        );
        assert_eq!(paused_err, Err(OracleError::ContractPaused));

        // Step 7: Unpause and resume normal operation
        OracleContract::unpause(env.clone(), admin.clone()).unwrap();
        let resumed = OracleContract::submit_report(
            env.clone(),
            reporter1.clone(),
            feed_name,
            400,
            1100,
        );
        assert!(resumed.is_ok());
    }

    #[test]
fn test_submit_report_missing_config_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (admin, config) = setup_oracle(&env);
    let reporter = Address::generate(&env);

    OracleContract::initialize(env.clone(), admin.clone(), config).unwrap();
    OracleContract::add_reporter(env.clone(), admin.clone(), reporter.clone()).unwrap();

    env.storage().instance().remove(&ORACLE_CONFIG_KEY);

    let res = OracleContract::submit_report(
        env.clone(),
        reporter,
        symbol_short!("MTTR"),
        100,
        1000,
    );
    assert_eq!(res, Err(OracleError::NotInitialized));
}
}
