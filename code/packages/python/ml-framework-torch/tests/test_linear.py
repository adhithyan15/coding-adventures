"""Tests for the Linear layer."""

import math

from ml_framework_core import Tensor

from ml_framework_torch.nn.linear import Linear


class TestLinearInit:
    def test_basic_creation(self) -> None:
        layer = Linear(10, 5)
        assert layer.in_features == 10
        assert layer.out_features == 5
        assert layer.weight.shape == (5, 10)
        assert layer.bias is not None
        assert layer.bias.shape == (5,)

    def test_no_bias(self) -> None:
        layer = Linear(10, 5, bias=False)
        assert layer.bias is None
        assert layer._bias_enabled is False

    def test_xavier_init_scale(self) -> None:
        """Weight values should be scaled by 1/sqrt(in_features)."""
        layer = Linear(100, 50)
        expected_std = 1.0 / math.sqrt(100)
        # Variance should be roughly expected_std^2
        mean = sum(layer.weight.data) / len(layer.weight.data)
        var = sum((x - mean) ** 2 for x in layer.weight.data) / len(layer.weight.data)
        actual_std = math.sqrt(var)
        # Should be in the right ballpark (within 2x)
        assert actual_std < expected_std * 3

    def test_bias_initialized_to_zero(self) -> None:
        layer = Linear(5, 3)
        for v in layer.bias.data:
            assert v == 0.0


class TestLinearForward:
    def test_output_shape(self) -> None:
        layer = Linear(4, 3)
        x = Tensor.randn(2, 4)  # batch=2, in=4
        y = layer(x)
        assert y.shape == (2, 3)

    def test_no_bias_forward(self) -> None:
        layer = Linear(3, 2, bias=False)
        x = Tensor.ones(1, 3)
        y = layer(x)
        assert y.shape == (1, 2)

    def test_manual_computation(self) -> None:
        """Verify output matches manual x @ W.T + b."""
        layer = Linear(2, 2)
        # Set known weights
        layer.weight.data = [1.0, 0.0, 0.0, 1.0]  # identity
        layer.bias.data = [0.5, -0.5]

        x = Tensor.from_list([[1.0, 2.0]])  # (1, 2)
        y = layer(x)
        # y = [1, 2] @ I + [0.5, -0.5] = [1.5, 1.5]
        assert abs(y.data[0] - 1.5) < 1e-6
        assert abs(y.data[1] - 1.5) < 1e-6


class TestLinearParameters:
    def test_parameters_with_bias(self) -> None:
        layer = Linear(5, 3)
        params = list(layer.parameters())
        assert len(params) == 2

    def test_parameters_no_bias(self) -> None:
        layer = Linear(5, 3, bias=False)
        params = list(layer.parameters())
        assert len(params) == 1  # only weight

    def test_named_parameters(self) -> None:
        layer = Linear(5, 3)
        names = dict(layer.named_parameters())
        assert "weight" in names
        assert "bias" in names


class TestLinearGrad:
    def test_gradient_flows(self) -> None:
        """Verify that gradients flow through the linear layer."""
        layer = Linear(2, 1)
        layer.weight.data = [1.0, 1.0]
        layer.bias.data = [0.0]

        x = Tensor.from_list([[1.0, 2.0]], requires_grad=True)
        y = layer(x)
        loss = y.sum()
        loss.backward()

        assert layer.weight.grad is not None
        assert layer.bias is not None


class TestLinearRepr:
    def test_repr(self) -> None:
        layer = Linear(10, 5)
        r = repr(layer)
        assert "Linear" in r
        assert "10" in r
        assert "5" in r
