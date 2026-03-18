"""Sequential Logic — memory elements that give circuits the ability to remember.

=== From Combinational to Sequential ===

The gates in gates.py are "combinational" — their output depends ONLY on the
current inputs. They have no memory. If you remove the input, the output
disappears. This is like a light switch: the light is on only while the switch
is held in the ON position.

Sequential logic is fundamentally different. Sequential circuits can REMEMBER
their previous state. Even after the input changes, the output can persist.
This is what makes computers possible — without memory, there are no variables,
no registers, no stored programs, no state machines.

=== The Key Insight: Feedback ===

Memory arises from FEEDBACK — wiring a gate's output back into its own input.
When you cross-couple two NOR gates (each feeding its output into the other's
input), you create a stable loop that "latches" into one of two states and
stays there. This is the SR Latch, the simplest memory element.

From this single idea, we build the entire memory hierarchy:

    SR Latch          → raw 1-bit memory (2 cross-coupled NOR gates)
    D Latch           → controlled 1-bit memory (SR + enable signal)
    D Flip-Flop       → edge-triggered 1-bit memory (2 D latches)
    Register          → N-bit word storage (N flip-flops in parallel)
    Shift Register    → serial-to-parallel converter (chained flip-flops)
    Counter           → binary counting (register + incrementer)

=== Why This Matters for GPUs ===

GPUs are built on massive parallelism, and every parallel unit needs its own
local storage:

- Registers hold intermediate computation values in shader cores
- Shift registers align mantissas during floating-point addition
- Counters track pipeline stages and warp scheduling
- A modern GPU has millions of flip-flops organized into register files

This module builds each component from the gates defined in gates.py,
showing exactly how physical memory works at the transistor level.
"""

from logic_gates.gates import AND, NOR, NOT, XOR, _validate_bit


# ===========================================================================
# SR LATCH — The Simplest Memory Element
# ===========================================================================
#
# The SR (Set-Reset) Latch is where memory begins. It is built from just
# two NOR gates, cross-coupled so that each gate's output feeds into the
# other gate's input. This feedback loop creates two stable states:
#
#     State "Set":   Q=1, Q_bar=0   (the latch remembers a 1)
#     State "Reset": Q=0, Q_bar=1   (the latch remembers a 0)
#
# Once the latch enters one of these states, it STAYS there even after
# the input that caused it is removed. This is memory.
#
# Circuit diagram:
#
#     Reset ──┐         ┌── Q
#             │  ┌────┐ │
#             ├──┤ NOR ├─┤
#             │  └────┘ │
#             │    ↑    │
#             │    │    │
#             │    ↓    │
#             │  ┌────┐ │
#             ├──┤ NOR ├─┤
#             │  └────┘ │
#     Set   ──┘         └── Q_bar
#
# The cross-coupling (each NOR feeds into the other) is what creates the
# feedback loop. In software, we simulate this by iterating until the
# outputs stabilize.


def sr_latch(
    set_: int, reset: int, q: int = 0, q_bar: int = 1
) -> tuple[int, int]:
    """SR Latch — the fundamental 1-bit memory element.

    Built from two NOR gates feeding back into each other. The feedback
    creates two stable states that persist even after inputs are removed.

    Parameters:
        set_:  When 1, forces output Q to 1 (stores a 1)
        reset: When 1, forces output Q to 0 (stores a 0)
        q:     Current Q output (previous state, default 0)
        q_bar: Current Q_bar output (previous state, default 1)

    Returns:
        (Q, Q_bar) — the new stable state of the latch

    Truth table:
        S  R  | Q    Q_bar  | Action
        ------+-------------+----------------------------------
        0  0  | Q    Q_bar  | Hold — remember previous state
        1  0  | 1    0      | Set — store a 1
        0  1  | 0    1      | Reset — store a 0
        1  1  | 0    0      | Invalid — both outputs forced low

    Why S=1, R=1 is "invalid":
        Both NOR gates receive a 1 input, so both output 0. This means
        Q = Q_bar = 0, which violates the invariant that Q and Q_bar
        should be complements. In real hardware, releasing both inputs
        simultaneously leads to a race condition — the circuit may
        oscillate or settle unpredictably. We still compute it (returning
        0, 0) because that IS what the gates produce, but the caller
        should avoid this combination.

    How the feedback simulation works:
        In real hardware, the two NOR gates evaluate continuously and
        simultaneously. In software, we simulate this by computing both
        gates in a loop until the outputs stop changing (convergence).
        For an SR latch, this always converges in at most 2 iterations.

    Example:
        >>> sr_latch(1, 0)           # Set the latch
        (1, 0)
        >>> sr_latch(0, 0, 1, 0)     # Hold — remembers the 1
        (1, 0)
        >>> sr_latch(0, 1, 1, 0)     # Reset — back to 0
        (0, 1)
    """
    _validate_bit(set_, "set_")
    _validate_bit(reset, "reset")
    _validate_bit(q, "q")
    _validate_bit(q_bar, "q_bar")

    # --- Feedback simulation ---
    # We iterate because the two NOR gates depend on each other's outputs.
    # Each iteration computes both gates using the previous iteration's
    # outputs. We stop when the outputs stabilize (no change between
    # iterations). For an SR latch, this always converges within 2-3
    # iterations because there are no oscillating states (except the
    # invalid S=R=1 case, which converges to (0,0) in 1-2 steps).

    max_iterations = 10  # Safety limit; real convergence happens in 2-3
    for _ in range(max_iterations):
        # Compute new outputs from current state
        #   Q_new     = NOR(Reset, Q_bar_current)
        #   Q_bar_new = NOR(Set,   Q_current)
        new_q = NOR(reset, q_bar)
        new_q_bar = NOR(set_, q)

        # Check for convergence — have the outputs stabilized?
        if new_q == q and new_q_bar == q_bar:
            break

        # Update state for next iteration
        q = new_q
        q_bar = new_q_bar

    return (q, q_bar)


# ===========================================================================
# D LATCH — Controlled Memory
# ===========================================================================
#
# The SR latch has a problem: the caller must carefully manage Set and Reset
# to avoid the invalid S=R=1 state. The D Latch solves this by deriving S
# and R from a single data input D, using a NOT gate to guarantee that S
# and R are always complementary (never both 1 at the same time).
#
# An "enable" signal controls WHEN the latch listens to the data input:
#   - Enable = 1: the latch is "transparent" — output follows input
#   - Enable = 0: the latch is "opaque" — output holds its last value
#
# Circuit diagram:
#
#                     ┌──────────┐
#     Data ──┬────────┤ AND      ├── Set ──┐
#            │        │          │         │    ┌──────────┐
#            │   ┌────┤          │         ├────┤ SR Latch ├── Q
#            │   │    └──────────┘         │    │          │
#     Enable─┼───┤                         │    │          ├── Q_bar
#            │   │    ┌──────────┐         │    └──────────┘
#            │   └────┤ AND      ├── Reset─┘
#            │        │          │
#            └──▷○────┤          │
#              NOT    └──────────┘
#
#     S = AND(Data, Enable)
#     R = AND(NOT(Data), Enable)
#
# Notice: if Data=0 then S=0, R=Enable. If Data=1 then S=Enable, R=0.
# S and R can NEVER both be 1 at the same time. Problem solved!


def d_latch(
    data: int, enable: int, q: int = 0, q_bar: int = 1
) -> tuple[int, int]:
    """D Latch — data latch with enable control.

    When enable=1, the output transparently follows the data input.
    When enable=0, the output holds its previous value regardless of data.

    This is the workhorse of level-sensitive storage. The "D" stands for
    "Data" — there is only one data input, eliminating the invalid state
    problem of the SR latch.

    Parameters:
        data:   The bit value to store (0 or 1)
        enable: When 1, the latch is transparent (output = data).
                When 0, the latch holds its previous state.
        q:      Current Q output (previous state)
        q_bar:  Current Q_bar output (previous state)

    Returns:
        (Q, Q_bar) — the new state of the latch

    Truth table:
        D  E  | Q    Q_bar  | Action
        ------+-------------+----------------------------------
        X  0  | Q    Q_bar  | Hold — latch is opaque
        0  1  | 0    1      | Store 0 — transparent
        1  1  | 1    0      | Store 1 — transparent

    (X means "don't care" — the value doesn't matter)

    Why not just use the D latch everywhere?
        The D latch is "level-sensitive" — it passes data through the entire
        time Enable is high. This causes problems in pipelines where you
        want data captured at a precise INSTANT, not during a whole interval.
        That's why we need the D Flip-Flop (see below).

    Example:
        >>> d_latch(1, 1)           # Enable=1, store the 1
        (1, 0)
        >>> d_latch(0, 0, 1, 0)     # Enable=0, hold the 1
        (1, 0)
        >>> d_latch(0, 1, 1, 0)     # Enable=1, now store the 0
        (0, 1)
    """
    _validate_bit(data, "data")
    _validate_bit(enable, "enable")
    _validate_bit(q, "q")
    _validate_bit(q_bar, "q_bar")

    # Derive Set and Reset from Data and Enable
    #   S = AND(Data, Enable)       — set when data=1 and enabled
    #   R = AND(NOT(Data), Enable)  — reset when data=0 and enabled
    set_ = AND(data, enable)
    reset = AND(NOT(data), enable)

    # Feed into the SR latch with current state
    return sr_latch(set_, reset, q, q_bar)


# ===========================================================================
# D FLIP-FLOP — Edge-Triggered Memory
# ===========================================================================
#
# The D Latch is transparent whenever Enable is high. In a synchronous
# circuit (where everything runs off a shared clock), this transparency
# creates race conditions: data can ripple through multiple latches in
# a single clock cycle if the clock stays high long enough.
#
# The D Flip-Flop solves this with a MASTER-SLAVE configuration:
# two D latches connected in series, with opposite enable signals.
#
#     ┌─────────────────────────────────────────────────────┐
#     │                                                     │
#     │  Clock=0: Master transparent    Clock=1: Slave transparent
#     │  (absorbs new data)             (outputs stored data)
#     │                                                     │
#     │         ┌────────────┐          ┌────────────┐      │
#     │  Data ──┤ D Latch    ├──────────┤ D Latch    ├── Q  │
#     │         │ (Master)   │          │ (Slave)    │      │
#     │  CLK' ──┤ Enable     │   CLK ──┤ Enable     │      │
#     │         └────────────┘          └────────────┘      │
#     │                                                     │
#     │  CLK' = NOT(CLK)                                    │
#     └─────────────────────────────────────────────────────┘
#
# How it works:
#   1. When Clock=0: Master is transparent (captures data), Slave holds
#   2. When Clock=1: Master holds, Slave is transparent (outputs master's value)
#
# The result: data is effectively captured at the RISING EDGE of the clock
# (the transition from 0 to 1). During the entire high period, new data
# cannot pass through because the master is holding. This is "edge-triggered"
# behavior.
#
# Why edge-triggering matters:
#   In a GPU, thousands of operations happen per clock cycle. Edge-triggering
#   ensures every flip-flop samples its input at exactly the same instant,
#   preventing data races between pipeline stages. Without this, a pipeline
#   would be chaos — data from stage 3 could leak into stage 5 before
#   stage 4 finishes processing.


def d_flip_flop(
    data: int,
    clock: int,
    master_q: int = 0,
    master_q_bar: int = 1,
    slave_q: int = 0,
    slave_q_bar: int = 1,
) -> tuple[int, int, dict[str, int]]:
    """D Flip-Flop — captures data on the clock signal, master-slave design.

    The master-slave configuration creates edge-like behavior:
    - When clock=0: master latch is transparent (absorbs data)
    - When clock=1: slave latch is transparent (outputs master's captured value)

    To simulate a rising edge (0 -> 1 transition), call twice:
      1. First with clock=0 (master absorbs data)
      2. Then with clock=1 (slave outputs what master captured)

    Parameters:
        data:          The bit to capture
        clock:         Clock signal (0 or 1)
        master_q:      Master latch Q state
        master_q_bar:  Master latch Q_bar state
        slave_q:       Slave latch Q state
        slave_q_bar:   Slave latch Q_bar state

    Returns:
        (Q, Q_bar, internal_state) where:
        - Q, Q_bar are the flip-flop's output (from the slave latch)
        - internal_state is a dict with master_q, master_q_bar, slave_q,
          slave_q_bar for passing back to the next call

    Example — simulating a rising edge:
        >>> # Clock low: master absorbs data=1
        >>> q, q_bar, state = d_flip_flop(1, 0)
        >>> # Clock high: slave outputs master's value
        >>> q, q_bar, state = d_flip_flop(1, 1, **state)
        >>> q
        1
    """
    _validate_bit(data, "data")
    _validate_bit(clock, "clock")
    _validate_bit(master_q, "master_q")
    _validate_bit(master_q_bar, "master_q_bar")
    _validate_bit(slave_q, "slave_q")
    _validate_bit(slave_q_bar, "slave_q_bar")

    # Master latch: enabled when clock is LOW (NOT clock)
    #   When clock=0, NOT(clock)=1, so master is transparent — absorbs data
    #   When clock=1, NOT(clock)=0, so master holds its value
    not_clock = NOT(clock)
    master_q, master_q_bar = d_latch(data, not_clock, master_q, master_q_bar)

    # Slave latch: enabled when clock is HIGH (clock directly)
    #   When clock=1, slave is transparent — outputs master's stored value
    #   When clock=0, slave holds its value
    slave_q, slave_q_bar = d_latch(master_q, clock, slave_q, slave_q_bar)

    internal_state = {
        "master_q": master_q,
        "master_q_bar": master_q_bar,
        "slave_q": slave_q,
        "slave_q_bar": slave_q_bar,
    }

    return (slave_q, slave_q_bar, internal_state)


# ===========================================================================
# REGISTER — N-Bit Word Storage
# ===========================================================================
#
# A register is simply N flip-flops arranged in parallel, one per bit.
# All flip-flops share the same clock signal, so they all capture their
# data at the same instant.
#
#     Bit 0:  Data[0] ──┤ D-FF ├── Out[0]
#     Bit 1:  Data[1] ──┤ D-FF ├── Out[1]
#     Bit 2:  Data[2] ──┤ D-FF ├── Out[2]
#     ...
#     Bit N:  Data[N] ──┤ D-FF ├── Out[N]
#                         │
#     Clock ──────────────┘ (shared by all flip-flops)
#
# Registers are the workhorses of any processor:
#   - x86 CPUs have 16 general-purpose 64-bit registers (RAX, RBX, ...)
#   - GPUs have thousands of 32-bit registers per streaming multiprocessor
#   - A GPU's register file can be several megabytes in total
#
# Each 32-bit register is literally 32 D flip-flops side by side.


def register(
    data: list[int],
    clock: int,
    state: list[dict[str, int]] | None = None,
    width: int | None = None,
) -> tuple[list[int], list[dict[str, int]]]:
    """N-bit register — stores a binary word on the clock signal.

    Each bit position has its own D flip-flop. All flip-flops share
    the same clock, so the entire word is captured simultaneously.

    Parameters:
        data:   List of bits to store, one per flip-flop position.
                Length determines register width (unless width is given).
        clock:  Clock signal shared by all flip-flops.
        state:  List of internal states from previous call (one dict per
                flip-flop). Pass None for initial state.
        width:  Expected register width. If given, data must match.

    Returns:
        (output_bits, new_state) where:
        - output_bits is a list of Q values from each flip-flop
        - new_state is a list of internal state dicts for chaining

    Example — store and retrieve a 4-bit value:
        >>> # Clock low: flip-flops absorb data
        >>> out, state = register([1, 0, 1, 1], 0)
        >>> # Clock high: flip-flops output stored data
        >>> out, state = register([1, 0, 1, 1], 1, state)
        >>> out
        [1, 0, 1, 1]
    """
    _validate_bit(clock, "clock")

    if not isinstance(data, list):
        msg = "data must be a list of bits"
        raise TypeError(msg)

    if len(data) == 0:
        msg = "data must not be empty"
        raise ValueError(msg)

    for i, bit in enumerate(data):
        _validate_bit(bit, f"data[{i}]")

    if width is not None and len(data) != width:
        msg = f"data length {len(data)} does not match width {width}"
        raise ValueError(msg)

    n = len(data)

    # Initialize state if this is the first call
    if state is None:
        state = [
            {
                "master_q": 0,
                "master_q_bar": 1,
                "slave_q": 0,
                "slave_q_bar": 1,
            }
            for _ in range(n)
        ]

    if len(state) != n:
        msg = f"state length {len(state)} does not match data length {n}"
        raise ValueError(msg)

    # Run each flip-flop independently with the shared clock
    output_bits: list[int] = []
    new_state: list[dict[str, int]] = []

    for i in range(n):
        q, _q_bar, ff_state = d_flip_flop(data[i], clock, **state[i])
        output_bits.append(q)
        new_state.append(ff_state)

    return (output_bits, new_state)


# ===========================================================================
# SHIFT REGISTER — Serial-to-Parallel Conversion
# ===========================================================================
#
# A shift register is a chain of flip-flops where each one's output feeds
# into the next one's input. On each clock cycle, every bit shifts one
# position (left or right), and a new bit enters from the serial input.
#
# Right shift (direction="right"):
#
#     serial_in → [FF_0] → [FF_1] → [FF_2] → ... → [FF_N-1] → serial_out
#
#     Each clock cycle:
#       FF_0 gets serial_in
#       FF_1 gets old FF_0
#       FF_2 gets old FF_1
#       ...
#       serial_out = old FF_N-1
#
# Left shift (direction="left"):
#
#     serial_out ← [FF_0] ← [FF_1] ← [FF_2] ← ... ← [FF_N-1] ← serial_in
#
# Why shift registers matter for floating-point arithmetic:
#   When adding two floating-point numbers, their mantissas must be aligned
#   to the same exponent. If one number has exponent 5 and the other has
#   exponent 3, the smaller number's mantissa must be shifted RIGHT by 2
#   positions. This is done by a barrel shifter, which is built from
#   multiplexers and shift registers.
#
#   Example: 1.5 x 2^5 + 1.25 x 2^3
#   Step 1: Shift 1.25's mantissa right by 2: 0.0125 x 2^5
#   Step 2: Add aligned mantissas: 1.5 + 0.3125 = 1.8125 x 2^5


def shift_register(
    serial_in: int,
    clock: int,
    state: list[dict[str, int]] | None = None,
    width: int = 8,
    direction: str = "right",
) -> tuple[list[int], int, list[dict[str, int]]]:
    """Shift register — shifts bits through a chain of flip-flops.

    On each clock cycle, bits shift one position and a new bit enters
    from the serial input. The bit that falls off the end becomes the
    serial output.

    Parameters:
        serial_in:  Bit to feed into the first position
        clock:      Clock signal
        state:      Internal state from previous call (None for initial)
        width:      Number of bit positions (default 8)
        direction:  "right" shifts bits toward higher indices,
                    "left" shifts bits toward lower indices

    Returns:
        (parallel_out, serial_out, new_state) where:
        - parallel_out: current value of all bit positions
        - serial_out: the bit that was shifted out
        - new_state: internal state for chaining

    Example — shift in three 1s from the right:
        >>> out, sout, st = shift_register(1, 0, width=4)
        >>> out, sout, st = shift_register(1, 1, st, width=4)
        >>> out  # [1, 0, 0, 0] — first 1 entered position 0
        [1, 0, 0, 0]
    """
    _validate_bit(serial_in, "serial_in")
    _validate_bit(clock, "clock")

    if direction not in ("right", "left"):
        msg = f"direction must be 'right' or 'left', got '{direction}'"
        raise ValueError(msg)

    if width < 1:
        msg = f"width must be >= 1, got {width}"
        raise ValueError(msg)

    # Initialize state: all flip-flops start at 0
    if state is None:
        state = [
            {
                "master_q": 0,
                "master_q_bar": 1,
                "slave_q": 0,
                "slave_q_bar": 1,
            }
            for _ in range(width)
        ]

    if len(state) != width:
        msg = f"state length {len(state)} does not match width {width}"
        raise ValueError(msg)

    # Read current parallel output before shifting (from slave_q of each FF)
    current_values = [s["slave_q"] for s in state]

    # Determine the data inputs for each flip-flop based on shift direction
    #
    # Right shift: serial_in → FF[0] → FF[1] → ... → FF[N-1] → serial_out
    #   FF[0] gets serial_in
    #   FF[i] gets current value of FF[i-1]
    #   serial_out = current value of FF[N-1]
    #
    # Left shift: serial_out ← FF[0] ← FF[1] ← ... ← FF[N-1] ← serial_in
    #   FF[N-1] gets serial_in
    #   FF[i] gets current value of FF[i+1]
    #   serial_out = current value of FF[0]

    if direction == "right":
        serial_out = current_values[width - 1]
        data_inputs = [serial_in] + current_values[: width - 1]
    else:  # left
        serial_out = current_values[0]
        data_inputs = current_values[1:] + [serial_in]

    # Clock all flip-flops with their new data inputs
    new_state: list[dict[str, int]] = []
    parallel_out: list[int] = []

    for i in range(width):
        q, _q_bar, ff_state = d_flip_flop(data_inputs[i], clock, **state[i])
        parallel_out.append(q)
        new_state.append(ff_state)

    return (parallel_out, serial_out, new_state)


# ===========================================================================
# COUNTER — Binary Counting
# ===========================================================================
#
# A counter is a register that increments its stored value on each clock
# cycle. It combines storage (register) with arithmetic (incrementer).
#
# The incrementer is built from a chain of half-adders:
#
#     Bit 0: sum = XOR(bit, carry_in=1)    carry_out = AND(bit, carry_in=1)
#     Bit 1: sum = XOR(bit, carry_out_0)   carry_out = AND(bit, carry_out_0)
#     Bit 2: sum = XOR(bit, carry_out_1)   carry_out = AND(bit, carry_out_1)
#     ...
#
# Starting with carry_in=1 means we add 1 to the current value — that's
# incrementing!
#
# When the counter reaches its maximum value (all 1s), the next increment
# wraps around to 0 (overflow). For an 8-bit counter, this means:
#     255 + 1 = 0 (with carry out)
#
# In GPUs, counters are used for:
#   - Pipeline stage tracking (which stage is each instruction in?)
#   - Warp schedulers (round-robin selection of thread warps)
#   - Performance counters (how many instructions executed?)
#   - Loop iteration counting in shader programs


def counter(
    clock: int,
    reset: int = 0,
    state: dict[str, list[int] | list[dict[str, int]]] | None = None,
    width: int = 8,
) -> tuple[list[int], dict[str, list[int] | list[dict[str, int]]]]:
    """Binary counter — increments on each clock cycle.

    Combines a register with an incrementer circuit (chain of half-adders
    starting with carry_in=1).

    Parameters:
        clock:  Clock signal
        reset:  When 1, counter resets to all zeros
        state:  Internal state from previous call (None for initial).
                Contains 'value' (current count as bit list) and
                'ff_state' (flip-flop internal states).
        width:  Number of bits (default 8, max count = 2^width - 1)

    Returns:
        (count_bits, new_state) where:
        - count_bits: current counter value as a list of bits
          (index 0 = least significant bit)
        - new_state: internal state dict for chaining

    Example — count from 0 to 3:
        >>> bits, st = counter(0, width=4)  # Initialize
        >>> bits, st = counter(1, state=st, width=4)  # Tick 1
        >>> bits  # [1, 0, 0, 0] = decimal 1
        [1, 0, 0, 0]
    """
    _validate_bit(clock, "clock")
    _validate_bit(reset, "reset")

    if width < 1:
        msg = f"width must be >= 1, got {width}"
        raise ValueError(msg)

    # Initialize state
    if state is None:
        state = {
            "value": [0] * width,
            "ff_state": [
                {
                    "master_q": 0,
                    "master_q_bar": 1,
                    "slave_q": 0,
                    "slave_q_bar": 1,
                }
                for _ in range(width)
            ],
        }

    current_value: list[int] = list(state["value"])
    ff_state: list[dict[str, int]] = list(state["ff_state"])

    # Reset: force all bits to 0
    if reset == 1:
        next_value = [0] * width
    else:
        # Increment: add 1 using a chain of half-adders
        # A half-adder computes: sum = XOR(a, b), carry = AND(a, b)
        # We chain them with carry_in=1 (adding 1 to the current value)
        next_value: list[int] = []
        carry = 1  # carry_in = 1 means "add 1"
        for i in range(width):
            bit = current_value[i]
            # Half-adder: sum and carry
            sum_bit = XOR(bit, carry)
            carry = AND(bit, carry)
            next_value.append(sum_bit)
        # If carry=1 after the last bit, the counter overflows to 0
        # (which is exactly what next_value already contains — all the
        # XOR results with the carry propagated through)

    # Store the new value in the register
    output, new_ff_state = register(next_value, clock, ff_state, width)

    # Only update the stored value when the register captures it (clock=1).
    # On clock=0, the master latch is transparent but slave holds — so the
    # counter value hasn't actually changed yet.
    stored_value = next_value if clock == 1 else current_value

    new_state: dict[str, list[int] | list[dict[str, int]]] = {
        "value": stored_value,
        "ff_state": new_ff_state,
    }

    return (output, new_state)
