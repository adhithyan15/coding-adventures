"""End-to-end integration tests: training, multi-layer nets, cross-entropy."""

import math

from ml_framework_core.functions import (
    GELUFunction,
    ReLUFunction,
    SigmoidFunction,
    SoftmaxFunction,
    TanhFunction,
)
from ml_framework_core.parameter import Parameter
from ml_framework_core.tensor import Tensor

# =========================================================================
# Linear regression: y = Wx + b, train with MSE loss
# =========================================================================


class TestLinearRegression:
    def test_single_step_gradient(self):
        """One step of gradient descent on a simple linear model."""
        x = Tensor.from_list([[1.0], [2.0], [3.0]])
        y_true = Tensor.from_list([[3.0], [5.0], [7.0]])

        w = Parameter(Tensor.from_list([[0.5]]))
        b_data = Tensor.from_list([[0.1], [0.1], [0.1]])

        y_pred = x @ w + b_data
        diff = y_pred - y_true
        loss = (diff * diff).mean()

        loss.backward()
        assert w.grad is not None

    def test_training_loop_converges(self):
        """Train a linear regression model to convergence."""
        x = Tensor.from_list([[1.0], [2.0], [3.0], [4.0]])
        y_true = Tensor.from_list(
            [[2.0], [4.0], [6.0], [8.0]]
        )

        w = Parameter(Tensor.from_list([[0.1]]))
        lr = 0.05

        for _ in range(100):
            y_pred = x @ w
            diff = y_pred - y_true
            loss = (diff * diff).mean()

            w.grad = None
            loss.backward()

            w.data = [
                v - lr * g
                for v, g in zip(w.data, w.grad.data, strict=True)
            ]

        assert abs(w.data[0] - 2.0) < 0.1

    def test_training_with_bias(self):
        """Train y = Wx + b to learn y = 3x + 1."""
        x_data = [[1.0], [2.0], [3.0], [4.0]]
        y_data = [[4.0], [7.0], [10.0], [13.0]]

        w = Parameter(Tensor.from_list([[0.0]]))
        b_val = 0.0
        lr = 0.01

        for _ in range(500):
            x = Tensor.from_list(x_data)
            y_true = Tensor.from_list(y_data)

            b_param = Parameter(Tensor.full((4, 1), b_val))
            y_pred = x @ w + b_param
            diff = y_pred - y_true
            loss = (diff * diff).mean()

            w.grad = None
            loss.backward()

            w.data = [
                v - lr * g
                for v, g in zip(
                    w.data, w.grad.data, strict=True
                )
            ]
            b_grad = sum(b_param.grad.data) / len(
                b_param.grad.data
            )
            b_val -= lr * b_grad

        assert abs(w.data[0] - 3.0) < 0.3
        assert abs(b_val - 1.0) < 0.5


# =========================================================================
# Multi-layer network with ReLU
# =========================================================================


class TestMultiLayerNetwork:
    def test_two_layer_forward(self):
        """y = relu(w2 @ relu(w1 @ x))."""
        x = Tensor.from_list([[1.0], [2.0]])

        w1 = Parameter(
            Tensor.from_list([[0.5, 0.3], [-0.2, 0.8]])
        )
        w2 = Parameter(
            Tensor.from_list([[0.4, -0.1], [0.6, 0.2]])
        )

        h = w1 @ x
        h_relu = ReLUFunction.apply(h)
        out = w2 @ h_relu
        out_relu = ReLUFunction.apply(out)

        assert out_relu.shape == (2, 1)
        for v in out_relu.data:
            assert v >= 0.0

    def test_gradients_flow_through_layers(self):
        """Verify gradients propagate through multiple layers."""
        x = Tensor.from_list([[1.0], [0.5]])

        w1 = Parameter(
            Tensor.from_list([[0.5, 0.3], [-0.2, 0.8]])
        )
        w2 = Parameter(Tensor.from_list([[0.4, -0.1]]))

        h = w1 @ x
        h_relu = ReLUFunction.apply(h)
        out = w2 @ h_relu
        loss = out.sum()

        loss.backward()

        assert w1.grad is not None
        assert w2.grad is not None
        assert w1.grad.shape == (2, 2)
        assert w2.grad.shape == (1, 2)

    def test_relu_kills_gradient(self):
        """When ReLU input is negative, gradient is zero."""
        x = Tensor.from_list([-1.0], requires_grad=True)
        y = ReLUFunction.apply(x).sum()
        y.backward()
        assert x.grad.data[0] == 0.0

    def test_multi_layer_training_step(self):
        """One training step on a 2-layer network reduces loss."""
        x = Tensor.from_list([[1.0], [2.0]])
        y_true = Tensor.from_list([[1.0]])

        w1 = Parameter(
            Tensor.from_list([[0.1, 0.2], [0.3, 0.4]])
        )
        w2 = Parameter(Tensor.from_list([[0.5, 0.6]]))

        h = ReLUFunction.apply(w1 @ x)
        y_pred = w2 @ h
        diff = y_pred - y_true
        loss1 = (diff * diff).sum()
        loss1_val = loss1.data[0]

        loss1.backward()

        lr = 0.01
        w1.data = [
            v - lr * g
            for v, g in zip(
                w1.data, w1.grad.data, strict=True
            )
        ]
        w2.data = [
            v - lr * g
            for v, g in zip(
                w2.data, w2.grad.data, strict=True
            )
        ]

        w1_new = Parameter(Tensor(list(w1.data), w1.shape))
        w2_new = Parameter(Tensor(list(w2.data), w2.shape))
        h2 = ReLUFunction.apply(w1_new @ x)
        y_pred2 = w2_new @ h2
        diff2 = y_pred2 - y_true
        loss2 = (diff2 * diff2).sum()

        assert loss2.data[0] < loss1_val


# =========================================================================
# Activation functions in networks
# =========================================================================


class TestActivationsInNetworks:
    def test_sigmoid_output_range(self):
        """Sigmoid output should be in (0, 1)."""
        x = Tensor.from_list([-10.0, -1.0, 0.0, 1.0, 10.0])
        y = SigmoidFunction.apply(x)
        for v in y.data:
            assert 0.0 < v < 1.0

    def test_tanh_output_range(self):
        """Tanh output should be in (-1, 1)."""
        x = Tensor.from_list([-10.0, -1.0, 0.0, 1.0, 10.0])
        y = TanhFunction.apply(x)
        for v in y.data:
            assert -1.0 < v < 1.0

    def test_gelu_at_various_points(self):
        x = Tensor.from_list([-2.0, -1.0, 0.0, 1.0, 2.0])
        y = GELUFunction.apply(x)
        assert abs(y.data[2]) < 1e-10
        assert y.data[3] > 0
        assert y.data[4] > 0

    def test_sigmoid_gradient_chain(self):
        """Gradient through sigmoid layer."""
        x = Tensor.from_list([0.0], requires_grad=True)
        y = SigmoidFunction.apply(x)
        loss = y.sum()
        loss.backward()
        assert abs(x.grad.data[0] - 0.25) < 1e-10

    def test_tanh_gradient_chain(self):
        x = Tensor.from_list([0.0], requires_grad=True)
        y = TanhFunction.apply(x)
        loss = y.sum()
        loss.backward()
        assert abs(x.grad.data[0] - 1.0) < 1e-10


# =========================================================================
# Softmax cross-entropy
# =========================================================================


class TestSoftmaxCrossEntropy:
    def test_softmax_cross_entropy_forward(self):
        """Cross-entropy: -sum(y_true * log(softmax(logits)))."""
        logits = Tensor.from_list([2.0, 1.0, 0.1])
        target = Tensor.from_list([1.0, 0.0, 0.0])

        probs = SoftmaxFunction.apply(logits, 0)
        log_probs = probs.log()
        loss = -(target * log_probs).sum()

        assert loss.data[0] > 0

    def test_softmax_cross_entropy_gradient(self):
        """Gradient flows through softmax -> log -> sum."""
        logits = Tensor.from_list(
            [2.0, 1.0, 0.1], requires_grad=True
        )
        target = Tensor.from_list([1.0, 0.0, 0.0])

        probs = SoftmaxFunction.apply(logits, 0)
        log_probs = probs.log()
        loss = -(target * log_probs).sum()

        loss.backward()
        assert logits.grad is not None
        assert logits.grad.data[0] < 0
        assert logits.grad.data[1] > 0
        assert logits.grad.data[2] > 0

    def test_correct_class_has_smallest_loss(self):
        """Correct class logit produces the smallest loss."""
        target_class = 0
        target = Tensor.from_list([1.0, 0.0, 0.0])

        losses = []
        for boost_class in range(3):
            logits_vals = [1.0, 1.0, 1.0]
            logits_vals[boost_class] = 5.0
            logits_t = Tensor.from_list(logits_vals)
            probs = SoftmaxFunction.apply(logits_t, 0)
            log_probs = probs.log()
            loss = -(target * log_probs).sum()
            losses.append(loss.data[0])

        assert losses[target_class] < losses[1]
        assert losses[target_class] < losses[2]

    def test_softmax_ce_2d_batch(self):
        """Cross-entropy on a batch of 2D logits."""
        logits = Tensor.from_list(
            [[2.0, 0.5], [0.5, 2.0]], requires_grad=True
        )
        targets = Tensor.from_list(
            [[1.0, 0.0], [0.0, 1.0]]
        )

        probs = SoftmaxFunction.apply(logits, 1)
        log_probs = probs.log()
        loss = -(targets * log_probs).sum()

        loss.backward()
        assert logits.grad is not None
        assert logits.grad.shape == (2, 2)


# =========================================================================
# Complex computation graphs
# =========================================================================


class TestComplexGraphs:
    def test_shared_parameter_multiple_losses(self):
        """Same weight used in two different loss terms."""
        w = Parameter(Tensor.from_list([[1.0, 0.5]]))
        x1 = Tensor.from_list([[1.0], [0.0]])
        x2 = Tensor.from_list([[0.0], [1.0]])

        y1 = (w @ x1).sum()
        y1.backward()
        grad1 = list(w.grad.data)

        w.grad = None
        w2 = Parameter(Tensor(list(w.data), w.shape))
        y2_fresh = (w2 @ x2).sum()
        y2_fresh.backward()

        assert len(grad1) == 2
        assert w2.grad is not None

    def test_diamond_graph(self):
        """x -> a, x -> b, y = a + b (diamond pattern)."""
        x = Tensor.from_list([3.0], requires_grad=True)
        a = x * 2.0
        b = x * 3.0
        y = (a + b).sum()
        y.backward()
        # dy/dx = 2 + 3 = 5
        assert abs(x.grad.data[0] - 5.0) < 1e-10

    def test_long_chain(self):
        """x -> *2 -> *2 -> *2 -> *2 -> sum. Grad = 16."""
        x = Tensor.from_list([1.0], requires_grad=True)
        y = x
        for _ in range(4):
            y = y * 2.0
        z = y.sum()
        z.backward()
        assert abs(x.grad.data[0] - 16.0) < 1e-10

    def test_mse_loss_pattern(self):
        """loss = mean((pred - target)^2)."""
        pred = Tensor.from_list(
            [1.5, 2.5, 3.5], requires_grad=True
        )
        target = Tensor.from_list([1.0, 2.0, 3.0])

        diff = pred - target
        sq = diff**2.0
        loss = sq.mean()
        loss.backward()

        for g in pred.grad.data:
            assert abs(g - 1.0 / 3.0) < 1e-10

    def test_l1_loss_pattern(self):
        """L1 loss: mean(|pred - target|)."""
        pred = Tensor.from_list(
            [1.5, 2.5, 3.5], requires_grad=True
        )
        target = Tensor.from_list([1.0, 2.0, 3.0])

        diff = pred - target
        l1 = diff.abs()
        loss = l1.mean()
        loss.backward()

        for g in pred.grad.data:
            assert abs(g - 1.0 / 3.0) < 1e-10

    def test_exp_sum_log_roundtrip(self):
        """log(exp(x)) should give back x."""
        x = Tensor.from_list(
            [1.0, 2.0, 3.0], requires_grad=True
        )
        y = x.exp().log()
        loss = y.sum()
        loss.backward()
        for g in x.grad.data:
            assert abs(g - 1.0) < 1e-6

    def test_numerical_stability_softmax(self):
        """Softmax handles large inputs without overflow."""
        logits = Tensor.from_list(
            [100.0, 101.0, 102.0], requires_grad=True
        )
        probs = SoftmaxFunction.apply(logits, 0)
        loss = probs.sum()
        loss.backward()
        for v in probs.data:
            assert math.isfinite(v)
        for g in logits.grad.data:
            assert math.isfinite(g)
