import math
import pytest
from loss_functions import mse, mae, bce, cce

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
