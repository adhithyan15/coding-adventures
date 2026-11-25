//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `game-labyrinth3d`
//!
//! Validates on the real heap via `coding_adventures_test_bootstrap`'s `CountingAllocator`:
//! - zero heap allocation in steady-state `step_in_place`
//! - zero heap allocation in steady-state `render_frame_into` (`RenderMode::Raycast25D`)
//! - zero heap allocation in steady-state `render_frame_into` (`RenderMode::Software3D`)
//! - zero heap allocation in `frame_digest_fnv1a` and `state_digest_fnv1a`
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
use game_labyrinth3d::{
    frame_digest_fnv1a, render_frame_into, state_digest_fnv1a, step_in_place, LabyrinthAssets,
    LabyrinthInput, LabyrinthRenderScratch, LabyrinthState, PixelContainer, RenderMode,
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
        warmup_iterations: 5,
        warmup_max_duration: Duration::from_millis(1),
        min_batch_duration: Duration::from_micros(10),
        max_batch_size: 1000,
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
    let harness = BenchmarkHarness::new(fast_bench_config(15));
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

fn check_step_in_place_zero_alloc(_ctx: &Ctx) -> Verdict {
    let mut state = LabyrinthState::state_build_demo(42).expect("demo state");
    let input = LabyrinthInput::input_new(0.4, 0.1, 0.25, false, 16_666_667).expect("input");
    bench_zero_alloc("step_in_place", || {
        let out = step_in_place(&mut state, input).expect("step_in_place");
        let _ = black_box(out.state_digest);
    })
}

fn check_render_raycast_25d_zero_alloc(_ctx: &Ctx) -> Verdict {
    let state = LabyrinthState::state_build_demo(42).expect("demo state");
    let assets = LabyrinthAssets::assets_default().expect("assets");
    let mut scratch = LabyrinthRenderScratch::rscratch_for_state(&state).expect("scratch");
    let mut fb = PixelContainer::new(64, 48);
    let mut zb = vec![0.0_f64; 64 * 48];
    bench_zero_alloc("render_raycast_25d", || {
        let rep = render_frame_into(
            &state,
            &assets,
            RenderMode::Raycast25D,
            &mut scratch,
            &mut zb,
            &mut fb,
        )
        .expect("render raycast");
        let _ = black_box(rep.frame_digest);
    })
}

fn check_render_software_3d_zero_alloc(_ctx: &Ctx) -> Verdict {
    let state = LabyrinthState::state_build_demo(42).expect("demo state");
    let assets = LabyrinthAssets::assets_default().expect("assets");
    let mut scratch = LabyrinthRenderScratch::rscratch_for_state(&state).expect("scratch");
    let mut fb = PixelContainer::new(48, 36);
    let mut zb = vec![1.0_f64; 48 * 36];
    bench_zero_alloc("render_software_3d", || {
        let rep = render_frame_into(
            &state,
            &assets,
            RenderMode::Software3D,
            &mut scratch,
            &mut zb,
            &mut fb,
        )
        .expect("render 3d");
        let _ = black_box(rep.frame_digest);
    })
}

fn check_digests_zero_alloc(_ctx: &Ctx) -> Verdict {
    let state = LabyrinthState::state_build_demo(42).expect("demo state");
    let fb = PixelContainer::new(32, 32);
    bench_zero_alloc("digests", || {
        let _ = black_box(frame_digest_fnv1a(&fb) ^ state_digest_fnv1a(&state));
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(10));
    let budget = zero_budget();
    let counter = AllocCounter::new();
    let sabotaged = harness.run_with_counter("sabotage_alloc", &budget, &counter, || {
        let v = Vec::<u8>::with_capacity(1);
        let _ = black_box(v);
    });
    if !matches!(
        sabotaged.budget_verdict,
        BudgetVerdict::Failed {
            observed: 1,
            limit: 0,
            ..
        }
    ) {
        return Verdict::fail(
            FID_BENCH_FAIL,
            "a closure allocating heap bytes did not fail the zero-allocation budget",
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
        ("module", Json::str("game-labyrinth3d")),
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
        "bench/step_in_place/zero_alloc",
        check_step_in_place_zero_alloc,
    );
    suite.add(
        "bench/render_raycast_25d/zero_alloc",
        check_render_raycast_25d_zero_alloc,
    );
    suite.add(
        "bench/render_software_3d/zero_alloc",
        check_render_software_3d_zero_alloc,
    );
    suite.add("bench/digests/zero_alloc", check_digests_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
