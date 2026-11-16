//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `palette-indexed`
//!
//! Evaluates `palette-indexed` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates, on the real heap via test-bootstrap's counting global allocator:
//! - zero allocation in `pack_row_into` / `unpack_row_into` at every bit depth,
//! - zero allocation in palette fading (`fade_palette_into`, `crossfade_palette_into`),
//! - zero allocation in scanline and framebuffer blits into `PixelContainer`,
//! - zero allocation in indexed-to-indexed blits,
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
use palette_indexed::{
    blit_indexed_to_container, blit_scanline_to_container, crossfade_palette_into,
    expand_row_to_rgba_into, fade_palette_into, pack_row_into, unpack_row_into, BitDepth,
    ChannelBits, IndexedCanvas, IndexedImage, Palette, PixelContainer, BLIT_SPRITE,
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

fn gradient_palette(depth: BitDepth) -> Palette {
    let mut p = Palette::blank(depth);
    for i in 0..depth.color_count() {
        let v = (i * 255 / (depth.color_count() - 1)) as u8;
        p.set_entry(i as u8, [v, 255 - v, v / 2, 255]).unwrap();
    }
    p
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_pack_unpack_zero_alloc(_ctx: &Ctx) -> Verdict {
    for depth in [BitDepth::Bpp2, BitDepth::Bpp4, BitDepth::Bpp8] {
        let mut indices = [0u8; 64];
        for (i, v) in indices.iter_mut().enumerate() {
            *v = (i % depth.color_count()) as u8;
        }
        let mut packed = [0u8; 64];
        let mut back = [0u8; 64];
        let v = bench_zero_alloc("pack_unpack_zero_alloc", || {
            let n = pack_row_into(depth, black_box(&indices), black_box(&mut packed))
                .expect("pack should succeed");
            unpack_row_into(depth, black_box(&packed[..n]), 64, black_box(&mut back))
                .expect("unpack should succeed");
            black_box(&back);
        });
        if !matches!(v, Verdict::Pass) {
            return v;
        }
    }
    Verdict::Pass
}

fn check_fade_zero_alloc(_ctx: &Ctx) -> Verdict {
    let src = gradient_palette(BitDepth::Bpp8);
    let other = gradient_palette(BitDepth::Bpp8);
    let mut dst = Palette::blank(BitDepth::Bpp8);
    let v = bench_zero_alloc("fade_palette_into_zero_alloc", || {
        fade_palette_into(
            black_box(&src),
            &mut dst,
            [0, 0, 0, 255],
            black_box(5),
            16,
            ChannelBits::Bits5,
        )
        .expect("fade should succeed");
        black_box(&dst);
    });
    if !matches!(v, Verdict::Pass) {
        return v;
    }
    bench_zero_alloc("crossfade_palette_into_zero_alloc", || {
        crossfade_palette_into(
            black_box(&src),
            black_box(&other),
            &mut dst,
            black_box(3),
            8,
            ChannelBits::Bits3,
        )
        .expect("crossfade should succeed");
        black_box(&dst);
    })
}

fn check_blit_container_zero_alloc(_ctx: &Ctx) -> Verdict {
    for depth in [BitDepth::Bpp2, BitDepth::Bpp4, BitDepth::Bpp8] {
        let palette = gradient_palette(depth);
        let mut data = [0u8; 16 * 8];
        let mut canvas = IndexedCanvas::attach(depth, 16, 8, 16, &mut data).unwrap();
        canvas.clear_indices(1).unwrap();
        canvas.put_index(3, 3, 0).unwrap();
        let mut dst = PixelContainer::new(24, 24);
        let v = bench_zero_alloc("blit_indexed_to_container_zero_alloc", || {
            let img = canvas.as_image();
            let n = blit_indexed_to_container(
                black_box(&img),
                black_box(&palette),
                BLIT_SPRITE,
                &mut dst,
                black_box(-2),
                black_box(4),
            )
            .expect("blit should succeed");
            black_box(n);
        });
        if !matches!(v, Verdict::Pass) {
            return v;
        }
        let v = bench_zero_alloc("blit_scanline_to_container_zero_alloc", || {
            let img = canvas.as_image();
            let n = blit_scanline_to_container(
                black_box(&img),
                black_box(3),
                black_box(&palette),
                BLIT_SPRITE,
                &mut dst,
                black_box(2),
                black_box(10),
            )
            .expect("scanline blit should succeed");
            black_box(n);
        });
        if !matches!(v, Verdict::Pass) {
            return v;
        }
    }
    Verdict::Pass
}

fn check_expand_row_zero_alloc(_ctx: &Ctx) -> Verdict {
    let palette = gradient_palette(BitDepth::Bpp4);
    let packed = [0x01u8, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
    let mut out = [0u8; 64];
    bench_zero_alloc("expand_row_to_rgba_into_zero_alloc", || {
        let n = expand_row_to_rgba_into(
            BitDepth::Bpp4,
            black_box(&packed),
            16,
            black_box(&palette),
            true,
            black_box(&mut out),
        )
        .expect("expand should succeed");
        black_box(n);
    })
}

fn check_indexed_blit_zero_alloc(_ctx: &Ctx) -> Verdict {
    let sprite_data = [0x12u8, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
    let sprite = IndexedImage::view(BitDepth::Bpp4, 4, 4, 2, &sprite_data).unwrap();
    let mut fb = [0u8; 8 * 8];
    let mut canvas = IndexedCanvas::attach(BitDepth::Bpp4, 16, 8, 8, &mut fb).unwrap();
    bench_zero_alloc("blit_indexed_zero_alloc", || {
        let n = canvas
            .blit_indexed(black_box(&sprite), black_box(5), black_box(2), BLIT_SPRITE)
            .expect("indexed blit should succeed");
        black_box(n);
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
        ("module", Json::str("palette_indexed")),
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
    suite.add("bench/pack_unpack/zero_alloc", check_pack_unpack_zero_alloc);
    suite.add("bench/fade/zero_alloc", check_fade_zero_alloc);
    suite.add(
        "bench/blit_container/zero_alloc",
        check_blit_container_zero_alloc,
    );
    suite.add("bench/expand_row/zero_alloc", check_expand_row_zero_alloc);
    suite.add(
        "bench/blit_indexed/zero_alloc",
        check_indexed_blit_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
