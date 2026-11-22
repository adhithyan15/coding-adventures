//! Hostile-input and deterministic fuzz test suite for `palette-indexed`.

#![forbid(unsafe_code)]

use palette_indexed::{
    blit_indexed_to_container, blit_scanline_to_container, crossfade_palette_into,
    expand_row_to_rgba_into, fade_color, fade_palette_into, get_index, pack_row_into,
    packed_row_len, quantize_channel, quantize_color, required_buffer_len, set_index,
    unpack_row_into, BitDepth, BlitOptions, ChannelBits, IndexedCanvas, IndexedImage, Palette,
    PaletteError, PixelContainer, MAX_DIMENSION, MAX_FADE_STEPS,
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

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

#[test]
fn hostile_dimensions_strides_and_overflows() {
    for depth in [BitDepth::Bpp2, BitDepth::Bpp4, BitDepth::Bpp8] {
        assert_eq!(
            packed_row_len(depth, 0),
            Err(PaletteError::InvalidDimensions)
        );
        assert_eq!(
            packed_row_len(depth, MAX_DIMENSION + 1),
            Err(PaletteError::DimensionTooLarge {
                value: MAX_DIMENSION + 1
            })
        );
        assert_eq!(
            packed_row_len(depth, u32::MAX),
            Err(PaletteError::DimensionTooLarge { value: u32::MAX })
        );

        assert_eq!(
            required_buffer_len(depth, 0, 1, 1),
            Err(PaletteError::InvalidDimensions)
        );
        assert_eq!(
            required_buffer_len(depth, 1, 0, 1),
            Err(PaletteError::InvalidDimensions)
        );
        assert_eq!(
            required_buffer_len(depth, 1, u32::MAX, 1),
            Err(PaletteError::DimensionTooLarge { value: u32::MAX })
        );
        assert_eq!(
            required_buffer_len(depth, 8, 8, 0),
            Err(PaletteError::StrideTooSmall {
                stride: 0,
                required: packed_row_len(depth, 8).unwrap(),
            })
        );
        assert_eq!(
            required_buffer_len(depth, 8, MAX_DIMENSION, usize::MAX),
            Err(PaletteError::SizeOverflow)
        );
    }

    // Fading bounds (CWE-369 / CWE-190).
    assert_eq!(
        fade_color(
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            0,
            0,
            ChannelBits::Bits8
        ),
        Err(PaletteError::InvalidFadeStep)
    );
    assert_eq!(
        fade_color(
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            0,
            MAX_FADE_STEPS + 1,
            ChannelBits::Bits8
        ),
        Err(PaletteError::InvalidFadeStep)
    );
    assert_eq!(
        fade_color(
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            5,
            4,
            ChannelBits::Bits8
        ),
        Err(PaletteError::InvalidFadeStep)
    );
}

#[test]
fn hostile_atomic_buffer_preservation_and_oob_indices() {
    // pack_row_into must not modify `out` when any index is out of range.
    let sentinel = [0xAAu8; 8];
    let mut out = sentinel;
    assert!(matches!(
        pack_row_into(BitDepth::Bpp2, &[0, 1, 4, 2], &mut out),
        Err(PaletteError::IndexOutOfRange { index: 4, limit: 4 })
    ));
    assert_eq!(out, sentinel);

    assert!(matches!(
        pack_row_into(BitDepth::Bpp4, &[0, 15, 16], &mut out),
        Err(PaletteError::IndexOutOfRange {
            index: 16,
            limit: 16
        })
    ));
    assert_eq!(out, sentinel);

    assert!(matches!(
        pack_row_into(BitDepth::Bpp4, &[0, 1, 2, 3], &mut out[..1]),
        Err(PaletteError::BufferTooSmall { .. })
    ));
    assert_eq!(out, sentinel);

    // unpack_row_into / get_index / set_index / expand_row_to_rgba_into.
    let packed = [0x12u8, 0x34u8];
    let mut unpacked = [0x55u8; 4];
    assert!(matches!(
        unpack_row_into(BitDepth::Bpp4, &packed[..1], 4, &mut unpacked),
        Err(PaletteError::BufferTooSmall { .. })
    ));
    assert_eq!(unpacked, [0x55u8; 4]);

    assert_eq!(
        get_index(BitDepth::Bpp4, &packed, 4),
        Err(PaletteError::OutOfBounds)
    );
    assert_eq!(
        get_index(BitDepth::Bpp8, &packed, usize::MAX),
        Err(PaletteError::OutOfBounds)
    );
    let mut mut_packed = packed;
    assert_eq!(
        set_index(BitDepth::Bpp2, &mut mut_packed, 0, 4),
        Err(PaletteError::IndexOutOfRange { index: 4, limit: 4 })
    );
    assert_eq!(
        set_index(BitDepth::Bpp4, &mut mut_packed, 4, 1),
        Err(PaletteError::OutOfBounds)
    );
    assert_eq!(mut_packed, packed);

    // Palette validation & mismatch checks.
    let too_many = [[0u8; 4]; 5];
    assert_eq!(
        Palette::from_colors(BitDepth::Bpp2, &too_many),
        Err(PaletteError::TooManyColors {
            supplied: 5,
            capacity: 4
        })
    );
    let p2 = Palette::blank(BitDepth::Bpp2);
    let p4 = Palette::blank(BitDepth::Bpp4);
    assert_eq!(
        p2.entry(4),
        Err(PaletteError::IndexOutOfRange { index: 4, limit: 4 })
    );
    assert_eq!(
        p2.resolve(255, true),
        Err(PaletteError::IndexOutOfRange {
            index: 255,
            limit: 4
        })
    );
    let mut dst_pal = Palette::blank(BitDepth::Bpp4);
    assert_eq!(
        crossfade_palette_into(&p2, &p4, &mut dst_pal, 1, 2, ChannelBits::Bits5),
        Err(PaletteError::DepthMismatch)
    );

    let mut rgba_line = [0u8; 16];
    assert!(matches!(
        expand_row_to_rgba_into(BitDepth::Bpp8, &packed, 2, &p4, true, &mut rgba_line),
        Err(PaletteError::PaletteTooSmall { .. })
    ));

    // Container mismatch & OOB scanline.
    let img = IndexedImage::view(BitDepth::Bpp4, 4, 2, 2, &[0x12, 0x34, 0x56, 0x78]).unwrap();
    let mut bad_container = PixelContainer {
        width: 4,
        height: 4,
        data: vec![0u8; 4],
    };
    assert_eq!(
        blit_indexed_to_container(&img, &p4, BlitOptions::default(), &mut bad_container, 0, 0),
        Err(PaletteError::ContainerMismatch)
    );
    let mut ok_container = PixelContainer {
        width: 4,
        height: 4,
        data: vec![0u8; 4 * 4 * 4],
    };
    assert_eq!(
        blit_scanline_to_container(
            &img,
            2,
            &p4,
            BlitOptions::default(),
            &mut ok_container,
            0,
            0
        ),
        Err(PaletteError::OutOfBounds)
    );
}

#[test]
fn hostile_fuzz_1000_cases() {
    let mut rng = SplitMix64(0x2026_0020_BEEF_CAFE);
    let depths = [BitDepth::Bpp2, BitDepth::Bpp4, BitDepth::Bpp8];
    let cbits = [
        ChannelBits::Bits2,
        ChannelBits::Bits3,
        ChannelBits::Bits4,
        ChannelBits::Bits5,
        ChannelBits::Bits6,
        ChannelBits::Bits8,
    ];

    for _ in 0..1000 {
        let d = depths[(rng.next_u8() as usize) % depths.len()];
        let w = match rng.next_u8() % 5 {
            0 => 0,
            1 => u32::MAX,
            2 => MAX_DIMENSION + 1,
            _ => (rng.next_u32() % 32).max(1),
        };
        let h = match rng.next_u8() % 5 {
            0 => 0,
            1 => u32::MAX,
            2 => MAX_DIMENSION + 1,
            _ => (rng.next_u32() % 16).max(1),
        };
        let stride = match rng.next_u8() % 4 {
            0 => 0,
            1 => usize::MAX,
            _ => (rng.next_u8() as usize) % 64,
        };
        let _ = packed_row_len(d, w);
        let _ = required_buffer_len(d, w, h, stride);

        let mut buf = [0u8; 64];
        for b in &mut buf {
            *b = rng.next_u8();
        }
        let in_len = (rng.next_u8() as usize) % 32;
        let out_len = (rng.next_u8() as usize) % 32;
        let mut pack_out = [0u8; 32];
        let _ = pack_row_into(d, &buf[..in_len], &mut pack_out[..out_len]);
        let mut unpack_out = [0u8; 32];
        let _ = unpack_row_into(d, &buf[..in_len], w % 40, &mut unpack_out[..out_len]);

        let idx = if rng.next_u8().is_multiple_of(4) {
            usize::MAX
        } else {
            (rng.next_u8() as usize) % 256
        };
        let _ = get_index(d, &buf[..in_len], idx);
        let _ = set_index(d, &mut pack_out[..out_len], idx, rng.next_u8());

        let mut pal = Palette::blank(d);
        let _ = pal.set_entry(
            rng.next_u8(),
            [rng.next_u8(), rng.next_u8(), rng.next_u8(), 255],
        );
        let _ = pal.resolve(rng.next_u8(), rng.next_u8() & 1 != 0);

        let cb = cbits[(rng.next_u8() as usize) % cbits.len()];
        let _ = quantize_channel(rng.next_u8(), cb);
        let _ = quantize_color(
            [rng.next_u8(), rng.next_u8(), rng.next_u8(), rng.next_u8()],
            cb,
        );
        let mut dst_pal = Palette::blank(d);
        let _ = fade_palette_into(
            &pal,
            &mut dst_pal,
            [rng.next_u8(), rng.next_u8(), rng.next_u8(), 255],
            rng.next_u16(),
            rng.next_u16(),
            cb,
        );

        let cw = ((rng.next_u8() % 8) + 1) as u32;
        let ch = ((rng.next_u8() % 8) + 1) as u32;
        let cstride = packed_row_len(d, cw).unwrap_or(1);
        if let Ok(req) = required_buffer_len(d, cw, ch, cstride) {
            let mut canvas_data = vec![0u8; req];
            for b in &mut canvas_data {
                *b = rng.next_u8();
            }
            if let Ok(mut canvas) = IndexedCanvas::attach(d, cw, ch, cstride, &mut canvas_data) {
                let _ = canvas.put_index(rng.next_u32() % 16, rng.next_u32() % 16, rng.next_u8());
                let _ = canvas.clear_indices(rng.next_u8());
                let src_data = vec![rng.next_u8(); req];
                if let Ok(src) = IndexedImage::view(d, cw, ch, cstride, &src_data) {
                    let opts = BlitOptions {
                        color0_transparent: rng.next_u8() & 1 != 0,
                        h_flip: rng.next_u8() & 2 != 0,
                        v_flip: rng.next_u8() & 4 != 0,
                    };
                    let _ = canvas.blit_indexed(&src, rng.next_i32(), rng.next_i32(), opts);
                    let mut pc = PixelContainer {
                        width: 8,
                        height: 8,
                        data: vec![0u8; 8 * 8 * 4],
                    };
                    let _ = blit_indexed_to_container(
                        &src,
                        &pal,
                        opts,
                        &mut pc,
                        rng.next_i32(),
                        rng.next_i32(),
                    );
                    let _ = blit_scanline_to_container(
                        &src,
                        rng.next_u32() % 16,
                        &pal,
                        opts,
                        &mut pc,
                        rng.next_i32(),
                        rng.next_i32(),
                    );
                }
            }
        }
    }
}
