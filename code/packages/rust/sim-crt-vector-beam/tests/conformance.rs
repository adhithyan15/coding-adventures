//! # Conformance and Unit Test Suite for `sim-crt-vector-beam`
//!
//! Validates:
//! - Raster CRT timing standards (NTSC, Arcade) and interval flags (HBlank, VBlank, HSync, VSync).
//! - Phosphor persistence decay modeling (P4 White, P31 Green, P1 Green, Custom).
//! - Zero-allocation raster stepping and saturating intensity accumulation (CWE-190).
//! - Phosphor buffer bounds checking (CWE-125).
//! - Input validation on dimensions, timing, decay factors, and slew rates (CWE-20).
//! - Vector deflection amplifier simulation, bounded slewing, and Z-axis blanking.
//! - Bright-spot dwell timing for vector displays.
//! - Lowering to `PixelContainer` and `PaintScene`.

use sim_crt_vector_beam::{
    accumulate_intensity, clear_phosphor_buffer, decay_phosphor_buffer, decay_phosphor_sample,
    validate_raster_timing, validate_slew_rate, CrtError, CrtRasterBeam, PaintInstruction,
    PhosphorType, PixelContainer, RasterTiming, VectorBeamDisplay, VectorStroke,
};

#[test]
fn test_raster_timing_ntsc_and_arcade() {
    let ntsc = RasterTiming::standard_ntsc();
    assert!(ntsc.validate_timing().is_ok());
    assert_eq!(ntsc.visible_width, 320);
    assert_eq!(ntsc.visible_height, 240);
    assert_eq!(ntsc.total_raster_width(), 416);
    assert_eq!(ntsc.total_raster_height(), 262);
    assert_eq!(ntsc.total_raster_pixels(), 320 * 240);

    let arcade = RasterTiming::standard_arcade();
    assert!(arcade.validate_timing().is_ok());
    assert_eq!(arcade.visible_width, 256);
    assert_eq!(arcade.visible_height, 224);
    assert_eq!(arcade.total_raster_width(), 344);
    assert_eq!(arcade.total_raster_height(), 260);
    assert_eq!(arcade.total_raster_pixels(), 256 * 224);

    assert!(validate_raster_timing(ntsc).is_ok());
}

#[test]
fn test_raster_timing_validation_cwe20() {
    let mut invalid = RasterTiming::standard_ntsc();
    invalid.visible_width = 0;
    assert!(matches!(
        invalid.validate_timing(),
        Err(CrtError::InvalidDimensions { width: 0, .. })
    ));

    let mut invalid_h = RasterTiming::standard_ntsc();
    invalid_h.visible_height = 0;
    assert!(matches!(
        invalid_h.validate_timing(),
        Err(CrtError::InvalidDimensions { height: 0, .. })
    ));

    let mut overflow = RasterTiming::standard_ntsc();
    overflow.h_front_porch = u32::MAX;
    assert!(matches!(overflow.validate_timing(), Err(CrtError::InvalidTiming(_))));
}

#[test]
fn test_phosphor_decay_characteristics() {
    let p4 = PhosphorType::P4White;
    assert_eq!(p4.phosphor_decay_factor_q16(), 32768);
    assert_eq!(p4.phosphor_rgb_tint(), [240, 245, 255]);
    assert!(p4.validate_phosphor().is_ok());

    let p31 = PhosphorType::P31Green;
    assert_eq!(p31.phosphor_decay_factor_q16(), 45056);
    assert_eq!(p31.phosphor_rgb_tint(), [64, 255, 64]);
    assert!(p31.validate_phosphor().is_ok());

    let p1 = PhosphorType::P1Green;
    assert_eq!(p1.phosphor_decay_factor_q16(), 59904);
    assert_eq!(p1.phosphor_rgb_tint(), [50, 220, 50]);
    assert!(p1.validate_phosphor().is_ok());

    let custom = PhosphorType::Custom {
        decay_q16: 50000,
        tint: [255, 128, 0],
    };
    assert_eq!(custom.phosphor_decay_factor_q16(), 50000);
    assert_eq!(custom.phosphor_rgb_tint(), [255, 128, 0]);
    assert!(custom.validate_phosphor().is_ok());

    let invalid_decay_zero = PhosphorType::Custom {
        decay_q16: 0,
        tint: [255, 255, 255],
    };
    assert!(matches!(
        invalid_decay_zero.validate_phosphor(),
        Err(CrtError::InvalidPhosphorDecay(0))
    ));

    let invalid_decay_high = PhosphorType::Custom {
        decay_q16: 70000,
        tint: [255, 255, 255],
    };
    assert!(matches!(
        invalid_decay_high.validate_phosphor(),
        Err(CrtError::InvalidPhosphorDecay(70000))
    ));
}

#[test]
fn test_phosphor_sample_and_buffer_decay() {
    assert_eq!(decay_phosphor_sample(65535, 32768), 32767);
    assert_eq!(decay_phosphor_sample(65535, 65536), 65535);
    assert_eq!(decay_phosphor_sample(0, 32768), 0);

    let mut buffer = [65535u16; 8];
    assert!(decay_phosphor_buffer(&mut buffer, 32768).is_ok());
    for &val in &buffer {
        assert_eq!(val, 32767);
    }

    clear_phosphor_buffer(&mut buffer);
    for &val in &buffer {
        assert_eq!(val, 0);
    }
}

#[test]
fn test_accumulate_intensity_saturating_cwe190() {
    assert_eq!(accumulate_intensity(100, 200), 300);
    assert_eq!(accumulate_intensity(65500, 100), 65535);
    assert_eq!(accumulate_intensity(65535, 1), 65535);
}

#[test]
fn test_raster_beam_sweep_and_intervals() {
    let timing = RasterTiming {
        visible_width: 4,
        visible_height: 2,
        h_front_porch: 1,
        h_sync: 2,
        h_back_porch: 1,
        v_front_porch: 1,
        v_sync: 1,
        v_back_porch: 1,
    };
    assert_eq!(timing.total_raster_width(), 8);
    assert_eq!(timing.total_raster_height(), 5);

    let mut crt = CrtRasterBeam::new_raster_beam(timing, PhosphorType::P4White)
        .expect("raster beam creation should succeed");

    let mut phosphor = [0u16; 8]; // 4x2 visible

    // Step across first visible scanline: x=0..4
    for x in 0..4 {
        assert_eq!(crt.raster_beam_x(), x);
        assert_eq!(crt.raster_beam_y(), 0);
        let flags = crt
            .step_raster_beam(128, &mut phosphor)
            .expect("step should succeed");
        assert!(flags.visible_pixel);
        assert!(!flags.in_hblank);
        assert!(!flags.in_vblank);
        assert!(!flags.line_completed);
    }

    // Step across HBlank: x=4 (front porch)
    let fp_flags = crt
        .step_raster_beam(0, &mut phosphor)
        .expect("step should succeed");
    assert!(!fp_flags.visible_pixel);
    assert!(fp_flags.in_hblank);
    assert!(!fp_flags.in_hsync);

    // x=5, 6 (h_sync)
    let sync1 = crt
        .step_raster_beam(0, &mut phosphor)
        .expect("step should succeed");
    assert!(sync1.in_hsync);
    assert!(sync1.in_hblank);

    let sync2 = crt
        .step_raster_beam(0, &mut phosphor)
        .expect("step should succeed");
    assert!(sync2.in_hsync);
    assert!(sync2.in_hblank);

    // x=7 (back porch) -> completes line
    let bp = crt
        .step_raster_beam(0, &mut phosphor)
        .expect("step should succeed");
    assert!(bp.in_hblank);
    assert!(!bp.in_hsync);
    assert!(bp.line_completed);
    assert_eq!(crt.raster_beam_x(), 0);
    assert_eq!(crt.raster_beam_y(), 1);

    // Check excited phosphor values on line 0
    for &pixel in &phosphor[..4] {
        assert_eq!(pixel, (128 << 8) | 128);
    }
}

#[test]
fn test_raster_beam_frame_completion_and_bounds_cwe125() {
    let timing = RasterTiming {
        visible_width: 2,
        visible_height: 2,
        h_front_porch: 1,
        h_sync: 1,
        h_back_porch: 1,
        v_front_porch: 1,
        v_sync: 1,
        v_back_porch: 1,
    };
    // Total sweep: 5x5 = 25 clocks
    let mut crt = CrtRasterBeam::new_raster_beam(timing, PhosphorType::P31Green)
        .expect("raster beam creation should succeed");

    let mut empty_phosphor: [u16; 0] = [];
    assert!(matches!(
        crt.step_raster_beam(255, &mut empty_phosphor),
        Err(CrtError::BufferTooSmall { required: 1, provided: 0 })
    ));

    let mut phosphor = [0u16; 4];
    let mut frame_seen = false;

    for _ in 0..25 {
        let flags = crt
            .step_raster_beam(100, &mut phosphor)
            .expect("step should succeed");
        if flags.frame_completed {
            frame_seen = true;
        }
    }
    assert!(frame_seen);
    assert_eq!(crt.raster_frame_count(), 1);
    assert_eq!(crt.raster_beam_x(), 0);
    assert_eq!(crt.raster_beam_y(), 0);
}

#[test]
fn test_raster_lowering_to_pixel_buffer() {
    let timing = RasterTiming {
        visible_width: 2,
        visible_height: 2,
        h_front_porch: 1,
        h_sync: 1,
        h_back_porch: 1,
        v_front_porch: 1,
        v_sync: 1,
        v_back_porch: 1,
    };
    let crt = CrtRasterBeam::new_raster_beam(timing, PhosphorType::P4White)
        .expect("crt should succeed");

    let phosphor = [65535u16, 32768u16, 0u16, 65535u16];
    let mut pixels = PixelContainer::new(2, 2);

    crt.lower_raster_to_pixel_buffer(&phosphor, &mut pixels)
        .expect("lower to pixels should succeed");

    let p00 = pixels.pixel_at(0, 0);
    assert_eq!(p00, (240, 245, 255, 255));

    let p10 = pixels.pixel_at(1, 0);
    assert_eq!(p10.3, 255);
    assert!(p10.0 > 100 && p10.0 < 200);

    let p01 = pixels.pixel_at(0, 1);
    assert_eq!(p01, (0, 0, 0, 255));
}

#[test]
fn test_raster_lowering_to_paint_scene() {
    let timing = RasterTiming {
        visible_width: 4,
        visible_height: 2,
        h_front_porch: 1,
        h_sync: 1,
        h_back_porch: 1,
        v_front_porch: 1,
        v_sync: 1,
        v_back_porch: 1,
    };
    let crt = CrtRasterBeam::new_raster_beam(timing, PhosphorType::P31Green)
        .expect("crt should succeed");

    let phosphor = [
        60000u16, 60000u16, 0u16, 0u16, // Line 0: 2 lit pixels
        0u16, 60000u16, 60000u16, 60000u16, // Line 1: 3 lit pixels
    ];

    let scene = crt
        .lower_raster_to_paint_scene(&phosphor, 1000)
        .expect("scene lowering should succeed");

    assert_eq!(scene.width, 4.0);
    assert_eq!(scene.height, 2.0);
    assert_eq!(scene.instructions.len(), 2);

    if let PaintInstruction::Rect(r) = &scene.instructions[0] {
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.width, 2.0);
        assert_eq!(r.height, 1.0);
    } else {
        panic!("expected PaintInstruction::Rect");
    }
}

#[test]
fn test_vector_beam_display_construction_and_validation() {
    let disp = VectorBeamDisplay::new_vector_display(1024.0, 768.0, 50.0, PhosphorType::P31Green);
    assert!(disp.is_ok());

    let invalid_dim =
        VectorBeamDisplay::new_vector_display(0.0, 768.0, 50.0, PhosphorType::P31Green);
    assert!(matches!(invalid_dim, Err(CrtError::InvalidDimensions { .. })));

    let invalid_slew =
        VectorBeamDisplay::new_vector_display(1024.0, 768.0, -5.0, PhosphorType::P31Green);
    assert!(matches!(invalid_slew, Err(CrtError::InvalidSlewRate(_))));

    assert!(validate_slew_rate(100.0).is_ok());
    assert!(validate_slew_rate(0.0).is_err());
    assert!(validate_slew_rate(-1.0).is_err());
    assert!(validate_slew_rate(f64::NAN).is_err());
}

#[test]
fn test_vector_beam_amplifier_slewing_and_blanking() {
    let mut disp =
        VectorBeamDisplay::new_vector_display(1000.0, 1000.0, 10.0, PhosphorType::P1Green)
            .expect("vector display should succeed");

    assert_eq!(disp.vector_beam_x(), 0.0);
    assert_eq!(disp.vector_beam_y(), 0.0);

    // Target (100.0, 0.0): distance 100.0 > slew rate 10.0
    let pt1 = disp
        .step_vector_beam(100.0, 0.0, false, 255)
        .expect("step should succeed");
    assert!(!pt1.settled);
    assert_eq!(pt1.x, 10.0);
    assert_eq!(pt1.y, 0.0);
    assert!(!pt1.blanked);
    assert_eq!(pt1.intensity, 255);

    // Step with blanking
    let pt2 = disp
        .step_vector_beam(10.0, 0.0, true, 255)
        .expect("step should succeed");
    assert!(pt2.settled);
    assert!(pt2.blanked);
    assert_eq!(pt2.intensity, 0);

    // Non-finite coordinate rejection
    assert!(matches!(
        disp.step_vector_beam(f64::NAN, 50.0, false, 255),
        Err(CrtError::NonFiniteCoordinate { .. })
    ));
}

#[test]
fn test_vector_stroke_rasterization_zero_alloc() {
    let mut disp =
        VectorBeamDisplay::new_vector_display(100.0, 100.0, 20.0, PhosphorType::P31Green)
            .expect("vector display should succeed");

    let mut phosphor = [0u16; 100 * 100];

    // Horizontal stroke from (10, 20) to (50, 20)
    let stroke = VectorStroke::new_vector_stroke(10.0, 20.0, 50.0, 20.0, 200);
    let excited = disp
        .rasterize_vector_stroke_into(&stroke, &mut phosphor, 100, 100)
        .expect("rasterize should succeed");

    assert!(excited > 30);
    // Spot check pixels along line y=20
    assert!(phosphor[20 * 100 + 10] > 0);
    assert!(phosphor[20 * 100 + 30] > 0);
    assert!(phosphor[20 * 100 + 50] > 0);
    assert_eq!(phosphor[20 * 100 + 60], 0);

    // Dwell spot
    let dwell = VectorStroke::new_dwell_spot(75.0, 75.0, 200, 5);
    let dwell_excited = disp
        .rasterize_vector_stroke_into(&dwell, &mut phosphor, 100, 100)
        .expect("dwell rasterize should succeed");
    assert_eq!(dwell_excited, 1);
    assert!(phosphor[75 * 100 + 75] > 0);

    // Blanked transit (intensity 0)
    let blanked = VectorStroke::new_vector_stroke(0.0, 0.0, 10.0, 10.0, 0);
    let blanked_excited = disp
        .rasterize_vector_stroke_into(&blanked, &mut phosphor, 100, 100)
        .expect("blanked should succeed");
    assert_eq!(blanked_excited, 0);
}

#[test]
fn test_vector_strokes_lowering_to_paint_scene() {
    let strokes = [
        VectorStroke::new_vector_stroke(10.0, 10.0, 90.0, 90.0, 255),
        VectorStroke::new_dwell_spot(50.0, 50.0, 255, 10),
        VectorStroke::new_vector_stroke(0.0, 0.0, 5.0, 5.0, 0), // Blanked, should be skipped
    ];

    let scene = VectorBeamDisplay::lower_vector_strokes_to_paint_scene(
        &strokes,
        100.0,
        100.0,
        PhosphorType::P31Green,
    )
    .expect("lowering should succeed");

    assert_eq!(scene.width, 100.0);
    assert_eq!(scene.height, 100.0);
    assert_eq!(scene.instructions.len(), 2);

    match &scene.instructions[0] {
        PaintInstruction::Line(l) => {
            assert_eq!(l.x1, 10.0);
            assert_eq!(l.y1, 10.0);
            assert_eq!(l.x2, 90.0);
            assert_eq!(l.y2, 90.0);
        }
        _ => panic!("expected PaintInstruction::Line"),
    }

    match &scene.instructions[1] {
        PaintInstruction::Ellipse(e) => {
            assert_eq!(e.cx, 50.0);
            assert_eq!(e.cy, 50.0);
        }
        _ => panic!("expected PaintInstruction::Ellipse"),
    }
}
