//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `paint-palette-blend`
//!
//! Evaluates `paint-palette-blend` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates hard zero-heap-allocation budgets, measured on the real heap by
//! test-bootstrap's counting global allocator, across:
//! - per-pixel blending (`blend_pixel`) in every mode,
//! - indexed scanline blits and clipped, flipped image blits,
//! - RGBA span compositing,
//! - retained-mode scene diffing (`diff_paint_scene_into`).
//!
//! Also proves counting-allocator liveness and sabotage detection.

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
use paint_palette_blend::{
    blend_pixel, blend_rgba_scanline, blit_indexed_image, blit_indexed_scanline,
    diff_paint_scene_into, BlendMode, BlitParams, IndexedImage, PaintBase, PaintInstruction,
    PaintScene, PatchKind, PatchOp, Rgba8, RgbaTarget, OPAQUE_BLIT,
};

const FID_BENCH_FAIL: &str = "property-counterexample";

coding_adventures_test_bootstrap::install_counting_allocator!();

const PALETTE: [Rgba8; 4] = [
    [0, 0, 0, 0],
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 128],
];

const MODES: [BlendMode; 5] = [
    BlendMode::Normal,
    BlendMode::Add,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Alpha,
];

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

fn run_zero_alloc<F: FnMut()>(name: &str, body: F) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(20));
    let budget = Budget {
        max_alloc_bytes: Some(0),
        ..Default::default()
    };
    let alloc = AllocCounter::new();
    let res = harness.run_with_counter(name, &budget, &alloc, body);
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

fn check_blend_pixel_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut acc = 0u8;
    run_zero_alloc("blend_pixel_all_modes", || {
        for mode in MODES {
            let out = blend_pixel(
                black_box(mode),
                black_box([200, 100, 50, 220]),
                black_box([10, 20, 30, 255]),
                black_box(180),
            );
            acc ^= out[0] ^ out[1] ^ out[2] ^ out[3];
        }
        black_box(acc);
    })
}

fn check_scanline_zero_alloc(_ctx: &Ctx) -> Verdict {
    let indices: Vec<u8> = (0..256u32).map(|i| (i % 4) as u8).collect();
    let mut dst = vec![0u8; 256 * 4];
    let params = BlitParams {
        mode: BlendMode::Alpha,
        alpha: 200,
        transparent_index: Some(0),
        h_flip: true,
        v_flip: false,
    };
    run_zero_alloc("blit_indexed_scanline", || {
        let r = blit_indexed_scanline(
            black_box(&mut dst),
            black_box(&indices),
            black_box(&PALETTE),
            black_box(&params),
        );
        black_box(r).expect("scanline blit should succeed");
    })
}

fn check_image_blit_zero_alloc(_ctx: &Ctx) -> Verdict {
    let pixels: Vec<u8> = (0..32 * 32u32).map(|i| (i % 4) as u8).collect();
    let img = IndexedImage {
        width: 32,
        height: 32,
        stride: 32,
        pixels: &pixels,
    };
    let mut fb = vec![0u8; 64 * 64 * 4];
    let params = BlitParams {
        mode: BlendMode::Screen,
        alpha: 255,
        transparent_index: Some(0),
        h_flip: true,
        v_flip: true,
    };
    let mut target = RgbaTarget {
        width: 64,
        height: 64,
        stride: 64 * 4,
        pixels: &mut fb,
    };
    run_zero_alloc("blit_indexed_image_clipped", || {
        let r = blit_indexed_image(
            black_box(&mut target),
            black_box(-8),
            black_box(40),
            black_box(&img),
            black_box(&PALETTE),
            black_box(&params),
        );
        black_box(r).expect("image blit should succeed");
        let r2 = blit_indexed_image(
            black_box(&mut target),
            black_box(16),
            black_box(16),
            black_box(&img),
            black_box(&PALETTE),
            black_box(&OPAQUE_BLIT),
        );
        black_box(r2).expect("image blit should succeed");
    })
}

fn check_rgba_scanline_zero_alloc(_ctx: &Ctx) -> Verdict {
    let src = vec![120u8; 128 * 4];
    let mut dst = vec![60u8; 128 * 4];
    run_zero_alloc("blend_rgba_scanline", || {
        for mode in MODES {
            let r = blend_rgba_scanline(
                black_box(&mut dst),
                black_box(&src),
                black_box(mode),
                black_box(128),
            );
            black_box(r).expect("span blend should succeed");
        }
    })
}

fn rect(id: &str, x: f64) -> PaintInstruction {
    let mut r = paint_instructions::PaintRect::filled(x, 0.0, 8.0, 8.0, "#ff0000");
    r.base = PaintBase {
        id: Some(id.to_string()),
        metadata: None,
    };
    PaintInstruction::Rect(r)
}

fn check_scene_diff_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut prev = PaintScene::new(320.0, 240.0);
    let mut next = PaintScene::new(320.0, 240.0);
    for i in 0..32 {
        prev.instructions.push(rect(&format!("s{i}"), i as f64));
        let moved = if i % 4 == 0 { i as f64 + 1.0 } else { i as f64 };
        next.instructions.push(rect(&format!("s{i}"), moved));
    }
    prev.instructions.push(rect("gone", 0.0));
    next.instructions.push(rect("fresh", 0.0));
    let mut ops = [PatchOp {
        kind: PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 64];
    run_zero_alloc("diff_paint_scene_into", || {
        let r = diff_paint_scene_into(black_box(&prev), black_box(&next), black_box(&mut ops));
        let summary = black_box(r).expect("diff should succeed");
        assert_eq!(summary.removed, 1);
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = Budget {
        max_alloc_bytes: Some(0),
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
        ("module", Json::str("paint_palette_blend")),
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
    suite.add("bench/blend_pixel/zero_alloc", check_blend_pixel_zero_alloc);
    suite.add(
        "bench/blit_indexed_scanline/zero_alloc",
        check_scanline_zero_alloc,
    );
    suite.add(
        "bench/blit_indexed_image/zero_alloc",
        check_image_blit_zero_alloc,
    );
    suite.add(
        "bench/blend_rgba_scanline/zero_alloc",
        check_rgba_scanline_zero_alloc,
    );
    suite.add(
        "bench/diff_paint_scene_into/zero_alloc",
        check_scene_diff_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
