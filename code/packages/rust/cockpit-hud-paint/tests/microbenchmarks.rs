//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `cockpit-hud-paint`
//!
//! Validates, on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `build_hud_frame_into` (`CombinedPfdHud` and `HudOverlayOnly`)
//! - zero heap allocation in `update_paint_scene_into` (pre-allocated `PaintScene`)
//! - zero heap allocation in `hud_scratch_digest_fnv1a` and `paint_scene_digest_fnv1a`
//! - sabotage detection: an intentional allocation is caught and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use cockpit_hud_paint::{
    build_hud_frame_into, paint_scene_digest_fnv1a, prepare_paint_scene, update_paint_scene_into,
    FlightTelemetry, HudConfig, HudDisplayMode, HudScratch,
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

fn sample_telemetry() -> FlightTelemetry {
    let mut tel = FlightTelemetry::telemetry_level_flight();
    tel.pitch_rad = trig::radians(8.5);
    tel.roll_rad = trig::radians(-15.0);
    tel.heading_rad = trig::radians(274.0);
    tel.indicated_airspeed_mps = 142.0;
    tel.mach = 0.44;
    tel.altitude_msl_m = 4200.0;
    tel.radar_altitude_agl_m = 1800.0;
    tel.vertical_speed_mps = 6.2;
    tel.flight_path_pitch_rad = trig::radians(6.0);
    tel.flight_path_yaw_rad = trig::radians(1.5);
    tel.aoa_rad = trig::radians(4.2);
    tel.g_load = 1.15;
    tel
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_build_hud_frame_combined_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cfg = HudConfig::hud_config_default(1280.0, 720.0).expect("valid config");
    let tel = sample_telemetry();
    let mut scratch = HudScratch::hud_scratch_new();
    bench_zero_alloc("build_hud_frame_combined_zero_alloc", || {
        let sum = build_hud_frame_into(black_box(&tel), black_box(&cfg), black_box(&mut scratch))
            .expect("build should succeed");
        black_box(sum);
    })
}

fn check_build_hud_frame_overlay_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut cfg = HudConfig::hud_config_default(1280.0, 720.0).expect("valid config");
    cfg.mode = HudDisplayMode::HudOverlayOnly;
    let tel = sample_telemetry();
    let mut scratch = HudScratch::hud_scratch_new();
    bench_zero_alloc("build_hud_frame_overlay_zero_alloc", || {
        let sum = build_hud_frame_into(black_box(&tel), black_box(&cfg), black_box(&mut scratch))
            .expect("build should succeed");
        black_box(sum);
    })
}

fn check_update_paint_scene_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cfg = HudConfig::hud_config_default(1280.0, 720.0).expect("valid config");
    let tel = sample_telemetry();
    let mut scene = prepare_paint_scene(&cfg).expect("prepare should succeed");
    bench_zero_alloc("update_paint_scene_zero_alloc", || {
        let sum = update_paint_scene_into(black_box(&tel), black_box(&cfg), black_box(&mut scene))
            .expect("update should succeed");
        black_box(sum);
    })
}

fn check_digests_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cfg = HudConfig::hud_config_default(1280.0, 720.0).expect("valid config");
    let tel = sample_telemetry();
    let mut scratch = HudScratch::hud_scratch_new();
    build_hud_frame_into(&tel, &cfg, &mut scratch).expect("build should succeed");
    let mut scene = prepare_paint_scene(&cfg).expect("prepare should succeed");
    update_paint_scene_into(&tel, &cfg, &mut scene).expect("update should succeed");

    bench_zero_alloc("digests_zero_alloc", || {
        let d1 = black_box(&scratch)
            .hud_scratch_digest_fnv1a()
            .expect("scratch digest");
        let d2 = paint_scene_digest_fnv1a(black_box(&scene)).expect("scene digest");
        black_box((d1, d2));
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
        ("module", Json::str("cockpit-hud-paint")),
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
        "bench/build_hud_frame_into/combined_zero_alloc",
        check_build_hud_frame_combined_zero_alloc,
    );
    suite.add(
        "bench/build_hud_frame_into/overlay_zero_alloc",
        check_build_hud_frame_overlay_zero_alloc,
    );
    suite.add(
        "bench/update_paint_scene_into/zero_alloc",
        check_update_paint_scene_zero_alloc,
    );
    suite.add("bench/digests/zero_alloc", check_digests_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
