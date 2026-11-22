//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `terrain-lod`
//!
//! Validates, on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `Heightfield::height_at`
//! - zero heap allocation in `Heightfield::normal_at`
//! - zero heap allocation in `HeightfieldMut::place_runway`
//! - zero heap allocation in `refine_into` (reusing `TerrainScratch`)
//! - zero heap allocation in `emit_mesh_into` (writing into caller slice)
//! - zero heap allocation in `mesh_digest_fnv1a`
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
use math3d::Vec3;
use terrain_lod::{
    emit_mesh_into, mesh_digest_fnv1a, refine_into, Heightfield, HeightfieldMut, HeightfieldSpec,
    LodParams, RunwaySpec, TerrainScratch,
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
        warmup_max_duration: Duration::from_millis(2),
        min_batch_duration: Duration::from_micros(20),
        max_batch_size: 10_000,
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

fn synth_test_terrain(level: u32) -> (HeightfieldSpec, Vec<f64>) {
    let spec = HeightfieldSpec::spec_new(level, 10.0, -160.0, -160.0).unwrap();
    let mut buf = vec![0.0; spec.spec_vertex_count().unwrap()];
    HeightfieldMut::from_heights_mut(spec, &mut buf)
        .unwrap()
        .synthesize_diamond_square(0x4242, 50.0, 25.0, 0.6)
        .unwrap();
    (spec, buf)
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_height_and_normal_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (spec, buf) = synth_test_terrain(5);
    let hf = Heightfield::from_heights(spec, &buf).unwrap();
    bench_zero_alloc("height_and_normal_zero_alloc", || {
        let h = hf
            .height_at(black_box(12.5), black_box(-34.2))
            .expect("height_at");
        let n = hf
            .normal_at(black_box(12.5), black_box(-34.2))
            .expect("normal_at");
        black_box((h, n));
    })
}

fn check_place_runway_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (spec, mut buf) = synth_test_terrain(5);
    let mut hfm = HeightfieldMut::from_heights_mut(spec, &mut buf).unwrap();
    let runway = RunwaySpec::runway_new(0.0, 0.0, 0.35, 100.0, 30.0, 20.0, 55.0).unwrap();
    bench_zero_alloc("place_runway_zero_alloc", || {
        hfm.place_runway(black_box(&runway)).expect("place_runway");
    })
}

fn check_refine_into_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (spec, buf) = synth_test_terrain(5);
    let hf = Heightfield::from_heights(spec, &buf).unwrap();
    let mut scratch = TerrainScratch::scratch_with_capacity(4096).unwrap();
    let params =
        LodParams::lod_params_new(Vec3::new(0.0, 80.0, 0.0), 0.02, 1.0, 5, 4096, 16_384, None)
            .unwrap();
    bench_zero_alloc("refine_into_zero_alloc", || {
        let stats =
            refine_into(black_box(&hf), black_box(&params), &mut scratch).expect("refine_into");
        black_box(stats);
    })
}

fn check_emit_mesh_and_digest_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (spec, buf) = synth_test_terrain(5);
    let hf = Heightfield::from_heights(spec, &buf).unwrap();
    let mut scratch = TerrainScratch::scratch_with_capacity(4096).unwrap();
    let params =
        LodParams::lod_params_new(Vec3::new(0.0, 80.0, 0.0), 0.02, 1.0, 5, 4096, 16_384, None)
            .unwrap();
    let stats = refine_into(&hf, &params, &mut scratch).unwrap();
    let mut indices = vec![0u32; stats.index_count + 64];
    bench_zero_alloc("emit_mesh_and_digest_zero_alloc", || {
        let count = emit_mesh_into(black_box(&hf), black_box(&scratch), &mut indices, 16_384)
            .expect("emit_mesh_into");
        let digest = mesh_digest_fnv1a(black_box(&hf), &indices[..count]).expect("digest");
        black_box((count, digest));
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
        ("module", Json::str("terrain-lod")),
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
        "bench/height_and_normal/zero_alloc",
        check_height_and_normal_zero_alloc,
    );
    suite.add(
        "bench/place_runway/zero_alloc",
        check_place_runway_zero_alloc,
    );
    suite.add("bench/refine_into/zero_alloc", check_refine_into_zero_alloc);
    suite.add(
        "bench/emit_mesh_and_digest/zero_alloc",
        check_emit_mesh_and_digest_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
