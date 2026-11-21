//! Hostile-input and deterministic fuzz test suite for `sprite-oam`.

#![forbid(unsafe_code)]

use palette_indexed::{BitDepth, Palette};
use sprite_oam::{
    bank_tint, Affine2D, AffineParams, Animation, Animator, MetaFrame, OamEntry, OamError,
    OamTable, PlayMode, SpriteShape, SpriteSize, AFFINE_SLOTS, COORD_MAX, COORD_MIN, MAX_FRAMES,
    MAX_PALETTE_BANK, MAX_PARTS, MAX_PRIORITY, MAX_SCREEN_HEIGHT, MAX_TICK_DELTA, MAX_TILE_INDEX,
    OAM_SLOTS,
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

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }

    fn next_i16(&mut self) -> i16 {
        self.next_u16() as i16
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

fn valid_visible_entry() -> OamEntry {
    let mut e = OamEntry::blank_entry();
    e.hidden = false;
    e.x = 16;
    e.y = 24;
    e
}

#[test]
fn hostile_oob_slots_coords_buffers_and_affine() {
    let mut table = OamTable::blank_table();
    let e = valid_visible_entry();

    // OOB slot indices.
    assert_eq!(
        table.entry(OAM_SLOTS),
        Err(OamError::SlotOutOfRange { slot: OAM_SLOTS })
    );
    assert_eq!(
        table.entry(usize::MAX),
        Err(OamError::SlotOutOfRange { slot: usize::MAX })
    );
    assert_eq!(
        table.set_entry(OAM_SLOTS, e),
        Err(OamError::SlotOutOfRange { slot: OAM_SLOTS })
    );
    assert_eq!(
        table.hide_slot(usize::MAX),
        Err(OamError::SlotOutOfRange { slot: usize::MAX })
    );
    assert_eq!(
        table.affine_params(AFFINE_SLOTS),
        Err(OamError::AffineSlotOutOfRange { slot: AFFINE_SLOTS })
    );
    assert_eq!(
        table.set_affine(usize::MAX, AffineParams::identity_params()),
        Err(OamError::AffineSlotOutOfRange { slot: usize::MAX })
    );
    assert_eq!(
        table.texel_at(OAM_SLOTS, 0, 0),
        Err(OamError::SlotOutOfRange { slot: OAM_SLOTS })
    );

    // Invalid entry attributes rejected.
    let mut bad = e;
    bad.x = COORD_MIN - 1;
    assert!(matches!(
        bad.check_entry(),
        Err(OamError::CoordinateOutOfRange { .. })
    ));
    bad = e;
    bad.y = COORD_MAX + 1;
    assert!(matches!(
        bad.check_entry(),
        Err(OamError::CoordinateOutOfRange { .. })
    ));
    bad = e;
    bad.priority = MAX_PRIORITY + 1;
    assert!(matches!(
        bad.check_entry(),
        Err(OamError::InvalidPriority { .. })
    ));
    bad = e;
    bad.palette_bank = MAX_PALETTE_BANK + 1;
    assert!(matches!(
        bad.check_entry(),
        Err(OamError::InvalidPaletteBank { .. })
    ));
    bad = e;
    bad.tile = MAX_TILE_INDEX + 1;
    assert!(matches!(
        bad.check_entry(),
        Err(OamError::InvalidTile { .. })
    ));
    bad = e;
    bad.affine_slot = AFFINE_SLOTS as u8;
    assert!(matches!(
        bad.check_entry(),
        Err(OamError::AffineSlotOutOfRange { .. })
    ));
    bad = e;
    bad.double_size = true;
    bad.affine = false;
    assert_eq!(bad.check_entry(), Err(OamError::DoubleSizeWithoutAffine));

    // Extreme i32::MIN / i32::MAX screen coordinates on texel_at.
    table.set_entry(0, e).unwrap();
    assert_eq!(table.texel_at(0, i32::MIN, i32::MAX), Ok(None));
    assert_eq!(table.texel_at(0, i32::MAX, i32::MIN), Ok(None));
    let mut aff = e;
    aff.affine = true;
    table.set_entry(1, aff).unwrap();
    table
        .set_affine(
            0,
            AffineParams {
                pa: i16::MAX,
                pb: i16::MIN,
                pc: i16::MIN,
                pd: i16::MAX,
            },
        )
        .unwrap();
    assert!(table.texel_at(1, 16, 24).is_ok());

    // Non-finite and extreme Affine2D matrices.
    for m in [
        Affine2D::new(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0),
        Affine2D::new(1.0, f64::INFINITY, 0.0, 1.0, 0.0, 0.0),
        Affine2D::new(1.0, 0.0, f64::NEG_INFINITY, 1.0, 0.0, 0.0),
        Affine2D::new(1e9, 0.0, 0.0, 1.0, 0.0, 0.0),
        Affine2D::new(-1e9, 0.0, 0.0, 1.0, 0.0, 0.0),
    ] {
        assert_eq!(
            AffineParams::from_matrix(&m),
            Err(OamError::AffineNotRepresentable)
        );
    }

    // Truncated draw_order_into / scanline_sprites_into buffers leave buffer untouched.
    table.set_entry(2, e).unwrap();
    let mut tiny_order = [0xAAu8; 2];
    assert!(matches!(
        table.draw_order_into(&mut tiny_order),
        Err(OamError::BufferTooSmall {
            required: 3,
            actual: 2
        })
    ));
    assert_eq!(tiny_order, [0xAAu8; 2]);

    assert_eq!(
        table.scanline_sprites_into(0, 0, &mut tiny_order),
        Err(OamError::InvalidScanlineLimit { value: 0 })
    );
    assert_eq!(
        table.scanline_sprites_into(0, OAM_SLOTS + 1, &mut tiny_order),
        Err(OamError::InvalidScanlineLimit {
            value: OAM_SLOTS + 1
        })
    );
    assert_eq!(
        table.scanline_sprites_into(0, 8, &mut tiny_order),
        Err(OamError::BufferTooSmall {
            required: 8,
            actual: 2
        })
    );
    assert_eq!(tiny_order, [0xAAu8; 2]);

    // scanline_stats bounds.
    assert_eq!(
        table.scanline_stats(0, 8),
        Err(OamError::InvalidScreenHeight { value: 0 })
    );
    assert_eq!(
        table.scanline_stats(MAX_SCREEN_HEIGHT + 1, 8),
        Err(OamError::InvalidScreenHeight {
            value: MAX_SCREEN_HEIGHT + 1
        })
    );

    // Palette bank_tint & lower_to_paint_scene bounds.
    let small_pal = Palette::blank(BitDepth::Bpp4);
    assert_eq!(
        bank_tint(&small_pal, 1, 0),
        Err(OamError::PaletteEntryUnavailable { bank: 1, index: 0 })
    );
    assert_eq!(
        bank_tint(&small_pal, 0, 16),
        Err(OamError::PaletteEntryUnavailable { bank: 0, index: 16 })
    );
    assert_eq!(
        table.lower_to_paint_scene(f64::NAN, 160.0, "#000000", None, 0),
        Err(OamError::InvalidSceneSize)
    );
}

#[test]
fn hostile_atomic_stamp_frame_rollback_and_animation_bounds() {
    let mut table = OamTable::blank_table();
    let baseline = valid_visible_entry();
    for slot in 0..4 {
        table.set_entry(slot, baseline).unwrap();
    }
    let snapshot = table.clone();

    // Part 0 valid, part 1 lands out of COORD_MAX after translation -> atomic rollback!
    let mut p0 = baseline;
    p0.x = 10;
    let mut p1 = baseline;
    p1.x = COORD_MAX;
    let frame = MetaFrame::frame_from_parts(&[p0, p1], 4).unwrap();
    assert!(matches!(
        table.stamp_frame(0, &frame, 1, 0, false, false),
        Err(OamError::CoordinateOutOfRange { .. })
    ));
    assert_eq!(table, snapshot);

    // Slot range overflow (127 + 2 > 128 and usize::MAX) -> atomic rollback!
    assert!(matches!(
        table.stamp_frame(OAM_SLOTS - 1, &frame, 0, 0, false, false),
        Err(OamError::SlotOutOfRange { .. })
    ));
    assert!(matches!(
        table.stamp_frame(usize::MAX, &frame, 0, 0, false, false),
        Err(OamError::SlotOutOfRange { .. })
    ));
    assert_eq!(table, snapshot);

    // Frame / animation / animator bounds.
    assert_eq!(
        MetaFrame::frame_from_parts(&[], 1),
        Err(OamError::InvalidPartCount { count: 0 })
    );
    let too_many_parts = [baseline; MAX_PARTS + 1];
    assert_eq!(
        MetaFrame::frame_from_parts(&too_many_parts, 1),
        Err(OamError::InvalidPartCount {
            count: MAX_PARTS + 1
        })
    );
    assert_eq!(
        MetaFrame::frame_from_parts(&[baseline], 0),
        Err(OamError::InvalidFrameDuration { frame: 0 })
    );
    assert!(matches!(
        Animation::bind(&[], PlayMode::Loop),
        Err(OamError::InvalidFrameCount { count: 0 })
    ));
    let too_many_frames = [frame; MAX_FRAMES + 1];
    assert!(matches!(
        Animation::bind(&too_many_frames, PlayMode::Loop),
        Err(OamError::InvalidFrameCount { count }) if count == MAX_FRAMES + 1
    ));

    let frames2 = [frame, frame];
    let anim2 = Animation::bind(&frames2, PlayMode::PingPong).unwrap();
    let mut animator = Animator::start();
    assert_eq!(
        animator.advance(&anim2, MAX_TICK_DELTA + 1),
        Err(OamError::TickTooLarge {
            ticks: MAX_TICK_DELTA + 1
        })
    );
    animator.advance(&anim2, 5).unwrap();
    let frames1 = [frame];
    let anim1 = Animation::bind(&frames1, PlayMode::OneShot).unwrap();
    assert_eq!(animator.advance(&anim1, 1), Err(OamError::AnimatorMismatch));
}

#[test]
fn hostile_fuzz_1000_cases() {
    let mut rng = SplitMix64(0x2026_0022_1234_5678);
    let shapes = [SpriteShape::Square, SpriteShape::Wide, SpriteShape::Tall];
    let sizes = [
        SpriteSize::Size0,
        SpriteSize::Size1,
        SpriteSize::Size2,
        SpriteSize::Size3,
    ];
    let modes = [PlayMode::OneShot, PlayMode::Loop, PlayMode::PingPong];

    let mut table = OamTable::blank_table();
    let mut order_buf = [0u8; OAM_SLOTS];
    let mut scan_buf = [0u8; OAM_SLOTS];
    let mut quads = [sprite_oam::SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); OAM_SLOTS];
    let pal = Palette::blank(BitDepth::Bpp8);

    for _ in 0..1000 {
        let entry = OamEntry {
            x: rng.next_i16(),
            y: rng.next_i16(),
            shape: shapes[(rng.next_u8() as usize) % shapes.len()],
            size: sizes[(rng.next_u8() as usize) % sizes.len()],
            h_flip: rng.next_u8() & 1 != 0,
            v_flip: rng.next_u8() & 2 != 0,
            palette_bank: rng.next_u8() % 20,
            priority: rng.next_u8() % 6,
            affine_slot: rng.next_u8() % 40,
            tile: rng.next_u16() % 1200,
            hidden: rng.next_u8().is_multiple_of(5),
            affine: rng.next_u8() & 4 != 0,
            double_size: rng.next_u8() & 8 != 0,
        };
        let slot = (rng.next_u16() as usize) % (OAM_SLOTS + 16);
        let _ = table.set_entry(slot, entry);
        let _ = table.set_affine(
            (rng.next_u8() as usize) % (AFFINE_SLOTS + 4),
            AffineParams {
                pa: rng.next_i16(),
                pb: rng.next_i16(),
                pc: rng.next_i16(),
                pd: rng.next_i16(),
            },
        );

        let _ = table.texel_at(slot, rng.next_i32(), rng.next_i32());
        let out_slice_len = (rng.next_u8() as usize) % (OAM_SLOTS + 1);
        let _ = table.draw_order_into(&mut order_buf[..out_slice_len]);
        let _ = table.scanline_sprites_into(
            rng.next_i32(),
            (rng.next_u8() as usize) % (OAM_SLOTS + 8),
            &mut scan_buf[..out_slice_len],
        );
        let _ =
            table.lower_to_quads_into(Some(&pal), rng.next_u8() % 20, &mut quads[..out_slice_len]);
        let _ = table.lower_to_paint_scene(240.0, 160.0, "#000000", Some(&pal), rng.next_u8() % 20);

        if let Ok(frame) =
            MetaFrame::frame_from_parts(&[valid_visible_entry()], (rng.next_u16() % 10).max(1))
        {
            let _ = table.stamp_frame(
                slot,
                &frame,
                rng.next_i32(),
                rng.next_i32(),
                rng.next_u8() & 1 != 0,
                rng.next_u8() & 2 != 0,
            );
            let frames = [frame; 3];
            let mode = modes[(rng.next_u8() as usize) % modes.len()];
            if let Ok(anim) = Animation::bind(&frames, mode) {
                let mut anim_state = Animator::start();
                let _ = anim_state.advance(&anim, rng.next_u32() % (MAX_TICK_DELTA + 10));
            }
        }
    }
}
