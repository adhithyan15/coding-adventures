//! Hostile-input and deterministic fuzz test suite for `paint-palette-blend`.

#![forbid(unsafe_code)]

use paint_instructions::PaintRect;
use paint_palette_blend::{
    alpha_from_unit_f32, blend_mode_from_code, blend_pixel, blend_rgba_scanline, blend_solid_span,
    blit_indexed_image, blit_indexed_scanline, blit_indexed_to_pixel_container,
    diff_paint_scene_into, BlendMode, BlitParams, IndexedImage, PaintBase, PaintInstruction,
    PaintScene, PaletteBlendError, PatchOp, PixelContainer, RgbaTarget, MAX_DIMENSION,
    MAX_PALETTE_ENTRIES, MAX_SCENE_INSTRUCTIONS,
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

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() & 0xFF) as u8
    }

    fn next_hostile_f32(&mut self, idx: usize) -> f32 {
        match idx % 10 {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => f32::NEG_INFINITY,
            3 => -0.0,
            4 => f32::from_bits(1),
            5 => -f32::from_bits(1),
            6 => f32::MIN,
            7 => f32::MAX,
            8 => 1.000_001,
            _ => (self.next_u32() as f32) / (u32::MAX as f32),
        }
    }
}

fn rect_instr(id: Option<&str>, x: f64) -> PaintInstruction {
    PaintInstruction::Rect(PaintRect {
        base: PaintBase {
            id: id.map(ToString::to_string),
            metadata: None,
        },
        x,
        y: 0.0,
        width: 10.0,
        height: 10.0,
        fill: Some("#ffffff".to_string()),
        stroke: None,
        stroke_width: None,
        corner_radius: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    })
}

#[test]
fn hostile_f32_alpha_slices_and_clip_offsets() {
    for &bad in &[
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN,
        f32::MAX,
        -0.0001,
        1.0001,
        -f32::from_bits(1),
    ] {
        assert_eq!(
            alpha_from_unit_f32(bad),
            Err(PaletteBlendError::InvalidAlpha)
        );
    }
    assert_eq!(alpha_from_unit_f32(-0.0), Ok(0));
    assert_eq!(alpha_from_unit_f32(f32::from_bits(1)), Ok(0));
    assert_eq!(alpha_from_unit_f32(1.0), Ok(255));

    // Misaligned / truncated slices
    let mut odd = [0u8; 7];
    assert!(matches!(
        blend_solid_span(&mut odd, [255, 0, 0, 255], BlendMode::Alpha, 255),
        Err(PaletteBlendError::MisalignedSpan { len: 7 })
    ));
    let src_odd = [0u8; 7];
    assert!(matches!(
        blend_rgba_scanline(&mut odd, &src_odd, BlendMode::Alpha, 255),
        Err(PaletteBlendError::MisalignedSpan { len: 7 })
    ));
    let src_mismatch = [0u8; 8];
    assert!(matches!(
        blend_rgba_scanline(&mut odd, &src_mismatch, BlendMode::Alpha, 255),
        Err(PaletteBlendError::LengthMismatch { dst: 7, src: 8 })
    ));

    // Empty / oversized palette and out-of-range palette indices
    let palette = [[10u8, 20, 30, 255], [40, 50, 60, 255]];
    let mut dst = [0u8; 16];
    let bad_indices = [0u8, 1, 2, 0];
    assert!(matches!(
        blit_indexed_scanline(
            &mut dst,
            &bad_indices,
            &palette,
            &paint_palette_blend::OPAQUE_BLIT
        ),
        Err(PaletteBlendError::PaletteIndexOutOfRange {
            index: 2,
            palette_len: 2
        })
    ));
    assert_eq!(dst, [0u8; 16]);
    assert!(matches!(
        blit_indexed_scanline(&mut dst, &[0], &[], &paint_palette_blend::OPAQUE_BLIT),
        Err(PaletteBlendError::EmptyPalette)
    ));
    let oversized_palette = vec![[0u8; 4]; MAX_PALETTE_ENTRIES + 1];
    assert!(matches!(
        blit_indexed_scanline(
            &mut dst,
            &[0],
            &oversized_palette,
            &paint_palette_blend::OPAQUE_BLIT
        ),
        Err(PaletteBlendError::PaletteTooLarge { .. })
    ));

    // Clip offsets at i32::MIN / i32::MAX and PixelContainer mismatch
    let src_pixels = [0u8, 1, 1, 0];
    let src_img = IndexedImage {
        width: 2,
        height: 2,
        stride: 2,
        pixels: &src_pixels,
    };
    let mut target_buf = [0u8; 4 * 4 * 4];
    let mut target = RgbaTarget {
        width: 4,
        height: 4,
        stride: 16,
        pixels: &mut target_buf,
    };
    for &(ox, oy) in &[
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, 0),
        (0, i32::MAX),
        (-1, -1),
        (3, 3),
    ] {
        let params = BlitParams {
            mode: BlendMode::Alpha,
            alpha: 200,
            transparent_index: Some(1),
            h_flip: true,
            v_flip: true,
        };
        assert!(blit_indexed_image(&mut target, ox, oy, &src_img, &palette, &params).is_ok());
    }

    let mut bad_pc = PixelContainer::new(4, 4);
    bad_pc.data.pop();
    assert!(matches!(
        blit_indexed_to_pixel_container(
            &mut bad_pc,
            0,
            0,
            &src_img,
            &palette,
            &paint_palette_blend::OPAQUE_BLIT
        ),
        Err(PaletteBlendError::BufferTooSmall { .. })
    ));

    // Scene diffing error paths: missing id, duplicate id, oversized scene, small out buffer
    let s_missing = PaintScene {
        width: 100.0,
        height: 100.0,
        background: "#000000".to_string(),
        instructions: vec![rect_instr(None, 0.0)],
        id: None,
        metadata: None,
    };
    let s_dup = PaintScene {
        width: 100.0,
        height: 100.0,
        background: "#000000".to_string(),
        instructions: vec![rect_instr(Some("a"), 0.0), rect_instr(Some("a"), 1.0)],
        id: None,
        metadata: None,
    };
    let s_ok = PaintScene {
        width: 100.0,
        height: 100.0,
        background: "#000000".to_string(),
        instructions: vec![rect_instr(Some("a"), 0.0), rect_instr(Some("b"), 1.0)],
        id: None,
        metadata: None,
    };
    let mut ops = [PatchOp {
        kind: paint_palette_blend::PatchKind::Added,
        old_index: 0,
        new_index: 0,
    }; 4];
    assert!(diff_paint_scene_into(&s_missing, &s_ok, &mut ops).is_err());
    assert!(diff_paint_scene_into(&s_dup, &s_ok, &mut ops).is_err());
    assert!(diff_paint_scene_into(&s_missing, &s_ok, &mut []).is_err());
    let mut tiny_ops = [];
    assert!(matches!(
        diff_paint_scene_into(
            &s_ok,
            &PaintScene {
                instructions: vec![],
                ..s_ok.clone()
            },
            &mut tiny_ops
        ),
        Err(PaletteBlendError::PatchBufferTooSmall {
            required: 2,
            provided: 0
        })
    ));

    let too_many = PaintScene {
        width: 100.0,
        height: 100.0,
        background: "#000000".to_string(),
        instructions: (0..=MAX_SCENE_INSTRUCTIONS)
            .map(|i| rect_instr(Some(&format!("id_{i}")), i as f64))
            .collect(),
        id: None,
        metadata: None,
    };
    assert!(diff_paint_scene_into(&too_many, &s_ok, &mut ops).is_err());
}

#[test]
fn hostile_fuzz_campaign_1000_cases() {
    let mut rng = SplitMix64(0x9A1E_77E0_0000_0001);
    let palette: Vec<[u8; 4]> = (0..16)
        .map(|i| [(i * 17) as u8, 255 - (i * 13) as u8, (i * 7) as u8, 200])
        .collect();

    for i in 0..1_000 {
        let f = rng.next_hostile_f32(i);
        let _ = alpha_from_unit_f32(f);
        let mode = blend_mode_from_code((rng.next_u8()) % 7).unwrap_or(BlendMode::Normal);
        let src_px = [rng.next_u8(), rng.next_u8(), rng.next_u8(), rng.next_u8()];
        let dst_px = [rng.next_u8(), rng.next_u8(), rng.next_u8(), rng.next_u8()];
        let _ = blend_pixel(mode, src_px, dst_px, rng.next_u8());

        let span_len = (rng.next_u32() as usize) % 64;
        let mut d_span = vec![rng.next_u8(); span_len];
        let s_span = vec![rng.next_u8(); (rng.next_u32() as usize) % 64];
        let _ = blend_solid_span(&mut d_span, src_px, mode, rng.next_u8());
        let _ = blend_rgba_scanline(&mut d_span, &s_span, mode, rng.next_u8());

        let idx_len = (rng.next_u32() as usize) % 32;
        let indices: Vec<u8> = (0..idx_len).map(|_| rng.next_u8() % 24).collect();
        let params = BlitParams {
            mode,
            alpha: rng.next_u8(),
            transparent_index: if i % 2 == 0 {
                Some(rng.next_u8() % 24)
            } else {
                None
            },
            h_flip: (i & 1) == 0,
            v_flip: (i & 2) == 0,
        };
        let _ = blit_indexed_scanline(&mut d_span, &indices, &palette, &params);

        let w = match i % 8 {
            0 => 0,
            1 => MAX_DIMENSION + 1,
            2 => u32::MAX,
            _ => 1 + (rng.next_u32() % 8),
        };
        let h = match i % 9 {
            0 => 0,
            1 => MAX_DIMENSION + 1,
            2 => u32::MAX,
            _ => 1 + (rng.next_u32() % 8),
        };
        let src_img = IndexedImage {
            width: w,
            height: h,
            stride: (rng.next_u32() as usize) % 32,
            pixels: &indices,
        };
        let mut target_buf = vec![0u8; (rng.next_u32() as usize) % 256];
        let mut target = RgbaTarget {
            width: 4,
            height: 4,
            stride: if i % 5 == 0 { 4 } else { 16 },
            pixels: &mut target_buf,
        };
        let ox = match i % 5 {
            0 => i32::MIN,
            1 => i32::MAX,
            _ => rng.next_i32(),
        };
        let oy = match i % 6 {
            0 => i32::MIN,
            1 => i32::MAX,
            _ => rng.next_i32(),
        };
        let _ = blit_indexed_image(&mut target, ox, oy, &src_img, &palette, &params);
    }
}
