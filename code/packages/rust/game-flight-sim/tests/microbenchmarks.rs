//! # Microbenchmark Suite and Zero-Allocation Verification (`VER07`) for `game-flight-sim`
//!
//! Validates on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `step_in_place` (with `BlackBoxRecorder`)
//! - zero heap allocation in `render_frame_into` (3D terrain + `HudScratch` + retained `PaintScene`)
//! - zero heap allocation in `state_digest_fnv1a`
//! - zero heap allocation in `encode_black_box_into` and `decode_black_box_into`
//! - sabotage detection: an intentional 1-byte allocation is caught and rejected.

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use cockpit_hud_paint::{prepare_paint_scene, HudConfig};
use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use game_flight_sim::{
    decode_black_box_into, encode_black_box_into, render_frame_into, sim_init_level_flight,
    sim_init_on_runway, state_digest_fnv1a, step_in_place, BlackBoxRecorder, CameraMode,
    FlightInputFrame, FlightRenderScratch, FlightSimConfig,
};
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    BackendPreference, ClipSpace, DepthBufferMut, PostProcessMode, RenderTargetSpec,
};
use terrain_lod::{Heightfield, HeightfieldMut, HeightfieldSpec, LodParams, RunwaySpec};

const FID_BENCH_FAIL: &str = "property-counterexample";

coding_adventures_test_bootstrap::install_counting_allocator!();

fn zero_alloc_ok(result: &BenchmarkResult) -> bool {
    result.budget_verdict.is_acceptable()
        && result.alloc_bytes_per_iter == Some(0.0)
        && !result.unchecked_hard_budgets.contains(&"alloc_bytes")
}

fn fast_bench_config(samples: usize) -> BenchmarkConfig {
    BenchmarkConfig {
        warmup_iterations: 8,
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

fn build_terrain_and_runway() -> (HeightfieldSpec, Vec<f64>, RunwaySpec) {
    let spec = HeightfieldSpec::spec_new(4, 250.0, -2000.0, -2000.0).expect("spec");
    let mut heights = vec![0.0_f64; spec.spec_vertex_count().expect("vcount")];
    let runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 1200.0, 80.0, 20.0, 0.0).expect("rwy");
    {
        let mut hmut = HeightfieldMut::from_heights_mut(spec, &mut heights).expect("hmut");
        hmut.synthesize_fbm(45, 0.0, 60.0, 0.0008, 3, 0.5, 2.0)
            .expect("fbm");
        hmut.place_runway(&runway).expect("place rwy");
    }
    (spec, heights, runway)
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_step_in_place_with_fdr_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (spec, heights, runway) = build_terrain_and_runway();
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");
    let cfg = FlightSimConfig::sim_config_default(45, 45);
    let initial = sim_init_on_runway(cfg, &runway).expect("init");
    let header = initial.sim_state_black_box_header().expect("header");
    let mut sim = initial.clone();
    let mut recorder = BlackBoxRecorder::recorder_with_capacity(header, 1024).expect("rec");
    let input = FlightInputFrame {
        dt_ns: cfg.fixed_dt_ns,
        elevator: 0.05,
        aileron: 0.0,
        rudder: 0.0,
        throttle: 0.9,
        flaps: 0.2,
        brake: 0.0,
        gear_down: true,
    };

    bench_zero_alloc("step_in_place_with_fdr_zero_alloc", || {
        if recorder.recorder_is_full() {
            sim = initial.clone();
            recorder.recorder_clear(header).expect("clear");
        }
        let rep = step_in_place(
            black_box(&mut sim),
            black_box(&hf),
            black_box(&runway),
            black_box(&input),
            Some(black_box(&mut recorder)),
        )
        .expect("step");
        black_box(rep);
    })
}

fn check_render_frame_into_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (spec, heights, _runway) = build_terrain_and_runway();
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");
    let cfg = FlightSimConfig::sim_config_default(45, 45);
    let sim = sim_init_level_flight(cfg, 400.0, 55.0).expect("trim");

    let target = RenderTargetSpec {
        width: 32,
        height: 32,
        clear_color: [110, 165, 220, 255],
        clear_depth: 1.0,
        clip_space: ClipSpace::OpenGl,
        post_process: PostProcessMode::Identity,
    };
    let hud_cfg = HudConfig::hud_config_default(640.0, 480.0).expect("hud");
    let lod =
        LodParams::lod_params_new(sim.body.position, 2.0, 32.0, 4, 256, 1024, None).expect("lod");

    let mut fb = PixelContainer::new(32, 32);
    let mut zb = vec![1.0_f64; 32 * 32];
    let mut scratch = FlightRenderScratch::flight_scratch_with_capacity(
        256,
        hf.vertex_count(),
        1024,
        32,
        32,
        2048,
    )
    .expect("scratch");
    let mut scene = prepare_paint_scene(&hud_cfg).expect("scene");

    bench_zero_alloc("render_frame_into_zero_alloc", || {
        let rep = render_frame_into(
            black_box(&sim),
            black_box(&hf),
            black_box(&lod),
            black_box(&hud_cfg),
            black_box(&target),
            BackendPreference::AutoHardwareWithSoftwareFallback,
            CameraMode::Cockpit,
            black_box(&mut fb),
            DepthBufferMut::F64(black_box(&mut zb)),
            black_box(&mut scratch),
            Some(black_box(&mut scene)),
        )
        .expect("render");
        black_box(rep);
    })
}

fn check_state_digest_zero_alloc(_ctx: &Ctx) -> Verdict {
    let cfg = FlightSimConfig::sim_config_default(45, 45);
    let sim = sim_init_level_flight(cfg, 400.0, 55.0).expect("trim");

    bench_zero_alloc("state_digest_fnv1a_zero_alloc", || {
        let d = state_digest_fnv1a(black_box(&sim)).expect("digest");
        black_box(d);
    })
}

fn check_fdr_codec_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (spec, heights, runway) = build_terrain_and_runway();
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");
    let cfg = FlightSimConfig::sim_config_default(45, 45);
    let mut sim = sim_init_on_runway(cfg, &runway).expect("init");
    let header = sim.sim_state_black_box_header().expect("hdr");
    let mut rec = BlackBoxRecorder::recorder_with_capacity(header, 16).expect("rec");
    let input = FlightInputFrame {
        dt_ns: cfg.fixed_dt_ns,
        elevator: 0.0,
        aileron: 0.0,
        rudder: 0.0,
        throttle: 0.8,
        flaps: 0.0,
        brake: 0.0,
        gear_down: true,
    };
    for _ in 0..8 {
        step_in_place(&mut sim, &hf, &runway, &input, Some(&mut rec)).expect("step");
    }
    let needed = rec.recorder_encoded_len().expect("len");
    let mut blob = vec![0_u8; needed];
    let mut rec_out = BlackBoxRecorder::recorder_with_capacity(header, 16).expect("rec_out");

    bench_zero_alloc("fdr_encode_decode_into_zero_alloc", || {
        let n = encode_black_box_into(black_box(&rec), black_box(&mut blob)).expect("enc");
        let cnt =
            decode_black_box_into(black_box(&blob[..n]), black_box(&mut rec_out)).expect("dec");
        black_box(cnt);
    })
}

fn check_sabotage_allocation_detected(_ctx: &Ctx) -> Verdict {
    let harness = BenchmarkHarness::new(fast_bench_config(5));
    let budget = zero_budget();
    let counter = AllocCounter::new();
    let leaky = harness.run_with_counter("sabotage_alloc_one_byte", &budget, &counter, || {
        let v: Vec<u8> = vec![42];
        black_box(v);
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
        black_box(1_u64) ^ black_box(2_u64)
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
        ("module", Json::str("game-flight-sim")),
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
        "bench/step_in_place/with_fdr_zero_alloc",
        check_step_in_place_with_fdr_zero_alloc,
    );
    suite.add(
        "bench/render_frame_into/terrain_hud_scene_zero_alloc",
        check_render_frame_into_zero_alloc,
    );
    suite.add(
        "bench/state_digest_fnv1a/zero_alloc",
        check_state_digest_zero_alloc,
    );
    suite.add(
        "bench/fdr_codec/encode_decode_into_zero_alloc",
        check_fdr_codec_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
