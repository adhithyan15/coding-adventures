"""Tests for Dropout layer."""

import random

from ml_framework_core import Tensor

from ml_framework_torch.nn.dropout import Dropout


class TestDropoutInit:
    def test_default_p(self) -> None:
        d = Dropout()
        assert d.p == 0.5

    def test_custom_p(self) -> None:
        d = Dropout(p=0.3)
        assert d.p == 0.3

    def test_invalid_p(self) -> None:
        try:
            Dropout(p=1.0)
            assert False, "Should raise"
        except ValueError:
            pass

        try:
            Dropout(p=-0.1)
            assert False, "Should raise"
        except ValueError:
            pass


class TestDropoutForward:
    def test_eval_mode_passthrough(self) -> None:
        d = Dropout(p=0.5)
        d.eval()
        x = Tensor.from_list([1.0, 2.0, 3.0, 4.0])
        y = d(x)
        assert y.data == x.data

    def test_train_mode_zeros_some(self) -> None:
        random.seed(42)
        d = Dropout(p=0.5)
        d.train()
        x = Tensor.ones(100)
        y = d(x)
        # Some values should be zero
        zeros = sum(1 for v in y.data if v == 0.0)
        assert zeros > 0
        # Some values should be non-zero
        nonzeros = sum(1 for v in y.data if v != 0.0)
        assert nonzeros > 0

    def test_scaling(self) -> None:
        """Non-zero values should be scaled by 1/(1-p)."""
        random.seed(42)
        d = Dropout(p=0.5)
        d.train()
        x = Tensor.ones(1000)
        y = d(x)
        # Non-zero values should be 1.0 / (1 - 0.5) = 2.0
        for v in y.data:
            if v != 0.0:
                assert abs(v - 2.0) < 1e-6

    def test_zero_dropout(self) -> None:
        d = Dropout(p=0.0)
        d.train()
        x = Tensor.from_list([1.0, 2.0, 3.0])
        y = d(x)
        assert y.data == x.data

    def test_shape_preserved(self) -> None:
        d = Dropout(p=0.3)
        d.train()
        x = Tensor.randn(2, 3)
        y = d(x)
        assert y.shape == x.shape


class TestDropoutRepr:
    def test_repr(self) -> None:
        r = repr(Dropout(p=0.3))
        assert "Dropout" in r
        assert "0.3" in r
