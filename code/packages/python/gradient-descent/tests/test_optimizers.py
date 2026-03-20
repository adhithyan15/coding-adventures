import pytest
from gradient_descent import sgd

def test_sgd():
    weights = [1.0, -0.5, 2.0]
    gradients = [0.1, -0.2, 0.0]
    lr = 0.1
    res = sgd(weights, gradients, lr)
    
    assert abs(res[0] - 0.99) < 1e-6
    assert abs(res[1] - -0.48) < 1e-6
    assert abs(res[2] - 2.0) < 1e-6

def test_sgd_errors():
    with pytest.raises(ValueError):
        sgd([1.0], [], 0.1)
    with pytest.raises(ValueError):
        sgd([], [], 0.1)
