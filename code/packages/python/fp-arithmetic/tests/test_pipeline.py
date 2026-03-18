"""Tests for pipelined floating-point arithmetic.

These tests verify that the clock-driven pipelined FP units produce correct
results and exhibit proper pipeline behavior (latency, throughput, etc.).

Every test creates a fresh Clock and pipeline to avoid cross-test state.
"""

from __future__ import annotations

import math

from clock import Clock

from fp_arithmetic.formats import FP32, FloatBits
from fp_arithmetic.ieee754 import bits_to_float, float_to_bits, is_nan
from fp_arithmetic.pipeline import (
    FPUnit,
    PipelinedFMA,
    PipelinedFPAdder,
    PipelinedFPMultiplier,
)


# ---------------------------------------------------------------------------
# Helper: convert float result with tolerance check
# ---------------------------------------------------------------------------


def _approx(expected: float, actual: float, tol: float = 1e-6) -> bool:
    """Check if two floats are approximately equal."""
    if math.isnan(expected) and math.isnan(actual):
        return True
    if math.isinf(expected) and math.isinf(actual):
        return expected == actual
    return abs(expected - actual) <= tol


# ===========================================================================
# PipelinedFPAdder tests
# ===========================================================================


class TestPipelinedFPAdder:
    """Tests for the 5-stage pipelined FP adder."""

    def test_single_addition(self) -> None:
        """Submit 1.0 + 2.0, tick 5 cycles, verify result is 3.0."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        a = float_to_bits(1.0)
        b = float_to_bits(2.0)
        adder.submit(a, b)

        # 5 full cycles for the result to traverse all 5 stages
        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert result == 3.0

    def test_addition_negative_result(self) -> None:
        """Submit 1.0 + (-3.0), verify result is -2.0."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        a = float_to_bits(1.0)
        b = float_to_bits(-3.0)
        adder.submit(a, b)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert result == -2.0

    def test_addition_with_different_exponents(self) -> None:
        """Submit 1.5 + 0.25, verify result is 1.75."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        a = float_to_bits(1.5)
        b = float_to_bits(0.25)
        adder.submit(a, b)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert _approx(1.75, result)

    def test_pipeline_throughput(self) -> None:
        """Submit 5 additions, tick enough cycles, verify all 5 results.

        After the first 5 cycles (pipeline fill), results should come out
        one per cycle. Total cycles needed: 5 (latency) + 4 (remaining) = 9.
        """
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        # Submit 5 different additions
        test_cases = [
            (1.0, 2.0, 3.0),
            (3.0, 4.0, 7.0),
            (0.5, 0.5, 1.0),
            (10.0, -3.0, 7.0),
            (100.0, 200.0, 300.0),
        ]
        for a_val, b_val, _ in test_cases:
            adder.submit(float_to_bits(a_val), float_to_bits(b_val))

        # Need 5 + 4 = 9 cycles for all results
        for _ in range(9):
            clock.full_cycle()

        assert len(adder.results) == 5

        for i, (_, _, expected) in enumerate(test_cases):
            result = bits_to_float(adder.results[i])
            assert _approx(expected, result), (
                f"Case {i}: expected {expected}, got {result}"
            )

    def test_pipeline_latency(self) -> None:
        """Verify the first result takes exactly 5 cycles (latency = 5).

        We tick one cycle at a time and check when the first result appears.
        """
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        adder.submit(float_to_bits(1.0), float_to_bits(2.0))

        # After 4 cycles: no result yet
        for _ in range(4):
            clock.full_cycle()
        assert len(adder.results) == 0

        # After 5th cycle: first result appears
        clock.full_cycle()
        assert len(adder.results) == 1

    def test_empty_pipeline(self) -> None:
        """Tick without submitting anything — should not error."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        for _ in range(10):
            clock.full_cycle()

        assert len(adder.results) == 0
        assert adder.cycle_count == 10

    def test_nan_propagation(self) -> None:
        """NaN + anything = NaN."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        nan_bits = float_to_bits(float("nan"))
        one_bits = float_to_bits(1.0)
        adder.submit(nan_bits, one_bits)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        assert is_nan(adder.results[0])

    def test_nan_both_operands(self) -> None:
        """NaN + NaN = NaN."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        nan_bits = float_to_bits(float("nan"))
        adder.submit(nan_bits, nan_bits)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        assert is_nan(adder.results[0])

    def test_inf_addition(self) -> None:
        """Inf + finite = Inf."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        inf_bits = float_to_bits(float("inf"))
        one_bits = float_to_bits(1.0)
        adder.submit(inf_bits, one_bits)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert result == float("inf")

    def test_inf_plus_neg_inf(self) -> None:
        """Inf + (-Inf) = NaN."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        pos_inf = float_to_bits(float("inf"))
        neg_inf = float_to_bits(float("-inf"))
        adder.submit(pos_inf, neg_inf)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        assert is_nan(adder.results[0])

    def test_inf_same_sign(self) -> None:
        """Inf + Inf = Inf."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        inf_bits = float_to_bits(float("inf"))
        adder.submit(inf_bits, inf_bits)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert result == float("inf")

    def test_zero_addition(self) -> None:
        """0.0 + x = x."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        zero = float_to_bits(0.0)
        five = float_to_bits(5.0)
        adder.submit(zero, five)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert result == 5.0

    def test_zero_plus_zero(self) -> None:
        """0.0 + 0.0 = 0.0."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        zero = float_to_bits(0.0)
        adder.submit(zero, zero)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert result == 0.0

    def test_subtraction_to_zero(self) -> None:
        """5.0 + (-5.0) = 0.0."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        five = float_to_bits(5.0)
        neg_five = float_to_bits(-5.0)
        adder.submit(five, neg_five)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert result == 0.0

    def test_cycle_count(self) -> None:
        """Verify cycle counter increments correctly."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        assert adder.cycle_count == 0

        for _ in range(3):
            clock.full_cycle()

        assert adder.cycle_count == 3

    def test_b_inf(self) -> None:
        """finite + Inf = Inf."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        one = float_to_bits(1.0)
        inf_bits = float_to_bits(float("inf"))
        adder.submit(one, inf_bits)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        result = bits_to_float(adder.results[0])
        assert result == float("inf")

    def test_a_zero(self) -> None:
        """0 + x = x."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        zero = float_to_bits(0.0)
        three = float_to_bits(3.0)
        adder.submit(zero, three)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        assert bits_to_float(adder.results[0]) == 3.0

    def test_b_zero(self) -> None:
        """x + 0 = x."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        three = float_to_bits(3.0)
        zero = float_to_bits(0.0)
        adder.submit(three, zero)

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        assert bits_to_float(adder.results[0]) == 3.0

    def test_falling_edge_ignored(self) -> None:
        """Verify that falling edges don't advance the pipeline."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        adder.submit(float_to_bits(1.0), float_to_bits(2.0))

        # Do 5 rising edges only (10 ticks = 5 full cycles)
        for _ in range(5):
            clock.tick()  # rising
            clock.tick()  # falling — should be ignored

        assert len(adder.results) == 1

    def test_smaller_exponent_b(self) -> None:
        """Test when exp_b < exp_a (b's mantissa gets shifted)."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        # 128.0 (exp=134) + 0.5 (exp=126): b has smaller exponent
        adder.submit(float_to_bits(128.0), float_to_bits(0.5))

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        assert _approx(128.5, bits_to_float(adder.results[0]))

    def test_smaller_exponent_a(self) -> None:
        """Test when exp_a < exp_b (a's mantissa gets shifted)."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        # 0.5 (exp=126) + 128.0 (exp=134): a has smaller exponent
        adder.submit(float_to_bits(0.5), float_to_bits(128.0))

        for _ in range(5):
            clock.full_cycle()

        assert len(adder.results) == 1
        assert _approx(128.5, bits_to_float(adder.results[0]))


# ===========================================================================
# PipelinedFPMultiplier tests
# ===========================================================================


class TestPipelinedFPMultiplier:
    """Tests for the 4-stage pipelined FP multiplier."""

    def test_single_multiplication(self) -> None:
        """Submit 2.0 * 3.0, tick 4 cycles, verify result is 6.0."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        a = float_to_bits(2.0)
        b = float_to_bits(3.0)
        mul.submit(a, b)

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        result = bits_to_float(mul.results[0])
        assert result == 6.0

    def test_multiplication_with_negative(self) -> None:
        """Submit -2.0 * 3.0, verify result is -6.0."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(-2.0), float_to_bits(3.0))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        result = bits_to_float(mul.results[0])
        assert result == -6.0

    def test_multiplication_negative_times_negative(self) -> None:
        """Submit -2.0 * -3.0, verify result is 6.0."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(-2.0), float_to_bits(-3.0))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        result = bits_to_float(mul.results[0])
        assert result == 6.0

    def test_pipeline_latency_4_cycles(self) -> None:
        """Verify first result takes exactly 4 cycles."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(2.0), float_to_bits(3.0))

        for _ in range(3):
            clock.full_cycle()
        assert len(mul.results) == 0

        clock.full_cycle()
        assert len(mul.results) == 1

    def test_pipeline_throughput(self) -> None:
        """Submit 4 multiplications, verify all complete."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        cases = [
            (2.0, 3.0, 6.0),
            (1.5, 4.0, 6.0),
            (0.5, 10.0, 5.0),
            (7.0, 8.0, 56.0),
        ]
        for a, b, _ in cases:
            mul.submit(float_to_bits(a), float_to_bits(b))

        # 4 + 3 = 7 cycles
        for _ in range(7):
            clock.full_cycle()

        assert len(mul.results) == 4
        for i, (_, _, expected) in enumerate(cases):
            result = bits_to_float(mul.results[i])
            assert _approx(expected, result), f"Case {i}: expected {expected}, got {result}"

    def test_multiply_by_zero(self) -> None:
        """x * 0 = 0."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(42.0), float_to_bits(0.0))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        result = bits_to_float(mul.results[0])
        assert result == 0.0

    def test_multiply_nan(self) -> None:
        """NaN * x = NaN."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(float("nan")), float_to_bits(1.0))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        assert is_nan(mul.results[0])

    def test_multiply_inf(self) -> None:
        """Inf * finite = Inf."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(float("inf")), float_to_bits(2.0))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        result = bits_to_float(mul.results[0])
        assert result == float("inf")

    def test_multiply_inf_times_zero(self) -> None:
        """Inf * 0 = NaN."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(float("inf")), float_to_bits(0.0))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        assert is_nan(mul.results[0])

    def test_multiply_zero_times_inf(self) -> None:
        """0 * Inf = NaN."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(0.0), float_to_bits(float("inf")))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        assert is_nan(mul.results[0])

    def test_empty_pipeline(self) -> None:
        """Tick without submitting anything."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        for _ in range(10):
            clock.full_cycle()

        assert len(mul.results) == 0
        assert mul.cycle_count == 10

    def test_multiply_both_inf(self) -> None:
        """Inf * Inf = Inf."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(float("inf")), float_to_bits(float("inf")))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        result = bits_to_float(mul.results[0])
        assert result == float("inf")

    def test_multiply_zero_times_zero(self) -> None:
        """0 * 0 = 0."""
        clock = Clock()
        mul = PipelinedFPMultiplier(clock)

        mul.submit(float_to_bits(0.0), float_to_bits(0.0))

        for _ in range(4):
            clock.full_cycle()

        assert len(mul.results) == 1
        assert bits_to_float(mul.results[0]) == 0.0


# ===========================================================================
# PipelinedFMA tests
# ===========================================================================


class TestPipelinedFMA:
    """Tests for the 6-stage pipelined FMA unit."""

    def test_basic_fma(self) -> None:
        """FMA(2.0, 3.0, 1.0) = 2.0 * 3.0 + 1.0 = 7.0."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        a = float_to_bits(2.0)
        b = float_to_bits(3.0)
        c = float_to_bits(1.0)
        fma.submit(a, b, c)

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert _approx(7.0, result)

    def test_fma_latency(self) -> None:
        """Verify FMA takes exactly 6 cycles."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(1.0), float_to_bits(1.0), float_to_bits(1.0))

        for _ in range(5):
            clock.full_cycle()
        assert len(fma.results) == 0

        clock.full_cycle()
        assert len(fma.results) == 1

    def test_fma_throughput(self) -> None:
        """Submit 3 FMAs, verify all complete."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        cases = [
            (2.0, 3.0, 1.0, 7.0),
            (1.5, 2.0, 0.5, 3.5),
            (4.0, 0.5, 1.0, 3.0),
        ]
        for a, b, c, _ in cases:
            fma.submit(float_to_bits(a), float_to_bits(b), float_to_bits(c))

        # 6 + 2 = 8 cycles
        for _ in range(8):
            clock.full_cycle()

        assert len(fma.results) == 3
        for i, (_, _, _, expected) in enumerate(cases):
            result = bits_to_float(fma.results[i])
            assert _approx(expected, result), f"Case {i}: expected {expected}, got {result}"

    def test_fma_nan(self) -> None:
        """FMA with NaN input produces NaN."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(float("nan")), float_to_bits(1.0), float_to_bits(1.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        assert is_nan(fma.results[0])

    def test_fma_inf_times_zero(self) -> None:
        """FMA(Inf, 0, c) = NaN."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(
            float_to_bits(float("inf")),
            float_to_bits(0.0),
            float_to_bits(1.0),
        )

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        assert is_nan(fma.results[0])

    def test_fma_zero_times_finite(self) -> None:
        """FMA(0, x, c) = c."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(0.0), float_to_bits(5.0), float_to_bits(3.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert result == 3.0

    def test_fma_inf_operand(self) -> None:
        """FMA(Inf, 2, 1) = Inf."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(float("inf")), float_to_bits(2.0), float_to_bits(1.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert result == float("inf")

    def test_fma_c_inf(self) -> None:
        """FMA(2, 3, Inf) = Inf."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(2.0), float_to_bits(3.0), float_to_bits(float("inf")))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert result == float("inf")

    def test_fma_inf_plus_neg_inf(self) -> None:
        """FMA(Inf, 1, -Inf) = NaN (Inf + (-Inf))."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(
            float_to_bits(float("inf")),
            float_to_bits(1.0),
            float_to_bits(float("-inf")),
        )

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        assert is_nan(fma.results[0])

    def test_fma_zero_plus_zero(self) -> None:
        """FMA(0, 0, 0) = 0."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(0.0), float_to_bits(0.0), float_to_bits(0.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert result == 0.0

    def test_fma_empty_pipeline(self) -> None:
        """Tick empty FMA pipeline — no errors."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        for _ in range(10):
            clock.full_cycle()

        assert len(fma.results) == 0
        assert fma.cycle_count == 10

    def test_fma_cancellation(self) -> None:
        """FMA(2, 3, -6) = 0 (cancellation to zero)."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(2.0), float_to_bits(3.0), float_to_bits(-6.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert result == 0.0

    def test_fma_zero_b_operand(self) -> None:
        """FMA(5, 0, 3) = 3 (a*0 + c = c)."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(5.0), float_to_bits(0.0), float_to_bits(3.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert result == 3.0

    def test_fma_b_inf(self) -> None:
        """FMA(2, Inf, 1) = Inf."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(2.0), float_to_bits(float("inf")), float_to_bits(1.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert result == float("inf")

    def test_fma_zero_times_inf(self) -> None:
        """FMA(0, Inf, c) = NaN."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(0.0), float_to_bits(float("inf")), float_to_bits(1.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        assert is_nan(fma.results[0])

    def test_fma_c_zero(self) -> None:
        """FMA(0, 0, 0) with both product and c zero."""
        clock = Clock()
        fma = PipelinedFMA(clock)

        fma.submit(float_to_bits(0.0), float_to_bits(1.0), float_to_bits(0.0))

        for _ in range(6):
            clock.full_cycle()

        assert len(fma.results) == 1
        result = bits_to_float(fma.results[0])
        assert result == 0.0


# ===========================================================================
# FPUnit tests
# ===========================================================================


class TestFPUnit:
    """Tests for the complete FP unit with all three pipelines."""

    def test_all_pipelines_simultaneously(self) -> None:
        """Submit to all three pipelines, verify all complete."""
        clock = Clock()
        unit = FPUnit(clock)

        unit.adder.submit(float_to_bits(1.0), float_to_bits(2.0))
        unit.multiplier.submit(float_to_bits(3.0), float_to_bits(4.0))
        unit.fma.submit(
            float_to_bits(2.0), float_to_bits(3.0), float_to_bits(1.0)
        )

        # FMA has the longest latency (6 cycles), so tick 6
        unit.tick(6)

        assert len(unit.adder.results) == 1
        assert len(unit.multiplier.results) == 1
        assert len(unit.fma.results) == 1

        assert bits_to_float(unit.adder.results[0]) == 3.0
        assert bits_to_float(unit.multiplier.results[0]) == 12.0
        assert _approx(7.0, bits_to_float(unit.fma.results[0]))

    def test_tick_method(self) -> None:
        """Verify tick(n) advances n full cycles."""
        clock = Clock()
        unit = FPUnit(clock)

        unit.adder.submit(float_to_bits(10.0), float_to_bits(20.0))
        unit.tick(5)

        assert len(unit.adder.results) == 1
        assert bits_to_float(unit.adder.results[0]) == 30.0

    def test_empty_tick(self) -> None:
        """Tick an empty FP unit — no errors."""
        clock = Clock()
        unit = FPUnit(clock)

        unit.tick(10)

        assert len(unit.adder.results) == 0
        assert len(unit.multiplier.results) == 0
        assert len(unit.fma.results) == 0

    def test_unit_format(self) -> None:
        """Verify the FPUnit stores its format."""
        clock = Clock()
        unit = FPUnit(clock)

        assert unit.fmt == FP32
        assert unit.adder.fmt == FP32
        assert unit.multiplier.fmt == FP32
        assert unit.fma.fmt == FP32


# ===========================================================================
# Mixed pipeline tests — interleaving operations
# ===========================================================================


class TestMixedPipeline:
    """Tests for interleaving operations across multiple pipelines."""

    def test_interleaved_add_and_multiply(self) -> None:
        """Interleave additions and multiplications."""
        clock = Clock()
        unit = FPUnit(clock)

        # Submit alternating add and multiply
        unit.adder.submit(float_to_bits(1.0), float_to_bits(2.0))
        unit.multiplier.submit(float_to_bits(3.0), float_to_bits(4.0))
        unit.adder.submit(float_to_bits(5.0), float_to_bits(6.0))
        unit.multiplier.submit(float_to_bits(7.0), float_to_bits(8.0))

        # Run enough cycles for everything to complete
        # Adder: 5 + 1 = 6 cycles for 2 results
        # Multiplier: 4 + 1 = 5 cycles for 2 results
        unit.tick(7)

        assert len(unit.adder.results) == 2
        assert len(unit.multiplier.results) == 2

        assert bits_to_float(unit.adder.results[0]) == 3.0
        assert bits_to_float(unit.adder.results[1]) == 11.0
        assert bits_to_float(unit.multiplier.results[0]) == 12.0
        assert bits_to_float(unit.multiplier.results[1]) == 56.0

    def test_submit_after_initial_tick(self) -> None:
        """Submit new operations after some cycles have passed."""
        clock = Clock()
        unit = FPUnit(clock)

        unit.adder.submit(float_to_bits(1.0), float_to_bits(2.0))
        unit.tick(3)  # 3 cycles in

        # Submit another while first is still in pipeline
        unit.adder.submit(float_to_bits(10.0), float_to_bits(20.0))
        unit.tick(5)  # 3 + 5 = 8 total cycles

        assert len(unit.adder.results) == 2
        assert bits_to_float(unit.adder.results[0]) == 3.0
        assert bits_to_float(unit.adder.results[1]) == 30.0

    def test_heavy_throughput(self) -> None:
        """Submit many operations and verify throughput."""
        clock = Clock()
        adder = PipelinedFPAdder(clock)

        # Submit 10 additions
        for i in range(10):
            a = float(i)
            b = float(i + 1)
            adder.submit(float_to_bits(a), float_to_bits(b))

        # 5 (latency) + 9 (remaining) = 14 cycles
        for _ in range(14):
            clock.full_cycle()

        assert len(adder.results) == 10

        for i in range(10):
            expected = float(i) + float(i + 1)
            result = bits_to_float(adder.results[i])
            assert _approx(expected, result), f"i={i}: expected {expected}, got {result}"
