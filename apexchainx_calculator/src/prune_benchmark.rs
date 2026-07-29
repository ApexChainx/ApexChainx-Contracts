//! #204 – Reproducible benchmark artifact for history prune operations.
//!
//! This module provides a deterministic, reproducible benchmark suite for
//! the `prune_history` and `prune_history_by_age` operations at 1k, 10k,
//! and 100k history entry sizes.
//!
//! # Usage
//!
//! ```bash
//! cargo test --package apexchainx_calculator -- prune_benchmark --nocapture --ignored
//! ```
//!
//! The `--ignored` flag is required because 100k-entry tests are expensive
//! and should only run in CI or explicitly requested local runs.
//!
//! # Output Artifact
//!
//! The test prints a machine-readable JSON benchmark artifact to stdout,
//! suitable for CI ingestion and trend tracking.

#[cfg(test)]
mod prune_benchmark {
    use crate::SLACalculatorContract;
    use alloc::format;
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Ledger},
        Address, Env, Symbol,
    };

    /// CPU instruction budget ceiling per entry size tier.
    /// Values are generous to allow for test infrastructure overhead
    /// while still catching regressions.
    const BUDGET_1K: u64 = 5_000_000;   // 5M instructions
    const BUDGET_10K: u64 = 50_000_000;  // 50M instructions
    const BUDGET_100K: u64 = 500_000_000; // 500M instructions

    struct PruneBenchEntry {
        size: u32,
        prune_kept: u32,
        cpu_instructions: u64,
        budget: u64,
        passed: bool,
    }

    fn run_prune_bench(env: &Env, size: u32, prune_kept: u32, budget: u64) -> PruneBenchEntry {
        let admin = Address::generate(env);
        let op = Address::generate(env);

        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = crate::SLACalculatorContractClient::new(env, &contract_id);
        client.initialize(&admin, &op);

        // Populate history with `size` entries
        for i in 0..size {
            let outage_id = Symbol::new(env, &format!("BENCH_{}", i));
            client.calculate_sla(
                &op,
                &outage_id,
                &symbol_short!("low"),
                &(10u32 + (i % 5)),
            );
        }

        let history_before = client.get_history();
        assert_eq!(history_before.len(), size, "History population failed");

        // Measure prune performance
        env.budget().reset_default();
        let before = env.budget().cpu_instruction_cost();
        client.prune_history(&admin, &prune_kept);
        let after = env.budget().cpu_instruction_cost();
        let cpu_instructions = after.saturating_sub(before);

        let history_after = client.get_history();
        assert_eq!(
            history_after.len(),
            prune_kept,
            "Prune did not produce expected count"
        );

        PruneBenchEntry {
            size,
            prune_kept,
            cpu_instructions,
            budget,
            passed: cpu_instructions < budget,
        }
    }

    fn run_prune_by_age_bench(env: &Env, size: u32, keep_ratio: f64, budget: u64) -> PruneBenchEntry {
        let admin = Address::generate(env);
        let op = Address::generate(env);

        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = crate::SLACalculatorContractClient::new(env, &contract_id);
        client.initialize(&admin, &op);

        // Populate history with staggered timestamps
        let base_timestamp = 1000u64;
        for i in 0..size {
            let ts = base_timestamp + (i as u64 * 10);
            env.ledger().set_timestamp(ts);
            let outage_id = Symbol::new(env, &format!("BENCH_A_{}", i));
            client.calculate_sla(
                &op,
                &outage_id,
                &symbol_short!("low"),
                &(10u32 + (i % 5)),
            );
        }

        // Set timestamp far in the future so prune by age works
        let final_ts = base_timestamp + (size as u64 * 10) + 1000;
        env.ledger().set_timestamp(final_ts);

        // Calculate age window to keep `keep_ratio` fraction of entries
        let total_span = final_ts - base_timestamp;
        let age_window = (total_span as f64 * keep_ratio) as u64;

        let history_before = client.get_history();
        assert_eq!(history_before.len(), size);

        env.budget().reset_default();
        let before = env.budget().cpu_instruction_cost();
        client.prune_history_by_age(&admin, &age_window);
        let after = env.budget().cpu_instruction_cost();
        let cpu_instructions = after.saturating_sub(before);

        let history_after = client.get_history();
        assert!(
            history_after.len() < size,
            "Prune by age should have removed some entries"
        );

        PruneBenchEntry {
            size,
            prune_kept: history_after.len(),
            cpu_instructions,
            budget,
            passed: cpu_instructions < budget,
        }
    }

    // ================================================================
    // Individual benchmark tests (grouped by size)
    // ================================================================

    #[test]
    fn bench_prune_1k() {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();

        let result = run_prune_bench(&env, 1_000, 100, BUDGET_1K);
        assert!(result.passed, "1k prune exceeded budget: {} > {}", result.cpu_instructions, result.budget);
        println!("  prune(1k → 100): {} instructions [PASS]", result.cpu_instructions);
    }

    #[test]
    fn bench_prune_1k_by_age() {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();

        let result = run_prune_by_age_bench(&env, 1_000, 0.5, BUDGET_1K);
        assert!(result.passed, "1k prune_by_age exceeded budget");
        println!("  prune_by_age(1k, 50%): {} instructions [PASS]", result.cpu_instructions);
    }

    #[test]
    fn bench_prune_10k() {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();

        let result = run_prune_bench(&env, 10_000, 100, BUDGET_10K);
        assert!(result.passed, "10k prune exceeded budget: {} > {}", result.cpu_instructions, result.budget);
        println!("  prune(10k → 100): {} instructions [PASS]", result.cpu_instructions);
    }

    #[test]
    fn bench_prune_10k_by_age() {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();

        let result = run_prune_by_age_bench(&env, 10_000, 0.5, BUDGET_10K);
        assert!(result.passed, "10k prune_by_age exceeded budget");
        println!("  prune_by_age(10k, 50%): {} instructions [PASS]", result.cpu_instructions);
    }

    #[test]
    #[ignore = "expensive: 100k entries; run with --ignored"]
    fn bench_prune_100k() {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();

        let result = run_prune_bench(&env, 100_000, 100, BUDGET_100K);
        assert!(result.passed, "100k prune exceeded budget: {} > {}", result.cpu_instructions, result.budget);
        println!("  prune(100k → 100): {} instructions [PASS]", result.cpu_instructions);
    }

    #[test]
    #[ignore = "expensive: 100k entries; run with --ignored"]
    fn bench_prune_100k_by_age() {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();

        let result = run_prune_by_age_bench(&env, 100_000, 0.5, BUDGET_100K);
        assert!(result.passed, "100k prune_by_age exceeded budget");
        println!("  prune_by_age(100k, 50%): {} instructions [PASS]", result.cpu_instructions);
    }

    // ================================================================
    // Aggregate benchmark reporter (produces the reproducible artifact)
    // ================================================================

    #[test]
    #[ignore = "aggregate: runs all benchmarks including 100k; run with --ignored"]
    fn bench_prune_full_report() {
        println!("\n=== PRUNE BENCHMARK REPORT (#204) ===\n");

        let mut results: Vec<PruneBenchEntry> = Vec::new();

        // 1k benchmarks
        {
            let env = Env::default();
            env.mock_all_auths();
            env.budget().reset_unlimited();
            results.push(run_prune_bench(&env, 1_000, 100, BUDGET_1K));
        }
        {
            let env = Env::default();
            env.mock_all_auths();
            env.budget().reset_unlimited();
            results.push(run_prune_by_age_bench(&env, 1_000, 0.5, BUDGET_1K));
        }

        // 10k benchmarks
        {
            let env = Env::default();
            env.mock_all_auths();
            env.budget().reset_unlimited();
            results.push(run_prune_bench(&env, 10_000, 100, BUDGET_10K));
        }
        {
            let env = Env::default();
            env.mock_all_auths();
            env.budget().reset_unlimited();
            results.push(run_prune_by_age_bench(&env, 10_000, 0.5, BUDGET_10K));
        }

        // 100k benchmarks
        {
            let env = Env::default();
            env.mock_all_auths();
            env.budget().reset_unlimited();
            results.push(run_prune_bench(&env, 100_000, 100, BUDGET_100K));
        }
        {
            let env = Env::default();
            env.mock_all_auths();
            env.budget().reset_unlimited();
            results.push(run_prune_by_age_bench(&env, 100_000, 0.5, BUDGET_100K));
        }

        // Print results table
        println!("{:<12} {:<14} {:<18} {:<14} {:<8}", "Size", "Operation", "CPU Instructions", "Budget", "Status");
        println!("{}", "-".repeat(70));

        let mut all_passed = true;
        for r in &results {
            let op = if r.prune_kept == 100 { "prune_history" } else { "prune_by_age" };
            let status = if r.passed { "PASS" } else { "FAIL" };
            println!(
                "{:<12} {:<14} {:<18} {:<14} {:<8}",
                format!("{}", r.size),
                op,
                format!("{}", r.cpu_instructions),
                format!("{}", r.budget),
                status,
            );
            if !r.passed {
                all_passed = false;
            }
        }

        // Print JSON artifact for CI ingestion
        println!("\n--- BENCHMARK ARTIFACT (JSON) ---");
        print!("[");
        for (i, r) in results.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(
                "{{\"size\":{},\"operation\":\"{}\",\"cpu_instructions\":{},\"budget\":{},\"passed\":{}}}",
                r.size,
                if r.prune_kept == 100 { "prune_history" } else { "prune_by_age" },
                r.cpu_instructions,
                r.budget,
                r.passed,
            );
        }
        println!("]");
        println!("--- END BENCHMARK ARTIFACT ---\n");

        assert!(all_passed, "One or more prune benchmarks exceeded their budget");
    }
}
