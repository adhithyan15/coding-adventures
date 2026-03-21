"""Tests for tf.keras.activations — activation function lookup."""

import pytest
from ml_framework_core import Tensor
from ml_framework_tf.keras import activations


class TestActivationFunctions:
    def test_relu(self):
        x = Tensor.from_list([-1.0, 0.0, 1.0])
        y = activations.relu(x)
        assert y.data == [0.0, 0.0, 1.0]

    def test_sigmoid(self):
        x = Tensor.from_list([0.0])
        y = activations.sigmoid(x)
        assert abs(y.data[0] - 0.5) < 1e-6

    def test_tanh(self):
        x = Tensor.from_list([0.0])
        y = activations.tanh(x)
        assert abs(y.data[0]) < 1e-6

    def test_softmax(self):
        x = Tensor.from_list([1.0, 1.0, 1.0])
        y = activations.softmax(x)
        assert abs(sum(y.data) - 1.0) < 1e-6

    def test_gelu(self):
        x = Tensor.from_list([0.0])
        y = activations.gelu(x)
        assert abs(y.data[0]) < 1e-6

    def test_linear(self):
        x = Tensor.from_list([1.0, 2.0])
        y = activations.linear(x)
        assert y is x  # identity — same object


class TestGetFunction:
    def test_get_by_string(self):
        fn = activations.get("relu")
        assert fn is activations.relu

    def test_get_none(self):
        fn = activations.get(None)
        assert fn is activations.linear

    def test_get_callable(self):
        custom = lambda x: x  # noqa: E731
        fn = activations.get(custom)
        assert fn is custom

    def test_get_unknown(self):
        with pytest.raises(ValueError, match="Unknown activation"):
            activations.get("nonexistent")

    def test_all_string_keys(self):
        for key in ["relu", "sigmoid", "tanh", "softmax", "gelu", "linear"]:
            fn = activations.get(key)
            assert callable(fn)
