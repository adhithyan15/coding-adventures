import math

import pytest

from loss_functions import (
    bce,
    bce_derivative,
    cce,
    cce_derivative,
    mae,
    mae_derivative,
    mse,
    mse_derivative,
)


def almost_equal(a: float, b: float) -> bool:
    return abs(a - b) <= 1e-6


def test_mse():
    y_true = [1.0, 0.0]
    y_pred = [0.9, 0.1]
    result = mse(y_true, y_pred)
    assert almost_equal(result, 0.010)


def test_mae():
    y_true = [1.0, 0.0]
    y_pred = [0.9, 0.1]
    result = mae(y_true, y_pred)
    assert almost_equal(result, 0.100)


def test_bce():
    y_true = [1.0, 0.0]
    y_pred = [0.9, 0.1]
    result = bce(y_true, y_pred)
    assert almost_equal(result, 0.1053605)


def test_cce():
    y_true = [1.0, 0.0]
    y_pred = [0.9, 0.1]
    result = cce(y_true, y_pred)
    assert almost_equal(result, 0.0526802)


def test_errors_on_mismatch():
    y_true = [1.0]
    y_pred = [0.9, 0.1]
    with pytest.raises(ValueError):
        mse(y_true, y_pred)
    with pytest.raises(ValueError):
        mae(y_true, y_pred)
    with pytest.raises(ValueError):
        bce(y_true, y_pred)
    with pytest.raises(ValueError):
        cce(y_true, y_pred)


def test_errors_on_empty():
    with pytest.raises(ValueError):
        mse([], [])
    with pytest.raises(ValueError):
        mae([], [])
    with pytest.raises(ValueError):
        bce([], [])
    with pytest.raises(ValueError):
        cce([], [])


def test_identical_slices():
    y_true = [1.0, 0.0, 0.5]
    y_pred = [1.0, 0.0, 0.5]
    assert almost_equal(mse(y_true, y_pred), 0.0)
    assert almost_equal(mae(y_true, y_pred), 0.0)


def test_mse_derivative():
    y_true = [1.0, 0.0]
    y_pred = [0.8, 0.2]
    result = mse_derivative(y_true, y_pred)
    assert almost_equal(result[0], -0.2)
    assert almost_equal(result[1], 0.2)


def test_mae_derivative():
    y_true = [1.0, 0.0, 0.5]
    y_pred = [0.8, 0.2, 0.5]
    result = mae_derivative(y_true, y_pred)
    assert almost_equal(result[0], -1.0 / 3.0)
    assert almost_equal(result[1], 1.0 / 3.0)
    assert almost_equal(result[2], 0.0)


def test_bce_derivative():
    y_true = [1.0, 0.0]
    y_pred = [0.8, 0.2]
    result = bce_derivative(y_true, y_pred)
    assert almost_equal(result[0], -0.625)
    assert almost_equal(result[1], 0.625)


def test_cce_derivative():
    y_true = [1.0, 0.0]
    y_pred = [0.8, 0.2]
    result = cce_derivative(y_true, y_pred)
    assert almost_equal(result[0], -0.625)
    assert almost_equal(result[1], 0.0)


def test_ml01_parity_vectors():
    assert mse([1.0, 0.0, 0.0], [0.9, 0.1, 0.2]) == pytest.approx(0.02)
    assert mae([1.0, 0.0, 0.0], [0.9, 0.1, 0.2]) == pytest.approx(0.1333333333)
    assert bce([1.0, 0.0, 1.0], [0.9, 0.1, 0.8]) == pytest.approx(0.1446215275)
    assert cce([1.0, 0.0, 0.0], [0.8, 0.1, 0.1]) == pytest.approx(0.07438118)


def test_cross_entropy_clamps_zero_and_one_predictions():
    epsilon = 1e-7

    assert bce([1.0, 0.0], [1.0, 0.0]) == pytest.approx(-math.log(1.0 - epsilon))
    assert bce_derivative([1.0, 0.0], [1.0, 0.0]) == pytest.approx(
        [-1.0 / (2.0 * (1.0 - epsilon)), 1.0 / (2.0 * (1.0 - epsilon))]
    )

    assert cce([1.0, 0.0], [0.0, 1.0]) == pytest.approx(-math.log(epsilon) / 2.0)
    assert cce_derivative([1.0, 0.0], [0.0, 1.0]) == pytest.approx([-1.0 / (2.0 * epsilon), 0.0])

    assert cce([0.0, 1.0], [0.0, 1.0]) == pytest.approx(-math.log(1.0 - epsilon) / 2.0)
    assert cce_derivative([0.0, 1.0], [0.0, 1.0]) == pytest.approx(
        [0.0, -1.0 / (2.0 * (1.0 - epsilon))]
    )


@pytest.mark.parametrize(
    "derivative",
    [mse_derivative, mae_derivative, bce_derivative, cce_derivative],
)
def test_derivatives_validate_inputs_and_return_fresh_lists(derivative):
    with pytest.raises(ValueError):
        derivative([], [])
    with pytest.raises(ValueError):
        derivative([1.0], [1.0, 0.0])

    y_true = [1.0, 0.0]
    y_pred = [0.8, 0.2]
    first = derivative(y_true, y_pred)
    second = derivative(y_true, y_pred)
    assert first == second
    assert first is not second
    assert len(first) == len(y_true)
    assert y_true == [1.0, 0.0]
    assert y_pred == [0.8, 0.2]
