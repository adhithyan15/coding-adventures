"""Clock -- the heartbeat of every digital circuit.

Every sequential circuit in a computer -- flip-flops, registers, counters,
CPU pipeline stages, GPU cores -- is driven by a clock signal. The clock
is a square wave that alternates between 0 and 1:

    +--+  +--+  +--+  +--+
    |  |  |  |  |  |  |  |
----+  +--+  +--+  +--+  +--

On each rising edge (0->1), flip-flops capture their inputs. This is
what makes synchronous digital logic work -- everything happens in
lockstep, driven by the clock.

In real hardware:
- CPU clock: 3-5 GHz (3-5 billion cycles per second)
- GPU clock: 1-2 GHz
- Memory clock: 4-8 GHz (DDR5)
- The clock frequency is the single most important performance number

Why does the clock matter?
=========================

Without a clock, digital circuits would be chaotic. Imagine a chain of
logic gates where each gate has a slightly different propagation delay.
Without synchronization, signals would arrive at different times and
produce garbage. The clock solves this by saying: "Everyone, capture
your inputs NOW." This is called synchronous design.

The clock period must be long enough for the slowest signal path to
settle. This slowest path is called the "critical path," and it
determines the maximum clock frequency. Faster clocks = more operations
per second = faster computers, but only up to the point where signals
can still settle between edges.

Half-cycles and edges
=====================

A single clock cycle has two halves:

    Tick 0: value goes 0 -> 1 (RISING EDGE)   <- most circuits trigger here
    Tick 1: value goes 1 -> 0 (FALLING EDGE)   <- some DDR circuits use this too

"DDR" (Double Data Rate) memory uses BOTH edges, which is why DDR5-6400
actually runs at 3200 MHz but transfers data on both rising and falling
edges, achieving 6400 MT/s (megatransfers per second).

In our simulation, each call to tick() advances one half-cycle.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable


# ---------------------------------------------------------------------------
# ClockEdge -- a record of one transition
# ---------------------------------------------------------------------------


@dataclass
class ClockEdge:
    """Record of a clock transition.

    Every time the clock ticks, it produces an edge. An edge captures:
    - Which cycle we are in (cycles count from 1)
    - The current signal level (0 or 1)
    - Whether this was a rising edge (0->1) or falling edge (1->0)

    Think of it like a timestamp in a logic analyzer trace.
    """

    cycle: int
    value: int  # 0 or 1 (current level after the transition)
    is_rising: bool  # True if this was a 0->1 transition
    is_falling: bool  # True if this was a 1->0 transition


# ---------------------------------------------------------------------------
# Clock -- the main square-wave generator
# ---------------------------------------------------------------------------


@dataclass
class Clock:
    """System clock generator.

    The clock maintains a cycle count and alternates between low (0) and
    high (1) on each tick. Components connect to the clock and react to
    edges (transitions).

    A complete cycle is: low -> high -> low (two ticks).

    Example usage::

        clock = Clock(frequency_hz=1_000_000)  # 1 MHz
        edge = clock.tick()       # rising edge, cycle 1
        edge = clock.tick()       # falling edge, cycle 1
        edge = clock.tick()       # rising edge, cycle 2

    The observer pattern (listeners) allows components to react to clock
    edges without polling. This mirrors how real hardware works: components
    are physically connected to the clock line and react to voltage changes.
    """

    frequency_hz: int = 1_000_000  # default 1 MHz
    cycle: int = field(default=0, init=False)
    value: int = field(default=0, init=False)
    _tick_count: int = field(default=0, init=False)
    _listeners: list[Callable[[ClockEdge], None]] = field(
        default_factory=list, init=False
    )

    def tick(self) -> ClockEdge:
        """Advance one half-cycle. Returns the edge that occurred.

        The clock alternates like a toggle switch:
        - If currently 0, goes to 1 (rising edge, new cycle starts)
        - If currently 1, goes to 0 (falling edge, cycle ends)

        After toggling, all registered listeners are notified with the
        edge record. This is how connected components "see" the clock.

        Returns:
            ClockEdge with the transition details.
        """
        old_value = self.value
        self.value = 1 - self.value
        self._tick_count += 1

        is_rising = old_value == 0 and self.value == 1
        is_falling = old_value == 1 and self.value == 0

        # Cycle count increments on each rising edge.
        # Cycle 1 starts with the first rising edge, cycle 2 with the second, etc.
        if is_rising:
            self.cycle += 1

        edge = ClockEdge(
            cycle=self.cycle,
            value=self.value,
            is_rising=is_rising,
            is_falling=is_falling,
        )

        # Notify all listeners -- this is the observer pattern.
        # In real hardware, this is just electrical connectivity.
        for listener in self._listeners:
            listener(edge)

        return edge

    def full_cycle(self) -> tuple[ClockEdge, ClockEdge]:
        """Execute one complete cycle (rising + falling edge).

        A full cycle is two ticks:
        1. Rising edge (0 -> 1): the "active" half
        2. Falling edge (1 -> 0): the "idle" half

        Returns:
            Tuple of (rising_edge, falling_edge).
        """
        rising = self.tick()
        falling = self.tick()
        return rising, falling

    def run(self, cycles: int) -> list[ClockEdge]:
        """Run for N complete cycles. Returns all edges.

        This is a convenience method for running a fixed number of cycles.
        Since each cycle has two edges (rising + falling), running N cycles
        produces 2N edges total.

        Args:
            cycles: Number of complete cycles to execute.

        Returns:
            List of all ClockEdge objects produced.
        """
        edges: list[ClockEdge] = []
        for _ in range(cycles):
            r, f = self.full_cycle()
            edges.extend([r, f])
        return edges

    def register_listener(self, callback: Callable[[ClockEdge], None]) -> None:
        """Register a function to be called on every clock edge.

        In real hardware, this is like connecting a wire from the clock
        to a component's clock input pin. The component will "see" every
        transition.

        Args:
            callback: Function that takes a ClockEdge argument.
        """
        self._listeners.append(callback)

    def unregister_listener(self, callback: Callable[[ClockEdge], None]) -> None:
        """Remove a previously registered listener.

        Args:
            callback: The same function object that was registered.

        Raises:
            ValueError: If the callback was not registered.
        """
        self._listeners.remove(callback)

    def reset(self) -> None:
        """Reset the clock to its initial state.

        Sets the value back to 0, cycle count to 0, and tick count to 0.
        Listeners are preserved -- only the timing state is reset.
        This is like hitting the reset button on an oscillator.
        """
        self.cycle = 0
        self.value = 0
        self._tick_count = 0

    @property
    def period_ns(self) -> float:
        """Clock period in nanoseconds.

        The period is the time for one complete cycle (rising + falling).
        For a 1 GHz clock, the period is 1 ns. For 1 MHz, it is 1000 ns.

        Formula: period = 1 / frequency
        In nanoseconds: period_ns = 1e9 / frequency_hz
        """
        return 1e9 / self.frequency_hz

    @property
    def total_ticks(self) -> int:
        """Total half-cycles elapsed since creation or last reset."""
        return self._tick_count


# ---------------------------------------------------------------------------
# ClockDivider -- frequency division
# ---------------------------------------------------------------------------


class ClockDivider:
    """Divides a clock frequency by an integer factor.

    In hardware, clock dividers are used to generate slower clocks from
    a fast master clock. For example, a 1 GHz CPU clock might be divided
    by 4 to get a 250 MHz bus clock.

    How it works:
    - Count rising edges from the source clock
    - Every `divisor` rising edges, generate one full cycle on the output

    This means the output frequency = source frequency / divisor.

    Example::

        master = Clock(frequency_hz=1_000_000_000)  # 1 GHz
        divider = ClockDivider(master, divisor=4)
        # divider.output runs at 250 MHz

        master.run(8)  # 8 source cycles
        # divider.output has completed 2 cycles (8 / 4 = 2)

    Real-world uses:
    - CPU-to-bus clock ratio (e.g., CPU at 4 GHz, bus at 1 GHz)
    - USB clock derivation from system clock
    - Audio sample rate generation from master clock
    """

    def __init__(self, source: Clock, divisor: int) -> None:
        """Create a clock divider.

        Args:
            source: The faster clock to divide.
            divisor: Division factor (must be >= 2).

        Raises:
            ValueError: If divisor is less than 2.
        """
        if divisor < 2:
            msg = f"Divisor must be >= 2, got {divisor}"
            raise ValueError(msg)
        self.source = source
        self.divisor = divisor
        self.output = Clock(frequency_hz=source.frequency_hz // divisor)
        self._counter = 0
        source.register_listener(self._on_edge)

    def _on_edge(self, edge: ClockEdge) -> None:
        """Called on every source clock edge.

        We only count rising edges. When we have counted `divisor` rising
        edges, we generate one complete output cycle (rising + falling).
        """
        if edge.is_rising:
            self._counter += 1
            if self._counter >= self.divisor:
                self._counter = 0
                self.output.tick()  # rising
                self.output.tick()  # falling


# ---------------------------------------------------------------------------
# MultiPhaseClock -- non-overlapping phase generation
# ---------------------------------------------------------------------------


class MultiPhaseClock:
    """Generates multiple clock phases from a single source.

    Used in CPU pipelines where different stages need offset clocks.
    A 4-phase clock generates 4 non-overlapping clock signals, each
    active for 1/4 of the master cycle.

    Timing diagram for a 4-phase clock:

        Source:  _|^|_|^|_|^|_|^|_
        Phase 0: _|^|___|___|___|_
        Phase 1: _|___|^|___|___|_
        Phase 2: _|___|___|^|___|_
        Phase 3: _|___|___|___|^|_

    On each rising edge of the source, exactly ONE phase is active (1)
    and all others are inactive (0). The active phase rotates.

    Real-world uses:
    - Classic RISC pipelines (fetch, decode, execute, writeback)
    - DRAM refresh timing
    - Multiplexed bus access
    """

    def __init__(self, source: Clock, phases: int = 4) -> None:
        """Create a multi-phase clock.

        Args:
            source: The master clock to derive phases from.
            phases: Number of phases (must be >= 2).

        Raises:
            ValueError: If phases is less than 2.
        """
        if phases < 2:
            msg = f"Phases must be >= 2, got {phases}"
            raise ValueError(msg)
        self.source = source
        self.phases = phases
        self.active_phase = 0
        self.phase_values: list[int] = [0] * phases
        source.register_listener(self._on_edge)

    def _on_edge(self, edge: ClockEdge) -> None:
        """Called on every source clock edge.

        On rising edges, we rotate the active phase. Only one phase
        is high at any time -- this is the "non-overlapping" property
        that prevents pipeline hazards.
        """
        if edge.is_rising:
            self.phase_values = [0] * self.phases
            self.phase_values[self.active_phase] = 1
            self.active_phase = (self.active_phase + 1) % self.phases

    def get_phase(self, index: int) -> int:
        """Get current value of phase N.

        Args:
            index: Phase index (0 to phases-1).

        Returns:
            1 if phase is active, 0 if inactive.
        """
        return self.phase_values[index]
