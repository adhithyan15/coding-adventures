//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `game-breakout`
//!
//! Evaluates `game-breakout` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across core Breakout simulation steps (`step_breakout`),
//!   continuous collision checks, sprite quad generation (`build_breakout_sprites`),
//!   audio ring buffer rendering (`render_breakout_audio_into`), and PRNG queries,
//!   measured on the real heap by test-bootstrap's counting global allocator.
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
use game_breakout::{
    build_breakout_sprites, default_config, deterministic_prng_next, new_game,
    render_breakout_audio_into, step_breakout, BreakoutInput, PcmRingBuffer, SfxBank, SpriteItem,
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

fn check_step_breakout_zero_alloc(_ctx: &Ctx) -> Verdict {
    let config = default_config();
    let mut state = new_game(config, 0x1234_5678).expect("state init");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx init");
    let input = BreakoutInput::move_paddle_left(false);

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("step_breakout_zero_alloc", &budget, &alloc, || {
        let events = step_breakout(black_box(&mut state), black_box(&input), black_box(&mut sfx));
        black_box(events).expect("step_breakout must succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(FID_BENCH_FAIL, "step_breakout exceeded zero allocation budget");
    }

    Verdict::Pass
}

fn check_build_breakout_sprites_zero_alloc(_ctx: &Ctx) -> Verdict {
    let config = default_config();
    let state = new_game(config, 0x1234_5678).expect("state init");
    let mut out_sprites = [SpriteItem::new(0, 0.0, 0.0, 0.0, 0.0); 256];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(5_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "build_breakout_sprites_zero_alloc",
        &budget,
        &alloc,
        || {
            let count = build_breakout_sprites(black_box(&state), black_box(&mut out_sprites));
            black_box(count);
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "build_breakout_sprites exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_render_breakout_audio_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx init");
    let mut ring = PcmRingBuffer::<256>::empty();
    let mut scratch = [0i16; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter(
        "render_breakout_audio_zero_alloc",
        &budget,
        &alloc,
        || {
            ring.clear();
            let written = render_breakout_audio_into(
                black_box(&mut sfx),
                black_box(&mut ring),
                black_box(&mut scratch),
            );
            black_box(written).expect("render_breakout_audio_into must succeed");
        },
    );

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "render_breakout_audio_into exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_prng_next_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut seed = 0xcafe_babe_dead_beef;

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(100),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("prng_next_zero_alloc", &budget, &alloc, || {
        let val = deterministic_prng_next(black_box(&mut seed));
        black_box(val);
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "deterministic_prng_next exceeded zero allocation budget",
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
        ("module", Json::str("game_breakout")),
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
        "bench/step_breakout/zero_alloc",
        check_step_breakout_zero_alloc,
    );
    suite.add(
        "bench/build_breakout_sprites/zero_alloc",
        check_build_breakout_sprites_zero_alloc,
    );
    suite.add(
        "bench/render_breakout_audio/zero_alloc",
        check_render_breakout_audio_zero_alloc,
    );
    suite.add("bench/prng_next/zero_alloc", check_prng_next_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
