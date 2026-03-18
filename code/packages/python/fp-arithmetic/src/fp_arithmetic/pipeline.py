"""Pipelined floating-point arithmetic — the bridge to GPU architecture.

=== Why Pipelining? ===

Imagine a car factory with a single worker who does everything: welds the
frame, installs the engine, paints the body, mounts the wheels, inspects
the result. One car takes 5 hours. Want 100 cars? That's 500 hours.

Now imagine a factory with 5 stations, each doing one step. The first car
still takes 5 hours to pass through all 5 stations. But while it moves to
station 2, a NEW car enters station 1. After the initial 5-hour fill-up
time, a finished car rolls off the line every HOUR — 5x throughput!

This is pipelining, and it's exactly how GPUs achieve massive throughput.

=== Latency vs Throughput ===

These two concepts are often confused, but they're fundamentally different:

    Latency:     Time for ONE operation to complete start-to-finish.
    Throughput:  How many operations complete per unit time.

For a 5-stage pipeline:

    Latency = 5 clock cycles (one operation still takes 5 cycles)
    Throughput = 1 result per clock cycle (after pipeline fills up)

    Without pipeline:   Latency=5, Throughput=1/5
    With pipeline:      Latency=5, Throughput=1/1   ← 5x better!

This is the key insight: pipelining does NOT make individual operations
faster (same latency), but it makes the system process MORE operations
per second (higher throughput).

=== Pipeline Timing Diagram ===

Here's what happens when we submit 4 additions (A, B, C, D) to a
5-stage pipelined adder:

    Clock:  1    2    3    4    5    6    7    8
    ────────────────────────────────────────────
    Stage1: [A1] [B1] [C1] [D1]  -    -    -    -
    Stage2:  -   [A2] [B2] [C2] [D2]  -    -    -
    Stage3:  -    -   [A3] [B3] [C3] [D3]  -    -
    Stage4:  -    -    -   [A4] [B4] [C4] [D4]  -
    Stage5:  -    -    -    -   [A5] [B5] [C5] [D5]
                                 ↑    ↑    ↑    ↑
                              Result Result Result Result
                              for A  for B  for C  for D

    - A enters stage 1 at clock 1, exits stage 5 at clock 5 (latency = 5)
    - After clock 5, results come out every cycle (throughput = 1/cycle)
    - All 4 results done by clock 8 instead of clock 20 (without pipeline)

=== How This Connects to GPUs ===

A modern GPU has thousands of "CUDA cores" (NVIDIA) or "shader processors"
(AMD), and each one contains pipelined FP units. A typical GPU core has:

    - Pipelined FP32 adder (4-6 stages)
    - Pipelined FP32 multiplier (3-5 stages)
    - Pipelined FMA unit (6-8 stages)

With 5000 cores each running pipelined FP, the GPU can sustain:
    5000 cores x 1 result/cycle x 1.5 GHz = 7.5 TFLOPS

This is why GPUs dominate machine learning: the dot products in matrix
multiplication map perfectly to pipelined FMA units.

=== Clock-Driven Pipeline Registers ===

Between each stage, there's a set of "pipeline registers" — flip-flops
that capture the intermediate results on the rising edge of the clock.
These registers serve two critical purposes:

    1. ISOLATION: They prevent combinational logic from one stage from
       "bleeding through" to the next stage. Without them, a long chain
       of logic would have to settle within one clock cycle.

    2. SYNCHRONIZATION: All stages advance simultaneously on the clock
       edge. Stage 3 gets stage 2's result from LAST cycle, not the
       current cycle. This is what allows different operations to
       occupy different stages at the same time.

In our simulation, the Clock object fires its listeners on each edge,
and our pipeline's _on_clock_edge method shifts data between stages.
This mirrors exactly how hardware pipeline registers work — they're
D flip-flops clocked by the system clock.

    Clock: ──┐ ┌──┐ ┌──┐ ┌──┐ ┌──┐ ┌──
             └─┘  └─┘  └─┘  └─┘  └─┘

    Stage 1: [Unpack    ] → FF →
    Stage 2:               [Align     ] → FF →
    Stage 3:                             [Add/Sub   ] → FF →
    Stage 4:                                           [Normalize] → FF →
    Stage 5:                                                        [Round/Pack]
                                                                         ↓
                                                                      result
"""

from __future__ import annotations

from typing import Any

from clock import Clock, ClockEdge

from fp_arithmetic.formats import FP32, FloatBits, FloatFormat
from fp_arithmetic.fp_adder import fp_add
from fp_arithmetic.fp_multiplier import fp_mul
from fp_arithmetic.fma import fp_fma
from fp_arithmetic.ieee754 import (
    _bits_msb_to_int,
    _int_to_bits_msb,
    bits_to_float,
    float_to_bits,
    is_inf,
    is_nan,
    is_zero,
)


# ---------------------------------------------------------------------------
# PipelinedFPAdder — 5-stage pipelined floating-point adder
# ---------------------------------------------------------------------------


class PipelinedFPAdder:
    """A 5-stage pipelined floating-point adder driven by a clock.

    In real GPU hardware, the FP adder is pipelined so that while one
    addition is being normalized (stage 4), a newer addition is being
    aligned (stage 2), and an even newer one is being unpacked (stage 1).

    === Pipeline Stages ===

        Stage 1: UNPACK
            Extract sign, exponent, and mantissa from both operands.
            Add the implicit leading 1 for normal numbers.
            Handle special cases (NaN, Inf, zero).

        Stage 2: ALIGN
            Compare exponents and shift the smaller mantissa right
            so both operands are aligned to the same power of 2.

        Stage 3: ADD/SUB
            Perform mantissa addition or subtraction depending on
            the signs of the operands.

        Stage 4: NORMALIZE
            Shift the result so the leading 1 is in the correct
            position. Adjust the exponent accordingly.

        Stage 5: ROUND & PACK
            Apply IEEE 754 round-to-nearest-even and pack the
            result back into a FloatBits.

    === Usage ===

        clock = Clock()
        adder = PipelinedFPAdder(clock)

        a = float_to_bits(1.5)
        b = float_to_bits(2.5)
        adder.submit(a, b)

        # Tick 5 full cycles for the result to emerge
        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])  # 4.0
    """

    # Number of pipeline stages — this is the latency in clock cycles.
    # Real FP adders use 4-7 stages depending on the architecture.
    NUM_STAGES = 5

    def __init__(self, clock: Clock, fmt: FloatFormat = FP32) -> None:
        self.clock = clock
        self.fmt = fmt

        # Pipeline stage registers: each slot holds the intermediate data
        # being processed by that stage, or None if the stage is empty.
        # In hardware, these would be banks of D flip-flops.
        self._stages: list[Any] = [None] * self.NUM_STAGES

        # Input queue: operand pairs waiting to enter the pipeline.
        # In a real GPU, this would be a dispatch queue or instruction buffer.
        self._inputs_pending: list[tuple[FloatBits, FloatBits]] = []

        # Output: completed results that have exited the pipeline.
        self.results: list[FloatBits] = []

        # Cycle counter: how many rising edges we've seen.
        self.cycle_count: int = 0

        # Register with the clock so we advance automatically on edges.
        # This mirrors how hardware components are physically connected
        # to the clock distribution network.
        clock.register_listener(self._on_clock_edge)

    def submit(self, a: FloatBits, b: FloatBits) -> None:
        """Submit a new addition to the pipeline.

        The operands are queued and will enter stage 1 on the next rising
        clock edge. In hardware, this is the dispatch unit loading operands
        into the pipeline's input register.

        Args:
            a: First operand.
            b: Second operand. Must use the same FloatFormat as a.
        """
        self._inputs_pending.append((a, b))

    def _on_clock_edge(self, edge: ClockEdge) -> None:
        """Advance the pipeline on rising clock edges.

        This is the heart of the pipeline simulation. On every rising edge:

            1. Collect output from stage 5 (if any)
            2. Shift all stages forward: stage[i] = process(stage[i-1])
            3. Load new input into stage 1 (if any is pending)

        The order matters: we must read the last stage's output BEFORE
        shifting, otherwise we'd overwrite it.

        In hardware, all of this happens simultaneously because the
        flip-flops capture inputs and produce outputs on the same edge.
        In software, we simulate this with sequential reads and writes.
        """
        if not edge.is_rising:
            return

        self.cycle_count += 1

        # --- Shift pipeline forward ---
        # Each stage receives the previous stage's output and processes it.
        # We shift from the end to avoid overwriting data we haven't read yet.
        #
        #   Before: [S0_data] [S1_data] [S2_data] [S3_data] [S4_data]
        #   After:  [new_in]  [S0_proc] [S1_proc] [S2_proc] [S3_proc]
        for i in range(self.NUM_STAGES - 1, 0, -1):
            self._stages[i] = self._process_stage(i, self._stages[i - 1])

        # --- Load new input into stage 0 ---
        if self._inputs_pending:
            a, b = self._inputs_pending.pop(0)
            self._stages[0] = self._process_stage(0, (a, b))
        else:
            self._stages[0] = None

        # --- Collect output from the last stage ---
        # If the last stage has data after shifting, it's a completed result.
        # Move it to the results list and clear the stage so it isn't
        # collected again on the next cycle. In hardware, this would go
        # to the write-back bus on the same clock edge that produced it.
        if self._stages[self.NUM_STAGES - 1] is not None:
            self.results.append(self._stages[self.NUM_STAGES - 1])
            self._stages[self.NUM_STAGES - 1] = None

    def _process_stage(self, stage_num: int, input_data: Any) -> Any:
        """Execute one pipeline stage's logic.

        Each stage is a block of combinational logic that transforms its
        input into an output. The output is captured by the stage register
        (flip-flops) on the next clock edge.

        In hardware, each stage has a maximum propagation delay. The clock
        period must be long enough for the SLOWEST stage to complete.
        This slowest stage is the "critical path" and determines the
        maximum clock frequency.

        Args:
            stage_num: Which stage (0-4) to execute.
            input_data: The data from the previous stage (or raw inputs
                       for stage 0). None means the stage is idle.

        Returns:
            The processed data to be captured by this stage's register,
            or None if the stage is idle.
        """
        if input_data is None:
            return None

        if stage_num == 0:
            return self._stage_unpack(input_data)
        elif stage_num == 1:
            return self._stage_align(input_data)
        elif stage_num == 2:
            return self._stage_add(input_data)
        elif stage_num == 3:
            return self._stage_normalize(input_data)
        elif stage_num == 4:
            return self._stage_round_pack(input_data)
        return None  # pragma: no cover

    # ----- Stage 0: UNPACK -----
    # Extract the components of each operand and handle special cases.
    # This is the simplest stage — mostly just bit field extraction.

    def _stage_unpack(
        self, inputs: tuple[FloatBits, FloatBits]
    ) -> dict[str, Any]:
        """Stage 1: Unpack operands into sign, exponent, mantissa.

        In hardware, this stage consists of:
        - Bit field extractors (just wires routed to the right places)
        - Special value detectors (AND/OR trees on exponent and mantissa)
        - Implicit bit logic (MUX controlled by exponent-is-zero detector)

        The "heavy lifting" here is the special case detection, which
        involves wide AND and OR gates across the exponent field.
        """
        a, b = inputs
        fmt = self.fmt

        # Check for special values first — these bypass the pipeline
        # with pre-computed results.
        a_nan = is_nan(a)
        b_nan = is_nan(b)
        a_inf = is_inf(a)
        b_inf = is_inf(b)
        a_zero = is_zero(a)
        b_zero = is_zero(b)

        # NaN propagation
        if a_nan or b_nan:
            return {"special": FloatBits(
                sign=0,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                fmt=fmt,
            )}

        # Infinity handling
        if a_inf and b_inf:
            if a.sign == b.sign:
                return {"special": FloatBits(
                    sign=a.sign,
                    exponent=[1] * fmt.exponent_bits,
                    mantissa=[0] * fmt.mantissa_bits,
                    fmt=fmt,
                )}
            else:
                return {"special": FloatBits(
                    sign=0,
                    exponent=[1] * fmt.exponent_bits,
                    mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                    fmt=fmt,
                )}
        if a_inf:
            return {"special": a}
        if b_inf:
            return {"special": b}

        # Zero handling
        if a_zero and b_zero:
            from fp_arithmetic._gates import AND as GATE_AND
            result_sign = GATE_AND(a.sign, b.sign)
            return {"special": FloatBits(
                sign=result_sign,
                exponent=[0] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )}
        if a_zero:
            return {"special": b}
        if b_zero:
            return {"special": a}

        # Normal case: extract fields
        exp_a = _bits_msb_to_int(a.exponent)
        exp_b = _bits_msb_to_int(b.exponent)
        mant_a = _bits_msb_to_int(a.mantissa)
        mant_b = _bits_msb_to_int(b.mantissa)

        # Add implicit leading 1 for normal numbers
        if exp_a != 0:
            mant_a = (1 << fmt.mantissa_bits) | mant_a
        else:
            exp_a = 1

        if exp_b != 0:
            mant_b = (1 << fmt.mantissa_bits) | mant_b
        else:
            exp_b = 1

        # Add guard bits for rounding precision
        guard_bits = 3
        mant_a <<= guard_bits
        mant_b <<= guard_bits

        return {
            "sign_a": a.sign,
            "sign_b": b.sign,
            "exp_a": exp_a,
            "exp_b": exp_b,
            "mant_a": mant_a,
            "mant_b": mant_b,
            "guard_bits": guard_bits,
        }

    # ----- Stage 1: ALIGN -----
    # Shift the smaller mantissa to align exponents.

    def _stage_align(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 2: Align mantissas by shifting the smaller one right.

        In hardware, this stage uses a barrel shifter — a network of
        multiplexers arranged in log2(N) layers, where each layer
        conditionally shifts by a power of 2. A barrel shifter can
        shift by any amount in a single clock cycle.

            Barrel shifter for 8-bit shift amount:
            Layer 0: shift by 0 or 1   (controlled by shift_amount[0])
            Layer 1: shift by 0 or 2   (controlled by shift_amount[1])
            Layer 2: shift by 0 or 4   (controlled by shift_amount[2])
            ...

        This is much faster than shifting one position per cycle, but
        uses more gates (area/power tradeoff).
        """
        if "special" in data:
            return data

        fmt = self.fmt
        exp_a = data["exp_a"]
        exp_b = data["exp_b"]
        mant_a = data["mant_a"]
        mant_b = data["mant_b"]
        guard_bits = data["guard_bits"]

        if exp_a >= exp_b:
            exp_diff = exp_a - exp_b
            if exp_diff > 0 and exp_diff < (fmt.mantissa_bits + 1 + guard_bits):
                shifted_out = mant_b & ((1 << exp_diff) - 1)
                sticky = 1 if shifted_out != 0 else 0
            else:
                sticky = 1 if mant_b != 0 and exp_diff > 0 else 0
            mant_b >>= exp_diff
            if sticky and exp_diff > 0:
                mant_b |= 1
            result_exp = exp_a
        else:
            exp_diff = exp_b - exp_a
            if exp_diff > 0 and exp_diff < (fmt.mantissa_bits + 1 + guard_bits):
                shifted_out = mant_a & ((1 << exp_diff) - 1)
                sticky = 1 if shifted_out != 0 else 0
            else:
                sticky = 1 if mant_a != 0 and exp_diff > 0 else 0
            mant_a >>= exp_diff
            if sticky and exp_diff > 0:
                mant_a |= 1
            result_exp = exp_b

        return {
            "sign_a": data["sign_a"],
            "sign_b": data["sign_b"],
            "mant_a": mant_a,
            "mant_b": mant_b,
            "result_exp": result_exp,
            "guard_bits": guard_bits,
        }

    # ----- Stage 2: ADD/SUB -----
    # Add or subtract aligned mantissas based on signs.

    def _stage_add(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 3: Add or subtract mantissas.

        In hardware, this stage is a wide adder (or adder/subtractor).
        For FP32 with guard bits, this is a ~27-bit adder. Modern designs
        use carry-lookahead or carry-select adders for speed.

            Same signs:      result = mant_a + mant_b
            Different signs: result = |mant_a - mant_b|

        The sign of the result depends on which operand was larger.
        """
        if "special" in data:
            return data

        mant_a = data["mant_a"]
        mant_b = data["mant_b"]
        sign_a = data["sign_a"]
        sign_b = data["sign_b"]

        if sign_a == sign_b:
            result_mant = mant_a + mant_b
            result_sign = sign_a
        else:
            if mant_a >= mant_b:
                result_mant = mant_a - mant_b
                result_sign = sign_a
            else:
                result_mant = mant_b - mant_a
                result_sign = sign_b

        # Handle zero result
        if result_mant == 0:
            fmt = self.fmt
            return {"special": FloatBits(
                sign=0,
                exponent=[0] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )}

        return {
            "result_sign": result_sign,
            "result_mant": result_mant,
            "result_exp": data["result_exp"],
            "guard_bits": data["guard_bits"],
        }

    # ----- Stage 3: NORMALIZE -----
    # Shift result to get leading 1 in the correct position.

    def _stage_normalize(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 4: Normalize the result mantissa.

        In hardware, normalization uses a leading-zero counter (LZC) to
        determine how far to shift, then a barrel shifter to do the shift.
        The LZC is a priority encoder built from a tree of OR gates:

            Leading-zero counter for 8 bits:
                Level 0: check pairs → 4 results
                Level 1: check quad  → 2 results
                Level 2: check oct   → 1 result (the shift amount)

        This is one of the more expensive stages in terms of gate count,
        which is why some architectures split it across two pipeline stages.
        """
        if "special" in data:
            return data

        fmt = self.fmt
        result_mant = data["result_mant"]
        result_exp = data["result_exp"]
        guard_bits = data["guard_bits"]
        normal_pos = fmt.mantissa_bits + guard_bits
        leading_pos = result_mant.bit_length() - 1

        if leading_pos > normal_pos:
            shift_amount = leading_pos - normal_pos
            lost_bits = result_mant & ((1 << shift_amount) - 1)
            result_mant >>= shift_amount
            if lost_bits != 0:
                result_mant |= 1
            result_exp += shift_amount
        elif leading_pos < normal_pos:
            shift_amount = normal_pos - leading_pos
            if result_exp - shift_amount >= 1:
                result_mant <<= shift_amount
                result_exp -= shift_amount
            else:
                actual_shift = result_exp - 1
                if actual_shift > 0:
                    result_mant <<= actual_shift
                result_exp = 0

        return {
            "result_sign": data["result_sign"],
            "result_mant": result_mant,
            "result_exp": result_exp,
            "guard_bits": guard_bits,
        }

    # ----- Stage 4: ROUND & PACK -----
    # Apply rounding and pack into FloatBits.

    def _stage_round_pack(self, data: dict[str, Any]) -> FloatBits:
        """Stage 5: Round to nearest even and pack into FloatBits.

        In hardware, rounding logic is a small circuit that examines the
        guard/round/sticky bits and conditionally increments the mantissa.
        The increment can cause a carry that propagates through the entire
        mantissa, potentially requiring a re-normalization (shift right by 1
        and increment exponent).

            Guard/Round/Sticky decision table:
            ┌─────┬─────┬────────┬────────────────────────────┐
            │  G  │  R  │  S     │  Action                    │
            ├─────┼─────┼────────┼────────────────────────────┤
            │  0  │  X  │  X     │  Truncate (round down)     │
            │  1  │  0  │  0     │  Tie → round to even       │
            │  1  │  0  │  1     │  Round up                  │
            │  1  │  1  │  X     │  Round up                  │
            └─────┴─────┴────────┴────────────────────────────┘
        """
        if "special" in data:
            return data["special"]

        fmt = self.fmt
        result_mant = data["result_mant"]
        result_exp = data["result_exp"]
        result_sign = data["result_sign"]
        guard_bits = data["guard_bits"]

        # Extract guard, round, sticky bits
        guard = (result_mant >> (guard_bits - 1)) & 1
        round_bit = (result_mant >> (guard_bits - 2)) & 1
        sticky_bit = result_mant & ((1 << (guard_bits - 2)) - 1)
        sticky_bit = 1 if sticky_bit != 0 else 0

        # Remove guard bits
        result_mant >>= guard_bits

        # Apply rounding
        if guard == 1:
            if round_bit == 1 or sticky_bit == 1:
                result_mant += 1
            elif (result_mant & 1) == 1:
                result_mant += 1

        # Check if rounding caused overflow
        if result_mant >= (1 << (fmt.mantissa_bits + 1)):
            result_mant >>= 1
            result_exp += 1

        # Handle exponent overflow
        max_exp = (1 << fmt.exponent_bits) - 1
        if result_exp >= max_exp:
            return FloatBits(
                sign=result_sign,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )

        # Handle exponent underflow (denormals)
        if result_exp <= 0:
            if result_exp < -(fmt.mantissa_bits):
                return FloatBits(
                    sign=result_sign,
                    exponent=[0] * fmt.exponent_bits,
                    mantissa=[0] * fmt.mantissa_bits,
                    fmt=fmt,
                )
            shift = 1 - result_exp
            result_mant >>= shift
            result_exp = 0

        # Remove implicit leading 1 for normal numbers
        if result_exp > 0:
            result_mant &= (1 << fmt.mantissa_bits) - 1

        return FloatBits(
            sign=result_sign,
            exponent=_int_to_bits_msb(result_exp, fmt.exponent_bits),
            mantissa=_int_to_bits_msb(result_mant, fmt.mantissa_bits),
            fmt=fmt,
        )


# ---------------------------------------------------------------------------
# PipelinedFPMultiplier — 4-stage pipelined floating-point multiplier
# ---------------------------------------------------------------------------


class PipelinedFPMultiplier:
    """A 4-stage pipelined floating-point multiplier driven by a clock.

    Multiplication is simpler than addition because there's no alignment
    step — the exponents simply add and the mantissas multiply. This means
    the multiplier pipeline has fewer stages (4 vs 5 for the adder).

    === Pipeline Stages ===

        Stage 1: UNPACK + SIGN + EXPONENT
            Extract fields, XOR signs, add exponents, subtract bias.
            In hardware, the sign XOR is literally one gate, and the
            exponent addition is a small (8-bit for FP32) adder.

        Stage 2: MULTIPLY MANTISSAS
            This is the most expensive stage. For FP32, it's a 24x24
            bit multiplier producing a 48-bit product. In hardware,
            this uses a tree of partial product generators and
            carry-save adders (Wallace tree or Dadda tree).

        Stage 3: NORMALIZE
            The product of two 1.xxx numbers is between 1.0 and 3.999...,
            so at most a 1-bit right shift is needed. Much simpler than
            addition normalization.

        Stage 4: ROUND & PACK
            Same as the adder: apply round-to-nearest-even and pack.

    === Usage ===

        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        a = float_to_bits(3.0)
        b = float_to_bits(4.0)
        mul.submit(a, b)

        for _ in range(4):
            clock.full_cycle()

        result = bits_to_float(mul.results[0])  # 12.0
    """

    NUM_STAGES = 4

    def __init__(self, clock: Clock, fmt: FloatFormat = FP32) -> None:
        self.clock = clock
        self.fmt = fmt
        self._stages: list[Any] = [None] * self.NUM_STAGES
        self._inputs_pending: list[tuple[FloatBits, FloatBits]] = []
        self.results: list[FloatBits] = []
        self.cycle_count: int = 0
        clock.register_listener(self._on_clock_edge)

    def submit(self, a: FloatBits, b: FloatBits) -> None:
        """Submit a new multiplication to the pipeline."""
        self._inputs_pending.append((a, b))

    def _on_clock_edge(self, edge: ClockEdge) -> None:
        """Advance pipeline on rising clock edges."""
        if not edge.is_rising:
            return
        self.cycle_count += 1

        for i in range(self.NUM_STAGES - 1, 0, -1):
            self._stages[i] = self._process_stage(i, self._stages[i - 1])

        if self._inputs_pending:
            a, b = self._inputs_pending.pop(0)
            self._stages[0] = self._process_stage(0, (a, b))
        else:
            self._stages[0] = None

        if self._stages[self.NUM_STAGES - 1] is not None:
            self.results.append(self._stages[self.NUM_STAGES - 1])
            self._stages[self.NUM_STAGES - 1] = None

    def _process_stage(self, stage_num: int, input_data: Any) -> Any:
        """Execute one pipeline stage."""
        if input_data is None:
            return None
        if stage_num == 0:
            return self._stage_unpack_exp(input_data)
        elif stage_num == 1:
            return self._stage_multiply(input_data)
        elif stage_num == 2:
            return self._stage_normalize(input_data)
        elif stage_num == 3:
            return self._stage_round_pack(input_data)
        return None  # pragma: no cover

    # ----- Stage 0: UNPACK + SIGN + EXPONENT -----

    def _stage_unpack_exp(
        self, inputs: tuple[FloatBits, FloatBits]
    ) -> dict[str, Any]:
        """Stage 1: Unpack, compute result sign, and add exponents.

        In hardware:
        - Sign = XOR(sign_a, sign_b) — one gate!
        - Exponent = exp_a + exp_b - bias — two 8-bit adders
        - Mantissa extraction = just routing wires
        """
        a, b = inputs
        fmt = self.fmt
        from fp_arithmetic._gates import XOR as GATE_XOR

        result_sign = GATE_XOR(a.sign, b.sign)

        # Special case handling
        if is_nan(a) or is_nan(b):
            return {"special": FloatBits(
                sign=0,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                fmt=fmt,
            )}

        a_inf = is_inf(a)
        b_inf = is_inf(b)
        a_zero = is_zero(a)
        b_zero = is_zero(b)

        if (a_inf and b_zero) or (b_inf and a_zero):
            return {"special": FloatBits(
                sign=0,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                fmt=fmt,
            )}

        if a_inf or b_inf:
            return {"special": FloatBits(
                sign=result_sign,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )}

        if a_zero or b_zero:
            return {"special": FloatBits(
                sign=result_sign,
                exponent=[0] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )}

        # Extract fields
        exp_a = _bits_msb_to_int(a.exponent)
        exp_b = _bits_msb_to_int(b.exponent)
        mant_a = _bits_msb_to_int(a.mantissa)
        mant_b = _bits_msb_to_int(b.mantissa)

        if exp_a != 0:
            mant_a = (1 << fmt.mantissa_bits) | mant_a
        else:
            exp_a = 1

        if exp_b != 0:
            mant_b = (1 << fmt.mantissa_bits) | mant_b
        else:
            exp_b = 1

        result_exp = exp_a + exp_b - fmt.bias

        return {
            "result_sign": result_sign,
            "result_exp": result_exp,
            "mant_a": mant_a,
            "mant_b": mant_b,
        }

    # ----- Stage 1: MULTIPLY MANTISSAS -----

    def _stage_multiply(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 2: Multiply mantissas using shift-and-add.

        In hardware, this is the most expensive stage. For FP32 (24-bit
        mantissas), the multiplier produces a 48-bit product using either:

        - Wallace tree: reduces partial products in parallel using
          carry-save adders (CSAs), completing in O(log N) levels
        - Booth encoding: reduces the number of partial products by
          recoding the multiplier in radix-4

        Our implementation uses Python integer multiplication, which
        internally does the same shift-and-add but much faster.
        """
        if "special" in data:
            return data

        product = data["mant_a"] * data["mant_b"]

        return {
            "result_sign": data["result_sign"],
            "result_exp": data["result_exp"],
            "product": product,
        }

    # ----- Stage 2: NORMALIZE -----

    def _stage_normalize(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 3: Normalize the product.

        For multiplication, the product of two 1.xxx numbers is always
        in the range [1.0, 4.0), so the leading 1 is at either position
        2*mantissa_bits or 2*mantissa_bits+1. This means we only ever
        need to shift by 0 or 1 — much simpler than addition normalization.
        """
        if "special" in data:
            return data

        fmt = self.fmt
        product = data["product"]
        result_exp = data["result_exp"]

        product_leading = product.bit_length() - 1
        normal_pos = 2 * fmt.mantissa_bits

        if product_leading > normal_pos:
            extra = product_leading - normal_pos
            result_exp += extra
        elif product_leading < normal_pos:
            deficit = normal_pos - product_leading
            result_exp -= deficit

        return {
            "result_sign": data["result_sign"],
            "result_exp": result_exp,
            "product": product,
            "product_leading": product_leading,
        }

    # ----- Stage 3: ROUND & PACK -----

    def _stage_round_pack(self, data: dict[str, Any]) -> FloatBits:
        """Stage 4: Round and pack the multiplication result."""
        if "special" in data:
            return data["special"]

        fmt = self.fmt
        result_sign = data["result_sign"]
        result_exp = data["result_exp"]
        product = data["product"]
        product_leading = data["product_leading"]

        round_pos = product_leading - fmt.mantissa_bits

        if round_pos > 0:
            guard = (product >> (round_pos - 1)) & 1
            if round_pos >= 2:
                round_bit = (product >> (round_pos - 2)) & 1
                sticky = 1 if (product & ((1 << (round_pos - 2)) - 1)) != 0 else 0
            else:
                round_bit = 0
                sticky = 0

            result_mant = product >> round_pos

            if guard == 1:
                if round_bit == 1 or sticky == 1:
                    result_mant += 1
                elif (result_mant & 1) == 1:
                    result_mant += 1

            if result_mant >= (1 << (fmt.mantissa_bits + 1)):
                result_mant >>= 1
                result_exp += 1
        elif round_pos == 0:
            result_mant = product
        else:
            result_mant = product << (-round_pos)

        max_exp = (1 << fmt.exponent_bits) - 1
        if result_exp >= max_exp:
            return FloatBits(
                sign=result_sign,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )

        if result_exp <= 0:
            if result_exp < -(fmt.mantissa_bits):
                return FloatBits(
                    sign=result_sign,
                    exponent=[0] * fmt.exponent_bits,
                    mantissa=[0] * fmt.mantissa_bits,
                    fmt=fmt,
                )
            shift = 1 - result_exp
            result_mant >>= shift
            result_exp = 0

        if result_exp > 0:
            result_mant &= (1 << fmt.mantissa_bits) - 1

        return FloatBits(
            sign=result_sign,
            exponent=_int_to_bits_msb(result_exp, fmt.exponent_bits),
            mantissa=_int_to_bits_msb(result_mant, fmt.mantissa_bits),
            fmt=fmt,
        )


# ---------------------------------------------------------------------------
# PipelinedFMA — 6-stage pipelined fused multiply-add
# ---------------------------------------------------------------------------


class PipelinedFMA:
    """A 6-stage pipelined fused multiply-add (FMA) unit driven by a clock.

    FMA computes a * b + c with a single rounding step. It's the most
    important operation in machine learning because the dot product
    (the core of matrix multiplication) is just a chain of FMAs:

        dot(a, w) = a[0]*w[0] + a[1]*w[1] + ... + a[N]*w[N]
                  = FMA(a[0], w[0], FMA(a[1], w[1], FMA(...)))

    === Pipeline Stages ===

        Stage 1: UNPACK all three operands (a, b, c)
        Stage 2: MULTIPLY a * b mantissas (full precision, no rounding!)
        Stage 3: ALIGN product with c's mantissa
        Stage 4: ADD product + c
        Stage 5: NORMALIZE the sum
        Stage 6: ROUND & PACK (single rounding step!)

    The key advantage over separate multiply + add: the product in stage 2
    is kept at FULL PRECISION (48 bits for FP32). No rounding happens until
    stage 6. This gives more accurate results than the two-step approach.

    === Usage ===

        clock = Clock()
        fma = PipelinedFMA(clock)

        a = float_to_bits(2.0)
        b = float_to_bits(3.0)
        c = float_to_bits(1.0)
        fma.submit(a, b, c)  # computes 2.0 * 3.0 + 1.0 = 7.0

        for _ in range(6):
            clock.full_cycle()

        result = bits_to_float(fma.results[0])  # 7.0
    """

    NUM_STAGES = 6

    def __init__(self, clock: Clock, fmt: FloatFormat = FP32) -> None:
        self.clock = clock
        self.fmt = fmt
        self._stages: list[Any] = [None] * self.NUM_STAGES
        self._inputs_pending: list[tuple[FloatBits, FloatBits, FloatBits]] = []
        self.results: list[FloatBits] = []
        self.cycle_count: int = 0
        clock.register_listener(self._on_clock_edge)

    def submit(self, a: FloatBits, b: FloatBits, c: FloatBits) -> None:
        """Submit a new FMA operation (a * b + c) to the pipeline."""
        self._inputs_pending.append((a, b, c))

    def _on_clock_edge(self, edge: ClockEdge) -> None:
        """Advance pipeline on rising clock edges."""
        if not edge.is_rising:
            return
        self.cycle_count += 1

        for i in range(self.NUM_STAGES - 1, 0, -1):
            self._stages[i] = self._process_stage(i, self._stages[i - 1])

        if self._inputs_pending:
            a, b, c = self._inputs_pending.pop(0)
            self._stages[0] = self._process_stage(0, (a, b, c))
        else:
            self._stages[0] = None

        if self._stages[self.NUM_STAGES - 1] is not None:
            self.results.append(self._stages[self.NUM_STAGES - 1])
            self._stages[self.NUM_STAGES - 1] = None

    def _process_stage(self, stage_num: int, input_data: Any) -> Any:
        """Execute one FMA pipeline stage."""
        if input_data is None:
            return None
        if stage_num == 0:
            return self._stage_unpack(input_data)
        elif stage_num == 1:
            return self._stage_multiply(input_data)
        elif stage_num == 2:
            return self._stage_align(input_data)
        elif stage_num == 3:
            return self._stage_add(input_data)
        elif stage_num == 4:
            return self._stage_normalize(input_data)
        elif stage_num == 5:
            return self._stage_round_pack(input_data)
        return None  # pragma: no cover

    # ----- Stage 0: UNPACK all three operands -----

    def _stage_unpack(
        self, inputs: tuple[FloatBits, FloatBits, FloatBits]
    ) -> dict[str, Any]:
        """Stage 1: Unpack a, b, and c.

        This stage handles all the special-case detection for three operands.
        FMA special cases are more complex than add or multiply alone because
        we need to consider interactions like Inf*0+c = NaN.
        """
        a, b, c = inputs
        fmt = self.fmt
        from fp_arithmetic._gates import AND as GATE_AND, XOR as GATE_XOR

        # NaN propagation
        if is_nan(a) or is_nan(b) or is_nan(c):
            return {"special": FloatBits(
                sign=0,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                fmt=fmt,
            )}

        a_inf = is_inf(a)
        b_inf = is_inf(b)
        c_inf = is_inf(c)
        a_zero = is_zero(a)
        b_zero = is_zero(b)

        product_sign = GATE_XOR(a.sign, b.sign)

        # Inf * 0 = NaN
        if (a_inf and b_zero) or (b_inf and a_zero):
            return {"special": FloatBits(
                sign=0,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                fmt=fmt,
            )}

        # Inf * finite + c
        if a_inf or b_inf:
            if c_inf and product_sign != c.sign:
                return {"special": FloatBits(
                    sign=0,
                    exponent=[1] * fmt.exponent_bits,
                    mantissa=[1] + [0] * (fmt.mantissa_bits - 1),
                    fmt=fmt,
                )}
            return {"special": FloatBits(
                sign=product_sign,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )}

        # a * b = 0, result is c
        if a_zero or b_zero:
            if is_zero(c):
                result_sign = GATE_AND(product_sign, c.sign)
                return {"special": FloatBits(
                    sign=result_sign,
                    exponent=[0] * fmt.exponent_bits,
                    mantissa=[0] * fmt.mantissa_bits,
                    fmt=fmt,
                )}
            return {"special": c}

        # c is Inf
        if c_inf:
            return {"special": c}

        # Normal case: extract all fields
        exp_a = _bits_msb_to_int(a.exponent)
        exp_b = _bits_msb_to_int(b.exponent)
        mant_a = _bits_msb_to_int(a.mantissa)
        mant_b = _bits_msb_to_int(b.mantissa)
        exp_c = _bits_msb_to_int(c.exponent)
        mant_c = _bits_msb_to_int(c.mantissa)

        if exp_a != 0:
            mant_a = (1 << fmt.mantissa_bits) | mant_a
        else:
            exp_a = 1
        if exp_b != 0:
            mant_b = (1 << fmt.mantissa_bits) | mant_b
        else:
            exp_b = 1
        if exp_c != 0:
            mant_c = (1 << fmt.mantissa_bits) | mant_c
        else:
            exp_c = 1

        return {
            "product_sign": product_sign,
            "c_sign": c.sign,
            "exp_a": exp_a,
            "exp_b": exp_b,
            "mant_a": mant_a,
            "mant_b": mant_b,
            "exp_c": exp_c,
            "mant_c": mant_c,
        }

    # ----- Stage 1: MULTIPLY a * b -----

    def _stage_multiply(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 2: Full-precision multiply of a * b mantissas.

        The key insight of FMA: this product is NOT rounded. We keep all
        48 bits (for FP32) and carry them forward to the addition stage.
        This is what makes FMA more accurate than separate mul + add.
        """
        if "special" in data:
            return data

        fmt = self.fmt
        product = data["mant_a"] * data["mant_b"]
        product_exp = data["exp_a"] + data["exp_b"] - fmt.bias

        # Normalize product position
        product_leading = product.bit_length() - 1
        normal_product_pos = 2 * fmt.mantissa_bits

        if product_leading > normal_product_pos:
            product_exp += product_leading - normal_product_pos
        elif product_leading < normal_product_pos:
            product_exp -= normal_product_pos - product_leading

        return {
            "product_sign": data["product_sign"],
            "c_sign": data["c_sign"],
            "product": product,
            "product_exp": product_exp,
            "product_leading": product_leading,
            "exp_c": data["exp_c"],
            "mant_c": data["mant_c"],
        }

    # ----- Stage 2: ALIGN product with c -----

    def _stage_align(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 3: Align the full-precision product with c's mantissa.

        This is similar to the alignment in addition, but we're aligning
        a 48-bit product (for FP32) with a 24-bit mantissa. The wider
        product means we need wider shifters.
        """
        if "special" in data:
            return data

        fmt = self.fmt
        product = data["product"]
        product_exp = data["product_exp"]
        product_leading = data["product_leading"]
        exp_c = data["exp_c"]
        mant_c = data["mant_c"]

        exp_diff = product_exp - exp_c

        c_scale_shift = product_leading - fmt.mantissa_bits
        if c_scale_shift >= 0:
            c_aligned = mant_c << c_scale_shift
        else:
            c_aligned = mant_c >> (-c_scale_shift)

        if exp_diff >= 0:
            c_aligned >>= exp_diff
            result_exp = product_exp
        else:
            product >>= (-exp_diff)
            result_exp = exp_c

        return {
            "product_sign": data["product_sign"],
            "c_sign": data["c_sign"],
            "product": product,
            "c_aligned": c_aligned,
            "result_exp": result_exp,
            "product_leading": product_leading,
        }

    # ----- Stage 3: ADD product + c -----

    def _stage_add(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 4: Add the aligned product and c."""
        if "special" in data:
            return data

        fmt = self.fmt
        product = data["product"]
        c_aligned = data["c_aligned"]
        product_sign = data["product_sign"]
        c_sign = data["c_sign"]

        if product_sign == c_sign:
            result_mant = product + c_aligned
            result_sign = product_sign
        else:
            if product >= c_aligned:
                result_mant = product - c_aligned
                result_sign = product_sign
            else:
                result_mant = c_aligned - product
                result_sign = c_sign

        if result_mant == 0:
            return {"special": FloatBits(
                sign=0,
                exponent=[0] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )}

        return {
            "result_sign": result_sign,
            "result_mant": result_mant,
            "result_exp": data["result_exp"],
            "product_leading": data["product_leading"],
        }

    # ----- Stage 4: NORMALIZE -----

    def _stage_normalize(self, data: dict[str, Any]) -> dict[str, Any]:
        """Stage 5: Normalize the FMA result."""
        if "special" in data:
            return data

        fmt = self.fmt
        result_mant = data["result_mant"]
        result_exp = data["result_exp"]
        product_leading = data["product_leading"]
        target_pos = product_leading if product_leading > fmt.mantissa_bits else fmt.mantissa_bits

        result_leading = result_mant.bit_length() - 1

        if result_leading > target_pos:
            shift = result_leading - target_pos
            result_exp += shift
        elif result_leading < target_pos:
            shift_needed = target_pos - result_leading
            result_exp -= shift_needed

        return {
            "result_sign": data["result_sign"],
            "result_mant": result_mant,
            "result_exp": result_exp,
        }

    # ----- Stage 5: ROUND & PACK -----

    def _stage_round_pack(self, data: dict[str, Any]) -> FloatBits:
        """Stage 6: Round once and pack.

        The single rounding step here is what makes FMA "fused." Instead
        of rounding after multiplication AND after addition (two sources
        of error), we only round here (one source of error). For operations
        like dot products with millions of FMA ops, this small improvement
        compounds into significantly better numerical accuracy.
        """
        if "special" in data:
            return data["special"]

        fmt = self.fmt
        result_sign = data["result_sign"]
        result_exp = data["result_exp"]
        result_mant = data["result_mant"]

        result_leading = result_mant.bit_length() - 1
        round_pos = result_leading - fmt.mantissa_bits

        if round_pos > 0:
            guard = (result_mant >> (round_pos - 1)) & 1
            if round_pos >= 2:
                round_bit = (result_mant >> (round_pos - 2)) & 1
                sticky = 1 if (result_mant & ((1 << (round_pos - 2)) - 1)) != 0 else 0
            else:
                round_bit = 0
                sticky = 0

            result_mant >>= round_pos

            if guard == 1:
                if round_bit == 1 or sticky == 1:
                    result_mant += 1
                elif (result_mant & 1) == 1:
                    result_mant += 1

            if result_mant >= (1 << (fmt.mantissa_bits + 1)):
                result_mant >>= 1
                result_exp += 1
        elif round_pos < 0:
            result_mant <<= (-round_pos)

        max_exp = (1 << fmt.exponent_bits) - 1
        if result_exp >= max_exp:
            return FloatBits(
                sign=result_sign,
                exponent=[1] * fmt.exponent_bits,
                mantissa=[0] * fmt.mantissa_bits,
                fmt=fmt,
            )

        if result_exp <= 0:
            if result_exp < -(fmt.mantissa_bits):
                return FloatBits(
                    sign=result_sign,
                    exponent=[0] * fmt.exponent_bits,
                    mantissa=[0] * fmt.mantissa_bits,
                    fmt=fmt,
                )
            shift = 1 - result_exp
            result_mant >>= shift
            result_exp = 0

        if result_exp > 0:
            result_mant &= (1 << fmt.mantissa_bits) - 1

        return FloatBits(
            sign=result_sign,
            exponent=_int_to_bits_msb(result_exp, fmt.exponent_bits),
            mantissa=_int_to_bits_msb(result_mant, fmt.mantissa_bits),
            fmt=fmt,
        )


# ---------------------------------------------------------------------------
# FPUnit — a complete floating-point unit with all three pipelines
# ---------------------------------------------------------------------------


class FPUnit:
    """A complete floating-point unit with pipelined adder, multiplier, and FMA.

    This is what sits inside every GPU core (CUDA core / shader processor /
    execution unit). A single FP unit contains:

        ┌─────────────────────────────────────────────────┐
        │                    FP Unit                       │
        │                                                  │
        │   ┌───────────────────────────────┐             │
        │   │  Pipelined FP Adder (5 stages)│             │
        │   └───────────────────────────────┘             │
        │                                                  │
        │   ┌───────────────────────────────┐             │
        │   │  Pipelined FP Multiplier (4)  │             │
        │   └───────────────────────────────┘             │
        │                                                  │
        │   ┌───────────────────────────────┐             │
        │   │  Pipelined FMA Unit (6 stages)│             │
        │   └───────────────────────────────┘             │
        │                                                  │
        │   All three share the same clock signal          │
        └─────────────────────────────────────────────────┘

    A modern GPU like the NVIDIA RTX 4090 has 16,384 CUDA cores, each
    containing an FP unit like this. Running at ~2.5 GHz, that's:

        16,384 cores x 2 FLOPs/cycle (FMA) x 2.52 GHz = 82.6 TFLOPS

    (The "2 FLOPs/cycle" comes from FMA counting as both a multiply and
    an add — one FMA = two floating-point operations.)

    === Usage ===

        clock = Clock()
        fp_unit = FPUnit(clock)

        # Submit to different pipelines simultaneously
        fp_unit.adder.submit(float_to_bits(1.0), float_to_bits(2.0))
        fp_unit.multiplier.submit(float_to_bits(3.0), float_to_bits(4.0))

        # Run enough cycles for all results
        fp_unit.tick(10)

        # Collect results
        add_result = bits_to_float(fp_unit.adder.results[0])  # 3.0
        mul_result = bits_to_float(fp_unit.multiplier.results[0])  # 12.0
    """

    def __init__(self, clock: Clock, fmt: FloatFormat = FP32) -> None:
        self.clock = clock
        self.fmt = fmt
        self.adder = PipelinedFPAdder(clock, fmt)
        self.multiplier = PipelinedFPMultiplier(clock, fmt)
        self.fma = PipelinedFMA(clock, fmt)

    def tick(self, n: int = 1) -> None:
        """Run the clock for n complete cycles.

        Each full cycle consists of a rising edge (where pipeline stages
        advance) and a falling edge (idle half). So ticking N cycles
        advances the pipeline by N stages.

        Args:
            n: Number of complete clock cycles to execute.
        """
        for _ in range(n):
            self.clock.full_cycle()
