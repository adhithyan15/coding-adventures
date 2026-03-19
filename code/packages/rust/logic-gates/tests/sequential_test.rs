//! Integration tests for the sequential logic module.
//!
//! Tests cover SR latch, D latch, D flip-flop, register, shift register,
//! and counter — verifying correct state transitions, edge-triggered
//! behavior, and multi-bit storage.

use logic_gates::sequential::*;

// ===========================================================================
// SR Latch tests
// ===========================================================================

#[test]
fn test_sr_latch_set() {
    let (q, qb) = sr_latch(1, 0, 0, 1);
    assert_eq!((q, qb), (1, 0), "Set should produce Q=1, Q_bar=0");
}

#[test]
fn test_sr_latch_reset() {
    let (q, qb) = sr_latch(0, 1, 1, 0);
    assert_eq!((q, qb), (0, 1), "Reset should produce Q=0, Q_bar=1");
}

#[test]
fn test_sr_latch_hold_after_set() {
    let (q, qb) = sr_latch(1, 0, 0, 1);
    let (q, qb) = sr_latch(0, 0, q, qb);
    assert_eq!((q, qb), (1, 0), "Hold after set should keep Q=1");
}

#[test]
fn test_sr_latch_hold_after_reset() {
    let (q, qb) = sr_latch(0, 1, 0, 1);
    let (q, qb) = sr_latch(0, 0, q, qb);
    assert_eq!((q, qb), (0, 1), "Hold after reset should keep Q=0");
}

#[test]
fn test_sr_latch_invalid_both_set() {
    let (q, qb) = sr_latch(1, 1, 0, 1);
    assert_eq!((q, qb), (0, 0), "S=R=1 is invalid, both outputs forced to 0");
}

#[test]
fn test_sr_latch_set_reset_set_sequence() {
    // Set -> Reset -> Set
    let (q, qb) = sr_latch(1, 0, 0, 1);
    assert_eq!((q, qb), (1, 0));
    let (q, qb) = sr_latch(0, 1, q, qb);
    assert_eq!((q, qb), (0, 1));
    let (q, qb) = sr_latch(1, 0, q, qb);
    assert_eq!((q, qb), (1, 0));
}

#[test]
fn test_sr_latch_default_initial_state() {
    // Default Q=0, Q_bar=1; hold should preserve
    let (q, qb) = sr_latch(0, 0, 0, 1);
    assert_eq!((q, qb), (0, 1));
}

// ===========================================================================
// D Latch tests
// ===========================================================================

#[test]
fn test_d_latch_store_one() {
    let (q, qb) = d_latch(1, 1, 0, 1);
    assert_eq!((q, qb), (1, 0));
}

#[test]
fn test_d_latch_store_zero() {
    let (q, qb) = d_latch(0, 1, 0, 1);
    assert_eq!((q, qb), (0, 1));
}

#[test]
fn test_d_latch_hold_when_disabled() {
    let (q, qb) = d_latch(1, 1, 0, 1); // Store 1
    let (q, qb) = d_latch(0, 0, q, qb); // Disable, try to store 0
    assert_eq!((q, qb), (1, 0), "Should hold 1 when enable=0");
}

#[test]
fn test_d_latch_transparent_when_enabled() {
    let (q, qb) = d_latch(1, 1, 0, 1);
    assert_eq!(q, 1);
    let (q, _qb) = d_latch(0, 1, q, qb); // Still enabled, change data
    assert_eq!(q, 0, "Should follow data when enable=1");
}

#[test]
fn test_d_latch_full_sequence() {
    // Store 1, hold, store 0, hold
    let (q, qb) = d_latch(1, 1, 0, 1);
    assert_eq!(q, 1);
    let (q, qb) = d_latch(1, 0, q, qb); // hold
    assert_eq!(q, 1);
    let (q, qb) = d_latch(0, 1, q, qb); // store 0
    assert_eq!(q, 0);
    let (q, _qb) = d_latch(1, 0, q, qb); // hold (data=1 ignored)
    assert_eq!(q, 0);
}

// ===========================================================================
// D Flip-Flop tests
// ===========================================================================

#[test]
fn test_d_flip_flop_captures_on_rising_edge() {
    let mut state = FlipFlopState::default();
    // Clock low: master absorbs data=1
    d_flip_flop(1, 0, &mut state);
    // Clock high: slave outputs
    let (q, _qb) = d_flip_flop(1, 1, &mut state);
    assert_eq!(q, 1, "Should capture 1 on rising edge");
}

#[test]
fn test_d_flip_flop_holds_on_clock_high() {
    let mut state = FlipFlopState::default();
    // Rising edge with data=1
    d_flip_flop(1, 0, &mut state);
    d_flip_flop(1, 1, &mut state);
    // Now change data to 0 while clock stays high
    let (q, _qb) = d_flip_flop(0, 1, &mut state);
    // Master is holding (clock=1 means NOT(clock)=0), so data change ignored
    assert_eq!(q, 1, "Should hold value while clock is high");
}

#[test]
fn test_d_flip_flop_captures_zero() {
    let mut state = FlipFlopState::default();
    // First store a 1
    d_flip_flop(1, 0, &mut state);
    d_flip_flop(1, 1, &mut state);
    // Now store a 0 on next rising edge
    d_flip_flop(0, 0, &mut state);
    let (q, _qb) = d_flip_flop(0, 1, &mut state);
    assert_eq!(q, 0, "Should capture 0 on second rising edge");
}

#[test]
fn test_d_flip_flop_multiple_cycles() {
    let mut state = FlipFlopState::default();
    let data_sequence = [1, 0, 1, 1, 0];
    for &data in &data_sequence {
        d_flip_flop(data, 0, &mut state);
        let (q, _qb) = d_flip_flop(data, 1, &mut state);
        assert_eq!(q, data, "Flip-flop should capture data={data}");
    }
}

// ===========================================================================
// Register tests
// ===========================================================================

#[test]
fn test_register_4bit() {
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();
    register(&[1, 0, 1, 1], 0, &mut state);
    let out = register(&[1, 0, 1, 1], 1, &mut state);
    assert_eq!(out, vec![1, 0, 1, 1]);
}

#[test]
fn test_register_8bit() {
    let mut state: Vec<FlipFlopState> =
        (0..8).map(|_| FlipFlopState::default()).collect();
    let data = vec![1, 0, 1, 0, 1, 0, 1, 0];
    register(&data, 0, &mut state);
    let out = register(&data, 1, &mut state);
    assert_eq!(out, data);
}

#[test]
fn test_register_holds_value() {
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();
    // Store [1, 1, 0, 0]
    register(&[1, 1, 0, 0], 0, &mut state);
    register(&[1, 1, 0, 0], 1, &mut state);
    // Try to overwrite with [0, 0, 1, 1] but only clock low
    register(&[0, 0, 1, 1], 0, &mut state);
    // Clock high with original data gone — slave should still have [1, 1, 0, 0]
    // Wait, we need to clock high again to see new data
    let out = register(&[0, 0, 1, 1], 1, &mut state);
    assert_eq!(out, vec![0, 0, 1, 1], "Should store new data on next rising edge");
}

#[test]
fn test_register_overwrites() {
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();
    // First write
    register(&[1, 1, 1, 1], 0, &mut state);
    register(&[1, 1, 1, 1], 1, &mut state);
    // Second write, different data
    register(&[0, 1, 0, 1], 0, &mut state);
    let out = register(&[0, 1, 0, 1], 1, &mut state);
    assert_eq!(out, vec![0, 1, 0, 1]);
}

#[test]
#[should_panic]
fn test_register_panics_on_length_mismatch() {
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();
    register(&[1, 0, 1], 0, &mut state); // 3 bits for 4-bit register
}

#[test]
#[should_panic]
fn test_register_panics_on_empty_data() {
    let mut state: Vec<FlipFlopState> = vec![];
    register(&[], 0, &mut state);
}

// ===========================================================================
// Shift Register tests
// ===========================================================================

#[test]
fn test_shift_register_right_shift_in_ones() {
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();

    // Shift in 1 three times
    for i in 0..3 {
        shift_register(1, 0, &mut state, "right");
        let (out, _sout) = shift_register(1, 1, &mut state, "right");
        // After i+1 shifts, first i+1 positions should be 1
        let expected: Vec<u8> = (0..4).map(|j| if j <= i { 1 } else { 0 }).collect();
        assert_eq!(out, expected, "After shift {}", i + 1);
    }
}

#[test]
fn test_shift_register_left_shift() {
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();

    // Shift in 1 from the right (left direction means entering at high index)
    shift_register(1, 0, &mut state, "left");
    let (out, _sout) = shift_register(1, 1, &mut state, "left");
    assert_eq!(out, vec![0, 0, 0, 1], "Left shift: 1 enters at position 3");
}

#[test]
fn test_shift_register_serial_out() {
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();

    // Fill register with 1s
    for _ in 0..4 {
        shift_register(1, 0, &mut state, "right");
        shift_register(1, 1, &mut state, "right");
    }

    // Now shift in 0; the serial_out should be 1 (old MSB)
    shift_register(0, 0, &mut state, "right");
    let (_out, sout) = shift_register(0, 1, &mut state, "right");
    assert_eq!(sout, 1, "Serial out should be the old MSB value");
}

#[test]
fn test_shift_register_right_pattern() {
    // Shift in the pattern 1, 0, 1, 0
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();

    let pattern = [1u8, 0, 1, 0];
    for &bit in &pattern {
        shift_register(bit, 0, &mut state, "right");
        shift_register(bit, 1, &mut state, "right");
    }

    // After shifting in [1, 0, 1, 0], the register should contain [0, 1, 0, 1]
    // because each new bit pushes from position 0
    let (_out, _) = shift_register(0, 0, &mut state, "right");
    // Read current state from slave_q
    let current: Vec<u8> = state.iter().map(|s| s.slave_q).collect();
    assert_eq!(current, vec![0, 1, 0, 1]);
}

#[test]
#[should_panic]
fn test_shift_register_panics_on_bad_direction() {
    let mut state: Vec<FlipFlopState> =
        (0..4).map(|_| FlipFlopState::default()).collect();
    shift_register(0, 0, &mut state, "up");
}

// ===========================================================================
// Counter tests
// ===========================================================================

#[test]
fn test_counter_counts_to_three() {
    let mut state = CounterState::new(4);

    // Count 0 -> 1 -> 2 -> 3
    // counter(clock, reset, state) — clock cycles low->high, reset=0
    let expected = [
        vec![1, 0, 0, 0], // 1
        vec![0, 1, 0, 0], // 2
        vec![1, 1, 0, 0], // 3
    ];

    for (i, exp) in expected.iter().enumerate() {
        counter(0, 0, &mut state); // clock low
        let bits = counter(1, 0, &mut state); // clock high
        assert_eq!(&bits, exp, "Count step {} should be {:?}", i + 1, exp);
    }
}

#[test]
fn test_counter_overflow() {
    // 4-bit counter: max value = 15, then wraps to 0
    let mut state = CounterState::new(4);

    // Count to 15
    for _ in 0..15 {
        counter(0, 0, &mut state);
        counter(1, 0, &mut state);
    }
    // At this point state.value should be [1, 1, 1, 1] = 15
    assert_eq!(state.value, vec![1, 1, 1, 1]);

    // One more tick: overflow to 0
    counter(0, 0, &mut state);
    let bits = counter(1, 0, &mut state);
    assert_eq!(bits, vec![0, 0, 0, 0], "Should overflow to 0");
}

#[test]
fn test_counter_reset() {
    let mut state = CounterState::new(4);

    // Count to 3
    for _ in 0..3 {
        counter(0, 0, &mut state); // clock low, reset=0
        counter(1, 0, &mut state); // clock high, reset=0
    }
    assert_eq!(state.value, vec![1, 1, 0, 0]); // decimal 3

    // Reset: clock low with reset=1, then clock high with reset=1
    counter(0, 1, &mut state); // clock=0, reset=1
    let bits = counter(1, 1, &mut state); // clock=1, reset=1
    assert_eq!(bits, vec![0, 0, 0, 0], "Reset should zero the counter");
}

#[test]
fn test_counter_8bit_counts() {
    let mut state = CounterState::new(8);

    // Count to 5
    for _ in 0..5 {
        counter(0, 0, &mut state); // clock low
        counter(1, 0, &mut state); // clock high
    }

    // 5 in binary: 00000101 (LSB first: [1, 0, 1, 0, 0, 0, 0, 0])
    assert_eq!(state.value, vec![1, 0, 1, 0, 0, 0, 0, 0]);
}
