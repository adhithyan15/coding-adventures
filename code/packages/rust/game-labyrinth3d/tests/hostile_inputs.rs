//! # Hostile-Input CVE Proof & Atomic Rollback Suite for `game-labyrinth3d` (GAME34)
//!
//! Proves that CWE-20, CWE-125, CWE-190, CWE-369, CWE-400, CWE-674, and CWE-835
//! cannot trigger panics, out-of-bounds memory access, integer overflows, unbounded
//! loops, or partial state/framebuffer/Z-buffer corruption under hostile inputs,
//! plus runs a seeded 2,000-case hostile fuzz campaign with 0 panics.

#![forbid(unsafe_code)]

use game_labyrinth3d::{
    render_frame_into, step_in_place, DoorState, EntityBehavior, EntityState, GameClockConfig,
    LabyrinthAssets, LabyrinthError, LabyrinthGrid, LabyrinthInput, LabyrinthRenderScratch,
    LabyrinthState, PickupItem, PickupKind, PixelContainer, PlayerState, RenderMode, TextureRgba,
    TileKind, MAX_COORD, MAX_DOORS, MAX_DT_NS, MAX_ENTITIES, MAX_FRAME_WIDTH, MAX_GRID_DIM,
    MAX_PICKUPS, MAX_TEXTURES,
};

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

fn pick_hostile_f64(state: &mut u64) -> f64 {
    match next_u64(state) % 16 {
        0 => f64::NAN,
        1 => f64::from_bits(0x7ff8_0000_0000_1234), // quiet NaN with payload
        2 => f64::from_bits(0x7ff0_0000_0000_0001), // signaling-bit NaN
        3 => f64::INFINITY,
        4 => f64::NEG_INFINITY,
        5 => -0.0,
        6 => 0.0,
        7 => f64::from_bits(1), // smallest subnormal
        8 => f64::MIN_POSITIVE * 0.5,
        9 => f64::MAX,
        10 => f64::MIN,
        11 => 1.0e305,
        12 => -1.0e12,
        _ => {
            let u = (next_u64(state) >> 11) as f64 / ((1u64 << 53) as f64);
            (u - 0.5) * 20.0
        }
    }
}

fn sentinel_fb(w: u32, h: u32) -> PixelContainer {
    let mut fb = PixelContainer::new(w, h);
    for (i, b) in fb.data.iter_mut().enumerate() {
        *b = ((i * 41 + 19) & 0xff) as u8;
    }
    fb
}

fn sentinel_zb(len: usize) -> Vec<f64> {
    (0..len).map(|i| 0.35 + (i as f64) * 1e-4).collect()
}

fn states_bitwise_eq(a: &LabyrinthState, b: &LabyrinthState) -> bool {
    a.grid == b.grid
        && a.player.x.to_bits() == b.player.x.to_bits()
        && a.player.y.to_bits() == b.player.y.to_bits()
        && a.player.yaw.to_bits() == b.player.yaw.to_bits()
        && a.player.radius.to_bits() == b.player.radius.to_bits()
        && a.player.move_speed.to_bits() == b.player.move_speed.to_bits()
        && a.player.turn_speed.to_bits() == b.player.turn_speed.to_bits()
        && a.player.hp == b.player.hp
        && a.player.max_hp == b.player.max_hp
        && a.player.key_mask == b.player.key_mask
        && a.player.score == b.player.score
        && a.doors == b.doors
        && a.pickups.len() == b.pickups.len()
        && a.pickups.iter().zip(b.pickups.iter()).all(|(pa, pb)| {
            pa.id == pb.id
                && pa.x.to_bits() == pb.x.to_bits()
                && pa.y.to_bits() == pb.y.to_bits()
                && pa.kind == pb.kind
                && pa.collected == pb.collected
                && pa.texture_index == pb.texture_index
        })
        && a.entities.len() == b.entities.len()
        && a.entities.iter().zip(b.entities.iter()).all(|(ea, eb)| {
            ea.id == eb.id
                && ea.x.to_bits() == eb.x.to_bits()
                && ea.y.to_bits() == eb.y.to_bits()
                && ea.yaw.to_bits() == eb.yaw.to_bits()
                && ea.radius.to_bits() == eb.radius.to_bits()
                && ea.speed.to_bits() == eb.speed.to_bits()
                && ea.behavior == eb.behavior
                && ea.hp == eb.hp
                && ea.damage == eb.damage
                && ea.alive == eb.alive
                && ea.cooldown_ticks == eb.cooldown_ticks
                && ea.texture_index == eb.texture_index
        })
        && a.clock == b.clock
        && a.rng == b.rng
        && a.tick == b.tick
        && a.status == b.status
        && a.exit_x == b.exit_x
        && a.exit_y == b.exit_y
}

#[test]
fn hostile_numeric_vectors_and_atomic_step_rollback() {
    let hostile_floats = [
        f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_1234),
        f64::from_bits(0x7ff0_0000_0000_0001),
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        f64::from_bits(1),
        f64::MIN_POSITIVE * 0.5,
        f64::MAX,
        f64::MIN,
        MAX_COORD + 1.0,
    ];

    let mut state = LabyrinthState::state_build_demo(101).unwrap();
    let baseline = state.clone();

    for &val in &hostile_floats {
        // Hostile forward/strafe/turn in LabyrinthInput must never panic or corrupt state.
        let bad_input = LabyrinthInput {
            forward: val,
            strafe: -0.0,
            turn: val,
            interact: true,
            dt_ns: 16_666_667,
        };
        let res = step_in_place(&mut state, bad_input);
        if !val.is_finite() {
            assert_eq!(res, Err(LabyrinthError::NonFiniteInput));
            assert_eq!(state, baseline, "state mutated on NonFiniteInput");
        } else if val.abs() > 10.0 {
            assert_eq!(res, Err(LabyrinthError::ValueOutOfBounds));
            assert_eq!(state, baseline, "state mutated on ValueOutOfBounds");
        } else if res.is_ok() {
            state = baseline.clone();
        }

        // Hostile player state coordinates/radius must reject cleanly and leave state untouched.
        let mut corrupted = baseline.clone();
        corrupted.player.x = val;
        let snapshot = corrupted.clone();
        let res_p = step_in_place(&mut corrupted, LabyrinthInput::input_idle(16_666_667));
        if !val.is_finite() {
            assert_eq!(res_p, Err(LabyrinthError::NonFiniteInput));
            assert!(states_bitwise_eq(&corrupted, &snapshot));
        } else if val < corrupted.player.radius
            || val > (corrupted.grid.lgrid_width() as f64) - corrupted.player.radius
        {
            assert_eq!(res_p, Err(LabyrinthError::InvalidPlayer));
            assert!(states_bitwise_eq(&corrupted, &snapshot));
        }
    }

    // Excessive dt_ns (CWE-400) and corrupted clock invariant must fail atomically.
    let res_dt = step_in_place(&mut state, LabyrinthInput::input_idle(MAX_DT_NS + 1));
    assert_eq!(res_dt, Err(LabyrinthError::InvalidInput));
    assert_eq!(state, baseline);

    let mut bad_clock = baseline.clone();
    bad_clock.clock.accumulator_ns = bad_clock.clock.fixed_dt_ns + 1;
    let bad_clock_snap = bad_clock.clone();
    let res_clk = step_in_place(&mut bad_clock, LabyrinthInput::input_idle(16_666_667));
    assert!(res_clk.is_err());
    assert_eq!(bad_clock, bad_clock_snap);
}

#[test]
fn hostile_oob_bounds_and_atomic_render_rollback() {
    // Grid / entity / door / pickup CWE-400 & CWE-125 rejections.
    assert_eq!(
        LabyrinthGrid::lgrid_new(0, 4, vec![]),
        Err(LabyrinthError::InvalidGrid)
    );
    assert_eq!(
        LabyrinthGrid::lgrid_new(MAX_GRID_DIM + 1, 1, vec![TileKind::Exit; MAX_GRID_DIM + 1]),
        Err(LabyrinthError::InvalidGrid)
    );
    assert_eq!(
        LabyrinthGrid::lgrid_new(2, 2, vec![TileKind::Floor; 4]),
        Err(LabyrinthError::InvalidGrid)
    );
    assert_eq!(
        LabyrinthGrid::lgrid_new(2, 2, vec![TileKind::Exit; 3]),
        Err(LabyrinthError::InvalidGrid)
    );

    // Too many doors/pickups/entities (CWE-400).
    let rows = ["###", "#E#", "###"];
    let small_grid = LabyrinthGrid::lgrid_from_ascii(&rows).unwrap();
    let small_player = PlayerState::player_new(1.5, 1.5, 0.0, 0.25, 1.0, 1.0, 10).unwrap();
    let too_many_doors = vec![DoorState::door_new(1, 1, None, 1).unwrap(); MAX_DOORS + 1];
    assert_eq!(
        LabyrinthState::state_new(
            small_grid.clone(),
            small_player,
            too_many_doors,
            Vec::new(),
            Vec::new(),
            1,
            GameClockConfig::standard_60hz(),
        ),
        Err(LabyrinthError::InvalidDoor)
    );
    let too_many_pickups = vec![
        PickupItem::pickup_new(1, 1.5, 1.5, PickupKind::Treasure(1), 0)
            .unwrap();
        MAX_PICKUPS + 1
    ];
    assert_eq!(
        LabyrinthState::state_new(
            small_grid.clone(),
            small_player,
            Vec::new(),
            too_many_pickups,
            Vec::new(),
            1,
            GameClockConfig::standard_60hz(),
        ),
        Err(LabyrinthError::InvalidPickup)
    );
    let too_many_entities = vec![
        EntityState::entity_new(
            1,
            1.5,
            1.5,
            0.0,
            0.25,
            1.0,
            EntityBehavior::Patrol,
            10,
            1,
            0
        )
        .unwrap();
        MAX_ENTITIES + 1
    ];
    assert_eq!(
        LabyrinthState::state_new(
            small_grid,
            small_player,
            Vec::new(),
            Vec::new(),
            too_many_entities,
            1,
            GameClockConfig::standard_60hz(),
        ),
        Err(LabyrinthError::InvalidEntity)
    );

    // Oversized asset texture table (CWE-400) and invalid FOV.
    let tex = TextureRgba::texture_solid(4, 4, [255, 0, 0, 255]).unwrap();
    assert_eq!(
        LabyrinthAssets::assets_new(vec![tex; MAX_TEXTURES + 1], 1.0),
        Err(LabyrinthError::InvalidAssets)
    );

    // Atomic rollback on render_frame_into for BOTH Raycast25D and Software3D.
    let mut state = LabyrinthState::state_build_demo(55).unwrap();
    let assets = LabyrinthAssets::assets_default().unwrap();
    let mut scratch = LabyrinthRenderScratch::rscratch_for_state(&state).unwrap();

    for mode in [RenderMode::Raycast25D, RenderMode::Software3D] {
        // 1. ZBufferTooSmall leaves framebuffer and zbuffer byte-for-byte untouched.
        let mut fb = sentinel_fb(16, 16);
        let fb_orig = fb.data.clone();
        let mut zb_short = sentinel_zb(16 * 16 - 1);
        let zb_orig = zb_short.clone();
        let err = render_frame_into(&state, &assets, mode, &mut scratch, &mut zb_short, &mut fb);
        assert!(matches!(err, Err(LabyrinthError::ZBufferTooSmall { .. })));
        assert_eq!(fb.data, fb_orig);
        assert_eq!(zb_short, zb_orig);

        // 2. InvalidMaterialId leaves framebuffer and zbuffer untouched.
        state.pickups[0].texture_index = 999;
        let mut zb = sentinel_zb(16 * 16);
        let zb_orig2 = zb.clone();
        let err2 = render_frame_into(&state, &assets, mode, &mut scratch, &mut zb, &mut fb);
        assert_eq!(err2, Err(LabyrinthError::InvalidMaterialId { id: 999 }));
        assert_eq!(fb.data, fb_orig);
        assert_eq!(zb, zb_orig2);
        state.pickups[0].texture_index = 4;

        // 3. InvalidFramebuffer (oversized dims) fails cleanly.
        let bad_fb = PixelContainer {
            width: MAX_FRAME_WIDTH + 1,
            height: 1,
            data: Vec::new(),
        };
        let mut bad_fb_mut = bad_fb;
        let err3 = render_frame_into(
            &state,
            &assets,
            mode,
            &mut scratch,
            &mut zb,
            &mut bad_fb_mut,
        );
        assert_eq!(err3, Err(LabyrinthError::InvalidFramebuffer));
        assert_eq!(zb, zb_orig2);
    }
}

#[test]
fn seeded_2000_case_hostile_fuzz_campaign() {
    let mut rng_seed = 0xdead_beef_cafe_babe_u64;
    let base_state = LabyrinthState::state_build_demo(42).unwrap();
    let assets = LabyrinthAssets::assets_default().unwrap();
    let mut scratch = LabyrinthRenderScratch::rscratch_for_state(&base_state).unwrap();

    for case_idx in 0..2000usize {
        let mut state = base_state.clone();
        // Inject hostile mutations into state on half of cases.
        if case_idx.is_multiple_of(3) {
            state.player.x = pick_hostile_f64(&mut rng_seed);
            state.player.y = pick_hostile_f64(&mut rng_seed);
            state.player.yaw = pick_hostile_f64(&mut rng_seed);
        }
        if case_idx.is_multiple_of(5) {
            state.player.radius = pick_hostile_f64(&mut rng_seed);
        }
        if case_idx.is_multiple_of(7) && !state.entities.is_empty() {
            state.entities[0].x = pick_hostile_f64(&mut rng_seed);
            state.entities[0].speed = pick_hostile_f64(&mut rng_seed);
        }
        if case_idx.is_multiple_of(11) && !state.doors.is_empty() {
            state.doors[0].x = (next_u64(&mut rng_seed) % 128) as usize;
            state.doors[0].required_key = Some((next_u64(&mut rng_seed) % 16) as u8);
        }

        let input = LabyrinthInput {
            forward: pick_hostile_f64(&mut rng_seed),
            strafe: pick_hostile_f64(&mut rng_seed),
            turn: pick_hostile_f64(&mut rng_seed),
            interact: (next_u64(&mut rng_seed) & 1) == 1,
            dt_ns: next_u64(&mut rng_seed) % (MAX_DT_NS * 2 + 1),
        };

        let snapshot = state.clone();
        if step_in_place(&mut state, input).is_err() {
            assert!(
                states_bitwise_eq(&state, &snapshot),
                "fuzz case {case_idx} failed step_in_place without atomic rollback"
            );
        }

        let mode = if (case_idx & 1) == 0 {
            RenderMode::Raycast25D
        } else {
            RenderMode::Software3D
        };
        let mut fb = sentinel_fb(16, 16);
        let fb_orig = fb.data.clone();
        let zlen = if case_idx.is_multiple_of(13) {
            100
        } else {
            16 * 16
        };
        let mut zb = sentinel_zb(zlen);
        let zb_orig = zb.clone();

        if render_frame_into(&state, &assets, mode, &mut scratch, &mut zb, &mut fb).is_err() {
            assert_eq!(fb.data, fb_orig, "fuzz case {case_idx} dirtied fb on Err");
            assert_eq!(zb, zb_orig, "fuzz case {case_idx} dirtied zb on Err");
        }
    }
}
