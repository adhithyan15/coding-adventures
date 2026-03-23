"""Tests for tf.keras.losses — loss functions."""

import pytest
from ml_framework_core import Tensor
from ml_framework_tf.keras.losses import (
    BinaryCrossentropy,
    CategoricalCrossentropy,
    MeanAbsoluteError,
    MeanSquaredError,
    SparseCategoricalCrossentropy,
    get,
)


class TestMeanSquaredError:
    def test_zero_loss(self):
        loss_fn = MeanSquaredError()
        y_true = Tensor.from_list([1.0, 2.0, 3.0])
        y_pred = Tensor.from_list([1.0, 2.0, 3.0])
        loss = loss_fn(y_true, y_pred)
        assert abs(loss.data[0]) < 1e-6

    def test_nonzero_loss(self):
        loss_fn = MeanSquaredError()
        y_true = Tensor.from_list([0.0, 0.0, 0.0])
        y_pred = Tensor.from_list([1.0, 1.0, 1.0])
        loss = loss_fn(y_true, y_pred)
        assert abs(loss.data[0] - 1.0) < 1e-5

    def test_asymmetric(self):
        loss_fn = MeanSquaredError()
        y_true = Tensor.from_list([0.0])
        y_pred = Tensor.from_list([3.0])
        loss = loss_fn(y_true, y_pred)
        assert abs(loss.data[0] - 9.0) < 1e-5


class TestMeanAbsoluteError:
    def test_zero_loss(self):
        loss_fn = MeanAbsoluteError()
        y_true = Tensor.from_list([1.0, 2.0])
        y_pred = Tensor.from_list([1.0, 2.0])
        loss = loss_fn(y_true, y_pred)
        assert abs(loss.data[0]) < 1e-6

    def test_nonzero_loss(self):
        loss_fn = MeanAbsoluteError()
        y_true = Tensor.from_list([0.0, 0.0])
        y_pred = Tensor.from_list([1.0, 3.0])
        loss = loss_fn(y_true, y_pred)
        assert abs(loss.data[0] - 2.0) < 1e-5


class TestBinaryCrossentropy:
    def test_perfect_prediction(self):
        loss_fn = BinaryCrossentropy()
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([0.99, 0.01])
        loss = loss_fn(y_true, y_pred)
        assert loss.data[0] < 0.1  # low loss

    def test_worst_prediction(self):
        loss_fn = BinaryCrossentropy()
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([0.01, 0.99])
        loss = loss_fn(y_true, y_pred)
        assert loss.data[0] > 2.0  # high loss

    def test_from_logits(self):
        loss_fn = BinaryCrossentropy(from_logits=True)
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([5.0, -5.0])  # logits
        loss = loss_fn(y_true, y_pred)
        assert loss.data[0] < 0.1


class TestCategoricalCrossentropy:
    def test_perfect_prediction(self):
        loss_fn = CategoricalCrossentropy()
        y_true = Tensor.from_list([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]])
        y_pred = Tensor.from_list([[0.9, 0.05, 0.05], [0.05, 0.9, 0.05]])
        loss = loss_fn(y_true, y_pred)
        assert loss.data[0] < 0.2

    def test_from_logits(self):
        loss_fn = CategoricalCrossentropy(from_logits=True)
        y_true = Tensor.from_list([[1.0, 0.0, 0.0]])
        y_pred = Tensor.from_list([[5.0, 0.0, 0.0]])  # logits
        loss = loss_fn(y_true, y_pred)
        assert loss.data[0] < 0.1

    def test_repr(self):
        assert "CategoricalCrossentropy" in repr(CategoricalCrossentropy())


class TestSparseCategoricalCrossentropy:
    def test_correct_prediction(self):
        loss_fn = SparseCategoricalCrossentropy()
        y_true = Tensor.from_list([0.0, 1.0])  # integer class labels
        y_pred = Tensor.from_list([[0.9, 0.05, 0.05], [0.05, 0.9, 0.05]])
        loss = loss_fn(y_true, y_pred)
        assert loss.data[0] < 0.2

    def test_from_logits(self):
        loss_fn = SparseCategoricalCrossentropy(from_logits=True)
        y_true = Tensor.from_list([0.0])
        y_pred = Tensor.from_list([[5.0, 0.0, 0.0]])
        loss = loss_fn(y_true, y_pred)
        assert loss.data[0] < 0.1

    def test_wrong_dims(self):
        loss_fn = SparseCategoricalCrossentropy()
        y_true = Tensor.from_list([0.0])
        y_pred = Tensor.from_list([0.5, 0.5])  # 1-D, should be 2-D
        with pytest.raises(ValueError, match="2-D"):
            loss_fn(y_true, y_pred)


class TestReprs:
    def test_mse_repr(self):
        assert "MeanSquaredError" in repr(MeanSquaredError())

    def test_mae_repr(self):
        assert "MeanAbsoluteError" in repr(MeanAbsoluteError())

    def test_bce_repr(self):
        assert "BinaryCrossentropy" in repr(BinaryCrossentropy())

    def test_scce_repr(self):
        assert "SparseCategoricalCrossentropy" in repr(SparseCategoricalCrossentropy())


class TestLossGet:
    def test_get_by_string(self):
        loss = get("mse")
        assert isinstance(loss, MeanSquaredError)

    def test_get_by_alias(self):
        loss = get("mean_squared_error")
        assert isinstance(loss, MeanSquaredError)

    def test_get_object_passthrough(self):
        original = MeanSquaredError()
        assert get(original) is original

    def test_unknown_string(self):
        with pytest.raises(ValueError, match="Unknown loss"):
            get("nonexistent")

    def test_all_string_keys(self):
        for key in [
            "mse",
            "mae",
            "binary_crossentropy",
            "categorical_crossentropy",
            "sparse_categorical_crossentropy",
        ]:
            loss = get(key)
            assert loss is not None
