"""Tests for activation layers."""

import math

from ml_framework_core import Tensor

from ml_framework_torch.nn.activation import (
    GELU,
    ReLU,
    Sigmoid,
    Softmax,
    Tanh,
    LogSoftmax,
)


class TestReLU:
    def test_positive_values(self) -> None:
        act = ReLU()
        x = Tensor.from_list([1.0, 2.0, 3.0])
        y = act(x)
        assert y.data == [1.0, 2.0, 3.0]

    def test_negative_values(self) -> None:
        act = ReLU()
        x = Tensor.from_list([-1.0, -2.0, -3.0])
        y = act(x)
        assert y.data == [0.0, 0.0, 0.0]

    def test_mixed_values(self) -> None:
        act = ReLU()
        x = Tensor.from_list([-1.0, 0.0, 1.0])
        y = act(x)
        assert y.data == [0.0, 0.0, 1.0]

    def test_repr(self) -> None:
        assert repr(ReLU()) == "ReLU()"


class TestGELU:
    def test_positive_values(self) -> None:
        act = GELU()
        x = Tensor.from_list([1.0, 2.0])
        y = act(x)
        # GELU(1.0) ≈ 0.841
        assert abs(y.data[0] - 0.841) < 0.01

    def test_zero(self) -> None:
        act = GELU()
        x = Tensor.from_list([0.0])
        y = act(x)
        assert abs(y.data[0]) < 1e-6

    def test_negative(self) -> None:
        act = GELU()
        x = Tensor.from_list([-1.0])
        y = act(x)
        # GELU(-1) ≈ -0.159
        assert abs(y.data[0] - (-0.159)) < 0.01

    def test_repr(self) -> None:
        assert repr(GELU()) == "GELU()"


class TestSigmoid:
    def test_zero(self) -> None:
        act = Sigmoid()
        x = Tensor.from_list([0.0])
        y = act(x)
        assert abs(y.data[0] - 0.5) < 1e-6

    def test_large_positive(self) -> None:
        act = Sigmoid()
        x = Tensor.from_list([10.0])
        y = act(x)
        assert y.data[0] > 0.999

    def test_large_negative(self) -> None:
        act = Sigmoid()
        x = Tensor.from_list([-10.0])
        y = act(x)
        assert y.data[0] < 0.001

    def test_repr(self) -> None:
        assert repr(Sigmoid()) == "Sigmoid()"


class TestTanh:
    def test_zero(self) -> None:
        act = Tanh()
        x = Tensor.from_list([0.0])
        y = act(x)
        assert abs(y.data[0]) < 1e-6

    def test_positive(self) -> None:
        act = Tanh()
        x = Tensor.from_list([1.0])
        y = act(x)
        assert abs(y.data[0] - math.tanh(1.0)) < 1e-6

    def test_range(self) -> None:
        act = Tanh()
        x = Tensor.from_list([100.0])
        y = act(x)
        assert abs(y.data[0]) <= 1.0

    def test_repr(self) -> None:
        assert repr(Tanh()) == "Tanh()"


class TestSoftmax:
    def test_sums_to_one(self) -> None:
        act = Softmax(dim=0)
        x = Tensor.from_list([1.0, 2.0, 3.0])
        y = act(x)
        assert abs(sum(y.data) - 1.0) < 1e-6

    def test_all_positive(self) -> None:
        act = Softmax(dim=0)
        x = Tensor.from_list([-1.0, 0.0, 1.0])
        y = act(x)
        for v in y.data:
            assert v > 0

    def test_largest_gets_highest_prob(self) -> None:
        act = Softmax(dim=0)
        x = Tensor.from_list([1.0, 5.0, 2.0])
        y = act(x)
        assert y.data[1] > y.data[0]
        assert y.data[1] > y.data[2]

    def test_2d(self) -> None:
        act = Softmax(dim=1)
        x = Tensor.from_list([[1.0, 2.0], [3.0, 1.0]])
        y = act(x)
        # Row 0 should sum to 1
        assert abs(y.data[0] + y.data[1] - 1.0) < 1e-6
        # Row 1 should sum to 1
        assert abs(y.data[2] + y.data[3] - 1.0) < 1e-6

    def test_repr(self) -> None:
        assert "dim=-1" in repr(Softmax())


class TestLogSoftmax:
    def test_basic(self) -> None:
        act = LogSoftmax(dim=0)
        x = Tensor.from_list([1.0, 2.0, 3.0])
        y = act(x)
        # All values should be negative (log of probability < 1)
        for v in y.data:
            assert v < 0

    def test_matches_log_of_softmax(self) -> None:
        act_log = LogSoftmax(dim=0)
        act_soft = Softmax(dim=0)
        x = Tensor.from_list([1.0, 2.0, 3.0])
        log_result = act_log(x)
        soft_result = act_soft(x)
        for log_val, soft_val in zip(log_result.data, soft_result.data):
            assert abs(log_val - math.log(soft_val)) < 1e-6

    def test_2d(self) -> None:
        act = LogSoftmax(dim=1)
        x = Tensor.from_list([[1.0, 2.0, 3.0], [1.0, 1.0, 1.0]])
        y = act(x)
        # exp(log_softmax) should sum to 1 per row
        row0_sum = sum(math.exp(y.data[i]) for i in range(3))
        row1_sum = sum(math.exp(y.data[3 + i]) for i in range(3))
        assert abs(row0_sum - 1.0) < 1e-6
        assert abs(row1_sum - 1.0) < 1e-6

    def test_repr(self) -> None:
        assert "dim=-1" in repr(LogSoftmax())
