//! # Microbenchmark Suite and Zero-Allocation Verification for `flash-display-list`
//!
//! Validates via `test-bootstrap`'s counting global allocator:
//! - zero heap allocation in steady-state timeline advancement,
//! - zero heap allocation in hierarchy depth and display bounds validation,
//! - zero heap allocation in affine coordinate transforms,
//! - zero heap allocation in interactive button state machine transitions,
//! - zero heap allocation in retained-mode scene diffing (`diff_paint_scene_into`),
//! - sabotage detection: an intentional allocation is caught and rejected.

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
use flash_display_list::{
    diff_paint_scene_into, swf_matrix_to_affine, validate_display_depth, validate_hierarchy_depth,
    validate_morph_ratio, MovieClip, PaintBase, PaintInstruction, PaintRect, PaintScene, PatchKind,
    PatchOp, SimpleButton,
};
use point2d::Point;
use swf_container::SwfMatrix;

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

fn check_timeline_step_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut clip = MovieClip::new(100);
    bench_zero_alloc("timeline_step_zero_alloc", || {
        black_box(&mut clip).clip_step_timeline();
        let frame = black_box(&clip).clip_current_frame();
        black_box(frame);
    })
}

fn check_bounds_validation_zero_alloc(_ctx: &Ctx) -> Verdict {
    bench_zero_alloc("bounds_validation_zero_alloc", || {
        let v_hier = validate_hierarchy_depth(black_box(32)).is_ok();
        let v_depth = validate_display_depth(black_box(100)).is_ok();
        let v_ratio = validate_morph_ratio(black_box(32768)).is_ok();
        black_box((v_hier, v_depth, v_ratio));
    })
}

fn check_affine_transforms_zero_alloc(_ctx: &Ctx) -> Verdict {
    let m = SwfMatrix {
        scale_x: 2.0,
        scale_y: 2.0,
        rotate_skew_0: 0.1,
        rotate_skew_1: -0.1,
        translate_x: 400,
        translate_y: 600,
    };
    let pt = Point::new(10.0, 20.0);

    bench_zero_alloc("affine_transforms_zero_alloc", || {
        let affine = swf_matrix_to_affine(black_box(&m));
        let transformed = affine.apply_to_point(black_box(pt));
        let inverted = affine.invert();
        black_box((transformed, inverted));
    })
}

fn check_button_transitions_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut btn = SimpleButton::new();
    bench_zero_alloc("button_transitions_zero_alloc", || {
        let ev1 = black_box(&mut btn).button_process_mouse_move(black_box(true));
        let ev2 = black_box(&mut btn).button_process_mouse_down(black_box(true));
        let ev3 = black_box(&mut btn).button_process_mouse_up(black_box(true));
        let ev4 = black_box(&mut btn).button_process_mouse_move(black_box(false));
        black_box((ev1, ev2, ev3, ev4));
    })
}

fn make_rect(id: &str, x: f64) -> PaintInstruction {
    let mut r = PaintRect::filled(x, 0.0, 10.0, 10.0, "#ff0000");
    r.base = PaintBase {
        id: Some(id.to_string()),
        metadata: None,
    };
    PaintInstruction::Rect(r)
}

fn check_scene_diff_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut prev = PaintScene::new(320.0, 240.0);
    let mut next = PaintScene::new(320.0, 240.0);
    for i in 0..16 {
        prev.instructions
            .push(make_rect(&format!("obj_{i}"), i as f64 * 10.0));
        let moved = if i % 2 == 0 {
            (i as f64 + 1.0) * 10.0
        } else {
            i as f64 * 10.0
        };
        next.instructions
            .push(make_rect(&format!("obj_{i}"), moved));
    }
    prev.instructions.push(make_rect("removed_obj", 0.0));
    next.instructions.push(make_rect("added_obj", 0.0));

    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 32];

    bench_zero_alloc("diff_paint_scene_into_zero_alloc", || {
        let res = diff_paint_scene_into(black_box(&prev), black_box(&next), black_box(&mut ops));
        let summary = black_box(res).expect("diff must succeed");
        assert_eq!(summary.removed, 1);
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
        ("module", Json::str("flash_display_list")),
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
    suite.add("bench/timeline/zero_alloc", check_timeline_step_zero_alloc);
    suite.add(
        "bench/validation/zero_alloc",
        check_bounds_validation_zero_alloc,
    );
    suite.add(
        "bench/affine/zero_alloc",
        check_affine_transforms_zero_alloc,
    );
    suite.add(
        "bench/button/zero_alloc",
        check_button_transitions_zero_alloc,
    );
    suite.add("bench/diff/zero_alloc", check_scene_diff_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
