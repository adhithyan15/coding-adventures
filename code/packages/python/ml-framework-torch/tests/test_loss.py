"""Tests for loss functions."""

import math

from ml_framework_core import Tensor

from ml_framework_torch.nn.loss import (
    BCELoss,
    BCEWithLogitsLoss,
    CrossEntropyLoss,
    L1Loss,
    MSELoss,
    NLLLoss,
)


class TestMSELoss:
    def test_perfect_prediction(self) -> None:
        loss_fn = MSELoss()
        pred = Tensor.from_list([1.0, 2.0, 3.0])
        target = Tensor.from_list([1.0, 2.0, 3.0])
        loss = loss_fn(pred, target)
        assert abs(loss.data[0]) < 1e-6

    def test_known_value(self) -> None:
        loss_fn = MSELoss()
        pred = Tensor.from_list([1.0, 2.0])
        target = Tensor.from_list([2.0, 4.0])
        loss = loss_fn(pred, target)
        # (1^2 + 4) / 2 = 2.5
        assert abs(loss.data[0] - 2.5) < 1e-6

    def test_sum_reduction(self) -> None:
        loss_fn = MSELoss(reduction="sum")
        pred = Tensor.from_list([1.0, 2.0])
        target = Tensor.from_list([2.0, 4.0])
        loss = loss_fn(pred, target)
        assert abs(loss.data[0] - 5.0) < 1e-6

    def test_none_reduction(self) -> None:
        loss_fn = MSELoss(reduction="none")
        pred = Tensor.from_list([1.0, 2.0])
        target = Tensor.from_list([2.0, 4.0])
        loss = loss_fn(pred, target)
        assert abs(loss.data[0] - 1.0) < 1e-6
        assert abs(loss.data[1] - 4.0) < 1e-6

    def test_repr(self) -> None:
        assert "mean" in repr(MSELoss())


class TestL1Loss:
    def test_perfect_prediction(self) -> None:
        loss_fn = L1Loss()
        pred = Tensor.from_list([1.0, 2.0])
        target = Tensor.from_list([1.0, 2.0])
        loss = loss_fn(pred, target)
        assert abs(loss.data[0]) < 1e-6

    def test_known_value(self) -> None:
        loss_fn = L1Loss()
        pred = Tensor.from_list([1.0, 5.0])
        target = Tensor.from_list([2.0, 3.0])
        loss = loss_fn(pred, target)
        # (1 + 2) / 2 = 1.5
        assert abs(loss.data[0] - 1.5) < 1e-6

    def test_sum_reduction(self) -> None:
        loss_fn = L1Loss(reduction="sum")
        pred = Tensor.from_list([1.0, 5.0])
        target = Tensor.from_list([2.0, 3.0])
        loss = loss_fn(pred, target)
        assert abs(loss.data[0] - 3.0) < 1e-6

    def test_repr(self) -> None:
        assert "L1Loss" in repr(L1Loss())


class TestCrossEntropyLoss:
    def test_perfect_prediction(self) -> None:
        """High logit for correct class should give low loss."""
        loss_fn = CrossEntropyLoss()
        # Class 0 has highest logit
        pred = Tensor.from_list([[10.0, 0.0, 0.0]])
        target = Tensor.from_list([0.0])
        loss = loss_fn(pred, target)
        assert loss.data[0] < 0.1

    def test_wrong_prediction(self) -> None:
        """High logit for wrong class should give high loss."""
        loss_fn = CrossEntropyLoss()
        pred = Tensor.from_list([[0.0, 10.0, 0.0]])
        target = Tensor.from_list([0.0])  # correct is class 0
        loss = loss_fn(pred, target)
        assert loss.data[0] > 1.0

    def test_batch(self) -> None:
        loss_fn = CrossEntropyLoss()
        pred = Tensor.from_list(
            [
                [2.0, 0.5, 0.1],
                [0.1, 2.0, 0.5],
            ]
        )
        target = Tensor.from_list([0.0, 1.0])
        loss = loss_fn(pred, target)
        assert loss.data[0] > 0

    def test_gradient_flows(self) -> None:
        loss_fn = CrossEntropyLoss()
        pred = Tensor.from_list([[1.0, 2.0, 3.0]], requires_grad=True)
        target = Tensor.from_list([2.0])
        loss = loss_fn(pred, target)
        loss.backward()

    def test_repr(self) -> None:
        assert "CrossEntropyLoss" in repr(CrossEntropyLoss())


class TestBCELoss:
    def test_perfect_prediction(self) -> None:
        loss_fn = BCELoss()
        pred = Tensor.from_list([0.99, 0.01])
        target = Tensor.from_list([1.0, 0.0])
        loss = loss_fn(pred, target)
        assert loss.data[0] < 0.1

    def test_wrong_prediction(self) -> None:
        loss_fn = BCELoss()
        pred = Tensor.from_list([0.01, 0.99])
        target = Tensor.from_list([1.0, 0.0])
        loss = loss_fn(pred, target)
        assert loss.data[0] > 1.0

    def test_sum_reduction(self) -> None:
        loss_fn = BCELoss(reduction="sum")
        pred = Tensor.from_list([0.5])
        target = Tensor.from_list([1.0])
        loss = loss_fn(pred, target)
        expected = -math.log(0.5)
        assert abs(loss.data[0] - expected) < 1e-5

    def test_repr(self) -> None:
        assert "BCELoss" in repr(BCELoss())


class TestBCEWithLogitsLoss:
    def test_basic(self) -> None:
        loss_fn = BCEWithLogitsLoss()
        pred = Tensor.from_list([5.0, -5.0])
        target = Tensor.from_list([1.0, 0.0])
        loss = loss_fn(pred, target)
        assert loss.data[0] < 0.1

    def test_wrong_prediction(self) -> None:
        loss_fn = BCEWithLogitsLoss()
        pred = Tensor.from_list([-5.0, 5.0])
        target = Tensor.from_list([1.0, 0.0])
        loss = loss_fn(pred, target)
        assert loss.data[0] > 1.0

    def test_repr(self) -> None:
        assert "BCEWithLogitsLoss" in repr(BCEWithLogitsLoss())


class TestNLLLoss:
    def test_basic(self) -> None:
        loss_fn = NLLLoss()
        # Log probabilities (already log-softmaxed)
        log_probs = Tensor.from_list(
            [
                [-0.1, -2.0, -3.0],
                [-3.0, -0.1, -2.0],
            ]
        )
        target = Tensor.from_list([0.0, 1.0])
        loss = loss_fn(log_probs, target)
        # Should be mean of [0.1, 0.1] = 0.1
        assert abs(loss.data[0] - 0.1) < 1e-5

    def test_sum_reduction(self) -> None:
        loss_fn = NLLLoss(reduction="sum")
        log_probs = Tensor.from_list([[-1.0, -2.0]])
        target = Tensor.from_list([0.0])
        loss = loss_fn(log_probs, target)
        assert abs(loss.data[0] - 1.0) < 1e-5

    def test_repr(self) -> None:
        assert "NLLLoss" in repr(NLLLoss())
