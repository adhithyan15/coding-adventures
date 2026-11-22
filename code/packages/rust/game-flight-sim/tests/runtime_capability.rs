//! Runtime capability conformance gate (`VER08b`) for `game-flight-sim`.
//!
//! Pre-allocates compact buffers (level 4 `17x17` heightfield and `32x32`
//! framebuffer `< 1 MiB`) prior to arming `Rings::ALL` seccomp sandboxing so
//! glibc never issues `mremap`, and never calls `GpuRuntime::detect()` or
//! `Runtime::global()`.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use cockpit_hud_paint::{prepare_paint_scene, HudConfig};
use game_flight_sim::{
    decode_black_box_into, encode_black_box_into, render_frame_into, replay_black_box_verify,
    sim_init_on_runway, state_digest_fnv1a, step_in_place, validate_altitude_envelope_m,
    validate_dt_ns, validate_fdr_blob_bytes, validate_fdr_frame_count, validate_fuel_mg,
    validate_render_dim, BlackBoxRecorder, CameraMode, FlightInputFrame, FlightRenderScratch,
    FlightSimConfig,
};
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    BackendPreference, ClipSpace, DepthBufferMut, PostProcessMode, RenderTargetSpec,
};
use terrain_lod::{Heightfield, HeightfieldMut, HeightfieldSpec, LodParams, RunwaySpec};

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    // Pre-allocate compact <= level 4 (17x17) terrain and <= 32x32 (< 1 MiB) buffers before arming.
    let spec = HeightfieldSpec::spec_new(4, 250.0, -2000.0, -2000.0).expect("spec");
    let mut heights = vec![0.0_f64; spec.spec_vertex_count().expect("vcount")];
    let runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 1200.0, 80.0, 20.0, 0.0).expect("rwy");
    {
        let mut hmut = HeightfieldMut::from_heights_mut(spec, &mut heights).expect("hmut");
        hmut.place_runway(&runway).expect("place rwy");
    }

    let cfg = FlightSimConfig::sim_config_default(45, 45);
    let initial = sim_init_on_runway(cfg, &runway).expect("init");
    let header = initial.sim_state_black_box_header().expect("hdr");
    let mut rec = BlackBoxRecorder::recorder_with_capacity(header, 16).expect("rec");
    let mut rec_dec = BlackBoxRecorder::recorder_with_capacity(header, 16).expect("rec_dec");
    let mut fdr_blob = vec![0_u8; 88 + 16 * 152];

    let hud_cfg = HudConfig::hud_config_default(320.0, 240.0).expect("hud");
    let target = RenderTargetSpec {
        width: 32,
        height: 32,
        clear_color: [100, 150, 210, 255],
        clear_depth: 1.0,
        clip_space: ClipSpace::OpenGl,
        post_process: PostProcessMode::Identity,
    };
    let mut fb = PixelContainer::new(32, 32);
    let mut zb = vec![1.0_f64; 32 * 32];
    let v_count = spec.spec_vertex_count().expect("vcount");
    let mut scratch =
        FlightRenderScratch::flight_scratch_with_capacity(256, v_count, 1024, 32, 32, 2048)
            .expect("scratch");
    let mut scene = prepare_paint_scene(&hud_cfg).expect("scene");

    Box::new(move || {
        let mut checks = Vec::with_capacity(6);

        let validators_ok = validate_fdr_frame_count(65_536).is_ok()
            && validate_fdr_blob_bytes(392, 2).is_ok()
            && validate_dt_ns(16_666_667).is_ok()
            && validate_altitude_envelope_m(1500).is_ok()
            && validate_fuel_mg(0).is_ok()
            && validate_render_dim(32).is_ok();
        checks.push(check("warrant contract validators", validators_ok));

        let hf = match Heightfield::from_heights(spec, &heights) {
            Ok(h) => h,
            Err(_) => {
                checks.push(check("heightfield valid", false));
                return checks;
            }
        };

        let mut sim = initial.clone();
        let _ = rec.recorder_clear(header);
        let input = FlightInputFrame {
            dt_ns: cfg.fixed_dt_ns,
            elevator: 0.0,
            aileron: 0.0,
            rudder: 0.0,
            throttle: 0.9,
            flaps: 0.0,
            brake: 0.0,
            gear_down: true,
        };

        let mut step_ok = true;
        for _ in 0..8 {
            if step_in_place(&mut sim, &hf, &runway, &input, Some(&mut rec)).is_err() {
                step_ok = false;
                break;
            }
        }
        checks.push(check("6-dof step_in_place with FDR", step_ok));

        let digest_ok = state_digest_fnv1a(&sim).is_ok();
        checks.push(check("state_digest_fnv1a", digest_ok));

        let fdr_ok = match encode_black_box_into(&rec, &mut fdr_blob) {
            Ok(n) => match decode_black_box_into(&fdr_blob[..n], &mut rec_dec) {
                Ok(cnt) => {
                    cnt == 8 && replay_black_box_verify(&initial, &hf, &runway, &rec_dec).is_ok()
                }
                Err(_) => false,
            },
            Err(_) => false,
        };
        checks.push(check("fdr encode/decode/replay", fdr_ok));

        let lod_ok = LodParams::lod_params_new(sim.body.position, 2.0, 32.0, 4, 256, 1024, None);
        let render_ok = match lod_ok {
            Ok(lod) => render_frame_into(
                &sim,
                &hf,
                &lod,
                &hud_cfg,
                &target,
                BackendPreference::AutoHardwareWithSoftwareFallback,
                CameraMode::Cockpit,
                &mut fb,
                DepthBufferMut::F64(&mut zb),
                &mut scratch,
                Some(&mut scene),
            )
            .is_ok(),
            Err(_) => false,
        };
        checks.push(check("render_frame_into 3D terrain + HUD", render_ok));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/game-flight-sim",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
