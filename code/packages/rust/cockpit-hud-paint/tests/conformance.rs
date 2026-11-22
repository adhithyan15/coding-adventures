//! # Conformance Tests for `cockpit-hud-paint` (GAME43)
//!
//! Verifies:
//! - Viewport bounds (`64..=8192`), `HudDisplayMode`s, and `HudUnits`
//! - `paint_vm_runtime::analyze_scene` & `PaintBackendRegistry` compatibility
//! - Stable deterministic `PaintBase.id`s across telemetry updates (`PaintVM.patch()`)
//! - 2D Liang-Barsky & Sutherland-Hodgman clipping at `±90°` pitch / `±180°` roll
//! - Airspeed, altimeter, VSI, radar altitude, heading, FPM, AoA/G-load, and annunciators
//! - `rigidbody-6dof` (`RigidBodyState`) telemetry extraction
//! - Canonical `-0.0` / `NaN` FNV-1a digests and 1,000-case deterministic property suite

#![forbid(unsafe_code)]

use cockpit_hud_paint::{
    build_hud_frame_into, clip_line_liang_barsky, paint_scene_digest_fnv1a, prepare_paint_scene,
    render_hud_scene, update_paint_scene_into, verify_paint_scene_finite, FlightTelemetry,
    HudConfig, HudDisplayMode, HudError, HudPrimitiveKind, HudRect, HudScratch, HudUnits,
    MAX_VIEWPORT_DIM, MIN_VIEWPORT_DIM,
};
use math3d::{Quat, Vec3};
use paint_instructions::{PaintInstruction, PaintScene, PixelContainer};
use paint_vm_runtime::{
    analyze_scene, select_backend, PaintAcceleration, PaintBackendCapabilities,
    PaintBackendDescriptor, PaintBackendFamily, PaintBackendTier, PaintPlatformSupport,
    PaintRenderError, PaintRenderOptions, PaintRenderer, SupportLevel,
};
use rigidbody_6dof::{MassProperties, RigidBodyState};

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn next_range(state: &mut u64, lo: f64, hi: f64) -> f64 {
    let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
    lo + (hi - lo) * u
}

struct FullVectorBackend;

impl PaintRenderer for FullVectorBackend {
    fn descriptor(&self) -> PaintBackendDescriptor {
        PaintBackendDescriptor {
            id: "test-vector-cpu",
            display_name: "Test Vector CPU",
            family: PaintBackendFamily::Cairo,
            acceleration: PaintAcceleration::Cpu,
            tier: PaintBackendTier::Tier2NativeScenes,
            platforms: PaintPlatformSupport {
                windows: true,
                macos: true,
                ios: true,
                linux: true,
                bsd: true,
                android: true,
                web: true,
                headless: true,
            },
            capabilities: PaintBackendCapabilities {
                rect: SupportLevel::Supported,
                line: SupportLevel::Supported,
                ellipse: SupportLevel::Supported,
                path: SupportLevel::Supported,
                text: SupportLevel::Supported,
                offscreen_pixels: SupportLevel::Supported,
                ..PaintBackendCapabilities::none()
            },
            priority: 1,
        }
    }

    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError> {
        Ok(PixelContainer::new(scene.width as u32, scene.height as u32))
    }
}

#[test]
fn viewport_boundaries_and_modes() {
    for &(w, h) in &[
        (MIN_VIEWPORT_DIM, MIN_VIEWPORT_DIM),
        (640.0, 480.0),
        (1280.0, 720.0),
        (MAX_VIEWPORT_DIM, MAX_VIEWPORT_DIM),
    ] {
        for mode in [
            HudDisplayMode::CombinedPfdHud,
            HudDisplayMode::PfdOnly,
            HudDisplayMode::HudOverlayOnly,
        ] {
            let mut cfg = HudConfig::hud_config_default(w, h).unwrap();
            cfg.mode = mode;
            let tel = FlightTelemetry::telemetry_level_flight();
            let scene = render_hud_scene(&tel, &cfg).unwrap();
            verify_paint_scene_finite(&scene).unwrap();
            assert_eq!(scene.width, w);
            assert_eq!(scene.height, h);

            let mut scratch = HudScratch::hud_scratch_new();
            let summary = build_hud_frame_into(&tel, &cfg, &mut scratch).unwrap();
            assert!(summary.primitive_count > 10);
            let scratch_scene = scratch.hud_scratch_to_paint_scene(&cfg).unwrap();
            verify_paint_scene_finite(&scratch_scene).unwrap();
        }
    }
}

#[test]
fn paint_vm_runtime_analyze_and_select_backend() {
    let cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    let tel = FlightTelemetry::telemetry_level_flight();
    let scene = render_hud_scene(&tel, &cfg).unwrap();

    let reqs = analyze_scene(&scene);
    assert!(reqs.uses_rect);
    assert!(reqs.uses_line);
    assert!(reqs.uses_ellipse);
    assert!(reqs.uses_path);
    assert!(reqs.uses_text);
    assert!(!reqs.uses_path_arc_to);
    assert!(!reqs.uses_image);
    assert!(!reqs.uses_linear_gradient);

    let backend = FullVectorBackend;
    let backends: [&dyn PaintRenderer; 1] = [&backend];
    let selected = select_backend(&scene, PaintRenderOptions::default(), &backends).unwrap();
    assert_eq!(selected.descriptor().id, "test-vector-cpu");
}

#[test]
fn stable_deterministic_instruction_ids_across_frames() {
    let cfg = HudConfig::hud_config_default(1024.0, 768.0).unwrap();
    let mut scene = prepare_paint_scene(&cfg).unwrap();

    let mut t1 = FlightTelemetry::telemetry_level_flight();
    t1.pitch_rad = trig::radians(12.0);
    t1.roll_rad = trig::radians(-25.0);
    t1.heading_rad = trig::radians(95.0);
    let s1 = update_paint_scene_into(&t1, &cfg, &mut scene).unwrap();
    let ids1: Vec<Option<String>> = scene
        .instructions
        .iter()
        .map(|i| match i {
            PaintInstruction::Rect(r) => r.base.id.clone(),
            PaintInstruction::Line(l) => l.base.id.clone(),
            PaintInstruction::Ellipse(e) => e.base.id.clone(),
            PaintInstruction::Path(p) => p.base.id.clone(),
            PaintInstruction::Text(t) => t.base.id.clone(),
            _ => None,
        })
        .collect();

    let mut t2 = FlightTelemetry::telemetry_level_flight();
    t2.pitch_rad = trig::radians(-18.0);
    t2.roll_rad = trig::radians(40.0);
    t2.heading_rad = trig::radians(355.0);
    t2.stall_warning = true;
    let s2 = update_paint_scene_into(&t2, &cfg, &mut scene).unwrap();
    let ids2: Vec<Option<String>> = scene
        .instructions
        .iter()
        .map(|i| match i {
            PaintInstruction::Rect(r) => r.base.id.clone(),
            PaintInstruction::Line(l) => l.base.id.clone(),
            PaintInstruction::Ellipse(e) => e.base.id.clone(),
            PaintInstruction::Path(p) => p.base.id.clone(),
            PaintInstruction::Text(t) => t.base.id.clone(),
            _ => None,
        })
        .collect();

    assert_eq!(ids1, ids2);
    assert!(ids1.iter().all(|id| id.is_some()));
    assert_ne!(s1.frame_digest_fnv1a, s2.frame_digest_fnv1a);

    // Repeating t1 reproduces the exact s1 digest bit-for-bit.
    let s1_repeat = update_paint_scene_into(&t1, &cfg, &mut scene).unwrap();
    assert_eq!(s1.frame_digest_fnv1a, s1_repeat.frame_digest_fnv1a);
}

#[test]
fn extreme_pitch_and_roll_remain_clipped_and_finite() {
    let cfg = HudConfig::hud_config_default(800.0, 600.0).unwrap();
    let mut scratch = HudScratch::hud_scratch_new();
    let mut scene = prepare_paint_scene(&cfg).unwrap();

    for &pitch_deg in &[-90.0, -45.0, 0.0, 45.0, 90.0] {
        for &roll_deg in &[-180.0, -90.0, 0.0, 90.0, 180.0] {
            let mut tel = FlightTelemetry::telemetry_level_flight();
            tel.pitch_rad = trig::radians(pitch_deg);
            tel.roll_rad = trig::radians(roll_deg);

            let sum = build_hud_frame_into(&tel, &cfg, &mut scratch).unwrap();
            assert!(sum.primitive_count > 0);
            for prim in scratch.hud_scratch_primitives() {
                match prim.kind {
                    HudPrimitiveKind::Line { x1, y1, x2, y2, .. } => {
                        assert!(
                            x1.is_finite() && y1.is_finite() && x2.is_finite() && y2.is_finite()
                        );
                        assert!((0.0..=800.0).contains(&x1) && (0.0..=800.0).contains(&x2));
                        assert!((0.0..=600.0).contains(&y1) && (0.0..=600.0).contains(&y2));
                    }
                    HudPrimitiveKind::Polygon { vx, vy, count, .. } => {
                        assert!((3..=5).contains(&count));
                        for i in 0..count as usize {
                            assert!(vx[i].is_finite() && vy[i].is_finite());
                            assert!((0.0..=800.0).contains(&vx[i]));
                            assert!((0.0..=600.0).contains(&vy[i]));
                        }
                    }
                    _ => {}
                }
            }

            update_paint_scene_into(&tel, &cfg, &mut scene).unwrap();
            verify_paint_scene_finite(&scene).unwrap();
        }
    }
}

#[test]
fn liang_barsky_clipping_cases() {
    let rect = HudRect {
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 100.0,
    };
    // Fully inside
    assert_eq!(
        clip_line_liang_barsky(20.0, 20.0, 80.0, 80.0, rect),
        Some((20.0, 20.0, 80.0, 80.0))
    );
    // Crossing left and right edges
    let clipped = clip_line_liang_barsky(0.0, 50.0, 200.0, 50.0, rect).unwrap();
    assert_eq!(clipped, (10.0, 50.0, 110.0, 50.0));
    // Completely outside
    assert_eq!(clip_line_liang_barsky(0.0, 0.0, 5.0, 50.0, rect), None);
    // Non-finite input returns None
    assert_eq!(
        clip_line_liang_barsky(
            f64::NAN,
            0.0,
            50.0,
            50.0,
            HudRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        ),
        None
    );
}

#[test]
fn annunciators_and_units_work_as_specified() {
    let mut cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    cfg.units = HudUnits::Si;

    let mut tel = FlightTelemetry::telemetry_level_flight();
    tel.aoa_rad = trig::radians(18.0);
    tel.indicated_airspeed_mps = 380.0;
    tel.radar_altitude_agl_m = 45.0;
    tel.vertical_speed_mps = -12.0;

    let mut scratch = HudScratch::hud_scratch_new();
    let summary = build_hud_frame_into(&tel, &cfg, &mut scratch).unwrap();
    assert!(summary.annunciators.stall);
    assert!(summary.annunciators.overspeed);
    assert!(summary.annunciators.terrain);
    assert!(summary.annunciators.pull_up);
    assert_eq!(summary.annunciators.annunciator_count(), 4);

    let has_pullup = scratch
        .hud_scratch_primitives()
        .iter()
        .any(|p| p.primitive_label() == "PULL UP");
    assert!(has_pullup);
}

#[test]
fn rigidbody_6dof_telemetry_integration() {
    let mass = MassProperties::mass_props_from_solid_box(1200.0, Vec3::new(2.0, 0.6, 1.5)).unwrap();
    let pitch_up = Quat::from_axis_angle(Vec3::unit_z(), trig::radians(10.0)).unwrap();
    let mut body = RigidBodyState::body_new(Vec3::new(0.0, 1500.0, 0.0), pitch_up).unwrap();
    body.linear_momentum = Vec3::new(100.0 * 1200.0, 15.0 * 1200.0, 0.0);
    body.apply_world_force(Vec3::new(0.0, 1200.0 * 9.80665, 0.0))
        .unwrap();

    let tel = FlightTelemetry::telemetry_from_rigid_body(&body, &mass, 340.0, 200.0).unwrap();
    assert!((tel.pitch_rad - trig::radians(10.0)).abs() < 1e-6);
    assert!((tel.altitude_msl_m - 1500.0).abs() < 1e-9);
    assert!((tel.radar_altitude_agl_m - 1300.0).abs() < 1e-9);
    assert!(tel.indicated_airspeed_mps > 100.0);

    let cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    let scene = render_hud_scene(&tel, &cfg).unwrap();
    verify_paint_scene_finite(&scene).unwrap();
}

#[test]
fn canonical_digest_normalizes_negative_zero_and_nan() {
    let cfg = HudConfig::hud_config_default(1024.0, 768.0).unwrap();
    let mut t_pos = FlightTelemetry::telemetry_level_flight();
    t_pos.pitch_rad = 0.0;
    t_pos.roll_rad = 0.0;
    t_pos.heading_rad = 0.0;
    t_pos.vertical_speed_mps = 0.0;

    let mut t_neg = t_pos;
    t_neg.pitch_rad = -0.0;
    t_neg.roll_rad = -0.0;
    t_neg.heading_rad = -0.0;
    t_neg.vertical_speed_mps = -0.0;

    let mut s1 = HudScratch::hud_scratch_new();
    let mut s2 = HudScratch::hud_scratch_new();
    let sum1 = build_hud_frame_into(&t_pos, &cfg, &mut s1).unwrap();
    let sum2 = build_hud_frame_into(&t_neg, &cfg, &mut s2).unwrap();
    assert_eq!(sum1.frame_digest_fnv1a, sum2.frame_digest_fnv1a);

    let sc1 = render_hud_scene(&t_pos, &cfg).unwrap();
    let mut sc2 = sc1.clone();
    sc2.width = -0.0; // test canonicalization directly on PaintScene
    let mut sc3 = sc1.clone();
    sc3.width = 0.0;
    assert_eq!(
        paint_scene_digest_fnv1a(&sc2).unwrap(),
        paint_scene_digest_fnv1a(&sc3).unwrap()
    );

    // Distinct NaN payloads canonicalize identically.
    let mut sn1 = PaintScene::new(f64::from_bits(0x7ff8_0000_0000_0001), 100.0);
    let sn2 = PaintScene::new(f64::from_bits(0x7ff8_1234_5678_9abc), 100.0);
    sn1.background = sn2.background.clone();
    assert_eq!(
        paint_scene_digest_fnv1a(&sn1).unwrap(),
        paint_scene_digest_fnv1a(&sn2).unwrap()
    );
}

#[test]
fn property_1000_valid_frames_deterministic_and_finite() {
    let mut rng = 0x434f_434b_5049_5401_u64;
    let cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    let mut scratch = HudScratch::hud_scratch_new();
    let mut scene = prepare_paint_scene(&cfg).unwrap();

    for _ in 0..1000 {
        let tel = FlightTelemetry {
            pitch_rad: next_range(&mut rng, -trig::PI, trig::PI),
            roll_rad: next_range(&mut rng, -trig::PI, trig::PI),
            heading_rad: next_range(&mut rng, -20.0, 20.0),
            indicated_airspeed_mps: next_range(&mut rng, 0.0, 600.0),
            true_airspeed_mps: next_range(&mut rng, 0.0, 650.0),
            mach: next_range(&mut rng, 0.0, 2.5),
            altitude_msl_m: next_range(&mut rng, -300.0, 25_000.0),
            radar_altitude_agl_m: next_range(&mut rng, 0.0, 25_000.0),
            vertical_speed_mps: next_range(&mut rng, -200.0, 200.0),
            flight_path_pitch_rad: next_range(&mut rng, -1.5, 1.5),
            flight_path_yaw_rad: next_range(&mut rng, -1.5, 1.5),
            aoa_rad: next_range(&mut rng, -0.6, 0.6),
            sideslip_rad: next_range(&mut rng, -0.6, 0.6),
            g_load: next_range(&mut rng, -5.0, 12.0),
            stall_warning: next_u64(&mut rng) & 1 == 1,
            overspeed_warning: next_u64(&mut rng) & 1 == 1,
            terrain_warning: next_u64(&mut rng) & 1 == 1,
            pull_up_warning: next_u64(&mut rng) & 1 == 1,
        };

        let s_scratch = build_hud_frame_into(&tel, &cfg, &mut scratch).unwrap();
        let s_scene = update_paint_scene_into(&tel, &cfg, &mut scene).unwrap();
        assert!(s_scratch.primitive_count <= cfg.max_primitives);
        assert_eq!(
            s_scratch.frame_digest_fnv1a,
            scratch.hud_scratch_digest_fnv1a().unwrap()
        );
        verify_paint_scene_finite(&scene).unwrap();
        assert_eq!(
            s_scene.frame_digest_fnv1a,
            paint_scene_digest_fnv1a(&scene).unwrap()
        );
    }

    // Confirm invalid viewport still returns typed Err.
    assert_eq!(
        HudConfig::hud_config_default(63.0, 720.0),
        Err(HudError::InvalidViewport)
    );
}

#[test]
fn scratch_ids_are_unique_and_digest_matches_summary() {
    let cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    let mut tel = FlightTelemetry::telemetry_level_flight();
    tel.stall_warning = true;
    tel.overspeed_warning = true;
    tel.terrain_warning = true;
    tel.pull_up_warning = true;

    let mut scratch = HudScratch::hud_scratch_new();
    let summary = build_hud_frame_into(&tel, &cfg, &mut scratch).unwrap();
    assert_eq!(
        summary.frame_digest_fnv1a,
        scratch.hud_scratch_digest_fnv1a().unwrap()
    );

    let scene = scratch.hud_scratch_to_paint_scene(&cfg).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for instr in &scene.instructions {
        let id = match instr {
            PaintInstruction::Rect(r) => r.base.id.as_ref(),
            PaintInstruction::Line(l) => l.base.id.as_ref(),
            PaintInstruction::Ellipse(e) => e.base.id.as_ref(),
            PaintInstruction::Path(p) => p.base.id.as_ref(),
            PaintInstruction::Text(t) => t.base.id.as_ref(),
            _ => None,
        }
        .expect("every instruction must carry a PaintBase.id");
        assert!(seen.insert(id.clone()), "duplicate PaintBase.id: {id}");
    }
}
