"""Tests for sequential logic — latches, flip-flops, registers, and counters.

These tests verify each component's truth table, state-holding behavior,
and edge cases. We test from the bottom up, since higher-level components
(like registers) depend on lower-level ones (like flip-flops).
"""

import pytest

from logic_gates.sequential import (
    counter,
    d_flip_flop,
    d_latch,
    register,
    shift_register,
    sr_latch,
)


# ===========================================================================
# SR LATCH TESTS
# ===========================================================================


class TestSRLatch:
    """Tests for the SR (Set-Reset) latch."""

    def test_set(self) -> None:
        """S=1, R=0 should set Q=1."""
        q, q_bar = sr_latch(1, 0)
        assert q == 1
        assert q_bar == 0

    def test_reset(self) -> None:
        """S=0, R=1 should reset Q=0."""
        q, q_bar = sr_latch(0, 1)
        assert q == 0
        assert q_bar == 1

    def test_hold_after_set(self) -> None:
        """S=0, R=0 after set should hold Q=1."""
        q, q_bar = sr_latch(1, 0)
        assert q == 1
        # Now hold — pass the current state back
        q, q_bar = sr_latch(0, 0, q, q_bar)
        assert q == 1
        assert q_bar == 0

    def test_hold_after_reset(self) -> None:
        """S=0, R=0 after reset should hold Q=0."""
        q, q_bar = sr_latch(0, 1)
        assert q == 0
        q, q_bar = sr_latch(0, 0, q, q_bar)
        assert q == 0
        assert q_bar == 1

    def test_hold_default_state(self) -> None:
        """S=0, R=0 with default state should hold Q=0, Q_bar=1."""
        q, q_bar = sr_latch(0, 0)
        assert q == 0
        assert q_bar == 1

    def test_invalid_both_set_reset(self) -> None:
        """S=1, R=1 is the invalid state — both outputs go to 0."""
        q, q_bar = sr_latch(1, 1)
        assert q == 0
        assert q_bar == 0

    def test_set_then_reset_then_set(self) -> None:
        """Cycle through set, reset, set to verify state transitions."""
        q, q_bar = sr_latch(1, 0)
        assert q == 1

        q, q_bar = sr_latch(0, 1, q, q_bar)
        assert q == 0
        assert q_bar == 1

        q, q_bar = sr_latch(1, 0, q, q_bar)
        assert q == 1
        assert q_bar == 0

    def test_multiple_holds(self) -> None:
        """Holding state across many cycles should be stable."""
        q, q_bar = sr_latch(1, 0)  # Set
        for _ in range(10):
            q, q_bar = sr_latch(0, 0, q, q_bar)
        assert q == 1
        assert q_bar == 0

    def test_invalid_input_type(self) -> None:
        """Non-int inputs should raise TypeError."""
        with pytest.raises(TypeError):
            sr_latch(True, 0)  # type: ignore[arg-type]

    def test_invalid_input_value(self) -> None:
        """Out-of-range inputs should raise ValueError."""
        with pytest.raises(ValueError):
            sr_latch(2, 0)


# ===========================================================================
# D LATCH TESTS
# ===========================================================================


class TestDLatch:
    """Tests for the D (Data) latch."""

    def test_store_one_when_enabled(self) -> None:
        """Enable=1, Data=1 should store 1."""
        q, q_bar = d_latch(1, 1)
        assert q == 1
        assert q_bar == 0

    def test_store_zero_when_enabled(self) -> None:
        """Enable=1, Data=0 should store 0."""
        q, q_bar = d_latch(0, 1)
        assert q == 0
        assert q_bar == 1

    def test_hold_when_disabled_after_set(self) -> None:
        """Enable=0 should hold previous value."""
        q, q_bar = d_latch(1, 1)  # Store 1
        assert q == 1
        q, q_bar = d_latch(0, 0, q, q_bar)  # Disable — data is 0 but ignored
        assert q == 1
        assert q_bar == 0

    def test_hold_when_disabled_after_reset(self) -> None:
        """Enable=0 should hold previous value (0)."""
        q, q_bar = d_latch(0, 1)  # Store 0
        q, q_bar = d_latch(1, 0, q, q_bar)  # Disable — data is 1 but ignored
        assert q == 0
        assert q_bar == 1

    def test_transparent_mode(self) -> None:
        """When enabled, output should follow data changes."""
        q, q_bar = d_latch(1, 1)
        assert q == 1
        q, q_bar = d_latch(0, 1, q, q_bar)
        assert q == 0
        q, q_bar = d_latch(1, 1, q, q_bar)
        assert q == 1

    def test_hold_default_state(self) -> None:
        """Enable=0 with default state should hold Q=0."""
        q, q_bar = d_latch(1, 0)
        assert q == 0
        assert q_bar == 1

    def test_invalid_input_type(self) -> None:
        """Non-int inputs should raise TypeError."""
        with pytest.raises(TypeError):
            d_latch(True, 1)  # type: ignore[arg-type]

    def test_invalid_input_value(self) -> None:
        """Out-of-range inputs should raise ValueError."""
        with pytest.raises(ValueError):
            d_latch(0, 2)


# ===========================================================================
# D FLIP-FLOP TESTS
# ===========================================================================


class TestDFlipFlop:
    """Tests for the D Flip-Flop (master-slave configuration)."""

    def test_capture_on_rising_edge(self) -> None:
        """Data should be captured when clock transitions 0 -> 1."""
        # Clock low: master absorbs data=1
        q, q_bar, state = d_flip_flop(1, 0)
        # Clock high: slave outputs master's stored value
        q, q_bar, state = d_flip_flop(1, 1, **state)
        assert q == 1
        assert q_bar == 0

    def test_capture_zero(self) -> None:
        """Capturing data=0 on rising edge."""
        q, q_bar, state = d_flip_flop(0, 0)
        q, q_bar, state = d_flip_flop(0, 1, **state)
        assert q == 0
        assert q_bar == 1

    def test_hold_during_clock_high(self) -> None:
        """Changing data while clock is high should not change output."""
        # Capture 1
        q, q_bar, state = d_flip_flop(1, 0)
        q, q_bar, state = d_flip_flop(1, 1, **state)
        assert q == 1
        # Data changes to 0 while clock is still high — output should hold
        q, q_bar, state = d_flip_flop(0, 1, **state)
        assert q == 1

    def test_new_value_on_next_edge(self) -> None:
        """New data captured on the next rising edge."""
        # Capture 1
        q, q_bar, state = d_flip_flop(1, 0)
        q, q_bar, state = d_flip_flop(1, 1, **state)
        assert q == 1

        # Next cycle: capture 0
        q, q_bar, state = d_flip_flop(0, 0, **state)
        q, q_bar, state = d_flip_flop(0, 1, **state)
        assert q == 0
        assert q_bar == 1

    def test_multiple_clock_cycles(self) -> None:
        """Sequence of values captured over multiple clock cycles."""
        state: dict[str, int] = {
            "master_q": 0,
            "master_q_bar": 1,
            "slave_q": 0,
            "slave_q_bar": 1,
        }
        values = [1, 0, 1, 1, 0]
        for val in values:
            _, _, state = d_flip_flop(val, 0, **state)
            q, _, state = d_flip_flop(val, 1, **state)
            assert q == val

    def test_internal_state_keys(self) -> None:
        """Internal state should contain the expected keys."""
        _, _, state = d_flip_flop(1, 0)
        assert "master_q" in state
        assert "master_q_bar" in state
        assert "slave_q" in state
        assert "slave_q_bar" in state

    def test_invalid_input_type(self) -> None:
        """Non-int inputs should raise TypeError."""
        with pytest.raises(TypeError):
            d_flip_flop(True, 0)  # type: ignore[arg-type]

    def test_invalid_input_value(self) -> None:
        """Out-of-range inputs should raise ValueError."""
        with pytest.raises(ValueError):
            d_flip_flop(0, 2)


# ===========================================================================
# REGISTER TESTS
# ===========================================================================


class TestRegister:
    """Tests for the N-bit register."""

    def test_store_4bit_value(self) -> None:
        """Store and retrieve a 4-bit value."""
        # Clock low: absorb data
        out, state = register([1, 0, 1, 1], 0)
        # Clock high: output stored data
        out, state = register([1, 0, 1, 1], 1, state)
        assert out == [1, 0, 1, 1]

    def test_store_8bit_value(self) -> None:
        """Store and retrieve an 8-bit value."""
        data = [1, 0, 0, 1, 1, 0, 1, 0]
        out, state = register(data, 0)
        out, state = register(data, 1, state)
        assert out == data

    def test_hold_previous_value(self) -> None:
        """Register should hold value when new data is presented with clock low."""
        data1 = [1, 1, 0, 0]
        out, state = register(data1, 0)
        out, state = register(data1, 1, state)
        assert out == data1

        # Present new data, clock low then high
        data2 = [0, 0, 1, 1]
        out, state = register(data2, 0, state)
        out, state = register(data2, 1, state)
        assert out == data2

    def test_width_parameter(self) -> None:
        """Width parameter should enforce data length."""
        out, state = register([1, 0], 0, width=2)
        assert len(out) == 2

    def test_width_mismatch_raises(self) -> None:
        """Mismatched data length and width should raise ValueError."""
        with pytest.raises(ValueError, match="does not match width"):
            register([1, 0, 1], 0, width=2)

    def test_empty_data_raises(self) -> None:
        """Empty data should raise ValueError."""
        with pytest.raises(ValueError, match="must not be empty"):
            register([], 0)

    def test_non_list_data_raises(self) -> None:
        """Non-list data should raise TypeError."""
        with pytest.raises(TypeError, match="must be a list"):
            register(1, 0)  # type: ignore[arg-type]

    def test_state_length_mismatch_raises(self) -> None:
        """Mismatched state and data lengths should raise ValueError."""
        _, state = register([1, 0], 0)
        with pytest.raises(ValueError, match="does not match data length"):
            register([1, 0, 1], 0, state)

    def test_single_bit_register(self) -> None:
        """A 1-bit register is just a flip-flop."""
        out, state = register([1], 0)
        out, state = register([1], 1, state)
        assert out == [1]

    def test_invalid_bit_in_data(self) -> None:
        """Invalid bit values in data should raise."""
        with pytest.raises(ValueError):
            register([0, 2, 1], 0)

    def test_all_zeros(self) -> None:
        """Register storing all zeros."""
        data = [0, 0, 0, 0]
        out, state = register(data, 0)
        out, state = register(data, 1, state)
        assert out == [0, 0, 0, 0]

    def test_all_ones(self) -> None:
        """Register storing all ones."""
        data = [1, 1, 1, 1]
        out, state = register(data, 0)
        out, state = register(data, 1, state)
        assert out == [1, 1, 1, 1]


# ===========================================================================
# SHIFT REGISTER TESTS
# ===========================================================================


class TestShiftRegister:
    """Tests for the shift register."""

    def test_shift_right_single_bit(self) -> None:
        """Shift a single 1 into a 4-bit register from the right."""
        out, sout, state = shift_register(1, 0, width=4)
        out, sout, state = shift_register(1, 1, state, width=4)
        assert out == [1, 0, 0, 0]
        assert sout == 0  # Nothing shifted out yet

    def test_shift_right_fill(self) -> None:
        """Shift 1s into all positions from the right."""
        state = None
        for _ in range(4):
            _, _, state = shift_register(1, 0, state, width=4)
            out, sout, state = shift_register(1, 1, state, width=4)
        assert out == [1, 1, 1, 1]

    def test_shift_right_serial_out(self) -> None:
        """Bits shifted out from the right end."""
        # Fill with 1s
        state = None
        for _ in range(4):
            _, _, state = shift_register(1, 0, state, width=4)
            _, sout, state = shift_register(1, 1, state, width=4)

        # Now shift in 0 — the rightmost 1 should come out
        _, _, state = shift_register(0, 0, state, width=4)
        out, sout, state = shift_register(0, 1, state, width=4)
        assert sout == 1
        assert out == [0, 1, 1, 1]

    def test_shift_left_single_bit(self) -> None:
        """Shift a single 1 into a 4-bit register from the left."""
        out, sout, state = shift_register(1, 0, width=4, direction="left")
        out, sout, state = shift_register(
            1, 1, state, width=4, direction="left"
        )
        assert out == [0, 0, 0, 1]
        assert sout == 0

    def test_shift_left_fill(self) -> None:
        """Fill a register by shifting left."""
        state = None
        for _ in range(4):
            _, _, state = shift_register(
                1, 0, state, width=4, direction="left"
            )
            out, _, state = shift_register(
                1, 1, state, width=4, direction="left"
            )
        assert out == [1, 1, 1, 1]

    def test_shift_left_serial_out(self) -> None:
        """Bits shifted out from the left end."""
        # Fill with 1s
        state = None
        for _ in range(4):
            _, _, state = shift_register(
                1, 0, state, width=4, direction="left"
            )
            _, _, state = shift_register(
                1, 1, state, width=4, direction="left"
            )

        # Shift in 0 — leftmost 1 should come out
        _, _, state = shift_register(0, 0, state, width=4, direction="left")
        out, sout, state = shift_register(
            0, 1, state, width=4, direction="left"
        )
        assert sout == 1
        assert out == [1, 1, 1, 0]

    def test_pattern_shift_right(self) -> None:
        """Shift a pattern of bits through the register."""
        pattern = [1, 0, 1, 0]
        state = None
        for bit in pattern:
            _, _, state = shift_register(bit, 0, state, width=4)
            out, _, state = shift_register(bit, 1, state, width=4)
        # Last bit shifted in is at index 0, first bit at index 3
        assert out == [0, 1, 0, 1]

    def test_width_1(self) -> None:
        """Width=1 shift register."""
        out, sout, state = shift_register(1, 0, width=1)
        out, sout, state = shift_register(1, 1, state, width=1)
        assert out == [1]
        assert sout == 0

    def test_invalid_direction_raises(self) -> None:
        """Invalid direction should raise ValueError."""
        with pytest.raises(ValueError, match="direction must be"):
            shift_register(1, 0, direction="up")

    def test_invalid_width_raises(self) -> None:
        """Width < 1 should raise ValueError."""
        with pytest.raises(ValueError, match="width must be"):
            shift_register(1, 0, width=0)

    def test_state_length_mismatch_raises(self) -> None:
        """Mismatched state and width should raise ValueError."""
        _, _, state = shift_register(1, 0, width=4)
        with pytest.raises(ValueError, match="does not match width"):
            shift_register(1, 0, state, width=3)

    def test_default_width_8(self) -> None:
        """Default width should be 8."""
        out, _, _ = shift_register(0, 0)
        assert len(out) == 8


# ===========================================================================
# COUNTER TESTS
# ===========================================================================


class TestCounter:
    """Tests for the binary counter."""

    def test_count_to_three(self) -> None:
        """Counter should count 1, 2, 3, 4 after successive clock edges.

        The counter increments on each rising clock edge, so the first
        tick produces 1 (not 0). This matches hardware behavior — a real
        counter starts at 0 and the first edge gives 1.
        """
        state = None
        expected_values = [
            [1, 0, 0, 0],  # 1 (first tick)
            [0, 1, 0, 0],  # 2
            [1, 1, 0, 0],  # 3
            [0, 0, 1, 0],  # 4
        ]

        for i in range(4):
            bits, state = counter(0, state=state, width=4)
            bits, state = counter(1, state=state, width=4)
            assert bits == expected_values[i], f"Expected {expected_values[i]} at tick {i+1}, got {bits}"

    def test_reset(self) -> None:
        """Reset should force counter to zero."""
        state = None
        # Count up a couple times
        for _ in range(3):
            _, state = counter(0, state=state, width=4)
            _, state = counter(1, state=state, width=4)

        # Reset
        bits, state = counter(0, reset=1, state=state, width=4)
        bits, state = counter(1, reset=1, state=state, width=4)
        assert bits == [0, 0, 0, 0]

    def test_count_after_reset(self) -> None:
        """Counter should count normally after reset."""
        state = None
        # Count to 2
        for _ in range(3):
            _, state = counter(0, state=state, width=4)
            _, state = counter(1, state=state, width=4)

        # Reset
        _, state = counter(0, reset=1, state=state, width=4)
        _, state = counter(1, reset=1, state=state, width=4)

        # Count again
        bits, state = counter(0, state=state, width=4)
        bits, state = counter(1, state=state, width=4)
        assert bits == [1, 0, 0, 0]  # 1

    def test_overflow_wraps(self) -> None:
        """Counter should wrap from max to 0 on overflow."""
        state = None
        width = 3  # Max value = 7

        # Count to 7 (7 ticks since each tick increments)
        for _ in range(7):
            _, state = counter(0, state=state, width=width)
            bits, state = counter(1, state=state, width=width)

        assert bits == [1, 1, 1]  # 7

        # One more tick should wrap to 0
        _, state = counter(0, state=state, width=width)
        bits, state = counter(1, state=state, width=width)
        assert bits == [0, 0, 0]  # 0 (overflow)

    def test_1bit_counter(self) -> None:
        """1-bit counter should toggle between 0 and 1."""
        state = None
        # Tick 1 — starts at 0, increments to 1
        _, state = counter(0, state=state, width=1)
        bits, state = counter(1, state=state, width=1)
        assert bits == [1]

        # Tick 2 — wraps to 0
        _, state = counter(0, state=state, width=1)
        bits, state = counter(1, state=state, width=1)
        assert bits == [0]

        # Tick 3 — back to 1
        _, state = counter(0, state=state, width=1)
        bits, state = counter(1, state=state, width=1)
        assert bits == [1]

    def test_invalid_width_raises(self) -> None:
        """Width < 1 should raise ValueError."""
        with pytest.raises(ValueError, match="width must be"):
            counter(0, width=0)

    def test_default_width_8(self) -> None:
        """Default width should be 8."""
        bits, _ = counter(0)
        assert len(bits) == 8

    def test_counter_state_keys(self) -> None:
        """State should have 'value' and 'ff_state' keys."""
        _, state = counter(0, width=4)
        assert "value" in state
        assert "ff_state" in state
