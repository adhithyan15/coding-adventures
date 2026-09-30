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


@pytest.mark.parametrize(
    ("value", "expected", "expected_derivative"),
    [
        (0.0, 0.5, 0.25),
        (1.0, 0.7310585786300049, 0.19661193324148185),
        (10.0, 0.9999546021312976, 0.000045395807735907655),
    ],
)
def test_sigmoid_reference_values_and_derivative(
    value: float, expected: float, expected_derivative: float
) -> None:
    assert sigmoid(value) == pytest.approx(expected)
    assert sigmoid_derivative(value) == pytest.approx(expected_derivative)


def test_sigmoid_negative_reference_value() -> None:
    assert sigmoid(-1.0) == pytest.approx(0.2689414213699951)


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


@pytest.mark.parametrize(
    ("value", "expected", "expected_derivative"),
    [
        (0.0, 0.0, 1.0),
        (1.0, 0.7615941559557649, 0.4199743416140261),
        (-1.0, -0.7615941559557649, 0.4199743416140261),
    ],
)
def test_tanh_reference_values(value: float, expected: float, expected_derivative: float) -> None:
    assert tanh(value) == pytest.approx(expected)
    assert tanh_derivative(value) == pytest.approx(expected_derivative)


def test_tanh_compatibility_alias() -> None:
    assert tanh_func(0.75) == tanh(0.75)


@pytest.mark.parametrize(
    ("value", "expected", "expected_derivative"),
    [
        (0.0, 0.6931471805599453, 0.5),
        (1.0, 1.3132616875182228, 0.7310585786300049),
        (-1.0, 0.31326168751822286, 0.2689414213699951),
    ],
)
def test_softplus_reference_values(
    value: float, expected: float, expected_derivative: float
) -> None:
    assert softplus(value) == pytest.approx(expected)
    assert softplus_derivative(value) == pytest.approx(expected_derivative)


def test_softplus_is_stable_for_large_values() -> None:
    assert softplus(1000.0) == pytest.approx(1000.0)
    assert softplus(-1000.0) == pytest.approx(0.0)


@pytest.mark.parametrize("value", [-10.0, -1.0, 0.0, 1.0, 10.0])
def test_activation_ranges(value: float) -> None:
    assert 0.0 < sigmoid(value) < 1.0
    assert -1.0 < tanh(value) < 1.0
    assert softplus(value) >= 0.0


@pytest.mark.parametrize("value", [-10.0, -1.0, 0.0, 1.0, 10.0])
def test_activation_properties(value: float) -> None:
    assert sigmoid(-value) == pytest.approx(1.0 - sigmoid(value))
    assert relu(relu(value)) == relu(value)
    assert tanh(-value) == pytest.approx(-tanh(value))
    assert softplus_derivative(value) == pytest.approx(sigmoid(value))

    derivatives = [
        linear_derivative(value),
        sigmoid_derivative(value),
        relu_derivative(value),
        leaky_relu_derivative(value),
        tanh_derivative(value),
        softplus_derivative(value),
    ]
    assert all(derivative >= 0.0 for derivative in derivatives)
