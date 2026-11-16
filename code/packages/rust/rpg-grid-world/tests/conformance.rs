//! # Conformance Tests for `rpg-grid-world` (GAME23)
//!
//! Covers:
//! - Grid movement FSM: walking, sub-tile interpolation, turning, ledge hops,
//!   surfing, freezing, hazard / trigger reporting
//! - Collision against `tilemap2d` attribute bits
//! - Warp pad and door resolution
//! - NPC blocking, interaction reach, line of sight
//! - Deterministic step encounters
//! - Paged typewriter dialogue with choices and lowering
//! - CWE-20 / 125 / 190 / 369 / 400 / 787 guards

use rpg_grid_world::*;
use std::collections::HashSet;

// --- helpers ---------------------------------------------------------------

const TILE: u32 = 16;

fn grid(cols: u32, rows: u32, f: impl Fn(u32, u32) -> TileCell) -> Vec<TileCell> {
    let mut v = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            v.push(f(c, r));
        }
    }
    v
}

fn layer(cells: &[TileCell], cols: u32, rows: u32) -> TileLayer<'_> {
    TileLayer::attach(cells, cols, rows, TILE, TILE).unwrap()
}

fn open_cells(cols: u32, rows: u32) -> Vec<TileCell> {
    grid(cols, rows, |_, _| TileCell::tile(1))
}

fn tick(
    actor: &mut GridActor,
    world: &GridWorld<'_>,
    input: Option<Facing>,
    enc: &mut EncounterState,
) -> TickReport {
    actor.advance(world, input, enc).unwrap()
}

// --- facing / rng ----------------------------------------------------------

#[test]
fn facing_deltas_and_reverse() {
    assert_eq!(Facing::Up.delta_xy(), (0, -1));
    assert_eq!(Facing::Down.delta_xy(), (0, 1));
    assert_eq!(Facing::Left.delta_xy(), (-1, 0));
    assert_eq!(Facing::Right.delta_xy(), (1, 0));
    for f in [Facing::Up, Facing::Down, Facing::Left, Facing::Right] {
        assert_eq!(f.reversed().reversed(), f);
        let (a, b) = f.delta_xy();
        let (c, d) = f.reversed().delta_xy();
        assert_eq!((a + c, b + d), (0, 0));
    }
}

#[test]
fn rng_is_deterministic_and_bounded() {
    let mut a = GridRng::seeded(42);
    let mut b = GridRng::seeded(42);
    for _ in 0..200 {
        assert_eq!(a.next_roll(), b.next_roll());
    }
    let mut z = GridRng::seeded(0);
    assert_ne!(z.next_roll(), z.next_roll());
    let mut r = GridRng::seeded(7);
    for bound in [1u32, 2, 3, 100, 256, u32::MAX] {
        for _ in 0..100 {
            assert!(r.roll_below(bound) < bound);
        }
    }
    assert_eq!(r.roll_below(0), 0);
}

// --- placement and parameters ---------------------------------------------

#[test]
fn place_and_parameter_validation() {
    let cells = open_cells(4, 3);
    let l = layer(&cells, 4, 3);
    assert!(GridActor::place(&l, 3, 2, Facing::Up).is_ok());
    for (x, y) in [
        (4, 0),
        (0, 3),
        (-1, 0),
        (0, -1),
        (i32::MAX, 0),
        (0, i32::MIN),
    ] {
        assert!(matches!(
            GridActor::place(&l, x, y, Facing::Up),
            Err(GridError::OutOfBounds { .. })
        ));
    }
    let mut a = GridActor::place(&l, 0, 0, Facing::Up).unwrap();
    assert!(a.set_speed(1).is_ok());
    assert!(a.set_speed(MAX_SPEED).is_ok());
    assert!(matches!(
        a.set_speed(0),
        Err(GridError::InvalidParameter { .. })
    ));
    assert!(matches!(
        a.set_speed(MAX_SPEED + 1),
        Err(GridError::InvalidParameter { .. })
    ));
    assert!(a.set_turn_delay(MAX_TURN_TICKS).is_ok());
    assert!(matches!(
        a.set_turn_delay(MAX_TURN_TICKS + 1),
        Err(GridError::InvalidParameter { .. })
    ));
}

// --- walking ---------------------------------------------------------------

#[test]
fn walk_interpolates_sub_tile_and_arrives() {
    let cells = open_cells(4, 4);
    let l = layer(&cells, 4, 4);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 1, Facing::Right).unwrap();
    a.set_speed(4).unwrap();

    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.started && !r.turned && !r.arrived);
    assert_eq!(a.state, MoveState::Walking);
    assert_eq!(a.walk_dest(), (2, 1));
    assert_eq!(a.walk_distance(), TILE);
    assert_eq!(a.pixel_position(&l), (16, 16));
    for expect in [20, 24, 28] {
        tick(&mut a, &world, None, &mut enc);
        assert_eq!(a.pixel_position(&l), (expect, 16));
        assert_eq!(a.state, MoveState::Walking);
    }
    let r = tick(&mut a, &world, None, &mut enc);
    assert!(r.arrived && !r.started);
    assert_eq!((a.tile_x, a.tile_y), (2, 1));
    assert_eq!(a.state, MoveState::Idle);
    assert_eq!(a.pixel_position(&l), (32, 16));
}

#[test]
fn held_input_chains_walks_without_idle_frame() {
    let cells = open_cells(5, 1);
    let l = layer(&cells, 5, 1);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    a.set_speed(MAX_SPEED).unwrap();
    // speed >= tile size: one tick starts, the next arrives and chains.
    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.started);
    for expect in 1..=3 {
        let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
        assert!(r.arrived && r.started);
        assert_eq!(a.tile_x, expect);
    }
}

#[test]
fn vertical_walk_uses_tile_height() {
    let cells = open_cells(2, 3);
    let l = TileLayer::attach(&cells, 2, 3, 8, 24).unwrap();
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Down).unwrap();
    a.set_speed(6).unwrap();
    tick(&mut a, &world, Some(Facing::Down), &mut enc);
    assert_eq!(a.walk_distance(), 24);
    for _ in 0..4 {
        tick(&mut a, &world, None, &mut enc);
    }
    assert_eq!((a.tile_x, a.tile_y), (0, 1));
    assert_eq!(a.pixel_position(&l), (0, 24));
}

#[test]
fn walls_edges_and_npcs_block() {
    let cells = grid(4, 3, |c, r| {
        if (c, r) == (2, 1) {
            TileCell::tile(1).with_attr(ATTR_SOLID)
        } else {
            TileCell::tile(1)
        }
    });
    let l = layer(&cells, 4, 3);
    let npcs = [Npc {
        id: 9,
        x: 1,
        y: 2,
        facing: Facing::Up,
        kind: NpcKind::Person,
        script: 0,
        sight_range: 0,
        blocking: true,
    }];
    let world = GridWorld::bind(&l, &[], &npcs, &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 1, Facing::Right).unwrap();
    // wall to the right
    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.bumped && !r.started);
    assert_eq!(a.state, MoveState::Idle);
    // blocking NPC below
    let r = tick(&mut a, &world, Some(Facing::Down), &mut enc);
    assert!(r.bumped && r.turned && !r.started);
    // map edge above-left
    let mut b = GridActor::place(&l, 0, 0, Facing::Up).unwrap();
    assert!(tick(&mut b, &world, Some(Facing::Up), &mut enc).bumped);
    assert!(tick(&mut b, &world, Some(Facing::Left), &mut enc).bumped);
    assert_eq!((b.tile_x, b.tile_y), (0, 0));
}

#[test]
fn map_edge_blocks_even_with_wrapping_layer() {
    let cells = open_cells(3, 3);
    let l = layer(&cells, 3, 3); // WrapMode::Wrap by default
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Left).unwrap();
    assert!(tick(&mut a, &world, Some(Facing::Left), &mut enc).bumped);
    assert_eq!((a.tile_x, a.tile_y), (0, 0));
}

// --- turning ---------------------------------------------------------------

#[test]
fn turning_pauses_before_walking() {
    let cells = open_cells(3, 3);
    let l = layer(&cells, 3, 3);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 1, Facing::Down).unwrap();
    a.set_turn_delay(3).unwrap();
    a.set_speed(MAX_SPEED).unwrap();

    let r = tick(&mut a, &world, Some(Facing::Up), &mut enc);
    assert!(r.turned && !r.started);
    assert_eq!(a.state, MoveState::Turning);
    assert_eq!(a.facing, Facing::Up);
    assert!(tick(&mut a, &world, Some(Facing::Up), &mut enc).turned);
    assert_eq!(a.state, MoveState::Turning);
    assert!(tick(&mut a, &world, Some(Facing::Up), &mut enc).turned);
    assert_eq!(a.state, MoveState::Turning); // three ticks observed in Turning
                                             // the next tick ends the turn and starts the walk in the same tick
    let r = tick(&mut a, &world, Some(Facing::Up), &mut enc);
    assert!(r.started);
    assert_eq!(a.state, MoveState::Walking);
}

#[test]
fn turning_released_returns_to_idle() {
    let cells = open_cells(3, 3);
    let l = layer(&cells, 3, 3);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 1, Facing::Down).unwrap();
    a.set_turn_delay(2).unwrap();
    tick(&mut a, &world, Some(Facing::Left), &mut enc);
    tick(&mut a, &world, None, &mut enc);
    tick(&mut a, &world, None, &mut enc);
    assert_eq!(a.state, MoveState::Idle);
    assert_eq!((a.tile_x, a.tile_y), (1, 1));
    assert_eq!(a.facing, Facing::Left);
}

// --- ledges ----------------------------------------------------------------

fn ledge_cells() -> Vec<TileCell> {
    grid(3, 6, |c, r| match (c, r) {
        (1, 2) => TileCell::tile(2).with_attr(ATTR_ONE_WAY),
        (1, 4) => TileCell::tile(2).with_attr(ATTR_ONE_WAY),
        (1, 5) => TileCell::tile(1).with_attr(ATTR_SOLID),
        _ => TileCell::tile(1),
    })
}

#[test]
fn ledge_hop_jumps_two_tiles_with_arc() {
    let cells = ledge_cells();
    let l = layer(&cells, 3, 6);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 1, Facing::Down).unwrap();
    a.set_speed(8).unwrap();
    let r = tick(&mut a, &world, Some(Facing::Down), &mut enc);
    assert!(r.started);
    assert_eq!(a.state, MoveState::LedgeHop);
    assert_eq!(a.walk_distance(), 2 * TILE);
    assert_eq!(a.walk_dest(), (1, 3));
    assert_eq!(a.hop_arc(8), 0);
    tick(&mut a, &world, None, &mut enc);
    tick(&mut a, &world, None, &mut enc); // progress 16 of 32: apex
    assert_eq!(a.walk_progress(), 16);
    assert_eq!(a.hop_arc(8), 8);
    tick(&mut a, &world, None, &mut enc);
    let r = tick(&mut a, &world, None, &mut enc);
    assert!(r.arrived && r.landed);
    assert_eq!((a.tile_x, a.tile_y), (1, 3));
    assert_eq!(a.state, MoveState::Idle);
    assert_eq!(a.hop_arc(8), 0);
}

#[test]
fn ledge_blocks_other_directions_and_bad_landings() {
    let cells = ledge_cells();
    let l = layer(&cells, 3, 6);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    // approaching from below is blocked
    let mut up = GridActor::place(&l, 1, 3, Facing::Up).unwrap();
    assert!(tick(&mut up, &world, Some(Facing::Up), &mut enc).bumped);
    // landing on a solid tile is blocked
    let mut down = GridActor::place(&l, 1, 3, Facing::Down).unwrap();
    assert!(tick(&mut down, &world, Some(Facing::Down), &mut enc).bumped);
    // sideways into a ledge is blocked
    let mut side = GridActor::place(&l, 0, 2, Facing::Right).unwrap();
    assert!(tick(&mut side, &world, Some(Facing::Right), &mut enc).bumped);
}

#[test]
fn ledge_facing_is_configurable() {
    let cells = grid(5, 1, |c, _| {
        if c == 2 {
            TileCell::tile(2).with_attr(ATTR_ONE_WAY)
        } else {
            TileCell::tile(1)
        }
    });
    let l = layer(&cells, 5, 1);
    let mut world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    world.ledge_facing = Facing::Right;
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 0, Facing::Right).unwrap();
    a.set_speed(MAX_SPEED).unwrap();
    assert!(tick(&mut a, &world, Some(Facing::Right), &mut enc).started);
    assert_eq!(a.walk_dest(), (3, 0));
}

#[test]
fn ledge_landing_off_map_blocks() {
    let cells = grid(1, 2, |_, r| {
        if r == 1 {
            TileCell::tile(2).with_attr(ATTR_ONE_WAY)
        } else {
            TileCell::tile(1)
        }
    });
    let l = layer(&cells, 1, 2);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Down).unwrap();
    assert!(tick(&mut a, &world, Some(Facing::Down), &mut enc).bumped);
}

// --- surfing ---------------------------------------------------------------

fn water_cells() -> Vec<TileCell> {
    grid(5, 1, |c, _| match c {
        1..=2 => TileCell::tile(3).with_attr(ATTR_WATER),
        4 => TileCell::tile(1).with_attr(ATTR_SOLID),
        _ => TileCell::tile(1),
    })
}

#[test]
fn water_blocks_without_surf() {
    let cells = water_cells();
    let l = layer(&cells, 5, 1);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    assert!(tick(&mut a, &world, Some(Facing::Right), &mut enc).bumped);
    assert!(!a.surfing);
}

#[test]
fn surfing_mounts_and_dismounts() {
    let cells = water_cells();
    let l = layer(&cells, 5, 1);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    a.can_surf = true;
    a.set_speed(MAX_SPEED).unwrap();

    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.mounted && r.started);
    assert!(a.surfing);
    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.arrived && r.started && !r.dismounted);
    assert_eq!((a.tile_x, a.tile_y), (1, 0));
    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.arrived && r.started);
    assert_eq!((a.tile_x, a.tile_y), (2, 0));
    // now heading onto land at (3, 0)
    let r = tick(&mut a, &world, None, &mut enc);
    assert!(r.arrived && r.dismounted);
    assert!(!a.surfing);
    assert_eq!(a.state, MoveState::Idle);
}

#[test]
fn surfing_rests_in_surfing_state_and_cannot_hop() {
    let cells = grid(3, 3, |c, r| match (c, r) {
        (1, 1) => TileCell::tile(3).with_attr(ATTR_WATER),
        (1, 2) => TileCell::tile(2).with_attr(ATTR_ONE_WAY),
        _ => TileCell::tile(3).with_attr(ATTR_WATER),
    });
    let l = layer(&cells, 3, 3);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 0, Facing::Down).unwrap();
    a.can_surf = true;
    a.set_speed(MAX_SPEED).unwrap();
    tick(&mut a, &world, Some(Facing::Down), &mut enc);
    tick(&mut a, &world, None, &mut enc);
    assert_eq!(a.state, MoveState::Surfing);
    // surfing actors cannot hop ledges
    assert!(tick(&mut a, &world, Some(Facing::Down), &mut enc).bumped);
    assert_eq!(a.state, MoveState::Surfing);
}

// --- frozen ----------------------------------------------------------------

#[test]
fn freeze_defers_during_walk_and_thaws() {
    let cells = open_cells(4, 1);
    let l = layer(&cells, 4, 1);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    a.set_speed(8).unwrap();
    tick(&mut a, &world, Some(Facing::Right), &mut enc);
    a.freeze_actor();
    assert_eq!(a.state, MoveState::Walking); // finishes its step first
    tick(&mut a, &world, Some(Facing::Right), &mut enc);
    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.arrived && !r.started);
    assert_eq!(a.state, MoveState::Frozen);
    // frozen ignores input
    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert_eq!(r, TickReport::default());
    assert_eq!(a.tile_x, 1);
    a.thaw_actor();
    assert_eq!(a.state, MoveState::Idle);
    assert!(tick(&mut a, &world, Some(Facing::Right), &mut enc).started);
}

#[test]
fn freeze_at_rest_is_immediate_and_aborts_turn() {
    let cells = open_cells(3, 3);
    let l = layer(&cells, 3, 3);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 1, Facing::Down).unwrap();
    a.set_turn_delay(5).unwrap();
    tick(&mut a, &world, Some(Facing::Up), &mut enc);
    assert_eq!(a.state, MoveState::Turning);
    a.freeze_actor();
    assert_eq!(a.state, MoveState::Frozen);
    a.thaw_actor();
    assert_eq!(a.state, MoveState::Idle);
}

// --- hazards, triggers, warps ---------------------------------------------

#[test]
fn hazard_and_trigger_are_reported_on_arrival() {
    let cells = grid(3, 1, |c, _| match c {
        1 => TileCell::tile(4).with_attr(ATTR_HAZARD),
        2 => TileCell::tile(5).with_attr(ATTR_TRIGGER),
        _ => TileCell::tile(1),
    });
    let l = layer(&cells, 3, 1);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    a.set_speed(MAX_SPEED).unwrap();
    tick(&mut a, &world, Some(Facing::Right), &mut enc);
    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.arrived && r.hazard && !r.trigger);
    let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
    assert!(r.arrived && r.trigger && !r.hazard);
}

fn pad(x: i32, y: i32, trigger: WarpTrigger, map: u16) -> WarpPad {
    WarpPad {
        x,
        y,
        trigger,
        target_map: map,
        target_x: 3,
        target_y: 7,
        target_facing: Facing::Up,
    }
}

#[test]
fn warp_on_enter_fires_on_arrival() {
    let cells = open_cells(3, 1);
    let l = layer(&cells, 3, 1);
    let warps = [pad(1, 0, WarpTrigger::OnEnter, 12)];
    let world = GridWorld::bind(&l, &warps, &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    a.set_speed(MAX_SPEED).unwrap();
    tick(&mut a, &world, Some(Facing::Right), &mut enc);
    let r = tick(&mut a, &world, None, &mut enc);
    assert_eq!(
        r.warp,
        Some(WarpTarget {
            map: 12,
            x: 3,
            y: 7,
            facing: Facing::Up
        })
    );
}

#[test]
fn warp_on_bump_needs_the_blocked_direction() {
    let cells = open_cells(3, 2);
    let l = layer(&cells, 3, 2);
    let warps = [pad(1, 1, WarpTrigger::OnBump(Facing::Down), 5)];
    let world = GridWorld::bind(&l, &warps, &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 1, Facing::Down).unwrap();
    // wrong direction (left is open): walks, no warp
    let r = tick(&mut a, &world, Some(Facing::Left), &mut enc);
    assert!(r.started && r.warp.is_none());
    // right direction at the door: bumps through the map edge
    let mut b = GridActor::place(&l, 1, 1, Facing::Down).unwrap();
    let r = tick(&mut b, &world, Some(Facing::Down), &mut enc);
    assert!(r.bumped);
    assert_eq!(r.warp.map(|w| w.map), Some(5));
}

#[test]
fn resolve_warp_matches_trigger_and_tile() {
    let warps = [
        pad(1, 1, WarpTrigger::OnEnter, 1),
        pad(1, 1, WarpTrigger::OnBump(Facing::Up), 2),
        pad(2, 2, WarpTrigger::OnEnter, 3),
    ];
    assert_eq!(
        resolve_warp(&warps, 1, 1, WarpTrigger::OnEnter).map(|w| w.map),
        Some(1)
    );
    assert_eq!(
        resolve_warp(&warps, 1, 1, WarpTrigger::OnBump(Facing::Up)).map(|w| w.map),
        Some(2)
    );
    assert_eq!(
        resolve_warp(&warps, 1, 1, WarpTrigger::OnBump(Facing::Down)),
        None
    );
    assert_eq!(resolve_warp(&warps, 9, 9, WarpTrigger::OnEnter), None);
    assert_eq!(resolve_warp(&[], 0, 0, WarpTrigger::OnEnter), None);
}

#[test]
fn warp_validation() {
    let cells = open_cells(3, 3);
    let l = layer(&cells, 3, 3);
    assert!(validate_warp(&l, &pad(2, 2, WarpTrigger::OnEnter, 0)).is_ok());
    assert!(matches!(
        validate_warp(&l, &pad(3, 0, WarpTrigger::OnEnter, 0)),
        Err(GridError::OutOfBounds { .. })
    ));
    let mut bad_target = pad(0, 0, WarpTrigger::OnEnter, 0);
    bad_target.target_x = -1;
    assert!(validate_warp(&l, &bad_target).is_err());
    bad_target.target_x = 0;
    bad_target.target_y = MAX_TARGET_COORD;
    assert!(validate_warp(&l, &bad_target).is_err());
}

// --- NPCs ------------------------------------------------------------------

fn npc(id: u16, x: i32, y: i32, facing: Facing, kind: NpcKind, sight: u8, blocking: bool) -> Npc {
    Npc {
        id,
        x,
        y,
        facing,
        kind,
        script: id,
        sight_range: sight,
        blocking,
    }
}

#[test]
fn interact_target_finds_npc_within_reach() {
    let cells = open_cells(6, 3);
    let l = layer(&cells, 6, 3);
    let npcs = [
        npc(1, 3, 1, Facing::Left, NpcKind::Person, 0, true),
        npc(2, 1, 1, Facing::Right, NpcKind::Sign, 0, true),
    ];
    // adjacent
    assert_eq!(
        interact_target(&l, &npcs, 2, 1, Facing::Right, 1),
        Ok(Some(0))
    );
    // out of reach, then within reach
    assert_eq!(
        interact_target(&l, &npcs, 0, 1, Facing::Right, 1),
        Ok(Some(1))
    );
    assert_eq!(
        interact_target(&l, &npcs, 4, 1, Facing::Left, 1),
        Ok(Some(0))
    );
    assert_eq!(interact_target(&l, &npcs, 5, 1, Facing::Left, 1), Ok(None));
    assert_eq!(
        interact_target(&l, &npcs, 5, 1, Facing::Left, 2),
        Ok(Some(0))
    );
    // facing away
    assert_eq!(
        interact_target(&l, &npcs, 2, 1, Facing::Left, 1),
        Ok(Some(1))
    );
    assert_eq!(interact_target(&l, &npcs, 2, 1, Facing::Up, 3), Ok(None));
}

#[test]
fn interact_target_reaches_over_counters_and_stops_at_edges() {
    let cells = grid(5, 1, |c, _| {
        if c == 2 {
            TileCell::tile(9).with_attr(ATTR_SOLID)
        } else {
            TileCell::tile(1)
        }
    });
    let l = layer(&cells, 5, 1);
    let npcs = [npc(1, 3, 0, Facing::Left, NpcKind::Person, 0, true)];
    assert_eq!(
        interact_target(&l, &npcs, 1, 0, Facing::Right, 2),
        Ok(Some(0))
    );
    assert_eq!(interact_target(&l, &npcs, 0, 0, Facing::Left, 4), Ok(None));
}

#[test]
fn interact_target_rejects_bad_reach() {
    let cells = open_cells(2, 2);
    let l = layer(&cells, 2, 2);
    for reach in [0, MAX_INTERACT_RANGE + 1, 255] {
        assert!(matches!(
            interact_target(&l, &[], 0, 0, Facing::Up, reach),
            Err(GridError::InvalidParameter { .. })
        ));
    }
    // hostile coordinates never panic
    assert_eq!(
        interact_target(&l, &[], i32::MIN, i32::MAX, Facing::Up, 4),
        Ok(None)
    );
    assert_eq!(
        interact_target(&l, &[], i32::MAX, i32::MIN, Facing::Left, 4),
        Ok(None)
    );
}

#[test]
fn npc_line_of_sight() {
    let cells = grid(8, 1, |c, _| {
        if c == 5 {
            TileCell::tile(9).with_attr(ATTR_SOLID)
        } else {
            TileCell::tile(1)
        }
    });
    let l = layer(&cells, 8, 1);
    let t = npc(1, 0, 0, Facing::Right, NpcKind::Trainer, 6, true);
    assert!(npc_sees_actor(&l, &t, 1, 0));
    assert!(npc_sees_actor(&l, &t, 4, 0));
    // wall at 5 blocks sight beyond it
    assert!(!npc_sees_actor(&l, &t, 6, 0));
    // range limit
    let short = npc(2, 0, 0, Facing::Right, NpcKind::Trainer, 2, true);
    assert!(npc_sees_actor(&l, &short, 2, 0));
    assert!(!npc_sees_actor(&l, &short, 3, 0));
    // facing away / blind / self tile / off axis
    let away = npc(3, 3, 0, Facing::Left, NpcKind::Trainer, 6, true);
    assert!(!npc_sees_actor(&l, &away, 4, 0));
    let blind = npc(4, 0, 0, Facing::Right, NpcKind::Trainer, 0, true);
    assert!(!npc_sees_actor(&l, &blind, 1, 0));
    assert!(!npc_sees_actor(&l, &t, 0, 0));
    assert!(!npc_sees_actor(&l, &t, 1, 1));
    // sight range is clamped, never loops unboundedly
    let huge = npc(5, 0, 0, Facing::Right, NpcKind::Trainer, 255, true);
    assert!(npc_sees_actor(&l, &huge, 4, 0));
}

#[test]
fn face_npc_toward_turns_people_not_signs() {
    let mut p = npc(1, 1, 1, Facing::Up, NpcKind::Person, 0, true);
    assert!(face_npc_toward(&mut p, Facing::Up)); // actor looks up => NPC looks down
    assert_eq!(p.facing, Facing::Down);
    assert!(!face_npc_toward(&mut p, Facing::Up));
    let mut s = npc(2, 1, 1, Facing::Left, NpcKind::Sign, 0, true);
    assert!(!face_npc_toward(&mut s, Facing::Up));
    assert_eq!(s.facing, Facing::Left);
    let mut t = npc(3, 1, 1, Facing::Left, NpcKind::Trainer, 3, true);
    assert!(face_npc_toward(&mut t, Facing::Left));
    assert_eq!(t.facing, Facing::Right);
}

#[test]
fn npc_lookup_helpers() {
    let npcs = [
        npc(1, 1, 1, Facing::Up, NpcKind::Person, 0, false),
        npc(2, 2, 2, Facing::Up, NpcKind::Sign, 0, true),
    ];
    assert_eq!(find_npc_at(&npcs, 2, 2), Some(1));
    assert_eq!(find_npc_at(&npcs, 0, 0), None);
    assert!(!npc_blocks_tile(&npcs, 1, 1)); // non-blocking
    assert!(npc_blocks_tile(&npcs, 2, 2));
    assert!(!npc_blocks_tile(&npcs, i64::MIN, i64::MAX));
}

#[test]
fn non_blocking_npc_can_be_walked_through() {
    let cells = open_cells(3, 1);
    let l = layer(&cells, 3, 1);
    let npcs = [npc(1, 1, 0, Facing::Up, NpcKind::Person, 0, false)];
    let world = GridWorld::bind(&l, &[], &npcs, &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    assert!(tick(&mut a, &world, Some(Facing::Right), &mut enc).started);
}

// --- encounters ------------------------------------------------------------

const GRASS: u16 = 7;

fn entries() -> [EncounterEntry; 3] {
    [
        EncounterEntry {
            species: 10,
            min_level: 3,
            max_level: 5,
            weight: 6,
        },
        EncounterEntry {
            species: 11,
            min_level: 4,
            max_level: 4,
            weight: 3,
        },
        EncounterEntry {
            species: 12,
            min_level: 10,
            max_level: 12,
            weight: 1,
        },
    ]
}

fn table<'a>(e: &'a [EncounterEntry], min: u16, max: u16, rate: u16) -> EncounterTable<'a> {
    EncounterTable {
        tile_id: GRASS,
        min_steps: min,
        max_steps: max,
        rate_q8: rate,
        entries: e,
    }
}

#[test]
fn encounter_guaranteed_at_max_steps() {
    let e = entries();
    let t = table(&e, 3, 6, 0);
    let mut s = EncounterState::start(5);
    for n in 1..6 {
        assert_eq!(s.register_step(&t), None, "step {n}");
    }
    assert_eq!(s.steps, 5);
    assert!(s.register_step(&t).is_some());
    assert_eq!(s.steps, 0);
}

#[test]
fn encounter_never_before_min_steps_and_always_at_full_rate() {
    let e = entries();
    let t = table(&e, 3, 200, RATE_ONE);
    let mut s = EncounterState::start(9);
    assert!(s.register_step(&t).is_none());
    assert!(s.register_step(&t).is_none());
    assert!(s.register_step(&t).is_some());
}

#[test]
fn encounter_is_deterministic_per_seed() {
    let e = entries();
    let t = table(&e, 2, 30, 40);
    let run = |seed: u64| {
        let mut s = EncounterState::start(seed);
        (0..2000).map(|_| s.register_step(&t)).collect::<Vec<_>>()
    };
    assert_eq!(run(77), run(77));
    assert_ne!(run(77), run(78));
}

#[test]
fn encounter_rolls_respect_weights_and_levels() {
    let e = entries();
    let t = table(&e, 1, 1, 0);
    let mut s = EncounterState::start(1234);
    let mut counts = [0u32; 3];
    for _ in 0..6000 {
        let r = s.register_step(&t).expect("max_steps = 1 always fires");
        let (idx, entry) = e
            .iter()
            .enumerate()
            .find(|(_, x)| x.species == r.species)
            .expect("species comes from the table");
        assert!(r.level >= entry.min_level && r.level <= entry.max_level);
        counts[idx] += 1;
    }
    assert!(counts.iter().all(|&c| c > 0));
    assert!(counts[0] > counts[1] && counts[1] > counts[2]);
}

#[test]
fn encounter_table_validation() {
    let e = entries();
    assert!(validate_encounter_table(0, &table(&e, 3, 6, 64)).is_ok());
    let cases = [
        table(&e, 3, 0, 64),
        table(&e, 9, 3, 64),
        table(&e, 3, 6, RATE_ONE + 1),
        table(&[], 3, 6, 64),
        EncounterTable {
            tile_id: 0,
            ..table(&e, 3, 6, 64)
        },
    ];
    for (i, c) in cases.iter().enumerate() {
        assert_eq!(
            validate_encounter_table(i, c),
            Err(GridError::InvalidEncounterTable { index: i })
        );
    }
    let zero_weight = [EncounterEntry { weight: 0, ..e[0] }];
    assert!(validate_encounter_table(0, &table(&zero_weight, 1, 2, 3)).is_err());
    let zero_level = [EncounterEntry {
        min_level: 0,
        ..e[0]
    }];
    assert!(validate_encounter_table(0, &table(&zero_level, 1, 2, 3)).is_err());
    let inverted = [EncounterEntry {
        min_level: 9,
        max_level: 2,
        ..e[0]
    }];
    assert!(validate_encounter_table(0, &table(&inverted, 1, 2, 3)).is_err());
    let many = vec![e[0]; MAX_ENCOUNTER_ENTRIES + 1];
    assert!(matches!(
        validate_encounter_table(0, &table(&many, 1, 2, 3)),
        Err(GridError::TooMany { .. })
    ));
    let max = vec![e[0]; MAX_ENCOUNTER_ENTRIES];
    assert!(validate_encounter_table(0, &table(&max, 1, 2, 3)).is_ok());
}

#[test]
fn walking_on_grass_triggers_encounters() {
    let cells = grid(40, 1, |c, _| {
        if c == 0 {
            TileCell::tile(1)
        } else {
            TileCell::tile(GRASS)
        }
    });
    let l = layer(&cells, 40, 1);
    let e = entries();
    let tables = [table(&e, 4, 8, 0)];
    let world = GridWorld::bind(&l, &[], &[], &tables).unwrap();
    let mut enc = EncounterState::start(3);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    a.set_speed(MAX_SPEED).unwrap();
    let mut first = None;
    tick(&mut a, &world, Some(Facing::Right), &mut enc);
    for step in 1..=20 {
        let r = tick(&mut a, &world, Some(Facing::Right), &mut enc);
        assert!(r.arrived);
        if r.encounter.is_some() && first.is_none() {
            first = Some(step);
        }
    }
    // rate 0: the first encounter lands exactly on max_steps = 8
    assert_eq!(first, Some(8));
}

#[test]
fn encounters_do_not_count_on_plain_tiles_or_hops() {
    let cells = grid(5, 3, |c, r| match (c, r) {
        (2, 1) => TileCell::tile(GRASS).with_attr(ATTR_ONE_WAY),
        (2, 2) => TileCell::tile(GRASS),
        _ => TileCell::tile(1),
    });
    let l = layer(&cells, 5, 3);
    let e = entries();
    let tables = [table(&e, 1, 1, RATE_ONE)];
    let world = GridWorld::bind(&l, &[], &[], &tables).unwrap();
    let mut enc = EncounterState::start(3);
    // plain tiles accumulate nothing
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    a.set_speed(MAX_SPEED).unwrap();
    tick(&mut a, &world, Some(Facing::Right), &mut enc);
    let r = tick(&mut a, &world, None, &mut enc);
    assert!(r.arrived && r.encounter.is_none());
    assert_eq!(enc.steps, 0);
    // a ledge hop landing on grass does not count as a step
    let mut b = GridActor::place(&l, 2, 0, Facing::Down).unwrap();
    b.set_speed(MAX_SPEED).unwrap();
    tick(&mut b, &world, Some(Facing::Down), &mut enc);
    tick(&mut b, &world, None, &mut enc);
    let r = tick(&mut b, &world, None, &mut enc);
    let _ = r;
    assert_eq!(enc.steps, 0);
}

#[test]
fn warp_pre_empts_encounter() {
    let cells = grid(3, 1, |_, _| TileCell::tile(GRASS));
    let l = layer(&cells, 3, 1);
    let e = entries();
    let tables = [table(&e, 1, 1, RATE_ONE)];
    let warps = [pad(1, 0, WarpTrigger::OnEnter, 4)];
    let world = GridWorld::bind(&l, &warps, &[], &tables).unwrap();
    let mut enc = EncounterState::start(3);
    let mut a = GridActor::place(&l, 0, 0, Facing::Right).unwrap();
    a.set_speed(MAX_SPEED).unwrap();
    tick(&mut a, &world, Some(Facing::Right), &mut enc);
    let r = tick(&mut a, &world, None, &mut enc);
    assert!(r.warp.is_some() && r.encounter.is_none());
}

// --- world binding ---------------------------------------------------------

#[test]
fn bind_validates_tables() {
    let cells = open_cells(4, 4);
    let l = layer(&cells, 4, 4);
    let e = entries();

    let too_many_warps = vec![pad(0, 0, WarpTrigger::OnEnter, 0); MAX_WARPS + 1];
    assert!(matches!(
        GridWorld::bind(&l, &too_many_warps, &[], &[]),
        Err(GridError::TooMany { what: "warps", .. })
    ));
    let max_warps = vec![pad(0, 0, WarpTrigger::OnEnter, 0); MAX_WARPS];
    assert!(GridWorld::bind(&l, &max_warps, &[], &[]).is_ok());

    let n = npc(1, 0, 0, Facing::Up, NpcKind::Person, 0, true);
    let too_many_npcs = vec![n; MAX_NPCS + 1];
    assert!(matches!(
        GridWorld::bind(&l, &[], &too_many_npcs, &[]),
        Err(GridError::TooMany { what: "npcs", .. })
    ));
    let off_map = [npc(1, 4, 0, Facing::Up, NpcKind::Person, 0, true)];
    assert!(matches!(
        GridWorld::bind(&l, &[], &off_map, &[]),
        Err(GridError::OutOfBounds { .. })
    ));
    let neg = [npc(1, -1, 0, Facing::Up, NpcKind::Person, 0, true)];
    assert!(GridWorld::bind(&l, &[], &neg, &[]).is_err());
    let blind_far = [npc(
        1,
        0,
        0,
        Facing::Up,
        NpcKind::Trainer,
        MAX_SIGHT_RANGE + 1,
        true,
    )];
    assert!(matches!(
        GridWorld::bind(&l, &[], &blind_far, &[]),
        Err(GridError::InvalidParameter { .. })
    ));
    let bad_warp = [pad(9, 9, WarpTrigger::OnEnter, 0)];
    assert!(GridWorld::bind(&l, &bad_warp, &[], &[]).is_err());

    let many_tables = vec![table(&e, 1, 2, 3); MAX_ENCOUNTER_TABLES + 1];
    assert!(matches!(
        GridWorld::bind(&l, &[], &[], &many_tables),
        Err(GridError::TooMany { .. })
    ));
    let bad_table = [table(&e, 5, 1, 3)];
    assert_eq!(
        GridWorld::bind(&l, &[], &[], &bad_table).err(),
        Some(GridError::InvalidEncounterTable { index: 0 })
    );
    let w = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    assert_eq!(w.ledge_facing, Facing::Down);
    assert!(w.table_for_tile(0, 0).is_none());
}

#[test]
fn table_for_tile_matches_tile_id() {
    let cells = grid(3, 1, |c, _| TileCell::tile(if c == 1 { GRASS } else { 1 }));
    let l = layer(&cells, 3, 1);
    let e = entries();
    let tables = [table(&e, 1, 2, 3)];
    let w = GridWorld::bind(&l, &[], &[], &tables).unwrap();
    assert!(w.table_for_tile(1, 0).is_some());
    assert!(w.table_for_tile(0, 0).is_none());
    // hostile coordinates do not wrap into the map
    assert!(w.table_for_tile(i32::MAX, i32::MIN).is_none());
    assert!(w.table_for_tile(4, 0).is_none());
    assert!(w.table_for_tile(-2, 0).is_none());
}

#[test]
fn errors_display() {
    let errors = [
        GridError::OutOfBounds { x: 1, y: 2 },
        GridError::TooMany {
            what: "warps",
            count: 3,
            max: 2,
        },
        GridError::InvalidParameter {
            what: "speed",
            value: 0,
        },
        GridError::InvalidEncounterTable { index: 1 },
        GridError::InvalidDialogue {
            page: 2,
            reason: "x",
        },
        GridError::InvalidState { what: "y" },
        GridError::BufferTooSmall {
            required: 2,
            actual: 1,
        },
        GridError::SizeOverflow,
    ];
    let mut seen = HashSet::new();
    for e in errors {
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(seen.insert(s));
    }
}

// --- dialogue --------------------------------------------------------------

fn run_ticks(d: &mut DialogueBox<'_>, input: DialogueInput, n: usize) -> DialogueEvent {
    let mut last = DialogueEvent::Quiet;
    for _ in 0..n {
        last = d.tick_dialogue(input).unwrap();
    }
    last
}

#[test]
fn typewriter_reveals_across_lines() {
    let lines = ["Hello", "World"];
    let pages = [DialoguePage {
        lines: &lines,
        choices: &[],
        choice_targets: &[],
    }];
    let mut d = DialogueBox::open(&pages).unwrap();
    d.set_text_speed(256).unwrap(); // one char per tick
    assert_eq!(d.dialogue_state(), DialogueState::Typing);
    assert_eq!(d.visible_line(0), "");
    run_ticks(&mut d, DialogueInput::Idle, 3);
    assert_eq!(d.visible_line(0), "Hel");
    assert_eq!(d.visible_line(1), "");
    run_ticks(&mut d, DialogueInput::Idle, 4);
    assert_eq!(d.visible_line(0), "Hello");
    assert_eq!(d.visible_line(1), "Wo");
    assert_eq!(d.revealed_chars(), 7);
    let ev = run_ticks(&mut d, DialogueInput::Idle, 3);
    assert_eq!(ev, DialogueEvent::PageComplete);
    assert_eq!(d.dialogue_state(), DialogueState::WaitAdvance);
    assert_eq!(d.visible_line(1), "World");
    assert_eq!(d.visible_line(2), "");
    assert_eq!(d.visible_line(99), "");
}

#[test]
fn typewriter_fractional_speed() {
    let lines = ["abcd"];
    let pages = [DialoguePage {
        lines: &lines,
        choices: &[],
        choice_targets: &[],
    }];
    let mut d = DialogueBox::open(&pages).unwrap();
    d.set_text_speed(128).unwrap(); // half a char per tick
    run_ticks(&mut d, DialogueInput::Idle, 1);
    assert_eq!(d.revealed_chars(), 0);
    run_ticks(&mut d, DialogueInput::Idle, 1);
    assert_eq!(d.revealed_chars(), 1);
    run_ticks(&mut d, DialogueInput::Idle, 4);
    assert_eq!(d.revealed_chars(), 3);
    // a huge speed completes a page in one tick, never overflowing
    let mut fast = DialogueBox::open(&pages).unwrap();
    fast.set_text_speed(MAX_TEXT_SPEED_Q8).unwrap();
    assert_eq!(
        fast.tick_dialogue(DialogueInput::Idle),
        Ok(DialogueEvent::PageComplete)
    );
}

#[test]
fn confirm_skips_typing_then_advances_pages() {
    let l1 = ["One"];
    let l2 = ["Two"];
    let pages = [
        DialoguePage {
            lines: &l1,
            choices: &[],
            choice_targets: &[],
        },
        DialoguePage {
            lines: &l2,
            choices: &[],
            choice_targets: &[],
        },
    ];
    let mut d = DialogueBox::open(&pages).unwrap();
    assert_eq!(
        d.tick_dialogue(DialogueInput::Confirm),
        Ok(DialogueEvent::PageComplete)
    );
    assert_eq!(d.visible_line(0), "One");
    // up/down/idle do nothing while waiting
    assert_eq!(d.tick_dialogue(DialogueInput::Up), Ok(DialogueEvent::Quiet));
    assert_eq!(
        d.tick_dialogue(DialogueInput::Confirm),
        Ok(DialogueEvent::PageAdvanced(1))
    );
    assert_eq!(d.page_index(), 1);
    assert_eq!(d.revealed_chars(), 0);
    d.tick_dialogue(DialogueInput::Confirm).unwrap();
    assert_eq!(
        d.tick_dialogue(DialogueInput::Confirm),
        Ok(DialogueEvent::Finished)
    );
    assert_eq!(d.dialogue_state(), DialogueState::Finished);
    // finished is inert
    assert_eq!(
        d.tick_dialogue(DialogueInput::Confirm),
        Ok(DialogueEvent::Quiet)
    );
}

#[test]
fn cancel_ends_dialogue() {
    let l = ["Hi"];
    let pages = [DialoguePage {
        lines: &l,
        choices: &[],
        choice_targets: &[],
    }];
    let mut d = DialogueBox::open(&pages).unwrap();
    assert_eq!(
        d.tick_dialogue(DialogueInput::Cancel),
        Ok(DialogueEvent::Cancelled)
    );
    assert_eq!(d.dialogue_state(), DialogueState::Finished);
    let mut e = DialogueBox::open(&pages).unwrap();
    e.tick_dialogue(DialogueInput::Confirm).unwrap();
    assert_eq!(
        e.tick_dialogue(DialogueInput::Cancel),
        Ok(DialogueEvent::Cancelled)
    );
}

#[test]
fn empty_page_completes_immediately() {
    let pages = [DialoguePage {
        lines: &[],
        choices: &[],
        choice_targets: &[],
    }];
    let mut d = DialogueBox::open(&pages).unwrap();
    assert_eq!(
        d.tick_dialogue(DialogueInput::Idle),
        Ok(DialogueEvent::PageComplete)
    );
    assert_eq!(d.dialogue_state(), DialogueState::WaitAdvance);
}

#[test]
fn choices_move_clamp_and_branch() {
    let q = ["Yes or no?"];
    let yes = ["Great!"];
    let no = ["Oh."];
    let choices = ["Yes", "No"];
    let targets = [2u16, 3];
    let pages = [
        DialoguePage {
            lines: &q,
            choices: &choices,
            choice_targets: &targets,
        },
        DialoguePage {
            lines: &q,
            choices: &[],
            choice_targets: &[],
        },
        DialoguePage {
            lines: &yes,
            choices: &[],
            choice_targets: &[],
        },
        DialoguePage {
            lines: &no,
            choices: &[],
            choice_targets: &[],
        },
    ];
    let mut d = DialogueBox::open(&pages).unwrap();
    d.set_text_speed(MAX_TEXT_SPEED_Q8).unwrap();
    assert_eq!(
        d.tick_dialogue(DialogueInput::Idle),
        Ok(DialogueEvent::PageComplete)
    );
    assert_eq!(d.dialogue_state(), DialogueState::Choosing);
    // clamp at the top
    assert_eq!(d.tick_dialogue(DialogueInput::Up), Ok(DialogueEvent::Quiet));
    assert_eq!(
        d.tick_dialogue(DialogueInput::Down),
        Ok(DialogueEvent::CursorMoved(1))
    );
    // clamp at the bottom
    assert_eq!(
        d.tick_dialogue(DialogueInput::Down),
        Ok(DialogueEvent::Quiet)
    );
    assert_eq!(d.cursor_index(), 1);
    // cancel is ignored while choosing
    assert_eq!(
        d.tick_dialogue(DialogueInput::Cancel),
        Ok(DialogueEvent::Quiet)
    );
    assert_eq!(
        d.tick_dialogue(DialogueInput::Confirm),
        Ok(DialogueEvent::ChoiceMade(1))
    );
    assert_eq!(d.last_choice_made(), Some(1));
    assert_eq!(d.page_index(), 3);
    assert_eq!(d.visible_line(0), "");
    assert_eq!(d.cursor_index(), 0);

    // choose "Yes" instead
    let mut e = DialogueBox::open(&pages).unwrap();
    e.set_text_speed(MAX_TEXT_SPEED_Q8).unwrap();
    e.tick_dialogue(DialogueInput::Idle).unwrap();
    assert_eq!(
        e.tick_dialogue(DialogueInput::Confirm),
        Ok(DialogueEvent::ChoiceMade(0))
    );
    assert_eq!(e.page_index(), 2);
}

#[test]
fn choices_fall_through_without_targets() {
    let q = ["Pick"];
    let choices = ["A", "B", "C"];
    let end = ["End"];
    let pages = [
        DialoguePage {
            lines: &q,
            choices: &choices,
            choice_targets: &[],
        },
        DialoguePage {
            lines: &end,
            choices: &[],
            choice_targets: &[],
        },
    ];
    let mut d = DialogueBox::open(&pages).unwrap();
    d.tick_dialogue(DialogueInput::Confirm).unwrap();
    d.tick_dialogue(DialogueInput::Down).unwrap();
    d.tick_dialogue(DialogueInput::Down).unwrap();
    assert_eq!(
        d.tick_dialogue(DialogueInput::Up),
        Ok(DialogueEvent::CursorMoved(1))
    );
    assert_eq!(
        d.tick_dialogue(DialogueInput::Confirm),
        Ok(DialogueEvent::ChoiceMade(1))
    );
    assert_eq!(d.page_index(), 1);
    // last page's choices finish the dialogue
    let lone = [DialoguePage {
        lines: &q,
        choices: &choices,
        choice_targets: &[],
    }];
    let mut f = DialogueBox::open(&lone).unwrap();
    f.tick_dialogue(DialogueInput::Confirm).unwrap();
    f.tick_dialogue(DialogueInput::Confirm).unwrap();
    assert_eq!(f.dialogue_state(), DialogueState::Finished);
}

#[test]
fn dialogue_validation() {
    let ok = ["fine"];
    let page = |lines: &'static [&'static str]| DialoguePage {
        lines,
        choices: &[],
        choice_targets: &[],
    };
    assert!(matches!(
        DialogueBox::open(&[]),
        Err(GridError::InvalidDialogue { .. })
    ));
    let too_many_pages = vec![page(&["a"]); MAX_PAGES + 1];
    assert!(matches!(
        DialogueBox::open(&too_many_pages),
        Err(GridError::TooMany { .. })
    ));
    let max_pages = vec![page(&["a"]); MAX_PAGES];
    assert!(DialogueBox::open(&max_pages).is_ok());

    let five = ["1", "2", "3", "4", "5"];
    assert!(DialogueBox::open(&[DialoguePage {
        lines: &five,
        choices: &[],
        choice_targets: &[]
    }])
    .is_err());
    let long = "x".repeat(MAX_LINE_LEN + 1);
    let long_lines = [long.as_str()];
    assert!(DialogueBox::open(&[DialoguePage {
        lines: &long_lines,
        choices: &[],
        choice_targets: &[]
    }])
    .is_err());
    let exact = "x".repeat(MAX_LINE_LEN);
    let exact_lines = [exact.as_str()];
    assert!(DialogueBox::open(&[DialoguePage {
        lines: &exact_lines,
        choices: &[],
        choice_targets: &[]
    }])
    .is_ok());
    for bad in [
        "tab\there",
        "caf\u{e9}",
        "nul\0",
        "\u{7f}",
        "emoji \u{1F600}",
    ] {
        let b = [bad];
        assert!(DialogueBox::open(&[DialoguePage {
            lines: &b,
            choices: &[],
            choice_targets: &[]
        }])
        .is_err());
    }
    let many_choices = ["a", "b", "c", "d", "e"];
    assert!(DialogueBox::open(&[DialoguePage {
        lines: &ok,
        choices: &many_choices,
        choice_targets: &[]
    }])
    .is_err());
    let bad_choice = ["caf\u{e9}"];
    assert!(DialogueBox::open(&[DialoguePage {
        lines: &ok,
        choices: &bad_choice,
        choice_targets: &[]
    }])
    .is_err());
    let two = ["a", "b"];
    let one_target = [0u16];
    assert!(DialogueBox::open(&[DialoguePage {
        lines: &ok,
        choices: &two,
        choice_targets: &one_target
    }])
    .is_err());
    let oob_targets = [0u16, 9];
    assert!(DialogueBox::open(&[DialoguePage {
        lines: &ok,
        choices: &two,
        choice_targets: &oob_targets
    }])
    .is_err());

    let mut d = DialogueBox::open(&max_pages).unwrap();
    assert!(d.set_text_speed(0).is_err());
    assert!(d.set_text_speed(MAX_TEXT_SPEED_Q8 + 1).is_err());
    assert!(d.set_text_speed(1).is_ok());
}

fn style() -> DialogueStyle {
    DialogueStyle {
        col: 2,
        row: 1,
        inner_cols: 12,
        inner_rows: 3,
        glyph_base: 100,
        cursor_tile: 99,
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
        tint: [10, 20, 30],
    }
}

fn cell_at(cells: &[TileBatchCell], col: u16, row: u16) -> TileBatchCell {
    *cells
        .iter()
        .find(|c| c.grid_col == col && c.grid_row == row)
        .expect("cell present")
}

#[test]
fn dialogue_lowers_frame_and_glyphs() {
    let lines = ["Hi there"];
    let pages = [DialoguePage {
        lines: &lines,
        choices: &[],
        choice_targets: &[],
    }];
    let mut d = DialogueBox::open(&pages).unwrap();
    let st = style();
    let n = count_dialogue_cells(&st).unwrap();
    assert_eq!(n, 14 * 5);
    let mut out = vec![TileBatchCell::new(0, 0, 0); n];

    // nothing revealed yet: interior is all fill
    assert_eq!(lower_dialogue_cells_into(&d, &st, &mut out), Ok(n));
    assert_eq!(cell_at(&out, 3, 2).tile_id, 5);

    d.tick_dialogue(DialogueInput::Confirm).unwrap();
    assert_eq!(lower_dialogue_cells_into(&d, &st, &mut out), Ok(n));
    // frame
    assert_eq!(cell_at(&out, 2, 1).tile_id, 1);
    assert_eq!(cell_at(&out, 5, 1).tile_id, 2);
    assert_eq!(cell_at(&out, 15, 1).tile_id, 3);
    assert_eq!(cell_at(&out, 2, 3).tile_id, 4);
    assert_eq!(cell_at(&out, 15, 3).tile_id, 6);
    assert_eq!(cell_at(&out, 2, 5).tile_id, 7);
    assert_eq!(cell_at(&out, 8, 5).tile_id, 8);
    assert_eq!(cell_at(&out, 15, 5).tile_id, 9);
    // glyphs: 'H' = 0x48 -> base + 0x28; space -> fill
    assert_eq!(cell_at(&out, 3, 2).tile_id, 100 + 0x28);
    assert_eq!(cell_at(&out, 4, 2).tile_id, 100 + (b'i' - 0x20) as u16);
    assert_eq!(cell_at(&out, 5, 2).tile_id, 5);
    assert_eq!(cell_at(&out, 6, 2).tile_id, 100 + (b't' - 0x20) as u16);
    assert_eq!(cell_at(&out, 3, 2).color_rgb, [10, 20, 30]);
    // one cell per grid position
    let unique: HashSet<(u16, u16)> = out.iter().map(|c| (c.grid_col, c.grid_row)).collect();
    assert_eq!(unique.len(), n);
}

#[test]
fn dialogue_lowering_shows_choices_and_cursor() {
    let q = ["Sure?"];
    let choices = ["Yes", "No"];
    let pages = [DialoguePage {
        lines: &q,
        choices: &choices,
        choice_targets: &[],
    }];
    let mut d = DialogueBox::open(&pages).unwrap();
    let st = style();
    let n = count_dialogue_cells(&st).unwrap();
    let mut out = vec![TileBatchCell::new(0, 0, 0); n];
    // while typing the choices are hidden
    lower_dialogue_cells_into(&d, &st, &mut out).unwrap();
    assert_eq!(cell_at(&out, 3, 3).tile_id, 5);
    d.tick_dialogue(DialogueInput::Confirm).unwrap();
    lower_dialogue_cells_into(&d, &st, &mut out).unwrap();
    // row 1 of the interior = grid row 3: cursor at col 3, text from col 4
    assert_eq!(cell_at(&out, 3, 3).tile_id, 99);
    assert_eq!(cell_at(&out, 3, 4).tile_id, 5);
    assert_eq!(cell_at(&out, 4, 3).tile_id, 100 + (b'Y' - 0x20) as u16);
    d.tick_dialogue(DialogueInput::Down).unwrap();
    lower_dialogue_cells_into(&d, &st, &mut out).unwrap();
    assert_eq!(cell_at(&out, 3, 3).tile_id, 5);
    assert_eq!(cell_at(&out, 3, 4).tile_id, 99);
    assert_eq!(cell_at(&out, 4, 4).tile_id, 100 + (b'N' - 0x20) as u16);
}

#[test]
fn dialogue_lowering_guards() {
    let lines = ["Hello"];
    let pages = [DialoguePage {
        lines: &lines,
        choices: &[],
        choice_targets: &[],
    }];
    let mut d = DialogueBox::open(&pages).unwrap();
    d.tick_dialogue(DialogueInput::Confirm).unwrap();
    let st = style();
    let n = count_dialogue_cells(&st).unwrap();

    // buffer too small reports the exact requirement and writes nothing
    let mut small = vec![TileBatchCell::new(0, 0, 77); n - 1];
    assert_eq!(
        lower_dialogue_cells_into(&d, &st, &mut small),
        Err(GridError::BufferTooSmall {
            required: n,
            actual: n - 1
        })
    );
    assert!(small.iter().all(|c| c.tile_id == 77));
    assert_eq!(
        lower_dialogue_cells_into(&d, &st, &mut []),
        Err(GridError::BufferTooSmall {
            required: n,
            actual: 0
        })
    );

    let mut out = vec![TileBatchCell::new(0, 0, 0); n];
    let narrow = DialogueStyle {
        inner_cols: 4,
        ..st
    };
    assert!(matches!(
        lower_dialogue_cells_into(&d, &narrow, &mut out),
        Err(GridError::InvalidParameter { .. })
    ));
    let zero_cols = DialogueStyle {
        inner_cols: 0,
        ..st
    };
    assert!(count_dialogue_cells(&zero_cols).is_err());
    let zero_rows = DialogueStyle {
        inner_rows: 0,
        ..st
    };
    assert!(count_dialogue_cells(&zero_rows).is_err());
    let zero_glyph = DialogueStyle {
        glyph_base: 0,
        ..st
    };
    assert!(count_dialogue_cells(&zero_glyph).is_err());
    let huge = DialogueStyle {
        inner_cols: 200,
        inner_rows: 200,
        ..st
    };
    assert!(matches!(
        count_dialogue_cells(&huge),
        Err(GridError::TooMany { .. })
    ));
    let off_grid = DialogueStyle {
        col: u16::MAX - 3,
        ..st
    };
    assert_eq!(
        count_dialogue_cells(&off_grid),
        Err(GridError::SizeOverflow)
    );
    let glyph_overflow = DialogueStyle {
        glyph_base: u16::MAX - 5,
        ..st
    };
    assert_eq!(
        lower_dialogue_cells_into(&d, &glyph_overflow, &mut out),
        Err(GridError::SizeOverflow)
    );
    // too many rows for the interior
    let l4 = ["a", "b", "c", "d"];
    let tall = [DialoguePage {
        lines: &l4,
        choices: &[],
        choice_targets: &[],
    }];
    let d2 = DialogueBox::open(&tall).unwrap();
    assert!(lower_dialogue_cells_into(&d2, &st, &mut out).is_err());
}

#[test]
fn dialogue_lowers_to_paint_scene() {
    let lines = ["Hi"];
    let pages = [DialoguePage {
        lines: &lines,
        choices: &[],
        choice_targets: &[],
    }];
    let mut d = DialogueBox::open(&pages).unwrap();
    d.tick_dialogue(DialogueInput::Confirm).unwrap();
    let st = style();
    let scene = lower_dialogue_to_paint_scene(&d, &st, 8, 8, 160, 144, "#000000").unwrap();
    assert_eq!(scene.width, 160.0);
    assert_eq!(scene.height, 144.0);
    assert_eq!(scene.instructions.len(), 14 * 5);
    assert!(scene
        .instructions
        .iter()
        .all(|i| matches!(i, PaintInstruction::Rect(_))));
    // determinism
    let again = lower_dialogue_to_paint_scene(&d, &st, 8, 8, 160, 144, "#000000").unwrap();
    assert_eq!(scene, again);
}

#[test]
fn paint_scene_guards() {
    let lines = ["Hi"];
    let pages = [DialoguePage {
        lines: &lines,
        choices: &[],
        choice_targets: &[],
    }];
    let d = DialogueBox::open(&pages).unwrap();
    let st = style();
    for (tw, th) in [(0, 8), (8, 0), (MAX_TILE_PX + 1, 8), (8, MAX_TILE_PX + 1)] {
        assert!(matches!(
            lower_dialogue_to_paint_scene(&d, &st, tw, th, 160, 144, "#000"),
            Err(GridError::InvalidParameter { .. })
        ));
    }
    // zero-sized view is rejected by the batch lowering
    assert!(matches!(
        lower_dialogue_to_paint_scene(&d, &st, 8, 8, 0, 144, "#000"),
        Err(GridError::Batch(_))
    ));
}

// --- hostile state ---------------------------------------------------------

#[test]
fn extreme_tile_positions_stay_in_bounds() {
    let cells = open_cells(2, 2);
    let l = layer(&cells, 2, 2);
    let world = GridWorld::bind(&l, &[], &[], &[]).unwrap();
    let mut enc = EncounterState::start(1);
    let mut a = GridActor::place(&l, 1, 1, Facing::Right).unwrap();
    // pushing into every edge never leaves the map
    for dir in [Facing::Right, Facing::Down, Facing::Left, Facing::Up] {
        for _ in 0..40 {
            tick(&mut a, &world, Some(dir), &mut enc);
            assert!((0..2).contains(&a.tile_x) && (0..2).contains(&a.tile_y));
        }
    }
}

#[test]
fn pixel_position_uses_wide_math() {
    let cells = open_cells(4096, 1);
    let l = TileLayer::attach(&cells, 4096, 1, 256, 256).unwrap();
    let a = GridActor::place(&l, 4095, 0, Facing::Right).unwrap();
    assert_eq!(a.pixel_position(&l), (4095 * 256, 0));
}

#[test]
fn long_random_walk_never_panics_and_is_deterministic() {
    let cells = grid(12, 12, |c, r| match (c * 7 + r * 13) % 11 {
        0 => TileCell::tile(1).with_attr(ATTR_SOLID),
        1 => TileCell::tile(2).with_attr(ATTR_ONE_WAY),
        2 => TileCell::tile(3).with_attr(ATTR_WATER),
        3 => TileCell::tile(GRASS),
        4 => TileCell::tile(1).with_attr(ATTR_HAZARD),
        _ => TileCell::tile(1),
    });
    let l = layer(&cells, 12, 12);
    let e = entries();
    let tables = [table(&e, 2, 10, 80)];
    let warps = [pad(5, 5, WarpTrigger::OnEnter, 1)];
    let world = GridWorld::bind(&l, &warps, &[], &tables).unwrap();
    let run = || {
        let mut enc = EncounterState::start(99);
        let mut rng = GridRng::seeded(5);
        let mut a = GridActor::place(&l, 6, 6, Facing::Down).unwrap();
        a.can_surf = true;
        a.set_speed(5).unwrap();
        a.set_turn_delay(2).unwrap();
        let mut log = Vec::new();
        for _ in 0..4000 {
            let input = match rng.roll_below(6) {
                0 => Some(Facing::Up),
                1 => Some(Facing::Down),
                2 => Some(Facing::Left),
                3 => Some(Facing::Right),
                _ => None,
            };
            let r = a.advance(&world, input, &mut enc).unwrap();
            assert!(a.tile_x >= 0 && a.tile_x < 12 && a.tile_y >= 0 && a.tile_y < 12);
            log.push((
                a.tile_x,
                a.tile_y,
                r.encounter.map(|x| x.species),
                r.arrived,
            ));
        }
        log
    };
    assert_eq!(run(), run());
}
