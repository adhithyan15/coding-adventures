"""Tests for the activations module."""

import pytest
from ml_framework_core import Tensor

from ml_framework_keras.activations import (
    gelu,
    get_activation,
    linear,
    relu,
    sigmoid,
    softmax,
    tanh,
)


class TestReLU:
    def test_positive_values_unchanged(self):
        x = Tensor.from_list([1.0, 2.0, 3.0])
        result = relu(x)
        assert result.data == [1.0, 2.0, 3.0]

    def test_negative_values_zeroed(self):
        x = Tensor.from_list([-1.0, -2.0, -3.0])
        result = relu(x)
        assert result.data == [0.0, 0.0, 0.0]

    def test_mixed_values(self):
        x = Tensor.from_list([-1.0, 0.0, 1.0])
        result = relu(x)
        assert result.data == [0.0, 0.0, 1.0]


class TestSigmoid:
    def test_zero_gives_half(self):
        x = Tensor.from_list([0.0])
        result = sigmoid(x)
        assert abs(result.data[0] - 0.5) < 1e-6

    def test_large_positive_near_one(self):
        x = Tensor.from_list([10.0])
        result = sigmoid(x)
        assert result.data[0] > 0.999

    def test_large_negative_near_zero(self):
        x = Tensor.from_list([-10.0])
        result = sigmoid(x)
        assert result.data[0] < 0.001

    def test_output_range(self):
        x = Tensor.from_list([-5.0, -1.0, 0.0, 1.0, 5.0])
        result = sigmoid(x)
        for val in result.data:
            assert 0.0 < val < 1.0


class TestTanh:
    def test_zero(self):
        x = Tensor.from_list([0.0])
        result = tanh(x)
        assert abs(result.data[0]) < 1e-6

    def test_output_range(self):
        x = Tensor.from_list([-5.0, -1.0, 0.0, 1.0, 5.0])
        result = tanh(x)
        for val in result.data:
            assert -1.0 <= val <= 1.0


class TestSoftmax:
    def test_sums_to_one(self):
        x = Tensor.from_list([1.0, 2.0, 3.0])
        result = softmax(x)
        assert abs(sum(result.data) - 1.0) < 1e-6

    def test_all_positive(self):
        x = Tensor.from_list([1.0, 2.0, 3.0])
        result = softmax(x)
        for val in result.data:
            assert val > 0.0

    def test_largest_input_largest_output(self):
        x = Tensor.from_list([1.0, 5.0, 2.0])
        result = softmax(x)
        assert result.data[1] > result.data[0]
        assert result.data[1] > result.data[2]


class TestGELU:
    def test_positive_values(self):
        x = Tensor.from_list([1.0, 2.0])
        result = gelu(x)
        # GELU(1.0) ≈ 0.841
        assert abs(result.data[0] - 0.841) < 0.01

    def test_zero(self):
        x = Tensor.from_list([0.0])
        result = gelu(x)
        assert abs(result.data[0]) < 1e-6

    def test_negative(self):
        x = Tensor.from_list([-1.0])
        result = gelu(x)
        # GELU(-1) ≈ -0.159
        assert abs(result.data[0] - (-0.159)) < 0.01


class TestLinear:
    def test_identity(self):
        x = Tensor.from_list([1.0, 2.0, 3.0])
        result = linear(x)
        assert result.data == [1.0, 2.0, 3.0]

    def test_same_tensor(self):
        x = Tensor.from_list([1.0, 2.0])
        result = linear(x)
        assert result is x


class TestGetActivation:
    def test_string_relu(self):
        fn = get_activation("relu")
        assert fn is relu

    def test_string_sigmoid(self):
        fn = get_activation("sigmoid")
        assert fn is sigmoid

    def test_string_tanh(self):
        fn = get_activation("tanh")
        assert fn is tanh

    def test_string_softmax(self):
        fn = get_activation("softmax")
        assert fn is softmax

    def test_string_gelu(self):
        fn = get_activation("gelu")
        assert fn is gelu

    def test_string_linear(self):
        fn = get_activation("linear")
        assert fn is linear

    def test_string_case_insensitive(self):
        fn = get_activation("ReLU")
        assert fn is relu

    def test_none_returns_none(self):
        fn = get_activation(None)
        assert fn is None

    def test_callable_passthrough(self):
        def my_fn(x):
            return x

        fn = get_activation(my_fn)
        assert fn is my_fn

    def test_unknown_string_raises(self):
        with pytest.raises(ValueError, match="Unknown activation"):
            get_activation("unknown")

    def test_invalid_type_raises(self):
        with pytest.raises(TypeError):
            get_activation(42)
