"""Tests for MatrixMultiplyUnit — Google TPU MXU simulator."""

from __future__ import annotations

from clock import Clock

from compute_unit import Architecture, WorkItem
from compute_unit.matrix_multiply_unit import MatrixMultiplyUnit, MXUConfig

# ---------------------------------------------------------------------------
# MXUConfig tests
# ---------------------------------------------------------------------------


class TestMXUConfig:
    """Test MXUConfig defaults and customization."""

    def test_defaults(self) -> None:
        config = MXUConfig()
        assert config.array_rows == 128
        assert config.array_cols == 128
        assert config.vector_width == 128
        assert config.accumulator_count == 128
        assert config.weight_buffer_size == 4194304
        assert config.activation_buffer_size == 2097152

    def test_custom_config(self) -> None:
        config = MXUConfig(array_rows=4, array_cols=4)
        assert config.array_rows == 4
        assert config.array_cols == 4


# ---------------------------------------------------------------------------
# MatrixMultiplyUnit tests
# ---------------------------------------------------------------------------


class TestMatrixMultiplyUnit:
    """Test the TPU MXU simulator."""

    def test_creation(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        assert mxu.name == "MXU"
        assert mxu.architecture == Architecture.GOOGLE_MXU
        assert mxu.idle

    def test_simple_matmul_2x2(self) -> None:
        """Test a 2x2 matrix multiplication.

        [1, 2]   [5, 6]   [1*5+2*7, 1*6+2*8]   [19, 22]
        [3, 4] x [7, 8] = [3*5+4*7, 3*6+4*8] = [43, 50]
        """
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        result = mxu.run_matmul(
            activations=[[1.0, 2.0], [3.0, 4.0]],
            weights=[[5.0, 6.0], [7.0, 8.0]],
        )
        assert len(result) == 2
        assert len(result[0]) == 2
        assert abs(result[0][0] - 19.0) < 0.1
        assert abs(result[0][1] - 22.0) < 0.1
        assert abs(result[1][0] - 43.0) < 0.1
        assert abs(result[1][1] - 50.0) < 0.1

    def test_identity_matmul(self) -> None:
        """Multiply by identity matrix -> should get original."""
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        result = mxu.run_matmul(
            activations=[[1.0, 2.0], [3.0, 4.0]],
            weights=[[1.0, 0.0], [0.0, 1.0]],
        )
        assert abs(result[0][0] - 1.0) < 0.1
        assert abs(result[0][1] - 2.0) < 0.1
        assert abs(result[1][0] - 3.0) < 0.1
        assert abs(result[1][1] - 4.0) < 0.1

    def test_matmul_with_relu_activation(self) -> None:
        """ReLU should zero out negative values."""
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        # Result will have a negative value
        result = mxu.run_matmul(
            activations=[[1.0, -2.0]],
            weights=[[1.0], [1.0]],
            activation_fn="relu",
        )
        # 1*1 + (-2)*1 = -1.0, ReLU(-1.0) = 0.0
        assert abs(result[0][0] - 0.0) < 0.1

    def test_matmul_with_sigmoid_activation(self) -> None:
        """Sigmoid should squash to [0, 1]."""
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        result = mxu.run_matmul(
            activations=[[0.0]],
            weights=[[1.0]],
            activation_fn="sigmoid",
        )
        # sigmoid(0) = 0.5
        assert abs(result[0][0] - 0.5) < 0.01

    def test_matmul_with_tanh_activation(self) -> None:
        """Tanh should squash to [-1, 1]."""
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        result = mxu.run_matmul(
            activations=[[0.0]],
            weights=[[1.0]],
            activation_fn="tanh",
        )
        # tanh(0) = 0.0
        assert abs(result[0][0]) < 0.01

    def test_matmul_no_activation(self) -> None:
        """No activation should pass through."""
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        result = mxu.run_matmul(
            activations=[[1.0, -2.0]],
            weights=[[1.0], [1.0]],
            activation_fn="none",
        )
        assert abs(result[0][0] - (-1.0)) < 0.1

    def test_dispatch_and_run(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        mxu.dispatch(WorkItem(
            work_id=0,
            input_data=[[1.0, 2.0], [3.0, 4.0]],
            weight_data=[[5.0, 6.0], [7.0, 8.0]],
        ))
        traces = mxu.run()
        assert len(traces) > 0
        assert mxu.idle
        assert len(mxu.result) == 2

    def test_dispatch_without_data(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        mxu.dispatch(WorkItem(work_id=0))
        mxu.run()
        assert mxu.idle
        assert mxu.result == []

    def test_trace_architecture(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        mxu.dispatch(WorkItem(
            work_id=0,
            input_data=[[1.0]],
            weight_data=[[2.0]],
        ))
        traces = mxu.run()
        for trace in traces:
            assert trace.architecture == Architecture.GOOGLE_MXU
            assert trace.unit_name == "MXU"

    def test_idle_trace(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        from clock import ClockEdge

        trace = mxu.step(ClockEdge(cycle=1, value=1, is_rising=True, is_falling=False))
        assert trace.scheduler_action == "idle"
        assert trace.occupancy == 0.0

    def test_multiple_dispatches(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        mxu.dispatch(WorkItem(
            work_id=0,
            input_data=[[1.0]],
            weight_data=[[2.0]],
        ))
        mxu.dispatch(WorkItem(
            work_id=1,
            input_data=[[3.0]],
            weight_data=[[4.0]],
        ))
        mxu.run()
        assert mxu.idle

    def test_reset(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        mxu.run_matmul([[1.0]], [[2.0]])
        mxu.reset()
        assert mxu.idle
        assert mxu.result == []

    def test_systolic_array_accessible(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        assert mxu.systolic_array is not None

    def test_repr(self) -> None:
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        r = repr(mxu)
        assert "MatrixMultiplyUnit" in r
        assert "4x4" in r

    def test_3x2_times_2x3_matmul(self) -> None:
        """Non-square matrix multiplication."""
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        # [1, 2]         [7,  8,  9]      [1*7+2*10, 1*8+2*11, 1*9+2*12]
        # [3, 4]    x    [10, 11, 12]  =  [3*7+4*10, 3*8+4*11, 3*9+4*12]
        # [5, 6]                           [5*7+6*10, 5*8+6*11, 5*9+6*12]
        result = mxu.run_matmul(
            activations=[[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]],
            weights=[[7.0, 8.0, 9.0], [10.0, 11.0, 12.0]],
        )
        assert len(result) == 3
        assert len(result[0]) == 3
        assert abs(result[0][0] - 27.0) < 0.1  # 1*7+2*10
        assert abs(result[1][0] - 61.0) < 0.1  # 3*7+4*10
        assert abs(result[2][2] - 117.0) < 0.1  # 5*9+6*12

    def test_unknown_activation_passthrough(self) -> None:
        """Unknown activation function should pass through."""
        clock = Clock()
        mxu = MatrixMultiplyUnit(
            MXUConfig(array_rows=4, array_cols=4), clock
        )
        result = mxu.run_matmul(
            activations=[[5.0]],
            weights=[[1.0]],
            activation_fn="unknown_fn",
        )
        assert abs(result[0][0] - 5.0) < 0.1
