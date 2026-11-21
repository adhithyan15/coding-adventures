//! # Microbenchmark Suite and Zero-Allocation Verification for `flash-player-host` (`VER07`)
//!
//! Validates via `test-bootstrap`'s counting global allocator:
//! - Zero heap allocation in `sanitize_navigation_uri` (allowed & blocked paths),
//! - Zero heap allocation in `evaluate_fscommand` / `is_fscommand_exec`,
//! - Zero heap allocation in steady-state `player_step_clock_and_audio_zero_alloc`,
//! - Zero heap allocation in `diff_paint_scene_into`,
//! - Zero heap allocation in warrant validation helpers,
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
use flash_player_host::{
    diff_paint_scene_into, evaluate_fscommand, is_fscommand_exec, sanitize_navigation_uri,
    validate_frame_rate_hz, validate_stage_dimension_px, validate_uri_length,
    validate_vm_step_budget, FlashPlayer, FlashPlayerConfig, FsCommandOutcome,
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

fn make_minimal_swf_bytes() -> Vec<u8> {
    // FWS v8, 0..1000 twips rect (nbits=10 => 5 + 40 = 45 bits = 6 bytes), 24fps, 2 frames, End tag
    let mut swf = Vec::new();
    swf.extend_from_slice(b"FWS");
    swf.push(8);
    swf.extend_from_slice(&20u32.to_le_bytes());
    // nbits=10 (01010), xmin=0, xmax=1000, ymin=0, ymax=1000 -> pack simply via valid minimal 1-byte zero rect (0x00)
    swf.push(0x00); // 0-bit RECT (0x0)
    swf.extend_from_slice(&[0x00, 24]); // 24 fps
    swf.extend_from_slice(&2u16.to_le_bytes()); // 2 frames
    swf.extend_from_slice(&[0x00, 0x00]); // End tag
    let total = swf.len() as u32;
    swf[4..8].copy_from_slice(&total.to_le_bytes());
    swf
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_sanitize_uri_zero_alloc(_ctx: &Ctx) -> Verdict {
    bench_zero_alloc("sanitize_navigation_uri_zero_alloc", || {
        let ok = sanitize_navigation_uri(black_box("  https://example.com/game.swf?v=1  "));
        let blocked = sanitize_navigation_uri(black_box("  JaVaScRiPt:alert(1)  "));
        let rel = sanitize_navigation_uri(black_box("/assets/level1.swf"));
        black_box((ok.is_ok(), blocked.is_err(), rel.is_ok()));
    })
}

fn check_fscommand_zero_alloc(_ctx: &Ctx) -> Verdict {
    bench_zero_alloc("fscommand_zero_alloc", || {
        let exec_flag = is_fscommand_exec(black_box("  EXEC\t "));
        let blocked = evaluate_fscommand(black_box("exec"), black_box("/bin/sh"));
        let benign = evaluate_fscommand(black_box("fullscreen"), black_box("true"));
        assert!(exec_flag);
        assert_eq!(blocked, FsCommandOutcome::BlockedNoOp);
        assert_eq!(benign, FsCommandOutcome::HandledNoOp);
    })
}

fn check_player_steady_step_zero_alloc(_ctx: &Ctx) -> Verdict {
    let swf = make_minimal_swf_bytes();
    let mut player =
        FlashPlayer::player_from_swf_bytes(&swf, FlashPlayerConfig::cfg_standard()).expect("swf");
    let dt_ns = player.player_clock().fixed_dt_ns;

    let mut prev = PaintScene::new(100.0, 100.0);
    let mut next = PaintScene::new(100.0, 100.0);
    let mut r1 = PaintRect::filled(0.0, 0.0, 10.0, 10.0, "#ff0000");
    r1.base = PaintBase {
        id: Some("shape_1".to_string()),
        metadata: None,
    };
    let mut r2 = PaintRect::filled(5.0, 0.0, 10.0, 10.0, "#ff0000");
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

    bench_zero_alloc("player_step_clock_and_audio_zero_alloc", || {
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

fn check_scene_diff_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut prev = PaintScene::new(320.0, 240.0);
    let mut next = PaintScene::new(320.0, 240.0);
    for i in 0..8 {
        let mut r_prev = PaintRect::filled(i as f64 * 10.0, 0.0, 10.0, 10.0, "#00ff00");
        r_prev.base.id = Some(format!("node_{i}"));
        let mut r_next = PaintRect::filled((i + 1) as f64 * 10.0, 0.0, 10.0, 10.0, "#00ff00");
        r_next.base.id = Some(format!("node_{i}"));
        prev.instructions.push(PaintInstruction::Rect(r_prev));
        next.instructions.push(PaintInstruction::Rect(r_next));
    }
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 16];

    bench_zero_alloc("diff_paint_scene_into_zero_alloc", || {
        let summary =
            diff_paint_scene_into(black_box(&prev), black_box(&next), black_box(&mut ops))
                .expect("diff");
        black_box(summary);
    })
}

fn check_warrant_bounds_zero_alloc(_ctx: &Ctx) -> Verdict {
    bench_zero_alloc("warrant_bounds_zero_alloc", || {
        let a = validate_vm_step_budget(black_box(2_000_000)).is_ok();
        let b = validate_frame_rate_hz(black_box(60)).is_ok();
        let c = validate_stage_dimension_px(black_box(800)).is_ok();
        let d = validate_uri_length(black_box(256)).is_ok();
        black_box((a, b, c, d));
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
        ("module", Json::str("flash_player_host")),
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
        "bench/security/sanitize_uri_zero_alloc",
        check_sanitize_uri_zero_alloc,
    );
    suite.add(
        "bench/security/fscommand_zero_alloc",
        check_fscommand_zero_alloc,
    );
    suite.add(
        "bench/player/step_clock_audio_diff_zero_alloc",
        check_player_steady_step_zero_alloc,
    );
    suite.add(
        "bench/diff/scene_diff_zero_alloc",
        check_scene_diff_zero_alloc,
    );
    suite.add(
        "bench/validation/warrant_bounds_zero_alloc",
        check_warrant_bounds_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
