//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `sprite-oam`
//!
//! Evaluates `sprite-oam` under the Level 5 Microbenchmark Harness
//! (`coding_adventures_test_bootstrap::bench`).
//!
//! Validates, on the real heap via test-bootstrap's counting global allocator:
//! - zero allocation in `draw_order_into` (priority sort),
//! - zero allocation in `scanline_sprites_into` / `scanline_stats`,
//! - zero allocation in `texel_at` (flip and affine sampling),
//! - zero allocation in `Animator::advance` and `OamTable::stamp_frame`,
//! - zero allocation in `lower_to_quads_into`,
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
use sprite_oam::{
    AffineParams, Animation, Animator, MetaFrame, OamEntry, OamTable, Palette, PlayMode,
    SpriteQuad, OAM_SLOTS,
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

fn bench_table() -> OamTable {
    let mut t = OamTable::blank_table();
    for s in 0..OAM_SLOTS {
        let e = OamEntry {
            x: (s as i16 * 5) % 240,
            y: (s as i16 * 3) % 160,
            priority: (s % 4) as u8,
            hidden: s % 7 == 0,
            affine: s % 5 == 0,
            double_size: s % 10 == 0,
            affine_slot: (s % 32) as u8,
            palette_bank: (s % 16) as u8,
            ..OamEntry::blank_entry()
        };
        t.set_entry(s, e).expect("valid entry");
    }
    t.set_affine(
        0,
        AffineParams {
            pa: 181,
            pb: -181,
            pc: 181,
            pd: 181,
        },
    )
    .expect("valid affine slot");
    t
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_draw_order_zero_alloc(_ctx: &Ctx) -> Verdict {
    let t = bench_table();
    let mut out = [0u8; OAM_SLOTS];
    bench_zero_alloc("draw_order_into_zero_alloc", || {
        let n = black_box(&t)
            .draw_order_into(black_box(&mut out))
            .expect("sort should succeed");
        black_box(n);
        black_box(&out);
    })
}

fn check_scanline_zero_alloc(_ctx: &Ctx) -> Verdict {
    let t = bench_table();
    let mut out = [0u8; 8];
    let v = bench_zero_alloc("scanline_sprites_into_zero_alloc", || {
        let r = black_box(&t)
            .scanline_sprites_into(black_box(40), 8, black_box(&mut out))
            .expect("scanline should succeed");
        black_box(r);
    });
    if !matches!(v, Verdict::Pass) {
        return v;
    }
    bench_zero_alloc("scanline_stats_zero_alloc", || {
        let s = black_box(&t)
            .scanline_stats(black_box(160), 8)
            .expect("stats should succeed");
        black_box(s);
    })
}

fn check_texel_zero_alloc(_ctx: &Ctx) -> Verdict {
    let t = bench_table();
    bench_zero_alloc("texel_at_zero_alloc", || {
        for slot in [0usize, 1, 5, 10] {
            let r = black_box(&t)
                .texel_at(black_box(slot), black_box(12), black_box(9))
                .expect("slot is valid");
            black_box(r);
        }
    })
}

fn check_animation_zero_alloc(_ctx: &Ctx) -> Verdict {
    let part = OamEntry {
        hidden: false,
        ..OamEntry::blank_entry()
    };
    let frames = [
        MetaFrame::frame_from_parts(&[part, part], 3).expect("frame"),
        MetaFrame::frame_from_parts(&[part], 2).expect("frame"),
        MetaFrame::frame_from_parts(&[part, part, part], 4).expect("frame"),
    ];
    for mode in [PlayMode::OneShot, PlayMode::Loop, PlayMode::PingPong] {
        let anim = Animation::bind(&frames, mode).expect("animation");
        let mut a = Animator::start();
        let v = bench_zero_alloc("animator_advance_zero_alloc", || {
            let s = a.advance(black_box(&anim), black_box(5)).expect("tick");
            black_box(s);
            if a.is_finished() {
                a.rewind();
            }
        });
        if !matches!(v, Verdict::Pass) {
            return v;
        }
    }
    let mut t = OamTable::blank_table();
    bench_zero_alloc("stamp_frame_zero_alloc", || {
        let n = t
            .stamp_frame(
                black_box(4),
                black_box(&frames[2]),
                black_box(100),
                black_box(60),
                true,
                false,
            )
            .expect("stamp");
        black_box(n);
    })
}

fn check_lowering_zero_alloc(_ctx: &Ctx) -> Verdict {
    let t = bench_table();
    let mut palette = Palette::blank(palette_indexed::BitDepth::Bpp8);
    for i in 0..=255u8 {
        palette
            .set_entry(i, [i, 255 - i, i / 2, 255])
            .expect("entry");
    }
    let mut quads = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); OAM_SLOTS];
    bench_zero_alloc("lower_to_quads_into_zero_alloc", || {
        let n = black_box(&t)
            .lower_to_quads_into(Some(black_box(&palette)), 3, black_box(&mut quads))
            .expect("lowering");
        black_box(n);
        black_box(&quads);
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
        ("module", Json::str("sprite_oam")),
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
    suite.add("bench/draw_order/zero_alloc", check_draw_order_zero_alloc);
    suite.add("bench/scanline/zero_alloc", check_scanline_zero_alloc);
    suite.add("bench/texel/zero_alloc", check_texel_zero_alloc);
    suite.add("bench/animation/zero_alloc", check_animation_zero_alloc);
    suite.add("bench/lowering/zero_alloc", check_lowering_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
