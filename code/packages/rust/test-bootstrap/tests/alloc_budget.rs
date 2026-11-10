//! # Real allocation budgets (LH-003a): sabotage and control
//!
//! `max_alloc_bytes = 0` used to pass for every closure, because the old
//! counter only counted what a closure chose to report. These tests install
//! the counting global allocator and prove the budget now bites:
//!
//! | closure                          | budget `max_alloc_bytes` | expected         |
//! |----------------------------------|--------------------------|------------------|
//! | `1 + 1`                          | 0                        | pass, 0 B / iter |
//! | `Vec::<u8>::with_capacity(1)`    | 0                        | **fail**, 1 B    |
//! | `Vec::<u8>::with_capacity(8)`    | 8                        | pass, 8 B / iter |
//! | `Vec::<u8>::with_capacity(9)`    | 8                        | **fail**, 9 B    |
//!
//! The sabotage rows are the point: a counter that cannot see a one-byte
//! allocation would make them pass, and the suite fails.
//!
//! libtest runs these. Tallies are per-thread, so concurrently running tests
//! cannot contaminate each other's measurements.

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, Budget, BudgetVerdict, CostCounter,
    CounterSample,
};
use coding_adventures_test_bootstrap::counting_alloc::{self, AllocStats};

coding_adventures_test_bootstrap::install_counting_allocator!();

fn harness() -> BenchmarkHarness {
    BenchmarkHarness::new(BenchmarkConfig {
        warmup_iterations: 2,
        warmup_max_duration: Duration::from_micros(200),
        min_batch_duration: Duration::from_micros(20),
        max_batch_size: 4096,
        sample_count: 5,
        seed: 3,
    })
}

fn budget(max_alloc_bytes: u64) -> Budget {
    Budget {
        max_alloc_bytes: Some(max_alloc_bytes),
        ..Default::default()
    }
}

#[test]
fn the_counting_allocator_is_installed_in_this_binary() {
    assert!(counting_alloc::is_installed());
}

#[test]
fn a_closure_that_does_not_allocate_passes_the_zero_budget() {
    let counter = AllocCounter::new();
    let result = harness().run_with_counter("alloc/quiet", &budget(0), &counter, || {
        black_box(1u64) + black_box(1u64)
    });
    assert_eq!(result.budget_verdict, BudgetVerdict::Pass);
    assert_eq!(result.alloc_bytes_per_iter, Some(0.0));
    // The budget was genuinely checked, not merely declared.
    assert!(result.unchecked_hard_budgets.is_empty());
}

#[test]
fn sabotage_a_one_byte_allocation_fails_the_zero_budget() {
    let counter = AllocCounter::new();
    let result = harness().run_with_counter("alloc/sabotage", &budget(0), &counter, || {
        black_box(Vec::<u8>::with_capacity(1))
    });
    assert!(
        matches!(
            result.budget_verdict,
            BudgetVerdict::Failed {
                observed: 1,
                limit: 0,
                ..
            }
        ),
        "a closure that allocates must fail max_alloc_bytes = 0, got {:?}",
        result.budget_verdict
    );
    assert_eq!(result.alloc_bytes_per_iter, Some(1.0));
    assert!(!result.budget_verdict.is_acceptable());
}

#[test]
fn a_large_allocation_is_reported_with_its_real_size() {
    let counter = AllocCounter::new();
    let result = harness().run_with_counter("alloc/4k", &budget(0), &counter, || {
        black_box(vec![0u8; 4096])
    });
    assert!(matches!(
        result.budget_verdict,
        BudgetVerdict::Failed {
            observed: 4096,
            limit: 0,
            ..
        }
    ));
}

#[test]
fn a_nonzero_budget_is_an_exact_boundary() {
    let at_limit = harness().run_with_counter("alloc/8", &budget(8), AllocCounter::new(), || {
        black_box(Vec::<u8>::with_capacity(8))
    });
    assert_eq!(at_limit.budget_verdict, BudgetVerdict::Pass);
    assert_eq!(at_limit.alloc_bytes_per_iter, Some(8.0));

    let over = harness().run_with_counter("alloc/9", &budget(8), AllocCounter::new(), || {
        black_box(Vec::<u8>::with_capacity(9))
    });
    assert!(matches!(
        over.budget_verdict,
        BudgetVerdict::Failed {
            observed: 9,
            limit: 8,
            ..
        }
    ));
}

#[test]
fn an_allocation_hidden_inside_a_callee_is_still_seen() {
    // The old counter needed the closure to call record_alloc itself. Here the
    // allocation is buried in a callee that knows nothing about counters.
    #[inline(never)]
    fn sneaky() -> usize {
        black_box(String::from("x")).len()
    }
    let counter = AllocCounter::new();
    let result = harness().run_with_counter("alloc/sneaky", &budget(0), &counter, sneaky);
    assert!(!result.budget_verdict.is_acceptable());
}

#[test]
fn stats_expose_counts_for_reporting() {
    let counter = AllocCounter::new();
    let mut by_ref = &counter;
    by_ref.start();
    let v = black_box(vec![1u8; 100]);
    let stats = counter.stats().expect("installed");
    assert_eq!(stats.allocations, 1);
    assert_eq!(stats.bytes_allocated, 100);
    assert_eq!(by_ref.sample(), CounterSample::Live(100));
    drop(v);
    let after_drop = counter.stats().expect("installed");
    assert_eq!(after_drop.deallocations, 1);
    assert_eq!(after_drop.bytes_deallocated, 100);
}

#[test]
fn measure_reports_exact_activity_and_probe_is_invisible() {
    let (len, stats) = counting_alloc::measure(|| black_box(vec![0u8; 32]).len());
    assert_eq!(len, 32);
    let stats = stats.expect("installed");
    assert_eq!(
        stats,
        AllocStats {
            allocations: 1,
            deallocations: 1,
            bytes_allocated: 32,
            bytes_deallocated: 32,
        }
    );
    // Asking whether the allocator is installed must not move the tally.
    let before = counting_alloc::thread_stats();
    assert!(counting_alloc::is_installed());
    assert_eq!(counting_alloc::thread_stats(), before);
}
