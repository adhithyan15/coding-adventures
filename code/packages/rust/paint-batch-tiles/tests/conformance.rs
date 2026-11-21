//! # Unit and Conformance Tests for Paint Batch Tiles
//!
//! Covers:
//! - Sprite lowering, z-order priority sorting, visibility filtering, and attribute mapping
//! - Zero-allocation sprite batch diffing (Added, Modified, Removed, Unchanged) for both sorted and unsorted batches
//! - Retained-mode `PaintScene` diffing by stable `PaintBase.id`
//! - Tile batch lowering to grid cells with row-major ordering and metadata
//! - Buffer too small handling in `diff_sprite_batches_into`
//! - CWE-400 batch item cap enforcement (>4096 items rejected)
//! - CWE-20 input validation: non-finite, negative, zero dimensions, and duplicate IDs

use paint_batch_tiles::{
    diff_paint_scenes, diff_sprite_batches_into, lower_sprites_to_paint_scene,
    lower_tile_batch_to_paint_scene, DeltaKind, PaintBase, PaintBatchError, PaintInstruction,
    PaintRect, PaintScene, SpriteDeltaOp, SpriteQuad, TileBatchCell, MAX_BATCH_SPRITES, NO_INDEX,
};

#[test]
fn test_sprite_lowering_and_z_order() {
    let sprites = vec![
        SpriteQuad::new(10, 100.0, 50.0, 16.0, 16.0)
            .with_z_priority(2)
            .with_color([255, 0, 0]),
        SpriteQuad::new(5, 20.0, 30.0, 8.0, 8.0)
            .with_z_priority(1)
            .with_color([0, 255, 0]),
        SpriteQuad::new(20, 10.0, 10.0, 32.0, 32.0)
            .with_z_priority(1)
            .with_color([0, 0, 255]),
        SpriteQuad::new(99, 0.0, 0.0, 16.0, 16.0)
            .with_z_priority(0)
            .with_visible(false), // invisible should be skipped
    ];

    let scene = lower_sprites_to_paint_scene(320.0, 240.0, "#000000", &sprites)
        .expect("lowering should succeed");

    assert_eq!(scene.width, 320.0);
    assert_eq!(scene.height, 240.0);
    assert_eq!(scene.background, "#000000");
    // Invisible sprite 99 must be skipped: 3 instructions remain
    assert_eq!(scene.instructions.len(), 3);

    // Verify ordering: priority 1 (id 5, then id 20), priority 2 (id 10)
    let ids: Vec<Option<String>> = scene
        .instructions
        .iter()
        .map(|instr| match instr {
            PaintInstruction::Rect(r) => r.base.id.clone(),
            _ => None,
        })
        .collect();

    assert_eq!(
        ids,
        vec![
            Some("sprite:5".to_string()),
            Some("sprite:20".to_string()),
            Some("sprite:10".to_string()),
        ]
    );

    // Check properties of first rect (id 5)
    if let PaintInstruction::Rect(r) = &scene.instructions[0] {
        assert_eq!(r.x, 20.0);
        assert_eq!(r.y, 30.0);
        assert_eq!(r.width, 8.0);
        assert_eq!(r.height, 8.0);
        assert_eq!(r.fill, Some("#00ff00".to_string()));
    } else {
        panic!("expected PaintInstruction::Rect");
    }
}

#[test]
fn test_sprite_lowering_flip_metadata() {
    let sprites = vec![
        SpriteQuad::new(1, 0.0, 0.0, 16.0, 16.0).with_flip(true, false),
        SpriteQuad::new(2, 16.0, 0.0, 16.0, 16.0).with_flip(false, true),
        SpriteQuad::new(3, 32.0, 0.0, 16.0, 16.0).with_flip(true, true),
        SpriteQuad::new(4, 48.0, 0.0, 16.0, 16.0).with_flip(false, false),
    ];

    let scene = lower_sprites_to_paint_scene(100.0, 100.0, "#ffffff", &sprites)
        .expect("lowering should succeed");

    assert_eq!(scene.instructions.len(), 4);

    if let PaintInstruction::Rect(r) = &scene.instructions[0] {
        let meta = r.base.metadata.as_ref().expect("metadata present");
        assert_eq!(meta.get("h_flip").map(|s| s.as_str()), Some("true"));
        assert!(!meta.contains_key("v_flip"));
    }
    if let PaintInstruction::Rect(r) = &scene.instructions[1] {
        let meta = r.base.metadata.as_ref().expect("metadata present");
        assert!(!meta.contains_key("h_flip"));
        assert_eq!(meta.get("v_flip").map(|s| s.as_str()), Some("true"));
    }
    if let PaintInstruction::Rect(r) = &scene.instructions[2] {
        let meta = r.base.metadata.as_ref().expect("metadata present");
        assert_eq!(meta.get("h_flip").map(|s| s.as_str()), Some("true"));
        assert_eq!(meta.get("v_flip").map(|s| s.as_str()), Some("true"));
    }
    if let PaintInstruction::Rect(r) = &scene.instructions[3] {
        assert!(r.base.metadata.is_none());
    }
}

#[test]
fn test_zero_alloc_sprite_diff_sorted() {
    let prev = vec![
        SpriteQuad::new(1, 10.0, 10.0, 16.0, 16.0), // Unchanged
        SpriteQuad::new(2, 20.0, 20.0, 16.0, 16.0), // Modified (will move)
        SpriteQuad::new(3, 30.0, 30.0, 16.0, 16.0), // Removed
    ];

    let next = vec![
        SpriteQuad::new(1, 10.0, 10.0, 16.0, 16.0), // Unchanged
        SpriteQuad::new(2, 25.0, 20.0, 16.0, 16.0), // Modified
        SpriteQuad::new(4, 40.0, 40.0, 16.0, 16.0), // Added
    ];

    let mut ops = [SpriteDeltaOp {
        kind: DeltaKind::Unchanged,
        id: 0,
        old_index: 0,
        new_index: 0,
    }; 16];

    let count = diff_sprite_batches_into(&prev, &next, &mut ops).expect("diff should succeed");

    assert_eq!(count, 4);

    // Sorted by ID diff should produce:
    // id 1: Unchanged (old: 0, new: 0)
    // id 2: Modified (old: 1, new: 1)
    // id 3: Removed (old: 2, new: NO_INDEX)
    // id 4: Added (old: NO_INDEX, new: 2)
    assert_eq!(
        &ops[..count],
        &[
            SpriteDeltaOp {
                kind: DeltaKind::Unchanged,
                id: 1,
                old_index: 0,
                new_index: 0,
            },
            SpriteDeltaOp {
                kind: DeltaKind::Modified,
                id: 2,
                old_index: 1,
                new_index: 1,
            },
            SpriteDeltaOp {
                kind: DeltaKind::Removed,
                id: 3,
                old_index: 2,
                new_index: NO_INDEX,
            },
            SpriteDeltaOp {
                kind: DeltaKind::Added,
                id: 4,
                old_index: NO_INDEX,
                new_index: 2,
            },
        ]
    );
}

#[test]
fn test_zero_alloc_sprite_diff_unsorted() {
    let prev = vec![
        SpriteQuad::new(3, 30.0, 30.0, 16.0, 16.0), // Removed
        SpriteQuad::new(1, 10.0, 10.0, 16.0, 16.0), // Unchanged
    ];

    let next = vec![
        SpriteQuad::new(4, 40.0, 40.0, 16.0, 16.0), // Added
        SpriteQuad::new(1, 10.0, 10.0, 16.0, 16.0), // Unchanged
    ];

    let mut ops = [SpriteDeltaOp {
        kind: DeltaKind::Unchanged,
        id: 0,
        old_index: 0,
        new_index: 0,
    }; 16];

    let count = diff_sprite_batches_into(&prev, &next, &mut ops).expect("diff should succeed");

    assert_eq!(count, 3);

    // Verify ops contain Removed id 3, Unchanged id 1, Added id 4
    let mut has_removed_3 = false;
    let mut has_unchanged_1 = false;
    let mut has_added_4 = false;

    for op in &ops[..count] {
        if op.id == 3
            && op.kind == DeltaKind::Removed
            && op.old_index == 0
            && op.new_index == NO_INDEX
        {
            has_removed_3 = true;
        }
        if op.id == 1 && op.kind == DeltaKind::Unchanged && op.old_index == 1 && op.new_index == 1 {
            has_unchanged_1 = true;
        }
        if op.id == 4
            && op.kind == DeltaKind::Added
            && op.old_index == NO_INDEX
            && op.new_index == 0
        {
            has_added_4 = true;
        }
    }

    assert!(has_removed_3);
    assert!(has_unchanged_1);
    assert!(has_added_4);
}

#[test]
fn test_paint_scene_diffing_by_id() {
    let mut prev = PaintScene::new(320.0, 240.0);
    prev.instructions = vec![
        PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some("wall:top".to_string()),
                metadata: None,
            },
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 8.0,
            fill: Some("#ffffff".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some("paddle:left".to_string()),
                metadata: None,
            },
            x: 16.0,
            y: 100.0,
            width: 8.0,
            height: 40.0,
            fill: Some("#ffffff".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some("ball".to_string()),
                metadata: None,
            },
            x: 160.0,
            y: 120.0,
            width: 8.0,
            height: 8.0,
            fill: Some("#ffffff".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
    ];

    let mut next = PaintScene::new(320.0, 240.0);
    next.instructions = vec![
        // wall:top unchanged
        PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some("wall:top".to_string()),
                metadata: None,
            },
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 8.0,
            fill: Some("#ffffff".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        // paddle:left modified (y changed to 104.0)
        PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some("paddle:left".to_string()),
                metadata: None,
            },
            x: 16.0,
            y: 104.0,
            width: 8.0,
            height: 40.0,
            fill: Some("#ffffff".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        // ball removed, score added
        PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some("hud:score".to_string()),
                metadata: None,
            },
            x: 100.0,
            y: 20.0,
            width: 30.0,
            height: 20.0,
            fill: Some("#ffff00".to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
    ];

    let patch = diff_paint_scenes(&prev, &next).expect("diff_paint_scenes should succeed");

    assert_eq!(patch.unchanged_count, 1);
    assert_eq!(patch.modified_ids, vec!["paddle:left".to_string()]);
    assert_eq!(patch.added_ids, vec!["hud:score".to_string()]);
    assert_eq!(patch.removed_ids, vec!["ball".to_string()]);
}

#[test]
fn test_tile_batch_lowering() {
    let cells = vec![
        TileBatchCell::new(1, 0, 10).with_color([255, 0, 0]),
        TileBatchCell::new(0, 0, 5).with_color([0, 255, 0]),
        TileBatchCell::new(0, 1, 20).with_color([0, 0, 255]),
    ];

    let scene = lower_tile_batch_to_paint_scene(256.0, 256.0, 16.0, 16.0, "#000000", &cells)
        .expect("lowering tile batch should succeed");

    assert_eq!(scene.instructions.len(), 3);

    // Verify row-major sorting: (0, 0), (1, 0), (0, 1)
    let ids: Vec<Option<String>> = scene
        .instructions
        .iter()
        .map(|instr| match instr {
            PaintInstruction::Rect(r) => r.base.id.clone(),
            _ => None,
        })
        .collect();

    assert_eq!(
        ids,
        vec![
            Some("tile:0:0".to_string()),
            Some("tile:1:0".to_string()),
            Some("tile:0:1".to_string()),
        ]
    );

    // Check pixel positions:
    // tile 0:0 -> x=0, y=0
    // tile 1:0 -> x=16, y=0
    // tile 0:1 -> x=0, y=16
    if let PaintInstruction::Rect(r) = &scene.instructions[0] {
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.width, 16.0);
        assert_eq!(r.height, 16.0);
    }
    if let PaintInstruction::Rect(r) = &scene.instructions[1] {
        assert_eq!(r.x, 16.0);
        assert_eq!(r.y, 0.0);
    }
    if let PaintInstruction::Rect(r) = &scene.instructions[2] {
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 16.0);
    }
}

#[test]
fn test_cwe_400_batch_cap() {
    // 4097 sprites exceeds MAX_BATCH_SPRITES
    let oversized_sprites: Vec<SpriteQuad> = (0..=MAX_BATCH_SPRITES)
        .map(|i| SpriteQuad::new(i as u32, 0.0, 0.0, 8.0, 8.0))
        .collect();

    assert_eq!(oversized_sprites.len(), 4097);

    let res_lowering = lower_sprites_to_paint_scene(100.0, 100.0, "#ffffff", &oversized_sprites);
    assert!(matches!(
        res_lowering,
        Err(PaintBatchError::BatchTooLarge {
            count: 4097,
            max: MAX_BATCH_SPRITES
        })
    ));

    let mut ops = [SpriteDeltaOp {
        kind: DeltaKind::Unchanged,
        id: 0,
        old_index: 0,
        new_index: 0,
    }; 1];

    let res_diff = diff_sprite_batches_into(&oversized_sprites, &[], &mut ops);
    assert!(matches!(
        res_diff,
        Err(PaintBatchError::BatchTooLarge {
            count: 4097,
            max: MAX_BATCH_SPRITES
        })
    ));

    // 4097 tile cells
    let oversized_cells: Vec<TileBatchCell> = (0..=MAX_BATCH_SPRITES)
        .map(|i| TileBatchCell::new((i % 64) as u16, (i / 64) as u16, 1))
        .collect();

    let res_tile_lowering =
        lower_tile_batch_to_paint_scene(100.0, 100.0, 8.0, 8.0, "#ffffff", &oversized_cells);
    assert!(matches!(
        res_tile_lowering,
        Err(PaintBatchError::BatchTooLarge {
            count: 4097,
            max: MAX_BATCH_SPRITES
        })
    ));
}

#[test]
fn test_cwe_20_nan_inf_negative_dimensions() {
    let sprites = vec![SpriteQuad::new(1, 0.0, 0.0, 16.0, 16.0)];

    // NaN viewport
    assert!(matches!(
        lower_sprites_to_paint_scene(f64::NAN, 100.0, "#fff", &sprites),
        Err(PaintBatchError::InvalidDimensions { .. })
    ));
    assert!(matches!(
        lower_sprites_to_paint_scene(100.0, f64::NAN, "#fff", &sprites),
        Err(PaintBatchError::InvalidDimensions { .. })
    ));

    // Inf viewport
    assert!(matches!(
        lower_sprites_to_paint_scene(f64::INFINITY, 100.0, "#fff", &sprites),
        Err(PaintBatchError::InvalidDimensions { .. })
    ));

    // Negative / zero viewport
    assert!(matches!(
        lower_sprites_to_paint_scene(-100.0, 100.0, "#fff", &sprites),
        Err(PaintBatchError::InvalidDimensions { .. })
    ));
    assert!(matches!(
        lower_sprites_to_paint_scene(100.0, 0.0, "#fff", &sprites),
        Err(PaintBatchError::InvalidDimensions { .. })
    ));

    // Corrupted sprite coordinates/dimensions
    let bad_x = vec![SpriteQuad::new(1, f64::NAN, 0.0, 16.0, 16.0)];
    assert!(matches!(
        lower_sprites_to_paint_scene(100.0, 100.0, "#fff", &bad_x),
        Err(PaintBatchError::InvalidCoordinates { id: 1, .. })
    ));

    let bad_w = vec![SpriteQuad::new(2, 0.0, 0.0, -5.0, 16.0)];
    assert!(matches!(
        lower_sprites_to_paint_scene(100.0, 100.0, "#fff", &bad_w),
        Err(PaintBatchError::InvalidCoordinates { id: 2, .. })
    ));

    // Corrupted tile dimensions
    let cells = vec![TileBatchCell::new(0, 0, 1)];
    assert!(matches!(
        lower_tile_batch_to_paint_scene(100.0, 100.0, f64::NAN, 8.0, "#fff", &cells),
        Err(PaintBatchError::InvalidTileDimensions { .. })
    ));
    assert!(matches!(
        lower_tile_batch_to_paint_scene(100.0, 100.0, 0.0, 8.0, "#fff", &cells),
        Err(PaintBatchError::InvalidTileDimensions { .. })
    ));
    assert!(matches!(
        lower_tile_batch_to_paint_scene(100.0, 100.0, 8.0, -8.0, "#fff", &cells),
        Err(PaintBatchError::InvalidTileDimensions { .. })
    ));
}

#[test]
fn test_cwe_20_duplicate_ids() {
    // Duplicate sprite ID in lowering
    let dup_sprites = vec![
        SpriteQuad::new(42, 0.0, 0.0, 8.0, 8.0),
        SpriteQuad::new(42, 10.0, 10.0, 8.0, 8.0),
    ];
    assert_eq!(
        lower_sprites_to_paint_scene(100.0, 100.0, "#fff", &dup_sprites),
        Err(PaintBatchError::DuplicateSpriteId(42))
    );

    // Duplicate sprite ID in diffing
    let mut ops = [SpriteDeltaOp {
        kind: DeltaKind::Unchanged,
        id: 0,
        old_index: 0,
        new_index: 0,
    }; 4];
    assert_eq!(
        diff_sprite_batches_into(&dup_sprites, &[], &mut ops),
        Err(PaintBatchError::DuplicateSpriteId(42))
    );

    // Duplicate tile coordinates
    let dup_cells = vec![TileBatchCell::new(3, 4, 1), TileBatchCell::new(3, 4, 2)];
    assert_eq!(
        lower_tile_batch_to_paint_scene(100.0, 100.0, 8.0, 8.0, "#fff", &dup_cells),
        Err(PaintBatchError::DuplicateTileId { col: 3, row: 4 })
    );

    // Duplicate instruction IDs in scene diff
    let mut scene_a = PaintScene::new(100.0, 100.0);
    scene_a.instructions = vec![
        PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some("dup_id".to_string()),
                metadata: None,
            },
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            fill: None,
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
        PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some("dup_id".to_string()),
                metadata: None,
            },
            x: 10.0,
            y: 10.0,
            width: 10.0,
            height: 10.0,
            fill: None,
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }),
    ];
    let scene_b = PaintScene::new(100.0, 100.0);

    assert_eq!(
        diff_paint_scenes(&scene_a, &scene_b),
        Err(PaintBatchError::DuplicateInstructionId(
            "dup_id".to_string()
        ))
    );

    // Missing instruction ID
    let mut scene_missing = PaintScene::new(100.0, 100.0);
    scene_missing.instructions = vec![PaintInstruction::Rect(PaintRect {
        base: PaintBase {
            id: None,
            metadata: None,
        },
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
        fill: None,
        stroke: None,
        stroke_width: None,
        corner_radius: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    })];

    assert_eq!(
        diff_paint_scenes(&scene_missing, &scene_b),
        Err(PaintBatchError::MissingInstructionId)
    );
}

#[test]
fn test_buffer_too_small() {
    let prev = vec![SpriteQuad::new(1, 0.0, 0.0, 8.0, 8.0)];
    let next = vec![SpriteQuad::new(2, 0.0, 0.0, 8.0, 8.0)];

    let mut tiny_ops = [SpriteDeltaOp {
        kind: DeltaKind::Unchanged,
        id: 0,
        old_index: 0,
        new_index: 0,
    }; 1]; // needs 2 ops (one removed, one added)

    let res = diff_sprite_batches_into(&prev, &next, &mut tiny_ops);
    assert!(matches!(res, Err(PaintBatchError::BufferTooSmall { .. })));
}
