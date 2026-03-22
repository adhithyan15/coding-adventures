"""Tests for the functional API (nn.functional)."""

import math

from ml_framework_core import Tensor

from ml_framework_torch.nn import functional as F


class TestFunctionalActivations:
    def test_relu(self) -> None:
        x = Tensor.from_list([-1.0, 0.0, 1.0])
        y = F.relu(x)
        assert y.data == [0.0, 0.0, 1.0]

    def test_gelu(self) -> None:
        x = Tensor.from_list([0.0])
        y = F.gelu(x)
        assert abs(y.data[0]) < 1e-6

    def test_sigmoid(self) -> None:
        x = Tensor.from_list([0.0])
        y = F.sigmoid(x)
        assert abs(y.data[0] - 0.5) < 1e-6

    def test_tanh(self) -> None:
        x = Tensor.from_list([0.0])
        y = F.tanh(x)
        assert abs(y.data[0]) < 1e-6

    def test_softmax(self) -> None:
        x = Tensor.from_list([1.0, 2.0, 3.0])
        y = F.softmax(x, dim=0)
        assert abs(sum(y.data) - 1.0) < 1e-6

    def test_log_softmax_1d(self) -> None:
        x = Tensor.from_list([1.0, 2.0, 3.0])
        y = F.log_softmax(x, dim=0)
        # exp(log_softmax) should sum to 1
        total = sum(math.exp(v) for v in y.data)
        assert abs(total - 1.0) < 1e-6

    def test_log_softmax_2d(self) -> None:
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        y = F.log_softmax(x, dim=1)
        # Each row's exp should sum to 1
        row0 = sum(math.exp(y.data[i]) for i in range(2))
        row1 = sum(math.exp(y.data[2 + i]) for i in range(2))
        assert abs(row0 - 1.0) < 1e-6
        assert abs(row1 - 1.0) < 1e-6


class TestFunctionalLinear:
    def test_linear_with_bias(self) -> None:
        x = Tensor.from_list([[1.0, 2.0]])
        w = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])  # identity
        b = Tensor.from_list([0.5, -0.5])
        y = F.linear(x, w, b)
        assert abs(y.data[0] - 1.5) < 1e-6
        assert abs(y.data[1] - 1.5) < 1e-6

    def test_linear_no_bias(self) -> None:
        x = Tensor.from_list([[1.0, 2.0]])
        w = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = F.linear(x, w)
        assert abs(y.data[0] - 1.0) < 1e-6
        assert abs(y.data[1] - 2.0) < 1e-6


class TestFunctionalLoss:
    def test_mse_loss(self) -> None:
        pred = Tensor.from_list([1.0, 2.0])
        target = Tensor.from_list([1.0, 2.0])
        loss = F.mse_loss(pred, target)
        assert abs(loss.data[0]) < 1e-6

    def test_mse_loss_sum(self) -> None:
        pred = Tensor.from_list([1.0])
        target = Tensor.from_list([3.0])
        loss = F.mse_loss(pred, target, reduction="sum")
        assert abs(loss.data[0] - 4.0) < 1e-6

    def test_mse_loss_none(self) -> None:
        pred = Tensor.from_list([1.0, 2.0])
        target = Tensor.from_list([2.0, 4.0])
        loss = F.mse_loss(pred, target, reduction="none")
        assert abs(loss.data[0] - 1.0) < 1e-6
        assert abs(loss.data[1] - 4.0) < 1e-6

    def test_l1_loss(self) -> None:
        pred = Tensor.from_list([1.0, 5.0])
        target = Tensor.from_list([2.0, 3.0])
        loss = F.l1_loss(pred, target)
        assert abs(loss.data[0] - 1.5) < 1e-6

    def test_cross_entropy(self) -> None:
        pred = Tensor.from_list([[10.0, 0.0, 0.0]])
        target = Tensor.from_list([0.0])
        loss = F.cross_entropy(pred, target)
        assert loss.data[0] < 0.1

    def test_binary_cross_entropy(self) -> None:
        pred = Tensor.from_list([0.9])
        target = Tensor.from_list([1.0])
        loss = F.binary_cross_entropy(pred, target)
        assert loss.data[0] < 0.2

    def test_nll_loss(self) -> None:
        log_probs = Tensor.from_list([[-0.1, -2.0, -3.0]])
        target = Tensor.from_list([0.0])
        loss = F.nll_loss(log_probs, target)
        assert abs(loss.data[0] - 0.1) < 1e-5
