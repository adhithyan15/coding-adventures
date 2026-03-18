"""Tests for the clock package.

These tests verify the fundamental clock behavior: signal toggling,
edge detection, cycle counting, listener notification, frequency
division, and multi-phase generation.
"""

from __future__ import annotations

import pytest

from clock import Clock, ClockDivider, ClockEdge, MultiPhaseClock


# ---------------------------------------------------------------------------
# Basic clock behavior
# ---------------------------------------------------------------------------


class TestClockInitialState:
    """Verify the clock starts in a known state."""

    def test_starts_at_zero(self) -> None:
        """The clock signal starts low (0), like a real oscillator
        before it begins oscillating."""
        clk = Clock()
        assert clk.value == 0

    def test_starts_at_cycle_zero(self) -> None:
        """No cycles have elapsed before the first tick."""
        clk = Clock()
        assert clk.cycle == 0

    def test_starts_with_zero_ticks(self) -> None:
        """No ticks have occurred yet."""
        clk = Clock()
        assert clk.total_ticks == 0

    def test_default_frequency(self) -> None:
        """Default frequency is 1 MHz."""
        clk = Clock()
        assert clk.frequency_hz == 1_000_000

    def test_custom_frequency(self) -> None:
        """Can specify a custom frequency."""
        clk = Clock(frequency_hz=3_000_000_000)
        assert clk.frequency_hz == 3_000_000_000


class TestClockTick:
    """Verify the tick() method produces correct edges."""

    def test_first_tick_is_rising(self) -> None:
        """First tick goes from 0 to 1 -- a rising edge."""
        clk = Clock()
        edge = clk.tick()
        assert edge.is_rising is True
        assert edge.is_falling is False
        assert edge.value == 1
        assert clk.value == 1

    def test_second_tick_is_falling(self) -> None:
        """Second tick goes from 1 to 0 -- a falling edge."""
        clk = Clock()
        clk.tick()  # rising
        edge = clk.tick()  # falling
        assert edge.is_rising is False
        assert edge.is_falling is True
        assert edge.value == 0
        assert clk.value == 0

    def test_alternates_correctly(self) -> None:
        """The clock should alternate: rise, fall, rise, fall, ..."""
        clk = Clock()
        for i in range(10):
            edge = clk.tick()
            if i % 2 == 0:
                assert edge.is_rising is True, f"Tick {i} should be rising"
            else:
                assert edge.is_falling is True, f"Tick {i} should be falling"

    def test_cycle_increments_on_rising(self) -> None:
        """Cycle count goes up by 1 on each rising edge."""
        clk = Clock()
        edge1 = clk.tick()  # rising
        assert edge1.cycle == 1
        assert clk.cycle == 1

        edge2 = clk.tick()  # falling
        assert edge2.cycle == 1  # still cycle 1
        assert clk.cycle == 1

        edge3 = clk.tick()  # rising
        assert edge3.cycle == 2
        assert clk.cycle == 2

    def test_tick_count_increments_every_tick(self) -> None:
        """Total ticks counts every half-cycle."""
        clk = Clock()
        clk.tick()
        assert clk.total_ticks == 1
        clk.tick()
        assert clk.total_ticks == 2
        clk.tick()
        assert clk.total_ticks == 3


class TestClockFullCycle:
    """Verify full_cycle() runs one complete cycle."""

    def test_returns_rising_then_falling(self) -> None:
        """full_cycle produces exactly one rising and one falling edge."""
        clk = Clock()
        rising, falling = clk.full_cycle()
        assert rising.is_rising is True
        assert falling.is_falling is True

    def test_ends_at_zero(self) -> None:
        """After a full cycle, the clock is back to 0."""
        clk = Clock()
        clk.full_cycle()
        assert clk.value == 0

    def test_cycle_count_is_one(self) -> None:
        """One full_cycle means one cycle elapsed."""
        clk = Clock()
        clk.full_cycle()
        assert clk.cycle == 1

    def test_two_ticks_elapsed(self) -> None:
        """A full cycle is two half-cycles."""
        clk = Clock()
        clk.full_cycle()
        assert clk.total_ticks == 2


class TestClockRun:
    """Verify run(N) executes N complete cycles."""

    def test_run_produces_correct_edge_count(self) -> None:
        """N cycles = 2N edges (each cycle has rising + falling)."""
        clk = Clock()
        edges = clk.run(5)
        assert len(edges) == 10

    def test_run_edges_alternate(self) -> None:
        """Edges should alternate rising/falling."""
        clk = Clock()
        edges = clk.run(3)
        for i, edge in enumerate(edges):
            if i % 2 == 0:
                assert edge.is_rising is True
            else:
                assert edge.is_falling is True

    def test_run_final_cycle_count(self) -> None:
        """After run(N), cycle count should be N."""
        clk = Clock()
        clk.run(7)
        assert clk.cycle == 7

    def test_run_zero_cycles(self) -> None:
        """run(0) does nothing."""
        clk = Clock()
        edges = clk.run(0)
        assert len(edges) == 0
        assert clk.cycle == 0


# ---------------------------------------------------------------------------
# Listeners (observer pattern)
# ---------------------------------------------------------------------------


class TestClockListeners:
    """Verify the observer pattern for clock edges."""

    def test_listener_called_on_tick(self) -> None:
        """A registered listener receives every edge."""
        clk = Clock()
        received: list[ClockEdge] = []
        clk.register_listener(received.append)
        clk.tick()
        assert len(received) == 1
        assert received[0].is_rising is True

    def test_listener_sees_all_edges(self) -> None:
        """Listener is called for both rising and falling edges."""
        clk = Clock()
        received: list[ClockEdge] = []
        clk.register_listener(received.append)
        clk.run(3)
        assert len(received) == 6

    def test_multiple_listeners(self) -> None:
        """Multiple listeners all get notified."""
        clk = Clock()
        a: list[ClockEdge] = []
        b: list[ClockEdge] = []
        clk.register_listener(a.append)
        clk.register_listener(b.append)
        clk.tick()
        assert len(a) == 1
        assert len(b) == 1

    def test_unregister_listener(self) -> None:
        """After unregistering, listener stops receiving edges."""
        clk = Clock()
        received: list[ClockEdge] = []
        clk.register_listener(received.append)
        clk.tick()  # 1 edge received
        clk.unregister_listener(received.append)
        clk.tick()  # should NOT be received
        assert len(received) == 1

    def test_unregister_nonexistent_raises(self) -> None:
        """Unregistering a callback that was never registered raises ValueError."""
        clk = Clock()

        def dummy(_edge: ClockEdge) -> None:
            pass

        with pytest.raises(ValueError):
            clk.unregister_listener(dummy)


# ---------------------------------------------------------------------------
# Reset
# ---------------------------------------------------------------------------


class TestClockReset:
    """Verify reset() restores the initial state."""

    def test_reset_value(self) -> None:
        """Value goes back to 0."""
        clk = Clock()
        clk.tick()  # now 1
        clk.reset()
        assert clk.value == 0

    def test_reset_cycle(self) -> None:
        """Cycle count goes back to 0."""
        clk = Clock()
        clk.run(5)
        clk.reset()
        assert clk.cycle == 0

    def test_reset_ticks(self) -> None:
        """Tick count goes back to 0."""
        clk = Clock()
        clk.run(5)
        clk.reset()
        assert clk.total_ticks == 0

    def test_reset_preserves_listeners(self) -> None:
        """Listeners survive a reset."""
        clk = Clock()
        received: list[ClockEdge] = []
        clk.register_listener(received.append)
        clk.run(3)
        clk.reset()
        clk.tick()
        # Should still receive edges after reset
        assert len(received) == 7  # 6 from run(3) + 1 from tick()

    def test_reset_preserves_frequency(self) -> None:
        """Frequency is unchanged after reset."""
        clk = Clock(frequency_hz=5_000_000)
        clk.run(10)
        clk.reset()
        assert clk.frequency_hz == 5_000_000


# ---------------------------------------------------------------------------
# Period calculation
# ---------------------------------------------------------------------------


class TestClockPeriod:
    """Verify period_ns property."""

    def test_1mhz_period(self) -> None:
        """1 MHz = 1000 ns period."""
        clk = Clock(frequency_hz=1_000_000)
        assert clk.period_ns == 1000.0

    def test_1ghz_period(self) -> None:
        """1 GHz = 1 ns period."""
        clk = Clock(frequency_hz=1_000_000_000)
        assert clk.period_ns == 1.0

    def test_3ghz_period(self) -> None:
        """3 GHz ~ 0.333 ns period."""
        clk = Clock(frequency_hz=3_000_000_000)
        assert abs(clk.period_ns - 1e9 / 3_000_000_000) < 1e-10


# ---------------------------------------------------------------------------
# ClockDivider
# ---------------------------------------------------------------------------


class TestClockDivider:
    """Verify frequency division."""

    def test_divide_by_2(self) -> None:
        """Dividing by 2: every 2 source cycles = 1 output cycle."""
        master = Clock(frequency_hz=1_000_000)
        divider = ClockDivider(master, divisor=2)
        master.run(4)  # 4 master cycles
        assert divider.output.cycle == 2

    def test_divide_by_4(self) -> None:
        """Dividing by 4: every 4 source cycles = 1 output cycle."""
        master = Clock(frequency_hz=1_000_000_000)
        divider = ClockDivider(master, divisor=4)
        master.run(8)
        assert divider.output.cycle == 2

    def test_output_frequency(self) -> None:
        """Output clock has the divided frequency."""
        master = Clock(frequency_hz=1_000_000_000)
        divider = ClockDivider(master, divisor=4)
        assert divider.output.frequency_hz == 250_000_000

    def test_divisor_too_small(self) -> None:
        """Divisor must be >= 2."""
        master = Clock()
        with pytest.raises(ValueError, match="Divisor must be >= 2"):
            ClockDivider(master, divisor=1)

    def test_divisor_zero(self) -> None:
        """Divisor of 0 is invalid."""
        master = Clock()
        with pytest.raises(ValueError, match="Divisor must be >= 2"):
            ClockDivider(master, divisor=0)

    def test_divisor_negative(self) -> None:
        """Negative divisor is invalid."""
        master = Clock()
        with pytest.raises(ValueError, match="Divisor must be >= 2"):
            ClockDivider(master, divisor=-1)

    def test_output_value_returns_to_zero(self) -> None:
        """Output clock value returns to 0 after each output cycle."""
        master = Clock(frequency_hz=1_000_000)
        divider = ClockDivider(master, divisor=2)
        master.run(2)  # Should trigger 1 output cycle
        assert divider.output.value == 0  # Full cycle completed


# ---------------------------------------------------------------------------
# MultiPhaseClock
# ---------------------------------------------------------------------------


class TestMultiPhaseClock:
    """Verify multi-phase clock generation."""

    def test_initial_state_all_zero(self) -> None:
        """Before any ticks, all phases are 0."""
        master = Clock()
        mpc = MultiPhaseClock(master, phases=4)
        for i in range(4):
            assert mpc.get_phase(i) == 0

    def test_first_rising_activates_phase_0(self) -> None:
        """After the first rising edge, phase 0 is active."""
        master = Clock()
        mpc = MultiPhaseClock(master, phases=4)
        master.tick()  # rising edge
        assert mpc.get_phase(0) == 1
        assert mpc.get_phase(1) == 0
        assert mpc.get_phase(2) == 0
        assert mpc.get_phase(3) == 0

    def test_phases_rotate(self) -> None:
        """Each rising edge rotates to the next phase."""
        master = Clock()
        mpc = MultiPhaseClock(master, phases=4)

        # Cycle through all 4 phases
        for expected_phase in range(4):
            master.tick()  # rising
            for p in range(4):
                if p == expected_phase:
                    assert mpc.get_phase(p) == 1, f"Phase {p} should be active"
                else:
                    assert mpc.get_phase(p) == 0, f"Phase {p} should be inactive"
            master.tick()  # falling (no change)

    def test_phases_wrap_around(self) -> None:
        """After cycling through all phases, it wraps back to phase 0."""
        master = Clock()
        mpc = MultiPhaseClock(master, phases=3)

        # 3 rising edges cycle through phases 0, 1, 2
        for _ in range(3):
            master.full_cycle()

        # 4th rising edge should activate phase 0 again
        master.tick()  # rising
        assert mpc.get_phase(0) == 1
        assert mpc.get_phase(1) == 0
        assert mpc.get_phase(2) == 0

    def test_only_one_phase_active(self) -> None:
        """At any time, at most one phase is active (non-overlapping)."""
        master = Clock()
        mpc = MultiPhaseClock(master, phases=4)

        for _ in range(20):
            master.tick()
            active_count = sum(mpc.get_phase(i) for i in range(4))
            assert active_count <= 1, "More than one phase active!"

    def test_phases_too_small(self) -> None:
        """Phases must be >= 2."""
        master = Clock()
        with pytest.raises(ValueError, match="Phases must be >= 2"):
            MultiPhaseClock(master, phases=1)

    def test_phases_zero(self) -> None:
        """Zero phases is invalid."""
        master = Clock()
        with pytest.raises(ValueError, match="Phases must be >= 2"):
            MultiPhaseClock(master, phases=0)

    def test_two_phase_clock(self) -> None:
        """A 2-phase clock alternates between two phases."""
        master = Clock()
        mpc = MultiPhaseClock(master, phases=2)

        master.tick()  # rising -> phase 0 active
        assert mpc.get_phase(0) == 1
        assert mpc.get_phase(1) == 0

        master.tick()  # falling -> no change
        master.tick()  # rising -> phase 1 active
        assert mpc.get_phase(0) == 0
        assert mpc.get_phase(1) == 1


# ---------------------------------------------------------------------------
# ClockEdge dataclass
# ---------------------------------------------------------------------------


class TestClockEdge:
    """Verify ClockEdge fields."""

    def test_edge_fields(self) -> None:
        """ClockEdge stores all transition information."""
        edge = ClockEdge(cycle=3, value=1, is_rising=True, is_falling=False)
        assert edge.cycle == 3
        assert edge.value == 1
        assert edge.is_rising is True
        assert edge.is_falling is False

    def test_edge_equality(self) -> None:
        """Two edges with the same fields are equal (dataclass)."""
        a = ClockEdge(cycle=1, value=1, is_rising=True, is_falling=False)
        b = ClockEdge(cycle=1, value=1, is_rising=True, is_falling=False)
        assert a == b
