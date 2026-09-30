import math

import pytest

from activation_functions import (
    leaky_relu,
    leaky_relu_derivative,
    linear,
    linear_derivative,
    relu,
    relu_derivative,
    sigmoid,
    sigmoid_derivative,
    softplus,
    softplus_derivative,
    tanh,
    tanh_derivative,
    tanh_func,
)


@pytest.mark.parametrize("value", [-3.5, 0.0, 7.25])
def test_linear_and_derivative(value: float) -> None:
    assert linear(value) == value
    assert linear_derivative(value) == 1.0


def test_sigmoid_reference_values_and_derivative() -> None:
    assert sigmoid(0.0) == 0.5
    assert sigmoid_derivative(0.0) == 0.25
    assert sigmoid(1.0) == pytest.approx(0.7310585786300049)
    assert sigmoid(-1.0) == pytest.approx(1.0 - sigmoid(1.0))


def test_sigmoid_clamps_overflow_boundaries() -> None:
    assert sigmoid(-710.0) == 0.0
    assert sigmoid(710.0) == 1.0
    assert sigmoid_derivative(-710.0) == 0.0
    assert sigmoid_derivative(710.0) == 0.0


@pytest.mark.parametrize(
    ("value", "expected", "derivative"),
    [(-2.0, 0.0, 0.0), (0.0, 0.0, 0.0), (2.0, 2.0, 1.0)],
)
def test_relu_zero_convention(value: float, expected: float, derivative: float) -> None:
    assert relu(value) == expected
    assert relu_derivative(value) == derivative


@pytest.mark.parametrize(
    ("value", "expected", "derivative"),
    [(-2.0, -0.02, 0.01), (0.0, 0.0, 0.01), (2.0, 2.0, 1.0)],
)
def test_leaky_relu_zero_convention(value: float, expected: float, derivative: float) -> None:
    assert leaky_relu(value) == pytest.approx(expected)
    assert leaky_relu_derivative(value) == derivative


def test_tanh_canonical_name_and_compatibility_alias() -> None:
    assert tanh(0.75) == pytest.approx(math.tanh(0.75))
    assert tanh_func(0.75) == tanh(0.75)
    assert tanh_derivative(0.0) == 1.0


def test_softplus_is_stable_for_large_values() -> None:
    assert softplus(1000.0) == pytest.approx(1000.0)
    assert softplus(-1000.0) == pytest.approx(0.0)
    assert softplus_derivative(0.0) == 0.5


@pytest.mark.parametrize("value", [-10.0, -1.0, 0.0, 1.0, 10.0])
def test_activation_ranges(value: float) -> None:
    assert 0.0 <= sigmoid(value) <= 1.0
    assert -1.0 <= tanh(value) <= 1.0
    assert softplus(value) >= 0.0
