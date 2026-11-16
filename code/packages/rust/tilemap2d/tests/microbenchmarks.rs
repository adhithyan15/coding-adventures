//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `tilemap2d`
//!
//! Evaluates `tilemap2d` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates, on the real heap via test-bootstrap's counting global allocator:
//! - zero allocation in scroll resolution and viewport culling,
//! - zero allocation in point / AABB collision queries,
//! - zero allocation in tile-cell lowering into a caller buffer,
//! - zero allocation in scanline and full-frame indexed rendering,
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
use tilemap2d::{
    aabb_attrs, aabb_blocked, aabb_first_hit, attr_at_point, cull_layer, layer_scroll,
    lower_layer_cells_into, render_layer_scanline, render_layers_to_container, Aabb, BitDepth,
    BlendMode, Camera, MotionHint, Palette, PixelContainer, TileBatchCell, TileCell, TileLayer,
    Tileset, Viewport, WrapMode, ATTR_ONE_WAY, ATTR_SOLID, ATTR_TRIGGER,
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

/// A 16 x 16 map of 8 x 8 tiles with a mix of tiles, flips, banks and attributes.
fn fixture_cells() -> Vec<TileCell> {
    (0..16u32 * 16)
        .map(|i| {
            let (c, r) = (i % 16, i / 16);
            TileCell::tile((i % 16) as u16)
                .with_flip(c % 2 == 0, r % 3 == 0)
                .with_bank((i % 2) as u8)
                .with_attr(match i % 5 {
                    0 => ATTR_SOLID,
                    1 => ATTR_ONE_WAY,
                    2 => ATTR_TRIGGER,
                    _ => 0,
                })
        })
        .collect()
}

fn fixture_tiles() -> Vec<u8> {
    // 16 Bpp4 tiles of 8 x 8: row_len 4, tile_bytes 32. Every byte is a valid pair of indices.
    (0..16 * 32).map(|i| (i * 37 % 251) as u8).collect()
}

fn fixture_banks() -> [Palette; 2] {
    let mut a = Palette::blank(BitDepth::Bpp4);
    let mut b = Palette::blank(BitDepth::Bpp4);
    for i in 0..16u8 {
        a.set_entry(i, [i * 16, 255 - i * 16, 40, 255]).unwrap();
        b.set_entry(i, [255 - i * 16, i * 16, 200, 255]).unwrap();
    }
    [a, b]
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_scroll_and_cull_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cells = fixture_cells();
    let mut layer = TileLayer::attach(&cells, 16, 16, 8, 8).unwrap();
    layer.parallax_x_q8 = 128;
    let view = Viewport {
        width: 160,
        height: 144,
    };
    for mode in [WrapMode::Wrap, WrapMode::Clamp, WrapMode::Void] {
        layer.wrap_x = mode;
        layer.wrap_y = mode;
        let v = bench_zero_alloc("scroll_cull_zero_alloc", || {
            let cam = Camera {
                x: black_box(37),
                y: black_box(-19),
            };
            let s = layer_scroll(black_box(&layer), view, cam).expect("scroll");
            let r = cull_layer(black_box(&layer), view, cam).expect("cull");
            black_box((s, r));
        });
        if !matches!(v, Verdict::Pass) {
            return v;
        }
    }
    Verdict::Pass
}

fn check_collision_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cells = fixture_cells();
    let layer = TileLayer::attach(&cells, 16, 16, 8, 8).unwrap();
    bench_zero_alloc("collision_zero_alloc", || {
        let p = attr_at_point(black_box(&layer), black_box(37), black_box(-5));
        let b = Aabb {
            x: black_box(10),
            y: black_box(12),
            w: 20,
            h: 24,
        };
        let u = aabb_attrs(&layer, b).expect("union");
        let h = aabb_first_hit(&layer, b, ATTR_TRIGGER).expect("hit");
        let blocked = aabb_blocked(
            &layer,
            b,
            MotionHint {
                moving_down: black_box(true),
                prev_bottom: 8,
            },
        )
        .expect("blocked");
        black_box((p, u, h, blocked));
    })
}

fn check_lowering_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cells = fixture_cells();
    let layer = TileLayer::attach(&cells, 16, 16, 8, 8).unwrap();
    let view = Viewport {
        width: 160,
        height: 144,
    };
    let tints = [[255u8, 0, 0], [0, 255, 0]];
    let mut out = [TileBatchCell::new(0, 0, 0); 512];
    bench_zero_alloc("lower_cells_zero_alloc", || {
        let rect = cull_layer(&layer, view, black_box(Camera { x: 5, y: 9 })).expect("cull");
        let n = lower_layer_cells_into(&layer, &rect, &tints, &mut out).expect("lower");
        black_box(n);
    })
}

fn check_render_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cells = fixture_cells();
    let tiles = fixture_tiles();
    let banks = fixture_banks();
    let tileset = Tileset::attach_tiles(BitDepth::Bpp4, &tiles, 8, 8, 16).unwrap();
    let mut back = TileLayer::attach(&cells, 16, 16, 8, 8).unwrap();
    back.color0_transparent = false;
    let mut front = TileLayer::attach(&cells, 16, 16, 8, 8).unwrap();
    front.parallax_x_q8 = 512;
    front.blend = BlendMode::Add;
    let layers = [back, front];
    let mut dst = PixelContainer::new(96, 64);

    let v = bench_zero_alloc("render_frame_zero_alloc", || {
        let n = render_layers_to_container(
            black_box(&layers),
            &tileset,
            &banks,
            black_box(Camera { x: 11, y: -7 }),
            Some([0, 0, 0, 255]),
            &mut dst,
        )
        .expect("render");
        black_box(n);
    });
    if !matches!(v, Verdict::Pass) {
        return v;
    }
    let view = Viewport {
        width: 96,
        height: 64,
    };
    let mut line = [0u8; 96 * 4];
    bench_zero_alloc("render_scanline_zero_alloc", || {
        let n = render_layer_scanline(
            black_box(&front),
            &tileset,
            &banks,
            view,
            black_box(Camera { x: 3, y: 4 }),
            black_box(17),
            &mut line,
        )
        .expect("scanline");
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
        ("module", Json::str("tilemap2d")),
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
        "bench/scroll_cull/zero_alloc",
        check_scroll_and_cull_zero_alloc,
    );
    suite.add("bench/collision/zero_alloc", check_collision_zero_alloc);
    suite.add("bench/lowering/zero_alloc", check_lowering_zero_alloc);
    suite.add("bench/render/zero_alloc", check_render_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
