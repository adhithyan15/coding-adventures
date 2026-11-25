//! # Microbenchmark Suite and Zero-Allocation Verification for `swf-conformance-corpus` (`VER07`)
//!
//! Validates via `test-bootstrap`'s counting global allocator:
//! - Zero heap allocation in `validate_corpus_swf_bytes`, `validate_corpus_frame_count`,
//!   `validate_corpus_bit_width`, and `validate_corpus_digest_hex`,
//! - Zero heap allocation in steady-state digest verification and `player_step_clock_and_audio_zero_alloc`,
//! - Sabotage detection: an intentional allocation is caught and rejected.

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use flash_display_list::{PaintBase, PaintInstruction, PaintRect, PaintScene, PatchKind, PatchOp};
use flash_player_host::{FlashPlayer, FlashPlayerConfig};
use swf_conformance_corpus::{
    validate_corpus_bit_width, validate_corpus_digest_hex, validate_corpus_frame_count,
    validate_corpus_swf_bytes, CorpusId, CorpusRunner,
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

fn zero_budget() -> Budget {
    Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    }
}

fn bench_zero_alloc(name: &str, f: impl FnMut()) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = zero_budget();
    let alloc = AllocCounter::new();
    let res = harness.run_with_counter(name, &budget, &alloc, f);
    if !zero_alloc_ok(&res) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            format!("{name} exceeded zero allocation budget"),
        );
    }
    Verdict::Pass
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_warrant_bounds_zero_alloc(_ctx: &Ctx) -> Verdict {
    bench_zero_alloc("corpus_warrant_bounds_zero_alloc", || {
        let a = validate_corpus_swf_bytes(black_box(4096)).is_ok();
        let b = validate_corpus_frame_count(black_box(120)).is_ok();
        let c = validate_corpus_bit_width(black_box(15)).is_ok();
        let d = validate_corpus_digest_hex(black_box(
            b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ))
        .is_ok();
        black_box((a, b, c, d));
    })
}

fn check_corpus_digest_equality_zero_alloc(_ctx: &Ctx) -> Verdict {
    let expected = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let actual = b"0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef";
    bench_zero_alloc("corpus_digest_verify_zero_alloc", || {
        let valid_e = validate_corpus_digest_hex(black_box(expected)).is_ok();
        let valid_a = validate_corpus_digest_hex(black_box(actual)).is_ok();
        let eq = expected.eq_ignore_ascii_case(black_box(actual));
        black_box((valid_e, valid_a, eq));
    })
}

fn check_corpus_player_steady_step_zero_alloc(_ctx: &Ctx) -> Verdict {
    let swf = CorpusRunner::runner_build_swf(CorpusId::Timeline).expect("timeline swf");
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).expect("swf");
    let dt_ns = player.player_clock().fixed_dt_ns;

    let mut prev = PaintScene::new(400.0, 300.0);
    let mut next = PaintScene::new(400.0, 300.0);
    let mut r1 = PaintRect::filled(0.0, 0.0, 20.0, 20.0, "#ff6d00");
    r1.base = PaintBase {
        id: Some("shape_1".to_string()),
        metadata: None,
    };
    let mut r2 = PaintRect::filled(10.0, 0.0, 20.0, 20.0, "#ff6d00");
    r2.base = PaintBase {
        id: Some("shape_1".to_string()),
        metadata: None,
    };
    prev.instructions.push(PaintInstruction::Rect(r1));
    next.instructions.push(PaintInstruction::Rect(r2));

    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 16];
    let mut audio_pcm = [0i16; 64];

    bench_zero_alloc("corpus_steady_step_zero_alloc", || {
        let tick = player
            .player_step_clock_and_audio_zero_alloc(
                black_box(dt_ns),
                black_box(&prev),
                black_box(&next),
                black_box(&mut ops),
                black_box(&mut audio_pcm),
            )
            .expect("zero-alloc tick");
        black_box(tick);
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = zero_budget();
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
        ("module", Json::str("swf_conformance_corpus")),
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
        "bench/validation/corpus_warrant_bounds_zero_alloc",
        check_warrant_bounds_zero_alloc,
    );
    suite.add(
        "bench/verify/corpus_digest_verify_zero_alloc",
        check_corpus_digest_equality_zero_alloc,
    );
    suite.add(
        "bench/player/corpus_steady_step_zero_alloc",
        check_corpus_player_steady_step_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
