"""Combinational Circuits — building blocks between primitive gates and full arithmetic.

=== What are combinational circuits? ===

Combinational circuits produce outputs that depend ONLY on the current inputs —
no memory, no state, no clock. They are built entirely from the primitive gates
defined in gates.py (AND, OR, NOT, XOR, etc.).

These circuits fill the gap between individual gates and the ALU:

    Primitive gates (gates.py)
        │
    Combinational circuits (THIS MODULE)
        │  MUX, DEMUX, decoder, encoder, tri-state buffer
        │
    Arithmetic circuits (arithmetic package)
        │  half adder, full adder, ALU
        │
    CPU, FPGA, memory controllers
        │  everything above uses these building blocks

=== Why these circuits matter ===

- **MUX (Multiplexer)**: The selector switch of digital logic. A K-input LUT in
  an FPGA is literally a 2^K-to-1 MUX with SRAM storing the truth table. CPUs use
  MUXes to select between register outputs, ALU inputs, and forwarded values.

- **DEMUX (Demultiplexer)**: Routes one signal to one of many destinations.
  Used in memory write addressing and bus arbitration.

- **Decoder**: Converts binary addresses into one-hot select lines. Every memory
  chip has a row decoder that activates exactly one word line based on the address.

- **Encoder / Priority Encoder**: The inverse of a decoder. Priority encoders
  are the heart of interrupt controllers — when multiple interrupts fire
  simultaneously, the priority encoder picks the most important one.

- **Tri-state buffer**: Enables shared buses by letting devices "disconnect"
  from the wire when they're not talking. Without tri-state buffers, you'd need
  separate wires for every device pair.
"""

from __future__ import annotations

from logic_gates.gates import (
    AND,
    NOT,
    OR,
    _safe_int_text,
    _safe_type_name,
    _validate_bit,
)

# ---------------------------------------------------------------------------
# Size caps (denial-of-service defence)
# ---------------------------------------------------------------------------
#
# A decoder with N select bits materialises 2^N outputs, and each output costs
# O(N) gate evaluations.  If N comes from caller data, ``decoder([0] * 40)``
# asks for a trillion-element list and the process dies of memory exhaustion.
# The same is true of ``demux(..., n_outputs=2**40)`` and of wide encoders.
#
# So every size is checked against a cap BEFORE anything is allocated:
#
#   MAX_SELECT_BITS   = 16      -> at most 2^16 = 65,536 decoder/demux/mux lines
#   MAX_WIDTH         = 1 << 16 -> at most 65,536 elements in any bit vector
#   MAX_COUNTER_WIDTH = 64      -> counters (see sequential.py) are <= 64 bits
#
# 16 select bits is far beyond any real gate-level structure (a 16:1 mux has
# 4 select bits) and matches the Rust, Go and TypeScript ports.
MAX_SELECT_BITS = 16
MAX_WIDTH = 1 << 16
MAX_COUNTER_WIDTH = 64


def _require_bit_list(value: object, name: str, *, min_len: int = 0) -> list[int]:
    """Require ``value`` to be exactly a ``list`` of valid bits, <= MAX_WIDTH.

    ``type(value) is list`` (not isinstance) so that dicts, tuples, sets,
    strings and list subclasses with hostile ``__len__``/``__getitem__`` are
    rejected with TypeError before any of their contents are touched.
    Returns a fresh validated copy.
    The length cap is checked before the per-element loop so an oversize
    list is rejected in O(1).
    """
    if type(value) is not list:
        msg = f"{name} must be a list of bits, got {_safe_type_name(value)}"
        raise TypeError(msg)
    if len(value) > MAX_WIDTH:
        msg = f"{name} has {len(value)} elements; maximum is {MAX_WIDTH}"
        raise ValueError(msg)
    if min_len == 1 and len(value) == 0:
        msg = f"{name} must not be empty"
        raise ValueError(msg)
    if len(value) < min_len:
        msg = f"{name} must have at least {min_len} elements, got {len(value)}"
        raise ValueError(msg)
    for i, bit in enumerate(value):
        _validate_bit(bit, f"{name}[{i}]")
    # Return a fresh plain list of canonical ints so callers compute only on
    # the validated copy, never on the caller's (mutable) original.
    return [int(bit) for bit in value]


# ===========================================================================
# MULTIPLEXER (MUX) — The Selector Switch
# ===========================================================================
#
# A multiplexer takes N data inputs and a set of select lines, and routes
# exactly one input to the output. Think of it as a railroad switch that
# directs one of several trains onto a single track.
#
# The number of select lines determines how many inputs can be selected:
#   1 select line  → 2 inputs  (2:1 MUX)
#   2 select lines → 4 inputs  (4:1 MUX)
#   3 select lines → 8 inputs  (8:1 MUX)
#   N select lines → 2^N inputs (2^N:1 MUX)
#
# Every larger MUX can be built recursively from 2:1 MUXes:
#   4:1  = two 2:1 MUXes feeding a third 2:1 MUX
#   8:1  = two 4:1 MUXes feeding a 2:1 MUX
#   16:1 = two 8:1 MUXes feeding a 2:1 MUX
#
# This recursive structure is exactly how FPGA look-up tables (LUTs) work:
# a 4-input LUT is a 16:1 MUX tree with the truth table stored in SRAM.


def mux2(d0: int, d1: int, sel: int) -> int:
    """2-to-1 Multiplexer — the simplest selector circuit.

    Routes one of two data inputs to the output based on a select signal.

    Circuit:
        d0 ──┐
             │──── output
        d1 ──┘
              ↑
        sel ──┘

    Built from gates:
        output = OR(AND(d0, NOT(sel)), AND(d1, sel))

    When sel=0, the NOT(sel)=1 enables d0 through the top AND gate.
    When sel=1, sel itself enables d1 through the bottom AND gate.

    Truth table:
        sel  │ output
        ─────┼───────
         0   │  d0
         1   │  d1

    Parameters:
        d0:  Data input 0 (selected when sel=0)
        d1:  Data input 1 (selected when sel=1)
        sel: Select line (0 or 1)

    Returns:
        The selected data input value (0 or 1)

    Example:
        >>> mux2(0, 1, 0)  # sel=0, select d0=0
        0
        >>> mux2(0, 1, 1)  # sel=1, select d1=1
        1
    """
    _validate_bit(d0, "d0")
    _validate_bit(d1, "d1")
    _validate_bit(sel, "sel")

    # output = OR(AND(d0, NOT(sel)), AND(d1, sel))
    #
    # When sel=0: NOT(sel)=1, so AND(d0, 1)=d0; AND(d1, 0)=0 → OR(d0, 0) = d0
    # When sel=1: NOT(sel)=0, so AND(d0, 0)=0; AND(d1, 1)=d1 → OR(0, d1) = d1
    return OR(AND(d0, NOT(sel)), AND(d1, sel))


def mux4(d0: int, d1: int, d2: int, d3: int, sel: list[int]) -> int:
    """4-to-1 Multiplexer — selects one of four inputs using 2 select lines.

    Built from three 2:1 MUXes arranged in a tree:

        d0 ──┐                     sel[0] controls first level
             MUX ── r0 ──┐
        d1 ──┘             │        sel[1] controls second level
                           MUX ── output
        d2 ──┐             │
             MUX ── r1 ──┘
        d3 ──┘

    Truth table:
        sel[1] sel[0] │ output
        ───────────────┼───────
          0      0     │  d0
          0      1     │  d1
          1      0     │  d2
          1      1     │  d3

    Parameters:
        d0, d1, d2, d3: Four data inputs (each 0 or 1)
        sel: List of 2 select bits [s0, s1] (LSB first)

    Returns:
        The selected data input value (0 or 1)

    Example:
        >>> mux4(1, 0, 0, 0, [0, 0])  # sel=00, select d0=1
        1
        >>> mux4(0, 0, 0, 1, [1, 1])  # sel=11, select d3=1
        1
    """
    _validate_bit(d0, "d0")
    _validate_bit(d1, "d1")
    _validate_bit(d2, "d2")
    _validate_bit(d3, "d3")

    # Exact-list check plus validated snapshot: compute only from the copy.
    sel = _require_bit_list(sel, "sel")
    if len(sel) != 2:
        msg = "sel must be a list of exactly 2 bits"
        raise ValueError(msg)

    # First level: sel[0] selects within each pair
    r0 = mux2(d0, d1, sel[0])
    r1 = mux2(d2, d3, sel[0])

    # Second level: sel[1] selects between the two pairs
    return mux2(r0, r1, sel[1])


def mux8(inputs: list[int], sel: list[int]) -> int:
    """8-to-1 Multiplexer — selects one of eight inputs using 3 select lines.

    Built from two 4:1 MUXes feeding a 2:1 MUX:

        inputs[0:4] → [4:1 MUX] ── r0 ──┐
                                          MUX ── output
        inputs[4:8] → [4:1 MUX] ── r1 ──┘
                                           ↑
                                         sel[2]

    Parameters:
        inputs: List of exactly 8 data inputs (each 0 or 1)
        sel: List of 3 select bits [s0, s1, s2] (LSB first)

    Returns:
        The selected data input value (0 or 1)

    Example:
        >>> mux8([0, 0, 0, 0, 0, 0, 0, 1], [1, 1, 1])  # sel=111, select inputs[7]=1
        1
    """
    inputs = _require_bit_list(inputs, "inputs")
    if len(inputs) != 8:
        msg = "inputs must be a list of exactly 8 bits"
        raise ValueError(msg)

    sel = _require_bit_list(sel, "sel")
    if len(sel) != 3:
        msg = "sel must be a list of exactly 3 bits"
        raise ValueError(msg)

    # Lower 4 inputs selected by sel[0:2]
    r0 = mux4(inputs[0], inputs[1], inputs[2], inputs[3], sel[0:2])
    # Upper 4 inputs selected by sel[0:2]
    r1 = mux4(inputs[4], inputs[5], inputs[6], inputs[7], sel[0:2])

    # sel[2] selects between lower and upper halves
    return mux2(r0, r1, sel[2])


def mux_n(inputs: list[int], sel: list[int]) -> int:
    """N-to-1 Multiplexer — selects one of N inputs using log2(N) select lines.

    N must be a power of 2 (2, 4, 8, 16, 32, 64, ...).

    Built recursively: split inputs in half, recurse on each half with
    sel[:-1], then use a 2:1 MUX with sel[-1] to pick between the two halves.

    This recursive construction is exactly how FPGA look-up tables work:
    a K-input LUT is a 2^K-to-1 MUX tree.

    Parameters:
        inputs: List of N data inputs (N must be power of 2, N >= 2)
        sel: List of log2(N) select bits (LSB first)

    Returns:
        The selected data input value (0 or 1)

    Example:
        >>> # 16:1 MUX — select input 5 (binary 0101)
        >>> data = [0] * 16
        >>> data[5] = 1
        >>> mux_n(data, [1, 0, 1, 0])  # sel=0101 LSB-first → index 5
        1
    """
    _require_bit_list(inputs, "inputs", min_len=2)
    n = len(inputs)

    # Check power of 2: a number is a power of 2 if it has exactly one bit set
    if n & (n - 1) != 0:
        msg = f"inputs length must be a power of 2, got {n}"
        raise ValueError(msg)

    expected_sel_bits = n.bit_length() - 1
    if type(sel) is not list:
        msg = f"sel must be a list of bits, got {_safe_type_name(sel)}"
        raise TypeError(msg)
    if expected_sel_bits > MAX_SELECT_BITS or len(sel) != expected_sel_bits:
        msg = (
            f"sel must be a list of {expected_sel_bits} bits "
            f"for {n} inputs, got {len(sel)}"
        )
        raise ValueError(msg)

    for i, bit in enumerate(sel):
        _validate_bit(bit, f"sel[{i}]")

    # Base case: 2:1 MUX
    if n == 2:
        return mux2(inputs[0], inputs[1], sel[0])

    # Recursive case: split in half, recurse, combine with 2:1 MUX
    half = n // 2
    lower = _mux_n_inner(inputs[:half], sel[:-1])
    upper = _mux_n_inner(inputs[half:], sel[:-1])
    return mux2(lower, upper, sel[-1])


def _mux_n_inner(inputs: list[int], sel: list[int]) -> int:
    """Inner recursive helper for mux_n — skips validation (already done)."""
    n = len(inputs)
    if n == 2:
        return mux2(inputs[0], inputs[1], sel[0])

    half = n // 2
    lower = _mux_n_inner(inputs[:half], sel[:-1])
    upper = _mux_n_inner(inputs[half:], sel[:-1])
    return mux2(lower, upper, sel[-1])


# ===========================================================================
# DEMULTIPLEXER (DEMUX) — The Inverse of MUX
# ===========================================================================
#
# A demultiplexer takes one data input and routes it to one of N outputs.
# The select lines determine which output receives the data; all other
# outputs are 0.
#
# Think of it as an address decoder that also carries data: the decoder
# picks which output line is active, and the data signal determines
# whether that line is 0 or 1.


def demux(data: int, sel: list[int], n_outputs: int) -> list[int]:
    """1-to-N Demultiplexer — routes one input to one of N outputs.

    The selected output receives the data value; all other outputs are 0.

    Built from a decoder + AND gates:
        1. Decoder converts sel bits into one-hot (exactly one output = 1)
        2. AND each decoder output with the data input

    1-to-4 DEMUX truth table:
        sel[1] sel[0]  data │ y0  y1  y2  y3
        ─────────────────────┼─────────────────
          0      0      0   │  0   0   0   0
          0      0      1   │  1   0   0   0
          0      1      0   │  0   0   0   0
          0      1      1   │  0   1   0   0
          1      0      0   │  0   0   0   0
          1      0      1   │  0   0   1   0
          1      1      0   │  0   0   0   0
          1      1      1   │  0   0   0   1

    Parameters:
        data:      The data bit to route (0 or 1)
        sel:       List of select bits (LSB first), length = log2(n_outputs)
        n_outputs: Number of outputs (must be power of 2, >= 2)

    Returns:
        List of n_outputs bits. Exactly one equals data, rest are 0.

    Example:
        >>> demux(1, [1, 0], 4)  # sel=01, route data=1 to output 1
        [0, 1, 0, 0]
    """
    _validate_bit(data, "data")

    if type(n_outputs) is not int:
        msg = f"n_outputs must be an int, got {_safe_type_name(n_outputs)}"
        raise TypeError(msg)

    # Cap BEFORE any allocation: n_outputs is caller-chosen.
    if n_outputs < 2 or (n_outputs & (n_outputs - 1)) != 0:
        msg = f"n_outputs must be a power of 2 >= 2, got {_safe_int_text(n_outputs)}"
        raise ValueError(msg)
    if n_outputs > MAX_WIDTH:
        msg = f"n_outputs {_safe_int_text(n_outputs)} exceeds maximum {MAX_WIDTH}"
        raise ValueError(msg)

    expected_sel_bits = n_outputs.bit_length() - 1
    if type(sel) is not list:
        msg = f"sel must be a list of bits, got {_safe_type_name(sel)}"
        raise TypeError(msg)
    if len(sel) != expected_sel_bits:
        msg = f"sel must be a list of {expected_sel_bits} bits for {n_outputs} outputs"
        raise ValueError(msg)

    for i, bit in enumerate(sel):
        _validate_bit(bit, f"sel[{i}]")

    # Use decoder to get one-hot output, then AND each with data
    decoded = decoder(sel)
    return [AND(decoded[i], data) for i in range(n_outputs)]


# ===========================================================================
# DECODER — Binary to One-Hot
# ===========================================================================
#
# A decoder converts an N-bit binary input into a one-hot output:
# exactly one of 2^N output lines is 1, the rest are 0.
#
# It's essentially a DEMUX with data hardwired to 1.
#
# Decoders are fundamental to memory addressing: the row decoder in an SRAM
# chip takes the address bits and activates exactly one word line, enabling
# read/write access to that row of cells.
#
# Construction: each output Y_i is an AND of all N input bits (or their
# complements), corresponding to the binary representation of i.
#
# Example for 2-to-4:
#   Y0 = AND(NOT(A1), NOT(A0))  — active when input = 00
#   Y1 = AND(NOT(A1), A0)       — active when input = 01
#   Y2 = AND(A1, NOT(A0))       — active when input = 10
#   Y3 = AND(A1, A0)            — active when input = 11


def decoder(inputs: list[int]) -> list[int]:
    """N-to-2^N Decoder — converts binary input to one-hot output.

    For an N-bit input, produces 2^N outputs where exactly one is 1.
    The output at index i is 1 when the input represents the binary
    value i.

    2-to-4 Decoder truth table:
        A1  A0  │ Y0  Y1  Y2  Y3
        ────────┼─────────────────
         0   0  │  1   0   0   0
         0   1  │  0   1   0   0
         1   0  │  0   0   1   0
         1   1  │  0   0   0   1

    Built from AND and NOT gates:
        Y0 = AND(NOT(A1), NOT(A0))
        Y1 = AND(NOT(A1), A0)
        Y2 = AND(A1, NOT(A0))
        Y3 = AND(A1, A0)

    Parameters:
        inputs: List of N input bits (LSB first). N >= 1.

    Returns:
        List of 2^N bits, exactly one of which is 1 (one-hot encoding).

    Example:
        >>> decoder([1, 0])  # input = 01 (binary) → output index 1
        [0, 1, 0, 0]
        >>> decoder([0, 0, 0])  # input = 000 → output index 0
        [1, 0, 0, 0, 0, 0, 0, 0]
    """
    # 2^N outputs are allocated below, so N is capped first (MAX_SELECT_BITS).
    if type(inputs) is list and len(inputs) > MAX_SELECT_BITS:
        msg = (
            f"decoder supports at most {MAX_SELECT_BITS} input bits, "
            f"got {len(inputs)}"
        )
        raise ValueError(msg)
    _require_bit_list(inputs, "inputs", min_len=1)
    n = len(inputs)
    n_outputs = 1 << n  # 2^n

    # Precompute complements once
    complements = [NOT(b) for b in inputs]

    outputs: list[int] = []
    for i in range(n_outputs):
        # Output i is the AND of all input bits where the bit corresponding
        # to the binary representation of i is taken directly, and the rest
        # are complemented.
        #
        # For i=5 (binary 101) with 3 inputs [A0, A1, A2]:
        #   Y5 = AND(A0, NOT(A1), A2)
        #   because 5 in binary is: bit0=1, bit1=0, bit2=1
        result = 1
        for bit_pos in range(n):
            if (i >> bit_pos) & 1:
                # This bit position is 1 in i's binary representation
                result = AND(result, inputs[bit_pos])
            else:
                # This bit position is 0 — use the complement
                result = AND(result, complements[bit_pos])
        outputs.append(result)

    return outputs


# ===========================================================================
# ENCODER — One-Hot to Binary
# ===========================================================================
#
# The inverse of a decoder: takes a one-hot input (exactly one bit is 1)
# and produces the binary index of that bit.
#
# If input bit 5 is active (out of 8 inputs), the encoder outputs 101
# (the binary representation of 5).


def encoder(inputs: list[int]) -> list[int]:
    """2^N-to-N Encoder — converts one-hot input to binary output.

    Exactly one input bit must be 1. The output is the binary
    representation of the index of that active bit.

    4-to-2 Encoder truth table:
        I0  I1  I2  I3  │ A1  A0
        ────────────────┼────────
         1   0   0   0  │  0   0
         0   1   0   0  │  0   1
         0   0   1   0  │  1   0
         0   0   0   1  │  1   1

    Parameters:
        inputs: List of 2^N bits in one-hot encoding (exactly one must be 1).
                Length must be a power of 2, >= 2.

    Returns:
        List of N bits representing the binary index of the active input (LSB first).

    Raises:
        ValueError: If input is not valid one-hot (zero or multiple bits set).

    Example:
        >>> encoder([0, 0, 1, 0])  # I2 active → binary 10 → [0, 1]
        [0, 1]
    """
    _require_bit_list(inputs, "inputs")
    n_inputs = len(inputs)
    if n_inputs < 2 or (n_inputs & (n_inputs - 1)) != 0:
        msg = f"inputs length must be a power of 2 >= 2, got {n_inputs}"
        raise ValueError(msg)

    # Validate one-hot: exactly one bit must be 1
    active_count = sum(inputs)
    if active_count != 1:
        msg = (
            "inputs must be one-hot (exactly one bit = 1), "
            f"got {active_count} active bits"
        )
        raise ValueError(msg)

    n_output_bits = n_inputs.bit_length() - 1

    # Find the active index
    active_index = inputs.index(1)

    # Convert to binary (LSB first)
    output: list[int] = []
    for bit_pos in range(n_output_bits):
        output.append((active_index >> bit_pos) & 1)

    return output


# ===========================================================================
# PRIORITY ENCODER — Multiple Inputs, Highest Wins
# ===========================================================================
#
# A regular encoder requires exactly one active input (one-hot). In real
# systems, multiple signals can be active simultaneously — for example,
# multiple interrupt lines firing at the same time.
#
# The priority encoder solves this: it outputs the binary index of the
# HIGHEST-PRIORITY active input. Priority is determined by index — the
# highest index has the highest priority.
#
# It also outputs a "valid" flag that indicates whether ANY input is active.
# This distinguishes "no input active" from "input 0 is active" (both would
# produce output 00 without the valid flag).


def priority_encoder(inputs: list[int]) -> tuple[list[int], int]:
    """Priority encoder — encodes the highest-priority active input.

    When multiple inputs are active, the one with the highest index wins.
    A "valid" output indicates whether any input is active at all.

    4-to-2 Priority Encoder truth table:
        I0  I1  I2  I3  │ A1  A0  Valid
        ────────────────┼─────────────
         0   0   0   0  │  0   0    0     No input active
         1   0   0   0  │  0   0    1     I0 wins (only one)
         X   1   0   0  │  0   1    1     I1 wins over I0
         X   X   1   0  │  1   0    1     I2 wins over I0,I1
         X   X   X   1  │  1   1    1     I3 always wins

    Parameters:
        inputs: List of 2^N input bits. Length must be a power of 2, >= 2.

    Returns:
        (binary_output, valid) where:
        - binary_output: List of N bits (LSB first) — index of highest active input
        - valid: 1 if any input is active, 0 if all inputs are 0

    Example:
        >>> priority_encoder([1, 0, 1, 0])  # I0 and I2 active, I2 wins
        ([0, 1], 1)
        >>> priority_encoder([0, 0, 0, 0])  # No input active
        ([0, 0], 0)
    """
    _require_bit_list(inputs, "inputs")
    n_inputs = len(inputs)
    if n_inputs < 2 or (n_inputs & (n_inputs - 1)) != 0:
        msg = f"inputs length must be a power of 2 >= 2, got {n_inputs}"
        raise ValueError(msg)

    n_output_bits = n_inputs.bit_length() - 1

    # Scan from highest index to lowest — first active input wins
    highest_active = -1
    for i in range(n_inputs - 1, -1, -1):
        if inputs[i] == 1:
            highest_active = i
            break

    # Valid flag: 1 if any input was active
    valid = 0 if highest_active == -1 else 1

    # Convert active index to binary (LSB first)
    # If no input is active, output all zeros
    index = max(highest_active, 0)
    output: list[int] = []
    for bit_pos in range(n_output_bits):
        output.append((index >> bit_pos) & 1)

    return (output, valid)


# ===========================================================================
# TRI-STATE BUFFER — Three Output States
# ===========================================================================
#
# Normal gates have two possible outputs: 0 or 1. A tri-state buffer adds
# a third state: HIGH-IMPEDANCE (Z), which means the output is electrically
# disconnected — as if the wire were cut.
#
# This is essential for shared buses. In a computer, the data bus connects
# the CPU, memory, and I/O devices on the same wires. Only one device can
# drive the bus at a time. Tri-state buffers let each device disconnect
# when it's not its turn, preventing electrical conflicts.
#
# In FPGAs, tri-state buffers appear in I/O blocks where pins can be
# configured as inputs (high-Z) or outputs (driven).
#
# We represent high-impedance as None in Python:
#   - enable=1: output = data (0 or 1)
#   - enable=0: output = None (high-Z, disconnected)


def tri_state(data: int, enable: int) -> int | None:
    """Tri-state buffer — output can be 0, 1, or high-impedance (None).

    When enabled, the buffer passes the data input through to the output.
    When disabled, the output is high-impedance (None) — electrically
    disconnected from the wire.

    Truth table:
        data  enable │ output
        ─────────────┼───────
          0      0   │  None    (high-Z, disconnected)
          1      0   │  None    (high-Z, disconnected)
          0      1   │   0      (driving low)
          1      1   │   1      (driving high)

    Parameters:
        data:   The data bit to pass through (0 or 1)
        enable: When 1, buffer is active. When 0, output is high-Z (None).

    Returns:
        data value (int) when enabled, None when disabled (high-impedance).

    Example:
        >>> tri_state(1, 1)  # Enabled → pass data through
        1
        >>> tri_state(1, 0)  # Disabled → high-impedance
        None
        >>> tri_state(0, 1)  # Enabled → pass data through
        0
    """
    _validate_bit(data, "data")
    _validate_bit(enable, "enable")

    if enable == 0:
        return None

    return data
