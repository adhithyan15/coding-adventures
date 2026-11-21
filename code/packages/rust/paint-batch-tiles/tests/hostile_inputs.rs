//! Hostile-input and deterministic fuzz test suite for `paint-batch-tiles`.

#![forbid(unsafe_code)]

use paint_batch_tiles::{
    diff_paint_scenes, diff_sprite_batches_into, lower_sprites_to_paint_scene,
    lower_tile_batch_to_paint_scene, DeltaKind, PaintBase, PaintBatchError, PaintInstruction,
    PaintRect, PaintScene, SpriteDeltaOp, SpriteQuad, TileBatchCell, MAX_BATCH_SPRITES, NO_INDEX,
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
        (self.next_u64() & 0xFFFF) as u16
    }

    fn next_hostile_f64(&mut self, idx: usize) -> f64 {
        match idx % 10 {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => -0.0,
            4 => f64::from_bits(1),
            5 => -f64::from_bits(1),
            6 => f64::MAX,
            7 => f64::MIN,
            8 => 1.0e308,
            _ => ((self.next_u64() as i64) as f64) / 1.0e15,
        }
    }
}

fn make_rect(id: Option<&str>, x: f64) -> PaintInstruction {
    PaintInstruction::Rect(PaintRect {
        base: PaintBase {
            id: id.map(ToString::to_string),
            metadata: None,
        },
        x,
        y: 0.0,
        width: 8.0,
        height: 8.0,
        fill: Some("#ffffff".to_string()),
        stroke: None,
        stroke_width: None,
        corner_radius: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    })
}

#[test]
fn hostile_atomic_diff_and_tile_overflow_and_scene_limits() {
    // 1. Atomic out_ops preservation on BufferTooSmall (both sorted and unsorted paths)
    let sentinel = SpriteDeltaOp {
        kind: DeltaKind::Unchanged,
        id: 0xDEAD_BEEF,
        old_index: 99,
        new_index: NO_INDEX,
    };
    let mut out_ops = [sentinel; 2];

    let prev_sorted = [
        SpriteQuad::new(1, 0.0, 0.0, 10.0, 10.0),
        SpriteQuad::new(2, 0.0, 0.0, 10.0, 10.0),
        SpriteQuad::new(3, 0.0, 0.0, 10.0, 10.0),
    ];
    let next_sorted = [
        SpriteQuad::new(2, 1.0, 0.0, 10.0, 10.0),
        SpriteQuad::new(4, 0.0, 0.0, 10.0, 10.0),
    ];
    assert_eq!(
        diff_sprite_batches_into(&prev_sorted, &next_sorted, &mut out_ops),
        Err(PaintBatchError::BufferTooSmall {
            required: 4,
            provided: 2
        })
    );
    assert_eq!(out_ops, [sentinel; 2]);

    let prev_unsorted = [
        SpriteQuad::new(3, 0.0, 0.0, 10.0, 10.0),
        SpriteQuad::new(1, 0.0, 0.0, 10.0, 10.0),
        SpriteQuad::new(2, 0.0, 0.0, 10.0, 10.0),
    ];
    let next_unsorted = [
        SpriteQuad::new(4, 0.0, 0.0, 10.0, 10.0),
        SpriteQuad::new(2, 1.0, 0.0, 10.0, 10.0),
    ];
    assert_eq!(
        diff_sprite_batches_into(&prev_unsorted, &next_unsorted, &mut out_ops),
        Err(PaintBatchError::BufferTooSmall {
            required: 4,
            provided: 2
        })
    );
    assert_eq!(out_ops, [sentinel; 2]);

    // 2. 1e308 tile coordinate overflow -> Err instead of +Inf PaintRect
    let cells = [TileBatchCell::new(2, 0, 1)];
    assert!(matches!(
        lower_tile_batch_to_paint_scene(100.0, 100.0, 1.0e308, 16.0, "#000000", &cells),
        Err(PaintBatchError::InvalidTileDimensions { .. })
    ));

    // 3. Duplicate IDs & > MAX_BATCH_SPRITES
    let dup = [
        SpriteQuad::new(7, 0.0, 0.0, 4.0, 4.0),
        SpriteQuad::new(7, 1.0, 1.0, 4.0, 4.0),
    ];
    assert_eq!(
        diff_sprite_batches_into(&dup, &[], &mut out_ops),
        Err(PaintBatchError::DuplicateSpriteId(7))
    );
    let oversized: Vec<SpriteQuad> = (0..=MAX_BATCH_SPRITES)
        .map(|i| SpriteQuad::new(i as u32, 0.0, 0.0, 1.0, 1.0))
        .collect();
    assert!(matches!(
        lower_sprites_to_paint_scene(100.0, 100.0, "#000000", &oversized),
        Err(PaintBatchError::BatchTooLarge { .. })
    ));

    // 4. diff_paint_scenes missing ID, duplicate ID, and oversized scene
    let scene_ok = PaintScene {
        width: 100.0,
        height: 100.0,
        background: "#000000".to_string(),
        instructions: vec![make_rect(Some("a"), 0.0)],
        id: None,
        metadata: None,
    };
    let scene_missing = PaintScene {
        instructions: vec![make_rect(None, 0.0)],
        ..scene_ok.clone()
    };
    assert_eq!(
        diff_paint_scenes(&scene_missing, &scene_ok),
        Err(PaintBatchError::MissingInstructionId)
    );
    let scene_too_big = PaintScene {
        instructions: (0..=MAX_BATCH_SPRITES)
            .map(|i| make_rect(Some(&format!("r_{i}")), i as f64))
            .collect(),
        ..scene_ok.clone()
    };
    assert!(matches!(
        diff_paint_scenes(&scene_too_big, &scene_ok),
        Err(PaintBatchError::BatchTooLarge { .. })
    ));
}

#[test]
fn hostile_fuzz_campaign_1500_cases() {
    let mut rng = SplitMix64(0x9A1B_47C0_0000_0001);
    let mut ops = [SpriteDeltaOp {
        kind: DeltaKind::Unchanged,
        id: 0,
        old_index: 0,
        new_index: 0,
    }; 16];

    for i in 0..1_500 {
        let sw = rng.next_hostile_f64(i);
        let sh = rng.next_hostile_f64(i + 1);
        let tw = rng.next_hostile_f64(i + 2);
        let th = rng.next_hostile_f64(i + 3);

        let n_sprites = (rng.next_u32() as usize) % 12;
        let sprites: Vec<SpriteQuad> = (0..n_sprites)
            .map(|k| SpriteQuad {
                id: rng.next_u32() % 16,
                x: rng.next_hostile_f64(i + k),
                y: rng.next_hostile_f64(i + k + 1),
                width: rng.next_hostile_f64(i + k + 2),
                height: rng.next_hostile_f64(i + k + 3),
                z_priority: (rng.next_u32() & 0xFF) as u8,
                h_flip: (k & 1) == 0,
                v_flip: (k & 2) == 0,
                color_rgb: [255, 128, 64],
                visible: k % 3 != 0,
            })
            .collect();
        let next_sprites: Vec<SpriteQuad> = (0..((rng.next_u32() as usize) % 12))
            .map(|_| SpriteQuad::new(rng.next_u32() % 16, 0.0, 0.0, 4.0, 4.0))
            .collect();

        let _ = lower_sprites_to_paint_scene(sw, sh, "#000000", &sprites);
        let cap = (rng.next_u32() as usize) % 16;
        let _ = diff_sprite_batches_into(&sprites, &next_sprites, &mut ops[..cap]);

        let cells: Vec<TileBatchCell> = (0..((rng.next_u32() as usize) % 8))
            .map(|_| TileBatchCell::new(rng.next_u16(), rng.next_u16(), rng.next_u16()))
            .collect();
        let _ = lower_tile_batch_to_paint_scene(sw, sh, tw, th, "#000000", &cells);

        if i % 10 == 0 {
            let s1 = PaintScene {
                width: 100.0,
                height: 100.0,
                background: "#000000".to_string(),
                instructions: (0..4)
                    .map(|k| {
                        let id = format!("id_{}", rng.next_u32() % 5);
                        make_rect(
                            if k == 0 && i % 20 == 0 {
                                None
                            } else {
                                Some(id.as_str())
                            },
                            k as f64,
                        )
                    })
                    .collect(),
                id: None,
                metadata: None,
            };
            let s2 = PaintScene {
                width: 100.0,
                height: 100.0,
                background: "#000000".to_string(),
                instructions: (0..4)
                    .map(|k| {
                        let id = format!("id_{}", rng.next_u32() % 5);
                        make_rect(Some(id.as_str()), k as f64)
                    })
                    .collect(),
                id: None,
                metadata: None,
            };
            let _ = diff_paint_scenes(&s1, &s2);
        }
    }
}
