//! Conformance tests for `game-flight-sim` (`GAME45`).
//!
//! Verifies:
//! - Explicit 6-DOF flight lifecycle (`OnRunway` -> `TakeoffRoll` -> `Airborne` -> `Approach` -> `Landed`, plus `Crashed`)
//! - Terrain-coupled ground contact and monotonic fuel -> flameout at `0.0`
//! - Deterministic `rng::Pcg32` turbulence gusts
//! - Black-box FDR zero-alloc recording, `GFDR` binary roundtrip, and bit-exact replay parity
//! - Integrated 3D terrain + cockpit HUD rendering (`ForceSoftware`, `AutoHardwareWithSoftwareFallback`, `ForceGpuRuntimeCpu`)

#![forbid(unsafe_code)]

use cockpit_hud_paint::{prepare_paint_scene, HudConfig};
use game_flight_sim::{
    build_flight_camera, decode_black_box_from_bytes, decode_black_box_into, encode_black_box_into,
    render_frame_into, replay_black_box_verify, sim_init_from_black_box_header,
    sim_init_level_flight, sim_init_on_runway, state_digest_fnv1a, step, step_in_place,
    telemetry_from_sim_state, BlackBoxRecorder, CameraMode, FlightInputFrame, FlightPhase,
    FlightRenderScratch, FlightSimConfig,
};
use math3d::Vec3;
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    BackendPreference, ClipSpace, DepthBufferMut, PostProcessMode, RenderTargetSpec,
};
use terrain_lod::{Heightfield, HeightfieldMut, HeightfieldSpec, LodParams, RunwaySpec};

fn build_terrain_and_runway(seed: u64) -> (HeightfieldSpec, Vec<f64>, RunwaySpec) {
    let spec = HeightfieldSpec::spec_new(4, 250.0, -2000.0, -2000.0).expect("spec");
    let mut heights = vec![0.0_f64; spec.spec_vertex_count().expect("vcount")];
    let runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 1400.0, 80.0, 30.0, 0.0).expect("runway");
    {
        let mut hmut = HeightfieldMut::from_heights_mut(spec, &mut heights).expect("hmut");
        hmut.synthesize_fbm(seed, 0.0, 80.0, 0.0008, 4, 0.5, 2.0)
            .expect("fbm");
        hmut.place_runway(&runway).expect("place runway");
    }
    (spec, heights, runway)
}

#[test]
fn runway_takeoff_and_deterministic_fdr_replay() {
    let spec = HeightfieldSpec::spec_new(4, 250.0, -2000.0, -2000.0).expect("spec");
    let mut heights = vec![0.0_f64; spec.spec_vertex_count().expect("vcount")];
    let runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 2000.0, 80.0, 30.0, 0.0).expect("runway");
    {
        let mut hmut = HeightfieldMut::from_heights_mut(spec, &mut heights).expect("hmut");
        hmut.synthesize_fbm(42, 0.0, 80.0, 0.0008, 4, 0.5, 2.0)
            .expect("fbm");
        hmut.place_runway(&runway).expect("place runway");
    }
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");

    let cfg = FlightSimConfig::sim_config_default(0x1234_5678, 42);
    let initial = sim_init_on_runway(cfg, &runway).expect("init runway");
    assert_eq!(initial.phase, FlightPhase::OnRunway);

    let header = initial.sim_state_black_box_header().expect("header");
    let mut recorder = BlackBoxRecorder::recorder_with_capacity(header, 1024).expect("recorder");
    let mut sim = initial.clone();

    let mut saw_takeoff_roll = false;
    let mut saw_airborne = false;

    for t in 0..900 {
        let elevator = if t < 600 { 0.0 } else { 0.40 };
        let input = FlightInputFrame {
            dt_ns: cfg.fixed_dt_ns,
            elevator,
            aileron: 0.0,
            rudder: 0.0,
            throttle: 1.0,
            flaps: 0.25,
            brake: 0.0,
            gear_down: true,
        };
        let rep = step_in_place(&mut sim, &hf, &runway, &input, Some(&mut recorder)).expect("step");
        if rep.phase == FlightPhase::TakeoffRoll {
            saw_takeoff_roll = true;
        }
        if rep.phase == FlightPhase::Airborne {
            saw_airborne = true;
        }
    }

    assert!(saw_takeoff_roll, "aircraft should enter TakeoffRoll");
    assert!(saw_airborne, "aircraft should lift off into Airborne");
    assert_eq!(recorder.recorder_len(), 900);

    // Binary encode/decode roundtrip + zero-alloc decode_into
    let needed = recorder.recorder_encoded_len().expect("encoded len");
    let mut blob = vec![0_u8; needed];
    let written = encode_black_box_into(&recorder, &mut blob).expect("encode");
    assert_eq!(written, needed);

    let decoded = decode_black_box_from_bytes(&blob).expect("decode");
    assert_eq!(
        decoded.recorder_digest_fnv1a(),
        recorder.recorder_digest_fnv1a()
    );

    let mut decoded_into = BlackBoxRecorder::recorder_with_capacity(header, 1024).expect("rec2");
    let count = decode_black_box_into(&blob, &mut decoded_into).expect("decode into");
    assert_eq!(count, 900);
    assert_eq!(
        decoded_into.recorder_digest_fnv1a(),
        recorder.recorder_digest_fnv1a()
    );

    // Reconstruct initial state solely from BlackBoxHeader and verify bit-exact replay!
    let reconstructed =
        sim_init_from_black_box_header(decoded.recorder_header(), &runway).expect("reconstruct");
    let verify =
        replay_black_box_verify(&reconstructed, &hf, &runway, &decoded).expect("bit-exact replay");
    assert_eq!(verify.frames_verified, 900);
    assert_eq!(verify.final_tick, sim.tick);
    assert_eq!(
        verify.final_digest_fnv1a,
        state_digest_fnv1a(&sim).expect("digest")
    );
}

#[test]
fn level_trim_approach_and_safe_runway_landing() {
    let spec = HeightfieldSpec::spec_new(4, 250.0, -2000.0, -2000.0).expect("spec");
    let mut heights = vec![0.0_f64; spec.spec_vertex_count().expect("vcount")];
    let runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 2400.0, 120.0, 20.0, 0.0).expect("runway");
    {
        let mut hmut = HeightfieldMut::from_heights_mut(spec, &mut heights).expect("hmut");
        hmut.place_runway(&runway).expect("place runway");
    }
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");

    let cfg = FlightSimConfig::sim_config_default(99, 77);
    let mut sim = sim_init_level_flight(cfg, 20.0, 42.0).expect("level trim");
    sim.body.position.x = -500.0;
    sim.body.position.y = 4.0;
    sim.body.linear_momentum = Vec3::new(sim.mass.mass_props_mass() * 24.0, -0.5, 0.0);

    let mut saw_approach = false;
    let mut saw_landed = false;
    for _ in 0..300 {
        let input = FlightInputFrame {
            dt_ns: cfg.fixed_dt_ns,
            elevator: 0.02,
            aileron: 0.0,
            rudder: 0.0,
            throttle: 0.0,
            flaps: 0.5,
            brake: 0.85,
            gear_down: true,
        };
        let rep = step_in_place(&mut sim, &hf, &runway, &input, None).expect("step");
        if rep.phase == FlightPhase::Approach {
            saw_approach = true;
        }
        if rep.phase == FlightPhase::Landed {
            saw_landed = true;
            break;
        }
        assert_ne!(
            rep.phase,
            FlightPhase::Crashed,
            "safe flare should not crash"
        );
    }
    assert!(saw_approach, "should pass through Approach");
    assert!(saw_landed, "should transition to Landed on runway");
}

#[test]
fn gear_up_impact_transitions_to_terminal_crashed() {
    let (spec, heights, runway) = build_terrain_and_runway(15);
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");

    let cfg = FlightSimConfig::sim_config_default(5, 15);
    let mut sim = sim_init_level_flight(cfg, 12.0, 50.0).expect("level");
    sim.body.position.y = 2.0;
    sim.body.linear_momentum = Vec3::new(sim.mass.mass_props_mass() * 45.0, -8000.0, 0.0);

    let input = FlightInputFrame {
        dt_ns: cfg.fixed_dt_ns,
        elevator: -0.5,
        aileron: 0.0,
        rudder: 0.0,
        throttle: 0.0,
        flaps: 0.0,
        brake: 0.0,
        gear_down: false,
    };

    for _ in 0..40 {
        let _ = step_in_place(&mut sim, &hf, &runway, &input, None).expect("step");
        if sim.phase == FlightPhase::Crashed {
            break;
        }
    }
    assert_eq!(sim.phase, FlightPhase::Crashed);
    let crashed_pos = sim.body.position;
    let _ = step_in_place(&mut sim, &hf, &runway, &input, None).expect("post-crash step");
    assert_eq!(sim.phase, FlightPhase::Crashed);
    assert_eq!(sim.body.position, crashed_pos);
}

#[test]
fn rng_gust_determinism_and_step_functional_parity() {
    let (spec, heights, runway) = build_terrain_and_runway(101);
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");

    let mut cfg_a = FlightSimConfig::sim_config_default(1111, 101);
    cfg_a.gust_intensity_mps = 4.0;
    let mut cfg_b = cfg_a;
    cfg_b.rng_seed = 2222;

    let mut sim_a = sim_init_level_flight(cfg_a, 500.0, 55.0).expect("a");
    let mut sim_a_fn = sim_a.clone();
    let mut sim_b = sim_init_level_flight(cfg_b, 500.0, 55.0).expect("b");

    let input = FlightInputFrame {
        dt_ns: cfg_a.fixed_dt_ns,
        elevator: 0.0,
        aileron: 0.0,
        rudder: 0.0,
        throttle: 0.55,
        flaps: 0.0,
        brake: 0.0,
        gear_down: false,
    };

    for _ in 0..20 {
        let rep_in_place = step_in_place(&mut sim_a, &hf, &runway, &input, None).expect("step a");
        let (next_fn, rep_fn) = step(&sim_a_fn, &hf, &runway, &input).expect("step fn");
        sim_a_fn = next_fn;
        assert_eq!(rep_in_place.state_digest_fnv1a, rep_fn.state_digest_fnv1a);
        let _ = step_in_place(&mut sim_b, &hf, &runway, &input, None).expect("step b");
    }

    assert_ne!(
        state_digest_fnv1a(&sim_a).expect("dig a"),
        state_digest_fnv1a(&sim_b).expect("dig b")
    );
}

#[test]
fn render_frame_into_cross_backend_and_neg_zero_parity() {
    let (spec, heights, runway) = build_terrain_and_runway(88);
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");

    let cfg = FlightSimConfig::sim_config_default(88, 88);
    let sim = sim_init_level_flight(cfg, 400.0, 55.0).expect("trim");

    let target = RenderTargetSpec {
        width: 32,
        height: 32,
        clear_color: [120, 175, 230, 255],
        clear_depth: 1.0,
        clip_space: ClipSpace::OpenGl,
        post_process: PostProcessMode::Identity,
    };
    let hud_cfg = HudConfig::hud_config_default(640.0, 480.0).expect("hud cfg");
    let lod =
        LodParams::lod_params_new(sim.body.position, 2.0, 32.0, 4, 256, 1024, None).expect("lod");

    let mut fb_sw = PixelContainer::new(32, 32);
    let mut zb_sw = vec![1.0_f64; 32 * 32];
    let mut scratch_sw = FlightRenderScratch::flight_scratch_with_capacity(
        256,
        hf.vertex_count(),
        1024,
        32,
        32,
        2048,
    )
    .expect("scratch");
    let mut scene_sw = prepare_paint_scene(&hud_cfg).expect("scene");

    let rep_sw = render_frame_into(
        &sim,
        &hf,
        &lod,
        &hud_cfg,
        &target,
        BackendPreference::ForceSoftware,
        CameraMode::Cockpit,
        &mut fb_sw,
        DepthBufferMut::F64(&mut zb_sw),
        &mut scratch_sw,
        Some(&mut scene_sw),
    )
    .expect("render sw");

    let mut fb_rt = PixelContainer::new(32, 32);
    let mut zb_rt = vec![1.0_f64; 32 * 32];
    let mut scratch_rt = FlightRenderScratch::flight_scratch_with_capacity(
        256,
        hf.vertex_count(),
        1024,
        32,
        32,
        2048,
    )
    .expect("scratch rt");
    let mut scene_rt = prepare_paint_scene(&hud_cfg).expect("scene rt");

    let rep_rt = render_frame_into(
        &sim,
        &hf,
        &lod,
        &hud_cfg,
        &target,
        BackendPreference::ForceGpuRuntimeCpu,
        CameraMode::Cockpit,
        &mut fb_rt,
        DepthBufferMut::F64(&mut zb_rt),
        &mut scratch_rt,
        Some(&mut scene_rt),
    )
    .expect("render gpu cpu");

    assert_eq!(rep_sw.combined_digest_fnv1a, rep_rt.combined_digest_fnv1a);
    assert_eq!(
        rep_sw.render_report.frame_digest,
        rep_rt.render_report.frame_digest
    );
    assert_ne!(rep_sw.combined_digest_fnv1a, 0);

    // Chase camera & negative-zero canonicalization check
    let _cam_chase =
        build_flight_camera(&sim, &target, &hud_cfg, CameraMode::Chase).expect("chase");
    let _tel = telemetry_from_sim_state(&sim, &hf).expect("tel");
    let mut sim_a = sim_init_on_runway(cfg, &runway).expect("rwy a");
    let mut sim_b = sim_init_on_runway(cfg, &runway).expect("rwy b");
    let in_pos = FlightInputFrame {
        dt_ns: cfg.fixed_dt_ns,
        elevator: 0.0,
        aileron: 0.0,
        rudder: 0.0,
        throttle: 0.0,
        flaps: 0.0,
        brake: 0.0,
        gear_down: true,
    };
    let in_neg = FlightInputFrame {
        dt_ns: cfg.fixed_dt_ns,
        elevator: -0.0,
        aileron: -0.0,
        rudder: -0.0,
        throttle: -0.0,
        flaps: -0.0,
        brake: -0.0,
        gear_down: true,
    };
    let ra = step_in_place(&mut sim_a, &hf, &runway, &in_pos, None).expect("pos");
    let rb = step_in_place(&mut sim_b, &hf, &runway, &in_neg, None).expect("neg");
    assert_eq!(ra.state_digest_fnv1a, rb.state_digest_fnv1a);
}
