//! Hostile-input and deterministic fuzz test suite for `game-monster-rpg`.

#![forbid(unsafe_code)]

use arcade_sfx::SfxBank;
use audio_mixer_realtime::PcmRingBuffer;
use game_monster_rpg::{
    build_rpg_sprites, create_monster, default_rpg_config, get_map_tile_attr, init_rpg_game,
    lower_rpg_to_paint_scene, render_rpg_audio_into, step_rpg_frame, validate_fade_fraction,
    validate_map_id, validate_move_slot, validate_party_index, validate_step_rate, Command, Facing,
    GameMode, RpgConfig, RpgError, RpgInput, RpgNpcKind, SpriteQuad, Status, WalkDirection,
    ATTR_SOLID, MAP_COUNT, MAX_MOVES, MAX_PARTY,
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
}

#[test]
fn hostile_heal_party_oob_move_count_and_validators() {
    let mut game = init_rpg_game(default_rpg_config(), 0xCAFE_BABE).expect("init");
    if let Some(c) = game.party[0].as_mut() {
        c.hp = 1;
        c.status = Status::Sleep;
        c.sleep_turns = 3;
        c.pp = [0; MAX_MOVES];
        c.move_count = u8::MAX;
    }
    game.heal_entire_party();
    let healed = game.party_creature(0).expect("starter");
    assert_eq!(healed.hp, healed.max_hp);
    assert_eq!(healed.status, Status::None);
    assert_eq!(healed.sleep_turns, 0);
    for i in 0..MAX_MOVES {
        assert_eq!(healed.pp[i], healed.moves[i].max_pp);
    }

    // Validators reject usize::MAX / u32::MAX / boundary+1.
    assert_eq!(validate_map_id(MAP_COUNT), Err(RpgError::InvalidMapId));
    assert_eq!(validate_map_id(usize::MAX), Err(RpgError::InvalidMapId));
    assert_eq!(
        validate_party_index(MAX_PARTY),
        Err(RpgError::InvalidPartyIndex)
    );
    assert_eq!(
        validate_party_index(usize::MAX),
        Err(RpgError::InvalidPartyIndex)
    );
    assert_eq!(
        validate_move_slot(MAX_MOVES),
        Err(RpgError::InvalidMoveSlot)
    );
    assert_eq!(
        validate_move_slot(usize::MAX),
        Err(RpgError::InvalidMoveSlot)
    );
    assert_eq!(
        validate_fade_fraction(256),
        Err(RpgError::InvalidFadeFraction)
    );
    assert_eq!(
        validate_fade_fraction(u32::MAX),
        Err(RpgError::InvalidFadeFraction)
    );
    assert_eq!(validate_step_rate(257), Err(RpgError::InvalidStepRate));
    assert_eq!(validate_step_rate(u32::MAX), Err(RpgError::InvalidStepRate));
}

#[test]
fn hostile_oob_coords_config_and_slots() {
    let bad_configs = [
        RpgConfig {
            screen_width: 0,
            ..default_rpg_config()
        },
        RpgConfig {
            screen_height: 0,
            ..default_rpg_config()
        },
        RpgConfig {
            sample_rate_hz: 0,
            ..default_rpg_config()
        },
        RpgConfig {
            walk_ticks: 0,
            ..default_rpg_config()
        },
        RpgConfig {
            encounter_rate: 257,
            ..default_rpg_config()
        },
        RpgConfig {
            encounter_rate: u32::MAX,
            ..default_rpg_config()
        },
    ];
    for cfg in bad_configs {
        assert!(init_rpg_game(cfg, 1).is_err());
    }

    // Out-of-range map and tile coordinates return ATTR_SOLID and never panic.
    assert_eq!(get_map_tile_attr(255, 255, 255), ATTR_SOLID);
    assert_eq!(get_map_tile_attr(0, 255, 0), ATTR_SOLID);
    assert_eq!(get_map_tile_attr(0, 0, 255), ATTR_SOLID);

    let mut game = init_rpg_game(default_rpg_config(), 7).expect("init");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");

    assert!(game.party_creature(MAX_PARTY).is_none());
    assert!(game.party_creature(usize::MAX).is_none());

    // Mutate map and player coordinates to 255, step in all directions, and lower scene.
    game.current_map = 255;
    game.player_x = 255;
    game.player_y = 255;
    for dir in [
        WalkDirection::Up,
        WalkDirection::Down,
        WalkDirection::Left,
        WalkDirection::Right,
        WalkDirection::None,
    ] {
        let input = RpgInput {
            walk: dir,
            action: true,
            cancel: true,
            battle_command: Some(Command::Fight { slot: u8::MAX }),
        };
        let _ = step_rpg_frame(&mut game, &input, &mut sfx);
        let _ = lower_rpg_to_paint_scene(&game);
    }

    // OOB move/party slots on input builders.
    let fight_oob = RpgInput::battle_fight(usize::MAX);
    assert_eq!(
        fight_oob.battle_command,
        Some(Command::Fight { slot: u8::MAX })
    );
    let switch_oob = RpgInput::battle_switch(usize::MAX);
    assert_eq!(
        switch_oob.battle_command,
        Some(Command::Switch { index: u8::MAX })
    );

    // Small sprite buffer & empty audio scratch are handled cleanly.
    let mut tiny = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); 7];
    assert_eq!(
        build_rpg_sprites(&game, &mut tiny),
        Err(RpgError::BufferTooSmall)
    );
    let mut ring = PcmRingBuffer::<64>::empty();
    assert_eq!(render_rpg_audio_into(&mut sfx, &mut ring, &mut []), Ok(0));

    // Empty party during BattleTransitionIn returns PartyEmpty cleanly.
    game.party[0] = None;
    game.mode = GameMode::BattleTransitionIn {
        step: u16::MAX,
        wild: true,
        species: 255,
        level: 255,
        trainer: None,
    };
    assert_eq!(
        step_rpg_frame(
            &mut game,
            &RpgInput::walk_dir(WalkDirection::None),
            &mut sfx
        ),
        Err(RpgError::PartyEmpty)
    );

    // Species & level clamping never panics.
    let _ = create_monster(u8::MAX, 0).expect("clamped level");
    let _ = create_monster(u8::MAX, u8::MAX).expect("clamped level");
}

#[test]
fn hostile_fuzz_1000_cases_step_and_lower() {
    let mut rng = SplitMix64(0x2026_0025_DEAD_BEEF);
    let mut game = init_rpg_game(default_rpg_config(), 0x1122_3344).expect("init");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");

    let facings = [Facing::Up, Facing::Down, Facing::Left, Facing::Right];
    let walks = [
        WalkDirection::None,
        WalkDirection::Up,
        WalkDirection::Down,
        WalkDirection::Left,
        WalkDirection::Right,
    ];
    let trainers = [
        None,
        Some(RpgNpcKind::Guide),
        Some(RpgNpcKind::Nurse),
        Some(RpgNpcKind::TrainerYoungster),
        Some(RpgNpcKind::GymLeader),
    ];

    for i in 0..1000u32 {
        if i.is_multiple_of(50) {
            game = init_rpg_game(default_rpg_config(), rng.next_u64()).expect("reinit");
        }

        // Periodically inject hostile field mutations.
        if rng.next_u8().is_multiple_of(3) {
            game.current_map = rng.next_u8();
            game.player_x = rng.next_u8();
            game.player_y = rng.next_u8();
            game.player_facing = facings[(rng.next_u8() as usize) % facings.len()];
            game.config.encounter_rate = rng.next_u32();
            if let Some(c) = game.party[0].as_mut() {
                c.move_count = rng.next_u8();
                c.hp = rng.next_u16();
            }
            if rng.next_u8().is_multiple_of(4) {
                game.heal_entire_party();
            }
            if rng.next_u8().is_multiple_of(5) {
                game.mode = match rng.next_u8() % 5 {
                    0 => GameMode::Overworld,
                    1 => GameMode::Dialogue,
                    2 => GameMode::BattleTransitionIn {
                        step: rng.next_u16(),
                        wild: rng.next_u8() & 1 != 0,
                        species: rng.next_u8(),
                        level: rng.next_u8(),
                        trainer: trainers[(rng.next_u8() as usize) % trainers.len()],
                    },
                    3 => GameMode::Battle,
                    _ => GameMode::BattleTransitionOut {
                        step: rng.next_u16(),
                    },
                };
            }
        }

        let cmd = match rng.next_u8() % 5 {
            0 => None,
            1 => Some(Command::Fight {
                slot: rng.next_u8(),
            }),
            2 => Some(Command::Switch {
                index: rng.next_u8(),
            }),
            3 => Some(Command::Catch {
                bonus_x10: rng.next_u16(),
            }),
            _ => Some(Command::Run),
        };
        let input = RpgInput {
            walk: walks[(rng.next_u8() as usize) % walks.len()],
            action: rng.next_u8() & 1 != 0,
            cancel: rng.next_u8() & 1 != 0,
            battle_command: cmd,
        };

        let _ = step_rpg_frame(&mut game, &input, &mut sfx);
        let _ = lower_rpg_to_paint_scene(&game);
    }
}
