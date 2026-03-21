"""Tests for tf.keras.models — Sequential and Model."""

import pytest
from ml_framework_core import Tensor
from ml_framework_tf.keras.layers import Dense
from ml_framework_tf.keras.models import Sequential, Model, _slice_batch


class TestSequential:
    def test_forward_pass(self):
        model = Sequential(
            [
                Dense(8, activation="relu", input_dim=4),
                Dense(2),
            ]
        )
        x = Tensor.randn(3, 4)
        y = model(x)
        assert y.shape == (3, 2)

    def test_add_layer(self):
        model = Sequential()
        model.add(Dense(8, input_dim=4))
        model.add(Dense(2))
        x = Tensor.randn(3, 4)
        y = model(x)
        assert y.shape == (3, 2)

    def test_trainable_variables(self):
        model = Sequential(
            [
                Dense(8, input_dim=4),
                Dense(2),
            ]
        )
        # Trigger build
        model(Tensor.randn(1, 4))
        params = model.trainable_variables
        assert len(params) == 4  # 2 layers x (kernel + bias)

    def test_layers_property(self):
        layers = [Dense(8, input_dim=4), Dense(2)]
        model = Sequential(layers)
        assert len(model.layers) == 2


class TestCompile:
    def test_compile_strings(self):
        model = Sequential([Dense(4, input_dim=2)])
        model.compile(optimizer="adam", loss="mse", metrics=["accuracy"])
        assert model._compiled

    def test_compile_objects(self):
        from ml_framework_tf.keras.optimizers import Adam
        from ml_framework_tf.keras.losses import MeanSquaredError

        model = Sequential([Dense(4, input_dim=2)])
        model.compile(optimizer=Adam(), loss=MeanSquaredError())
        assert model._compiled

    def test_unknown_optimizer(self):
        model = Sequential([Dense(4, input_dim=2)])
        with pytest.raises(ValueError, match="Unknown optimizer"):
            model.compile(optimizer="nonexistent")

    def test_not_compiled_fit_error(self):
        model = Sequential([Dense(4, input_dim=2)])
        with pytest.raises(RuntimeError, match="compiled"):
            model.fit(Tensor.randn(2, 2), Tensor.randn(2, 4))


class TestFit:
    def test_basic_fit(self):
        model = Sequential(
            [
                Dense(4, activation="relu", input_dim=2),
                Dense(1),
            ]
        )
        model.compile(optimizer="sgd", loss="mse")

        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.0, 0.0]])
        y = Tensor.from_list([[1.0], [1.0], [0.0], [0.0]])

        history = model.fit(x, y, epochs=2, batch_size=4, verbose=0)
        assert "loss" in history.history
        assert len(history.history["loss"]) == 2

    def test_fit_loss_decreases(self):
        model = Sequential(
            [
                Dense(8, activation="relu", input_dim=2),
                Dense(1),
            ]
        )
        model.compile(optimizer="adam", loss="mse")

        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])

        history = model.fit(x, y, epochs=20, batch_size=2, verbose=0)
        # Loss should generally decrease
        first_loss = history.history["loss"][0]
        last_loss = history.history["loss"][-1]
        assert last_loss < first_loss

    def test_fit_with_metrics(self):
        model = Sequential(
            [
                Dense(4, input_dim=2),
                Dense(1),
            ]
        )
        model.compile(optimizer="adam", loss="mse", metrics=["mse"])
        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])
        history = model.fit(x, y, epochs=3, batch_size=2, verbose=0)
        assert "mean_squared_error" in history.history

    def test_fit_with_validation(self):
        model = Sequential([Dense(4, input_dim=2), Dense(1)])
        model.compile(optimizer="adam", loss="mse")
        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])
        history = model.fit(
            x,
            y,
            epochs=3,
            batch_size=2,
            validation_data=(x, y),
            verbose=0,
        )
        assert "val_loss" in history.history


class TestEvaluate:
    def test_evaluate(self):
        model = Sequential([Dense(4, input_dim=2), Dense(1)])
        model.compile(optimizer="adam", loss="mse", metrics=["mse"])
        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])
        results = model.evaluate(x, y)
        assert len(results) == 2  # loss + mse metric
        assert all(isinstance(r, float) for r in results)

    def test_evaluate_not_compiled(self):
        model = Sequential([Dense(4, input_dim=2)])
        with pytest.raises(RuntimeError, match="compiled"):
            model.evaluate(Tensor.randn(2, 2), Tensor.randn(2, 4))


class TestPredict:
    def test_predict(self):
        model = Sequential([Dense(4, input_dim=2), Dense(1)])
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        pred = model.predict(x)
        assert pred.shape == (2, 1)


class TestSummary:
    def test_summary_runs(self, capsys):
        model = Sequential(
            [
                Dense(8, input_dim=4),
                Dense(2),
            ]
        )
        # Trigger build
        model(Tensor.randn(1, 4))
        model.summary()
        captured = capsys.readouterr()
        assert "Model Summary" in captured.out
        assert "Total trainable" in captured.out


class TestModelClass:
    def test_model_like_sequential(self):
        model = Model(layers=[Dense(4, input_dim=2), Dense(1)])
        x = Tensor.randn(3, 2)
        y = model(x)
        assert y.shape == (3, 1)


class TestEvaluateVerbose:
    def test_evaluate_verbose(self, capsys):
        model = Sequential([Dense(4, input_dim=2), Dense(1)])
        model.compile(optimizer="adam", loss="mse")
        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])
        model.evaluate(x, y, verbose=1)
        captured = capsys.readouterr()
        assert "Evaluate" in captured.out


class TestFitVerbose:
    def test_verbose_prints(self, capsys):
        model = Sequential([Dense(4, input_dim=2), Dense(1)])
        model.compile(optimizer="sgd", loss="mse")
        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])
        model.fit(x, y, epochs=1, batch_size=2, verbose=1)
        captured = capsys.readouterr()
        assert "Epoch 1/1" in captured.out

    def test_fit_with_val_metrics(self):
        model = Sequential([Dense(4, input_dim=2), Dense(1)])
        model.compile(optimizer="adam", loss="mse", metrics=["mse"])
        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])
        history = model.fit(
            x,
            y,
            epochs=2,
            batch_size=2,
            validation_data=(x, y),
            verbose=0,
        )
        assert "val_mean_squared_error" in history.history


class TestSliceBatch:
    def test_slice_2d(self):
        t = Tensor.from_list([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
        batch = _slice_batch(t, 0, 2)
        assert batch.shape == (2, 2)
        assert batch.data == [1.0, 2.0, 3.0, 4.0]

    def test_slice_1d(self):
        t = Tensor.from_list([1.0, 2.0, 3.0, 4.0])
        batch = _slice_batch(t, 1, 3)
        assert batch.shape == (2,)
        assert batch.data == [2.0, 3.0]
