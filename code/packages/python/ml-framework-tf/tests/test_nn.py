"""Tests for tf.nn — activation functions."""

import ml_framework_tf as tf
from ml_framework_core import Tensor


class TestReLU:
    def test_positive(self):
        x = tf.constant([1.0, 2.0, 3.0])
        y = tf.nn.relu(x)
        assert y.data == [1.0, 2.0, 3.0]

    def test_negative(self):
        x = tf.constant([-1.0, -2.0, -3.0])
        y = tf.nn.relu(x)
        assert y.data == [0.0, 0.0, 0.0]

    def test_mixed(self):
        x = tf.constant([-2.0, 0.0, 3.0])
        y = tf.nn.relu(x)
        assert y.data == [0.0, 0.0, 3.0]


class TestSigmoid:
    def test_zero(self):
        x = tf.constant([0.0])
        y = tf.nn.sigmoid(x)
        assert abs(y.data[0] - 0.5) < 1e-6

    def test_large_positive(self):
        x = tf.constant([10.0])
        y = tf.nn.sigmoid(x)
        assert y.data[0] > 0.99

    def test_large_negative(self):
        x = tf.constant([-10.0])
        y = tf.nn.sigmoid(x)
        assert y.data[0] < 0.01

    def test_range(self):
        x = tf.constant([-2.0, 0.0, 2.0])
        y = tf.nn.sigmoid(x)
        for val in y.data:
            assert 0.0 < val < 1.0


class TestSoftmax:
    def test_sums_to_one(self):
        x = tf.constant([2.0, 1.0, 0.1])
        y = tf.nn.softmax(x)
        assert abs(sum(y.data) - 1.0) < 1e-6

    def test_largest_input_gets_largest_prob(self):
        x = tf.constant([2.0, 1.0, 0.1])
        y = tf.nn.softmax(x)
        assert y.data[0] > y.data[1] > y.data[2]

    def test_uniform_input(self):
        x = tf.constant([1.0, 1.0, 1.0])
        y = tf.nn.softmax(x)
        for val in y.data:
            assert abs(val - 1.0 / 3.0) < 1e-6

    def test_2d_axis_minus_1(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        y = tf.nn.softmax(x, axis=-1)
        # Each row should sum to 1
        row1_sum = y.data[0] + y.data[1]
        row2_sum = y.data[2] + y.data[3]
        assert abs(row1_sum - 1.0) < 1e-6
        assert abs(row2_sum - 1.0) < 1e-6


class TestGELU:
    def test_zero(self):
        x = tf.constant([0.0])
        y = tf.nn.gelu(x)
        assert abs(y.data[0]) < 1e-6

    def test_positive(self):
        x = tf.constant([2.0])
        y = tf.nn.gelu(x)
        assert y.data[0] > 1.9  # GELU(2) ≈ 1.95

    def test_negative_suppressed(self):
        x = tf.constant([-3.0])
        y = tf.nn.gelu(x)
        assert abs(y.data[0]) < 0.01
