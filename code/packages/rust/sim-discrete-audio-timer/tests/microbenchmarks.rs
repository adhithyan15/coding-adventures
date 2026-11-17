//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `sim-discrete-audio-timer`
//!
//! Evaluates `sim-discrete-audio-timer` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across real-time sound rendering (`render_timer_audio_into`),
//!   noise filter stepping (`render_noise_audio_into`), arcade audio mixing (`render_system_audio_into`),
//!   and debouncer clock stepping, measured on the real heap by test-bootstrap's counting global allocator.
//! - Counting allocator liveness and active heap metering.
//! - Sabotage detection: intentional dynamic memory allocation is detected and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use sim_discrete_audio_timer::{
    render_noise_audio_into, render_system_audio_into, render_timer_audio_into, CoinMechDebounce,
    DiscreteArcadeAudio, DiscreteNoiseFilter, Timer555Astable,
};

const FID_BENCH_FAIL: &str = "property-counterexample";

coding_adventures_test_bootstrap::install_counting_allocator!();

fn zero_alloc_ok(result: &BenchmarkResult) -> bool {
    result.budget_verdict.is_acceptable()
        && result.alloc_bytes_per_iter == Some(0.0)
        && !result.unchecked_hard_budgets.contains(&"alloc_bytes")
}

fn fast_bench_config(samples: usize) -> BenchmarkConfig {
    BenchmarkConfig {
        warmup_iterations: 10,
        warmup_max_duration: Duration::from_millis(1),
        min_batch_duration: Duration::from_micros(10),
        max_batch_size: 100_000,
        sample_count: samples,
        seed: 0x9e37_79b9_7f4a_7c15,
    }
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_render_timer_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut timer = Timer555Astable::paddle_hit_timer(44_100).expect("timer");
    let mut out_buf = [0i16; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("render_timer_zero_alloc", &budget, &alloc, || {
        let written = render_timer_audio_into(black_box(&mut timer), black_box(&mut out_buf));
        black_box(written).expect("render should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "render_timer_audio_into exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_render_noise_filter_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut filter = DiscreteNoiseFilter::explosion_filter(44_100).expect("filter");
    let mut out_buf = [0i16; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("render_noise_zero_alloc", &budget, &alloc, || {
        let written = render_noise_audio_into(black_box(&mut filter), black_box(&mut out_buf));
        black_box(written).expect("render should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "render_noise_audio_into exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_debounce_stepping_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut debouncer = CoinMechDebounce::new_debouncer(16, 99);

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("debounce_stepping_zero_alloc", &budget, &alloc, || {
        let coin = debouncer.step_debounce(black_box(true));
        black_box(coin);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "step_debounce exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_render_system_audio_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut system = DiscreteArcadeAudio::new_arcade_audio(44_100).expect("system");
    system.trigger_paddle_tone(1000);
    system.trigger_explosion(1000);
    let mut out_buf = [0i16; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("render_system_audio_zero_alloc", &budget, &alloc, || {
        let written = render_system_audio_into(black_box(&mut system), black_box(&mut out_buf));
        black_box(written).expect("render should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "render_system_audio_into exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(10));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(100),
        ..Default::default()
    };
    let counter = AllocCounter::new();
    let leaky = harness.run_with_counter("sabotage_alloc", &budget, &counter, || {
        black_box(Vec::<u8>::with_capacity(1))
    });
    if !matches!(
        leaky.budget_verdict,
        BudgetVerdict::Failed {
            observed: 1,
            limit: 0,
            ..
        }
    ) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a closure allocating one byte did not fail the zero-allocation budget",
        );
    }
    let clean = harness.run_with_counter("clean_alloc", &budget, &counter, || {
        black_box(1u64) ^ black_box(2u64)
    });
    if !zero_alloc_ok(&clean) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a non-allocating closure did not pass the zero-allocation budget",
        );
    }
    Verdict::Pass
}

fn write_bench_manifest(outcomes: &[Outcome]) {
    let Some(path) = manifest::output_path("WARRANT_BENCH_OUT") else {
        return;
    };
    let mut tests: Vec<Json> = outcomes
        .iter()
        .map(|o| {
            Json::obj([
                ("id", Json::str(o.id.clone())),
                ("kind", Json::str("bench")),
                (
                    "verdict",
                    Json::str(if o.passed() { "pass" } else { "fail" }),
                ),
            ])
        })
        .collect();
    tests.sort_by_key(|t| t.render());
    let doc = Json::obj([
        ("schema", Json::str("warrant-bench-evidence-v1")),
        ("module", Json::str("sim-discrete-audio-timer")),
        ("hw_instruction_counters", Json::str("not-live")),
        (
            "source_sha256",
            Json::str(coding_adventures_sha256::sha256_hex(
                &manifest::source_tree_framing(
                    &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
                )
                .expect("bind bench manifest to src"),
            )),
        ),
        ("tests", Json::Arr(tests)),
    ]);
    manifest::write(&path, &doc).expect("write bench manifest");
}

fn main() {
    let mut suite = Suite::new();

    suite.add(
        "bench/alloc/counting_allocator_is_live",
        check_counting_allocator_is_live,
    );
    suite.add(
        "bench/render_timer/zero_alloc",
        check_render_timer_zero_alloc,
    );
    suite.add(
        "bench/render_noise/zero_alloc",
        check_render_noise_filter_zero_alloc,
    );
    suite.add(
        "bench/debounce/zero_alloc",
        check_debounce_stepping_zero_alloc,
    );
    suite.add(
        "bench/render_system/zero_alloc",
        check_render_system_audio_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
