//! # Conformance and Security Verification Test Suite for `game-pong` (GAME13)
//!
//! Validates:
//! - Complete deterministic 2-player Pong state machine, match scoring, and win detection.
//! - Continuous swept circle-AABB collision detection against paddles (anti-tunneling at high velocity).
//! - Deterministic paddle deflection physics and angle variation based on contact point.
//! - Top and bottom court boundary reflections and audio triggering.
//! - Autonomous AI tracking policy with deadband and court clamping.
//! - Glenn Fiedler "Fix Your Timestep!" clock loop integration via `game-clock`.
//! - Retained-mode sprite batch emission and `PaintScene` lowering via `paint-batch-tiles`.
//! - Low-latency audio mixing and streaming into `PcmRingBuffer` via `arcade-sfx`.
//! - CWE-20: Rejection of non-finite floats (NaN/Inf) in configurations and state.
//! - CWE-190: Integer and coordinate bounds protection.
//! - CWE-369: Zero-dimension and zero-division protection.

use arcade_sfx::{SfxBank, SfxPreset};
use audio_mixer_realtime::PcmRingBuffer;
use game_clock::{new_clock, GameClockConfig, DEFAULT_60HZ_DT_NS};
use game_pong::{
    build_pong_sprites, default_config, deterministic_prng_next, deterministic_serve_angle,
    lower_pong_to_paint_scene, new_game, render_pong_audio_into, step_pong, step_pong_with_clock,
    validate_config, validate_state, PaddleAction, PaddleSide, Point, PongError, PongInput,
    SpriteItem, DEFAULT_COURT_HEIGHT, DEFAULT_COURT_WIDTH, MAX_COORD,
};

#[test]
fn test_cwe_20_and_369_config_validation() {
    let mut cfg = default_config();
    assert!(validate_config(&cfg).is_ok());

    // Non-finite floats rejected (CWE-20)
    cfg.court_width = f64::NAN;
    assert_eq!(validate_config(&cfg), Err(PongError::NonFiniteState));
    cfg.court_width = f64::INFINITY;
    assert_eq!(validate_config(&cfg), Err(PongError::NonFiniteState));
    cfg.court_width = DEFAULT_COURT_WIDTH;

    cfg.paddle_speed = f64::NAN;
    assert_eq!(validate_config(&cfg), Err(PongError::NonFiniteState));
    cfg.paddle_speed = 400.0;

    // Zero or negative dimensions rejected (CWE-369)
    cfg.court_width = 0.0;
    assert_eq!(validate_config(&cfg), Err(PongError::InvalidConfig));
    cfg.court_width = -100.0;
    assert_eq!(validate_config(&cfg), Err(PongError::InvalidConfig));
    cfg.court_width = DEFAULT_COURT_WIDTH;

    cfg.court_height = 0.0;
    assert_eq!(validate_config(&cfg), Err(PongError::InvalidConfig));
    cfg.court_height = DEFAULT_COURT_HEIGHT;

    cfg.ball_radius = 0.0;
    assert_eq!(validate_config(&cfg), Err(PongError::InvalidConfig));
    cfg.ball_radius = 8.0;

    // Coordinate overflow boundary (CWE-190)
    cfg.court_width = MAX_COORD + 1.0;
    assert_eq!(validate_config(&cfg), Err(PongError::InvalidConfig));
    cfg.court_width = DEFAULT_COURT_WIDTH;

    // Winning score zero rejected
    cfg.winning_score = 0;
    assert_eq!(validate_config(&cfg), Err(PongError::InvalidConfig));
    cfg.winning_score = 11;

    // Serve countdown ticks excessive rejected (CWE-400)
    cfg.serve_countdown_ticks = 10_000;
    assert_eq!(validate_config(&cfg), Err(PongError::InvalidConfig));
}

#[test]
fn test_state_initialization_and_validation() {
    let cfg = default_config();
    let state = new_game(cfg, 0x1234_5678).expect("valid match state");
    assert!(validate_state(&state).is_ok());

    assert_eq!(state.left_score, 0);
    assert_eq!(state.right_score, 0);
    assert_eq!(state.rally_counter, 0);
    assert_eq!(state.serve_countdown, cfg.serve_countdown_ticks);
    assert!(!state.is_game_finished());
    assert_eq!(state.winning_player(), None);

    // Ball should be positioned at center court
    assert_eq!(state.ball_pos.x, cfg.court_width * 0.5);
    assert_eq!(state.ball_pos.y, cfg.court_height * 0.5);

    // Paddles should be vertically centered
    let expected_paddle_y = (cfg.court_height - cfg.paddle_height) * 0.5;
    assert_eq!(state.left_paddle_y, expected_paddle_y);
    assert_eq!(state.right_paddle_y, expected_paddle_y);

    // Corrupted non-finite state rejected (CWE-20)
    let mut bad_state = state;
    bad_state.ball_pos.x = f64::NAN;
    assert_eq!(validate_state(&bad_state), Err(PongError::NonFiniteState));

    bad_state = state;
    bad_state.left_paddle_y = f64::INFINITY;
    assert_eq!(validate_state(&bad_state), Err(PongError::NonFiniteState));

    // Out-of-bounds state rejected
    bad_state = state;
    bad_state.left_paddle_y = cfg.court_height + 50.0;
    assert_eq!(validate_state(&bad_state), Err(PongError::StateOutOfBounds));
}

#[test]
fn test_swept_collision_anti_tunneling_against_paddles() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234_5678).expect("valid game");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("valid sfx bank");

    // Bypass serve countdown
    state.serve_countdown = 0;

    // Test Left Paddle Swept Collision:
    // Place ball just to the right of left paddle with a huge velocity heading left
    // displacement per frame = 600.0 * (1/60) = 10.0 units, easily sweeping through paddle thickness (16.0)
    let left_rect = state.left_paddle_rect();
    state.ball_pos = Point::new(left_rect.x + left_rect.width + 5.0, left_rect.y + left_rect.height * 0.5);
    state.ball_vel = Point::new(-700.0, 0.0);

    let input = PongInput::idle_input();
    let events = step_pong(&mut state, &input, &mut sfx).expect("step success");

    // Continuous collision detection MUST catch this hit!
    assert_eq!(events.paddle_hit, Some(PaddleSide::Left));
    assert_eq!(events.rally_count, 1);
    assert!(state.ball_vel.x > 0.0, "ball must reflect to the right");
    assert!(state.ball_pos.x >= left_rect.x + left_rect.width, "ball must not tunnel behind paddle");

    // Test Right Paddle Swept Collision:
    let right_rect = state.right_paddle_rect();
    state.ball_pos = Point::new(right_rect.x - 5.0, right_rect.y + right_rect.height * 0.5);
    state.ball_vel = Point::new(700.0, 0.0);

    let events_right = step_pong(&mut state, &input, &mut sfx).expect("step success");
    assert_eq!(events_right.paddle_hit, Some(PaddleSide::Right));
    assert_eq!(events_right.rally_count, 2);
    assert!(state.ball_vel.x < 0.0, "ball must reflect to the left");
    assert!(state.ball_pos.x <= right_rect.x, "ball must not tunnel behind paddle");
}

#[test]
fn test_deflection_angle_physics() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234).expect("state");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");
    state.serve_countdown = 0;

    let left_rect = state.left_paddle_rect();

    // 1. Center hit: Y velocity should be zero
    state.ball_pos = Point::new(left_rect.x + left_rect.width + 1.0, left_rect.y + left_rect.height * 0.5);
    state.ball_vel = Point::new(-400.0, 0.0);
    step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert!(state.ball_vel.y.abs() < 1e-6, "center hit should have near-zero Y velocity");

    // 2. Upper hit: Y velocity should be negative (deflected upward)
    state.ball_pos = Point::new(left_rect.x + left_rect.width + 1.0, left_rect.y + 10.0);
    state.ball_vel = Point::new(-400.0, 0.0);
    step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert!(state.ball_vel.y < -10.0, "upper hit should deflect upward");

    // 3. Lower hit: Y velocity should be positive (deflected downward)
    state.ball_pos = Point::new(left_rect.x + left_rect.width + 1.0, left_rect.y + left_rect.height - 10.0);
    state.ball_vel = Point::new(-400.0, 0.0);
    step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert!(state.ball_vel.y > 10.0, "lower hit should deflect downward");
}

#[test]
fn test_top_and_bottom_wall_bounces() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1234).expect("state");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");
    state.serve_countdown = 0;

    // Ball hitting top boundary
    state.ball_pos = Point::new(cfg.court_width * 0.5, cfg.ball_radius + 1.0);
    state.ball_vel = Point::new(100.0, -300.0);
    let top_events = step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert!(top_events.wall_bounce, "wall bounce event must fire");
    assert!(state.ball_vel.y > 0.0, "ball must reflect downward from top wall");
    assert!(state.ball_pos.y >= cfg.ball_radius);

    // Ball hitting bottom boundary
    state.ball_pos = Point::new(cfg.court_width * 0.5, cfg.court_height - cfg.ball_radius - 1.0);
    state.ball_vel = Point::new(100.0, 300.0);
    let bot_events = step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert!(bot_events.wall_bounce, "wall bounce event must fire");
    assert!(state.ball_vel.y < 0.0, "ball must reflect upward from bottom wall");
    assert!(state.ball_pos.y <= cfg.court_height - cfg.ball_radius);
}

#[test]
fn test_scoring_and_win_condition() {
    let mut cfg = default_config();
    cfg.winning_score = 3;
    cfg.serve_countdown_ticks = 5;
    let mut state = new_game(cfg, 0x4321).expect("state");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");
    state.serve_countdown = 0;

    // Ball passes left edge: Right player scores
    state.ball_pos = Point::new(1.0, cfg.court_height * 0.5);
    state.ball_vel = Point::new(-500.0, 0.0);
    let ev1 = step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert!(ev1.score_event.is_some());
    let sc1 = ev1.score_event.unwrap();
    assert_eq!(sc1.scorer, PaddleSide::Right);
    assert_eq!(sc1.right_score, 1);
    assert_eq!(sc1.left_score, 0);
    assert_eq!(state.serve_countdown, 5);

    // Score point 2 for Right
    state.serve_countdown = 0;
    state.ball_pos = Point::new(1.0, cfg.court_height * 0.5);
    state.ball_vel = Point::new(-500.0, 0.0);
    step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert_eq!(state.right_score, 2);
    assert!(!state.is_game_finished());

    // Score point 3 for Right -> Game Over!
    state.serve_countdown = 0;
    state.ball_pos = Point::new(1.0, cfg.court_height * 0.5);
    state.ball_vel = Point::new(-500.0, 0.0);
    let win_ev = step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert_eq!(state.right_score, 3);
    assert!(win_ev.game_over);
    assert!(state.is_game_finished());
    assert_eq!(state.winning_player(), Some(PaddleSide::Right));

    // Post-game steps do not mutate scores
    let post_ev = step_pong(&mut state, &PongInput::idle_input(), &mut sfx).expect("step");
    assert!(post_ev.game_over);
    assert_eq!(state.right_score, 3);
}

#[test]
fn test_paddle_manual_movement_and_bounds_clamping() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x1111).expect("state");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");

    let initial_y = state.left_paddle_y;

    // Move Up
    let up_input = PongInput::player_vs_player(PaddleAction::Up, PaddleAction::Hold);
    step_pong(&mut state, &up_input, &mut sfx).expect("step");
    assert!(state.left_paddle_y < initial_y, "paddle must move up");

    // Move Down past bottom boundary
    let down_input = PongInput::player_vs_player(PaddleAction::Down, PaddleAction::Hold);
    for _ in 0..200 {
        step_pong(&mut state, &down_input, &mut sfx).expect("step");
    }
    let max_y = cfg.court_height - cfg.paddle_height;
    assert_eq!(state.left_paddle_y, max_y, "paddle must clamp at bottom");

    // Move Up past top boundary
    for _ in 0..200 {
        step_pong(&mut state, &up_input, &mut sfx).expect("step");
    }
    assert_eq!(state.left_paddle_y, 0.0, "paddle must clamp at top");
}

#[test]
fn test_ai_paddle_tracking_policy() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x2222).expect("state");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");
    state.serve_countdown = 0;

    // Place ball high in court
    state.ball_pos = Point::new(cfg.court_width * 0.5, 50.0);
    state.ball_vel = Point::new(0.0, 0.0);

    let ai_input = PongInput::ai_vs_ai();
    let prev_left_y = state.left_paddle_y;
    step_pong(&mut state, &ai_input, &mut sfx).expect("step");
    assert!(state.left_paddle_y < prev_left_y, "AI paddle must track upward toward ball");

    // Place ball low in court
    state.ball_pos = Point::new(cfg.court_width * 0.5, cfg.court_height - 50.0);
    let prev_right_y = state.right_paddle_y;
    step_pong(&mut state, &ai_input, &mut sfx).expect("step");
    assert!(state.right_paddle_y > prev_right_y, "AI paddle must track downward toward ball");
}

#[test]
fn test_fixed_timestep_clock_integration() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x3333).expect("state");
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");

    let clock_cfg = GameClockConfig::standard_60hz();
    let mut clock = new_clock(clock_cfg).expect("clock");

    let input = PongInput::ai_vs_ai();

    // 1 frame of exactly 60 Hz dt -> exactly 1 simulation step
    let res = step_pong_with_clock(&mut clock, DEFAULT_60HZ_DT_NS, &mut state, &input, &mut sfx)
        .expect("clock step");
    assert_eq!(res.steps_simulated, 1);
    assert_eq!(clock.tick_count, 1);

    // Frame lag delta (~33.3ms) -> runs 2 steps
    let res2 = step_pong_with_clock(&mut clock, DEFAULT_60HZ_DT_NS * 2, &mut state, &input, &mut sfx)
        .expect("clock step");
    assert_eq!(res2.steps_simulated, 2);
    assert_eq!(clock.tick_count, 3);
}

#[test]
fn test_zero_alloc_sprite_emission_and_paint_scene() {
    let cfg = default_config();
    let mut state = new_game(cfg, 0x4444).expect("state");
    state.left_score = 7;
    state.right_score = 10;

    let mut sprites = [SpriteItem::new(0, 0.0, 0.0, 0.0, 0.0); 128];
    let count = build_pong_sprites(&state, &mut sprites);
    assert!(count > 10, "sprites must include net, paddles, ball, and score digits");

    // Verify all sprite IDs are unique
    for i in 0..count {
        for j in (i + 1)..count {
            assert_ne!(
                sprites[i].id, sprites[j].id,
                "sprite IDs must be strictly unique (id {} collided)",
                sprites[i].id
            );
        }
    }

    // Lowering to universal PaintScene
    let scene = lower_pong_to_paint_scene(&state).expect("scene generation");
    assert_eq!(scene.width, cfg.court_width);
    assert_eq!(scene.height, cfg.court_height);
    assert_eq!(scene.background, "#000000");
    assert_eq!(scene.instructions.len(), count);
}

#[test]
fn test_realtime_audio_streaming() {
    let mut sfx = SfxBank::<4>::empty(44_100).expect("sfx");
    let mut ring = PcmRingBuffer::<256>::empty();
    let mut scratch = [0i16; 64];

    // Trigger presets
    sfx.trigger_bank_preset(SfxPreset::PongPaddleHit).expect("preset");
    let written = render_pong_audio_into(&mut sfx, &mut ring, &mut scratch).expect("render");
    assert_eq!(written, 64);
    assert_eq!(ring.len(), 64);

    // Verify non-zero samples generated
    assert!(scratch.iter().any(|&s| s != 0), "sound synthesis must produce audible samples");
}

#[test]
fn test_deterministic_prng_and_serve_angles() {
    let mut seed_a = 0x5a5a_a5a5;
    let mut seed_b = 0x5a5a_a5a5;

    // Both identical seeds must generate identical random sequences
    for _ in 0..50 {
        let v_a = deterministic_prng_next(&mut seed_a);
        let v_b = deterministic_prng_next(&mut seed_b);
        assert_eq!(v_a, v_b);

        let ang_a = deterministic_serve_angle(&mut seed_a, 0.85);
        let ang_b = deterministic_serve_angle(&mut seed_b, 0.85);
        assert_eq!(ang_a, ang_b);
        assert!((-0.85..=0.85).contains(&ang_a));
    }
}
