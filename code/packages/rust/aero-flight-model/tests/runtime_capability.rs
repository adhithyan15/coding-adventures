//! # VER08b runtime capability conformance for `aero-flight-model`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use aero_flight_model::{
    compute_air_data, isa_atmosphere, AircraftModel, ControlInputs, FlightEnvironment,
};
use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use math3d::{Quat, Vec3};
use rigidbody_6dof::{IntegratorKind, MassProperties, RigidBodyState};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        let atm_sl = isa_atmosphere(0.0);
        let atm_trop = isa_atmosphere(15_000.0);
        checks.push(check(
            "isa atmosphere evaluation",
            atm_sl.is_ok() && atm_trop.is_ok(),
        ));

        let model_res = AircraftModel::aircraft_preset_trainer();
        checks.push(check("aircraft preset construction", model_res.is_ok()));

        let mass_res = MassProperties::mass_props_from_diagonal(1000.0, 1280.0, 2660.0, 1820.0);
        checks.push(check("mass properties construction", mass_res.is_ok()));

        if let (Ok(atm), Ok(model), Ok(mass)) = (atm_sl, model_res, mass_res) {
            let env = FlightEnvironment::env_isa_calm();
            let trim_res = model.aircraft_solve_level_trim(&mass, &env, 1500.0, 55.0);
            checks.push(check("level flight trim solver", trim_res.is_ok()));

            if let Ok((alpha_trim, controls)) = trim_res {
                let q = Quat::from_axis_angle(Vec3::unit_z(), alpha_trim).unwrap();
                let mut body = RigidBodyState::body_new(Vec3::new(0.0, 1500.0, 0.0), q).unwrap();
                body.linear_momentum = Vec3::new(55_000.0, 0.0, 0.0);

                let ad_ok =
                    compute_air_data(Vec3::new(55.0, 0.0, 0.0), Vec3::zero(), q, &atm).is_ok();
                checks.push(check("air data computation", ad_ok));

                let eval_ok = model
                    .aircraft_evaluate(&body, &mass, &env, &controls)
                    .and_then(|out| out.aero_eval_digest_fnv1a())
                    .is_ok();
                checks.push(check("aero evaluation and canonical digest", eval_ok));

                let step_ok = model
                    .step_aircraft_in_place(
                        &mut body,
                        &mass,
                        &env,
                        &controls,
                        1.0 / 60.0,
                        IntegratorKind::RK4,
                    )
                    .is_ok()
                    && model
                        .step_aircraft_in_place(
                            &mut body,
                            &mass,
                            &env,
                            &ControlInputs::controls_neutral(),
                            1.0 / 60.0,
                            IntegratorKind::VelocityVerlet,
                        )
                        .is_ok();
                checks.push(check("coupled 6-dof flight steps", step_ok));
            }
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/aero-flight-model",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
