//! # Microbenchmark Suite and Zero-Allocation Verification (VER07) for `aero-flight-model`
//!
//! Validates, on the real heap via `coding_adventures_test_bootstrap`'s
//! `CountingAllocator`:
//! - zero heap allocation in `isa_atmosphere`
//! - zero heap allocation in `AeroTable::aero_table_eval`
//! - zero heap allocation in `compute_air_data`
//! - zero heap allocation in `AircraftModel::compute_world_wrench`
//! - zero heap allocation in `AircraftModel::step_aircraft_in_place` (`VelocityVerlet` & `RK4`)
//! - zero heap allocation in `AeroEvalOutput::aero_eval_digest_fnv1a`
//! - sabotage detection: an intentional allocation is caught and rejected.

#![deny(unsafe_code)]

use std::hint::black_box;
use std::time::Duration;

use aero_flight_model::{
    compute_air_data, isa_atmosphere, AircraftModel, ControlInputs, FlightEnvironment,
};
use coding_adventures_test_bootstrap::bench::{
    AllocCounter, BenchmarkConfig, BenchmarkHarness, BenchmarkResult, Budget, BudgetVerdict,
};
use coding_adventures_test_bootstrap::counting_alloc;
use coding_adventures_test_bootstrap::manifest::{self, Json};
use coding_adventures_test_bootstrap::run::{self, Ctx, Outcome, Suite};
use coding_adventures_test_bootstrap::Verdict;
use math3d::{Quat, Vec3};
use rigidbody_6dof::{IntegratorKind, MassProperties, RigidBodyState};

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

fn sample_flight_setup() -> (
    AircraftModel,
    MassProperties,
    RigidBodyState,
    FlightEnvironment,
    ControlInputs,
) {
    let model = AircraftModel::aircraft_preset_trainer().unwrap();
    let mass = MassProperties::mass_props_from_diagonal(1000.0, 1280.0, 2660.0, 1820.0).unwrap();
    let env = FlightEnvironment::env_isa_calm();
    let (alpha_trim, controls) = model
        .aircraft_solve_level_trim(&mass, &env, 1500.0, 55.0)
        .unwrap();
    let mut body = RigidBodyState::body_new(
        Vec3::new(0.0, 1500.0, 0.0),
        Quat::from_axis_angle(Vec3::unit_z(), alpha_trim).unwrap(),
    )
    .unwrap();
    body.linear_momentum = Vec3::new(55_000.0, 0.0, 0.0);
    (model, mass, body, env, controls)
}

fn check_counting_allocator_is_live(_ctx: &Ctx) -> Verdict {
    if !counting_alloc::is_installed() {
        return Verdict::fail(FID_BENCH_FAIL, "counting allocator is not installed");
    }
    Verdict::Pass
}

fn check_isa_atmosphere_zero_alloc(_ctx: &Ctx) -> Verdict {
    bench_zero_alloc("isa_atmosphere_zero_alloc", || {
        let atm = isa_atmosphere(black_box(11_500.0)).expect("isa should succeed");
        black_box(atm);
    })
}

fn check_polar_eval_zero_alloc(_ctx: &Ctx) -> Verdict {
    let model = AircraftModel::aircraft_preset_trainer().unwrap();
    let cl_tbl = *model.aircraft_cl_table();
    bench_zero_alloc("polar_eval_zero_alloc", || {
        let cl = cl_tbl
            .aero_table_eval(black_box(0.14))
            .expect("polar eval should succeed");
        black_box(cl);
    })
}

fn check_air_data_zero_alloc(_ctx: &Ctx) -> Verdict {
    let atm = isa_atmosphere(1500.0).unwrap();
    let q = Quat::from_axis_angle(Vec3::unit_z(), 0.08).unwrap();
    let v = Vec3::new(55.0, 0.5, 1.2);
    let w = Vec3::new(-3.0, 0.0, 2.0);
    bench_zero_alloc("air_data_zero_alloc", || {
        let ad = compute_air_data(black_box(v), black_box(w), black_box(q), black_box(&atm))
            .expect("air data should succeed");
        black_box(ad);
    })
}

fn check_compute_world_wrench_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (model, mass, body, env, controls) = sample_flight_setup();
    bench_zero_alloc("compute_world_wrench_zero_alloc", || {
        let w = model
            .compute_world_wrench(
                black_box(&body),
                black_box(&mass),
                black_box(&env),
                black_box(&controls),
            )
            .expect("wrench eval should succeed");
        black_box(w);
    })
}

fn check_step_aircraft_verlet_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (model, mass, init, env, controls) = sample_flight_setup();
    bench_zero_alloc("step_aircraft_verlet_zero_alloc", || {
        let mut s = black_box(init);
        model
            .step_aircraft_in_place(
                &mut s,
                black_box(&mass),
                black_box(&env),
                black_box(&controls),
                black_box(1.0 / 60.0),
                IntegratorKind::VelocityVerlet,
            )
            .expect("verlet step should succeed");
        black_box(s);
    })
}

fn check_step_aircraft_rk4_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (model, mass, init, env, controls) = sample_flight_setup();
    bench_zero_alloc("step_aircraft_rk4_zero_alloc", || {
        let mut s = black_box(init);
        model
            .step_aircraft_in_place(
                &mut s,
                black_box(&mass),
                black_box(&env),
                black_box(&controls),
                black_box(1.0 / 60.0),
                IntegratorKind::RK4,
            )
            .expect("rk4 step should succeed");
        black_box(s);
    })
}

fn check_eval_digest_zero_alloc(_ctx: &Ctx) -> Verdict {
    let (model, mass, body, env, controls) = sample_flight_setup();
    let out = model
        .aircraft_evaluate(&body, &mass, &env, &controls)
        .unwrap();
    bench_zero_alloc("eval_digest_zero_alloc", || {
        let dig = black_box(&out)
            .aero_eval_digest_fnv1a()
            .expect("digest should succeed");
        black_box(dig);
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
        ("module", Json::str("aero-flight-model")),
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
        "bench/isa_atmosphere/zero_alloc",
        check_isa_atmosphere_zero_alloc,
    );
    suite.add("bench/polar_eval/zero_alloc", check_polar_eval_zero_alloc);
    suite.add("bench/air_data/zero_alloc", check_air_data_zero_alloc);
    suite.add(
        "bench/compute_world_wrench/zero_alloc",
        check_compute_world_wrench_zero_alloc,
    );
    suite.add(
        "bench/step_aircraft_verlet/zero_alloc",
        check_step_aircraft_verlet_zero_alloc,
    );
    suite.add(
        "bench/step_aircraft_rk4/zero_alloc",
        check_step_aircraft_rk4_zero_alloc,
    );
    suite.add("bench/eval_digest/zero_alloc", check_eval_digest_zero_alloc);
    suite.add(
        "bench/sabotage/allocation_detected",
        check_sabotage_allocation_detected,
    );

    std::process::exit(run::main_recorded(&suite, write_bench_manifest));
}
