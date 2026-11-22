//! Mandatory `#429` Hostile-Input & Atomic Rollback Suite for `game-flight-sim` (`GAME45`).
//!
//! Verifies zero panics, explicit typed `Err` rejections, and byte-for-byte
//! atomic buffer/state preservation across:
//! - qNaN, sNaN, `±Inf`, `-0.0`, subnormals, `±f64::MAX`
//! - Out-of-range control surfaces, `dt_ns = 0`/`u64::MAX`, clock overflow
//! - Full `BlackBoxRecorder` rollback, hardened `GFDR` binary decoder
//! - Tampered FDR replay verification (`ReplayMismatch`)
//! - `BackendPreference::RequireHardware` rejection
//! - `render_frame_into` byte-for-byte preservation across `framebuffer`, `depth`, `scratch`, `PaintScene`
//! - Seeded 2,000-case hostile fuzz campaign over `step_in_place`, `render_frame_into`, and `decode_black_box_from_bytes`

#![forbid(unsafe_code)]

use cockpit_hud_paint::{paint_scene_digest_fnv1a, prepare_paint_scene, HudConfig};
use game_flight_sim::{
    decode_black_box_from_bytes, decode_black_box_into, encode_black_box_into, render_frame_into,
    replay_black_box_verify, sim_init_level_flight, sim_init_on_runway, state_digest_fnv1a,
    step_in_place, BlackBoxRecorder, CameraMode, FlightInputFrame, FlightRenderScratch,
    FlightSimConfig, FlightSimError, BLACK_BOX_HEADER_BYTES, BLACK_BOX_TRAILER_BYTES,
    MAX_BLACK_BOX_FRAMES, MAX_STEP_DT_NS,
};
use math3d::Vec3;
use pixel_container::PixelContainer;
use render3d_gpu_bridge::{
    BackendPreference, ClipSpace, DepthBufferMut, PostProcessMode, RenderBackendKind,
    RenderTargetSpec,
};
use terrain_lod::{Heightfield, HeightfieldMut, HeightfieldSpec, LodParams, RunwaySpec};

const QNAN: f64 = f64::from_bits(0x7ff8_0000_0000_0001);
const SNAN: f64 = f64::from_bits(0x7ff0_0000_0000_0001);
const SUBNORMAL: f64 = f64::from_bits(1);

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn next_f64_hostile(&mut self) -> f64 {
        match self.next_u64() % 12 {
            0 => QNAN,
            1 => SNAN,
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => -0.0,
            5 => SUBNORMAL,
            6 => -SUBNORMAL,
            7 => f64::MAX,
            8 => -f64::MAX,
            9 => 1.0e6,
            10 => -1.0e6,
            _ => {
                let raw = (self.next_u64() & 0xffff) as f64 / 65536.0;
                raw * 4.0 - 2.0
            }
        }
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn test_terrain() -> (HeightfieldSpec, Vec<f64>, RunwaySpec) {
    let spec = HeightfieldSpec::spec_new(4, 250.0, -2000.0, -2000.0).expect("spec");
    let mut heights = vec![0.0_f64; spec.spec_vertex_count().expect("vcount")];
    let runway = RunwaySpec::runway_new(0.0, 0.0, 0.0, 1200.0, 80.0, 20.0, 0.0).expect("rwy");
    {
        let mut hmut = HeightfieldMut::from_heights_mut(spec, &mut heights).expect("hmut");
        hmut.place_runway(&runway).expect("place rwy");
    }
    (spec, heights, runway)
}

fn valid_input(dt_ns: u64) -> FlightInputFrame {
    FlightInputFrame {
        dt_ns,
        elevator: 0.0,
        aileron: 0.0,
        rudder: 0.0,
        throttle: 0.5,
        flaps: 0.0,
        brake: 0.0,
        gear_down: true,
    }
}

#[test]
fn step_in_place_rejects_hostile_scalars_and_preserves_state_and_recorder() {
    let (spec, heights, runway) = test_terrain();
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");
    let cfg = FlightSimConfig::sim_config_default(42, 42);
    let mut sim = sim_init_on_runway(cfg, &runway).expect("init");
    let header = sim.sim_state_black_box_header().expect("hdr");
    let mut rec = BlackBoxRecorder::recorder_with_capacity(header, 4).expect("rec");

    // Advance 1 valid step first.
    step_in_place(
        &mut sim,
        &hf,
        &runway,
        &valid_input(cfg.fixed_dt_ns),
        Some(&mut rec),
    )
    .expect("valid step");

    let snap_digest = state_digest_fnv1a(&sim).expect("digest");
    let snap_rec_digest = rec.recorder_digest_fnv1a().expect("rec digest");
    let snap_tick = sim.tick;
    let snap_rng_steps = sim.rng_steps;
    let snap_fuel = sim.fuel_kg.to_bits();

    let hostile_floats = [
        QNAN,
        SNAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        -f64::MAX,
        1.01,
        -1.01,
    ];

    for bad in hostile_floats {
        let mut inputs = [
            valid_input(cfg.fixed_dt_ns),
            valid_input(cfg.fixed_dt_ns),
            valid_input(cfg.fixed_dt_ns),
            valid_input(cfg.fixed_dt_ns),
            valid_input(cfg.fixed_dt_ns),
            valid_input(cfg.fixed_dt_ns),
        ];
        inputs[0].elevator = bad;
        inputs[1].aileron = bad;
        inputs[2].rudder = bad;
        inputs[3].throttle = bad;
        inputs[4].flaps = bad;
        inputs[5].brake = bad;

        for bad_in in inputs {
            let err = step_in_place(&mut sim, &hf, &runway, &bad_in, Some(&mut rec));
            assert!(err.is_err(), "expected error for bad float {bad:?}");
            assert_eq!(state_digest_fnv1a(&sim).expect("dig"), snap_digest);
            assert_eq!(rec.recorder_digest_fnv1a().expect("rdig"), snap_rec_digest);
            assert_eq!(sim.tick, snap_tick);
            assert_eq!(sim.rng_steps, snap_rng_steps);
            assert_eq!(sim.fuel_kg.to_bits(), snap_fuel);
        }
    }

    // Subnormal controls are finite in [-1, 1] / [0, 1] and must not panic.
    let mut sub_in = valid_input(cfg.fixed_dt_ns);
    sub_in.elevator = SUBNORMAL;
    sub_in.throttle = SUBNORMAL;
    sub_in.flaps = -0.0;
    let _ = step_in_place(&mut sim, &hf, &runway, &sub_in, Some(&mut rec)).expect("subnormal ok");

    // Hostile dt_ns = 0, MAX_STEP_DT_NS + 1, u64::MAX
    let snap2 = state_digest_fnv1a(&sim).expect("dig2");
    let rsnap2 = rec.recorder_digest_fnv1a().expect("rdig2");
    for bad_dt in [0_u64, MAX_STEP_DT_NS + 1, u64::MAX] {
        let err = step_in_place(&mut sim, &hf, &runway, &valid_input(bad_dt), Some(&mut rec));
        assert_eq!(err, Err(FlightSimError::InvalidClockDelta));
        assert_eq!(state_digest_fnv1a(&sim).expect("dig"), snap2);
        assert_eq!(rec.recorder_digest_fnv1a().expect("rdig"), rsnap2);
    }

    // Clock overflow preservation
    let mut overflow_sim = sim.clone();
    overflow_sim.clock.elapsed_wall_ns = u64::MAX - 1;
    let ov_err = step_in_place(
        &mut overflow_sim,
        &hf,
        &runway,
        &valid_input(cfg.fixed_dt_ns),
        None,
    );
    assert!(ov_err.is_err());

    // Corrupted FlightSimState scalars rejected atomically
    let mut bad_state = sim.clone();
    bad_state.fuel_kg = -0.01;
    assert_eq!(
        step_in_place(
            &mut bad_state,
            &hf,
            &runway,
            &valid_input(cfg.fixed_dt_ns),
            None
        ),
        Err(FlightSimError::ValueOutOfBounds)
    );
    bad_state.fuel_kg = QNAN;
    assert_eq!(
        step_in_place(
            &mut bad_state,
            &hf,
            &runway,
            &valid_input(cfg.fixed_dt_ns),
            None
        ),
        Err(FlightSimError::NonFiniteInput)
    );
    let mut bad_pos = sim.clone();
    bad_pos.body.position.x = SNAN;
    assert_eq!(
        step_in_place(
            &mut bad_pos,
            &hf,
            &runway,
            &valid_input(cfg.fixed_dt_ns),
            None
        ),
        Err(FlightSimError::NonFiniteInput)
    );

    // Full BlackBoxRecorder atomic rejection leaves BOTH state and recorder untouched
    while !rec.recorder_is_full() {
        step_in_place(
            &mut sim,
            &hf,
            &runway,
            &valid_input(cfg.fixed_dt_ns),
            Some(&mut rec),
        )
        .expect("fill");
    }
    let full_state_dig = state_digest_fnv1a(&sim).expect("full sdig");
    let full_rec_dig = rec.recorder_digest_fnv1a().expect("full rdig");
    let full_err = step_in_place(
        &mut sim,
        &hf,
        &runway,
        &valid_input(cfg.fixed_dt_ns),
        Some(&mut rec),
    );
    assert_eq!(full_err, Err(FlightSimError::BlackBoxFull));
    assert_eq!(state_digest_fnv1a(&sim).expect("sdig"), full_state_dig);
    assert_eq!(rec.recorder_digest_fnv1a().expect("rdig"), full_rec_dig);
}

#[test]
fn fdr_codec_rejects_corrupted_truncated_oversized_and_nan_injected_blobs() {
    let (spec, heights, runway) = test_terrain();
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");
    let cfg = FlightSimConfig::sim_config_default(7, 7);
    let mut sim = sim_init_on_runway(cfg, &runway).expect("init");
    let header = sim.sim_state_black_box_header().expect("hdr");
    let mut rec = BlackBoxRecorder::recorder_with_capacity(header, 4).expect("rec");

    for _ in 0..3 {
        step_in_place(
            &mut sim,
            &hf,
            &runway,
            &valid_input(cfg.fixed_dt_ns),
            Some(&mut rec),
        )
        .expect("step");
    }

    let len = rec.recorder_encoded_len().expect("len");
    let mut blob = vec![0_u8; len];
    let _ = encode_black_box_into(&rec, &mut blob).expect("encode");

    // Undersized encode buffer remains byte-for-byte untouched!
    let mut tiny = vec![0xAA_u8; len - 1];
    assert_eq!(
        encode_black_box_into(&rec, &mut tiny),
        Err(FlightSimError::BufferTooSmall)
    );
    assert!(tiny.iter().all(|&b| b == 0xAA));

    // Truncated slices
    for cut in [0, 4, 16, BLACK_BOX_HEADER_BYTES, len - 1] {
        assert_eq!(
            decode_black_box_from_bytes(&blob[..cut]),
            Err(FlightSimError::BlackBoxCorrupt)
        );
    }

    // Trailing garbage byte
    let mut over = blob.clone();
    over.push(0);
    assert_eq!(
        decode_black_box_from_bytes(&over),
        Err(FlightSimError::BlackBoxCorrupt)
    );

    // Bad magic & version
    let mut bad_magic = blob.clone();
    bad_magic[0] = b'X';
    assert_eq!(
        decode_black_box_from_bytes(&bad_magic),
        Err(FlightSimError::BlackBoxCorrupt)
    );

    let mut bad_ver = blob.clone();
    bad_ver[4..8].copy_from_slice(&2_u32.to_le_bytes());
    assert_eq!(
        decode_black_box_from_bytes(&bad_ver),
        Err(FlightSimError::BlackBoxCorrupt)
    );

    // Oversized frame_count in header with recomputed checksum
    let mut oversized_cnt = blob.clone();
    let bad_count = (MAX_BLACK_BOX_FRAMES as u32) + 1;
    oversized_cnt[76..80].copy_from_slice(&bad_count.to_le_bytes());
    let chk = fnv1a(&oversized_cnt[..len - BLACK_BOX_TRAILER_BYTES]);
    oversized_cnt[len - BLACK_BOX_TRAILER_BYTES..].copy_from_slice(&chk.to_le_bytes());
    assert_eq!(
        decode_black_box_from_bytes(&oversized_cnt),
        Err(FlightSimError::BlackBoxCorrupt)
    );

    // Bad checksum (flip bit in payload)
    let mut bad_chk = blob.clone();
    bad_chk[90] ^= 0x01;
    assert_eq!(
        decode_black_box_from_bytes(&bad_chk),
        Err(FlightSimError::BlackBoxCorrupt)
    );

    // NaN-injected scalar WITH recomputed valid FNV-1a checksum!
    let mut nan_blob = blob.clone();
    nan_blob[BLACK_BOX_HEADER_BYTES + 16..BLACK_BOX_HEADER_BYTES + 24]
        .copy_from_slice(&QNAN.to_bits().to_le_bytes());
    let nan_chk = fnv1a(&nan_blob[..len - BLACK_BOX_TRAILER_BYTES]);
    nan_blob[len - BLACK_BOX_TRAILER_BYTES..].copy_from_slice(&nan_chk.to_le_bytes());
    assert_eq!(
        decode_black_box_from_bytes(&nan_blob),
        Err(FlightSimError::BlackBoxCorrupt)
    );

    // Invalid FlightPhase enum tag WITH valid checksum
    let mut bad_phase = blob.clone();
    bad_phase[BLACK_BOX_HEADER_BYTES + 65] = 99;
    let phase_chk = fnv1a(&bad_phase[..len - BLACK_BOX_TRAILER_BYTES]);
    bad_phase[len - BLACK_BOX_TRAILER_BYTES..].copy_from_slice(&phase_chk.to_le_bytes());
    assert_eq!(
        decode_black_box_from_bytes(&bad_phase),
        Err(FlightSimError::BlackBoxCorrupt)
    );

    // decode_black_box_into preserves existing recorder on Err / BlackBoxFull
    let mut small_rec = BlackBoxRecorder::recorder_with_capacity(header, 1).expect("small");
    step_in_place(
        &mut sim,
        &hf,
        &runway,
        &valid_input(cfg.fixed_dt_ns),
        Some(&mut small_rec),
    )
    .expect("1 frame");
    let small_dig = small_rec.recorder_digest_fnv1a().expect("sdig");
    assert_eq!(
        decode_black_box_into(&blob, &mut small_rec),
        Err(FlightSimError::BlackBoxFull)
    );
    assert_eq!(small_rec.recorder_digest_fnv1a().expect("sdig2"), small_dig);
}

#[test]
fn tampered_replay_is_rejected() {
    let (spec, heights, runway) = test_terrain();
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");
    let cfg = FlightSimConfig::sim_config_default(19, 19);
    let init = sim_init_on_runway(cfg, &runway).expect("init");
    let header = init.sim_state_black_box_header().expect("hdr");
    let mut rec = BlackBoxRecorder::recorder_with_capacity(header, 8).expect("rec");
    let mut sim = init.clone();
    for _ in 0..4 {
        step_in_place(
            &mut sim,
            &hf,
            &runway,
            &valid_input(cfg.fixed_dt_ns),
            Some(&mut rec),
        )
        .expect("step");
    }

    // Tamper initial state
    let mut wrong_init = init.clone();
    wrong_init.fuel_kg += 1.0;
    assert_eq!(
        replay_black_box_verify(&wrong_init, &hf, &runway, &rec),
        Err(FlightSimError::ReplayMismatch)
    );

    // Tamper one recorded frame's digest in serialized bytes and re-sign checksum
    let len = rec.recorder_encoded_len().expect("len");
    let mut blob = vec![0_u8; len];
    let _ = encode_black_box_into(&rec, &mut blob).expect("enc");
    let digest_offset = BLACK_BOX_HEADER_BYTES + 144;
    blob[digest_offset] ^= 0xff;
    let chk = fnv1a(&blob[..len - BLACK_BOX_TRAILER_BYTES]);
    blob[len - BLACK_BOX_TRAILER_BYTES..].copy_from_slice(&chk.to_le_bytes());

    let tampered = decode_black_box_from_bytes(&blob).expect("dec tampered");
    assert_eq!(
        replay_black_box_verify(&init, &hf, &runway, &tampered),
        Err(FlightSimError::ReplayMismatch)
    );
}

#[test]
fn render_frame_into_atomic_preservation_and_require_hardware_rejection() {
    let (spec, heights, _runway) = test_terrain();
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");
    let cfg = FlightSimConfig::sim_config_default(12, 12);
    let sim = sim_init_level_flight(cfg, 300.0, 50.0).expect("trim");

    let target = RenderTargetSpec {
        width: 32,
        height: 32,
        clear_color: [10, 20, 30, 255],
        clear_depth: 1.0,
        clip_space: ClipSpace::OpenGl,
        post_process: PostProcessMode::Identity,
    };
    let hud_cfg = HudConfig::hud_config_default(320.0, 240.0).expect("hud");
    let lod =
        LodParams::lod_params_new(sim.body.position, 2.0, 32.0, 4, 256, 1024, None).expect("lod");

    let mut fb = PixelContainer::new(32, 32);
    fb.fill(0x55, 0x66, 0x77, 0x88);
    let mut zb = vec![0.42_f64; 32 * 32];
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

    let fb_snap = fb.data.clone();
    let zb_snap: Vec<u64> = zb.iter().map(|d| d.to_bits()).collect();
    let scene_snap = paint_scene_digest_fnv1a(&scene).expect("scene dig");

    // 1. RequireHardware rejected atomically
    for kind in [
        RenderBackendKind::Metal,
        RenderBackendKind::Wgpu,
        RenderBackendKind::Vulkan,
    ] {
        let err = render_frame_into(
            &sim,
            &hf,
            &lod,
            &hud_cfg,
            &target,
            BackendPreference::RequireHardware(kind),
            CameraMode::Cockpit,
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &mut scratch,
            Some(&mut scene),
        );
        assert_eq!(err, Err(FlightSimError::BackendUnavailable));
        assert_eq!(fb.data, fb_snap);
        assert_eq!(zb.iter().map(|d| d.to_bits()).collect::<Vec<_>>(), zb_snap);
        assert_eq!(paint_scene_digest_fnv1a(&scene).expect("sdig"), scene_snap);
    }

    // 2. NaN in Heightfield, LodParams, HudConfig, RenderTargetSpec, and wrong PaintScene
    let mut bad_heights = heights.clone();
    bad_heights[10] = QNAN;
    assert!(Heightfield::from_heights(spec, &bad_heights).is_err());

    let mut bad_lod = lod;
    bad_lod.max_screen_error = SNAN;
    assert!(render_frame_into(
        &sim,
        &hf,
        &bad_lod,
        &hud_cfg,
        &target,
        BackendPreference::ForceSoftware,
        CameraMode::Cockpit,
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &mut scratch,
        Some(&mut scene),
    )
    .is_err());
    assert_eq!(fb.data, fb_snap);
    assert_eq!(zb.iter().map(|d| d.to_bits()).collect::<Vec<_>>(), zb_snap);
    assert_eq!(paint_scene_digest_fnv1a(&scene).expect("sdig"), scene_snap);

    let mut bad_hud = hud_cfg;
    bad_hud.max_primitives = 64; // valid config, too small for full HUD -> dry-run rejects atomically!
    assert!(render_frame_into(
        &sim,
        &hf,
        &lod,
        &bad_hud,
        &target,
        BackendPreference::ForceSoftware,
        CameraMode::Cockpit,
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &mut scratch,
        Some(&mut scene),
    )
    .is_err());
    assert_eq!(fb.data, fb_snap);
    assert_eq!(zb.iter().map(|d| d.to_bits()).collect::<Vec<_>>(), zb_snap);
    assert_eq!(paint_scene_digest_fnv1a(&scene).expect("sdig"), scene_snap);

    let mut bad_target = target;
    bad_target.clear_depth = f64::INFINITY;
    assert!(render_frame_into(
        &sim,
        &hf,
        &lod,
        &hud_cfg,
        &bad_target,
        BackendPreference::ForceSoftware,
        CameraMode::Cockpit,
        &mut fb,
        DepthBufferMut::F64(&mut zb),
        &mut scratch,
        Some(&mut scene),
    )
    .is_err());
    assert_eq!(fb.data, fb_snap);
    assert_eq!(zb.iter().map(|d| d.to_bits()).collect::<Vec<_>>(), zb_snap);
    assert_eq!(paint_scene_digest_fnv1a(&scene).expect("sdig"), scene_snap);

    let mut empty_scene = paint_instructions::PaintScene::new(320.0, 240.0);
    assert_eq!(
        render_frame_into(
            &sim,
            &hf,
            &lod,
            &hud_cfg,
            &target,
            BackendPreference::ForceSoftware,
            CameraMode::Cockpit,
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &mut scratch,
            Some(&mut empty_scene),
        ),
        Err(FlightSimError::BufferTooSmall)
    );
    assert_eq!(fb.data, fb_snap);
    assert_eq!(zb.iter().map(|d| d.to_bits()).collect::<Vec<_>>(), zb_snap);
}

#[test]
fn seeded_2000_case_hostile_fuzz_campaign_zero_panics() {
    let (spec, heights, runway) = test_terrain();
    let hf = Heightfield::from_heights(spec, &heights).expect("hf");
    let cfg = FlightSimConfig::sim_config_default(0xF002_CAFE, 77);
    let base_sim = sim_init_on_runway(cfg, &runway).expect("init");
    let hud_cfg = HudConfig::hud_config_default(320.0, 240.0).expect("hud");
    let target = RenderTargetSpec {
        width: 16,
        height: 16,
        clear_color: [0, 0, 0, 255],
        clear_depth: 1.0,
        clip_space: ClipSpace::OpenGl,
        post_process: PostProcessMode::Identity,
    };
    let mut fb = PixelContainer::new(16, 16);
    let mut zb = vec![1.0_f64; 16 * 16];
    let mut scratch = FlightRenderScratch::flight_scratch_with_capacity(
        256,
        hf.vertex_count(),
        1024,
        16,
        16,
        2048,
    )
    .expect("scratch");
    let mut scene = prepare_paint_scene(&hud_cfg).expect("scene");

    let mut rng = SplitMix64(0x2026_0045_DEAD_BEEF);
    let mut sim = base_sim.clone();
    let mut fuzz_blob = [0_u8; 256];

    for iter in 0..2000_usize {
        if iter.is_multiple_of(100) {
            sim = base_sim.clone();
        }
        let dt_ns = match rng.next_u64() % 5 {
            0 => 0,
            1 => u64::MAX,
            2 => MAX_STEP_DT_NS + 1,
            _ => cfg.fixed_dt_ns,
        };
        let input = FlightInputFrame {
            dt_ns,
            elevator: rng.next_f64_hostile(),
            aileron: rng.next_f64_hostile(),
            rudder: rng.next_f64_hostile(),
            throttle: rng.next_f64_hostile(),
            flaps: rng.next_f64_hostile(),
            brake: rng.next_f64_hostile(),
            gear_down: (rng.next_u64() & 1) != 0,
        };
        let _ = step_in_place(&mut sim, &hf, &runway, &input, None);

        let lod = LodParams {
            camera_pos: Vec3::new(
                rng.next_f64_hostile(),
                rng.next_f64_hostile(),
                rng.next_f64_hostile(),
            ),
            max_screen_error: rng.next_f64_hostile(),
            projection_scale: rng.next_f64_hostile(),
            max_depth: 4,
            max_nodes: 256,
            max_triangles: 1024,
            frustum: None,
        };
        let pref = if (rng.next_u64() & 3) == 0 {
            BackendPreference::RequireHardware(RenderBackendKind::Vulkan)
        } else {
            BackendPreference::AutoHardwareWithSoftwareFallback
        };
        let _ = render_frame_into(
            &sim,
            &hf,
            &lod,
            &hud_cfg,
            &target,
            pref,
            CameraMode::Cockpit,
            &mut fb,
            DepthBufferMut::F64(&mut zb),
            &mut scratch,
            Some(&mut scene),
        );

        // Fuzz binary FDR decoder on random/mutated bytes
        let len = (rng.next_u64() as usize) % fuzz_blob.len();
        for b in &mut fuzz_blob[..len] {
            *b = (rng.next_u64() & 0xff) as u8;
        }
        if len >= 4 && (rng.next_u64() & 1) == 0 {
            fuzz_blob[0..4].copy_from_slice(b"GFDR");
        }
        let _ = decode_black_box_from_bytes(&fuzz_blob[..len]);
    }
}
