//! # Conformance and Security Verification Test Suite for `game-breakout` (GAME14)
//!
//! Validates:
//! - Complete deterministic Breakout state machine, lives counter, level progression, and scoring.
//! - Continuous swept circle-AABB collision detection against paddle and bricks (anti-tunneling).
//! - Multi-hit durable brick damage and destruction mechanics.
//! - Saturating score and combo multiplier arithmetic (CWE-190).
//! - Boundary wall reflections and bottom gutter life loss.
//! - Ball attachment and deterministic launch angle physics.
//! - Fixed-timestep clock loop integration via `game-clock`.
//! - Retained-mode sprite batch emission and `PaintScene` lowering via `paint-batch-tiles`.
//! - Low-latency audio mixing and streaming into `PcmRingBuffer` via `arcade-sfx`.
//! - CWE-20: Rejection of non-finite floats (NaN/Inf) in configurations and state.
//! - CWE-190: Integer and coordinate bounds protection.
//! - CWE-369: Zero-dimension and zero-division protection.

use arcade_sfx::SfxBank;
use audio_mixer_realtime::PcmRingBuffer;
use game_breakout::{
    build_breakout_sprites, default_config, deterministic_launch_angle, deterministic_prng_next,
    lower_breakout_to_paint_scene, new_game, render_breakout_audio_into, step_breakout,
    step_breakout_with_clock, validate_config, validate_state, BreakoutError, BreakoutInput, Point,
    SpriteItem, DEFAULT_ARENA_HEIGHT, DEFAULT_ARENA_WIDTH, MAX_BRICK_COLS, MAX_COORD,
};
use game_clock::{new_clock, GameClockConfig, DEFAULT_60HZ_DT_NS};

#[test]
fn test_cwe_20_and_369_config_validation() {
    let mut cfg = default_config();
    assert!(validate_config(&cfg).is_ok());

    // Non-finite floats rejected (CWE-20)
    cfg.arena_width = f64::NAN;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::NonFiniteState));
    cfg.arena_width = f64::INFINITY;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::NonFiniteState));
    cfg.arena_width = DEFAULT_ARENA_WIDTH;

    cfg.paddle_speed = f64::NAN;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::NonFiniteState));
    cfg.paddle_speed = 480.0;

    // Zero or negative dimensions rejected (CWE-369)
    cfg.arena_width = 0.0;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.arena_width = -100.0;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.arena_width = DEFAULT_ARENA_WIDTH;

    cfg.arena_height = 0.0;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.arena_height = DEFAULT_ARENA_HEIGHT;

    cfg.ball_radius = 0.0;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.ball_radius = 8.0;

    // Coordinate overflow boundary (CWE-190)
    cfg.arena_width = MAX_COORD + 1.0;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.arena_width = DEFAULT_ARENA_WIDTH;

    // Initial lives zero rejected
    cfg.initial_lives = 0;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.initial_lives = 3;

    // Brick row/column count bounds (CWE-400)
    cfg.brick_rows = 0;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.brick_rows = 9;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.brick_rows = 8;

    cfg.brick_cols = 0;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.brick_cols = 15;
    assert_eq!(validate_config(&cfg), Err(BreakoutError::InvalidConfig));
    cfg.brick_cols = 14;
    assert!(validate_config(&cfg).is_ok());
}

#[test]
fn test_state_initialization_and_validation() {
    let cfg = default_config();
    let state = new_game(cfg, 0x1234_5678).expect("valid match state");
    assert!(validate_state(&state).is_ok());

    assert_eq!(state.score, 0);
    assert_eq!(state.lives, 3);
    assert_eq!(state.level, 1);
    assert_eq!(state.combo_counter, 0);
    assert_eq!(state.active_bricks, cfg.brick_rows * cfg.brick_cols);
    assert!(state.ball_attached);
    assert!(!state.is_breakout_game_over());

    // Ball should be positioned atop the paddle
    let paddle_rect = state.paddle_bounding_rect();
    assert_eq!(state.ball_pos.x, state.paddle_x + cfg.paddle_width * 0.5);
    assert!((state.ball_pos.y - (paddle_rect.y - cfg.ball_radius)).abs() < 1e-6);

    // Corrupted non-finite state rejected (CWE-20)
    let mut bad_state = state;
    bad_state.ball_pos.x = f64::NAN;
    assert_eq!(
        validate_state(&bad_state),
        Err(BreakoutError::NonFiniteState)
    );

    bad_state = state;
    bad_state.paddle_x = f64::INFINITY;
    assert_eq!(
        validate_state(&bad_state),
        Err(BreakoutError::NonFiniteState)
    );
}

#[test]
fn test_paddle_movement_and_clamping() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");

    let initial_x = state.paddle_x;

    // Move left
    let input_left = BreakoutInput::move_paddle_left(false);
    step_breakout(&mut state, &input_left, &mut sfx).expect("step");
    assert!(state.paddle_x < initial_x);

    // Keep moving left until clamp at 0.0
    for _ in 0..100 {
        step_breakout(&mut state, &input_left, &mut sfx).expect("step");
    }
    assert_eq!(state.paddle_x, 0.0);

    // Move right until clamp at arena_width - paddle_width
    let input_right = BreakoutInput::move_paddle_right(false);
    let max_x = cfg.arena_width - cfg.paddle_width;
    for _ in 0..200 {
        step_breakout(&mut state, &input_right, &mut sfx).expect("step");
    }
    assert_eq!(state.paddle_x, max_x);
}

#[test]
fn test_ball_launch_and_flight() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");

    assert!(state.ball_attached);

    // Paddle move should move attached ball too
    let input_right = BreakoutInput::move_paddle_right(false);
    step_breakout(&mut state, &input_right, &mut sfx).expect("step");
    assert_eq!(state.ball_pos.x, state.paddle_x + cfg.paddle_width * 0.5);

    // Launch ball
    let input_launch = BreakoutInput::launch_only();
    step_breakout(&mut state, &input_launch, &mut sfx).expect("step");
    assert!(!state.ball_attached);
    assert!(state.ball_vel.y < 0.0); // Moving upward towards bricks

    // Move next step - ball should move independently of paddle
    let ball_y_before = state.ball_pos.y;
    let input_idle = BreakoutInput::idle_breakout_input();
    step_breakout(&mut state, &input_idle, &mut sfx).expect("step");
    assert!(state.ball_pos.y < ball_y_before);
}

#[test]
fn test_continuous_swept_paddle_deflection() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");

    // Launch ball first
    state.ball_attached = false;
    let paddle_rect = state.paddle_bounding_rect();

    // Position ball directly above paddle moving downward at high speed
    state.ball_pos = Point::new(
        state.paddle_x + cfg.paddle_width * 0.5,
        paddle_rect.y - 15.0,
    );
    state.ball_vel = Point::new(0.0, 500.0);
    state.combo_counter = 5;

    let input = BreakoutInput::idle_breakout_input();
    let events = step_breakout(&mut state, &input, &mut sfx).expect("step");

    assert!(events.paddle_hit);
    assert_eq!(state.combo_counter, 0); // Reset combo on paddle bounce
    assert!(state.ball_vel.y < 0.0); // Bounced upward
}

#[test]
fn test_continuous_swept_brick_collision_and_durability() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");

    // Deactivate row 1 col 0 so row 0 col 0 is exposed from below
    let row1_idx = MAX_BRICK_COLS;
    state.bricks[row1_idx].hits_remaining = 0;
    state.active_bricks -= 1;

    // Row 0 has 2 hits durability
    let brick_idx = 0;
    assert_eq!(state.bricks[brick_idx].hits_remaining, 2);
    let initial_active = state.active_bricks;

    // Position ball below row 0 col 0 brick moving upward within 1 tick of contact
    let b_rect = game_breakout::brick_bounding_rect(&cfg, 0, 0);
    state.ball_attached = false;
    state.ball_pos = Point::new(
        b_rect.x + b_rect.width * 0.5,
        b_rect.y + b_rect.height + cfg.ball_radius + 3.0,
    );
    state.ball_vel = Point::new(0.0, -400.0);

    let input = BreakoutInput::idle_breakout_input();

    // First hit: brick damaged (hits remaining becomes 1)
    let events1 = step_breakout(&mut state, &input, &mut sfx).expect("step");
    assert_eq!(events1.brick_hit_row, Some(0));
    assert_eq!(state.bricks[brick_idx].hits_remaining, 1);
    assert_eq!(state.active_bricks, initial_active); // Still active!
    assert!(state.score > 0);
    assert_eq!(state.combo_counter, 1);
    assert!(state.ball_vel.y > 0.0); // Reflected downward

    // Turn ball back upward to hit same brick again
    state.ball_vel = Point::new(0.0, -400.0);
    state.ball_pos = Point::new(
        b_rect.x + b_rect.width * 0.5,
        b_rect.y + b_rect.height + cfg.ball_radius + 3.0,
    );

    // Second hit: brick destroyed (hits remaining becomes 0)
    let events2 = step_breakout(&mut state, &input, &mut sfx).expect("step");
    assert_eq!(events2.brick_hit_row, Some(0));
    assert_eq!(state.bricks[brick_idx].hits_remaining, 0);
    assert_eq!(state.active_bricks, initial_active - 1); // Now destroyed!
    assert_eq!(state.combo_counter, 2); // Combo incremented
}

#[test]
fn test_combo_counter_and_saturating_score() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");

    // Artificially test saturating score with high combo
    state.score = u32::MAX - 10;
    state.combo_counter = 100;
    state.ball_attached = false;

    // Hit a 1-hit brick in row 6
    let b_rect = game_breakout::brick_bounding_rect(&cfg, 6, 0);
    state.ball_pos = Point::new(
        b_rect.x + b_rect.width * 0.5,
        b_rect.y + b_rect.height + 8.0,
    );
    state.ball_vel = Point::new(0.0, -400.0);

    let input = BreakoutInput::idle_breakout_input();
    let events = step_breakout(&mut state, &input, &mut sfx).expect("step");

    assert_eq!(events.brick_hit_row, Some(6));
    assert_eq!(state.score, u32::MAX); // Saturating addition without panic (CWE-190)
}

#[test]
fn test_wall_bounces() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");
    state.ball_attached = false;

    // 1. Left wall bounce
    state.ball_pos = Point::new(5.0, 300.0);
    state.ball_vel = Point::new(-300.0, 0.0);
    let ev_left =
        step_breakout(&mut state, &BreakoutInput::idle_breakout_input(), &mut sfx).expect("step");
    assert!(ev_left.wall_bounce);
    assert!(state.ball_vel.x > 0.0);

    // 2. Right wall bounce
    state.ball_pos = Point::new(cfg.arena_width - 5.0, 300.0);
    state.ball_vel = Point::new(300.0, 0.0);
    let ev_right =
        step_breakout(&mut state, &BreakoutInput::idle_breakout_input(), &mut sfx).expect("step");
    assert!(ev_right.wall_bounce);
    assert!(state.ball_vel.x < 0.0);

    // 3. Top wall bounce
    state.ball_pos = Point::new(400.0, 5.0);
    state.ball_vel = Point::new(0.0, -300.0);
    let ev_top =
        step_breakout(&mut state, &BreakoutInput::idle_breakout_input(), &mut sfx).expect("step");
    assert!(ev_top.wall_bounce);
    assert!(state.ball_vel.y > 0.0);
}

#[test]
fn test_gutter_loss_and_game_over() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");
    state.ball_attached = false;

    assert_eq!(state.lives, 3);

    // Move ball past bottom gutter
    state.ball_pos = Point::new(400.0, cfg.arena_height - 2.0);
    state.ball_vel = Point::new(0.0, 400.0);

    let ev1 =
        step_breakout(&mut state, &BreakoutInput::idle_breakout_input(), &mut sfx).expect("step");
    assert!(ev1.life_lost);
    assert_eq!(state.lives, 2);
    assert!(state.ball_attached); // Reset to paddle
    assert!(!state.is_breakout_game_over());

    // Lose second life
    state.ball_attached = false;
    state.ball_pos = Point::new(400.0, cfg.arena_height - 2.0);
    state.ball_vel = Point::new(0.0, 400.0);
    step_breakout(&mut state, &BreakoutInput::idle_breakout_input(), &mut sfx).expect("step");
    assert_eq!(state.lives, 1);

    // Lose third and final life -> Game Over
    state.ball_attached = false;
    state.ball_pos = Point::new(400.0, cfg.arena_height - 2.0);
    state.ball_vel = Point::new(0.0, 400.0);
    let ev_final =
        step_breakout(&mut state, &BreakoutInput::idle_breakout_input(), &mut sfx).expect("step");
    assert!(ev_final.life_lost);
    assert!(ev_final.game_over);
    assert_eq!(state.lives, 0);
    assert!(state.is_breakout_game_over());
}

#[test]
fn test_level_clear_progression() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");

    // Deactivate all bricks except 1 in row 7 col 0
    let last_brick_idx = 7 * MAX_BRICK_COLS;
    for (idx, brick) in state.bricks.iter_mut().enumerate() {
        if idx != last_brick_idx {
            brick.hits_remaining = 0;
        }
    }
    state.active_bricks = 1;
    state.bricks[last_brick_idx].hits_remaining = 1;

    let b_rect = game_breakout::brick_bounding_rect(&cfg, 7, 0);
    state.ball_attached = false;
    state.ball_pos = Point::new(
        b_rect.x + b_rect.width * 0.5,
        b_rect.y + b_rect.height + 8.0,
    );
    state.ball_vel = Point::new(0.0, -400.0);

    let events =
        step_breakout(&mut state, &BreakoutInput::idle_breakout_input(), &mut sfx).expect("step");
    assert!(events.level_cleared);
    assert_eq!(state.level, 2);
    assert_eq!(state.active_bricks, cfg.brick_rows * cfg.brick_cols); // Respawed!
    assert!(state.ball_attached);
}

#[test]
fn test_step_with_clock() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");
    let clock_cfg = GameClockConfig::standard_60hz();
    let mut clock = new_clock(clock_cfg).expect("clock");

    let input = BreakoutInput::move_paddle_right(false);
    let res =
        step_breakout_with_clock(&mut clock, DEFAULT_60HZ_DT_NS, &mut state, &input, &mut sfx)
            .expect("clock step");

    assert_eq!(res.steps_simulated, 1);
}

#[test]
fn test_sprite_emission_and_paint_scene() {
    let cfg = default_config();
    let state = new_game(cfg, 0x1234_5678).expect("valid game");

    let mut sprites = [SpriteItem::new(0, 0.0, 0.0, 0.0, 0.0); 256];
    let count = build_breakout_sprites(&state, &mut sprites);
    assert!(count > 0);
    assert!(count <= 256);

    let scene = lower_breakout_to_paint_scene(&state).expect("paint scene");
    assert_eq!(scene.width, cfg.arena_width);
    assert_eq!(scene.height, cfg.arena_height);
}

#[test]
fn test_audio_rendering() {
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx bank");
    let mut ring = PcmRingBuffer::<256>::empty();
    let mut scratch = [0i16; 64];

    let written =
        render_breakout_audio_into(&mut sfx, &mut ring, &mut scratch).expect("audio render");
    assert!(written > 0);
}

#[test]
fn test_deterministic_prng() {
    let mut seed = 0xbeef_cafe;
    let r1 = deterministic_prng_next(&mut seed);
    let r2 = deterministic_prng_next(&mut seed);
    assert_ne!(r1, r2);

    let angle = deterministic_launch_angle(&mut seed, 0.8);
    assert!((-0.8..=0.8).contains(&angle));
}
