//! Hostile-input and deterministic fuzz test suite for `game-breakout`.

#![forbid(unsafe_code)]

use game_breakout::{
    default_config, lower_breakout_to_paint_scene, new_game, step_breakout, validate_config,
    validate_state, BreakoutConfig, BreakoutError, BreakoutInput, PaddleMove, Point, SfxBank,
    MAX_BRICKS,
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

    fn next_hostile_f64(&mut self, idx: usize) -> f64 {
        match idx % 9 {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => -0.0,
            4 => f64::from_bits(1),
            5 => f64::MIN,
            6 => f64::MAX,
            7 => 1.0e9,
            _ => ((self.next_u64() as i64) as f64) / 1.0e15,
        }
    }
}

#[test]
fn hostile_configs_and_lowering_rejections() {
    let hostile_floats = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN,
        f64::MAX,
    ];

    for &v in &hostile_floats {
        let mut cfg = default_config();
        cfg.arena_width = v;
        let _ = validate_config(&cfg);
        let _ = new_game(cfg, 1);

        let mut cfg2 = default_config();
        cfg2.fixed_dt_seconds = v;
        let _ = validate_config(&cfg2);

        let mut cfg3 = default_config();
        cfg3.brick_width = v;
        let _ = validate_config(&cfg3);
    }

    let base = new_game(default_config(), 77).unwrap();
    for &bad in &[f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0e15, -1.0e15] {
        let mut s = base;
        s.ball_pos = Point::new(bad, 200.0);
        assert!(matches!(
            lower_breakout_to_paint_scene(&s),
            Err(BreakoutError::NonFiniteState | BreakoutError::StateOutOfBounds)
        ));

        let mut s2 = base;
        s2.paddle_x = bad;
        assert!(lower_breakout_to_paint_scene(&s2).is_err());

        let mut s3 = base;
        s3.config.arena_height = bad;
        assert!(lower_breakout_to_paint_scene(&s3).is_err());
    }

    let mut bad_bricks = base;
    bad_bricks.active_bricks = MAX_BRICKS + 1;
    assert!(matches!(
        lower_breakout_to_paint_scene(&bad_bricks),
        Err(BreakoutError::StateOutOfBounds)
    ));

    let mut sub = base;
    sub.paddle_x = f64::from_bits(1);
    sub.ball_vel = Point::new(-0.0, f64::from_bits(1));
    assert!(validate_state(&sub).is_ok());
    assert!(lower_breakout_to_paint_scene(&sub).is_ok());
}

#[test]
fn hostile_fuzz_campaign_1000_cases() {
    let mut rng = SplitMix64(0xB4EA_0000_0000_0001);
    let mut state = new_game(default_config(), 98765).unwrap();
    let mut sfx = SfxBank::<4>::empty(44_100).unwrap();

    for i in 0..1_000 {
        let mut cfg = BreakoutConfig::standard_breakout();
        cfg.arena_width = rng.next_hostile_f64(i);
        cfg.arena_height = rng.next_hostile_f64(i + 1);
        cfg.paddle_speed = rng.next_hostile_f64(i + 2);
        cfg.ball_radius = rng.next_hostile_f64(i + 3);
        cfg.fixed_dt_seconds = rng.next_hostile_f64(i + 4);
        cfg.brick_rows = (rng.next_u64() as usize) % 16;
        cfg.brick_cols = (rng.next_u64() as usize) % 24;
        let _ = validate_config(&cfg);
        let _ = new_game(cfg, rng.next_u64());

        let mut mutated = state;
        mutated.paddle_x = rng.next_hostile_f64(i + 5);
        mutated.ball_pos = Point::new(rng.next_hostile_f64(i + 6), rng.next_hostile_f64(i + 7));
        mutated.ball_vel = Point::new(rng.next_hostile_f64(i + 8), rng.next_hostile_f64(i + 2));
        if i % 15 == 0 {
            mutated.active_bricks = usize::MAX;
        }
        let _ = validate_state(&mutated);
        let _ = lower_breakout_to_paint_scene(&mutated);

        let input = BreakoutInput {
            movement: match i % 3 {
                0 => PaddleMove::Left,
                1 => PaddleMove::Right,
                _ => PaddleMove::Hold,
            },
            launch: i % 5 == 0,
        };
        let _ = step_breakout(&mut mutated, &input, &mut sfx);
        let _ = step_breakout(&mut state, &input, &mut sfx);
        if state.game_over {
            state = new_game(default_config(), rng.next_u64() | 1).unwrap();
        }
    }
}
