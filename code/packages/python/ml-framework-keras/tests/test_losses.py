"""Tests for the losses module."""

import pytest
from ml_framework_core import Tensor

from ml_framework_keras.losses import (
    BinaryCrossentropy,
    CategoricalCrossentropy,
    Loss,
    MeanAbsoluteError,
    MeanSquaredError,
    SparseCategoricalCrossentropy,
    get_loss,
)


class TestMeanSquaredError:
    def test_perfect_prediction(self):
        y_true = Tensor.from_list([1.0, 2.0, 3.0])
        y_pred = Tensor.from_list([1.0, 2.0, 3.0])
        loss = MeanSquaredError()(y_true, y_pred)
        assert abs(loss.data[0]) < 1e-6

    def test_known_value(self):
        y_true = Tensor.from_list([0.0, 0.0])
        y_pred = Tensor.from_list([1.0, 1.0])
        loss = MeanSquaredError()(y_true, y_pred)
        # MSE = mean([1, 1]) = 1.0
        assert abs(loss.data[0] - 1.0) < 1e-6

    def test_2d_input(self):
        y_true = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        y_pred = Tensor.from_list([[0.5, 0.5]], shape=(1, 2))
        loss = MeanSquaredError()(y_true, y_pred)
        # MSE = mean([0.25, 0.25]) = 0.25
        assert abs(loss.data[0] - 0.25) < 1e-6


class TestMeanAbsoluteError:
    def test_perfect_prediction(self):
        y_true = Tensor.from_list([1.0, 2.0])
        y_pred = Tensor.from_list([1.0, 2.0])
        loss = MeanAbsoluteError()(y_true, y_pred)
        assert abs(loss.data[0]) < 1e-6

    def test_known_value(self):
        y_true = Tensor.from_list([0.0, 0.0])
        y_pred = Tensor.from_list([1.0, 3.0])
        loss = MeanAbsoluteError()(y_true, y_pred)
        # MAE = mean([1, 3]) = 2.0
        assert abs(loss.data[0] - 2.0) < 1e-6


class TestBinaryCrossentropy:
    def test_perfect_prediction(self):
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([0.999, 0.001])
        loss = BinaryCrossentropy()(y_true, y_pred)
        assert loss.data[0] < 0.01  # very small loss

    def test_worst_prediction(self):
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([0.01, 0.99])
        loss = BinaryCrossentropy()(y_true, y_pred)
        assert loss.data[0] > 2.0  # large loss

    def test_from_logits(self):
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([5.0, -5.0])  # raw logits
        loss = BinaryCrossentropy(from_logits=True)(y_true, y_pred)
        assert loss.data[0] < 0.1

    def test_get_config(self):
        loss = BinaryCrossentropy(from_logits=True)
        config = loss.get_config()
        assert config["from_logits"] is True


class TestCategoricalCrossentropy:
    def test_perfect_prediction(self):
        y_true = Tensor.from_list([[0.0, 1.0, 0.0]], shape=(1, 3))
        y_pred = Tensor.from_list([[0.01, 0.98, 0.01]], shape=(1, 3))
        loss = CategoricalCrossentropy()(y_true, y_pred)
        assert loss.data[0] < 0.05

    def test_wrong_prediction(self):
        y_true = Tensor.from_list([[0.0, 1.0, 0.0]], shape=(1, 3))
        y_pred = Tensor.from_list([[0.9, 0.05, 0.05]], shape=(1, 3))
        loss = CategoricalCrossentropy()(y_true, y_pred)
        assert loss.data[0] > 2.0

    def test_from_logits(self):
        y_true = Tensor.from_list([[0.0, 1.0, 0.0]], shape=(1, 3))
        y_pred = Tensor.from_list([[-5.0, 5.0, -5.0]], shape=(1, 3))
        loss = CategoricalCrossentropy(from_logits=True)(y_true, y_pred)
        assert loss.data[0] < 0.1

    def test_batch(self):
        y_true = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        y_pred = Tensor.from_list([[0.9, 0.1], [0.1, 0.9]], shape=(2, 2))
        loss = CategoricalCrossentropy()(y_true, y_pred)
        assert loss.data[0] > 0

    def test_get_config(self):
        loss = CategoricalCrossentropy(from_logits=True)
        config = loss.get_config()
        assert config["from_logits"] is True


class TestSparseCategoricalCrossentropy:
    def test_perfect_prediction(self):
        y_true = Tensor.from_list([1.0], shape=(1,))
        y_pred = Tensor.from_list([[0.01, 0.98, 0.01]], shape=(1, 3))
        loss = SparseCategoricalCrossentropy()(y_true, y_pred)
        assert loss.data[0] < 0.05

    def test_wrong_prediction(self):
        y_true = Tensor.from_list([1.0], shape=(1,))
        y_pred = Tensor.from_list([[0.9, 0.05, 0.05]], shape=(1, 3))
        loss = SparseCategoricalCrossentropy()(y_true, y_pred)
        assert loss.data[0] > 2.0

    def test_batch(self):
        y_true = Tensor.from_list([0.0, 1.0], shape=(2,))
        y_pred = Tensor.from_list([[0.9, 0.1], [0.2, 0.8]], shape=(2, 2))
        loss = SparseCategoricalCrossentropy()(y_true, y_pred)
        assert loss.data[0] > 0

    def test_from_logits(self):
        y_true = Tensor.from_list([2.0], shape=(1,))
        y_pred = Tensor.from_list([[-5.0, -5.0, 5.0]], shape=(1, 3))
        loss = SparseCategoricalCrossentropy(from_logits=True)(y_true, y_pred)
        assert loss.data[0] < 0.1

    def test_get_config(self):
        loss = SparseCategoricalCrossentropy(from_logits=True)
        config = loss.get_config()
        assert config["from_logits"] is True


class TestGetLoss:
    def test_string_mse(self):
        loss = get_loss("mse")
        assert isinstance(loss, MeanSquaredError)

    def test_string_mae(self):
        loss = get_loss("mae")
        assert isinstance(loss, MeanAbsoluteError)

    def test_string_mean_squared_error(self):
        loss = get_loss("mean_squared_error")
        assert isinstance(loss, MeanSquaredError)

    def test_string_binary_crossentropy(self):
        loss = get_loss("binary_crossentropy")
        assert isinstance(loss, BinaryCrossentropy)

    def test_string_categorical_crossentropy(self):
        loss = get_loss("categorical_crossentropy")
        assert isinstance(loss, CategoricalCrossentropy)

    def test_string_sparse_categorical_crossentropy(self):
        loss = get_loss("sparse_categorical_crossentropy")
        assert isinstance(loss, SparseCategoricalCrossentropy)

    def test_instance_passthrough(self):
        loss = MeanSquaredError()
        result = get_loss(loss)
        assert result is loss

    def test_unknown_string_raises(self):
        with pytest.raises(ValueError, match="Unknown loss"):
            get_loss("nonexistent")

    def test_invalid_type_raises(self):
        with pytest.raises(TypeError):
            get_loss(42)


class TestLossBase:
    def test_call_not_implemented(self):
        loss = Loss()
        with pytest.raises(NotImplementedError):
            loss(Tensor.from_list([1.0]), Tensor.from_list([1.0]))

    def test_get_config(self):
        loss = Loss()
        config = loss.get_config()
        assert config["class_name"] == "Loss"
