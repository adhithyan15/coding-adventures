//! Hostile-input and deterministic fuzz test suite for `game-clock`.

#![forbid(unsafe_code)]

use game_clock::{
    advance_frame, new_clock, validate_state, ClockError, GameClockConfig, GameClockState,
    HARD_MAX_DELTA_NS, HARD_MAX_FRAME_STEPS,
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
}

#[test]
fn hostile_clock_configs_overflows_and_atomic_advance() {
    assert_eq!(
        GameClockConfig::from_hz(0, 8),
        Err(ClockError::ZeroFixedTimestep)
    );
    assert_eq!(
        GameClockConfig::from_hz(u32::MAX, 8),
        Err(ClockError::ZeroFixedTimestep)
    );
    assert_eq!(
        GameClockConfig::from_hz(60, 0),
        Err(ClockError::InvalidMaxFrameSteps)
    );
    assert_eq!(
        GameClockConfig::from_hz(60, HARD_MAX_FRAME_STEPS + 1),
        Err(ClockError::InvalidMaxFrameSteps)
    );
    assert_eq!(
        GameClockConfig::from_hz(60, u32::MAX),
        Err(ClockError::InvalidMaxFrameSteps)
    );
    assert_eq!(
        GameClockConfig::new(0, 8, 100),
        Err(ClockError::ZeroFixedTimestep)
    );
    assert_eq!(
        GameClockConfig::new(HARD_MAX_DELTA_NS + 1, 8, HARD_MAX_DELTA_NS + 1),
        Err(ClockError::InvalidMaxFrameDelta)
    );

    // Atomic preservation on TimestampOverflow
    let mut overflow_clock = new_clock(GameClockConfig::standard_60hz()).unwrap();
    overflow_clock.elapsed_wall_ns = u64::MAX;
    let snapshot = overflow_clock;
    assert_eq!(
        overflow_clock.advance(16_666_667),
        Err(ClockError::TimestampOverflow)
    );
    assert_eq!(overflow_clock, snapshot);

    // Atomic preservation on CorruptedStateInvariant
    let mut corrupted = new_clock(GameClockConfig::standard_60hz()).unwrap();
    corrupted.accumulator_ns = corrupted.fixed_dt_ns; // violates accumulator < fixed_dt_ns
    let corrupted_snap = corrupted;
    assert_eq!(
        corrupted.advance(1_000),
        Err(ClockError::CorruptedStateInvariant)
    );
    assert_eq!(corrupted, corrupted_snap);

    // u64::MAX delta on valid clock must clamp cleanly, not overflow
    let mut valid = new_clock(GameClockConfig::standard_60hz()).unwrap();
    let res = valid.advance(u64::MAX).unwrap();
    assert!(res.steps_to_run <= valid.max_frame_steps);
}

#[test]
fn hostile_fuzz_campaign_2000_cases() {
    let mut rng = SplitMix64(0x6C0C_4100_0000_0001);

    for i in 0..2_000 {
        let hz = match i % 5 {
            0 => 0,
            1 => u32::MAX,
            _ => rng.next_u32(),
        };
        let steps = match i % 6 {
            0 => 0,
            1 => u32::MAX,
            _ => rng.next_u32() % (HARD_MAX_FRAME_STEPS + 16),
        };
        let _ = GameClockConfig::from_hz(hz, steps);

        let cfg = GameClockConfig {
            fixed_dt_ns: if i % 7 == 0 {
                0
            } else {
                rng.next_u64() % (HARD_MAX_DELTA_NS + 100)
            },
            max_frame_steps: steps,
            max_frame_delta_ns: rng.next_u64(),
        };
        let _ = new_clock(cfg);

        let mut state = GameClockState {
            tick_count: if i % 8 == 0 { u64::MAX } else { rng.next_u64() },
            frame_count: if i % 9 == 0 { u64::MAX } else { rng.next_u64() },
            elapsed_sim_ns: if i % 10 == 0 {
                u64::MAX
            } else {
                rng.next_u64()
            },
            elapsed_wall_ns: if i % 11 == 0 {
                u64::MAX
            } else {
                rng.next_u64()
            },
            accumulator_ns: rng.next_u64(),
            fixed_dt_ns: if i % 12 == 0 {
                0
            } else {
                1 + (rng.next_u64() % HARD_MAX_DELTA_NS)
            },
            max_frame_steps: 1 + (rng.next_u32() % HARD_MAX_FRAME_STEPS),
            max_frame_delta_ns: 1 + (rng.next_u64() % HARD_MAX_DELTA_NS),
            total_dropped_ns: rng.next_u64(),
            spiral_clamped_frames: rng.next_u64(),
        };
        let delta = if i % 4 == 0 { u64::MAX } else { rng.next_u64() };
        let _ = validate_state(&state);
        let _ = advance_frame(&state, delta);
        let before = state;
        if state.advance(delta).is_err() {
            assert_eq!(state, before);
        }
    }
}
