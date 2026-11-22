//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `rigidbody-6dof`
//!
//! Validates, on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `step_in_place` (`VelocityVerlet`)
//! - zero heap allocation in `step_in_place` (`RK4`)
//! - zero heap allocation in `step_with_contacts_in_place` (`Penalty`)
//! - zero heap allocation in `step_with_contacts_in_place` (`Impulse`)
//! - zero heap allocation in `state_digest_fnv1a` and `kinetic_energy`
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
use math3d::{Plane, Quat, Vec3};
use rigidbody_6dof::{GroundContactParams, IntegratorKind, MassProperties, RigidBodyState};

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

fn sample_body_and_mass() -> (RigidBodyState, MassProperties) {
    let mass = MassProperties::mass_props_from_diagonal(5.0, 2.0, 4.0, 6.0).unwrap();
    let mut body = RigidBodyState::body_new(
        Vec3::new(0.0, 0.45, 0.0),
        Quat::from_axis_angle(Vec3::unit_y(), 0.25).unwrap(),
    )
    .unwrap();
    body.linear_momentum = Vec3::new(1.0, -0.5, 0.2);
    body.angular_momentum = Vec3::new(0.3, 0.7, -0.2);
    body.force = Vec3::new(0.0, -49.05, 0.0);
    (body, mass)
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_step_verlet_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (init, mass) = sample_body_and_mass();
    bench_zero_alloc("step_verlet_zero_alloc", || {
        let mut s = black_box(init);
        s.step_in_place(
            black_box(&mass),
            black_box(1.0 / 60.0),
            IntegratorKind::VelocityVerlet,
        )
        .expect("step should succeed");
        black_box(s);
    })
}

fn check_step_rk4_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (init, mass) = sample_body_and_mass();
    bench_zero_alloc("step_rk4_zero_alloc", || {
        let mut s = black_box(init);
        s.step_in_place(black_box(&mass), black_box(1.0 / 60.0), IntegratorKind::RK4)
            .expect("step should succeed");
        black_box(s);
    })
}

fn check_step_contacts_penalty_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (init, mass) = sample_body_and_mass();
    let ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y()).unwrap();
    let params =
        GroundContactParams::contact_params_penalty(ground, 10_000.0, 400.0, 0.5, 4).unwrap();
    let probes = [
        Vec3::new(-1.0, -0.5, -1.0),
        Vec3::new(1.0, -0.5, -1.0),
        Vec3::new(-1.0, -0.5, 1.0),
        Vec3::new(1.0, -0.5, 1.0),
    ];
    bench_zero_alloc("step_contacts_penalty_zero_alloc", || {
        let mut s = black_box(init);
        s.step_with_contacts_in_place(
            black_box(&mass),
            black_box(1.0 / 60.0),
            IntegratorKind::VelocityVerlet,
            black_box(&params),
            black_box(&probes),
        )
        .expect("contact step should succeed");
        black_box(s);
    })
}

fn check_step_contacts_impulse_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (init, mass) = sample_body_and_mass();
    let ground = Plane::plane_from_point_normal(Vec3::zero(), Vec3::unit_y()).unwrap();
    let params = GroundContactParams::contact_params_impulse(ground, 0.5, 0.4, 0.2, 4).unwrap();
    let probes = [
        Vec3::new(-1.0, -0.5, -1.0),
        Vec3::new(1.0, -0.5, -1.0),
        Vec3::new(-1.0, -0.5, 1.0),
        Vec3::new(1.0, -0.5, 1.0),
    ];
    bench_zero_alloc("step_contacts_impulse_zero_alloc", || {
        let mut s = black_box(init);
        s.step_with_contacts_in_place(
            black_box(&mass),
            black_box(1.0 / 60.0),
            IntegratorKind::VelocityVerlet,
            black_box(&params),
            black_box(&probes),
        )
        .expect("impulse contact step should succeed");
        black_box(s);
    })
}

fn check_diagnostics_and_digest_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (init, mass) = sample_body_and_mass();
    bench_zero_alloc("diagnostics_and_digest_zero_alloc", || {
        let s = black_box(&init);
        let ke = s
            .kinetic_energy(black_box(&mass))
            .expect("ke should succeed");
        let dig = s.state_digest_fnv1a().expect("digest should succeed");
        black_box((ke, dig));
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
        ("module", Json::str("rigidbody-6dof")),
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
    suite.add("bench/step_verlet/zero_alloc", check_step_verlet_zero_alloc);
    suite.add("bench/step_rk4/zero_alloc", check_step_rk4_zero_alloc);
    suite.add(
        "bench/step_contacts_penalty/zero_alloc",
        check_step_contacts_penalty_zero_alloc,
    );
    suite.add(
        "bench/step_contacts_impulse/zero_alloc",
        check_step_contacts_impulse_zero_alloc,
    );
    suite.add(
        "bench/diagnostics_and_digest/zero_alloc",
        check_diagnostics_and_digest_zero_alloc,
    );
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
