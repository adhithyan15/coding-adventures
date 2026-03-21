"""Tests for tf.GradientTape — explicit gradient computation."""

import pytest
import ml_framework_tf as tf
from ml_framework_tf.variable import Variable
from ml_framework_tf.gradient_tape import GradientTape


class TestBasicGradients:
    """Test basic gradient computation with GradientTape."""

    def test_simple_square(self):
        """d/dx(x^2) = 2x at x=[1,2,3] → [2,4,6]."""
        x = Variable([1.0, 2.0, 3.0])
        with GradientTape() as tape:
            tape.watch(x)
            y = x * x
            loss = tf.reduce_sum(y)
        grads = tape.gradient(loss, [x])
        assert abs(grads[0].data[0] - 2.0) < 1e-5
        assert abs(grads[0].data[1] - 4.0) < 1e-5
        assert abs(grads[0].data[2] - 6.0) < 1e-5

    def test_linear_gradient(self):
        """d/dx(2*x) = 2 everywhere."""
        x = Variable([1.0, 2.0, 3.0])
        with GradientTape() as tape:
            tape.watch(x)
            y = x * 2.0
            loss = tf.reduce_sum(y)
        grads = tape.gradient(loss, [x])
        for g in grads[0].data:
            assert abs(g - 2.0) < 1e-5

    def test_multiple_sources(self):
        """Compute gradients w.r.t. multiple variables."""
        w = Variable([2.0, 3.0])
        b = Variable([1.0, 1.0])
        x = tf.constant([1.0, 1.0])

        with GradientTape() as tape:
            tape.watch(w)
            tape.watch(b)
            y = w * x + b
            loss = tf.reduce_sum(y)

        grads = tape.gradient(loss, [w, b])
        # dL/dw = x = [1, 1]
        assert abs(grads[0].data[0] - 1.0) < 1e-5
        # dL/db = 1
        assert abs(grads[1].data[0] - 1.0) < 1e-5


class TestTapeConsumption:
    """Test that non-persistent tapes are consumed after one gradient call."""

    def test_tape_consumed(self):
        x = Variable([1.0])
        with GradientTape() as tape:
            tape.watch(x)
            loss = tf.reduce_sum(x * x)
        tape.gradient(loss, [x])
        with pytest.raises(RuntimeError, match="non-persistent"):
            tape.gradient(loss, [x])

    def test_persistent_tape(self):
        x = Variable([1.0, 2.0])
        with GradientTape(persistent=True) as tape:
            tape.watch(x)
            y = x * x
            loss = tf.reduce_sum(y)

        grads1 = tape.gradient(loss, [x])
        grads2 = tape.gradient(loss, [x])
        assert grads1[0].data == grads2[0].data


class TestWatchConstant:
    """Test watching non-Variable tensors."""

    def test_watch_constant(self):
        x = tf.constant([1.0, 2.0, 3.0])
        with GradientTape() as tape:
            tape.watch(x)
            y = x * x
            loss = tf.reduce_sum(y)
        grads = tape.gradient(loss, [x])
        assert abs(grads[0].data[0] - 2.0) < 1e-5

    def test_no_watch_no_grad(self):
        """Without watch, constant has no gradient path."""
        x = Variable([1.0, 2.0])
        y = tf.constant([3.0, 4.0])
        with GradientTape() as tape:
            tape.watch(x)
            # y is not watched
            loss = tf.reduce_sum(x * x)
        grads = tape.gradient(loss, [x, y])
        assert grads[0] is not None
        # y may have None grad since it's not in the computation graph
