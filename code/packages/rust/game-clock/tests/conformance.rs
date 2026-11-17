//! # Conformance and Differential Test Suite for `game-clock` (GAME10)
//!
//! Validates:
//! - Steady 60 Hz and 120 Hz pace (exact 1-step cadence, zero alpha, zero dropped time).
//! - Sub-tick fractional accumulator alpha interpolation (`alpha_milli`, `alpha_q16`, `alpha_f64`).
//! - Exact nanosecond conservation law across a 10,000-frame pseudo-random jitter sequence:
//!   `elapsed_sim_ns + accumulator_ns + total_dropped_ns == sum(raw_deltas)`.
//! - Spiral-of-Death mitigation (CWE-400): $O(1)$ response to massive frame spikes (`u64::MAX`, 100s freeze),
//!   step clamping to `max_frame_steps`, excess backlog draining, and instant frame-to-frame recovery.
//! - Divide-by-zero rejection (CWE-369): zero timestep and zero Hz error handling.
//! - Integer overflow protection (CWE-190): `u64::MAX` saturation across all counters.
//! - Architectural state invariant validation (`validate_state`, `validate_config`).

use game_clock::{
    advance_frame, new_clock, ClockError, GameClockConfig, GameClockState, DEFAULT_120HZ_DT_NS,
    DEFAULT_60HZ_DT_NS, DEFAULT_MAX_FRAME_DELTA_NS, DEFAULT_MAX_FRAME_STEPS, HARD_MAX_DELTA_NS,
    HARD_MAX_FRAME_STEPS,
};

#[test]
fn test_standard_configs() {
    let cfg60 = GameClockConfig::standard_60hz();
    assert_eq!(cfg60.fixed_dt_ns, DEFAULT_60HZ_DT_NS);
    assert_eq!(cfg60.max_frame_steps, DEFAULT_MAX_FRAME_STEPS);
    assert_eq!(cfg60.max_frame_delta_ns, DEFAULT_MAX_FRAME_DELTA_NS);
    assert!(cfg60.validate().is_ok());

    let cfg120 = GameClockConfig::standard_120hz();
    assert_eq!(cfg120.fixed_dt_ns, DEFAULT_120HZ_DT_NS);
    assert_eq!(cfg120.max_frame_steps, DEFAULT_MAX_FRAME_STEPS);
    assert_eq!(cfg120.max_frame_delta_ns, DEFAULT_MAX_FRAME_DELTA_NS);
    assert!(cfg120.validate().is_ok());

    let cfg_custom = GameClockConfig::from_hz(60, 8).expect("valid 60hz config");
    assert_eq!(cfg_custom.fixed_dt_ns, 1_000_000_000 / 60);
    assert_eq!(cfg_custom.max_frame_steps, 8);
    assert!(cfg_custom.validate().is_ok());
}

#[test]
fn test_steady_60hz_pace() {
    let config = GameClockConfig::standard_60hz();
    let mut state = new_clock(config).expect("new_clock should succeed");
    assert_eq!(state.tick_count, 0);
    assert_eq!(state.frame_count, 0);
    assert_eq!(state.elapsed_sim_ns, 0);
    assert_eq!(state.elapsed_wall_ns, 0);
    assert_eq!(state.accumulator_ns, 0);
    assert_eq!(state.total_dropped_ns, 0);
    assert_eq!(state.spiral_clamped_frames, 0);

    for frame in 1..=1000 {
        let res = advance_frame(&state, DEFAULT_60HZ_DT_NS).expect("advance_frame should succeed");
        assert_eq!(res.steps_to_run, 1, "frame {frame} must run exactly 1 step");
        assert_eq!(res.alpha_milli, 0, "frame {frame} alpha_milli must be 0");
        assert_eq!(res.alpha_q16, 0, "frame {frame} alpha_q16 must be 0");
        assert!(
            (res.alpha_f64() - 0.0).abs() < 1e-12,
            "frame {frame} alpha_f64 must be 0.0"
        );
        assert!(
            !res.spiral_of_death_clamped,
            "frame {frame} must not spiral clamp"
        );
        assert_eq!(res.dropped_ns, 0, "frame {frame} must drop 0 ns");

        state = res.state;
        assert_eq!(state.tick_count, frame);
        assert_eq!(state.frame_count, frame);
        assert_eq!(state.elapsed_sim_ns, frame * DEFAULT_60HZ_DT_NS);
        assert_eq!(state.elapsed_wall_ns, frame * DEFAULT_60HZ_DT_NS);
        assert_eq!(state.accumulator_ns, 0);
        assert_eq!(state.total_dropped_ns, 0);
        assert_eq!(state.spiral_clamped_frames, 0);
    }
}

#[test]
fn test_steady_120hz_pace() {
    let config = GameClockConfig::standard_120hz();
    let mut state = new_clock(config).expect("new_clock should succeed");

    for _frame in 1..=500 {
        let res = advance_frame(&state, DEFAULT_120HZ_DT_NS).expect("advance_frame should succeed");
        assert_eq!(res.steps_to_run, 1);
        assert_eq!(res.alpha_milli, 0);
        assert_eq!(res.alpha_q16, 0);
        assert!(!res.spiral_of_death_clamped);
        assert_eq!(res.dropped_ns, 0);
        state = res.state;
    }
    assert_eq!(state.tick_count, 500);
    assert_eq!(state.frame_count, 500);
    assert_eq!(state.elapsed_sim_ns, 500 * DEFAULT_120HZ_DT_NS);
    assert_eq!(state.elapsed_wall_ns, 500 * DEFAULT_120HZ_DT_NS);
    assert_eq!(state.accumulator_ns, 0);
}

#[test]
fn test_sub_tick_rendering_interpolation_alpha() {
    let config = GameClockConfig::standard_60hz();
    let state = new_clock(config).expect("new_clock should succeed");

    // Advance by half a tick: 8_333_333 ns
    let half_tick = 8_333_333;
    let res1 = advance_frame(&state, half_tick).expect("first advance");
    assert_eq!(res1.steps_to_run, 0, "half tick produces 0 simulation steps");
    assert_eq!(res1.state.accumulator_ns, half_tick);
    assert_eq!(res1.state.tick_count, 0);
    assert_eq!(res1.state.elapsed_sim_ns, 0);
    assert_eq!(res1.state.elapsed_wall_ns, half_tick);

    // Alpha checks: 8_333_333 / 16_666_667 ~= 0.49999998
    assert_eq!(res1.alpha_milli, 499);
    assert_eq!(res1.alpha_q16, 32767);
    assert!((res1.alpha_f64() - 0.5).abs() < 1e-6);

    // Advance by remainder of the tick: 16_666_667 - 8_333_333 = 8_333_334 ns
    let rem_tick = DEFAULT_60HZ_DT_NS - half_tick;
    let res2 = advance_frame(&res1.state, rem_tick).expect("second advance");
    assert_eq!(res2.steps_to_run, 1, "accumulated full tick runs 1 step");
    assert_eq!(res2.state.accumulator_ns, 0);
    assert_eq!(res2.state.tick_count, 1);
    assert_eq!(res2.state.elapsed_sim_ns, DEFAULT_60HZ_DT_NS);
    assert_eq!(res2.state.elapsed_wall_ns, DEFAULT_60HZ_DT_NS);
    assert_eq!(res2.alpha_milli, 0);
    assert_eq!(res2.alpha_q16, 0);
    assert_eq!(res2.alpha_f64(), 0.0);
}

#[test]
fn test_10000_frame_random_jitter_exact_conservation_law() {
    let config = GameClockConfig::standard_60hz();
    let mut state = new_clock(config).expect("new_clock should succeed");

    // Simple deterministic 64-bit LCG for reproducible jitter
    let mut rng_state = 0x1234_5678_9ABC_DEF0u64;
    let mut lcg = || {
        rng_state = rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        rng_state
    };

    let mut total_raw_deltas: u128 = 0;

    for _ in 0..10_000 {
        // Delta between 1 ms and 300 ms (covering sub-tick, multi-tick, spiral-clamp, and wall-clamp)
        let raw_delta = 1_000_000 + (lcg() % 299_000_000);
        total_raw_deltas += raw_delta as u128;

        let prev_accum = state.accumulator_ns;
        let res = advance_frame(&state, raw_delta).expect("advance_frame under random jitter");

        // Per-frame invariant assertions
        assert!(res.steps_to_run <= state.max_frame_steps);
        assert!(res.state.accumulator_ns < state.fixed_dt_ns);
        assert!(res.alpha_milli < 1000);
        assert!(res.alpha_q16 < 65536);
        assert!(res.alpha_f64() >= 0.0 && res.alpha_f64() < 1.0);

        // Per-frame exact nanosecond conservation:
        // prev_accum + raw_delta == (steps_to_run * fixed_dt) + dropped_ns + next_accum
        let frame_sim_ns = (res.steps_to_run as u64) * state.fixed_dt_ns;
        let frame_inflow = (prev_accum as u128) + (raw_delta as u128);
        let frame_outflow =
            (frame_sim_ns as u128) + (res.dropped_ns as u128) + (res.state.accumulator_ns as u128);
        assert_eq!(
            frame_inflow, frame_outflow,
            "per-frame conservation law violated"
        );

        state = res.state;
    }

    // Global conservation law across all 10,000 frames:
    // total_raw_deltas == elapsed_sim_ns + accumulator_ns + total_dropped_ns
    let global_outflow = (state.elapsed_sim_ns as u128)
        + (state.accumulator_ns as u128)
        + (state.total_dropped_ns as u128);

    assert_eq!(
        total_raw_deltas, global_outflow,
        "10,000-frame global nanosecond conservation law violated: expected {total_raw_deltas}, got {global_outflow}"
    );

    // Verify tick count matches elapsed simulation time exactly
    assert_eq!(state.elapsed_sim_ns, state.tick_count * state.fixed_dt_ns);
    assert_eq!(state.frame_count, 10_000);
    assert!(
        state.spiral_clamped_frames > 0,
        "some frames should have triggered spiral clamp"
    );
    assert!(
        state.total_dropped_ns > 0,
        "some nanoseconds should have been dropped"
    );
}

#[test]
fn test_cwe_400_spiral_of_death_massive_freeze() {
    let config = GameClockConfig::standard_60hz();
    let state = new_clock(config).expect("new_clock");

    // Simulate a massive 100-second freeze (e.g. window dragging or debugger breakpoint)
    let freeze_delta_ns = 100_000_000_000u64; // 100s
    let res = advance_frame(&state, freeze_delta_ns).expect("advance_frame after 100s freeze");

    // 1. Clamps to max_frame_steps (8 steps = ~133 ms, not 6000 steps!)
    assert_eq!(res.steps_to_run, DEFAULT_MAX_FRAME_STEPS);
    assert!(res.spiral_of_death_clamped);

    // 2. Excess backlog is drained: accumulator is < fixed_dt_ns
    assert!(res.state.accumulator_ns < DEFAULT_60HZ_DT_NS);

    // 3. Dropped nanoseconds account for both delta clamp and excess spiral steps:
    // raw 100s capped to 250ms -> 99.75s delta dropped
    // 250ms has 15 steps: 8 run, 7 dropped -> 7 * 16_666_667 = 116_666_669 dropped
    let expected_delta_dropped = freeze_delta_ns - DEFAULT_MAX_FRAME_DELTA_NS;
    let excess_steps = (DEFAULT_MAX_FRAME_DELTA_NS / DEFAULT_60HZ_DT_NS) - (DEFAULT_MAX_FRAME_STEPS as u64);
    let expected_spiral_dropped = excess_steps * DEFAULT_60HZ_DT_NS;
    let expected_total_dropped = expected_delta_dropped + expected_spiral_dropped;
    assert_eq!(res.dropped_ns, expected_total_dropped);
    assert_eq!(res.state.total_dropped_ns, expected_total_dropped);
    assert_eq!(res.state.spiral_clamped_frames, 1);

    // 4. On the very next frame with normal 60 Hz delta, clock recovers immediately!
    let next_res = advance_frame(&res.state, DEFAULT_60HZ_DT_NS).expect("next frame advance");
    assert_eq!(
        next_res.steps_to_run, 1,
        "clock recovered immediately to 1 step"
    );
    assert!(!next_res.spiral_of_death_clamped);
    assert_eq!(next_res.dropped_ns, 0);
}

#[test]
fn test_cwe_400_u64_max_frame_delta() {
    let config = GameClockConfig::standard_60hz();
    let state = new_clock(config).expect("new_clock");

    // Extreme adversarial delta: u64::MAX
    let res = advance_frame(&state, u64::MAX).expect("advance_frame with u64::MAX");
    assert_eq!(res.steps_to_run, DEFAULT_MAX_FRAME_STEPS);
    assert!(res.spiral_of_death_clamped);
    assert!(res.state.accumulator_ns < DEFAULT_60HZ_DT_NS);
    assert_eq!(res.state.frame_count, 1);
    assert_eq!(res.state.tick_count, DEFAULT_MAX_FRAME_STEPS as u64);
}

#[test]
fn test_cwe_369_zero_timestep() {
    // Zero fixed_dt_ns rejected in config constructors
    assert_eq!(
        GameClockConfig::new(0, 8, 250_000_000),
        Err(ClockError::ZeroFixedTimestep)
    );
    assert_eq!(
        GameClockConfig::from_hz(0, 8),
        Err(ClockError::ZeroFixedTimestep)
    );

    // Ultra-high Hz producing 0 fixed_dt_ns (1_000_000_000 / 2_000_000_000 = 0)
    assert_eq!(
        GameClockConfig::from_hz(2_000_000_000, 8),
        Err(ClockError::ZeroFixedTimestep)
    );

    // Zero fixed_dt_ns in state rejected by validate_state
    let bad_state = GameClockState {
        tick_count: 0,
        frame_count: 0,
        elapsed_sim_ns: 0,
        elapsed_wall_ns: 0,
        accumulator_ns: 0,
        fixed_dt_ns: 0,
        max_frame_steps: 8,
        max_frame_delta_ns: 250_000_000,
        total_dropped_ns: 0,
        spiral_clamped_frames: 0,
    };
    assert_eq!(
        advance_frame(&bad_state, 16_666_667),
        Err(ClockError::ZeroFixedTimestep)
    );
}

#[test]
fn test_cwe_190_integer_overflow() {
    let mut state = new_clock(GameClockConfig::standard_60hz()).expect("new_clock");

    // 1. tick_count overflow
    state.tick_count = u64::MAX;
    state.elapsed_sim_ns = u64::MAX.wrapping_mul(state.fixed_dt_ns); // trigger
    assert!(matches!(
        advance_frame(&state, DEFAULT_60HZ_DT_NS),
        Err(ClockError::TimestampOverflow | ClockError::CorruptedStateInvariant)
    ));

    // 2. frame_count overflow
    let mut state2 = new_clock(GameClockConfig::standard_60hz()).expect("new_clock");
    state2.frame_count = u64::MAX;
    assert_eq!(
        advance_frame(&state2, DEFAULT_60HZ_DT_NS),
        Err(ClockError::TimestampOverflow)
    );

    // 3. elapsed_sim_ns overflow
    let mut state3 = new_clock(GameClockConfig::standard_60hz()).expect("new_clock");
    state3.tick_count = u64::MAX / state3.fixed_dt_ns;
    state3.elapsed_sim_ns = state3.tick_count * state3.fixed_dt_ns;
    // Advancing 8 steps would push tick_count * fixed_dt_ns over u64::MAX
    let max_delta = DEFAULT_MAX_FRAME_DELTA_NS;
    assert_eq!(
        advance_frame(&state3, max_delta),
        Err(ClockError::TimestampOverflow)
    );

    // 4. total_dropped_ns overflow
    let mut state4 = new_clock(GameClockConfig::standard_60hz()).expect("new_clock");
    state4.total_dropped_ns = u64::MAX - 10;
    // Drop 100 ns -> overflow
    let spike_delta = state4.max_frame_delta_ns + 100;
    assert_eq!(
        advance_frame(&state4, spike_delta),
        Err(ClockError::TimestampOverflow)
    );
}

#[test]
fn test_state_invariant_corruption_rejection() {
    let valid_state = new_clock(GameClockConfig::standard_60hz()).expect("valid");

    // accumulator_ns >= fixed_dt_ns
    let mut bad_accum = valid_state;
    bad_accum.accumulator_ns = valid_state.fixed_dt_ns;
    assert_eq!(
        advance_frame(&bad_accum, 1000),
        Err(ClockError::CorruptedStateInvariant)
    );

    // max_frame_steps == 0
    let mut bad_steps = valid_state;
    bad_steps.max_frame_steps = 0;
    assert_eq!(
        advance_frame(&bad_steps, 1000),
        Err(ClockError::CorruptedStateInvariant)
    );

    // max_frame_steps > HARD_MAX_FRAME_STEPS
    let mut bad_steps_high = valid_state;
    bad_steps_high.max_frame_steps = HARD_MAX_FRAME_STEPS + 1;
    assert_eq!(
        advance_frame(&bad_steps_high, 1000),
        Err(ClockError::CorruptedStateInvariant)
    );

    // max_frame_delta_ns < fixed_dt_ns
    let mut bad_delta = valid_state;
    bad_delta.max_frame_delta_ns = valid_state.fixed_dt_ns - 1;
    assert_eq!(
        advance_frame(&bad_delta, 1000),
        Err(ClockError::CorruptedStateInvariant)
    );

    // max_frame_delta_ns > HARD_MAX_DELTA_NS
    let mut bad_delta_high = valid_state;
    bad_delta_high.max_frame_delta_ns = HARD_MAX_DELTA_NS + 1;
    assert_eq!(
        advance_frame(&bad_delta_high, 1000),
        Err(ClockError::CorruptedStateInvariant)
    );

    // elapsed_sim_ns != tick_count * fixed_dt_ns
    let mut bad_sim_time = valid_state;
    bad_sim_time.elapsed_sim_ns = 42; // tick_count is 0, so should be 0
    assert_eq!(
        advance_frame(&bad_sim_time, 1000),
        Err(ClockError::CorruptedStateInvariant)
    );
}

#[test]
fn test_config_validation() {
    assert_eq!(
        GameClockConfig::new(DEFAULT_60HZ_DT_NS, 0, DEFAULT_MAX_FRAME_DELTA_NS),
        Err(ClockError::InvalidMaxFrameSteps)
    );
    assert_eq!(
        GameClockConfig::new(
            DEFAULT_60HZ_DT_NS,
            HARD_MAX_FRAME_STEPS + 1,
            DEFAULT_MAX_FRAME_DELTA_NS
        ),
        Err(ClockError::InvalidMaxFrameSteps)
    );
    assert_eq!(
        GameClockConfig::new(DEFAULT_60HZ_DT_NS, 8, DEFAULT_60HZ_DT_NS - 1),
        Err(ClockError::InvalidMaxFrameDelta)
    );
    assert_eq!(
        GameClockConfig::new(DEFAULT_60HZ_DT_NS, 8, HARD_MAX_DELTA_NS + 1),
        Err(ClockError::InvalidMaxFrameDelta)
    );
}

#[test]
fn test_state_advance_method_in_place() {
    let mut state = new_clock(GameClockConfig::standard_60hz()).expect("new_clock");
    let res = state.advance(DEFAULT_60HZ_DT_NS).expect("advance in place");
    assert_eq!(res.steps_to_run, 1);
    assert_eq!(state.tick_count, 1);
    assert_eq!(state.frame_count, 1);
    assert_eq!(state.elapsed_sim_ns, DEFAULT_60HZ_DT_NS);
}

#[test]
fn test_clock_error_display() {
    let errors = [
        ClockError::ZeroFixedTimestep,
        ClockError::InvalidMaxFrameSteps,
        ClockError::InvalidMaxFrameDelta,
        ClockError::CorruptedStateInvariant,
        ClockError::TimestampOverflow,
    ];
    for err in &errors {
        let msg = format!("{err}");
        assert!(!msg.is_empty(), "Error display string must not be empty");
    }
}
