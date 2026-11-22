//! # Conformance Tests for `game-labyrinth3d` (GAME34)
//!
//! Covers:
//! - Warrant contract validation helpers (`validate_*`)
//! - Circle-vs-grid sliding collision (zero wall clipping, smooth diagonal slide)
//! - Locked doors, key pickups, health/treasure pickups, and exit win transitions
//! - Patrolling and LOS-chasing entities, contact damage, and loss transitions
//! - Dual renderers (`RenderMode::Raycast25D` and `RenderMode::Software3D`) into `PixelContainer`
//! - Bit-exact deterministic replay (`step`, `step_in_place`, `state_digest_fnv1a`, `frame_digest_fnv1a`)

#![forbid(unsafe_code)]

use game_labyrinth3d::{
    frame_digest_fnv1a, render_frame_into, state_digest_fnv1a, step, step_in_place,
    validate_door_count, validate_entity_count, validate_frame_dim, validate_grid_dim,
    validate_key_id, validate_pickup_count, validate_player_radius, validate_player_radius_micro,
    DoorState, EntityBehavior, EntityState, GameClockConfig, GameStatus, LabyrinthAssets,
    LabyrinthError, LabyrinthGrid, LabyrinthInput, LabyrinthRenderScratch, LabyrinthState,
    PickupItem, PickupKind, PixelContainer, PlayerState, RenderMode, MAX_DOORS, MAX_ENTITIES,
    MAX_FRAME_HEIGHT, MAX_FRAME_WIDTH, MAX_GRID_DIM, MAX_KEYS, MAX_PICKUPS, MAX_PLAYER_RADIUS,
    MIN_PLAYER_RADIUS,
};

#[test]
fn warrant_contract_validators() {
    assert_eq!(validate_grid_dim(1, 1), Ok(true));
    assert_eq!(validate_grid_dim(MAX_GRID_DIM, MAX_GRID_DIM), Ok(true));
    assert_eq!(validate_grid_dim(0, 8), Err(LabyrinthError::InvalidGrid));
    assert_eq!(
        validate_grid_dim(MAX_GRID_DIM + 1, 8),
        Err(LabyrinthError::InvalidGrid)
    );

    assert_eq!(validate_frame_dim(1, 1), Ok(true));
    assert_eq!(
        validate_frame_dim(MAX_FRAME_WIDTH, MAX_FRAME_HEIGHT),
        Ok(true)
    );
    assert_eq!(
        validate_frame_dim(0, 64),
        Err(LabyrinthError::InvalidFramebuffer)
    );
    assert_eq!(
        validate_frame_dim(MAX_FRAME_WIDTH + 1, 64),
        Err(LabyrinthError::InvalidFramebuffer)
    );

    assert_eq!(validate_entity_count(MAX_ENTITIES), Ok(true));
    assert_eq!(
        validate_entity_count(MAX_ENTITIES + 1),
        Err(LabyrinthError::InvalidEntity)
    );

    assert_eq!(validate_door_count(MAX_DOORS), Ok(true));
    assert_eq!(
        validate_door_count(MAX_DOORS + 1),
        Err(LabyrinthError::InvalidDoor)
    );

    assert_eq!(validate_pickup_count(MAX_PICKUPS), Ok(true));
    assert_eq!(
        validate_pickup_count(MAX_PICKUPS + 1),
        Err(LabyrinthError::InvalidPickup)
    );

    assert_eq!(validate_key_id(MAX_KEYS - 1), Ok(true));
    assert_eq!(
        validate_key_id(MAX_KEYS),
        Err(LabyrinthError::ValueOutOfBounds)
    );

    assert_eq!(validate_player_radius(MIN_PLAYER_RADIUS), Ok(true));
    assert_eq!(validate_player_radius(MAX_PLAYER_RADIUS), Ok(true));
    assert_eq!(
        validate_player_radius(0.01),
        Err(LabyrinthError::InvalidPlayer)
    );
    assert_eq!(validate_player_radius_micro(250_000), Ok(true));
    assert_eq!(
        validate_player_radius_micro(10_000),
        Err(LabyrinthError::InvalidPlayer)
    );
}

#[test]
fn circle_vs_grid_sliding_collision_never_clips_walls() {
    let rows = ["#####", "#S..#", "#.#.#", "#..E#", "#####"];
    let grid = LabyrinthGrid::lgrid_from_ascii(&rows).unwrap();
    let player = PlayerState::player_new(1.5, 1.5, 0.4, 0.25, 4.0, 2.0, 100).unwrap();
    let mut state = LabyrinthState::state_new(
        grid,
        player,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        42,
        GameClockConfig::standard_60hz(),
    )
    .unwrap();

    // Push diagonally against the top wall (y=1.0 boundary): X should advance while Y stops at >= 1.25.
    let input = LabyrinthInput::input_new(1.0, -1.0, 0.0, false, 16_666_667).unwrap();
    let start_x = state.player.x;
    for _ in 0..30 {
        step_in_place(&mut state, input).unwrap();
        assert!(
            state.player.y >= 1.25 - 1e-9,
            "player clipped into top wall: y={}",
            state.player.y
        );
        assert!(
            state.player.x <= 3.75 + 1e-9,
            "player clipped into east wall: x={}",
            state.player.x
        );
        let cell_x = state.player.x.floor() as usize;
        let cell_y = state.player.y.floor() as usize;
        assert!(!state.state_cell_blocked(cell_x, cell_y));
    }
    assert!(
        state.player.x > start_x + 0.5,
        "player should slide smoothly along wall"
    );
}

#[test]
fn key_pickup_door_unlock_and_win_transition() {
    let rows = ["#######", "#S...E#", "#######"];
    let grid = LabyrinthGrid::lgrid_from_ascii(&rows).unwrap();
    let player = PlayerState::player_new(1.5, 1.5, 0.0, 0.25, 3.0, 2.0, 100).unwrap();
    // Locked door at (3, 1) requires key 2.
    let doors = vec![DoorState::door_new(3, 1, Some(2), 3).unwrap()];
    let pickups = vec![
        PickupItem::pickup_new(1, 2.2, 1.5, PickupKind::Key(2), 4).unwrap(),
        PickupItem::pickup_new(2, 4.2, 1.5, PickupKind::Treasure(500), 5).unwrap(),
    ];
    let mut state = LabyrinthState::state_new(
        grid,
        player,
        doors,
        pickups,
        Vec::new(),
        7,
        GameClockConfig::standard_60hz(),
    )
    .unwrap();

    // Try interacting before picking up key -> door stays closed and blocks player.
    let interact_fwd = LabyrinthInput::input_new(1.0, 0.0, 0.0, true, 16_666_667).unwrap();
    let out0 = step_in_place(&mut state, interact_fwd).unwrap();
    assert_eq!(out0.doors_opened, 0);
    assert!(!state.doors[0].open);

    // Move forward to collect key 2, open door at (3, 1), and reach exit at (5, 1).
    let mut opened = false;
    for _ in 0..90 {
        let out = step_in_place(&mut state, interact_fwd).unwrap();
        if out.doors_opened > 0 {
            opened = true;
        }
        if state.status == GameStatus::Won {
            break;
        }
    }
    assert!(opened, "key should unlock the door");
    assert!(state.player.player_has_key(2));
    assert!(state.doors[0].open);
    assert_eq!(state.player.score, 500);
    assert_eq!(state.status, GameStatus::Won);

    // Once Won, further steps remain Won and do not regress state.
    let frozen_digest = state_digest_fnv1a(&state);
    let post = step_in_place(&mut state, interact_fwd).unwrap();
    assert_eq!(post.status, GameStatus::Won);
    assert_eq!(post.state_digest, frozen_digest);
}

#[test]
fn entity_chase_contact_damage_and_loss_transition() {
    let rows = ["#######", "#S...E#", "#######"];
    let grid = LabyrinthGrid::lgrid_from_ascii(&rows).unwrap();
    let player = PlayerState::player_new(1.5, 1.5, 0.0, 0.25, 0.0, 0.0, 30).unwrap();
    let entities = vec![EntityState::entity_new(
        1,
        2.5,
        1.5,
        trig::PI,
        0.25,
        3.0,
        EntityBehavior::Chase,
        20,
        15,
        6,
    )
    .unwrap()];
    let mut state = LabyrinthState::state_new(
        grid,
        player,
        Vec::new(),
        Vec::new(),
        entities,
        99,
        GameClockConfig::standard_60hz(),
    )
    .unwrap();

    let idle = LabyrinthInput::input_idle(16_666_667);
    let mut total_dmg = 0u32;
    for _ in 0..120 {
        let out = step_in_place(&mut state, idle).unwrap();
        total_dmg += out.damage_taken;
        if state.status == GameStatus::Lost {
            break;
        }
    }
    assert_eq!(state.status, GameStatus::Lost);
    assert_eq!(state.player.hp, 0);
    assert_eq!(total_dmg, 30);
}

#[test]
fn dual_renderers_produce_deterministic_distinct_frames() {
    let mut state = LabyrinthState::state_build_demo(12345).unwrap();
    let assets = LabyrinthAssets::assets_default().unwrap();
    let mut scratch = LabyrinthRenderScratch::rscratch_for_state(&state).unwrap();
    let mut fb25 = PixelContainer::new(64, 48);
    let mut fb3d = PixelContainer::new(64, 48);
    let mut zb = vec![0.0_f64; 64 * 48];

    let rep25_a = render_frame_into(
        &state,
        &assets,
        RenderMode::Raycast25D,
        &mut scratch,
        &mut zb,
        &mut fb25,
    )
    .unwrap();
    let rep3d_a = render_frame_into(
        &state,
        &assets,
        RenderMode::Software3D,
        &mut scratch,
        &mut zb,
        &mut fb3d,
    )
    .unwrap();

    assert!(rep25_a.wall_hits > 0);
    assert!(rep3d_a.triangles_rasterized > 0);
    assert!(rep3d_a.pixels_written > 0);

    // Identical state + assets must yield bit-exact FNV-1a frame digests.
    let mut fb25_repeat = PixelContainer::new(64, 48);
    let mut fb3d_repeat = PixelContainer::new(64, 48);
    let rep25_b = render_frame_into(
        &state,
        &assets,
        RenderMode::Raycast25D,
        &mut scratch,
        &mut zb,
        &mut fb25_repeat,
    )
    .unwrap();
    let rep3d_b = render_frame_into(
        &state,
        &assets,
        RenderMode::Software3D,
        &mut scratch,
        &mut zb,
        &mut fb3d_repeat,
    )
    .unwrap();

    assert_eq!(rep25_a.frame_digest, rep25_b.frame_digest);
    assert_eq!(rep3d_a.frame_digest, rep3d_b.frame_digest);
    assert_eq!(fb25.data, fb25_repeat.data);
    assert_eq!(fb3d.data, fb3d_repeat.data);

    // Rotating the player must change both rendered frames.
    let turn = LabyrinthInput::input_new(0.0, 0.0, 1.0, false, 100_000_000).unwrap();
    step_in_place(&mut state, turn).unwrap();

    let rep25_turned = render_frame_into(
        &state,
        &assets,
        RenderMode::Raycast25D,
        &mut scratch,
        &mut zb,
        &mut fb25,
    )
    .unwrap();
    let rep3d_turned = render_frame_into(
        &state,
        &assets,
        RenderMode::Software3D,
        &mut scratch,
        &mut zb,
        &mut fb3d,
    )
    .unwrap();

    assert_ne!(rep25_a.frame_digest, rep25_turned.frame_digest);
    assert_ne!(rep3d_a.frame_digest, rep3d_turned.frame_digest);
}

#[test]
fn step_matches_step_in_place_and_canonical_neg_zero_digest() {
    let mut s1 = LabyrinthState::state_build_demo(777).unwrap();
    let s2 = s1.clone();
    let input = LabyrinthInput::input_new(0.5, 0.25, -0.3, false, 33_333_334).unwrap();

    let out1 = step_in_place(&mut s1, input).unwrap();
    let (s2_next, out2) = step(&s2, input).unwrap();
    assert_eq!(out1, out2);
    assert_eq!(s1, s2_next);

    // -0.0 and +0.0 must hash identically in state_digest_fnv1a.
    let mut s_pos = s1.clone();
    s_pos.player.yaw = 0.0;
    let mut s_neg = s1;
    s_neg.player.yaw = -0.0;
    assert_eq!(state_digest_fnv1a(&s_pos), state_digest_fnv1a(&s_neg));

    // Deterministic 500-tick replay produces identical final digests.
    let mut a = LabyrinthState::state_build_demo(9001).unwrap();
    let mut b = LabyrinthState::state_build_demo(9001).unwrap();
    for tick in 0..500u64 {
        let inp = LabyrinthInput::input_new(
            if tick % 3 == 0 { 1.0 } else { 0.2 },
            if tick % 5 == 0 { -0.4 } else { 0.0 },
            if tick % 7 == 0 { 0.8 } else { -0.2 },
            tick % 10 == 0,
            16_666_667,
        )
        .unwrap();
        step_in_place(&mut a, inp).unwrap();
        step_in_place(&mut b, inp).unwrap();
    }
    assert_eq!(state_digest_fnv1a(&a), state_digest_fnv1a(&b));
    let fb = PixelContainer::new(8, 8);
    assert_ne!(frame_digest_fnv1a(&fb), 0);
}
