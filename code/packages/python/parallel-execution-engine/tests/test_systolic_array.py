"""Tests for systolic_array.py — dataflow execution (Google TPU style)."""

from __future__ import annotations

from clock import Clock, ClockEdge
from fp_arithmetic import FP32, float_to_bits

from parallel_execution_engine import (
    ExecutionModel,
    SystolicArray,
    SystolicConfig,
    SystolicPE,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_edge(cycle: int = 1) -> ClockEdge:
    return ClockEdge(cycle=cycle, value=1, is_rising=True, is_falling=False)


# ---------------------------------------------------------------------------
# SystolicConfig
# ---------------------------------------------------------------------------


class TestSystolicConfig:
    def test_defaults(self) -> None:
        config = SystolicConfig()
        assert config.rows == 4
        assert config.cols == 4

    def test_custom(self) -> None:
        config = SystolicConfig(rows=8, cols=8)
        assert config.rows == 8
        assert config.cols == 8


# ---------------------------------------------------------------------------
# SystolicPE
# ---------------------------------------------------------------------------


class TestSystolicPE:
    def test_creation(self) -> None:
        zero = float_to_bits(0.0, FP32)
        pe = SystolicPE(row=0, col=0, weight=zero, accumulator=zero)
        assert pe.row == 0
        assert pe.col == 0
        assert pe.input_buffer is None

    def test_compute_no_input(self) -> None:
        zero = float_to_bits(0.0, FP32)
        pe = SystolicPE(row=0, col=0, weight=zero, accumulator=zero)
        result = pe.compute()
        assert result is None  # No input → no computation

    def test_compute_with_input(self) -> None:
        weight = float_to_bits(3.0, FP32)
        zero = float_to_bits(0.0, FP32)
        input_val = float_to_bits(2.0, FP32)
        pe = SystolicPE(row=0, col=0, weight=weight, accumulator=zero,
                         input_buffer=input_val)

        output = pe.compute()
        assert output is not None  # Input was consumed and passed through

        from fp_arithmetic import bits_to_float
        acc = bits_to_float(pe.accumulator)
        # acc = 0 + 2.0 * 3.0 = 6.0
        assert abs(acc - 6.0) < 0.01

    def test_compute_accumulates(self) -> None:
        """Multiple computes should accumulate."""
        weight = float_to_bits(1.0, FP32)
        zero = float_to_bits(0.0, FP32)
        pe = SystolicPE(row=0, col=0, weight=weight, accumulator=zero)

        # First MAC: acc = 0 + 2.0 * 1.0 = 2.0
        pe.input_buffer = float_to_bits(2.0, FP32)
        pe.compute()

        # Second MAC: acc = 2.0 + 3.0 * 1.0 = 5.0
        pe.input_buffer = float_to_bits(3.0, FP32)
        pe.compute()

        from fp_arithmetic import bits_to_float
        assert abs(bits_to_float(pe.accumulator) - 5.0) < 0.01


# ---------------------------------------------------------------------------
# SystolicArray — basic properties
# ---------------------------------------------------------------------------


class TestSystolicArrayProperties:
    def test_name(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        assert array.name == "SystolicArray"

    def test_width(self) -> None:
        array = SystolicArray(SystolicConfig(rows=3, cols=4), Clock())
        assert array.width == 12

    def test_execution_model(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        assert array.execution_model == ExecutionModel.SYSTOLIC

    def test_initial_not_halted(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        assert array.halted is False

    def test_config_access(self) -> None:
        config = SystolicConfig(rows=3, cols=3)
        array = SystolicArray(config, Clock())
        assert array.config is config

    def test_grid_access(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        assert len(array.grid) == 2
        assert len(array.grid[0]) == 2

    def test_repr(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        r = repr(array)
        assert "SystolicArray" in r
        assert "2x2" in r


# ---------------------------------------------------------------------------
# SystolicArray — weight loading
# ---------------------------------------------------------------------------


class TestSystolicArrayWeights:
    def test_load_weights(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        array.load_weights([[1.0, 2.0], [3.0, 4.0]])

        from fp_arithmetic import bits_to_float
        assert bits_to_float(array.grid[0][0].weight) == 1.0
        assert bits_to_float(array.grid[0][1].weight) == 2.0
        assert bits_to_float(array.grid[1][0].weight) == 3.0
        assert bits_to_float(array.grid[1][1].weight) == 4.0


# ---------------------------------------------------------------------------
# SystolicArray — input feeding
# ---------------------------------------------------------------------------


class TestSystolicArrayInput:
    def test_feed_input(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        array.feed_input(0, 5.0)
        # The input should be in the queue

    def test_feed_input_out_of_range(self) -> None:
        import pytest
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        with pytest.raises(IndexError):
            array.feed_input(2, 1.0)
        with pytest.raises(IndexError):
            array.feed_input(-1, 1.0)

    def test_feed_input_vector(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        array.feed_input_vector([1.0, 2.0])
        # Both rows should have inputs queued


# ---------------------------------------------------------------------------
# SystolicArray — matrix multiplication
# ---------------------------------------------------------------------------


class TestSystolicArrayMatmul:
    def test_identity_weights(self) -> None:
        """Multiply by identity matrix should return the input."""
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        result = array.run_matmul(
            activations=[[1.0, 0.0], [0.0, 1.0]],
            weights=[[1.0, 0.0], [0.0, 1.0]],
        )
        assert abs(result[0][0] - 1.0) < 0.01
        assert abs(result[0][1] - 0.0) < 0.01
        assert abs(result[1][0] - 0.0) < 0.01
        assert abs(result[1][1] - 1.0) < 0.01

    def test_simple_matmul(self) -> None:
        """2x2 matrix multiply."""
        # A = [[1, 2], [3, 4]]
        # W = [[5, 6], [7, 8]]
        # C = A x W = [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]]
        #           = [[19, 22], [43, 50]]
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        result = array.run_matmul(
            activations=[[1.0, 2.0], [3.0, 4.0]],
            weights=[[5.0, 6.0], [7.0, 8.0]],
        )
        assert abs(result[0][0] - 19.0) < 0.1
        assert abs(result[0][1] - 22.0) < 0.1
        assert abs(result[1][0] - 43.0) < 0.1
        assert abs(result[1][1] - 50.0) < 0.1

    def test_3x3_matmul(self) -> None:
        """3x3 matrix multiply."""
        A = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]]
        W = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]
        # C = A x W = [[1, 2, 3], [8, 10, 12], [21, 24, 27]]
        array = SystolicArray(SystolicConfig(rows=3, cols=3), Clock())
        result = array.run_matmul(activations=A, weights=W)
        assert abs(result[0][0] - 1.0) < 0.1
        assert abs(result[0][1] - 2.0) < 0.1
        assert abs(result[0][2] - 3.0) < 0.1
        assert abs(result[1][0] - 8.0) < 0.1
        assert abs(result[1][1] - 10.0) < 0.1
        assert abs(result[2][2] - 27.0) < 0.1

    def test_drain_outputs(self) -> None:
        """drain_outputs returns the correct shape."""
        array = SystolicArray(SystolicConfig(rows=2, cols=3), Clock())
        result = array.drain_outputs()
        assert len(result) == 2
        assert len(result[0]) == 3


# ---------------------------------------------------------------------------
# SystolicArray — stepping and traces
# ---------------------------------------------------------------------------


class TestSystolicArrayStepping:
    def test_step_produces_trace(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        array.load_weights([[1.0, 0.0], [0.0, 1.0]])
        array.feed_input(0, 2.0)

        trace = array.step(make_edge())
        assert trace.cycle == 1
        assert trace.engine_name == "SystolicArray"
        assert trace.execution_model == ExecutionModel.SYSTOLIC
        assert trace.dataflow_info is not None

    def test_halts_when_no_data(self) -> None:
        """Array halts when all input has flowed through."""
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        array.load_weights([[1.0, 0.0], [0.0, 1.0]])
        array.feed_input(0, 1.0)

        # Step until halted
        for i in range(20):
            array.step(make_edge(i + 1))
            if array.halted:
                break
        assert array.halted is True


# ---------------------------------------------------------------------------
# SystolicArray — reset
# ---------------------------------------------------------------------------


class TestSystolicArrayReset:
    def test_reset(self) -> None:
        array = SystolicArray(SystolicConfig(rows=2, cols=2), Clock())
        array.load_weights([[1.0, 2.0], [3.0, 4.0]])
        array.run_matmul(
            activations=[[1.0, 0.0], [0.0, 1.0]],
            weights=[[1.0, 2.0], [3.0, 4.0]],
        )

        array.reset()
        assert array.halted is False
        # Accumulators should be zero
        from fp_arithmetic import bits_to_float
        assert bits_to_float(array.grid[0][0].accumulator) == 0.0
