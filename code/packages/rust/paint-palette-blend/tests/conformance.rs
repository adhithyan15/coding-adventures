//! # Unit and conformance tests for `paint-palette-blend`
//!
//! Covers blend arithmetic, indexed scanline/image blits (flip, clip, stride, color-key),
//! pixel-container integration, scene diffing, and CWE-20/125/190/369/400/787 guards.

use paint_palette_blend::{
    alpha_from_unit_f32, blend_mode_from_code, blend_mode_from_paint, blend_pixel,
    blend_rgba_scanline, blend_solid_span, blit_indexed_image, blit_indexed_scanline,
    blit_indexed_to_pixel_container, diff_paint_scene_into, instruction_id, BlendMode, BlitParams,
    IndexedImage, PaintBase, PaintInstruction, PaintScene, PaletteBlendError, PatchKind, PatchOp,
    PixelContainer, Rgba8, RgbaTarget, MAX_DIMENSION, MAX_PALETTE_ENTRIES, MAX_SCENE_INSTRUCTIONS,
    NO_INDEX, OPAQUE_BLIT,
};

const PALETTE: [Rgba8; 4] = [
    [0, 0, 0, 0],
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 128],
];

fn keyed(mode: BlendMode, alpha: u8) -> BlitParams {
    BlitParams {
        mode,
        alpha,
        transparent_index: Some(0),
        h_flip: false,
        v_flip: false,
    }
}

// ---------------------------------------------------------------------------
// Blend arithmetic
// ---------------------------------------------------------------------------

#[test]
fn normal_replaces_destination() {
    let out = blend_pixel(BlendMode::Normal, [1, 2, 3, 4], [9, 9, 9, 9], 0);
    assert_eq!(out, [1, 2, 3, 4]);
}

#[test]
fn add_saturates() {
    let out = blend_pixel(
        BlendMode::Add,
        [200, 100, 0, 255],
        [100, 100, 100, 255],
        255,
    );
    assert_eq!(out, [255, 200, 100, 255]);
}

#[test]
fn multiply_known_values() {
    let out = blend_pixel(
        BlendMode::Multiply,
        [255, 128, 0, 255],
        [100, 100, 100, 255],
        255,
    );
    assert_eq!(out, [100, 50, 0, 255]);
    // Multiply by white is identity.
    let id = blend_pixel(BlendMode::Multiply, [255; 4], [10, 20, 30, 255], 255);
    assert_eq!(id, [10, 20, 30, 255]);
}

#[test]
fn screen_known_values() {
    // Screen with black is identity; with white is white.
    assert_eq!(
        blend_pixel(BlendMode::Screen, [0, 0, 0, 255], [10, 20, 30, 255], 255),
        [10, 20, 30, 255]
    );
    assert_eq!(
        blend_pixel(
            BlendMode::Screen,
            [255, 255, 255, 255],
            [10, 20, 30, 255],
            255
        ),
        [255, 255, 255, 255]
    );
}

#[test]
fn alpha_blend_half() {
    let out = blend_pixel(BlendMode::Alpha, [255, 0, 0, 255], [0, 0, 255, 255], 128);
    assert_eq!(out, [128, 0, 127, 255]);
}

#[test]
fn zero_effective_alpha_returns_destination() {
    for mode in [
        BlendMode::Add,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Alpha,
    ] {
        let dst = [5, 6, 7, 8];
        assert_eq!(blend_pixel(mode, [200, 200, 200, 0], dst, 255), dst);
        assert_eq!(blend_pixel(mode, [200, 200, 200, 255], dst, 0), dst);
    }
}

#[test]
fn blend_never_panics_exhaustive_corners() {
    let vals = [0u8, 1, 127, 128, 254, 255];
    for mode in [
        BlendMode::Normal,
        BlendMode::Add,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Alpha,
    ] {
        for &a in &vals {
            for &b in &vals {
                for &c in &vals {
                    for &d in &vals {
                        let _ = blend_pixel(mode, [a, b, c, d], [d, c, b, a], b);
                    }
                }
            }
        }
    }
}

#[test]
fn alpha_result_is_monotonic_union() {
    for ea in 0..=255u8 {
        let out = blend_pixel(BlendMode::Alpha, [10, 10, 10, ea], [0, 0, 0, 255], 255);
        assert_eq!(out[3], 255);
    }
}

#[test]
fn determinism() {
    let a = blend_pixel(BlendMode::Screen, [77, 88, 99, 200], [11, 22, 33, 255], 190);
    let b = blend_pixel(BlendMode::Screen, [77, 88, 99, 200], [11, 22, 33, 255], 190);
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// Input conversion
// ---------------------------------------------------------------------------

#[test]
fn blend_mode_codes() {
    assert_eq!(blend_mode_from_code(0), Ok(BlendMode::Normal));
    assert_eq!(blend_mode_from_code(1), Ok(BlendMode::Add));
    assert_eq!(blend_mode_from_code(2), Ok(BlendMode::Multiply));
    assert_eq!(blend_mode_from_code(3), Ok(BlendMode::Screen));
    assert_eq!(blend_mode_from_code(4), Ok(BlendMode::Alpha));
    assert_eq!(
        blend_mode_from_code(5),
        Err(PaletteBlendError::InvalidBlendMode(5))
    );
    assert_eq!(
        blend_mode_from_code(255),
        Err(PaletteBlendError::InvalidBlendMode(255))
    );
}

#[test]
fn paint_blend_mode_mapping() {
    use paint_instructions::BlendMode as PB;
    assert_eq!(blend_mode_from_paint(&PB::Normal), Some(BlendMode::Normal));
    assert_eq!(
        blend_mode_from_paint(&PB::Multiply),
        Some(BlendMode::Multiply)
    );
    assert_eq!(blend_mode_from_paint(&PB::Screen), Some(BlendMode::Screen));
    assert_eq!(blend_mode_from_paint(&PB::Overlay), None);
}

#[test]
fn alpha_conversion_validates() {
    assert_eq!(alpha_from_unit_f32(0.0), Ok(0));
    assert_eq!(alpha_from_unit_f32(1.0), Ok(255));
    assert_eq!(alpha_from_unit_f32(0.5), Ok(128));
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
        assert_eq!(
            alpha_from_unit_f32(bad),
            Err(PaletteBlendError::InvalidAlpha)
        );
    }
}

// ---------------------------------------------------------------------------
// Scanline blits
// ---------------------------------------------------------------------------

#[test]
fn scanline_normal_with_color_key() {
    let indices = [0u8, 1, 2, 3];
    let mut dst = [9u8; 16];
    let drawn = blit_indexed_scanline(&mut dst, &indices, &PALETTE, &keyed(BlendMode::Normal, 255))
        .unwrap();
    assert_eq!(drawn, 3);
    assert_eq!(&dst[0..4], &[9, 9, 9, 9], "color-keyed pixel untouched");
    assert_eq!(&dst[4..8], &[255, 0, 0, 255]);
    assert_eq!(&dst[8..12], &[0, 255, 0, 255]);
    assert_eq!(&dst[12..16], &[0, 0, 255, 128]);
}

#[test]
fn scanline_h_flip() {
    let indices = [1u8, 2];
    let mut dst = [0u8; 8];
    let params = BlitParams {
        h_flip: true,
        ..OPAQUE_BLIT
    };
    blit_indexed_scanline(&mut dst, &indices, &PALETTE, &params).unwrap();
    assert_eq!(&dst[0..4], &[0, 255, 0, 255]);
    assert_eq!(&dst[4..8], &[255, 0, 0, 255]);
}

#[test]
fn scanline_alpha_blend_over_opaque() {
    let indices = [3u8];
    let mut dst = [0, 0, 0, 255];
    blit_indexed_scanline(&mut dst, &indices, &PALETTE, &keyed(BlendMode::Alpha, 255)).unwrap();
    assert_eq!(dst, [0, 0, 128, 255]);
}

#[test]
fn scanline_only_writes_prefix() {
    let indices = [1u8];
    let mut dst = [7u8; 12];
    blit_indexed_scanline(&mut dst, &indices, &PALETTE, &OPAQUE_BLIT).unwrap();
    assert_eq!(&dst[4..], &[7u8; 8]);
}

#[test]
fn scanline_empty_is_ok() {
    let mut dst = [];
    assert_eq!(
        blit_indexed_scanline(&mut dst, &[], &PALETTE, &OPAQUE_BLIT),
        Ok(0)
    );
}

#[test]
fn scanline_rejects_bad_index_without_writing() {
    let indices = [1u8, 2, 200];
    let mut dst = [9u8; 12];
    let err = blit_indexed_scanline(&mut dst, &indices, &PALETTE, &OPAQUE_BLIT).unwrap_err();
    assert_eq!(
        err,
        PaletteBlendError::PaletteIndexOutOfRange {
            index: 200,
            palette_len: 4
        }
    );
    assert_eq!(dst, [9u8; 12], "CWE-787: failed blit must not mutate dst");
}

#[test]
fn scanline_rejects_short_destination() {
    let indices = [1u8, 1, 1];
    let mut dst = [0u8; 11];
    assert_eq!(
        blit_indexed_scanline(&mut dst, &indices, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::BufferTooSmall {
            required: 12,
            provided: 11
        })
    );
}

#[test]
fn scanline_rejects_bad_palettes() {
    let mut dst = [0u8; 4];
    assert_eq!(
        blit_indexed_scanline(&mut dst, &[0], &[], &OPAQUE_BLIT),
        Err(PaletteBlendError::EmptyPalette)
    );
    let big = vec![[0u8; 4]; MAX_PALETTE_ENTRIES + 1];
    assert_eq!(
        blit_indexed_scanline(&mut dst, &[0], &big, &OPAQUE_BLIT),
        Err(PaletteBlendError::PaletteTooLarge {
            len: MAX_PALETTE_ENTRIES + 1,
            max: MAX_PALETTE_ENTRIES
        })
    );
    let full = vec![[1u8; 4]; MAX_PALETTE_ENTRIES];
    assert_eq!(
        blit_indexed_scanline(&mut dst, &[255], &full, &OPAQUE_BLIT),
        Ok(1)
    );
}

#[test]
fn scanline_rejects_oversized_run() {
    let indices = vec![0u8; MAX_DIMENSION as usize + 1];
    let mut dst = vec![0u8; (MAX_DIMENSION as usize + 1) * 4];
    assert!(matches!(
        blit_indexed_scanline(&mut dst, &indices, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::InvalidDimensions { .. })
    ));
}

#[test]
fn solid_span_and_rgba_scanline() {
    let mut dst = [10u8, 10, 10, 255, 20, 20, 20, 255];
    assert_eq!(
        blend_solid_span(&mut dst, [255, 255, 255, 255], BlendMode::Multiply, 255),
        Ok(2)
    );
    assert_eq!(dst, [10, 10, 10, 255, 20, 20, 20, 255]);

    let src = [100u8, 100, 100, 255, 100, 100, 100, 255];
    assert_eq!(
        blend_rgba_scanline(&mut dst, &src, BlendMode::Add, 255),
        Ok(2)
    );
    assert_eq!(dst, [110, 110, 110, 255, 120, 120, 120, 255]);
}

#[test]
fn span_validation() {
    let mut odd = [0u8; 5];
    assert_eq!(
        blend_solid_span(&mut odd, [0; 4], BlendMode::Add, 255),
        Err(PaletteBlendError::MisalignedSpan { len: 5 })
    );
    let mut a = [0u8; 8];
    let b = [0u8; 4];
    assert_eq!(
        blend_rgba_scanline(&mut a, &b, BlendMode::Add, 255),
        Err(PaletteBlendError::LengthMismatch { dst: 8, src: 4 })
    );
    let mut c = [0u8; 6];
    let d = [0u8; 6];
    assert_eq!(
        blend_rgba_scanline(&mut c, &d, BlendMode::Add, 255),
        Err(PaletteBlendError::MisalignedSpan { len: 6 })
    );
    let mut huge = vec![0u8; (MAX_DIMENSION as usize + 1) * 4];
    assert!(blend_solid_span(&mut huge, [0; 4], BlendMode::Add, 255).is_err());
    let huge_src = vec![0u8; huge.len()];
    assert!(blend_rgba_scanline(&mut huge, &huge_src, BlendMode::Add, 255).is_err());
}

// ---------------------------------------------------------------------------
// Image blits
// ---------------------------------------------------------------------------

fn target_of(w: u32, h: u32, buf: &mut [u8]) -> RgbaTarget<'_> {
    RgbaTarget {
        width: w,
        height: h,
        stride: w as usize * 4,
        pixels: buf,
    }
}

/// 3x2 source: row0 = 1 2 1 ; row1 = 2 1 2
const SRC_PIXELS: [u8; 6] = [1, 2, 1, 2, 1, 2];

fn src_image() -> IndexedImage<'static> {
    IndexedImage {
        width: 3,
        height: 2,
        stride: 3,
        pixels: &SRC_PIXELS,
    }
}

fn px(buf: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
    let o = (y * w + x) * 4;
    [buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]
}

#[test]
fn image_blit_basic() {
    let mut buf = vec![0u8; 5 * 4 * 4];
    let mut t = target_of(5, 4, &mut buf);
    let drawn = blit_indexed_image(&mut t, 1, 1, &src_image(), &PALETTE, &OPAQUE_BLIT).unwrap();
    assert_eq!(drawn, 6);
    assert_eq!(px(&buf, 5, 1, 1), [255, 0, 0, 255]);
    assert_eq!(px(&buf, 5, 2, 1), [0, 255, 0, 255]);
    assert_eq!(px(&buf, 5, 1, 2), [0, 255, 0, 255]);
    assert_eq!(px(&buf, 5, 0, 0), [0, 0, 0, 0]);
    assert_eq!(px(&buf, 5, 4, 3), [0, 0, 0, 0]);
}

#[test]
fn image_blit_flips() {
    let mut buf = vec![0u8; 3 * 2 * 4];
    let params = BlitParams {
        h_flip: true,
        v_flip: true,
        ..OPAQUE_BLIT
    };
    let mut t = target_of(3, 2, &mut buf);
    blit_indexed_image(&mut t, 0, 0, &src_image(), &PALETTE, &params).unwrap();
    // Source rotated 180: row0 = 2 1 2 reversed -> (2,1,2), row1 -> (1,2,1)
    assert_eq!(px(&buf, 3, 0, 0), [0, 255, 0, 255]);
    assert_eq!(px(&buf, 3, 1, 0), [255, 0, 0, 255]);
    assert_eq!(px(&buf, 3, 0, 1), [255, 0, 0, 255]);
    assert_eq!(px(&buf, 3, 2, 1), [255, 0, 0, 255]);
}

#[test]
fn image_blit_clips_all_edges() {
    // Distinct pixel values so clipping offsets are verifiable.
    let pixels = [1u8, 2, 3, 1, 2, 3, 1, 2, 3];
    let img = IndexedImage {
        width: 3,
        height: 3,
        stride: 3,
        pixels: &pixels,
    };
    for (x, y) in [
        (-1, -1),
        (-2, 0),
        (2, 2),
        (1, -2),
        (-5, -5),
        (50, 50),
        (0, 9),
    ] {
        for (hf, vf) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut buf = vec![0u8; 3 * 3 * 4];
            let params = BlitParams {
                h_flip: hf,
                v_flip: vf,
                ..OPAQUE_BLIT
            };
            let mut t = target_of(3, 3, &mut buf);
            let drawn = blit_indexed_image(&mut t, x, y, &img, &PALETTE, &params).unwrap();
            // Reference: per-pixel
            let mut expect = vec![0u8; 3 * 3 * 4];
            let mut expect_drawn = 0;
            for sy in 0..3i32 {
                for sx in 0..3i32 {
                    let (dx, dy) = (x + sx, y + sy);
                    if !(0..3).contains(&dx) || !(0..3).contains(&dy) {
                        continue;
                    }
                    let ux = if hf { 2 - sx } else { sx } as usize;
                    let uy = if vf { 2 - sy } else { sy } as usize;
                    let c = PALETTE[pixels[uy * 3 + ux] as usize];
                    let o = (dy as usize * 3 + dx as usize) * 4;
                    expect[o..o + 4].copy_from_slice(&c);
                    expect_drawn += 1;
                }
            }
            assert_eq!(buf, expect, "x={x} y={y} hf={hf} vf={vf}");
            assert_eq!(drawn, expect_drawn);
        }
    }
}

#[test]
fn image_blit_extreme_offsets_do_not_overflow() {
    let mut buf = vec![0u8; 16];
    let mut t = target_of(2, 2, &mut buf);
    for (x, y) in [
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, 0),
        (0, i32::MAX),
    ] {
        assert_eq!(
            blit_indexed_image(&mut t, x, y, &src_image(), &PALETTE, &OPAQUE_BLIT),
            Ok(0)
        );
    }
    assert_eq!(buf, vec![0u8; 16]);
}

#[test]
fn image_blit_honors_strides() {
    // Source stride 5 with padding bytes 255 (invalid index) that must never be read.
    let pixels = [1u8, 2, 255, 255, 255, 2, 1, 255, 255, 255];
    let img = IndexedImage {
        width: 2,
        height: 2,
        stride: 5,
        pixels: &pixels[..8],
    };
    // Target stride 40 bytes (10 px) for a 3x2 viewport.
    let mut buf = vec![0xEEu8; 40 + 12];
    let mut t = RgbaTarget {
        width: 3,
        height: 2,
        stride: 40,
        pixels: &mut buf,
    };
    blit_indexed_image(&mut t, 0, 0, &img, &PALETTE, &OPAQUE_BLIT).unwrap();
    assert_eq!(&buf[0..4], &[255, 0, 0, 255]);
    assert_eq!(&buf[4..8], &[0, 255, 0, 255]);
    assert_eq!(
        &buf[8..12],
        &[0xEE; 4],
        "column outside the image untouched"
    );
    assert_eq!(&buf[12..40], &[0xEEu8; 28], "stride padding untouched");
    assert_eq!(&buf[40..44], &[0, 255, 0, 255]);
    assert_eq!(&buf[44..48], &[255, 0, 0, 255]);
}

#[test]
fn image_blit_bad_index_leaves_target_untouched() {
    let pixels = [1u8, 1, 1, 1, 1, 9];
    let img = IndexedImage {
        width: 3,
        height: 2,
        stride: 3,
        pixels: &pixels,
    };
    let mut buf = vec![7u8; 3 * 2 * 4];
    let mut t = target_of(3, 2, &mut buf);
    let err = blit_indexed_image(&mut t, 0, 0, &img, &PALETTE, &OPAQUE_BLIT).unwrap_err();
    assert!(matches!(
        err,
        PaletteBlendError::PaletteIndexOutOfRange { index: 9, .. }
    ));
    assert_eq!(buf, vec![7u8; 24]);
}

#[test]
fn image_blit_invalid_index_outside_clip_is_ignored() {
    let pixels = [1u8, 99];
    let img = IndexedImage {
        width: 2,
        height: 1,
        stride: 2,
        pixels: &pixels,
    };
    let mut buf = vec![0u8; 4];
    let mut t = target_of(1, 1, &mut buf);
    assert_eq!(
        blit_indexed_image(&mut t, 0, 0, &img, &PALETTE, &OPAQUE_BLIT),
        Ok(1)
    );
}

#[test]
fn image_blit_geometry_validation() {
    let mut buf = vec![0u8; 64];
    let good = [1u8; 4];
    let mut t = target_of(4, 4, &mut buf);

    for (w, h) in [
        (0u32, 1u32),
        (1, 0),
        (MAX_DIMENSION + 1, 1),
        (1, MAX_DIMENSION + 1),
    ] {
        let img = IndexedImage {
            width: w,
            height: h,
            stride: w as usize,
            pixels: &good,
        };
        assert!(matches!(
            blit_indexed_image(&mut t, 0, 0, &img, &PALETTE, &OPAQUE_BLIT),
            Err(PaletteBlendError::InvalidDimensions { .. })
        ));
    }

    let small_stride = IndexedImage {
        width: 2,
        height: 2,
        stride: 1,
        pixels: &good,
    };
    assert_eq!(
        blit_indexed_image(&mut t, 0, 0, &small_stride, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::InvalidStride { stride: 1, min: 2 })
    );

    let short = IndexedImage {
        width: 2,
        height: 3,
        stride: 2,
        pixels: &good,
    };
    assert_eq!(
        blit_indexed_image(&mut t, 0, 0, &short, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::BufferTooSmall {
            required: 6,
            provided: 4
        })
    );

    // Absurd stride must be rejected, not overflow.
    let huge_stride = IndexedImage {
        width: 2,
        height: 2,
        stride: usize::MAX,
        pixels: &good,
    };
    assert_eq!(
        blit_indexed_image(&mut t, 0, 0, &huge_stride, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::ArithmeticOverflow)
    );
}

#[test]
fn target_geometry_validation() {
    let img = src_image();
    let mut small = vec![0u8; 8];
    let mut t = RgbaTarget {
        width: 4,
        height: 4,
        stride: 16,
        pixels: &mut small,
    };
    assert!(matches!(
        blit_indexed_image(&mut t, 0, 0, &img, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::BufferTooSmall { .. })
    ));

    let mut buf = vec![0u8; 64];
    let mut t = RgbaTarget {
        width: 4,
        height: 4,
        stride: 8,
        pixels: &mut buf,
    };
    assert_eq!(
        blit_indexed_image(&mut t, 0, 0, &img, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::InvalidStride { stride: 8, min: 16 })
    );

    let mut t = RgbaTarget {
        width: 0,
        height: 4,
        stride: 0,
        pixels: &mut buf,
    };
    assert!(matches!(
        blit_indexed_image(&mut t, 0, 0, &img, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::InvalidDimensions { .. })
    ));

    let mut t = RgbaTarget {
        width: 4,
        height: 4,
        stride: usize::MAX,
        pixels: &mut buf,
    };
    assert_eq!(
        blit_indexed_image(&mut t, 0, 0, &img, &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::ArithmeticOverflow)
    );
}

#[test]
fn image_blit_blend_modes_over_background() {
    let mut buf = vec![0u8; 3 * 2 * 4];
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&[100, 100, 100, 255]);
    }
    let mut t = target_of(3, 2, &mut buf);
    blit_indexed_image(
        &mut t,
        0,
        0,
        &src_image(),
        &PALETTE,
        &keyed(BlendMode::Add, 255),
    )
    .unwrap();
    assert_eq!(px(&buf, 3, 0, 0), [255, 100, 100, 255]);
    assert_eq!(px(&buf, 3, 1, 0), [100, 255, 100, 255]);
}

#[test]
fn pixel_container_integration() {
    let mut pc = PixelContainer::new(4, 4);
    let n = blit_indexed_to_pixel_container(&mut pc, 1, 1, &src_image(), &PALETTE, &OPAQUE_BLIT)
        .unwrap();
    assert_eq!(n, 6);
    assert_eq!(pc.pixel_at(1, 1), (255, 0, 0, 255));
    assert_eq!(pc.pixel_at(2, 1), (0, 255, 0, 255));
    assert_eq!(pc.pixel_at(0, 0), (0, 0, 0, 0));
}

#[test]
fn pixel_container_rejects_inconsistent_data() {
    let mut pc = PixelContainer::new(4, 4);
    pc.data.truncate(10);
    assert!(matches!(
        blit_indexed_to_pixel_container(&mut pc, 0, 0, &src_image(), &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::BufferTooSmall { .. })
    ));
    let mut zero = PixelContainer::new(0, 0);
    assert!(matches!(
        blit_indexed_to_pixel_container(&mut zero, 0, 0, &src_image(), &PALETTE, &OPAQUE_BLIT),
        Err(PaletteBlendError::InvalidDimensions { .. })
    ));
}

// ---------------------------------------------------------------------------
// Scene diffing
// ---------------------------------------------------------------------------

fn rect(id: Option<&str>, x: f64) -> PaintInstruction {
    let mut r = paint_instructions::PaintRect::filled(x, 0.0, 10.0, 10.0, "#ffffff");
    r.base = PaintBase {
        id: id.map(str::to_string),
        metadata: None,
    };
    PaintInstruction::Rect(r)
}

fn scene(items: Vec<PaintInstruction>) -> PaintScene {
    let mut s = PaintScene::new(320.0, 240.0);
    s.instructions = items;
    s
}

fn blank_ops(n: usize) -> Vec<PatchOp> {
    vec![
        PatchOp {
            kind: PatchKind::Added,
            old_index: 0,
            new_index: 0
        };
        n
    ]
}

#[test]
fn diff_identical_scenes() {
    let a = scene(vec![rect(Some("a"), 0.0), rect(Some("b"), 1.0)]);
    let mut ops = blank_ops(4);
    let s = diff_paint_scene_into(&a, &a.clone(), &mut ops).unwrap();
    assert_eq!(s.ops_written, 0);
    assert_eq!(s.unchanged, 2);
    assert!(!s.header_changed);
}

#[test]
fn diff_added_modified_removed_order() {
    let prev = scene(vec![
        rect(Some("keep"), 0.0),
        rect(Some("move"), 1.0),
        rect(Some("gone"), 2.0),
    ]);
    let next = scene(vec![
        rect(Some("keep"), 0.0),
        rect(Some("new"), 9.0),
        rect(Some("move"), 5.0),
    ]);
    let mut ops = blank_ops(8);
    let s = diff_paint_scene_into(&prev, &next, &mut ops).unwrap();
    assert_eq!(
        (s.added, s.modified, s.removed, s.unchanged, s.ops_written),
        (1, 1, 1, 1, 3)
    );
    assert_eq!(
        &ops[..3],
        &[
            PatchOp {
                kind: PatchKind::Removed,
                old_index: 2,
                new_index: NO_INDEX
            },
            PatchOp {
                kind: PatchKind::Added,
                old_index: NO_INDEX,
                new_index: 1
            },
            PatchOp {
                kind: PatchKind::Modified,
                old_index: 1,
                new_index: 2
            },
        ]
    );
}

#[test]
fn diff_header_change() {
    let prev = scene(vec![]);
    let mut next = scene(vec![]);
    next.background = "#123456".to_string();
    let mut ops = blank_ops(1);
    assert!(
        diff_paint_scene_into(&prev, &next, &mut ops)
            .unwrap()
            .header_changed
    );
    let mut wider = scene(vec![]);
    wider.width = 641.0;
    assert!(
        diff_paint_scene_into(&prev, &wider, &mut ops)
            .unwrap()
            .header_changed
    );
}

#[test]
fn diff_reports_required_capacity() {
    let prev = scene(vec![rect(Some("a"), 0.0), rect(Some("b"), 0.0)]);
    let next = scene(vec![rect(Some("c"), 0.0), rect(Some("d"), 0.0)]);
    let mut ops = blank_ops(1);
    assert_eq!(
        diff_paint_scene_into(&prev, &next, &mut ops),
        Err(PaletteBlendError::PatchBufferTooSmall {
            required: 4,
            provided: 1
        })
    );
    let mut empty: [PatchOp; 0] = [];
    assert!(matches!(
        diff_paint_scene_into(&prev, &next, &mut empty),
        Err(PaletteBlendError::PatchBufferTooSmall {
            required: 4,
            provided: 0
        })
    ));
}

#[test]
fn diff_rejects_missing_and_duplicate_ids() {
    let ok = scene(vec![rect(Some("a"), 0.0)]);
    let missing = scene(vec![rect(Some("a"), 0.0), rect(None, 0.0)]);
    let dup = scene(vec![
        rect(Some("a"), 0.0),
        rect(Some("b"), 0.0),
        rect(Some("a"), 1.0),
    ]);
    let mut ops = blank_ops(4);
    assert_eq!(
        diff_paint_scene_into(&ok, &missing, &mut ops),
        Err(PaletteBlendError::MissingInstructionId { index: 1 })
    );
    assert_eq!(
        diff_paint_scene_into(&missing, &ok, &mut ops),
        Err(PaletteBlendError::MissingInstructionId { index: 1 })
    );
    assert_eq!(
        diff_paint_scene_into(&ok, &dup, &mut ops),
        Err(PaletteBlendError::DuplicateInstructionId { index: 2 })
    );
}

#[test]
fn diff_caps_scene_size() {
    let big: Vec<PaintInstruction> = (0..=MAX_SCENE_INSTRUCTIONS)
        .map(|i| rect(Some(&format!("n{i}")), 0.0))
        .collect();
    let big = scene(big);
    let ok = scene(vec![]);
    let mut ops = blank_ops(1);
    assert_eq!(
        diff_paint_scene_into(&big, &ok, &mut ops),
        Err(PaletteBlendError::SceneTooLarge {
            count: MAX_SCENE_INSTRUCTIONS + 1,
            max: MAX_SCENE_INSTRUCTIONS
        })
    );
    assert!(matches!(
        diff_paint_scene_into(&ok, &big, &mut ops),
        Err(PaletteBlendError::SceneTooLarge { .. })
    ));
}

#[test]
fn diff_handles_reordered_scenes() {
    let prev = scene(vec![
        rect(Some("a"), 0.0),
        rect(Some("b"), 1.0),
        rect(Some("c"), 2.0),
    ]);
    let next = scene(vec![
        rect(Some("c"), 2.0),
        rect(Some("a"), 0.0),
        rect(Some("b"), 1.0),
    ]);
    let mut ops = blank_ops(4);
    let s = diff_paint_scene_into(&prev, &next, &mut ops).unwrap();
    assert_eq!(s.unchanged, 3);
    assert_eq!(s.ops_written, 0);
}

#[test]
fn diff_is_deterministic_and_applies() {
    let prev = scene(vec![rect(Some("a"), 0.0), rect(Some("b"), 1.0)]);
    let next = scene(vec![rect(Some("b"), 7.0), rect(Some("z"), 1.0)]);
    let mut o1 = blank_ops(6);
    let mut o2 = blank_ops(6);
    let s1 = diff_paint_scene_into(&prev, &next, &mut o1).unwrap();
    let s2 = diff_paint_scene_into(&prev, &next, &mut o2).unwrap();
    assert_eq!(s1, s2);
    assert_eq!(o1, o2);
    // Reconstruct next's ids from ops + unchanged: every next index is covered exactly once.
    let mut covered = [false; 2];
    for op in &o1[..s1.ops_written] {
        if op.new_index != NO_INDEX {
            covered[op.new_index as usize] = true;
        }
    }
    assert_eq!(
        covered.iter().filter(|c| **c).count(),
        s1.added + s1.modified
    );
}

#[test]
fn instruction_id_covers_variants() {
    let r = rect(Some("r"), 0.0);
    assert_eq!(instruction_id(&r), Some("r"));
    assert_eq!(instruction_id(&rect(None, 0.0)), None);
}

#[test]
fn error_display_is_informative() {
    let errs = [
        PaletteBlendError::InvalidDimensions {
            width: 0,
            height: 1,
        },
        PaletteBlendError::InvalidStride { stride: 1, min: 2 },
        PaletteBlendError::BufferTooSmall {
            required: 2,
            provided: 1,
        },
        PaletteBlendError::ArithmeticOverflow,
        PaletteBlendError::EmptyPalette,
        PaletteBlendError::PaletteTooLarge { len: 300, max: 256 },
        PaletteBlendError::PaletteIndexOutOfRange {
            index: 9,
            palette_len: 4,
        },
        PaletteBlendError::LengthMismatch { dst: 1, src: 2 },
        PaletteBlendError::MisalignedSpan { len: 3 },
        PaletteBlendError::InvalidBlendMode(9),
        PaletteBlendError::InvalidAlpha,
        PaletteBlendError::SceneTooLarge {
            count: 5000,
            max: 4096,
        },
        PaletteBlendError::MissingInstructionId { index: 1 },
        PaletteBlendError::DuplicateInstructionId { index: 2 },
        PaletteBlendError::PatchBufferTooSmall {
            required: 2,
            provided: 1,
        },
    ];
    for e in errs {
        assert!(!e.to_string().is_empty());
        let _: &dyn std::error::Error = &e;
    }
}
