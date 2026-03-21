"""Tests for tf.keras.optimizers — SGD, Adam, RMSprop, AdamW."""

from ml_framework_core import Parameter, Tensor
from ml_framework_tf.keras.optimizers import SGD, Adam, AdamW, Optimizer, RMSprop


class TestSGD:
    def test_basic_step(self):
        """w = w - lr * grad; with lr=0.1, grad=1.0, w=5.0 → 4.9"""
        w = Parameter(Tensor.from_list([5.0]))
        opt = SGD(learning_rate=0.1)
        grad = Tensor.from_list([1.0])
        opt.apply_gradients([(grad, w)])
        assert abs(w.data[0] - 4.9) < 1e-6

    def test_momentum(self):
        w = Parameter(Tensor.from_list([5.0]))
        opt = SGD(learning_rate=0.1, momentum=0.9)
        grad = Tensor.from_list([1.0])
        # Step 1: v = 0.9*0 + 1.0 = 1.0, w = 5.0 - 0.1*1.0 = 4.9
        opt.apply_gradients([(grad, w)])
        assert abs(w.data[0] - 4.9) < 1e-6
        # Step 2: v = 0.9*1.0 + 1.0 = 1.9, w = 4.9 - 0.1*1.9 = 4.71
        opt.apply_gradients([(grad, w)])
        assert abs(w.data[0] - 4.71) < 1e-5

    def test_skip_none_grad(self):
        w = Parameter(Tensor.from_list([5.0]))
        opt = SGD(learning_rate=0.1)
        opt.apply_gradients([(None, w)])
        assert w.data[0] == 5.0

    def test_multiple_params(self):
        w1 = Parameter(Tensor.from_list([1.0, 2.0]))
        w2 = Parameter(Tensor.from_list([3.0]))
        opt = SGD(learning_rate=0.5)
        g1 = Tensor.from_list([0.1, 0.2])
        g2 = Tensor.from_list([0.3])
        opt.apply_gradients([(g1, w1), (g2, w2)])
        assert abs(w1.data[0] - 0.95) < 1e-6
        assert abs(w2.data[0] - 2.85) < 1e-6


class TestAdam:
    def test_basic_step(self):
        w = Parameter(Tensor.from_list([5.0]))
        opt = Adam(learning_rate=0.1)
        grad = Tensor.from_list([1.0])
        opt.apply_gradients([(grad, w)])
        # After one step, weight should decrease
        assert w.data[0] < 5.0

    def test_convergence(self):
        """Adam should move weight toward zero with constant grad."""
        w = Parameter(Tensor.from_list([10.0]))
        opt = Adam(learning_rate=0.5)
        for _ in range(50):
            grad = Tensor.from_list([1.0])
            opt.apply_gradients([(grad, w)])
        assert w.data[0] < 5.0  # should have decreased significantly

    def test_iterations_count(self):
        opt = Adam()
        w = Parameter(Tensor.from_list([1.0]))
        grad = Tensor.from_list([0.1])
        opt.apply_gradients([(grad, w)])
        assert opt._iterations == 1
        opt.apply_gradients([(grad, w)])
        assert opt._iterations == 2


class TestRMSprop:
    def test_basic_step(self):
        w = Parameter(Tensor.from_list([5.0]))
        opt = RMSprop(learning_rate=0.1)
        grad = Tensor.from_list([1.0])
        opt.apply_gradients([(grad, w)])
        assert w.data[0] < 5.0

    def test_adaptive_lr(self):
        """Large gradients should get smaller effective LR."""
        w1 = Parameter(Tensor.from_list([5.0]))
        w2 = Parameter(Tensor.from_list([5.0]))
        opt = RMSprop(learning_rate=0.1)
        small_grad = Tensor.from_list([0.01])
        large_grad = Tensor.from_list([10.0])
        opt.apply_gradients([(small_grad, w1), (large_grad, w2)])
        # w2 should move less relative to its gradient magnitude
        delta1 = 5.0 - w1.data[0]
        delta2 = 5.0 - w2.data[0]
        assert delta2 / 10.0 < delta1 / 0.01  # smaller effective LR for large grad


class TestAdamW:
    def test_weight_decay(self):
        """With weight decay, weights should shrink even without gradients."""
        w = Parameter(Tensor.from_list([10.0]))
        opt = AdamW(learning_rate=0.01, weight_decay=0.1)
        grad = Tensor.from_list([0.0])  # zero gradient
        opt.apply_gradients([(grad, w)])
        # Weight should shrink due to decay
        assert w.data[0] < 10.0

    def test_basic_step(self):
        w = Parameter(Tensor.from_list([5.0]))
        opt = AdamW(learning_rate=0.1)
        grad = Tensor.from_list([1.0])
        opt.apply_gradients([(grad, w)])
        assert w.data[0] < 5.0


class TestOptimizerBase:
    def test_abstract_apply(self):
        opt = Optimizer(learning_rate=0.01)
        w = Parameter(Tensor.from_list([1.0]))
        grad = Tensor.from_list([0.1])
        import pytest

        with pytest.raises(NotImplementedError):
            opt.apply_gradients([(grad, w)])
