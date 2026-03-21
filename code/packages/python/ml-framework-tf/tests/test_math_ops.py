"""Tests for tf.math — element-wise mathematical operations."""

import math as pymath
import ml_framework_tf as tf


class TestLog:
    def test_log_one(self):
        x = tf.constant([1.0])
        y = tf.math.log(x)
        assert abs(y.data[0] - 0.0) < 1e-6

    def test_log_e(self):
        x = tf.constant([pymath.e])
        y = tf.math.log(x)
        assert abs(y.data[0] - 1.0) < 1e-6

    def test_log_multiple(self):
        x = tf.constant([1.0, pymath.e, pymath.e**2])
        y = tf.math.log(x)
        assert abs(y.data[0] - 0.0) < 1e-6
        assert abs(y.data[1] - 1.0) < 1e-6
        assert abs(y.data[2] - 2.0) < 1e-6


class TestExp:
    def test_exp_zero(self):
        x = tf.constant([0.0])
        y = tf.math.exp(x)
        assert abs(y.data[0] - 1.0) < 1e-6

    def test_exp_one(self):
        x = tf.constant([1.0])
        y = tf.math.exp(x)
        assert abs(y.data[0] - pymath.e) < 1e-5

    def test_exp_negative(self):
        x = tf.constant([-1.0])
        y = tf.math.exp(x)
        assert abs(y.data[0] - 1.0 / pymath.e) < 1e-5


class TestSqrt:
    def test_sqrt_perfect(self):
        x = tf.constant([4.0, 9.0, 16.0])
        y = tf.math.sqrt(x)
        assert abs(y.data[0] - 2.0) < 1e-5
        assert abs(y.data[1] - 3.0) < 1e-5
        assert abs(y.data[2] - 4.0) < 1e-5

    def test_sqrt_one(self):
        x = tf.constant([1.0])
        y = tf.math.sqrt(x)
        assert abs(y.data[0] - 1.0) < 1e-6


class TestAbs:
    def test_positive(self):
        x = tf.constant([3.0])
        y = tf.math.abs(x)
        assert y.data[0] == 3.0

    def test_negative(self):
        x = tf.constant([-5.0])
        y = tf.math.abs(x)
        assert y.data[0] == 5.0

    def test_mixed(self):
        x = tf.constant([-2.0, 0.0, 3.0])
        y = tf.math.abs(x)
        assert y.data == [2.0, 0.0, 3.0]
