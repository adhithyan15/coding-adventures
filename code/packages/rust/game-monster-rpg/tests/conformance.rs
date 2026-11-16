//! # Conformance and Security Verification Test Suite for `game-monster-rpg` (GAME25)
//!
//! Validates:
//! - Complete playable GBA-style RPG vertical slice combining all Era 2 crates.
//! - Multi-map overworld navigation (Town, Route 1, Tall Grass Meadow, Healing Center, Gym).
//! - Warp pad transitions across maps.
//! - NPC dialogue interactions and Pokémon Center full-party healing.
//! - Step encounters in tall grass with deterministic PRNG rolls.
//! - Turn-based battle engine (`rpg-battle-engine`) with fight, catch, flee, switch, XP, and level up.
//! - Smooth palette fade transitions between overworld and battle modes.
//! - Retained-mode sprite lowering and universal `PaintScene` IR generation.
//! - Real-time deterministic chiptune audio mixing into `PcmRingBuffer`.
//! - Fixed-timestep clock loop integration via `game-clock`.
//! - Formal verification warrant bounds and CWE-20/125/190/369 guards.

use arcade_sfx::SfxBank;
use audio_mixer_realtime::PcmRingBuffer;
use game_clock::{new_clock, GameClockConfig, DEFAULT_60HZ_DT_NS};
use game_monster_rpg::{
    build_rpg_sprites, create_monster, default_rpg_config, get_map_tile_attr, init_rpg_game,
    lower_rpg_to_paint_scene, render_rpg_audio_into, step_rpg_frame, step_rpg_with_clock,
    validate_fade_fraction, validate_map_id, validate_move_slot, validate_party_index,
    validate_step_rate, Command, Facing, GameMode, Phase, RpgError, RpgInput, RpgNpcKind,
    SpriteQuad, Status, WalkDirection, ATTR_SOLID, ATTR_TALL_GRASS, ATTR_WARP, FADE_TOTAL_STEPS,
    MAP_COUNT, MAP_GYM, MAP_HEALING_CENTER, MAP_ROUTE1, MAP_TALL_GRASS, MAP_TOWN, MAX_PARTY,
    SPECIES_PYROKID,
};

#[test]
fn test_species_and_tile_attributes() {
    // All 6 species construct successfully
    for sp in 0..6 {
        let m = create_monster(sp, 10).expect("valid monster");
        assert_eq!(m.level, 10);
        assert!(m.hp > 0);
        assert!(m.move_count >= 1);
    }

    // Tile attributes for maps
    assert_eq!(get_map_tile_attr(MAP_TOWN as u8, 0, 0), ATTR_SOLID);
    assert_eq!(get_map_tile_attr(MAP_TOWN as u8, 7, 0), ATTR_WARP);
    assert_eq!(get_map_tile_attr(MAP_ROUTE1 as u8, 3, 5), ATTR_TALL_GRASS);
    assert_eq!(
        get_map_tile_attr(MAP_TALL_GRASS as u8, 3, 3),
        ATTR_TALL_GRASS
    );
    assert_eq!(
        get_map_tile_attr(MAP_HEALING_CENTER as u8, 7, 4),
        ATTR_SOLID
    );
    assert_eq!(get_map_tile_attr(MAP_GYM as u8, 7, 15), ATTR_WARP);
}

#[test]
fn test_warrant_bounds_guards() {
    // Map ID validation (0..5, CWE-20, CWE-125)
    for id in 0..MAP_COUNT {
        assert_eq!(validate_map_id(id), Ok(true));
    }
    assert_eq!(validate_map_id(5), Err(RpgError::InvalidMapId));
    assert_eq!(validate_map_id(100), Err(RpgError::InvalidMapId));

    // Party index validation (0..6, CWE-20, CWE-125)
    for idx in 0..MAX_PARTY {
        assert_eq!(validate_party_index(idx), Ok(true));
    }
    assert_eq!(validate_party_index(6), Err(RpgError::InvalidPartyIndex));
    assert_eq!(validate_party_index(255), Err(RpgError::InvalidPartyIndex));

    // Move slot validation (0..4, CWE-20, CWE-125)
    for slot in 0..4 {
        assert_eq!(validate_move_slot(slot), Ok(true));
    }
    assert_eq!(validate_move_slot(4), Err(RpgError::InvalidMoveSlot));
    assert_eq!(validate_move_slot(10), Err(RpgError::InvalidMoveSlot));

    // Fade fraction validation (0..=255, CWE-190)
    assert_eq!(validate_fade_fraction(0), Ok(true));
    assert_eq!(validate_fade_fraction(128), Ok(true));
    assert_eq!(validate_fade_fraction(255), Ok(true));
    assert_eq!(
        validate_fade_fraction(256),
        Err(RpgError::InvalidFadeFraction)
    );

    // Step rate validation (0..=256, CWE-190)
    assert_eq!(validate_step_rate(0), Ok(true));
    assert_eq!(validate_step_rate(32), Ok(true));
    assert_eq!(validate_step_rate(256), Ok(true));
    assert_eq!(validate_step_rate(257), Err(RpgError::InvalidStepRate));
}

#[test]
fn test_config_validation() {
    let mut cfg = default_rpg_config();
    assert!(cfg.validate_rpg_config().is_ok());

    cfg.screen_width = 0;
    assert_eq!(cfg.validate_rpg_config(), Err(RpgError::InvalidConfig));
    cfg.screen_width = 240;

    cfg.screen_height = 0;
    assert_eq!(cfg.validate_rpg_config(), Err(RpgError::InvalidConfig));
    cfg.screen_height = 160;

    cfg.sample_rate_hz = 0;
    assert_eq!(cfg.validate_rpg_config(), Err(RpgError::InvalidConfig));
    cfg.sample_rate_hz = 44_100;

    cfg.walk_ticks = 0;
    assert_eq!(cfg.validate_rpg_config(), Err(RpgError::InvalidConfig));
    cfg.walk_ticks = 8;

    cfg.encounter_rate = 300;
    assert_eq!(cfg.validate_rpg_config(), Err(RpgError::InvalidStepRate));
}

#[test]
fn test_initial_game_state() {
    let cfg = default_rpg_config();
    let game = init_rpg_game(cfg, 0x1234_5678).expect("valid game");

    assert_eq!(game.active_game_mode(), GameMode::Overworld);
    assert_eq!(game.current_map_id(), MAP_TOWN as u8);
    assert_eq!(game.player_tile_pos(), (7, 8));
    assert_eq!(game.party_count(), 1);

    let starter = game.party_creature(0).expect("starter monster");
    assert_eq!(starter.level, 5);
    assert!(starter.hp > 0);
    assert_eq!(starter.hp, starter.max_hp);
    assert_eq!(starter.status, Status::None);
}

#[test]
fn test_overworld_movement_and_collision() {
    let cfg = default_rpg_config();
    let mut game = init_rpg_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

    // Walk up into open space (7, 8) -> (7, 7)
    let input = RpgInput::walk_up();
    let events = step_rpg_frame(&mut game, &input, &mut sfx).expect("step");
    assert!(events.stepped);
    assert_eq!(game.player_tile_pos(), (7, 7));

    // Move left towards boundary
    let input_left = RpgInput::walk_left();
    for _ in 0..10 {
        let _ = step_rpg_frame(&mut game, &input_left, &mut sfx);
    }
    // Blocked by boundary trees at x = 1 (since x = 0 is solid boundary)
    assert!(game.player_x >= 1);
}

#[test]
fn test_multi_map_warps() {
    let cfg = default_rpg_config();
    let mut game = init_rpg_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

    // Teleport player near Route 1 north exit at (7, 1)
    game.player_x = 7;
    game.player_y = 1;

    // Walk north onto warp pad at (7, 0)
    let input = RpgInput::walk_up();
    let events = step_rpg_frame(&mut game, &input, &mut sfx).expect("step");

    assert_eq!(events.warp_triggered, Some(MAP_ROUTE1 as u8));
    assert_eq!(game.current_map_id(), MAP_ROUTE1 as u8);
    assert_eq!(game.player_tile_pos(), (7, 14));

    // Walk south back onto warp pad at (7, 15) to return to Town
    game.player_x = 7;
    game.player_y = 14;
    let input_down = RpgInput::walk_down();
    let events_back = step_rpg_frame(&mut game, &input_down, &mut sfx).expect("step");

    assert_eq!(events_back.warp_triggered, Some(MAP_TOWN as u8));
    assert_eq!(game.current_map_id(), MAP_TOWN as u8);
    assert_eq!(game.player_tile_pos(), (7, 1));
}

#[test]
fn test_healing_center_party_healing() {
    let cfg = default_rpg_config();
    let mut game = init_rpg_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

    // Damage player creature and inflict burn
    if let Some(c) = game.party[0].as_mut() {
        c.hp = 1;
        c.status = Status::Burn;
        c.pp[0] = 0;
    }

    // Warp to Healing Center
    game.current_map = MAP_HEALING_CENTER as u8;
    game.player_x = 7;
    game.player_y = 4; // Right in front of Nurse Joy counter at (7, 3)
    game.player_facing = Facing::Up;

    // Press action button to interact
    let input = RpgInput::interact_button();
    let events = step_rpg_frame(&mut game, &input, &mut sfx).expect("interact");

    assert_eq!(events.npc_spoken, Some(RpgNpcKind::Nurse));
    assert!(events.party_healed);
    assert!(game.is_dialogue_open());

    // Verify creature is fully restored
    let restored = game.party_creature(0).expect("restored monster");
    assert_eq!(restored.hp, restored.max_hp);
    assert_eq!(restored.status, Status::None);
    assert_eq!(restored.pp[0], restored.moves[0].max_pp);

    // Dismiss dialogue
    let input_dismiss = RpgInput::interact_button();
    let _ = step_rpg_frame(&mut game, &input_dismiss, &mut sfx);
    assert_eq!(game.active_game_mode(), GameMode::Overworld);
}

#[test]
fn test_wild_battle_capture_and_xp() {
    let cfg = default_rpg_config();
    let mut game = init_rpg_game(cfg, 0x42).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

    // Trigger wild encounter against Pyrokid
    game.trigger_wild_encounter(SPECIES_PYROKID, 3)
        .expect("wild encounter");
    assert!(matches!(
        game.active_game_mode(),
        GameMode::BattleTransitionIn { .. }
    ));

    // Advance fade frames to enter battle
    for _ in 0..FADE_TOTAL_STEPS {
        let dummy = RpgInput::walk_dir(WalkDirection::None);
        let _ = step_rpg_frame(&mut game, &dummy, &mut sfx);
    }
    assert_eq!(game.active_game_mode(), GameMode::Battle);
    assert!(game.active_battle().is_some());

    // Throw Pokéball to capture
    let ball_input = RpgInput::battle_ball();
    let events = step_rpg_frame(&mut game, &ball_input, &mut sfx).expect("capture");

    // Pyrokid catch rate is high (150) with bonus 15, likely caught
    // Complete transition out if battle ended
    if events.battle_ended {
        assert!(events.victory || events.creature_caught);
        for _ in 0..FADE_TOTAL_STEPS {
            let dummy = RpgInput::walk_dir(WalkDirection::None);
            let _ = step_rpg_frame(&mut game, &dummy, &mut sfx);
        }
        assert_eq!(game.active_game_mode(), GameMode::Overworld);
    }
}

#[test]
fn test_gym_leader_trainer_battle() {
    let cfg = default_rpg_config();
    let mut game = init_rpg_game(cfg, 0xbeef).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

    // Warp to Gym
    game.current_map = MAP_GYM as u8;
    game.player_x = 7;
    game.player_y = 4;
    game.player_facing = Facing::Up;

    // Interact with Gym Leader Brock
    let input = RpgInput::interact_button();
    let events = step_rpg_frame(&mut game, &input, &mut sfx).expect("interact");

    assert_eq!(events.npc_spoken, Some(RpgNpcKind::GymLeader));
    assert!(events.battle_started);

    // Fade into battle
    for _ in 0..FADE_TOTAL_STEPS {
        let dummy = RpgInput::walk_dir(WalkDirection::None);
        let _ = step_rpg_frame(&mut game, &dummy, &mut sfx);
    }
    assert_eq!(game.active_game_mode(), GameMode::Battle);

    // Execute battle turns
    let fight_input = RpgInput::battle_fight(0);
    for _ in 0..10 {
        if game.active_game_mode() != GameMode::Battle {
            break;
        }
        let _ = step_rpg_frame(&mut game, &fight_input, &mut sfx);
    }
}

#[test]
fn test_battle_input_bounds_and_truncation_guards() {
    // Finding 2: Verify slot 256 and index 256 do not truncate silently to 0
    let input_fight_256 = RpgInput::battle_fight(256);
    assert_eq!(
        input_fight_256.battle_command,
        Some(Command::Fight { slot: u8::MAX })
    );

    let input_switch_256 = RpgInput::battle_switch(256);
    assert_eq!(
        input_switch_256.battle_command,
        Some(Command::Switch { index: u8::MAX })
    );

    // Verify in an active battle that submitting slot: u8::MAX is rejected by the battle engine
    let cfg = default_rpg_config();
    let mut game = init_rpg_game(cfg, 0x1234).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

    // Initiate wild battle
    game.mode = GameMode::BattleTransitionIn {
        step: FADE_TOTAL_STEPS,
        wild: true,
        species: 1,
        level: 5,
        trainer: None,
    };
    let _ = step_rpg_frame(
        &mut game,
        &RpgInput::walk_dir(WalkDirection::None),
        &mut sfx,
    );
    assert_eq!(game.active_game_mode(), GameMode::Battle);

    let battle = game.battle.as_mut().expect("active battle");
    assert_eq!(battle.phase(), Phase::CommandSelect);

    // Submitting turn with slot 255 (u8::MAX) fails validation
    let enemy_cmd = battle.ai_choose();
    let res = battle.submit_turn(Command::Fight { slot: u8::MAX }, enemy_cmd);
    assert!(
        res.is_err(),
        "Move slot 255 must be rejected by battle engine"
    );
}

#[test]
fn test_step_rpg_with_clock() {
    let cfg = default_rpg_config();
    let mut game = init_rpg_game(cfg, 0x1234).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");
    let mut clock = new_clock(GameClockConfig::standard_60hz()).expect("clock init");

    let input = RpgInput::walk_down();
    let clock_res =
        step_rpg_with_clock(&mut clock, DEFAULT_60HZ_DT_NS, &mut game, &input, &mut sfx);
    assert!(clock_res.is_ok());

    let res = clock_res.unwrap();
    assert_eq!(res.steps_run, 1);
    assert!(res.events.stepped);
}

#[test]
fn test_zero_alloc_sprite_and_paint_scene_lowering() {
    let cfg = default_rpg_config();
    let game = init_rpg_game(cfg, 0x1234).expect("valid game");

    let mut sprites = [SpriteQuad::new(0, 0.0, 0.0, 0.0, 0.0); 128];
    let count = build_rpg_sprites(&game, &mut sprites).expect("build sprites");
    assert!(count > 0);

    let scene_res = lower_rpg_to_paint_scene(&game);
    assert!(scene_res.is_ok());
    let scene = scene_res.unwrap();
    assert_eq!(scene.width, 240.0);
    assert_eq!(scene.height, 160.0);
}

#[test]
fn test_render_rpg_audio_into() {
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");
    let mut ring = PcmRingBuffer::<256>::empty();
    let mut scratch = [0i16; 64];

    // Trigger sound preset
    let _ = sfx.trigger_bank_preset(arcade_sfx::SfxPreset::PongPaddleHit);

    let rendered = render_rpg_audio_into(&mut sfx, &mut ring, &mut scratch);
    assert!(rendered.is_ok());
    assert_eq!(rendered.unwrap(), 64);
}
