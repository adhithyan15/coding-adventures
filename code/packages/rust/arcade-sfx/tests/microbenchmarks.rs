//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `arcade-sfx`
//!
//! Evaluates `arcade-sfx` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates:
//! - Hard zero-heap-allocation budgets across real-time sound rendering (`render_sfx_into`),
//!   preset triggering (`trigger_preset`), multi-voice bank mixing (`mix_bank_into`),
//!   and LFSR noise updates, measured on the real heap by test-bootstrap's counting global allocator.
//! - Counting allocator liveness and active heap metering.
//! - Sabotage detection: intentional dynamic memory allocation is detected and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use arcade_sfx::{
    mix_bank_into, render_sfx_into, AdsrEnvelope, SfxBank, SfxPatch, SfxPreset, SfxVoice, Waveform,
    DUTY_50_PERCENT, UNITY_GAIN_Q16,
};
use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;

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

fn check_render_sfx_square_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut voice = SfxVoice::new(44_100).expect("voice new");
    let env = AdsrEnvelope::from_samples(10, 10, UNITY_GAIN_Q16, 10).expect("env");
    let patch = SfxPatch::create_patch(
        Waveform::Square {
            duty_q16: DUTY_50_PERCENT,
        },
        440,
        440,
        0,
        env,
        1_000_000,
        UNITY_GAIN_Q16,
    )
    .expect("patch");
    voice.trigger_patch(patch).expect("trigger patch");

    let mut out_buf = [0i16; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("render_sfx_square_zero_alloc", &budget, &alloc, || {
        let written = render_sfx_into(black_box(&mut voice), black_box(&mut out_buf));
        black_box(written).expect("render should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "render_sfx_into (square) exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_render_sfx_lfsr_noise_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut voice = SfxVoice::new(44_100).expect("voice new");
    let env = AdsrEnvelope::from_samples(10, 10, UNITY_GAIN_Q16, 10).expect("env");
    let patch = SfxPatch::create_patch(
        Waveform::LfsrNoise { short_mode: false },
        1000,
        1000,
        0,
        env,
        1_000_000,
        UNITY_GAIN_Q16,
    )
    .expect("patch");
    voice.trigger_patch(patch).expect("trigger patch");

    let mut out_buf = [0i16; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("render_sfx_lfsr_noise_zero_alloc", &budget, &alloc, || {
        let written = render_sfx_into(black_box(&mut voice), black_box(&mut out_buf));
        black_box(written).expect("render noise should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "render_sfx_into (lfsr noise) exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_mix_bank_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut bank = SfxBank::<4>::empty(44_100).expect("bank empty");
    let env = AdsrEnvelope::from_samples(10, 10, UNITY_GAIN_Q16, 10).expect("env");
    let patch = SfxPatch::create_patch(
        Waveform::Square {
            duty_q16: DUTY_50_PERCENT,
        },
        440,
        440,
        0,
        env,
        1_000_000,
        UNITY_GAIN_Q16,
    )
    .expect("patch");

    for _ in 0..4 {
        bank.trigger_bank_patch(patch).expect("trigger voice");
    }

    let mut out_buf = [0i16; 64];

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(20_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("mix_bank_zero_alloc", &budget, &alloc, || {
        let written = mix_bank_into(black_box(&mut bank), black_box(&mut out_buf));
        black_box(written).expect("mix should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "mix_bank_into exceeded zero allocation budget",
        );
    }

    Verdict::Pass
}

fn check_trigger_preset_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut voice = SfxVoice::new(44_100).expect("voice new");

    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        max_steps: Some(10_000),
        ..Default::default()
    };
    let alloc = AllocCounter::new();

    let res = harness.run_with_counter("trigger_preset_zero_alloc", &budget, &alloc, || {
        let trig = voice.trigger_preset(black_box(SfxPreset::PongPaddleHit));
        black_box(trig).expect("trigger preset should succeed");
    });

    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "trigger_preset exceeded zero allocation budget",
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
        ("module", Json::str("arcade-sfx")),
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
        "bench/render_sfx/square_zero_alloc",
        check_render_sfx_square_zero_alloc,
    );
    suite.add(
        "bench/render_sfx/lfsr_noise_zero_alloc",
        check_render_sfx_lfsr_noise_zero_alloc,
    );
    suite.add("bench/mix_bank/zero_alloc", check_mix_bank_zero_alloc);
    suite.add(
        "bench/trigger_preset/zero_alloc",
        check_trigger_preset_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
