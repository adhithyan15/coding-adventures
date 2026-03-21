"""Tests for the metrics module."""

import pytest
from ml_framework_core import Tensor

from ml_framework_keras.metrics import (
    Accuracy,
    BinaryAccuracy,
    CategoricalAccuracy,
    MeanAbsoluteError,
    MeanSquaredError,
    Metric,
    get_metric,
)


class TestAccuracy:
    def test_perfect_binary(self):
        metric = Accuracy()
        y_true = Tensor.from_list([1.0, 0.0, 1.0])
        y_pred = Tensor.from_list([0.9, 0.1, 0.8])
        metric.update_state(y_true, y_pred)
        assert metric.result() == 1.0

    def test_all_wrong_binary(self):
        metric = Accuracy()
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([0.1, 0.9])
        metric.update_state(y_true, y_pred)
        assert metric.result() == 0.0

    def test_multiclass_with_argmax(self):
        metric = Accuracy()
        y_true = Tensor.from_list([[1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], shape=(2, 3))
        y_pred = Tensor.from_list([[0.7, 0.2, 0.1], [0.1, 0.1, 0.8]], shape=(2, 3))
        metric.update_state(y_true, y_pred)
        assert metric.result() == 1.0

    def test_multiclass_integer_labels(self):
        metric = Accuracy()
        y_true = Tensor.from_list([0.0, 2.0], shape=(2,))
        y_pred = Tensor.from_list([[0.7, 0.2, 0.1], [0.1, 0.1, 0.8]], shape=(2, 3))
        metric.update_state(y_true, y_pred)
        assert metric.result() == 1.0

    def test_reset_state(self):
        metric = Accuracy()
        y_true = Tensor.from_list([1.0])
        y_pred = Tensor.from_list([0.9])
        metric.update_state(y_true, y_pred)
        assert metric.result() == 1.0
        metric.reset_state()
        assert metric.result() == 0.0

    def test_accumulates_across_batches(self):
        metric = Accuracy()
        # Batch 1: 1/1 correct
        metric.update_state(Tensor.from_list([1.0]), Tensor.from_list([0.9]))
        # Batch 2: 0/1 correct
        metric.update_state(Tensor.from_list([0.0]), Tensor.from_list([0.9]))
        assert metric.result() == 0.5

    def test_name(self):
        metric = Accuracy()
        assert metric.name == "accuracy"

    def test_custom_name(self):
        metric = Accuracy(name="my_acc")
        assert metric.name == "my_acc"


class TestBinaryAccuracy:
    def test_perfect(self):
        metric = BinaryAccuracy()
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([0.9, 0.1])
        metric.update_state(y_true, y_pred)
        assert metric.result() == 1.0

    def test_custom_threshold(self):
        metric = BinaryAccuracy(threshold=0.7)
        y_true = Tensor.from_list([1.0])
        y_pred = Tensor.from_list([0.6])  # below 0.7 threshold
        metric.update_state(y_true, y_pred)
        assert metric.result() == 0.0

    def test_reset_state(self):
        metric = BinaryAccuracy()
        metric.update_state(Tensor.from_list([1.0]), Tensor.from_list([0.9]))
        metric.reset_state()
        assert metric.result() == 0.0

    def test_name(self):
        assert BinaryAccuracy().name == "binary_accuracy"


class TestCategoricalAccuracy:
    def test_perfect(self):
        metric = CategoricalAccuracy()
        y_true = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        y_pred = Tensor.from_list([[0.9, 0.1], [0.1, 0.9]], shape=(2, 2))
        metric.update_state(y_true, y_pred)
        assert metric.result() == 1.0

    def test_all_wrong(self):
        metric = CategoricalAccuracy()
        y_true = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        y_pred = Tensor.from_list([[0.1, 0.9], [0.9, 0.1]], shape=(2, 2))
        metric.update_state(y_true, y_pred)
        assert metric.result() == 0.0

    def test_reset_state(self):
        metric = CategoricalAccuracy()
        metric.update_state(
            Tensor.from_list([[1.0, 0.0]], shape=(1, 2)),
            Tensor.from_list([[0.9, 0.1]], shape=(1, 2)),
        )
        metric.reset_state()
        assert metric.result() == 0.0

    def test_name(self):
        assert CategoricalAccuracy().name == "categorical_accuracy"


class TestMeanSquaredErrorMetric:
    def test_perfect(self):
        metric = MeanSquaredError()
        y_true = Tensor.from_list([1.0, 2.0])
        y_pred = Tensor.from_list([1.0, 2.0])
        metric.update_state(y_true, y_pred)
        assert metric.result() == 0.0

    def test_known_value(self):
        metric = MeanSquaredError()
        y_true = Tensor.from_list([0.0])
        y_pred = Tensor.from_list([2.0])
        metric.update_state(y_true, y_pred)
        assert metric.result() == 4.0

    def test_reset_state(self):
        metric = MeanSquaredError()
        metric.update_state(Tensor.from_list([0.0]), Tensor.from_list([1.0]))
        metric.reset_state()
        assert metric.result() == 0.0

    def test_name(self):
        assert MeanSquaredError().name == "mean_squared_error"


class TestMeanAbsoluteErrorMetric:
    def test_perfect(self):
        metric = MeanAbsoluteError()
        y_true = Tensor.from_list([1.0, 2.0])
        y_pred = Tensor.from_list([1.0, 2.0])
        metric.update_state(y_true, y_pred)
        assert metric.result() == 0.0

    def test_known_value(self):
        metric = MeanAbsoluteError()
        y_true = Tensor.from_list([0.0])
        y_pred = Tensor.from_list([3.0])
        metric.update_state(y_true, y_pred)
        assert metric.result() == 3.0

    def test_reset_state(self):
        metric = MeanAbsoluteError()
        metric.update_state(Tensor.from_list([0.0]), Tensor.from_list([1.0]))
        metric.reset_state()
        assert metric.result() == 0.0

    def test_name(self):
        assert MeanAbsoluteError().name == "mean_absolute_error"


class TestGetMetric:
    def test_string_accuracy(self):
        m = get_metric("accuracy")
        assert isinstance(m, Accuracy)

    def test_string_binary_accuracy(self):
        m = get_metric("binary_accuracy")
        assert isinstance(m, BinaryAccuracy)

    def test_string_categorical_accuracy(self):
        m = get_metric("categorical_accuracy")
        assert isinstance(m, CategoricalAccuracy)

    def test_string_mse(self):
        m = get_metric("mse")
        assert isinstance(m, MeanSquaredError)

    def test_string_mae(self):
        m = get_metric("mae")
        assert isinstance(m, MeanAbsoluteError)

    def test_instance_passthrough(self):
        m = Accuracy()
        assert get_metric(m) is m

    def test_unknown_raises(self):
        with pytest.raises(ValueError, match="Unknown metric"):
            get_metric("nonexistent")

    def test_invalid_type_raises(self):
        with pytest.raises(TypeError):
            get_metric(42)


class TestMetricBase:
    def test_update_state_not_implemented(self):
        m = Metric()
        with pytest.raises(NotImplementedError):
            m.update_state(Tensor.from_list([1.0]), Tensor.from_list([1.0]))

    def test_result_not_implemented(self):
        m = Metric()
        with pytest.raises(NotImplementedError):
            m.result()

    def test_reset_not_implemented(self):
        m = Metric()
        with pytest.raises(NotImplementedError):
            m.reset_state()

    def test_get_config(self):
        m = Metric(name="test")
        config = m.get_config()
        assert config["name"] == "test"
        assert config["class_name"] == "Metric"

    def test_default_name(self):
        m = Metric()
        assert m.name == "metric"
