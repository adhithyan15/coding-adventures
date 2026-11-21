//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `math3d`
//!
//! Validates, on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `Mat4::mul`
//! - zero heap allocation in `Quat::slerp`
//! - zero heap allocation in `Frustum::intersects_aabb`
//! - zero heap allocation in `Vec3::normalize`
//! - zero heap allocation in `ray_aabb_intersect` and `Mat4::invert`
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
use math3d::{ray_aabb_intersect, Aabb3, Frustum, Mat4, Quat, Ray3, Vec3};

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

fn check_mat4_mul_zero_alloc(_ctx: &Ctx) -> Verdict {
    let view = Mat4::look_at(Vec3::new(0.0, 2.0, 10.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let proj = Mat4::perspective(1.047, 16.0 / 9.0, 0.1, 1000.0).unwrap();
    bench_zero_alloc("mat4_mul_zero_alloc", || {
        let r = black_box(proj)
            .mul(black_box(&view))
            .expect("mul should succeed");
        black_box(r);
    })
}

fn check_quat_slerp_zero_alloc(_ctx: &Ctx) -> Verdict {
    let a = Quat::from_axis_angle(Vec3::unit_y(), 0.2).unwrap();
    let b = Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), 1.4).unwrap();
    bench_zero_alloc("quat_slerp_zero_alloc", || {
        let q = black_box(a)
            .slerp(black_box(b), black_box(0.35))
            .expect("slerp should succeed");
        black_box(q);
    })
}

fn check_frustum_intersects_aabb_zero_alloc(_ctx: &Ctx) -> Verdict {
    let view = Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::zero(), Vec3::unit_y()).unwrap();
    let proj = Mat4::perspective(1.047, 1.0, 0.5, 100.0).unwrap();
    let vp = proj.mul(&view).unwrap();
    let frustum = Frustum::from_view_proj(&vp).unwrap();
    let aabb =
        Aabb3::aabb3_from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)).unwrap();
    bench_zero_alloc("frustum_intersects_aabb_zero_alloc", || {
        let hit = black_box(&frustum)
            .intersects_aabb(black_box(&aabb))
            .expect("intersects_aabb should succeed");
        black_box(hit);
    })
}

fn check_vec3_normalize_zero_alloc(_ctx: &Ctx) -> Verdict {
    let v = Vec3::new(3.0, -4.0, 12.0);
    bench_zero_alloc("vec3_normalize_zero_alloc", || {
        let n = black_box(v).normalize().expect("normalize should succeed");
        black_box(n);
    })
}

fn check_ray_aabb_zero_alloc(_ctx: &Ctx) -> Verdict {
    let ray = Ray3::ray3_from_origin_dir(Vec3::new(-5.0, 0.2, 0.1), Vec3::unit_x()).unwrap();
    let aabb =
        Aabb3::aabb3_from_min_max(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)).unwrap();
    bench_zero_alloc("ray_aabb_intersect_zero_alloc", || {
        let hit =
            ray_aabb_intersect(black_box(&ray), black_box(&aabb)).expect("ray_aabb should succeed");
        black_box(hit);
    })
}

fn check_mat4_invert_zero_alloc(_ctx: &Ctx) -> Verdict {
    let trs = Mat4::mat4_from_trs(
        Vec3::new(3.0, -4.0, 5.0),
        Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), 0.75).unwrap(),
        Vec3::new(2.0, 1.5, 3.0),
    )
    .unwrap();
    bench_zero_alloc("mat4_invert_zero_alloc", || {
        let inv = black_box(trs).invert().expect("invert should succeed");
        black_box(inv);
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
        ("module", Json::str("math3d")),
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
    suite.add("bench/mat4_mul/zero_alloc", check_mat4_mul_zero_alloc);
    suite.add("bench/quat_slerp/zero_alloc", check_quat_slerp_zero_alloc);
    suite.add(
        "bench/frustum_intersects_aabb/zero_alloc",
        check_frustum_intersects_aabb_zero_alloc,
    );
    suite.add(
        "bench/vec3_normalize/zero_alloc",
        check_vec3_normalize_zero_alloc,
    );
    suite.add("bench/ray_aabb/zero_alloc", check_ray_aabb_zero_alloc);
    suite.add("bench/mat4_invert/zero_alloc", check_mat4_invert_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
