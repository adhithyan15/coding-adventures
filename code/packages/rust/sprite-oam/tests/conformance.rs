//! # Conformance Tests for `sprite-oam` (GAME22)
//!
//! Covers:
//! - Shape/size geometry, entry validation (slot, priority, bank, tile, coordinate)
//! - Table access, hiding, affine matrix groups and `Affine2D` round trips
//! - Back-to-front priority ordering and OAM-index tie-breaking
//! - Scanline evaluation with per-line limits, overflow, whole-frame statistics
//! - Texel sampling (flips, affine rotation, double-size boxes)
//! - Metasprite stamping (offsets, mirroring, atomicity)
//! - Animation sequencing: one-shot, loop, ping-pong, large ticks, mismatch
//! - Lowering to `SpriteQuad` / `PaintScene` with palette-bank tints
//! - CWE-20 / 125 / 190 / 369 / 400 / 787 guards

use sprite_oam::{
    bank_tint, quad_id_for_slot, slot_from_quad_id, Affine2D, AffineParams, Animation, Animator,
    MetaFrame, OamEntry, OamError, OamTable, Palette, PlayMode, SpriteQuad, SpriteShape,
    SpriteSize, AFFINE_ONE, AFFINE_SLOTS, COORD_MAX, COORD_MIN, MAX_FRAMES, MAX_PARTS,
    MAX_SCREEN_HEIGHT, MAX_TICK_DELTA, OAM_SLOTS,
};

fn sprite(x: i16, y: i16, priority: u8) -> OamEntry {
    OamEntry {
        x,
        y,
        priority,
        hidden: false,
        ..OamEntry::blank_entry()
    }
}

fn frame_of(parts: &[OamEntry], duration: u16) -> MetaFrame {
    MetaFrame::frame_from_parts(parts, duration).unwrap()
}

// --- geometry -------------------------------------------------------------

#[test]
fn shape_size_geometry_matches_hardware() {
    use SpriteShape::{Square, Tall, Wide};
    use SpriteSize::{Size0, Size1, Size2, Size3};
    assert_eq!(Square.dimensions(Size0), (8, 8));
    assert_eq!(Square.dimensions(Size1), (16, 16));
    assert_eq!(Square.dimensions(Size2), (32, 32));
    assert_eq!(Square.dimensions(Size3), (64, 64));
    assert_eq!(Wide.dimensions(Size0), (16, 8));
    assert_eq!(Wide.dimensions(Size1), (32, 8));
    assert_eq!(Wide.dimensions(Size2), (32, 16));
    assert_eq!(Wide.dimensions(Size3), (64, 32));
    assert_eq!(Tall.dimensions(Size0), (8, 16));
    assert_eq!(Tall.dimensions(Size1), (8, 32));
    assert_eq!(Tall.dimensions(Size2), (16, 32));
    assert_eq!(Tall.dimensions(Size3), (32, 64));
}

#[test]
fn display_size_doubles_only_for_affine_double() {
    let mut e = sprite(0, 0, 0);
    e.shape = SpriteShape::Wide;
    e.size = SpriteSize::Size1;
    assert_eq!(e.display_size(), (32, 8));
    e.affine = true;
    assert_eq!(e.display_size(), (32, 8));
    e.double_size = true;
    assert_eq!(e.display_size(), (64, 16));
}

// --- validation -----------------------------------------------------------

#[test]
fn blank_table_is_all_hidden_identity() {
    let t = OamTable::blank_table();
    assert_eq!(t.entries().len(), OAM_SLOTS);
    assert_eq!(t.visible_count(), 0);
    assert_eq!(t.priority_counts(), [0, 0, 0, 0]);
    for s in 0..AFFINE_SLOTS {
        assert_eq!(t.affine_params(s), Ok(AffineParams::identity_params()));
    }
}

#[test]
fn cwe_20_125_787_slot_bounds() {
    let mut t = OamTable::blank_table();
    assert!(t.set_entry(0, sprite(0, 0, 0)).is_ok());
    assert!(t.set_entry(OAM_SLOTS - 1, sprite(0, 0, 0)).is_ok());
    assert_eq!(
        t.set_entry(OAM_SLOTS, sprite(0, 0, 0)),
        Err(OamError::SlotOutOfRange { slot: OAM_SLOTS })
    );
    assert_eq!(
        t.set_entry(usize::MAX, sprite(0, 0, 0)),
        Err(OamError::SlotOutOfRange { slot: usize::MAX })
    );
    assert_eq!(
        t.entry(OAM_SLOTS),
        Err(OamError::SlotOutOfRange { slot: OAM_SLOTS })
    );
    assert_eq!(
        t.hide_slot(OAM_SLOTS),
        Err(OamError::SlotOutOfRange { slot: OAM_SLOTS })
    );
    assert_eq!(
        t.set_affine(AFFINE_SLOTS, AffineParams::identity_params()),
        Err(OamError::AffineSlotOutOfRange { slot: AFFINE_SLOTS })
    );
    assert_eq!(
        t.affine_params(usize::MAX),
        Err(OamError::AffineSlotOutOfRange { slot: usize::MAX })
    );
}

#[test]
fn cwe_20_attribute_validation() {
    let mut t = OamTable::blank_table();
    let mut e = sprite(0, 0, 4);
    assert_eq!(
        t.set_entry(0, e),
        Err(OamError::InvalidPriority { value: 4 })
    );
    e = sprite(0, 0, 0);
    e.palette_bank = 16;
    assert_eq!(
        t.set_entry(0, e),
        Err(OamError::InvalidPaletteBank { value: 16 })
    );
    e = sprite(0, 0, 0);
    e.tile = 1024;
    assert_eq!(
        t.set_entry(0, e),
        Err(OamError::InvalidTile { value: 1024 })
    );
    e = sprite(0, 0, 0);
    e.affine_slot = AFFINE_SLOTS as u8;
    assert_eq!(
        t.set_entry(0, e),
        Err(OamError::AffineSlotOutOfRange { slot: AFFINE_SLOTS })
    );
    e = sprite(0, 0, 0);
    e.double_size = true;
    assert_eq!(t.set_entry(0, e), Err(OamError::DoubleSizeWithoutAffine));
    assert_eq!(
        t.set_entry(0, sprite(COORD_MIN - 1, 0, 0)),
        Err(OamError::CoordinateOutOfRange {
            value: i32::from(COORD_MIN) - 1
        })
    );
    assert_eq!(
        t.set_entry(0, sprite(0, COORD_MAX + 1, 0)),
        Err(OamError::CoordinateOutOfRange {
            value: i32::from(COORD_MAX) + 1
        })
    );
    // table untouched by every rejected write
    assert_eq!(t, OamTable::blank_table());
}

#[test]
fn hide_slot_keeps_attributes() {
    let mut t = OamTable::blank_table();
    let mut e = sprite(5, 6, 1);
    e.tile = 77;
    t.set_entry(3, e).unwrap();
    assert_eq!(t.visible_count(), 1);
    t.hide_slot(3).unwrap();
    assert_eq!(t.visible_count(), 0);
    let kept = t.entry(3).unwrap();
    assert_eq!((kept.x, kept.y, kept.tile, kept.hidden), (5, 6, 77, true));
    t.clear_all();
    assert_eq!(t.entry(3).unwrap().tile, 0);
}

// --- affine ---------------------------------------------------------------

#[test]
fn affine_params_round_trip_through_affine2d() {
    let p = AffineParams {
        pa: 181,
        pb: -181,
        pc: 181,
        pd: 181,
    };
    let m = p.to_matrix();
    assert_eq!(m.a, 181.0 / 256.0);
    assert_eq!(m.b, 181.0 / 256.0);
    assert_eq!(m.c, -181.0 / 256.0);
    assert_eq!(m.d, 181.0 / 256.0);
    assert_eq!(AffineParams::from_matrix(&m), Ok(p));
    assert_eq!(
        AffineParams::from_matrix(&Affine2D::identity()),
        Ok(AffineParams::identity_params())
    );
}

#[test]
fn affine_quantization_rounds_and_rejects() {
    let m = Affine2D::new(0.5, 0.0, 0.0, 2.0, 99.0, 99.0);
    let p = AffineParams::from_matrix(&m).unwrap();
    assert_eq!((p.pa, p.pd), (128, 512));
    // 127.99 * 256 = 32765.4 fits; 128.0 * 256 = 32768 does not
    assert!(AffineParams::from_matrix(&Affine2D::scale_uniform(127.99)).is_ok());
    for bad in [128.0, -129.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            AffineParams::from_matrix(&Affine2D::scale_uniform(bad)),
            Err(OamError::AffineNotRepresentable)
        );
    }
}

#[test]
fn map_offset_never_overflows() {
    let p = AffineParams {
        pa: i16::MIN,
        pb: i16::MIN,
        pc: i16::MAX,
        pd: i16::MAX,
    };
    // hostile offsets must not panic in release or debug
    let _ = p.map_offset(i32::MAX, i32::MAX);
    let _ = p.map_offset(i32::MIN, i32::MIN);
    assert_eq!(AffineParams::identity_params().map_offset(5, -7), (5, -7));
}

// --- priority ordering ----------------------------------------------------

#[test]
fn draw_order_is_priority_then_index_back_to_front() {
    let mut t = OamTable::blank_table();
    t.set_entry(0, sprite(0, 0, 1)).unwrap();
    t.set_entry(1, sprite(0, 0, 0)).unwrap();
    t.set_entry(2, sprite(0, 0, 3)).unwrap();
    t.set_entry(3, sprite(0, 0, 1)).unwrap();
    t.set_entry(4, sprite(0, 0, 3)).unwrap();
    t.set_entry(9, sprite(0, 0, 2)).unwrap(); // visible
    t.set_entry(10, OamEntry::blank_entry()).unwrap(); // hidden
    assert_eq!(t.visible_count(), 6);
    assert_eq!(t.priority_counts(), [1, 2, 1, 2]);
    let mut out = [0u8; OAM_SLOTS];
    let n = t.draw_order_into(&mut out).unwrap();
    // back (priority 3, high slot first) to front (priority 0)
    assert_eq!(&out[..n], &[4, 2, 9, 3, 0, 1]);
}

#[test]
fn draw_order_is_deterministic_and_covers_full_table() {
    let mut t = OamTable::blank_table();
    for s in 0..OAM_SLOTS {
        t.set_entry(s, sprite(s as i16, 0, (s % 4) as u8)).unwrap();
    }
    let mut a = [0u8; OAM_SLOTS];
    let mut b = [0u8; OAM_SLOTS];
    assert_eq!(t.draw_order_into(&mut a), Ok(OAM_SLOTS));
    assert_eq!(t.draw_order_into(&mut b), Ok(OAM_SLOTS));
    assert_eq!(a, b);
    let mut seen = [false; OAM_SLOTS];
    for &s in &a {
        assert!(!seen[s as usize], "slot listed twice");
        seen[s as usize] = true;
    }
    // priorities never increase front-ward
    let mut last = 3u8;
    for &s in &a {
        let p = t.entry(s as usize).unwrap().priority;
        assert!(p <= last);
        last = p;
    }
}

#[test]
fn draw_order_rejects_short_buffer_without_writing() {
    let mut t = OamTable::blank_table();
    t.set_entry(0, sprite(0, 0, 0)).unwrap();
    t.set_entry(1, sprite(0, 0, 0)).unwrap();
    let mut out = [0xEEu8; 1];
    assert_eq!(
        t.draw_order_into(&mut out),
        Err(OamError::BufferTooSmall {
            required: 2,
            actual: 1
        })
    );
    assert_eq!(out, [0xEE]);
    assert_eq!(
        t.draw_order_into(&mut []),
        Err(OamError::BufferTooSmall {
            required: 2,
            actual: 0
        })
    );
}

// --- scanline evaluation --------------------------------------------------

#[test]
fn scanline_selects_in_oam_order_and_reports_drops() {
    let mut t = OamTable::blank_table();
    for s in 0..10usize {
        t.set_entry(s, sprite(0, 0, 0)).unwrap(); // rows 0..8
    }
    t.set_entry(20, sprite(0, 100, 0)).unwrap();
    let mut out = [0u8; 8];
    let r = t.scanline_sprites_into(3, 8, &mut out).unwrap();
    assert_eq!((r.selected, r.dropped), (8, 2));
    assert_eq!(out, [0, 1, 2, 3, 4, 5, 6, 7]);
    let r = t.scanline_sprites_into(8, 8, &mut out).unwrap();
    assert_eq!((r.selected, r.dropped), (0, 0)); // row 8 is outside 8x8
    let r = t.scanline_sprites_into(100, 8, &mut out).unwrap();
    assert_eq!((r.selected, r.dropped), (1, 0));
    assert_eq!(out[0], 20);
    let r = t.scanline_sprites_into(-5, 8, &mut out).unwrap();
    assert_eq!((r.selected, r.dropped), (0, 0));
}

#[test]
fn scanline_skips_hidden_and_honours_double_size() {
    let mut t = OamTable::blank_table();
    let mut e = sprite(0, 0, 0);
    e.affine = true;
    e.double_size = true; // 16x16 box
    t.set_entry(0, e).unwrap();
    let mut hidden = sprite(0, 0, 0);
    hidden.hidden = true;
    t.set_entry(1, hidden).unwrap();
    let mut out = [0u8; 4];
    assert_eq!(
        t.scanline_sprites_into(15, 4, &mut out).unwrap().selected,
        1
    );
    assert_eq!(
        t.scanline_sprites_into(16, 4, &mut out).unwrap().selected,
        0
    );
}

#[test]
fn scanline_limit_and_buffer_validation() {
    let t = OamTable::blank_table();
    let mut out = [0u8; 4];
    assert_eq!(
        t.scanline_sprites_into(0, 0, &mut out),
        Err(OamError::InvalidScanlineLimit { value: 0 })
    );
    assert_eq!(
        t.scanline_sprites_into(0, OAM_SLOTS + 1, &mut out),
        Err(OamError::InvalidScanlineLimit {
            value: OAM_SLOTS + 1
        })
    );
    assert_eq!(
        t.scanline_sprites_into(0, 8, &mut out),
        Err(OamError::BufferTooSmall {
            required: 8,
            actual: 4
        })
    );
}

#[test]
fn scanline_stats_over_a_frame() {
    let mut t = OamTable::blank_table();
    // 10 sprites stacked on lines 0..8, one more on lines 16..24
    for s in 0..10usize {
        t.set_entry(s, sprite(0, 0, 0)).unwrap();
    }
    t.set_entry(10, sprite(0, 16, 0)).unwrap();
    let st = t.scanline_stats(32, 8).unwrap();
    assert_eq!(st.max_per_line, 10);
    assert_eq!(st.overflow_lines, 8);
    assert_eq!(st.dropped_total, 16);
    let st = t.scanline_stats(32, 128).unwrap();
    assert_eq!((st.overflow_lines, st.dropped_total), (0, 0));
    assert_eq!(
        t.scanline_stats(0, 8),
        Err(OamError::InvalidScreenHeight { value: 0 })
    );
    assert_eq!(
        t.scanline_stats(MAX_SCREEN_HEIGHT + 1, 8),
        Err(OamError::InvalidScreenHeight {
            value: MAX_SCREEN_HEIGHT + 1
        })
    );
    assert!(t.scanline_stats(MAX_SCREEN_HEIGHT, 8).is_ok());
    assert_eq!(
        t.scanline_stats(8, 0),
        Err(OamError::InvalidScanlineLimit { value: 0 })
    );
}

// --- texel sampling -------------------------------------------------------

#[test]
fn texel_at_honours_flips_and_bounds() {
    let mut t = OamTable::blank_table();
    let mut e = sprite(10, 20, 0);
    t.set_entry(0, e).unwrap();
    assert_eq!(t.texel_at(0, 10, 20), Ok(Some((0, 0))));
    assert_eq!(t.texel_at(0, 17, 27), Ok(Some((7, 7))));
    assert_eq!(t.texel_at(0, 18, 20), Ok(None));
    assert_eq!(t.texel_at(0, 9, 20), Ok(None));
    assert_eq!(t.texel_at(0, 10, 19), Ok(None));
    e.h_flip = true;
    t.set_entry(1, e).unwrap();
    assert_eq!(t.texel_at(1, 10, 20), Ok(Some((7, 0))));
    e.h_flip = false;
    e.v_flip = true;
    t.set_entry(2, e).unwrap();
    assert_eq!(t.texel_at(2, 10, 20), Ok(Some((0, 7))));
    assert_eq!(
        t.texel_at(OAM_SLOTS, 0, 0),
        Err(OamError::SlotOutOfRange { slot: OAM_SLOTS })
    );
}

#[test]
fn texel_at_affine_identity_and_scale() {
    let mut t = OamTable::blank_table();
    let mut e = sprite(0, 0, 0);
    e.affine = true;
    e.affine_slot = 2;
    t.set_entry(0, e).unwrap();
    // identity: centre maps to centre
    assert_eq!(t.texel_at(0, 4, 4), Ok(Some((4, 4))));
    assert_eq!(t.texel_at(0, 0, 0), Ok(Some((0, 0))));
    // 2x zoom-in: matrix 0.5 samples half as far from the centre
    t.set_affine(
        2,
        AffineParams {
            pa: AFFINE_ONE / 2,
            pb: 0,
            pc: 0,
            pd: AFFINE_ONE / 2,
        },
    )
    .unwrap();
    assert_eq!(t.texel_at(0, 0, 0), Ok(Some((2, 2))));
    assert_eq!(t.texel_at(0, 7, 7), Ok(Some((5, 5))));
    // 2x zoom-out: samples outside the texture become transparent
    t.set_affine(
        2,
        AffineParams {
            pa: 2 * AFFINE_ONE,
            pb: 0,
            pc: 0,
            pd: 2 * AFFINE_ONE,
        },
    )
    .unwrap();
    assert_eq!(t.texel_at(0, 0, 0), Ok(None));
    assert_eq!(t.texel_at(0, 4, 4), Ok(Some((4, 4))));
}

#[test]
fn texel_at_affine_rotation_90_degrees() {
    let mut t = OamTable::blank_table();
    let mut e = sprite(0, 0, 0);
    e.affine = true;
    t.set_entry(0, e).unwrap();
    // screen (dx, dy) -> texture (-dy, dx)
    t.set_affine(
        0,
        AffineParams {
            pa: 0,
            pb: -AFFINE_ONE,
            pc: AFFINE_ONE,
            pd: 0,
        },
    )
    .unwrap();
    // offset from centre (4,4): screen (6,4) = (+2, 0) -> texture (0, +2) -> (4, 6)
    assert_eq!(t.texel_at(0, 6, 4), Ok(Some((4, 6))));
    // screen (4,6) = (0, +2) -> texture (-2, 0) -> (2, 4)
    assert_eq!(t.texel_at(0, 4, 6), Ok(Some((2, 4))));
}

#[test]
fn texel_at_double_size_box_shows_full_rotation() {
    let mut t = OamTable::blank_table();
    let mut e = sprite(0, 0, 0);
    e.affine = true;
    e.double_size = true; // 16x16 box over an 8x8 texture
    t.set_entry(0, e).unwrap();
    assert_eq!(t.texel_at(0, 8, 8), Ok(Some((4, 4))));
    assert_eq!(t.texel_at(0, 15, 15), Ok(None)); // maps to texel (11, 11)
    assert_eq!(t.texel_at(0, 4, 4), Ok(Some((0, 0))));
    assert_eq!(t.texel_at(0, 16, 0), Ok(None));
}

// --- stamping -------------------------------------------------------------

fn part(dx: i16, dy: i16, tile: u16) -> OamEntry {
    OamEntry {
        tile,
        ..sprite(dx, dy, 0)
    }
}

#[test]
fn stamp_frame_places_parts_with_offsets() {
    let f = frame_of(&[part(0, 0, 1), part(8, 0, 2), part(0, 8, 3)], 5);
    let mut t = OamTable::blank_table();
    assert_eq!(t.stamp_frame(4, &f, 100, 50, false, false), Ok(3));
    let e = t.entry(5).unwrap();
    assert_eq!((e.x, e.y, e.tile, e.hidden), (108, 50, 2, false));
    let e = t.entry(6).unwrap();
    assert_eq!((e.x, e.y, e.tile), (100, 58, 3));
    assert_eq!(t.visible_count(), 3);
}

#[test]
fn stamp_frame_mirrors_about_origin() {
    let f = frame_of(&[part(0, 0, 1), part(8, 0, 2)], 5);
    let mut t = OamTable::blank_table();
    t.stamp_frame(0, &f, 100, 50, true, false).unwrap();
    // part 0 occupied [0,8) -> [-8,0); part 1 occupied [8,16) -> [-16,-8)
    let a = t.entry(0).unwrap();
    assert_eq!((a.x, a.h_flip, a.v_flip), (92, true, false));
    let b = t.entry(1).unwrap();
    assert_eq!((b.x, b.h_flip), (84, true));
    t.stamp_frame(2, &f, 100, 50, false, true).unwrap();
    let c = t.entry(2).unwrap();
    assert_eq!((c.x, c.y, c.h_flip, c.v_flip), (100, 42, false, true));
    // mirror twice cancels flip flags
    let mut mirrored = part(0, 0, 9);
    mirrored.h_flip = true;
    let g = frame_of(&[mirrored], 1);
    t.stamp_frame(10, &g, 0, 0, true, false).unwrap();
    assert!(!t.entry(10).unwrap().h_flip);
}

#[test]
fn stamp_frame_affine_part_keeps_matrix_but_moves() {
    let mut p = part(0, 0, 1);
    p.affine = true;
    p.affine_slot = 7;
    let f = frame_of(&[p], 1);
    let mut t = OamTable::blank_table();
    t.stamp_frame(0, &f, 50, 50, true, false).unwrap();
    let e = t.entry(0).unwrap();
    assert_eq!(
        (e.x, e.h_flip, e.affine_slot, e.affine),
        (42, false, 7, true)
    );
}

#[test]
fn cwe_787_stamp_frame_is_bounded_and_atomic() {
    let f = frame_of(&[part(0, 0, 1), part(8, 0, 2), part(16, 0, 3)], 5);
    let mut t = OamTable::blank_table();
    // runs off the end of OAM
    assert_eq!(
        t.stamp_frame(OAM_SLOTS - 2, &f, 0, 0, false, false),
        Err(OamError::SlotOutOfRange { slot: OAM_SLOTS })
    );
    assert_eq!(
        t.stamp_frame(usize::MAX, &f, 0, 0, false, false),
        Err(OamError::SlotOutOfRange { slot: usize::MAX })
    );
    // third part lands out of coordinate range: nothing may be written
    assert_eq!(
        t.stamp_frame(0, &f, i32::from(COORD_MAX) - 8, 0, false, false),
        Err(OamError::CoordinateOutOfRange {
            value: i32::from(COORD_MAX) + 8
        })
    );
    assert_eq!(
        t.stamp_frame(0, &f, i32::MAX, 0, false, false),
        Err(OamError::CoordinateOutOfRange { value: i32::MAX })
    );
    assert_eq!(t, OamTable::blank_table());
    // exactly filling the table is allowed
    assert_eq!(t.stamp_frame(OAM_SLOTS - 3, &f, 0, 0, false, false), Ok(3));
}

// --- frames ----------------------------------------------------------------

#[test]
fn frame_validation() {
    assert_eq!(
        MetaFrame::frame_from_parts(&[], 1),
        Err(OamError::InvalidPartCount { count: 0 })
    );
    let too_many = [part(0, 0, 0); MAX_PARTS + 1];
    assert_eq!(
        MetaFrame::frame_from_parts(&too_many, 1),
        Err(OamError::InvalidPartCount {
            count: MAX_PARTS + 1
        })
    );
    assert_eq!(
        MetaFrame::frame_from_parts(&[part(0, 0, 0)], 0),
        Err(OamError::InvalidFrameDuration { frame: 0 })
    );
    let mut bad = part(0, 0, 0);
    bad.priority = 9;
    assert_eq!(
        MetaFrame::frame_from_parts(&[bad], 1),
        Err(OamError::InvalidPriority { value: 9 })
    );
    assert!(MetaFrame::frame_from_parts(&[part(0, 0, 0); MAX_PARTS], 1).is_ok());
    assert!(MetaFrame::blank_frame().check_frame().is_err());
    // tampered public fields are re-validated
    let mut f = frame_of(&[part(0, 0, 0)], 1);
    f.part_count = 200;
    assert_eq!(
        f.check_frame(),
        Err(OamError::InvalidPartCount { count: 200 })
    );
}

fn timed_frames<const N: usize>(durations: [u16; N]) -> [MetaFrame; N] {
    let mut frames = [MetaFrame::blank_frame(); N];
    for (i, d) in durations.iter().enumerate() {
        frames[i] = frame_of(&[part(0, 0, i as u16)], *d);
    }
    frames
}

#[test]
fn animation_validation() {
    assert_eq!(
        Animation::bind(&[], PlayMode::Loop).unwrap_err(),
        OamError::InvalidFrameCount { count: 0 }
    );
    let many = vec![frame_of(&[part(0, 0, 0)], 1); MAX_FRAMES + 1];
    assert_eq!(
        Animation::bind(&many, PlayMode::Loop).unwrap_err(),
        OamError::InvalidFrameCount {
            count: MAX_FRAMES + 1
        }
    );
    assert!(Animation::bind(&many[..MAX_FRAMES], PlayMode::Loop).is_ok());
    let mut frames = timed_frames([2, 2, 2]);
    frames[1].duration = 0;
    assert_eq!(
        Animation::bind(&frames, PlayMode::Loop).unwrap_err(),
        OamError::InvalidFrameDuration { frame: 1 }
    );
    let frames = timed_frames([2, 3, 5]);
    let anim = Animation::bind(&frames, PlayMode::OneShot).unwrap();
    assert_eq!(anim.total_ticks(), 10);
    assert_eq!(anim.mode(), PlayMode::OneShot);
    assert_eq!(anim.frames().len(), 3);
}

// --- animation sequencing ----------------------------------------------------

#[test]
fn one_shot_holds_last_frame_and_finishes() {
    let frames = timed_frames([2, 3, 1]);
    let anim = Animation::bind(&frames, PlayMode::OneShot).unwrap();
    let mut a = Animator::start();
    assert_eq!(a.frame_index(), 0);
    let s = a.advance(&anim, 1).unwrap();
    assert_eq!((s.frames_advanced, s.finished), (0, false));
    assert_eq!(a.elapsed_ticks(), 1);
    let s = a.advance(&anim, 1).unwrap();
    assert_eq!((s.frames_advanced, a.frame_index()), (1, 1));
    a.advance(&anim, 3).unwrap();
    assert_eq!(a.frame_index(), 2);
    assert!(!a.is_finished());
    let s = a.advance(&anim, 1).unwrap();
    assert!(s.finished && a.is_finished());
    assert_eq!(a.frame_index(), 2);
    assert_eq!(a.current_frame(&anim).unwrap().duration, 1);
    // further ticks are no-ops
    let before = a;
    let s = a.advance(&anim, 100).unwrap();
    assert_eq!(
        (s.frames_advanced, s.loops_completed, s.finished),
        (0, 0, true)
    );
    assert_eq!(a, before);
    a.rewind();
    assert_eq!(a, Animator::start());
}

#[test]
fn loop_wraps_and_counts_cycles() {
    let frames = timed_frames([2, 3]);
    let anim = Animation::bind(&frames, PlayMode::Loop).unwrap();
    let mut a = Animator::start();
    let s = a.advance(&anim, 5).unwrap();
    assert_eq!((s.frames_advanced, s.loops_completed), (2, 1));
    assert_eq!((a.frame_index(), a.elapsed_ticks()), (0, 0));
    assert_eq!(a.loops_completed(), 1);
    let s = a.advance(&anim, 12).unwrap(); // 2 full cycles + 2 ticks
    assert_eq!(s.loops_completed, 2);
    assert_eq!((a.frame_index(), a.elapsed_ticks()), (1, 0));
    assert_eq!(a.loops_completed(), 3);
    assert!(!a.is_finished());
}

#[test]
fn ping_pong_visits_endpoints_once() {
    let frames = timed_frames([1, 1, 1]);
    let anim = Animation::bind(&frames, PlayMode::PingPong).unwrap();
    let mut a = Animator::start();
    let mut seq = [0usize; 10];
    for slot in seq.iter_mut() {
        *slot = a.frame_index();
        a.advance(&anim, 1).unwrap();
    }
    assert_eq!(seq, [0, 1, 2, 1, 0, 1, 2, 1, 0, 1]);
    assert_eq!(a.loops_completed(), 2);
}

#[test]
fn ping_pong_two_frames_and_one_frame() {
    let frames = timed_frames([1, 1]);
    let anim = Animation::bind(&frames, PlayMode::PingPong).unwrap();
    let mut a = Animator::start();
    let mut seq = [0usize; 6];
    for slot in seq.iter_mut() {
        *slot = a.frame_index();
        a.advance(&anim, 1).unwrap();
    }
    assert_eq!(seq, [0, 1, 0, 1, 0, 1]);
    let one = timed_frames([2]);
    let anim = Animation::bind(&one, PlayMode::PingPong).unwrap();
    let mut a = Animator::start();
    a.advance(&anim, 7).unwrap();
    assert_eq!((a.frame_index(), a.loops_completed()), (0, 3));
}

#[test]
fn advance_is_split_invariant() {
    // advancing N ticks at once equals N single-tick advances
    let frames = timed_frames([3, 1, 4, 2]);
    for mode in [PlayMode::OneShot, PlayMode::Loop, PlayMode::PingPong] {
        let anim = Animation::bind(&frames, mode).unwrap();
        for total in 0..60u32 {
            let mut once = Animator::start();
            once.advance(&anim, total).unwrap();
            let mut stepped = Animator::start();
            for _ in 0..total {
                stepped.advance(&anim, 1).unwrap();
            }
            assert_eq!(once, stepped, "mode {mode:?} total {total}");
        }
    }
}

#[test]
fn cwe_400_190_tick_bounds_and_big_deltas() {
    let frames = timed_frames([1, 1]);
    let anim = Animation::bind(&frames, PlayMode::Loop).unwrap();
    let mut a = Animator::start();
    assert_eq!(
        a.advance(&anim, MAX_TICK_DELTA + 1),
        Err(OamError::TickTooLarge {
            ticks: MAX_TICK_DELTA + 1
        })
    );
    assert_eq!(
        a.advance(&anim, u32::MAX),
        Err(OamError::TickTooLarge { ticks: u32::MAX })
    );
    assert_eq!(a, Animator::start());
    let s = a.advance(&anim, MAX_TICK_DELTA).unwrap();
    assert_eq!(s.frames_advanced, MAX_TICK_DELTA);
    // max-duration frames cannot overflow elapsed
    let long = timed_frames([u16::MAX]);
    let anim = Animation::bind(&long, PlayMode::Loop).unwrap();
    let mut a = Animator::start();
    for _ in 0..4 {
        a.advance(&anim, MAX_TICK_DELTA).unwrap();
    }
    assert_eq!(a.loops_completed(), 4);
}

#[test]
fn animator_mismatch_is_rejected_without_mutation() {
    let long = timed_frames([1, 1, 1]);
    let short = timed_frames([1]);
    let a_long = Animation::bind(&long, PlayMode::Loop).unwrap();
    let a_short = Animation::bind(&short, PlayMode::Loop).unwrap();
    let mut a = Animator::start();
    a.advance(&a_long, 2).unwrap();
    assert_eq!(a.frame_index(), 2);
    let before = a;
    assert_eq!(a.advance(&a_short, 1), Err(OamError::AnimatorMismatch));
    assert_eq!(a.current_frame(&a_short), Err(OamError::AnimatorMismatch));
    assert_eq!(a, before);
    // elapsed beyond the (shorter) frame duration is also a mismatch
    let slow = timed_frames([10]);
    let quick = timed_frames([2]);
    let a_slow = Animation::bind(&slow, PlayMode::Loop).unwrap();
    let a_quick = Animation::bind(&quick, PlayMode::Loop).unwrap();
    let mut a = Animator::start();
    a.advance(&a_slow, 5).unwrap();
    assert_eq!(a.advance(&a_quick, 1), Err(OamError::AnimatorMismatch));
}

#[test]
fn animator_drives_oam_via_stamping() {
    let frames = [
        frame_of(&[part(0, 0, 10)], 2),
        frame_of(&[part(0, 0, 11), part(8, 0, 12)], 2),
    ];
    let anim = Animation::bind(&frames, PlayMode::Loop).unwrap();
    let mut a = Animator::start();
    let mut t = OamTable::blank_table();
    let f = a.current_frame(&anim).unwrap();
    assert_eq!(t.stamp_frame(0, f, 20, 20, false, false), Ok(1));
    a.advance(&anim, 2).unwrap();
    let f = a.current_frame(&anim).unwrap();
    assert_eq!(t.stamp_frame(0, f, 20, 20, false, false), Ok(2));
    assert_eq!(t.entry(0).unwrap().tile, 11);
    assert_eq!(t.entry(1).unwrap().tile, 12);
}

// --- lowering ----------------------------------------------------------------

fn bank_palette() -> Palette {
    let mut p = Palette::blank(palette_indexed::BitDepth::Bpp8);
    for i in 0..=255u8 {
        p.set_entry(i, [i, 255 - i, i / 2, 255]).unwrap();
    }
    p
}

#[test]
fn quad_id_mapping_round_trips() {
    for slot in 0..OAM_SLOTS {
        assert_eq!(slot_from_quad_id(quad_id_for_slot(slot)), Some(slot));
    }
    assert_eq!(quad_id_for_slot(0), 127);
    assert_eq!(quad_id_for_slot(127), 0);
    assert_eq!(slot_from_quad_id(128), None);
    assert_eq!(slot_from_quad_id(u32::MAX), None);
}

#[test]
fn lower_to_quads_maps_attributes() {
    let mut t = OamTable::blank_table();
    let mut e = sprite(-4, 30, 2);
    e.shape = SpriteShape::Wide;
    e.size = SpriteSize::Size1;
    e.h_flip = true;
    e.palette_bank = 3;
    t.set_entry(5, e).unwrap();
    t.set_entry(9, OamEntry::blank_entry()).unwrap(); // hidden
    let mut aff = sprite(0, 0, 0);
    aff.affine = true;
    aff.double_size = true;
    aff.h_flip = true;
    t.set_entry(7, aff).unwrap();
    let pal = bank_palette();
    let mut out = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); 8];
    let n = t.lower_to_quads_into(Some(&pal), 4, &mut out).unwrap();
    assert_eq!(n, 2);
    let q = out[0];
    assert_eq!(q.id, quad_id_for_slot(5));
    assert_eq!((q.x, q.y, q.width, q.height), (-4.0, 30.0, 32.0, 8.0));
    assert_eq!(q.z_priority, 1);
    assert!(q.h_flip && !q.v_flip && q.visible);
    assert_eq!(q.color_rgb, [52, 203, 26]); // bank 3 * 16 + 4 = 52
    let q = out[1];
    assert_eq!((q.width, q.height), (16.0, 16.0));
    assert!(!q.h_flip, "affine sprites ignore flip flags");
    assert_eq!(q.z_priority, 3);
    // no palette -> white
    let n = t.lower_to_quads_into(None, 0, &mut out).unwrap();
    assert_eq!(n, 2);
    assert_eq!(out[0].color_rgb, [255, 255, 255]);
}

#[test]
fn lowered_paint_order_matches_hardware_order() {
    // lower slot wins at equal priority; lower priority value wins overall
    let mut t = OamTable::blank_table();
    t.set_entry(0, sprite(0, 0, 1)).unwrap();
    t.set_entry(1, sprite(0, 0, 1)).unwrap();
    t.set_entry(2, sprite(0, 0, 0)).unwrap();
    t.set_entry(3, sprite(0, 0, 3)).unwrap();
    let mut order = [0u8; OAM_SLOTS];
    let n = t.draw_order_into(&mut order).unwrap();
    let mut quads = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); OAM_SLOTS];
    let q = t.lower_to_quads_into(None, 0, &mut quads).unwrap();
    assert_eq!(n, q);
    let mut sorted: Vec<SpriteQuad> = quads[..q].to_vec();
    sorted.sort_by_key(|s| (s.z_priority, s.id));
    let paint: Vec<usize> = sorted
        .iter()
        .map(|s| slot_from_quad_id(s.id).unwrap())
        .collect();
    let expect: Vec<usize> = order[..n].iter().map(|&s| s as usize).collect();
    assert_eq!(paint, expect);
    assert_eq!(paint, vec![3, 1, 0, 2]);
}

#[test]
fn lower_guards() {
    let mut t = OamTable::blank_table();
    t.set_entry(0, sprite(0, 0, 0)).unwrap();
    t.set_entry(1, sprite(0, 0, 0)).unwrap();
    let mut small = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); 1];
    assert_eq!(
        t.lower_to_quads_into(None, 0, &mut small),
        Err(OamError::BufferTooSmall {
            required: 2,
            actual: 1
        })
    );
    let mut out = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); 4];
    let pal = bank_palette();
    assert_eq!(
        t.lower_to_quads_into(Some(&pal), 16, &mut out),
        Err(OamError::PaletteEntryUnavailable { bank: 0, index: 16 })
    );
    // a 16-colour palette cannot supply bank 3
    let small_pal = Palette::blank(palette_indexed::BitDepth::Bpp4);
    assert!(bank_tint(&small_pal, 0, 1).is_ok());
    assert_eq!(
        bank_tint(&small_pal, 1, 0),
        Err(OamError::PaletteEntryUnavailable { bank: 1, index: 0 })
    );
    assert_eq!(
        bank_tint(&pal, 16, 0),
        Err(OamError::InvalidPaletteBank { value: 16 })
    );
    assert_eq!(bank_tint(&pal, 15, 15), Ok([255, 0, 127]));
}

#[test]
fn lower_to_paint_scene_builds_instructions() {
    let mut t = OamTable::blank_table();
    t.set_entry(0, sprite(8, 8, 0)).unwrap();
    t.set_entry(1, sprite(24, 8, 1)).unwrap();
    let scene = t
        .lower_to_paint_scene(256.0, 224.0, "#000000", None, 0)
        .unwrap();
    assert_eq!(scene.instructions.len(), 2);
    assert_eq!(
        t.lower_to_paint_scene(0.0, 224.0, "#000000", None, 0)
            .unwrap_err(),
        OamError::InvalidSceneSize
    );
    assert_eq!(
        t.lower_to_paint_scene(f64::NAN, 224.0, "#000000", None, 0)
            .unwrap_err(),
        OamError::InvalidSceneSize
    );
}

#[test]
fn lower_empty_table_is_empty_scene() {
    let t = OamTable::blank_table();
    let scene = t
        .lower_to_paint_scene(64.0, 64.0, "#102030", None, 0)
        .unwrap();
    assert_eq!(scene.instructions.len(), 0);
}

// --- misc ----------------------------------------------------------------------

#[test]
fn deterministic_across_runs() {
    let build = || {
        let mut t = OamTable::blank_table();
        for s in 0..OAM_SLOTS {
            t.set_entry(s, sprite((s * 3) as i16, (s * 2) as i16, (s % 4) as u8))
                .unwrap();
        }
        t
    };
    assert_eq!(build(), build());
    let t = build();
    assert_eq!(t.scanline_stats(240, 32), build().scanline_stats(240, 32));
}

#[test]
fn error_display_is_informative() {
    let errs = [
        OamError::SlotOutOfRange { slot: 200 },
        OamError::AffineSlotOutOfRange { slot: 40 },
        OamError::InvalidPriority { value: 4 },
        OamError::InvalidPaletteBank { value: 16 },
        OamError::InvalidTile { value: 2000 },
        OamError::CoordinateOutOfRange { value: 5000 },
        OamError::DoubleSizeWithoutAffine,
        OamError::InvalidScanlineLimit { value: 0 },
        OamError::InvalidScreenHeight { value: 0 },
        OamError::BufferTooSmall {
            required: 2,
            actual: 1,
        },
        OamError::InvalidFrameCount { count: 0 },
        OamError::InvalidFrameDuration { frame: 1 },
        OamError::InvalidPartCount { count: 0 },
        OamError::TickTooLarge { ticks: 99999 },
        OamError::AnimatorMismatch,
        OamError::AffineNotRepresentable,
        OamError::PaletteEntryUnavailable { bank: 1, index: 2 },
        OamError::InvalidSceneSize,
        OamError::LoweringRejected,
    ];
    for e in errs {
        assert!(!e.to_string().is_empty());
        let _: &dyn std::error::Error = &e;
    }
}
