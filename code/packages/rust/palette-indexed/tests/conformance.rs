//! # Conformance Tests for `palette-indexed` (GAME20)
//!
//! Covers:
//! - Bit-depth geometry and MSB-first pack/unpack round trips (Bpp2/4/8, exhaustive per depth)
//! - Exact packed byte layouts
//! - Single-pixel `get_index` / `set_index`
//! - Palette storage, bounds, colour-0 transparency
//! - Quantized fading (GBA 5-bit, Genesis 3-bit, endpoints, monotonicity, alpha)
//! - Crossfades and depth-mismatch rejection
//! - Indexed views / canvases, strides, clipping, flips
//! - Scanline and framebuffer blits into `PixelContainer` (colour-0 transparency, raster palettes)
//! - CWE-20 / 125 / 190 / 369 / 400 / 787 guards

use palette_indexed::{
    blit_indexed_to_container, blit_scanline_to_container, crossfade_palette_into,
    expand_row_to_rgba_into, fade_color, fade_palette_into, get_index, pack_row_into,
    packed_row_len, quantize_channel, quantize_color, required_buffer_len, set_index,
    unpack_row_into, BitDepth, BlitOptions, ChannelBits, IndexedCanvas, IndexedImage, Palette,
    PaletteError, PixelContainer, BLIT_OPAQUE, BLIT_SPRITE, MAX_DIMENSION, MAX_FADE_STEPS,
    TRANSPARENT,
};

const DEPTHS: [BitDepth; 3] = [BitDepth::Bpp2, BitDepth::Bpp4, BitDepth::Bpp8];

fn gray_palette(depth: BitDepth) -> Palette {
    let mut p = Palette::blank(depth);
    for i in 0..depth.color_count() {
        let v = (i * 255 / (depth.color_count() - 1)) as u8;
        p.set_entry(i as u8, [v, v, v, 255]).unwrap();
    }
    p
}

// --- geometry -------------------------------------------------------------

#[test]
fn depth_geometry() {
    assert_eq!(BitDepth::Bpp2.bits(), 2);
    assert_eq!(BitDepth::Bpp4.bits(), 4);
    assert_eq!(BitDepth::Bpp8.bits(), 8);
    assert_eq!(BitDepth::Bpp2.color_count(), 4);
    assert_eq!(BitDepth::Bpp4.color_count(), 16);
    assert_eq!(BitDepth::Bpp8.color_count(), 256);
    assert_eq!(BitDepth::Bpp2.pixels_per_byte(), 4);
    assert_eq!(BitDepth::Bpp4.pixels_per_byte(), 2);
    assert_eq!(BitDepth::Bpp8.pixels_per_byte(), 1);
}

#[test]
fn packed_row_len_rounds_up() {
    assert_eq!(packed_row_len(BitDepth::Bpp2, 1), Ok(1));
    assert_eq!(packed_row_len(BitDepth::Bpp2, 4), Ok(1));
    assert_eq!(packed_row_len(BitDepth::Bpp2, 5), Ok(2));
    assert_eq!(packed_row_len(BitDepth::Bpp4, 3), Ok(2));
    assert_eq!(packed_row_len(BitDepth::Bpp8, 7), Ok(7));
}

#[test]
fn cwe_20_400_dimension_validation() {
    assert_eq!(
        packed_row_len(BitDepth::Bpp8, 0),
        Err(PaletteError::InvalidDimensions)
    );
    assert!(packed_row_len(BitDepth::Bpp8, MAX_DIMENSION).is_ok());
    assert_eq!(
        packed_row_len(BitDepth::Bpp8, MAX_DIMENSION + 1),
        Err(PaletteError::DimensionTooLarge {
            value: MAX_DIMENSION + 1
        })
    );
    assert_eq!(
        required_buffer_len(BitDepth::Bpp8, 4, 0, 4),
        Err(PaletteError::InvalidDimensions)
    );
    assert!(matches!(
        required_buffer_len(BitDepth::Bpp8, 4, u32::MAX, 4),
        Err(PaletteError::DimensionTooLarge { .. })
    ));
}

#[test]
fn required_buffer_len_and_stride() {
    // last row needs only one packed row
    assert_eq!(required_buffer_len(BitDepth::Bpp4, 5, 3, 8), Ok(2 * 8 + 3));
    assert_eq!(
        required_buffer_len(BitDepth::Bpp4, 5, 3, 2),
        Err(PaletteError::StrideTooSmall {
            stride: 2,
            required: 3
        })
    );
    // CWE-190: huge stride must not wrap
    let r = required_buffer_len(BitDepth::Bpp8, 4, MAX_DIMENSION, usize::MAX / 2);
    assert_eq!(r, Err(PaletteError::SizeOverflow));
}

// --- pack / unpack --------------------------------------------------------

#[test]
fn exact_packed_layout_msb_first() {
    let mut out = [0u8; 4];
    assert_eq!(
        pack_row_into(BitDepth::Bpp2, &[3, 2, 1, 0], &mut out),
        Ok(1)
    );
    assert_eq!(out[0], 0b11_10_01_00);
    assert_eq!(
        pack_row_into(BitDepth::Bpp4, &[0xA, 0x5, 0xF], &mut out),
        Ok(2)
    );
    assert_eq!(&out[..2], &[0xA5, 0xF0]);
    assert_eq!(pack_row_into(BitDepth::Bpp8, &[7, 200], &mut out), Ok(2));
    assert_eq!(&out[..2], &[7, 200]);
}

#[test]
fn round_trip_every_index_every_depth_every_width() {
    for depth in DEPTHS {
        for width in 1..=37usize {
            let indices: Vec<u8> = (0..width)
                .map(|i| ((i * 7 + 3) % depth.color_count()) as u8)
                .collect();
            let mut packed = vec![0xFFu8; width];
            let n = pack_row_into(depth, &indices, &mut packed).unwrap();
            assert_eq!(n, packed_row_len(depth, width as u32).unwrap());
            let mut back = vec![0u8; width];
            unpack_row_into(depth, &packed[..n], width as u32, &mut back).unwrap();
            assert_eq!(back, indices, "depth {depth:?} width {width}");
        }
    }
}

#[test]
fn pack_clears_stale_output_bits() {
    let mut out = [0xFFu8; 2];
    pack_row_into(BitDepth::Bpp4, &[1, 2, 3], &mut out).unwrap();
    assert_eq!(out, [0x12, 0x30]);
}

#[test]
fn cwe_20_pack_rejects_oversized_index_without_writing() {
    let mut out = [0xEEu8; 2];
    assert_eq!(
        pack_row_into(BitDepth::Bpp2, &[0, 1, 4], &mut out),
        Err(PaletteError::IndexOutOfRange { index: 4, limit: 4 })
    );
    assert_eq!(out, [0xEE, 0xEE]);
    assert_eq!(
        pack_row_into(BitDepth::Bpp4, &[16], &mut out),
        Err(PaletteError::IndexOutOfRange {
            index: 16,
            limit: 16
        })
    );
    assert!(pack_row_into(BitDepth::Bpp8, &[255], &mut out).is_ok());
}

#[test]
fn cwe_787_pack_and_unpack_buffer_guards() {
    let mut small = [0u8; 1];
    assert_eq!(
        pack_row_into(BitDepth::Bpp4, &[1, 2, 3], &mut small),
        Err(PaletteError::BufferTooSmall {
            required: 2,
            actual: 1
        })
    );
    assert_eq!(
        pack_row_into(BitDepth::Bpp4, &[], &mut small),
        Err(PaletteError::InvalidDimensions)
    );
    let mut out = [0u8; 2];
    assert_eq!(
        unpack_row_into(BitDepth::Bpp4, &[0x12], 3, &mut out),
        Err(PaletteError::BufferTooSmall {
            required: 2,
            actual: 1
        })
    );
    assert_eq!(
        unpack_row_into(BitDepth::Bpp8, &[1, 2, 3], 3, &mut out),
        Err(PaletteError::BufferTooSmall {
            required: 3,
            actual: 2
        })
    );
    assert_eq!(
        unpack_row_into(BitDepth::Bpp8, &[1], 0, &mut out),
        Err(PaletteError::InvalidDimensions)
    );
}

#[test]
fn unpack_never_yields_index_above_depth() {
    for depth in DEPTHS {
        let packed = [0xFFu8; 64];
        let mut out = [0u8; 64];
        unpack_row_into(depth, &packed, 64, &mut out).unwrap();
        assert!(out.iter().all(|&i| usize::from(i) < depth.color_count()));
    }
}

#[test]
fn get_and_set_index_single_pixel() {
    for depth in DEPTHS {
        let mut row = [0u8; 8];
        let max = (depth.color_count() - 1) as u8;
        for x in 0..(8 * depth.pixels_per_byte()) {
            set_index(depth, &mut row, x, max).unwrap();
            assert_eq!(get_index(depth, &row, x), Ok(max));
            set_index(depth, &mut row, x, 1).unwrap();
            assert_eq!(get_index(depth, &row, x), Ok(1));
        }
    }
    // neighbours untouched
    let mut row = [0u8; 1];
    set_index(BitDepth::Bpp2, &mut row, 1, 3).unwrap();
    assert_eq!(row[0], 0b00_11_00_00);
}

#[test]
fn cwe_125_787_get_set_bounds() {
    let mut row = [0u8; 2];
    assert_eq!(
        get_index(BitDepth::Bpp4, &row, 4),
        Err(PaletteError::OutOfBounds)
    );
    assert_eq!(
        get_index(BitDepth::Bpp4, &row, usize::MAX),
        Err(PaletteError::OutOfBounds)
    );
    assert_eq!(
        set_index(BitDepth::Bpp4, &mut row, 4, 1),
        Err(PaletteError::OutOfBounds)
    );
    assert_eq!(
        set_index(BitDepth::Bpp4, &mut row, 0, 16),
        Err(PaletteError::IndexOutOfRange {
            index: 16,
            limit: 16
        })
    );
    assert_eq!(row, [0, 0]);
}

// --- palette --------------------------------------------------------------

#[test]
fn palette_storage_and_bounds() {
    let mut p = Palette::blank(BitDepth::Bpp2);
    assert_eq!(p.palette_depth(), BitDepth::Bpp2);
    assert_eq!(p.entry_count(), 4);
    assert_eq!(p.active_entries().len(), 4);
    assert_eq!(p.entry(0), Ok(TRANSPARENT));
    p.set_entry(3, [1, 2, 3, 4]).unwrap();
    assert_eq!(p.entry(3), Ok([1, 2, 3, 4]));
    assert_eq!(
        p.entry(4),
        Err(PaletteError::IndexOutOfRange { index: 4, limit: 4 })
    );
    assert_eq!(
        p.set_entry(200, [0; 4]),
        Err(PaletteError::IndexOutOfRange {
            index: 200,
            limit: 4
        })
    );
    let p8 = Palette::blank(BitDepth::Bpp8);
    assert_eq!(p8.active_entries().len(), 256);
    assert!(p8.entry(255).is_ok());
}

#[test]
fn palette_from_colors() {
    let p = Palette::from_colors(BitDepth::Bpp4, &[[9, 9, 9, 255], [1, 1, 1, 255]]).unwrap();
    assert_eq!(p.entry(0), Ok([9, 9, 9, 255]));
    assert_eq!(p.entry(1), Ok([1, 1, 1, 255]));
    assert_eq!(p.entry(2), Ok(TRANSPARENT));
    let too_many = [[0u8; 4]; 5];
    assert_eq!(
        Palette::from_colors(BitDepth::Bpp2, &too_many),
        Err(PaletteError::TooManyColors {
            supplied: 5,
            capacity: 4
        })
    );
}

#[test]
fn color0_transparency_resolution() {
    let p = Palette::from_colors(BitDepth::Bpp2, &[[10, 20, 30, 255], [1, 2, 3, 255]]).unwrap();
    assert_eq!(p.resolve(0, false), Ok([10, 20, 30, 255]));
    assert_eq!(p.resolve(0, true), Ok(TRANSPARENT));
    assert_eq!(p.resolve(1, true), Ok([1, 2, 3, 255]));
    assert!(p.resolve(4, true).is_err());
}

// --- quantization and fading ----------------------------------------------

#[test]
fn quantize_channel_endpoints_and_replication() {
    for bits in [
        ChannelBits::Bits8,
        ChannelBits::Bits6,
        ChannelBits::Bits5,
        ChannelBits::Bits4,
        ChannelBits::Bits3,
        ChannelBits::Bits2,
    ] {
        assert_eq!(quantize_channel(0, bits), 0);
        assert_eq!(quantize_channel(255, bits), 255);
    }
    assert_eq!(quantize_channel(0xAB, ChannelBits::Bits8), 0xAB);
    // 5 bits: top5 = 0b10101 -> 10101_101
    assert_eq!(
        quantize_channel(0b1010_1111, ChannelBits::Bits5),
        0b1010_1101
    );
    // 2 bits: only 4 levels
    assert_eq!(quantize_channel(0x80, ChannelBits::Bits2), 0b1010_1010);
}

#[test]
fn quantize_levels_are_idempotent_and_counted() {
    for (bits, levels) in [
        (ChannelBits::Bits5, 32usize),
        (ChannelBits::Bits3, 8),
        (ChannelBits::Bits2, 4),
    ] {
        let mut seen = [false; 256];
        for v in 0..=255u8 {
            let q = quantize_channel(v, bits);
            assert_eq!(quantize_channel(q, bits), q);
            seen[usize::from(q)] = true;
        }
        assert_eq!(seen.iter().filter(|&&s| s).count(), levels);
    }
}

#[test]
fn quantize_color_preserves_alpha() {
    assert_eq!(
        quantize_color([255, 0, 128, 77], ChannelBits::Bits3),
        [255, 0, quantize_channel(128, ChannelBits::Bits3), 77]
    );
}

#[test]
fn fade_color_endpoints_and_midpoint() {
    let white = [255, 255, 255, 255];
    let black = [0, 0, 0, 255];
    assert_eq!(
        fade_color(white, black, 0, 4, ChannelBits::Bits8),
        Ok(white)
    );
    assert_eq!(
        fade_color(white, black, 4, 4, ChannelBits::Bits8),
        Ok(black)
    );
    assert_eq!(
        fade_color(black, white, 1, 2, ChannelBits::Bits8),
        Ok([128, 128, 128, 255])
    );
    // alpha comes from the source colour
    assert_eq!(
        fade_color([200, 100, 50, 9], [0, 0, 0, 255], 4, 4, ChannelBits::Bits8),
        Ok([0, 0, 0, 9])
    );
}

#[test]
fn fade_to_black_is_monotonic_and_quantized() {
    let from = [255, 200, 100, 255];
    let black = [0, 0, 0, 255];
    let mut prev = [255u8; 3];
    for step in 0..=16u16 {
        let c = fade_color(from, black, step, 16, ChannelBits::Bits5).unwrap();
        for ch in 0..3 {
            assert!(c[ch] <= prev[ch]);
            assert_eq!(quantize_channel(c[ch], ChannelBits::Bits5), c[ch]);
            prev[ch] = c[ch];
        }
    }
    assert_eq!(prev, [0, 0, 0]);
}

#[test]
fn cwe_369_190_fade_step_validation() {
    let c = [1, 2, 3, 255];
    assert_eq!(
        fade_color(c, c, 0, 0, ChannelBits::Bits8),
        Err(PaletteError::InvalidFadeStep)
    );
    assert_eq!(
        fade_color(c, c, 5, 4, ChannelBits::Bits8),
        Err(PaletteError::InvalidFadeStep)
    );
    assert_eq!(
        fade_color(c, c, 0, MAX_FADE_STEPS + 1, ChannelBits::Bits8),
        Err(PaletteError::InvalidFadeStep)
    );
    assert_eq!(
        fade_color(c, c, u16::MAX, u16::MAX, ChannelBits::Bits8),
        Err(PaletteError::InvalidFadeStep)
    );
    assert!(fade_color(c, c, MAX_FADE_STEPS, MAX_FADE_STEPS, ChannelBits::Bits8).is_ok());
}

#[test]
fn fade_palette_into_matches_fade_color() {
    for depth in DEPTHS {
        let src = gray_palette(depth);
        let mut dst = Palette::blank(BitDepth::Bpp2);
        let target = [10, 20, 30, 255];
        fade_palette_into(&src, &mut dst, target, 3, 8, ChannelBits::Bits5).unwrap();
        assert_eq!(dst.palette_depth(), depth);
        for i in 0..depth.color_count() {
            let expect = fade_color(
                src.entry(i as u8).unwrap(),
                target,
                3,
                8,
                ChannelBits::Bits5,
            )
            .unwrap();
            assert_eq!(dst.entry(i as u8), Ok(expect));
        }
    }
}

#[test]
fn fade_palette_rejects_bad_step_and_leaves_dst() {
    let src = gray_palette(BitDepth::Bpp4);
    let mut dst = Palette::blank(BitDepth::Bpp4);
    let before = dst;
    assert_eq!(
        fade_palette_into(&src, &mut dst, TRANSPARENT, 1, 0, ChannelBits::Bits8),
        Err(PaletteError::InvalidFadeStep)
    );
    assert_eq!(dst, before);
}

#[test]
fn crossfade_endpoints_and_mismatch() {
    let a = gray_palette(BitDepth::Bpp4);
    let mut b = Palette::blank(BitDepth::Bpp4);
    for i in 0..16u8 {
        b.set_entry(i, [255 - i * 16, 0, 0, 255]).unwrap();
    }
    let mut out = Palette::blank(BitDepth::Bpp4);
    crossfade_palette_into(&a, &b, &mut out, 0, 4, ChannelBits::Bits8).unwrap();
    assert_eq!(out, a);
    crossfade_palette_into(&a, &b, &mut out, 4, 4, ChannelBits::Bits8).unwrap();
    for i in 0..16u8 {
        let e = b.entry(i).unwrap();
        // alpha is taken from `a`
        assert_eq!(out.entry(i), Ok([e[0], e[1], e[2], 255]));
    }
    let c = gray_palette(BitDepth::Bpp8);
    assert_eq!(
        crossfade_palette_into(&a, &c, &mut out, 1, 2, ChannelBits::Bits8),
        Err(PaletteError::DepthMismatch)
    );
    assert_eq!(
        crossfade_palette_into(&a, &b, &mut out, 3, 2, ChannelBits::Bits8),
        Err(PaletteError::InvalidFadeStep)
    );
}

// --- views and canvases ---------------------------------------------------

#[test]
fn image_view_validation() {
    let data = [0u8; 16];
    assert!(IndexedImage::view(BitDepth::Bpp4, 4, 4, 2, &data).is_ok());
    assert_eq!(
        IndexedImage::view(BitDepth::Bpp4, 0, 4, 2, &data).unwrap_err(),
        PaletteError::InvalidDimensions
    );
    assert_eq!(
        IndexedImage::view(BitDepth::Bpp4, 4, 4, 1, &data).unwrap_err(),
        PaletteError::StrideTooSmall {
            stride: 1,
            required: 2
        }
    );
    assert_eq!(
        IndexedImage::view(BitDepth::Bpp8, 4, 5, 4, &data).unwrap_err(),
        PaletteError::BufferTooSmall {
            required: 20,
            actual: 16
        }
    );
    assert!(matches!(
        IndexedImage::view(BitDepth::Bpp8, MAX_DIMENSION + 1, 1, usize::MAX, &data),
        Err(PaletteError::DimensionTooLarge { .. })
    ));
}

#[test]
fn image_index_at_and_rows_with_stride() {
    // Bpp4, 3 px wide -> 2 bytes/row, stride 4 (2 padding bytes)
    let data = [0x12, 0x30, 0xEE, 0xEE, 0x45, 0x60];
    let img = IndexedImage::view(BitDepth::Bpp4, 3, 2, 4, &data).unwrap();
    assert_eq!(img.bit_depth(), BitDepth::Bpp4);
    assert_eq!((img.width(), img.height(), img.stride()), (3, 2, 4));
    assert_eq!(img.index_at(0, 0), Some(1));
    assert_eq!(img.index_at(2, 0), Some(3));
    assert_eq!(img.index_at(0, 1), Some(4));
    assert_eq!(img.index_at(2, 1), Some(6));
    assert_eq!(img.index_at(3, 0), None);
    assert_eq!(img.index_at(0, 2), None);
    assert_eq!(img.row_bytes(1), Some(&data[4..6]));
    assert_eq!(img.row_bytes(2), None);
    let mut out = [0u8; 3];
    img.unpack_scanline_into(1, &mut out).unwrap();
    assert_eq!(out, [4, 5, 6]);
    assert_eq!(
        img.unpack_scanline_into(2, &mut out),
        Err(PaletteError::OutOfBounds)
    );
}

#[test]
fn canvas_put_peek_clear() {
    for depth in DEPTHS {
        let mut data = vec![0u8; 64];
        let mut c = IndexedCanvas::attach(depth, 5, 3, 8, &mut data).unwrap();
        let max = (depth.color_count() - 1) as u8;
        c.clear_indices(max).unwrap();
        for y in 0..3 {
            for x in 0..5 {
                assert_eq!(c.peek_index(x, y), Some(max));
            }
        }
        c.put_index(4, 2, 1).unwrap();
        assert_eq!(c.peek_index(4, 2), Some(1));
        assert_eq!(c.peek_index(3, 2), Some(max));
        assert_eq!(c.put_index(5, 0, 1), Err(PaletteError::OutOfBounds));
        assert_eq!(c.put_index(0, 3, 1), Err(PaletteError::OutOfBounds));
        assert_eq!(c.peek_index(5, 0), None);
        if depth != BitDepth::Bpp8 {
            assert!(matches!(
                c.put_index(0, 0, max.wrapping_add(1)),
                Err(PaletteError::IndexOutOfRange { .. })
            ));
            assert!(matches!(
                c.clear_indices(max.wrapping_add(1)),
                Err(PaletteError::IndexOutOfRange { .. })
            ));
        }
    }
}

#[test]
fn canvas_attach_validation() {
    let mut data = [0u8; 4];
    assert!(matches!(
        IndexedCanvas::attach(BitDepth::Bpp8, 4, 2, 4, &mut data),
        Err(PaletteError::BufferTooSmall { .. })
    ));
}

#[test]
fn canvas_blit_indexed_transparency_flip_and_clip() {
    // 2x2 Bpp4 sprite: [0 1 / 2 0]
    let sprite_data = [0x01, 0x20];
    let sprite = IndexedImage::view(BitDepth::Bpp4, 2, 2, 1, &sprite_data).unwrap();
    let mut fb = [0u8; 16];
    let mut c = IndexedCanvas::attach(BitDepth::Bpp4, 4, 4, 2, &mut fb).unwrap();
    c.clear_indices(9).unwrap();
    let n = c.blit_indexed(&sprite, 1, 1, BLIT_SPRITE).unwrap();
    assert_eq!(n, 2);
    assert_eq!(c.peek_index(1, 1), Some(9)); // transparent
    assert_eq!(c.peek_index(2, 1), Some(1));
    assert_eq!(c.peek_index(1, 2), Some(2));
    assert_eq!(c.peek_index(2, 2), Some(9));

    c.clear_indices(9).unwrap();
    let n = c.blit_indexed(&sprite, 0, 0, BLIT_OPAQUE).unwrap();
    assert_eq!(n, 4);
    assert_eq!(c.peek_index(0, 0), Some(0));

    // h_flip
    c.clear_indices(9).unwrap();
    let flip = BlitOptions {
        color0_transparent: true,
        h_flip: true,
        v_flip: false,
    };
    c.blit_indexed(&sprite, 0, 0, flip).unwrap();
    assert_eq!(c.peek_index(0, 0), Some(1));
    assert_eq!(c.peek_index(1, 0), Some(9));
    assert_eq!(c.peek_index(0, 1), Some(9));
    assert_eq!(c.peek_index(1, 1), Some(2));

    // v_flip
    c.clear_indices(9).unwrap();
    let vflip = BlitOptions {
        color0_transparent: true,
        h_flip: false,
        v_flip: true,
    };
    c.blit_indexed(&sprite, 0, 0, vflip).unwrap();
    assert_eq!(c.peek_index(0, 0), Some(2));
    assert_eq!(c.peek_index(1, 1), Some(1));

    // partially off-screen on every side, and completely off-screen
    c.clear_indices(9).unwrap();
    assert_eq!(c.blit_indexed(&sprite, -1, -1, BLIT_OPAQUE), Ok(1));
    assert_eq!(c.blit_indexed(&sprite, 3, 3, BLIT_OPAQUE), Ok(1));
    assert_eq!(c.blit_indexed(&sprite, 4, 0, BLIT_OPAQUE), Ok(0));
    assert_eq!(c.blit_indexed(&sprite, 0, -2, BLIT_OPAQUE), Ok(0));
    assert_eq!(
        c.blit_indexed(&sprite, i32::MIN, i32::MAX, BLIT_OPAQUE),
        Ok(0)
    );
    assert_eq!(
        c.blit_indexed(&sprite, i32::MAX, i32::MIN, BLIT_OPAQUE),
        Ok(0)
    );
}

#[test]
fn canvas_blit_indexed_depth_mismatch() {
    let data = [0u8; 4];
    let sprite = IndexedImage::view(BitDepth::Bpp8, 2, 2, 2, &data).unwrap();
    let mut fb = [0u8; 8];
    let mut c = IndexedCanvas::attach(BitDepth::Bpp4, 4, 4, 2, &mut fb).unwrap();
    assert_eq!(
        c.blit_indexed(&sprite, 0, 0, BLIT_OPAQUE),
        Err(PaletteError::DepthMismatch)
    );
}

// --- RGBA blits -----------------------------------------------------------

fn red_green_palette() -> Palette {
    Palette::from_colors(
        BitDepth::Bpp2,
        &[
            [1, 1, 1, 255],
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
        ],
    )
    .unwrap()
}

#[test]
fn expand_row_to_rgba() {
    let palette = red_green_palette();
    let mut out = [0xAAu8; 20];
    // packed [0,1,2,3, 1]
    let n = expand_row_to_rgba_into(
        BitDepth::Bpp2,
        &[0b00_01_10_11, 0b01_00_00_00],
        5,
        &palette,
        true,
        &mut out,
    )
    .unwrap();
    assert_eq!(n, 20);
    assert_eq!(&out[0..4], &TRANSPARENT);
    assert_eq!(&out[4..8], &[255, 0, 0, 255]);
    assert_eq!(&out[8..12], &[0, 255, 0, 255]);
    assert_eq!(&out[12..16], &[0, 0, 255, 255]);
    assert_eq!(&out[16..20], &[255, 0, 0, 255]);

    // opaque colour 0
    expand_row_to_rgba_into(BitDepth::Bpp2, &[0, 0], 5, &palette, false, &mut out).unwrap();
    assert_eq!(&out[0..4], &[1, 1, 1, 255]);
}

#[test]
fn expand_row_guards() {
    let palette = red_green_palette();
    let mut out = [0u8; 8];
    assert_eq!(
        expand_row_to_rgba_into(BitDepth::Bpp2, &[0], 4, &palette, false, &mut out),
        Err(PaletteError::BufferTooSmall {
            required: 16,
            actual: 8
        })
    );
    assert_eq!(
        expand_row_to_rgba_into(BitDepth::Bpp2, &[], 2, &palette, false, &mut out),
        Err(PaletteError::BufferTooSmall {
            required: 1,
            actual: 0
        })
    );
    assert_eq!(
        expand_row_to_rgba_into(BitDepth::Bpp4, &[0], 2, &palette, false, &mut out),
        Err(PaletteError::PaletteTooSmall {
            required: 16,
            actual: 4
        })
    );
}

#[test]
fn blit_image_to_container_with_transparency() {
    let palette = red_green_palette();
    // 4x2 Bpp2: row0 = 0 1 2 3, row1 = 3 2 1 0
    let data = [0b00_01_10_11, 0b11_10_01_00];
    let img = IndexedImage::view(BitDepth::Bpp2, 4, 2, 1, &data).unwrap();
    let mut dst = PixelContainer::new(6, 4);
    dst.fill(9, 9, 9, 255);

    let n = blit_indexed_to_container(&img, &palette, BLIT_SPRITE, &mut dst, 1, 1).unwrap();
    assert_eq!(n, 6); // two colour-0 pixels skipped
    assert_eq!(dst.pixel_at(1, 1), (9, 9, 9, 255)); // transparent -> untouched
    assert_eq!(dst.pixel_at(2, 1), (255, 0, 0, 255));
    assert_eq!(dst.pixel_at(3, 1), (0, 255, 0, 255));
    assert_eq!(dst.pixel_at(4, 1), (0, 0, 255, 255));
    assert_eq!(dst.pixel_at(1, 2), (0, 0, 255, 255));
    assert_eq!(dst.pixel_at(4, 2), (9, 9, 9, 255));
    assert_eq!(dst.pixel_at(0, 0), (9, 9, 9, 255));
    assert_eq!(dst.pixel_at(5, 3), (9, 9, 9, 255));

    // opaque writes colour 0 as its palette colour
    let n = blit_indexed_to_container(&img, &palette, BLIT_OPAQUE, &mut dst, 0, 0).unwrap();
    assert_eq!(n, 8);
    assert_eq!(dst.pixel_at(0, 0), (1, 1, 1, 255));
}

#[test]
fn blit_to_container_flips_and_clips() {
    let palette = red_green_palette();
    let data = [0b01_10_11_00, 0b00_11_10_01];
    let img = IndexedImage::view(BitDepth::Bpp2, 4, 2, 1, &data).unwrap();
    let mut dst = PixelContainer::new(4, 2);
    let hv = BlitOptions {
        color0_transparent: false,
        h_flip: true,
        v_flip: true,
    };
    blit_indexed_to_container(&img, &palette, hv, &mut dst, 0, 0).unwrap();
    // v+h flip: dst(0,0) = src(3,1) = 1 -> red; dst(3,1) = src(0,0) = 1 -> red
    assert_eq!(dst.pixel_at(0, 0), (255, 0, 0, 255));
    assert_eq!(dst.pixel_at(3, 1), (255, 0, 0, 255));
    assert_eq!(dst.pixel_at(1, 0), (0, 255, 0, 255)); // src(2,1) = 2

    // clipping, all four edges + extremes
    let mut dst = PixelContainer::new(4, 4);
    assert_eq!(
        blit_indexed_to_container(&img, &palette, BLIT_OPAQUE, &mut dst, -3, -1),
        Ok(1)
    );
    assert_eq!(
        blit_indexed_to_container(&img, &palette, BLIT_OPAQUE, &mut dst, 3, 3),
        Ok(1)
    );
    for (x, y) in [
        (4, 0),
        (-4, 0),
        (0, 4),
        (0, -2),
        (i32::MAX, 0),
        (i32::MIN, i32::MIN),
    ] {
        assert_eq!(
            blit_indexed_to_container(&img, &palette, BLIT_OPAQUE, &mut dst, x, y),
            Ok(0)
        );
    }
}

#[test]
fn scanline_blit_enables_raster_palette_effects() {
    let data = [0b01_01_01_01, 0b01_01_01_01];
    let img = IndexedImage::view(BitDepth::Bpp2, 4, 2, 1, &data).unwrap();
    let mut pal_a = red_green_palette();
    let mut pal_b = red_green_palette();
    pal_a.set_entry(1, [10, 0, 0, 255]).unwrap();
    pal_b.set_entry(1, [0, 10, 0, 255]).unwrap();
    let mut dst = PixelContainer::new(4, 2);
    assert_eq!(
        blit_scanline_to_container(&img, 0, &pal_a, BLIT_OPAQUE, &mut dst, 0, 0),
        Ok(4)
    );
    assert_eq!(
        blit_scanline_to_container(&img, 1, &pal_b, BLIT_OPAQUE, &mut dst, 0, 1),
        Ok(4)
    );
    assert_eq!(dst.pixel_at(0, 0), (10, 0, 0, 255));
    assert_eq!(dst.pixel_at(3, 1), (0, 10, 0, 255));
}

#[test]
fn scanline_blit_guards() {
    let palette = red_green_palette();
    let data = [0u8; 2];
    let img = IndexedImage::view(BitDepth::Bpp2, 4, 2, 1, &data).unwrap();
    let mut dst = PixelContainer::new(4, 2);
    assert_eq!(
        blit_scanline_to_container(&img, 2, &palette, BLIT_OPAQUE, &mut dst, 0, 0),
        Err(PaletteError::OutOfBounds)
    );
    // off-screen destination rows are clipped, not errors
    assert_eq!(
        blit_scanline_to_container(&img, 0, &palette, BLIT_OPAQUE, &mut dst, 0, -1),
        Ok(0)
    );
    assert_eq!(
        blit_scanline_to_container(&img, 0, &palette, BLIT_OPAQUE, &mut dst, 0, 2),
        Ok(0)
    );
    assert_eq!(
        blit_scanline_to_container(&img, 0, &palette, BLIT_OPAQUE, &mut dst, 10, 0),
        Ok(0)
    );
    assert_eq!(
        blit_scanline_to_container(&img, 0, &palette, BLIT_OPAQUE, &mut dst, -10, 0),
        Ok(0)
    );
    // partial horizontal clip
    assert_eq!(
        blit_scanline_to_container(&img, 0, &palette, BLIT_OPAQUE, &mut dst, 2, 0),
        Ok(2)
    );
}

#[test]
fn cwe_125_787_container_and_palette_guards() {
    let palette = red_green_palette();
    let data = [0u8; 4];
    let img = IndexedImage::view(BitDepth::Bpp4, 2, 2, 1, &data).unwrap();
    let mut dst = PixelContainer::new(4, 4);
    // 4-colour palette cannot serve a 16-colour image
    assert_eq!(
        blit_indexed_to_container(&img, &palette, BLIT_OPAQUE, &mut dst, 0, 0),
        Err(PaletteError::PaletteTooSmall {
            required: 16,
            actual: 4
        })
    );
    // tampered container (public Vec) is rejected rather than indexed out of range
    let img2 = IndexedImage::view(BitDepth::Bpp2, 2, 2, 1, &data).unwrap();
    dst.data.truncate(10);
    assert_eq!(
        blit_indexed_to_container(&img2, &palette, BLIT_OPAQUE, &mut dst, 0, 0),
        Err(PaletteError::ContainerMismatch)
    );
    assert_eq!(
        blit_scanline_to_container(&img2, 0, &palette, BLIT_OPAQUE, &mut dst, 0, 0),
        Err(PaletteError::ContainerMismatch)
    );
}

#[test]
fn empty_container_is_a_noop() {
    let palette = red_green_palette();
    let data = [0u8; 2];
    let img = IndexedImage::view(BitDepth::Bpp2, 4, 2, 1, &data).unwrap();
    let mut dst = PixelContainer::new(0, 0);
    assert_eq!(
        blit_indexed_to_container(&img, &palette, BLIT_OPAQUE, &mut dst, 0, 0),
        Ok(0)
    );
}

#[test]
fn bpp8_full_palette_blit() {
    let palette = gray_palette(BitDepth::Bpp8);
    let mut row = [0u8; 16];
    for (i, v) in row.iter_mut().enumerate() {
        *v = (i * 17) as u8;
    }
    let img = IndexedImage::view(BitDepth::Bpp8, 16, 1, 16, &row).unwrap();
    let mut dst = PixelContainer::new(16, 1);
    assert_eq!(
        blit_indexed_to_container(&img, &palette, BLIT_OPAQUE, &mut dst, 0, 0),
        Ok(16)
    );
    for i in 0..16u32 {
        let v = (i * 17) as u8;
        assert_eq!(dst.pixel_at(i, 0), (v, v, v, 255));
    }
}

#[test]
fn deterministic_across_runs() {
    let palette = gray_palette(BitDepth::Bpp4);
    let data: Vec<u8> = (0..32u8).map(|i| i.wrapping_mul(37)).collect();
    let img = IndexedImage::view(BitDepth::Bpp4, 8, 4, 4, &data).unwrap();
    let mut a = PixelContainer::new(10, 10);
    let mut b = PixelContainer::new(10, 10);
    blit_indexed_to_container(&img, &palette, BLIT_SPRITE, &mut a, 1, 2).unwrap();
    blit_indexed_to_container(&img, &palette, BLIT_SPRITE, &mut b, 1, 2).unwrap();
    assert_eq!(a, b);
}

#[test]
fn error_display_is_informative() {
    let errs = [
        PaletteError::InvalidDimensions,
        PaletteError::DimensionTooLarge { value: 99999 },
        PaletteError::SizeOverflow,
        PaletteError::StrideTooSmall {
            stride: 1,
            required: 2,
        },
        PaletteError::BufferTooSmall {
            required: 2,
            actual: 1,
        },
        PaletteError::IndexOutOfRange { index: 9, limit: 4 },
        PaletteError::PaletteTooSmall {
            required: 16,
            actual: 4,
        },
        PaletteError::TooManyColors {
            supplied: 5,
            capacity: 4,
        },
        PaletteError::InvalidFadeStep,
        PaletteError::DepthMismatch,
        PaletteError::OutOfBounds,
        PaletteError::ContainerMismatch,
    ];
    for e in errs {
        assert!(!e.to_string().is_empty());
        let _: &dyn std::error::Error = &e;
    }
}
