# frozen_string_literal: true

require_relative "test_helper"

# ============================================================================
# Tests for Sequential Logic -- latches, flip-flops, registers, and counters.
#
# These tests verify that each sequential component:
#   1. Behaves according to its truth table
#   2. Properly stores and recalls state
#   3. Validates inputs correctly
#   4. Is built from the gates in gates.rb (no Ruby boolean shortcuts)
#
# We simulate multiple clock cycles by threading state from one call to the
# next, just like hardware would operate over time.
# ============================================================================

module CodingAdventures
  SEQ = LogicGates::Sequential
end

# =====================================================================
# SR Latch Tests
# =====================================================================

class TestSRLatch < Minitest::Test
  # --- Truth table: Set operation ---
  # S=1, R=0 -> Q=1, Q_bar=0

  def test_set
    result = CodingAdventures::SEQ.sr_latch(set_input: 1, reset: 0)
    assert_equal 1, result[:q], "Set should make Q=1"
    assert_equal 0, result[:q_bar], "Set should make Q_bar=0"
  end

  # --- Truth table: Reset operation ---
  # S=0, R=1 -> Q=0, Q_bar=1

  def test_reset
    result = CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 1)
    assert_equal 0, result[:q], "Reset should make Q=0"
    assert_equal 1, result[:q_bar], "Reset should make Q_bar=1"
  end

  # --- Truth table: Hold state ---
  # S=0, R=0 -> Q=previous Q, Q_bar=previous Q_bar

  def test_hold_after_set
    # First set the latch
    state = CodingAdventures::SEQ.sr_latch(set_input: 1, reset: 0)
    # Then hold
    result = CodingAdventures::SEQ.sr_latch(
      set_input: 0, reset: 0,
      q: state[:q], q_bar: state[:q_bar]
    )
    assert_equal 1, result[:q], "Hold after set should keep Q=1"
    assert_equal 0, result[:q_bar], "Hold after set should keep Q_bar=0"
  end

  def test_hold_after_reset
    # First reset the latch
    state = CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 1)
    # Then hold
    result = CodingAdventures::SEQ.sr_latch(
      set_input: 0, reset: 0,
      q: state[:q], q_bar: state[:q_bar]
    )
    assert_equal 0, result[:q], "Hold after reset should keep Q=0"
    assert_equal 1, result[:q_bar], "Hold after reset should keep Q_bar=1"
  end

  # --- Forbidden state: S=1, R=1 ---
  # Both NOR gates output 0. The circuit produces Q=0, Q_bar=0.

  def test_forbidden_state
    result = CodingAdventures::SEQ.sr_latch(set_input: 1, reset: 1)
    assert_equal 0, result[:q], "Forbidden state: Q should be 0"
    assert_equal 0, result[:q_bar], "Forbidden state: Q_bar should be 0"
  end

  # --- Default initial state ---

  def test_default_initial_state
    # Default: q=0, q_bar=1 (reset state). Holding should preserve it.
    result = CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 0)
    assert_equal 0, result[:q]
    assert_equal 1, result[:q_bar]
  end

  # --- State sequence: set, hold, reset, hold ---

  def test_set_hold_reset_hold_sequence
    # Set
    s = CodingAdventures::SEQ.sr_latch(set_input: 1, reset: 0)
    assert_equal 1, s[:q]

    # Hold
    s = CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 0, q: s[:q], q_bar: s[:q_bar])
    assert_equal 1, s[:q]

    # Reset
    s = CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 1, q: s[:q], q_bar: s[:q_bar])
    assert_equal 0, s[:q]

    # Hold
    s = CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 0, q: s[:q], q_bar: s[:q_bar])
    assert_equal 0, s[:q]
  end

  # --- Input validation ---

  def test_validates_set_input
    assert_raises(TypeError) { CodingAdventures::SEQ.sr_latch(set_input: "1", reset: 0) }
    assert_raises(ArgumentError) { CodingAdventures::SEQ.sr_latch(set_input: 2, reset: 0) }
  end

  def test_validates_reset
    assert_raises(TypeError) { CodingAdventures::SEQ.sr_latch(set_input: 0, reset: true) }
    assert_raises(ArgumentError) { CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 3) }
  end

  def test_validates_q
    assert_raises(TypeError) { CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 0, q: nil) }
  end

  def test_validates_q_bar
    assert_raises(ArgumentError) { CodingAdventures::SEQ.sr_latch(set_input: 0, reset: 0, q_bar: 5) }
  end
end

# =====================================================================
# D Latch Tests
# =====================================================================

class TestDLatch < Minitest::Test
  # --- Transparent mode (Enable=1): output follows data ---

  def test_transparent_data_1
    result = CodingAdventures::SEQ.d_latch(data: 1, enable: 1)
    assert_equal 1, result[:q], "D=1, E=1 should set Q=1"
    assert_equal 0, result[:q_bar]
  end

  def test_transparent_data_0
    result = CodingAdventures::SEQ.d_latch(data: 0, enable: 1)
    assert_equal 0, result[:q], "D=0, E=1 should set Q=0"
    assert_equal 1, result[:q_bar]
  end

  # --- Latch mode (Enable=0): output holds previous value ---

  def test_latch_holds_after_set
    # First, set Q=1 while enabled
    s = CodingAdventures::SEQ.d_latch(data: 1, enable: 1)
    assert_equal 1, s[:q]

    # Now disable -- Q should hold at 1 even though data changes
    result = CodingAdventures::SEQ.d_latch(
      data: 0, enable: 0,
      q: s[:q], q_bar: s[:q_bar]
    )
    assert_equal 1, result[:q], "Latch should hold Q=1 when disabled"
  end

  def test_latch_holds_after_reset
    # First, set Q=0 while enabled
    s = CodingAdventures::SEQ.d_latch(data: 0, enable: 1)
    assert_equal 0, s[:q]

    # Now disable -- Q should hold at 0 even though data changes
    result = CodingAdventures::SEQ.d_latch(
      data: 1, enable: 0,
      q: s[:q], q_bar: s[:q_bar]
    )
    assert_equal 0, result[:q], "Latch should hold Q=0 when disabled"
  end

  # --- Enable transitions ---

  def test_data_captured_on_enable_high
    # Start with Q=0
    s = CodingAdventures::SEQ.d_latch(data: 0, enable: 1)
    assert_equal 0, s[:q]

    # Switch to D=1 while enabled
    s = CodingAdventures::SEQ.d_latch(data: 1, enable: 1, q: s[:q], q_bar: s[:q_bar])
    assert_equal 1, s[:q], "Data should be captured while enabled"
  end

  # --- Default initial state ---

  def test_default_initial_state_hold
    # Default: q=0, q_bar=1. With enable=0, should hold.
    result = CodingAdventures::SEQ.d_latch(data: 1, enable: 0)
    assert_equal 0, result[:q]
    assert_equal 1, result[:q_bar]
  end

  # --- Input validation ---

  def test_validates_data
    assert_raises(TypeError) { CodingAdventures::SEQ.d_latch(data: "1", enable: 1) }
    assert_raises(ArgumentError) { CodingAdventures::SEQ.d_latch(data: 2, enable: 1) }
  end

  def test_validates_enable
    assert_raises(TypeError) { CodingAdventures::SEQ.d_latch(data: 0, enable: false) }
  end
end

# =====================================================================
# D Flip-Flop Tests
# =====================================================================

class TestDFlipFlop < Minitest::Test
  # --- Basic operation: capture on rising edge ---

  def test_captures_data_on_rising_edge
    # Phase 1: clock=0, data=1 -> master captures, output unchanged
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 0)
    assert_equal 0, state[:q], "Output should not change on clock low"

    # Phase 2: clock=1 -> slave captures master's value -> Q=1
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 1, state: state)
    assert_equal 1, state[:q], "Output should capture data on rising edge"
  end

  def test_captures_zero_on_rising_edge
    # Set up: first store a 1
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 0)
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 1, state: state)
    assert_equal 1, state[:q]

    # Now store a 0
    state = CodingAdventures::SEQ.d_flip_flop(data: 0, clock: 0, state: state)
    assert_equal 1, state[:q], "Output should hold during clock low"
    state = CodingAdventures::SEQ.d_flip_flop(data: 0, clock: 1, state: state)
    assert_equal 0, state[:q], "Output should capture 0 on rising edge"
  end

  # --- State persistence ---

  def test_holds_state_when_clock_stays_high
    # Capture a 1
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 0)
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 1, state: state)
    assert_equal 1, state[:q]

    # Clock stays high, data changes -- output should not change
    # (master is disabled when clock is high, slave holds)
    state = CodingAdventures::SEQ.d_flip_flop(data: 0, clock: 1, state: state)
    assert_equal 1, state[:q], "Output should hold when clock stays high"
  end

  # --- Default initial state ---

  def test_default_initial_state
    state = CodingAdventures::SEQ.d_flip_flop(data: 0, clock: 0)
    assert_equal 0, state[:q]
    assert_equal 1, state[:q_bar]
    assert_includes state, :master_q
    assert_includes state, :master_q_bar
  end

  # --- Multi-cycle sequence ---

  def test_multi_cycle_sequence
    # Cycle 1: store 1
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 0)
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 1, state: state)
    assert_equal 1, state[:q]

    # Cycle 2: store 0
    state = CodingAdventures::SEQ.d_flip_flop(data: 0, clock: 0, state: state)
    state = CodingAdventures::SEQ.d_flip_flop(data: 0, clock: 1, state: state)
    assert_equal 0, state[:q]

    # Cycle 3: store 1 again
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 0, state: state)
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 1, state: state)
    assert_equal 1, state[:q]
  end

  # --- Q and Q_bar are complements ---

  def test_q_and_q_bar_are_complements
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 0)
    state = CodingAdventures::SEQ.d_flip_flop(data: 1, clock: 1, state: state)
    assert_equal 1, state[:q]
    assert_equal 0, state[:q_bar]

    state = CodingAdventures::SEQ.d_flip_flop(data: 0, clock: 0, state: state)
    state = CodingAdventures::SEQ.d_flip_flop(data: 0, clock: 1, state: state)
    assert_equal 0, state[:q]
    assert_equal 1, state[:q_bar]
  end

  # --- Input validation ---

  def test_validates_data
    assert_raises(TypeError) { CodingAdventures::SEQ.d_flip_flop(data: "1", clock: 0) }
    assert_raises(ArgumentError) { CodingAdventures::SEQ.d_flip_flop(data: 2, clock: 0) }
  end

  def test_validates_clock
    assert_raises(TypeError) { CodingAdventures::SEQ.d_flip_flop(data: 0, clock: nil) }
  end
end

# =====================================================================
# Register Tests
# =====================================================================

class TestRegister < Minitest::Test
  # --- Basic load and read ---

  def test_stores_4bit_value
    # Clock low: master captures
    state = CodingAdventures::SEQ.register(data: [1, 0, 1, 1], clock: 0)
    assert_equal [0, 0, 0, 0], state[:bits], "Output should not change on clock low"

    # Clock high: slave captures -> output updated
    state = CodingAdventures::SEQ.register(data: [1, 0, 1, 1], clock: 1, state: state)
    assert_equal [1, 0, 1, 1], state[:bits], "Register should store the data"
  end

  # --- Hold state ---

  def test_holds_state_across_cycles
    # Store [1, 1, 0, 0]
    state = CodingAdventures::SEQ.register(data: [1, 1, 0, 0], clock: 0)
    state = CodingAdventures::SEQ.register(data: [1, 1, 0, 0], clock: 1, state: state)
    assert_equal [1, 1, 0, 0], state[:bits]

    # New cycle with different data but don't complete it
    state = CodingAdventures::SEQ.register(data: [0, 0, 1, 1], clock: 0, state: state)
    # Output should still be previous value
    assert_equal [1, 1, 0, 0], state[:bits], "Should hold during clock low"
  end

  # --- Overwrite ---

  def test_overwrites_value
    # Store [1, 1, 1, 1]
    state = CodingAdventures::SEQ.register(data: [1, 1, 1, 1], clock: 0)
    state = CodingAdventures::SEQ.register(data: [1, 1, 1, 1], clock: 1, state: state)
    assert_equal [1, 1, 1, 1], state[:bits]

    # Overwrite with [0, 0, 0, 0]
    state = CodingAdventures::SEQ.register(data: [0, 0, 0, 0], clock: 0, state: state)
    state = CodingAdventures::SEQ.register(data: [0, 0, 0, 0], clock: 1, state: state)
    assert_equal [0, 0, 0, 0], state[:bits]
  end

  # --- Various widths ---

  def test_1bit_register
    state = CodingAdventures::SEQ.register(data: [1], clock: 0)
    state = CodingAdventures::SEQ.register(data: [1], clock: 1, state: state)
    assert_equal [1], state[:bits]
  end

  def test_8bit_register
    data = [1, 0, 1, 0, 1, 0, 1, 0]
    state = CodingAdventures::SEQ.register(data: data, clock: 0)
    state = CodingAdventures::SEQ.register(data: data, clock: 1, state: state)
    assert_equal data, state[:bits]
  end

  # --- Default initial state ---

  def test_default_initial_state_is_zero
    state = CodingAdventures::SEQ.register(data: [0, 0, 0, 0], clock: 0)
    assert_equal [0, 0, 0, 0], state[:bits]
  end

  # --- Input validation ---

  def test_validates_data_is_array
    assert_raises(ArgumentError) { CodingAdventures::SEQ.register(data: 1, clock: 0) }
  end

  def test_validates_data_not_empty
    assert_raises(ArgumentError) { CodingAdventures::SEQ.register(data: [], clock: 0) }
  end

  def test_validates_data_bits
    assert_raises(ArgumentError) { CodingAdventures::SEQ.register(data: [0, 2, 1], clock: 0) }
    assert_raises(TypeError) { CodingAdventures::SEQ.register(data: [0, "1"], clock: 0) }
  end

  def test_validates_clock
    assert_raises(TypeError) { CodingAdventures::SEQ.register(data: [0, 1], clock: true) }
  end

  def test_validates_width_mismatch
    state = CodingAdventures::SEQ.register(data: [0, 1], clock: 0)
    assert_raises(ArgumentError) {
      CodingAdventures::SEQ.register(data: [0, 1, 0], clock: 0, state: state)
    }
  end
end

# =====================================================================
# Shift Register Tests
# =====================================================================

class TestShiftRegister < Minitest::Test
  # --- Right shift: bits enter from the left (position 0) ---

  def test_right_shift_single_bit
    # Shift in a 1 into a 4-bit register
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 0, width: 4)
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 1, width: 4, state: state)
    assert_equal [1, 0, 0, 0], state[:bits], "1 should enter at position 0"
  end

  def test_right_shift_propagation
    # Shift in a 1, then shift it through the register
    state = nil

    # Cycle 1: shift in 1
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 0, width: 4, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 1, width: 4, state: state)
    assert_equal [1, 0, 0, 0], state[:bits]

    # Cycle 2: shift in 0 -> previous 1 moves to position 1
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 4, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 1, width: 4, state: state)
    assert_equal [0, 1, 0, 0], state[:bits]

    # Cycle 3: shift again -> 1 moves to position 2
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 4, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 1, width: 4, state: state)
    assert_equal [0, 0, 1, 0], state[:bits]

    # Cycle 4: shift again -> 1 moves to position 3
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 4, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 1, width: 4, state: state)
    assert_equal [0, 0, 0, 1], state[:bits]

    # Cycle 5: shift again -> 1 falls off the end
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 4, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 1, width: 4, state: state)
    assert_equal [0, 0, 0, 0], state[:bits]
  end

  def test_right_shift_fill_pattern
    state = nil
    # Shift in alternating 1, 0, 1, 0
    [1, 0, 1, 0].each do |bit|
      state = CodingAdventures::SEQ.shift_register(serial_in: bit, clock: 0, width: 4, state: state)
      state = CodingAdventures::SEQ.shift_register(serial_in: bit, clock: 1, width: 4, state: state)
    end
    assert_equal [0, 1, 0, 1], state[:bits]
  end

  # --- Left shift: bits enter from the right (position N-1) ---

  def test_left_shift_single_bit
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 0, width: 4, direction: :left)
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 1, width: 4, direction: :left, state: state)
    assert_equal [0, 0, 0, 1], state[:bits], "1 should enter at last position"
  end

  def test_left_shift_propagation
    state = nil

    # Cycle 1: shift in 1 at the right
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 0, width: 4, direction: :left, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 1, width: 4, direction: :left, state: state)
    assert_equal [0, 0, 0, 1], state[:bits]

    # Cycle 2: shift in 0 -> 1 moves left to position 2
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 4, direction: :left, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 1, width: 4, direction: :left, state: state)
    assert_equal [0, 0, 1, 0], state[:bits]

    # Cycle 3: shift again -> 1 moves to position 1
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 4, direction: :left, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 1, width: 4, direction: :left, state: state)
    assert_equal [0, 1, 0, 0], state[:bits]

    # Cycle 4: shift again -> 1 moves to position 0
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 4, direction: :left, state: state)
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 1, width: 4, direction: :left, state: state)
    assert_equal [1, 0, 0, 0], state[:bits]
  end

  # --- Default width is 8 ---

  def test_default_width_is_8
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 0)
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 1, state: state)
    assert_equal 8, state[:bits].length
    assert_equal [1, 0, 0, 0, 0, 0, 0, 0], state[:bits]
  end

  # --- No change on clock low ---

  def test_no_change_on_clock_low
    state = CodingAdventures::SEQ.shift_register(serial_in: 1, clock: 0, width: 4)
    assert_equal [0, 0, 0, 0], state[:bits], "No change on clock low"
  end

  # --- Input validation ---

  def test_validates_serial_in
    assert_raises(TypeError) { CodingAdventures::SEQ.shift_register(serial_in: "1", clock: 0) }
    assert_raises(ArgumentError) { CodingAdventures::SEQ.shift_register(serial_in: 2, clock: 0) }
  end

  def test_validates_clock
    assert_raises(TypeError) { CodingAdventures::SEQ.shift_register(serial_in: 0, clock: nil) }
  end

  def test_validates_width
    assert_raises(ArgumentError) { CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 0) }
    assert_raises(ArgumentError) { CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: -1) }
  end

  def test_validates_direction
    assert_raises(ArgumentError) {
      CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, direction: :up)
    }
  end

  def test_validates_width_mismatch
    state = CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 4)
    assert_raises(ArgumentError) {
      CodingAdventures::SEQ.shift_register(serial_in: 0, clock: 0, width: 8, state: state)
    }
  end
end

# =====================================================================
# Counter Tests
# =====================================================================

class TestCounter < Minitest::Test
  # Helper: run one clock cycle (low then high)
  def clock_cycle(state, width: 4, reset: 0)
    state = CodingAdventures::SEQ.counter(clock: 0, state: state, width: width, reset: reset)
    CodingAdventures::SEQ.counter(clock: 1, state: state, width: width, reset: reset)
  end

  # --- Basic counting ---

  def test_counts_from_0_to_3
    state = nil

    # Count: 0 -> 1
    state = clock_cycle(state)
    assert_equal [1, 0, 0, 0], state[:bits], "Should count to 1 (binary 0001)"

    # Count: 1 -> 2
    state = clock_cycle(state)
    assert_equal [0, 1, 0, 0], state[:bits], "Should count to 2 (binary 0010)"

    # Count: 2 -> 3
    state = clock_cycle(state)
    assert_equal [1, 1, 0, 0], state[:bits], "Should count to 3 (binary 0011)"
  end

  def test_counts_to_15_and_wraps
    state = nil

    # Count from 0 to 15 (4-bit counter)
    expected_values = (1..15).to_a + [0]

    16.times do |i|
      state = clock_cycle(state)
      # Convert bits (LSB first) to integer
      value = state[:bits].each_with_index.sum { |bit, idx| bit * (2**idx) }
      assert_equal expected_values[i], value,
        "After #{i + 1} cycles, counter should be #{expected_values[i]}, got #{value}"
    end
  end

  # --- Reset ---

  def test_reset_clears_counter
    state = nil

    # Count up to 5
    5.times { state = clock_cycle(state) }
    value = state[:bits].each_with_index.sum { |bit, idx| bit * (2**idx) }
    assert_equal 5, value

    # Reset
    state = clock_cycle(state, reset: 1)
    assert_equal [0, 0, 0, 0], state[:bits], "Reset should clear counter to 0"
  end

  def test_counts_after_reset
    state = nil

    # Count to 3
    3.times { state = clock_cycle(state) }

    # Reset
    state = clock_cycle(state, reset: 1)
    assert_equal [0, 0, 0, 0], state[:bits]

    # Count again from 0
    state = clock_cycle(state)
    assert_equal [1, 0, 0, 0], state[:bits], "Should count from 0 after reset"
  end

  # --- Initial state ---

  def test_initial_state_is_zero
    state = CodingAdventures::SEQ.counter(clock: 0, width: 4)
    assert_equal [0, 0, 0, 0], state[:bits]
  end

  # --- Different widths ---

  def test_2bit_counter_wraps_at_4
    state = nil

    # Count 0 -> 1 -> 2 -> 3 -> 0 (wrap)
    4.times { state = clock_cycle(state, width: 2) }
    value = state[:bits].each_with_index.sum { |bit, idx| bit * (2**idx) }
    assert_equal 0, value, "2-bit counter should wrap at 4"
  end

  def test_8bit_counter
    state = nil

    # Count to 255 and verify wrap
    255.times { state = clock_cycle(state, width: 8) }
    value = state[:bits].each_with_index.sum { |bit, idx| bit * (2**idx) }
    assert_equal 255, value, "8-bit counter should reach 255"

    # One more -> wraps to 0
    state = clock_cycle(state, width: 8)
    value = state[:bits].each_with_index.sum { |bit, idx| bit * (2**idx) }
    assert_equal 0, value, "8-bit counter should wrap to 0"
  end

  # --- Input validation ---

  def test_validates_clock
    assert_raises(TypeError) { CodingAdventures::SEQ.counter(clock: "0") }
  end

  def test_validates_reset
    assert_raises(ArgumentError) { CodingAdventures::SEQ.counter(clock: 0, reset: 2) }
  end

  def test_validates_width
    assert_raises(ArgumentError) { CodingAdventures::SEQ.counter(clock: 0, width: 0) }
    assert_raises(ArgumentError) { CodingAdventures::SEQ.counter(clock: 0, width: -3) }
  end

  # --- No change on clock low ---

  def test_no_change_on_clock_low_alone
    state = CodingAdventures::SEQ.counter(clock: 0, width: 4)
    assert_equal [0, 0, 0, 0], state[:bits], "Counter should not advance on clock low alone"
  end
end
