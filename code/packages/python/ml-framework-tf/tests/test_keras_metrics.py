"""Tests for tf.keras.metrics — performance measurement."""

import pytest
from ml_framework_core import Tensor
from ml_framework_tf.keras.metrics import (
    Accuracy,
    BinaryAccuracy,
    CategoricalAccuracy,
    MeanAbsoluteError,
    MeanSquaredError,
    get,
)


class TestAccuracy:
    def test_perfect(self):
        acc = Accuracy()
        y_true = Tensor.from_list([0.0, 1.0, 2.0])
        y_pred = Tensor.from_list([0.0, 1.0, 2.0])
        acc.update_state(y_true, y_pred)
        assert acc.result() == 1.0

    def test_none_correct(self):
        acc = Accuracy()
        y_true = Tensor.from_list([0.0, 1.0, 2.0])
        y_pred = Tensor.from_list([1.0, 2.0, 0.0])
        acc.update_state(y_true, y_pred)
        assert acc.result() == 0.0

    def test_partial(self):
        acc = Accuracy()
        y_true = Tensor.from_list([0.0, 1.0, 2.0])
        y_pred = Tensor.from_list([0.0, 0.0, 2.0])
        acc.update_state(y_true, y_pred)
        assert abs(acc.result() - 2.0 / 3.0) < 1e-6

    def test_2d_argmax(self):
        acc = Accuracy()
        y_true = Tensor.from_list([0.0, 1.0])
        y_pred = Tensor.from_list([[0.9, 0.1], [0.1, 0.9]])
        acc.update_state(y_true, y_pred)
        assert acc.result() == 1.0

    def test_2d_with_2d_true(self):
        acc = Accuracy()
        y_true = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y_pred = Tensor.from_list([[0.9, 0.1], [0.1, 0.9]])
        acc.update_state(y_true, y_pred)
        assert acc.result() == 1.0

    def test_accumulation(self):
        acc = Accuracy()
        acc.update_state(Tensor.from_list([1.0]), Tensor.from_list([1.0]))
        acc.update_state(Tensor.from_list([1.0]), Tensor.from_list([0.0]))
        assert acc.result() == 0.5

    def test_reset(self):
        acc = Accuracy()
        acc.update_state(Tensor.from_list([1.0]), Tensor.from_list([1.0]))
        acc.reset_state()
        assert acc.result() == 0.0

    def test_name(self):
        assert Accuracy().name == "accuracy"


class TestBinaryAccuracy:
    def test_threshold(self):
        acc = BinaryAccuracy(threshold=0.5)
        y_true = Tensor.from_list([1.0, 0.0, 1.0])
        y_pred = Tensor.from_list([0.8, 0.2, 0.6])
        acc.update_state(y_true, y_pred)
        assert acc.result() == 1.0

    def test_wrong_predictions(self):
        acc = BinaryAccuracy(threshold=0.5)
        y_true = Tensor.from_list([1.0, 0.0])
        y_pred = Tensor.from_list([0.3, 0.7])
        acc.update_state(y_true, y_pred)
        assert acc.result() == 0.0

    def test_reset(self):
        acc = BinaryAccuracy()
        acc.update_state(Tensor.from_list([1.0]), Tensor.from_list([0.9]))
        acc.reset_state()
        assert acc.result() == 0.0


class TestCategoricalAccuracy:
    def test_perfect(self):
        acc = CategoricalAccuracy()
        y_true = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y_pred = Tensor.from_list([[0.9, 0.1], [0.1, 0.9]])
        acc.update_state(y_true, y_pred)
        assert acc.result() == 1.0

    def test_wrong(self):
        acc = CategoricalAccuracy()
        y_true = Tensor.from_list([[1.0, 0.0]])
        y_pred = Tensor.from_list([[0.1, 0.9]])
        acc.update_state(y_true, y_pred)
        assert acc.result() == 0.0

    def test_invalid_dims(self):
        acc = CategoricalAccuracy()
        with pytest.raises(ValueError, match="2-D"):
            acc.update_state(
                Tensor.from_list([1.0]),
                Tensor.from_list([0.5]),
            )


class TestMeanSquaredError:
    def test_zero(self):
        m = MeanSquaredError()
        y_true = Tensor.from_list([1.0, 2.0])
        y_pred = Tensor.from_list([1.0, 2.0])
        m.update_state(y_true, y_pred)
        assert m.result() == 0.0

    def test_nonzero(self):
        m = MeanSquaredError()
        y_true = Tensor.from_list([0.0])
        y_pred = Tensor.from_list([3.0])
        m.update_state(y_true, y_pred)
        assert abs(m.result() - 9.0) < 1e-6

    def test_accumulation(self):
        m = MeanSquaredError()
        m.update_state(Tensor.from_list([0.0]), Tensor.from_list([1.0]))
        m.update_state(Tensor.from_list([0.0]), Tensor.from_list([3.0]))
        # (1 + 9) / 2 = 5
        assert abs(m.result() - 5.0) < 1e-6

    def test_reset(self):
        m = MeanSquaredError()
        m.update_state(Tensor.from_list([0.0]), Tensor.from_list([1.0]))
        m.reset_state()
        assert m.result() == 0.0


class TestMeanAbsoluteError:
    def test_zero(self):
        m = MeanAbsoluteError()
        y_true = Tensor.from_list([1.0])
        y_pred = Tensor.from_list([1.0])
        m.update_state(y_true, y_pred)
        assert m.result() == 0.0

    def test_nonzero(self):
        m = MeanAbsoluteError()
        y_true = Tensor.from_list([0.0, 0.0])
        y_pred = Tensor.from_list([1.0, 3.0])
        m.update_state(y_true, y_pred)
        assert abs(m.result() - 2.0) < 1e-6


class TestMetricBase:
    def test_abstract_update_state(self):
        from ml_framework_tf.keras.metrics import Metric

        m = Metric()
        with pytest.raises(NotImplementedError):
            m.update_state(Tensor.from_list([1.0]), Tensor.from_list([1.0]))

    def test_abstract_result(self):
        from ml_framework_tf.keras.metrics import Metric

        m = Metric()
        with pytest.raises(NotImplementedError):
            m.result()

    def test_abstract_reset_state(self):
        from ml_framework_tf.keras.metrics import Metric

        m = Metric()
        with pytest.raises(NotImplementedError):
            m.reset_state()


class TestCategoricalAccuracyReset:
    def test_reset(self):
        acc = CategoricalAccuracy()
        y_true = Tensor.from_list([[1.0, 0.0]])
        y_pred = Tensor.from_list([[0.9, 0.1]])
        acc.update_state(y_true, y_pred)
        acc.reset_state()
        assert acc.result() == 0.0


class TestMAEEdgeCases:
    def test_empty_result(self):
        m = MeanAbsoluteError()
        assert m.result() == 0.0

    def test_reset(self):
        m = MeanAbsoluteError()
        m.update_state(Tensor.from_list([0.0]), Tensor.from_list([5.0]))
        m.reset_state()
        assert m.result() == 0.0


class TestMetricGet:
    def test_get_by_string(self):
        m = get("accuracy")
        assert isinstance(m, Accuracy)

    def test_get_passthrough(self):
        m = Accuracy()
        assert get(m) is m

    def test_unknown(self):
        with pytest.raises(ValueError, match="Unknown metric"):
            get("nonexistent")

    def test_all_keys(self):
        for key in [
            "accuracy",
            "binary_accuracy",
            "categorical_accuracy",
            "mse",
            "mae",
        ]:
            m = get(key)
            assert m is not None
