//! # Mandatory `#429` Hostile-Input CVE Proof Suite for `cockpit-hud-paint` (GAME43)
//!
//! Proves via explicit edge vectors and a seeded 2,000-case hostile fuzz
//! campaign that CWE-20, CWE-125, CWE-190, CWE-369, CWE-400, CWE-787, and
//! CWE-835 cannot trigger panics, non-finite coordinates, unbounded loops,
//! or partial buffer corruption:
//! - qNaN, sNaN (`0x7ff0_0000_0000_0001`), `±Inf`, `-0.0`, subnormals, `±f64::MAX`
//! - Huge angles (`1e12`, `1e300`), `±90°` pitch / `±180°` roll
//! - Negative airspeed, extreme altitudes (`-1e9`, `1e9`), `0x0` / `1e9` viewports
//! - `BufferTooSmall` byte-for-byte preservation on both `HudScratch` & `PaintScene`
//! - Finite-only verification on every emitted `PaintInstruction`
//! - Seeded 2,000-case hostile telemetry/viewport/config fuzz campaign (0 panics).

#![forbid(unsafe_code)]

use cockpit_hud_paint::{
    build_hud_frame_into, paint_scene_digest_fnv1a, prepare_paint_scene, render_hud_scene,
    update_paint_scene_into, verify_paint_scene_finite, FlightTelemetry, HudConfig, HudError,
    HudPrimitive, HudPrimitiveKind, HudScratch, MAX_ANGLE_RAD, MAX_PRIMITIVES, MAX_VIEWPORT_DIM,
    MIN_VIEWPORT_DIM,
};
use paint_instructions::PaintScene;

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn hostile_scalar(state: &mut u64) -> f64 {
    match next_u64(state) % 18 {
        0 => f64::NAN,
        1 => f64::from_bits(0x7ff0_0000_0000_0001), // signaling NaN
        2 => f64::INFINITY,
        3 => f64::NEG_INFINITY,
        4 => -0.0,
        5 => 0.0,
        6 => f64::from_bits(1), // smallest positive subnormal
        7 => f64::from_bits(0x8000_0000_0000_0001), // smallest negative subnormal
        8 => f64::MIN_POSITIVE * 0.5,
        9 => f64::MIN,
        10 => f64::MAX,
        11 => 1.0e12,
        12 => -1.0e12,
        13 => 1.0e300,
        14 => -1.0e300,
        15 => -50.0,
        16 => 1.0e9,
        _ => {
            let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
            -20.0 + 40.0 * u
        }
    }
}

fn snapshot_scratch(s: &HudScratch) -> (usize, HudScratch) {
    (s.hud_scratch_capacity(), s.clone())
}

fn scratch_bytes_equal(a: &HudScratch, cap_before: usize, b: &HudScratch) -> bool {
    if a.hud_scratch_len() != b.hud_scratch_len()
        || a.hud_scratch_capacity() != cap_before
        || a.hud_scratch_summary() != b.hud_scratch_summary()
    {
        return false;
    }
    a.hud_scratch_primitives()
        .iter()
        .zip(b.hud_scratch_primitives().iter())
        .all(|(x, y)| prim_bits_equal(x, y))
}

fn prim_bits_equal(a: &HudPrimitive, b: &HudPrimitive) -> bool {
    if a.primitive_id() != b.primitive_id()
        || a.primitive_label() != b.primitive_label()
        || a.primitive_color() != b.primitive_color()
    {
        return false;
    }
    match (a.kind, b.kind) {
        (
            HudPrimitiveKind::Rect {
                x: ax,
                y: ay,
                width: aw,
                height: ah,
                filled: af,
                stroked: as_,
            },
            HudPrimitiveKind::Rect {
                x: bx,
                y: by,
                width: bw,
                height: bh,
                filled: bf,
                stroked: bs,
            },
        ) => {
            ax.to_bits() == bx.to_bits()
                && ay.to_bits() == by.to_bits()
                && aw.to_bits() == bw.to_bits()
                && ah.to_bits() == bh.to_bits()
                && af == bf
                && as_ == bs
        }
        (
            HudPrimitiveKind::Line {
                x1: ax1,
                y1: ay1,
                x2: ax2,
                y2: ay2,
                stroke_width: aw,
                dashed: ad,
            },
            HudPrimitiveKind::Line {
                x1: bx1,
                y1: by1,
                x2: bx2,
                y2: by2,
                stroke_width: bw,
                dashed: bd,
            },
        ) => {
            ax1.to_bits() == bx1.to_bits()
                && ay1.to_bits() == by1.to_bits()
                && ax2.to_bits() == bx2.to_bits()
                && ay2.to_bits() == by2.to_bits()
                && aw.to_bits() == bw.to_bits()
                && ad == bd
        }
        (
            HudPrimitiveKind::Ellipse {
                cx: acx,
                cy: acy,
                rx: arx,
                ry: ary,
                stroke_width: aw,
            },
            HudPrimitiveKind::Ellipse {
                cx: bcx,
                cy: bcy,
                rx: brx,
                ry: bry,
                stroke_width: bw,
            },
        ) => {
            acx.to_bits() == bcx.to_bits()
                && acy.to_bits() == bcy.to_bits()
                && arx.to_bits() == brx.to_bits()
                && ary.to_bits() == bry.to_bits()
                && aw.to_bits() == bw.to_bits()
        }
        (
            HudPrimitiveKind::Polygon {
                vx: avx,
                vy: avy,
                count: ac,
                filled: af,
                stroked: as_,
            },
            HudPrimitiveKind::Polygon {
                vx: bvx,
                vy: bvy,
                count: bc,
                filled: bf,
                stroked: bs,
            },
        ) => {
            ac == bc
                && af == bf
                && as_ == bs
                && (0..5).all(|i| {
                    avx[i].to_bits() == bvx[i].to_bits() && avy[i].to_bits() == bvy[i].to_bits()
                })
        }
        (
            HudPrimitiveKind::Text {
                x: ax,
                y: ay,
                font_size: af,
                align_center: ac,
            },
            HudPrimitiveKind::Text {
                x: bx,
                y: by,
                font_size: bf,
                align_center: bc,
            },
        ) => {
            ax.to_bits() == bx.to_bits()
                && ay.to_bits() == by.to_bits()
                && af.to_bits() == bf.to_bits()
                && ac == bc
        }
        _ => false,
    }
}

#[test]
fn hostile_viewports_and_caps_rejected() {
    for &(w, h) in &[
        (0.0, 0.0),
        (-0.0, 720.0),
        (63.999, 720.0),
        (720.0, 63.0),
        (8192.001, 720.0),
        (1.0e9, 1.0e9),
        (f64::NAN, 720.0),
        (f64::from_bits(0x7ff0_0000_0000_0001), 720.0),
        (f64::INFINITY, 720.0),
        (f64::NEG_INFINITY, 720.0),
        (f64::MAX, f64::MAX),
        (f64::from_bits(1), 720.0),
    ] {
        assert!(HudConfig::hud_config_default(w, h).is_err());
    }

    let mut bad_caps = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    bad_caps.max_primitives = MAX_PRIMITIVES + 1;
    assert_eq!(
        bad_caps.hud_config_validate(),
        Err(HudError::PrimitiveLimitExceeded)
    );

    bad_caps.max_primitives = 10;
    assert_eq!(
        bad_caps.hud_config_validate(),
        Err(HudError::PrimitiveLimitExceeded)
    );

    bad_caps = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    bad_caps.max_pitch_rungs = 0;
    assert_eq!(
        bad_caps.hud_config_validate(),
        Err(HudError::PrimitiveLimitExceeded)
    );

    bad_caps.max_pitch_rungs = 73;
    assert_eq!(
        bad_caps.hud_config_validate(),
        Err(HudError::PrimitiveLimitExceeded)
    );

    // Valid extremes accepted
    assert!(HudConfig::hud_config_default(MIN_VIEWPORT_DIM, MAX_VIEWPORT_DIM).is_ok());
}

#[test]
fn hostile_scalar_and_angle_vectors_atomically_preserve_buffers() {
    let cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    let valid_tel = FlightTelemetry::telemetry_level_flight();

    let mut scratch = HudScratch::hud_scratch_new();
    build_hud_frame_into(&valid_tel, &cfg, &mut scratch).unwrap();
    let (scratch_cap, scratch_snap) = snapshot_scratch(&scratch);

    let mut scene = prepare_paint_scene(&cfg).unwrap();
    update_paint_scene_into(&valid_tel, &cfg, &mut scene).unwrap();
    let scene_snap = scene.clone();

    let snan = f64::from_bits(0x7ff0_0000_0000_0001);
    let subnormal_min = f64::from_bits(1);
    let subnormal_neg = f64::from_bits(0x8000_0000_0000_0001);
    let subnormal_mid = f64::MIN_POSITIVE * 0.5;

    let bad_angles = [
        f64::NAN,
        snan,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e12,
        -1.0e12,
        1.0e300,
        -1.0e300,
        MAX_ANGLE_RAD + 1.0,
        -MAX_ANGLE_RAD - 1.0,
        f64::MAX,
        f64::MIN,
    ];
    for &ang in &bad_angles {
        let mut tel = valid_tel;
        tel.pitch_rad = ang;
        assert!(build_hud_frame_into(&tel, &cfg, &mut scratch).is_err());
        assert!(scratch_bytes_equal(&scratch, scratch_cap, &scratch_snap));
        assert!(update_paint_scene_into(&tel, &cfg, &mut scene).is_err());
        assert_eq!(scene, scene_snap);

        let mut tel_r = valid_tel;
        tel_r.roll_rad = ang;
        assert!(build_hud_frame_into(&tel_r, &cfg, &mut scratch).is_err());
        assert!(scratch_bytes_equal(&scratch, scratch_cap, &scratch_snap));

        let mut tel_h = valid_tel;
        tel_h.heading_rad = ang;
        assert!(update_paint_scene_into(&tel_h, &cfg, &mut scene).is_err());
        assert_eq!(scene, scene_snap);
    }

    // Negative airspeed, extreme altitudes, negative AGL
    let mut neg_ias = valid_tel;
    neg_ias.indicated_airspeed_mps = -0.001;
    assert_eq!(
        build_hud_frame_into(&neg_ias, &cfg, &mut scratch),
        Err(HudError::InvalidTelemetry)
    );
    assert!(scratch_bytes_equal(&scratch, scratch_cap, &scratch_snap));

    let mut extreme_alt = valid_tel;
    extreme_alt.altitude_msl_m = 1.0e9;
    assert_eq!(
        update_paint_scene_into(&extreme_alt, &cfg, &mut scene),
        Err(HudError::InvalidTelemetry)
    );
    assert_eq!(scene, scene_snap);

    let mut neg_agl = valid_tel;
    neg_agl.radar_altitude_agl_m = -1.0;
    assert_eq!(
        build_hud_frame_into(&neg_agl, &cfg, &mut scratch),
        Err(HudError::InvalidTelemetry)
    );
    assert!(scratch_bytes_equal(&scratch, scratch_cap, &scratch_snap));

    // Subnormals and -0.0 inside valid bounds must never panic or produce NaN.
    for &sub in &[-0.0, subnormal_min, subnormal_neg, subnormal_mid] {
        let mut tel = valid_tel;
        tel.pitch_rad = sub;
        tel.roll_rad = sub;
        tel.heading_rad = sub;
        tel.vertical_speed_mps = sub;
        tel.indicated_airspeed_mps = sub.max(0.0);
        tel.radar_altitude_agl_m = sub.max(0.0);
        if tel.telemetry_validate().is_ok() {
            build_hud_frame_into(&tel, &cfg, &mut scratch).unwrap();
            let sc = scratch.hud_scratch_to_paint_scene(&cfg).unwrap();
            verify_paint_scene_finite(&sc).unwrap();
            update_paint_scene_into(&tel, &cfg, &mut scene).unwrap();
            verify_paint_scene_finite(&scene).unwrap();
        }
    }

    // Gimbal lock pitch +/-90 deg & inverted roll +/-180 deg must succeed cleanly.
    for &p in &[-0.5 * trig::PI, 0.5 * trig::PI] {
        for &r in &[-trig::PI, trig::PI] {
            let mut tel = valid_tel;
            tel.pitch_rad = p;
            tel.roll_rad = r;
            build_hud_frame_into(&tel, &cfg, &mut scratch).unwrap();
            let sc = scratch.hud_scratch_to_paint_scene(&cfg).unwrap();
            verify_paint_scene_finite(&sc).unwrap();
            update_paint_scene_into(&tel, &cfg, &mut scene).unwrap();
            verify_paint_scene_finite(&scene).unwrap();
        }
    }
}

#[test]
fn buffer_too_small_preserves_scratch_and_scene_byte_for_byte() {
    let cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    let valid_tel = FlightTelemetry::telemetry_level_flight();

    // Seed small scratch with 4 primitives and verify BufferTooSmall leaves them intact.
    let mut small_cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    small_cfg.max_primitives = 512;
    let mut small_scratch = HudScratch::hud_scratch_with_capacity(8);
    assert_eq!(
        build_hud_frame_into(&valid_tel, &small_cfg, &mut small_scratch),
        Err(HudError::BufferTooSmall)
    );
    assert_eq!(small_scratch.hud_scratch_len(), 0);
    assert_eq!(small_scratch.hud_scratch_summary(), None);

    // Build a valid frame in a 64-slot scratch? Wait: test with a cap smaller than frame count
    // after populating with a slightly smaller-count mode (HudOverlayOnly vs CombinedPfdHud).
    let mut hud_only_cfg = cfg;
    hud_only_cfg.mode = cockpit_hud_paint::HudDisplayMode::HudOverlayOnly;
    let mut probe_scratch = HudScratch::hud_scratch_new();
    let hud_sum = build_hud_frame_into(&valid_tel, &hud_only_cfg, &mut probe_scratch).unwrap();
    let comb_sum = build_hud_frame_into(&valid_tel, &cfg, &mut probe_scratch).unwrap();
    assert!(comb_sum.primitive_count > hud_sum.primitive_count);

    let mut tight_scratch = HudScratch::hud_scratch_with_capacity(hud_sum.primitive_count);
    build_hud_frame_into(&valid_tel, &hud_only_cfg, &mut tight_scratch).unwrap();
    let (tight_cap, tight_snap) = snapshot_scratch(&tight_scratch);

    assert_eq!(
        build_hud_frame_into(&valid_tel, &cfg, &mut tight_scratch),
        Err(HudError::BufferTooSmall)
    );
    assert!(scratch_bytes_equal(&tight_scratch, tight_cap, &tight_snap));

    // Undersized / empty PaintScene preserves byte-for-byte on BufferTooSmall.
    let mut empty_scene = PaintScene::new(800.0, 600.0);
    let empty_snap = empty_scene.clone();
    assert_eq!(
        update_paint_scene_into(&valid_tel, &cfg, &mut empty_scene),
        Err(HudError::BufferTooSmall)
    );
    assert_eq!(empty_scene, empty_snap);
}

#[test]
fn hostile_2000_case_fuzz_campaign() {
    let mut rng = 0xdead_beef_cafe_4343_u64;
    let base_cfg = HudConfig::hud_config_default(1280.0, 720.0).unwrap();
    let mut scratch = HudScratch::hud_scratch_new();
    let mut scene = prepare_paint_scene(&base_cfg).unwrap();

    // Seed with a known valid frame first.
    let init = FlightTelemetry::telemetry_level_flight();
    build_hud_frame_into(&init, &base_cfg, &mut scratch).unwrap();
    update_paint_scene_into(&init, &base_cfg, &mut scene).unwrap();

    for _ in 0..2000 {
        let mut cfg = base_cfg;
        if next_u64(&mut rng).is_multiple_of(4) {
            cfg.viewport_width = hostile_scalar(&mut rng);
            cfg.viewport_height = hostile_scalar(&mut rng);
            cfg.fov_vertical_deg = hostile_scalar(&mut rng);
        }
        if next_u64(&mut rng).is_multiple_of(5) {
            cfg.max_primitives = (next_u64(&mut rng) % 6000) as usize;
            cfg.max_pitch_rungs = (next_u64(&mut rng) % 100) as usize;
        }

        let tel = FlightTelemetry {
            pitch_rad: hostile_scalar(&mut rng),
            roll_rad: hostile_scalar(&mut rng),
            heading_rad: hostile_scalar(&mut rng),
            indicated_airspeed_mps: hostile_scalar(&mut rng),
            true_airspeed_mps: hostile_scalar(&mut rng),
            mach: hostile_scalar(&mut rng),
            altitude_msl_m: hostile_scalar(&mut rng),
            radar_altitude_agl_m: hostile_scalar(&mut rng),
            vertical_speed_mps: hostile_scalar(&mut rng),
            flight_path_pitch_rad: hostile_scalar(&mut rng),
            flight_path_yaw_rad: hostile_scalar(&mut rng),
            aoa_rad: hostile_scalar(&mut rng),
            sideslip_rad: hostile_scalar(&mut rng),
            g_load: hostile_scalar(&mut rng),
            stall_warning: next_u64(&mut rng) & 1 == 1,
            overspeed_warning: next_u64(&mut rng) & 1 == 1,
            terrain_warning: next_u64(&mut rng) & 1 == 1,
            pull_up_warning: next_u64(&mut rng) & 1 == 1,
        };

        let (scratch_cap, scratch_snap) = snapshot_scratch(&scratch);
        match build_hud_frame_into(&tel, &cfg, &mut scratch) {
            Ok(_) => {
                let sc = scratch.hud_scratch_to_paint_scene(&cfg).unwrap();
                verify_paint_scene_finite(&sc).unwrap();
            }
            Err(_) => {
                assert!(scratch_bytes_equal(&scratch, scratch_cap, &scratch_snap));
            }
        }

        let scene_snap = scene.clone();
        match update_paint_scene_into(&tel, &cfg, &mut scene) {
            Ok(summary) => {
                verify_paint_scene_finite(&scene).unwrap();
                assert_eq!(
                    summary.frame_digest_fnv1a,
                    paint_scene_digest_fnv1a(&scene).unwrap()
                );
            }
            Err(_) => {
                assert_eq!(scene, scene_snap);
            }
        }

        // Also test allocating render_hud_scene for zero-panic guarantee.
        if let Ok(rendered) = render_hud_scene(&tel, &cfg) {
            verify_paint_scene_finite(&rendered).unwrap();
        }
    }
}
