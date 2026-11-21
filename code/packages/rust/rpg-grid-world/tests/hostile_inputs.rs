//! Hostile-input and deterministic fuzz test suite for `rpg-grid-world`.

#![forbid(unsafe_code)]

use rpg_grid_world::{
    interact_target, lower_dialogue_cells_into, lower_dialogue_to_paint_scene, npc_blocks_tile,
    npc_sees_actor, resolve_warp, tile_in_bounds, validate_encounter_table, validate_warp,
    DialogueBox, DialogueInput, DialoguePage, DialogueStyle, EncounterEntry, EncounterState,
    EncounterTable, Facing, FrameTiles, GridActor, GridError, GridRng, GridWorld, Npc, NpcKind,
    TileBatchCell, TileCell, TileLayer, WarpPad, WarpTrigger, MAX_ENCOUNTER_ENTRIES,
    MAX_ENCOUNTER_TABLES, MAX_INTERACT_RANGE, MAX_LINE_LEN, MAX_NPCS, MAX_PAGES, MAX_SPEED,
    MAX_TARGET_COORD, MAX_TEXT_SPEED_Q8, MAX_TILE_PX, MAX_TURN_TICKS, MAX_WARPS, RATE_ONE,
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

    fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() >> 48) as u16
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

fn default_style() -> DialogueStyle {
    DialogueStyle {
        col: 1,
        row: 1,
        inner_cols: 10,
        inner_rows: 4,
        glyph_base: 10,
        cursor_tile: 9,
        frame: FrameTiles {
            tl: 1,
            t: 2,
            tr: 3,
            l: 4,
            fill: 5,
            r: 6,
            bl: 7,
            b: 8,
            br: 9,
        },
        tint: [255, 255, 255],
    }
}

#[test]
fn hostile_warps_npcs_encounters_and_dialogue() {
    let cells = [TileCell::tile(1); 16];
    let layer = TileLayer::attach(&cells, 4, 4, 16, 16).unwrap();

    // Extreme i32::MIN / i32::MAX warp and NPC coordinates, i64::MIN / i64::MAX blocking.
    for &(wx, wy, tx, ty) in &[
        (i32::MIN, i32::MIN, 0, 0),
        (i32::MAX, i32::MAX, 0, 0),
        (0, 0, -1, 0),
        (0, 0, MAX_TARGET_COORD, 0),
        (0, 0, 0, i32::MAX),
    ] {
        let warp = WarpPad {
            x: wx,
            y: wy,
            trigger: WarpTrigger::OnEnter,
            target_map: 1,
            target_x: tx,
            target_y: ty,
            target_facing: Facing::Down,
        };
        assert!(matches!(
            validate_warp(&layer, &warp),
            Err(GridError::OutOfBounds { .. })
        ));
    }

    let npc = Npc {
        id: 1,
        x: i32::MIN,
        y: i32::MAX,
        facing: Facing::Down,
        kind: NpcKind::Person,
        script: 0,
        sight_range: u8::MAX,
        blocking: true,
    };
    assert!(!npc_blocks_tile(&[npc], i64::MAX, i64::MIN));
    assert!(npc_blocks_tile(
        &[npc],
        i64::from(i32::MIN),
        i64::from(i32::MAX)
    ));
    assert!(!npc_sees_actor(&layer, &npc, i32::MIN, i32::MAX));
    assert!(!tile_in_bounds(&layer, i64::MIN, i64::MAX));

    assert!(matches!(
        interact_target(&layer, &[npc], 1, 1, Facing::Up, 0),
        Err(GridError::InvalidParameter { .. })
    ));
    assert!(matches!(
        interact_target(&layer, &[npc], 1, 1, Facing::Up, MAX_INTERACT_RANGE + 1),
        Err(GridError::InvalidParameter { .. })
    ));

    // GridWorld table cap rejections.
    let valid_warp = WarpPad {
        x: 0,
        y: 0,
        trigger: WarpTrigger::OnEnter,
        target_map: 1,
        target_x: 0,
        target_y: 0,
        target_facing: Facing::Down,
    };
    let too_many_warps = [valid_warp; MAX_WARPS + 1];
    assert!(matches!(
        GridWorld::bind(&layer, &too_many_warps, &[], &[]),
        Err(GridError::TooMany { max: MAX_WARPS, .. })
    ));
    let valid_npc = Npc {
        x: 1,
        y: 1,
        sight_range: 1,
        ..npc
    };
    let too_many_npcs = [valid_npc; MAX_NPCS + 1];
    assert!(matches!(
        GridWorld::bind(&layer, &[], &too_many_npcs, &[]),
        Err(GridError::TooMany { max: MAX_NPCS, .. })
    ));

    // Malformed EncounterTable variants.
    let good_entry = EncounterEntry {
        species: 1,
        min_level: 2,
        max_level: 5,
        weight: 10,
    };
    let good_entries = [good_entry];
    let bad_tables = [
        EncounterTable {
            tile_id: 0,
            min_steps: 1,
            max_steps: 2,
            rate_q8: 128,
            entries: &good_entries,
        },
        EncounterTable {
            tile_id: 1,
            min_steps: 1,
            max_steps: 2,
            rate_q8: 128,
            entries: &[],
        },
        EncounterTable {
            tile_id: 1,
            min_steps: 5,
            max_steps: 2,
            rate_q8: 128,
            entries: &good_entries,
        },
        EncounterTable {
            tile_id: 1,
            min_steps: 1,
            max_steps: 2,
            rate_q8: RATE_ONE + 1,
            entries: &good_entries,
        },
    ];
    for t in &bad_tables {
        assert_eq!(
            validate_encounter_table(0, t),
            Err(GridError::InvalidEncounterTable { index: 0 })
        );
    }
    let bad_entries = [good_entry; MAX_ENCOUNTER_ENTRIES + 1];
    assert!(matches!(
        validate_encounter_table(
            0,
            &EncounterTable {
                tile_id: 1,
                min_steps: 1,
                max_steps: 2,
                rate_q8: 128,
                entries: &bad_entries,
            }
        ),
        Err(GridError::TooMany {
            max: MAX_ENCOUNTER_ENTRIES,
            ..
        })
    ));
    let good_table = EncounterTable {
        tile_id: 1,
        min_steps: 1,
        max_steps: 2,
        rate_q8: 128,
        entries: &good_entries,
    };
    let too_many_tables = [good_table; MAX_ENCOUNTER_TABLES + 1];
    assert!(matches!(
        GridWorld::bind(&layer, &[], &[], &too_many_tables),
        Err(GridError::TooMany {
            max: MAX_ENCOUNTER_TABLES,
            ..
        })
    ));

    // GridActor bounds.
    let mut actor = GridActor::place(&layer, 1, 1, Facing::Down).unwrap();
    assert!(actor.set_speed(0).is_err());
    assert!(actor.set_speed(MAX_SPEED + 1).is_err());
    assert!(actor.set_turn_delay(MAX_TURN_TICKS + 1).is_err());

    // Malformed DialoguePage variants.
    assert!(matches!(
        DialogueBox::open(&[]),
        Err(GridError::InvalidDialogue { .. })
    ));
    let long_line = "A".repeat(MAX_LINE_LEN + 1);
    let too_long_page = [DialoguePage {
        lines: &[&long_line],
        choices: &[],
        choice_targets: &[],
    }];
    assert!(matches!(
        DialogueBox::open(&too_long_page),
        Err(GridError::InvalidDialogue { .. })
    ));
    let non_ascii_page = [DialoguePage {
        lines: &["bad\nline"],
        choices: &[],
        choice_targets: &[],
    }];
    assert!(matches!(
        DialogueBox::open(&non_ascii_page),
        Err(GridError::InvalidDialogue { .. })
    ));
    let bad_target_page = [DialoguePage {
        lines: &["Pick:"],
        choices: &["Yes"],
        choice_targets: &[99],
    }];
    assert!(matches!(
        DialogueBox::open(&bad_target_page),
        Err(GridError::InvalidDialogue { .. })
    ));
    let ok_page = DialoguePage {
        lines: &["Hello!"],
        choices: &[],
        choice_targets: &[],
    };
    let too_many_pages = [ok_page; MAX_PAGES + 1];
    assert!(matches!(
        DialogueBox::open(&too_many_pages),
        Err(GridError::TooMany { max: MAX_PAGES, .. })
    ));

    let ok_pages = [ok_page];
    let mut dlg = DialogueBox::open(&ok_pages).unwrap();
    assert!(dlg.set_text_speed(0).is_err());
    assert!(dlg.set_text_speed(MAX_TEXT_SPEED_Q8 + 1).is_err());
    let style = default_style();
    let mut tiny_cells = [TileBatchCell::new(0, 0, 0); 4];
    assert!(matches!(
        lower_dialogue_cells_into(&dlg, &style, &mut tiny_cells),
        Err(GridError::BufferTooSmall { .. })
    ));
    assert!(matches!(
        lower_dialogue_to_paint_scene(&dlg, &style, 0, 8, 240, 160, "#000000"),
        Err(GridError::InvalidParameter { .. })
    ));
    assert!(matches!(
        lower_dialogue_to_paint_scene(&dlg, &style, MAX_TILE_PX + 1, 8, 240, 160, "#000000"),
        Err(GridError::InvalidParameter { .. })
    ));
}

#[test]
fn hostile_fuzz_1000_cases() {
    let mut rng = SplitMix64(0x2026_0023_FACE_B00C);
    let facings = [Facing::Up, Facing::Down, Facing::Left, Facing::Right];
    let dlg_inputs = [
        DialogueInput::Idle,
        DialogueInput::Confirm,
        DialogueInput::Cancel,
        DialogueInput::Up,
        DialogueInput::Down,
    ];

    let cells = [TileCell::tile(1); 64];
    let layer = TileLayer::attach(&cells, 8, 8, 16, 16).unwrap();
    let enc_entries = [EncounterEntry {
        species: 3,
        min_level: 1,
        max_level: 5,
        weight: 10,
    }];
    let enc_table = EncounterTable {
        tile_id: 1,
        min_steps: 1,
        max_steps: 4,
        rate_q8: 128,
        entries: &enc_entries,
    };
    let enc_tables = [enc_table];
    let world = GridWorld::bind(&layer, &[], &[], &enc_tables).unwrap();
    let mut actor = GridActor::place(&layer, 3, 3, Facing::Down).unwrap();
    let mut enc_state = EncounterState::start(0);

    let pages = [
        DialoguePage {
            lines: &["Hi there!", "Choose:"],
            choices: &["One", "Two"],
            choice_targets: &[1, 0],
        },
        DialoguePage {
            lines: &["Done."],
            choices: &[],
            choice_targets: &[],
        },
    ];
    let mut dlg = DialogueBox::open(&pages).unwrap();
    let style = default_style();
    let mut grid_rng = GridRng::seeded(0);

    for i in 0..1000u32 {
        if i.is_multiple_of(100) {
            dlg = DialogueBox::open(&pages).unwrap();
        }
        let _ = grid_rng.roll_below(rng.next_u32());
        let dir = if rng.next_u8().is_multiple_of(5) {
            None
        } else {
            Some(facings[(rng.next_u8() as usize) % facings.len()])
        };
        let _ = actor.advance(&world, dir, &mut enc_state);
        let _ = actor.pixel_position(&layer);
        let _ = actor.hop_arc(rng.next_u32());

        let warp = WarpPad {
            x: rng.next_i32(),
            y: rng.next_i32(),
            trigger: WarpTrigger::OnEnter,
            target_map: rng.next_u16(),
            target_x: rng.next_i32(),
            target_y: rng.next_i32(),
            target_facing: facings[(rng.next_u8() as usize) % facings.len()],
        };
        let _ = validate_warp(&layer, &warp);
        let _ = resolve_warp(
            &[warp],
            rng.next_i32(),
            rng.next_i32(),
            WarpTrigger::OnEnter,
        );

        let npc = Npc {
            id: rng.next_u16(),
            x: rng.next_i32(),
            y: rng.next_i32(),
            facing: facings[(rng.next_u8() as usize) % facings.len()],
            kind: NpcKind::Trainer,
            script: 0,
            sight_range: rng.next_u8(),
            blocking: rng.next_u8() & 1 != 0,
        };
        let _ = npc_blocks_tile(&[npc], rng.next_i64(), rng.next_i64());
        let _ = npc_sees_actor(&layer, &npc, rng.next_i32(), rng.next_i32());
        let _ = interact_target(
            &layer,
            &[npc],
            rng.next_i32(),
            rng.next_i32(),
            facings[(rng.next_u8() as usize) % facings.len()],
            rng.next_u8(),
        );

        let _ = dlg.tick_dialogue(dlg_inputs[(rng.next_u8() as usize) % dlg_inputs.len()]);
        let _ = dlg.visible_line(rng.next_u8() as usize);
        let _ = lower_dialogue_to_paint_scene(
            &dlg,
            &style,
            (rng.next_u32() % 24) + 1,
            (rng.next_u32() % 24) + 1,
            320,
            240,
            "#000000",
        );
    }
}
