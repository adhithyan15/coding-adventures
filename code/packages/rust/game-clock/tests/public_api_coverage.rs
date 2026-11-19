//! Focused public-API edges that are easy to miss in the broader clock tests.

use game_clock::{
    ClockError, ClockStepResult, GameClockConfig, GameClockState, DEFAULT_MAX_FRAME_DELTA_NS,
    HARD_MAX_FRAME_STEPS,
};

#[test]
fn config_convenience_constructors_cover_valid_and_refusal_edges() {
    assert_eq!(GameClockConfig::default(), GameClockConfig::standard_60hz());
    assert_eq!(
        GameClockConfig::from_hz(0, 8),
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
        GameClockConfig::from_hz(u32::MAX, 8),
        Err(ClockError::ZeroFixedTimestep)
    );

    // At 3 Hz the timestep exceeds the default frame-delta limit, so the
    // constructor raises the per-frame cap to one timestep.
    let slow = GameClockConfig::from_hz(3, 8).expect("3 Hz is supported");
    assert_eq!(slow.fixed_dt_ns, 1_000_000_000 / 3);
    assert_eq!(slow.max_frame_delta_ns, slow.fixed_dt_ns);
    let ordinary = GameClockConfig::from_hz(60, 8).expect("60 Hz is supported");
    assert_eq!(ordinary.max_frame_delta_ns, DEFAULT_MAX_FRAME_DELTA_NS);
    assert_eq!(
        GameClockConfig::new(
            ordinary.fixed_dt_ns,
            ordinary.max_frame_steps,
            ordinary.max_frame_delta_ns
        ),
        Ok(ordinary)
    );

    let state = GameClockState::new(ordinary).expect("valid public constructor");
    assert_eq!(state.validate(), Ok(()));
}

#[test]
fn public_interpolation_handles_zero_and_nonzero_timestep() {
    let mut result = ClockStepResult {
        state: GameClockState {
            tick_count: 0,
            frame_count: 0,
            elapsed_sim_ns: 0,
            elapsed_wall_ns: 0,
            accumulator_ns: 0,
            fixed_dt_ns: 0,
            max_frame_steps: 1,
            max_frame_delta_ns: 1,
            total_dropped_ns: 0,
            spiral_clamped_frames: 0,
        },
        steps_to_run: 0,
        alpha_milli: 0,
        alpha_q16: 0,
        spiral_of_death_clamped: false,
        dropped_ns: 0,
    };
    assert_eq!(result.alpha_f64(), 0.0);

    result.state.fixed_dt_ns = 4;
    result.state.accumulator_ns = 1;
    assert_eq!(result.alpha_f64(), 0.25);
}
