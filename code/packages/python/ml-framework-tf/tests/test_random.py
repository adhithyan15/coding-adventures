"""Tests for tf.random — random tensor generation."""

import ml_framework_tf as tf


class TestRandomNormal:
    def test_shape(self):
        x = tf.random.normal((3, 4))
        assert x.shape == (3, 4)
        assert len(x.data) == 12

    def test_shape_list(self):
        x = tf.random.normal([2, 5])
        assert x.shape == (2, 5)

    def test_default_mean_and_std(self):
        # With enough samples, mean should be near 0, std near 1
        x = tf.random.normal((1000,))
        mean = sum(x.data) / len(x.data)
        var = sum((v - mean) ** 2 for v in x.data) / len(x.data)
        assert abs(mean) < 0.2  # rough check
        assert abs(var - 1.0) < 0.3

    def test_custom_mean(self):
        x = tf.random.normal((1000,), mean=5.0, stddev=0.1)
        mean = sum(x.data) / len(x.data)
        assert abs(mean - 5.0) < 0.1

    def test_custom_stddev(self):
        x = tf.random.normal((1000,), mean=0.0, stddev=0.01)
        var = sum(v**2 for v in x.data) / len(x.data)
        assert var < 0.001

    def test_no_grad(self):
        x = tf.random.normal((3,))
        assert x.requires_grad is False
