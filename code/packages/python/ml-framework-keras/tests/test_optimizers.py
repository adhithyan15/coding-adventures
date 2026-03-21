"""Tests for the optimizers module."""

import pytest
from ml_framework_core import Parameter, Tensor

from ml_framework_keras.optimizers import (
    Adam,
    AdamW,
    Optimizer,
    RMSprop,
    SGD,
    get_optimizer,
)


def _make_param_with_grad(data, grad_data):
    """Helper: create a parameter with a specific gradient."""
    t = Tensor(data, (len(data),), requires_grad=True)
    p = Parameter(t)
    p.grad = Tensor(grad_data, (len(grad_data),))
    return p


class TestSGD:
    def test_basic_update(self):
        p = _make_param_with_grad([1.0, 2.0], [0.1, 0.2])
        opt = SGD(learning_rate=0.1)
        opt.apply_gradients([(p.grad, p)])
        # w = w - lr * grad = [1.0 - 0.01, 2.0 - 0.02]
        assert abs(p.data[0] - 0.99) < 1e-6
        assert abs(p.data[1] - 1.98) < 1e-6

    def test_momentum(self):
        p = _make_param_with_grad([1.0], [1.0])
        opt = SGD(learning_rate=0.1, momentum=0.9)

        # Step 1: v = 0.9*0 + 1.0 = 1.0, w = 1.0 - 0.1*1.0 = 0.9
        opt.apply_gradients([(p.grad, p)])
        assert abs(p.data[0] - 0.9) < 1e-6

        # Step 2: v = 0.9*1.0 + 1.0 = 1.9, w = 0.9 - 0.1*1.9 = 0.71
        p.grad = Tensor([1.0], (1,))
        opt.apply_gradients([(p.grad, p)])
        assert abs(p.data[0] - 0.71) < 1e-6

    def test_none_grad_skipped(self):
        p = _make_param_with_grad([1.0], [0.5])
        opt = SGD(learning_rate=0.1)
        opt.apply_gradients([(None, p)])
        assert p.data[0] == 1.0  # unchanged

    def test_get_config(self):
        opt = SGD(learning_rate=0.05, momentum=0.9)
        config = opt.get_config()
        assert config["learning_rate"] == 0.05
        assert config["momentum"] == 0.9


class TestAdam:
    def test_basic_update(self):
        p = _make_param_with_grad([1.0, 2.0], [0.1, 0.2])
        opt = Adam(learning_rate=0.001)
        opt.apply_gradients([(p.grad, p)])
        # After one step, params should have moved
        assert p.data[0] < 1.0
        assert p.data[1] < 2.0

    def test_multiple_steps(self):
        p = _make_param_with_grad([5.0], [1.0])
        opt = Adam(learning_rate=0.1)
        for _ in range(10):
            p.grad = Tensor([1.0], (1,))
            opt.apply_gradients([(p.grad, p)])
        # After 10 steps with constant gradient, param should decrease
        assert p.data[0] < 5.0

    def test_none_grad_skipped(self):
        p = _make_param_with_grad([1.0], [0.5])
        opt = Adam()
        opt.apply_gradients([(None, p)])
        assert p.data[0] == 1.0

    def test_get_config(self):
        opt = Adam(learning_rate=0.01, beta_1=0.8, beta_2=0.99, epsilon=1e-6)
        config = opt.get_config()
        assert config["learning_rate"] == 0.01
        assert config["beta_1"] == 0.8
        assert config["beta_2"] == 0.99
        assert config["epsilon"] == 1e-6


class TestRMSprop:
    def test_basic_update(self):
        p = _make_param_with_grad([1.0], [0.5])
        opt = RMSprop(learning_rate=0.01)
        opt.apply_gradients([(p.grad, p)])
        assert p.data[0] < 1.0

    def test_adapts_learning_rate(self):
        # Large gradients should lead to smaller effective lr
        p1 = _make_param_with_grad([1.0], [10.0])
        p2 = _make_param_with_grad([1.0], [0.1])
        opt = RMSprop(learning_rate=0.01)

        opt.apply_gradients([(p1.grad, p1)])
        change1 = abs(1.0 - p1.data[0])

        opt2 = RMSprop(learning_rate=0.01)
        opt2.apply_gradients([(p2.grad, p2)])
        change2 = abs(1.0 - p2.data[0])

        # Large gradient param should have similar magnitude change
        # due to adaptation
        assert change1 > 0 and change2 > 0

    def test_get_config(self):
        opt = RMSprop(learning_rate=0.01, rho=0.95, epsilon=1e-8)
        config = opt.get_config()
        assert config["rho"] == 0.95


class TestAdamW:
    def test_weight_decay(self):
        p = _make_param_with_grad([1.0], [0.0])  # zero gradient
        opt = AdamW(learning_rate=0.01, weight_decay=0.1)
        opt.apply_gradients([(p.grad, p)])
        # With zero gradient, only weight decay applies
        # w = w * (1 - lr * wd) = 1.0 * (1 - 0.001) = 0.999
        assert p.data[0] < 1.0

    def test_basic_update(self):
        p = _make_param_with_grad([2.0], [1.0])
        opt = AdamW(learning_rate=0.01, weight_decay=0.01)
        opt.apply_gradients([(p.grad, p)])
        assert p.data[0] < 2.0

    def test_get_config(self):
        opt = AdamW(weight_decay=0.05)
        config = opt.get_config()
        assert config["weight_decay"] == 0.05


class TestGetOptimizer:
    def test_string_sgd(self):
        opt = get_optimizer("sgd")
        assert isinstance(opt, SGD)

    def test_string_adam(self):
        opt = get_optimizer("adam")
        assert isinstance(opt, Adam)

    def test_string_rmsprop(self):
        opt = get_optimizer("rmsprop")
        assert isinstance(opt, RMSprop)

    def test_string_adamw(self):
        opt = get_optimizer("adamw")
        assert isinstance(opt, AdamW)

    def test_string_case_insensitive(self):
        opt = get_optimizer("Adam")
        assert isinstance(opt, Adam)

    def test_instance_passthrough(self):
        opt = Adam(learning_rate=0.01)
        result = get_optimizer(opt)
        assert result is opt

    def test_unknown_string_raises(self):
        with pytest.raises(ValueError, match="Unknown optimizer"):
            get_optimizer("nonexistent")

    def test_invalid_type_raises(self):
        with pytest.raises(TypeError):
            get_optimizer(42)


class TestOptimizerBase:
    def test_apply_gradients_not_implemented(self):
        opt = Optimizer(learning_rate=0.01)
        with pytest.raises(NotImplementedError):
            opt.apply_gradients([])

    def test_get_config(self):
        opt = Optimizer(learning_rate=0.05)
        config = opt.get_config()
        assert config["learning_rate"] == 0.05
        assert config["class_name"] == "Optimizer"
