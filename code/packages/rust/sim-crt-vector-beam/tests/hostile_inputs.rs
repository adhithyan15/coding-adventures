//! Hostile-input and deterministic fuzz test suite for `sim-crt-vector-beam`.

#![forbid(unsafe_code)]

use sim_crt_vector_beam::{
    accumulate_intensity, decay_phosphor_buffer, decay_phosphor_sample, validate_raster_timing,
    validate_slew_rate, CrtError, CrtRasterBeam, PhosphorType, PixelContainer, RasterTiming,
    VectorBeamDisplay, VectorStroke,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_hostile_f64(&mut self, idx: usize) -> f64 {
        match idx % 10 {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => -0.0,
            4 => f64::from_bits(1), // subnormal
            5 => 1.0e20,
            6 => -1.0e20,
            7 => f64::MAX,
            8 => f64::MIN,
            _ => {
                let raw = (self.next_u64() as i64) as f64;
                raw / 1.0e14
            }
        }
    }
}

#[test]
fn hostile_raster_beam_u32_max_and_invalid_timings() {
    let timing = RasterTiming::standard_ntsc();
    let mut beam = CrtRasterBeam::new_raster_beam(timing, PhosphorType::P4White).unwrap();
    let mut phosphor = vec![0u16; timing.total_raster_pixels()];

    // Mutated public beam_x / beam_y = u32::MAX must not panic with overflow
    beam.beam_x = u32::MAX;
    beam.beam_y = u32::MAX;
    let flags = beam.step_raster_beam(255, &mut phosphor).unwrap();
    assert!(flags.line_completed);
    assert!(flags.frame_completed);
    assert_eq!(beam.beam_x, 0);
    assert_eq!(beam.beam_y, 0);

    // Mutated timing overflow must return Err rather than panicking
    beam.timing.visible_width = u32::MAX;
    beam.timing.h_front_porch = u32::MAX;
    assert!(beam.step_raster_beam(128, &mut phosphor).is_err());
    assert!(beam.lower_raster_to_paint_scene(&phosphor, 1).is_err());

    let mut pixels = PixelContainer::new(2, 2);
    assert!(beam
        .lower_raster_to_pixel_buffer(&phosphor, &mut pixels)
        .is_err());

    // Hostile RasterTiming configurations
    let bad_timings = [
        RasterTiming {
            visible_width: 0,
            ..timing
        },
        RasterTiming {
            visible_height: 0,
            ..timing
        },
        RasterTiming {
            visible_width: 4097,
            ..timing
        },
        RasterTiming {
            h_front_porch: u32::MAX,
            ..timing
        },
        RasterTiming {
            v_sync: u32::MAX,
            ..timing
        },
    ];
    for t in &bad_timings {
        assert!(validate_raster_timing(*t).is_err());
        let _ = t.total_raster_width();
        let _ = t.total_raster_height();
        let _ = t.total_raster_pixels();
    }
}

#[test]
fn hostile_truncated_phosphor_buffers() {
    let timing = RasterTiming::standard_arcade();
    let mut beam = CrtRasterBeam::new_raster_beam(timing, PhosphorType::P31Green).unwrap();
    let mut tiny = [0u16; 3];
    beam.beam_x = 3;
    beam.beam_y = 0;
    assert!(matches!(
        beam.step_raster_beam(255, &mut tiny),
        Err(CrtError::BufferTooSmall { .. })
    ));
    assert!(matches!(
        beam.lower_raster_to_paint_scene(&tiny, 1),
        Err(CrtError::BufferTooSmall { .. })
    ));
    let mut pixels = PixelContainer::new(timing.visible_width, timing.visible_height);
    assert!(matches!(
        beam.lower_raster_to_pixel_buffer(&tiny, &mut pixels),
        Err(CrtError::BufferTooSmall { .. })
    ));

    let mut disp =
        VectorBeamDisplay::new_vector_display(256.0, 224.0, 50.0, PhosphorType::P31Green).unwrap();
    let stroke = VectorStroke::new_vector_stroke(10.0, 10.0, 20.0, 20.0, 200);
    assert!(matches!(
        disp.rasterize_vector_stroke_into(&stroke, &mut tiny, 256, 224),
        Err(CrtError::BufferTooSmall { .. })
    ));
    assert!(matches!(
        disp.rasterize_vector_stroke_into(&stroke, &mut tiny, u32::MAX, u32::MAX),
        Err(CrtError::InvalidDimensions { .. } | CrtError::BufferTooSmall { .. })
    ));
    assert_eq!(disp.current_x, 0.0);
    assert_eq!(disp.current_y, 0.0);
}

#[test]
fn hostile_vector_strokes_preserve_atomic_state() {
    let mut disp =
        VectorBeamDisplay::new_vector_display(256.0, 256.0, 25.0, PhosphorType::P1Green).unwrap();
    disp.current_x = 12.5;
    disp.current_y = 34.5;
    disp.z_blanked = false;
    disp.current_intensity = 177;

    let mut phosphor = vec![42u16; 64 * 64];
    let original_phosphor = phosphor.clone();

    let hostile_coords = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e20,
        -1.0e20,
        f64::MAX,
        f64::MIN,
    ];

    for &bad in &hostile_coords {
        let strokes = [
            VectorStroke::new_vector_stroke(bad, 10.0, 20.0, 20.0, 200),
            VectorStroke::new_vector_stroke(10.0, bad, 20.0, 20.0, 200),
            VectorStroke::new_vector_stroke(10.0, 10.0, bad, 20.0, 200),
            VectorStroke::new_vector_stroke(10.0, 10.0, 20.0, bad, 200),
            VectorStroke::new_vector_stroke(bad, 10.0, 20.0, 20.0, 0),
            VectorStroke::new_vector_stroke(10.0, 10.0, bad, 20.0, 0),
        ];
        for s in &strokes {
            let res = disp.rasterize_vector_stroke_into(s, &mut phosphor, 64, 64);
            assert!(res.is_err(), "expected Err for hostile stroke {s:?}");
            assert_eq!(disp.current_x, 12.5);
            assert_eq!(disp.current_y, 34.5);
            assert!(!disp.z_blanked);
            assert_eq!(disp.current_intensity, 177);
            assert_eq!(phosphor, original_phosphor);
        }

        assert!(disp.step_vector_beam(bad, 10.0, false, 200).is_err());
        assert!(disp.step_vector_beam(10.0, bad, false, 200).is_err());
        assert_eq!(disp.current_x, 12.5);
        assert_eq!(disp.current_y, 34.5);
    }

    // Mutated internal state in VectorBeamDisplay must be rejected without panic
    let mut corrupted = disp.clone();
    corrupted.current_x = f64::NAN;
    assert!(corrupted.step_vector_beam(10.0, 10.0, false, 200).is_err());

    let mut corrupted_oob = disp.clone();
    corrupted_oob.current_y = 1.0e20;
    assert!(corrupted_oob
        .step_vector_beam(10.0, 10.0, false, 200)
        .is_err());

    let mut corrupted_slew = disp.clone();
    corrupted_slew.max_slew_rate = 0.0;
    assert!(corrupted_slew
        .step_vector_beam(10.0, 10.0, false, 200)
        .is_err());
    corrupted_slew.max_slew_rate = f64::NAN;
    assert!(corrupted_slew
        .step_vector_beam(10.0, 10.0, false, 200)
        .is_err());

    // Subnormal and -0.0 coordinates are finite and within bounds, must not panic
    for &val in &[-0.0_f64, f64::from_bits(1), -f64::from_bits(1)] {
        let s = VectorStroke::new_vector_stroke(val, val, val, val, 128);
        assert!(disp
            .rasterize_vector_stroke_into(&s, &mut phosphor, 64, 64)
            .is_ok());
        assert!(disp.step_vector_beam(val, val, false, 128).is_ok());
    }
}

#[test]
fn hostile_fuzz_campaign_1000_cases() {
    let mut rng = SplitMix64(0xC47B_EA40_0000_0001);
    let mut raster =
        CrtRasterBeam::new_raster_beam(RasterTiming::standard_arcade(), PhosphorType::P4White)
            .unwrap();
    let mut phosphor = vec![0u16; 64 * 64];
    let mut disp =
        VectorBeamDisplay::new_vector_display(64.0, 64.0, 10.0, PhosphorType::P31Green).unwrap();

    for i in 0..1_000 {
        let x0 = rng.next_hostile_f64(i);
        let y0 = rng.next_hostile_f64(i + 1);
        let x1 = rng.next_hostile_f64(i + 2);
        let y1 = rng.next_hostile_f64(i + 3);
        let intensity = (rng.next_u32() & 0xFF) as u8;
        let dwell = rng.next_u32();
        let stroke = VectorStroke {
            x0,
            y0,
            x1,
            y1,
            intensity,
            dwell_ticks: dwell,
        };
        let bw = match i % 5 {
            0 => 0,
            1 => u32::MAX,
            _ => 64,
        };
        let bh = match i % 7 {
            0 => 0,
            1 => u32::MAX,
            _ => 64,
        };
        let prev_x = disp.current_x;
        let prev_y = disp.current_y;
        let res = disp.rasterize_vector_stroke_into(&stroke, &mut phosphor, bw, bh);
        if res.is_err() {
            assert_eq!(disp.current_x, prev_x);
            assert_eq!(disp.current_y, prev_y);
        }

        let _ = disp.step_vector_beam(x0, y0, (i & 1) == 0, intensity);
        let _ = validate_slew_rate(x1);
        let _ = VectorBeamDisplay::lower_vector_strokes_to_paint_scene(
            &[stroke],
            x0,
            y0,
            PhosphorType::Custom {
                decay_q16: rng.next_u32(),
                tint: [255, 255, 255],
            },
        );

        raster.beam_x = if i % 9 == 0 { u32::MAX } else { rng.next_u32() };
        raster.beam_y = if i % 11 == 0 {
            u32::MAX
        } else {
            rng.next_u32()
        };
        let _ = raster.step_raster_beam(intensity, &mut phosphor);

        let _ = decay_phosphor_sample((rng.next_u32() & 0xFFFF) as u16, rng.next_u32());
        let _ = accumulate_intensity(
            (rng.next_u32() & 0xFFFF) as u16,
            (rng.next_u32() & 0xFFFF) as u16,
        );
        let _ = decay_phosphor_buffer(&mut phosphor[..16], rng.next_u32());
    }
}
