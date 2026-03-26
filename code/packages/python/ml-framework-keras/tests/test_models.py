"""Tests for the models module (Sequential and Functional API)."""

import pytest
from ml_framework_core import Tensor

from ml_framework_keras.layers import Dense, Input
from ml_framework_keras.models import Sequential, Model


class TestSequential:
    def test_creation_with_layers(self):
        model = Sequential([Dense(4), Dense(2)])
        assert len(model.layers) == 2

    def test_creation_empty(self):
        model = Sequential()
        assert len(model.layers) == 0

    def test_add(self):
        model = Sequential()
        model.add(Dense(4))
        model.add(Dense(2))
        assert len(model.layers) == 2

    def test_forward_pass(self):
        model = Sequential([Dense(4), Dense(2)])
        x = Tensor.from_list([[1.0, 2.0, 3.0]], shape=(1, 3))
        y = model(x)
        assert y.shape == (1, 2)

    def test_trainable_weights(self):
        model = Sequential([Dense(4, use_bias=True)])
        x = Tensor.from_list([[1.0, 2.0]], shape=(1, 2))
        model(x)  # trigger build
        weights = model.trainable_weights
        assert len(weights) == 2  # kernel + bias

    def test_count_params(self):
        model = Sequential([Dense(4, use_bias=True)])
        x = Tensor.from_list([[1.0, 2.0, 3.0]], shape=(1, 3))
        model(x)
        # kernel: 3*4=12, bias: 4 → total 16
        assert model.count_params() == 16

    def test_compile(self):
        model = Sequential([Dense(4)])
        model.compile(optimizer="sgd", loss="mse")
        assert model._compiled

    def test_compile_with_metrics(self):
        model = Sequential([Dense(4)])
        model.compile(optimizer="adam", loss="mse", metrics=["accuracy"])
        assert len(model._metrics) == 1

    def test_predict(self):
        model = Sequential([Dense(2)])
        x = Tensor.from_list([[1.0, 2.0]], shape=(1, 2))
        pred = model.predict(x)
        assert pred.shape == (1, 2)

    def test_summary(self):
        model = Sequential([Dense(4), Dense(2)])
        x = Tensor.from_list([[1.0, 2.0, 3.0]], shape=(1, 3))
        model(x)  # build layers
        summary = model.summary()
        assert "Dense" in summary
        assert "Total params" in summary

    def test_fit_requires_compile(self):
        model = Sequential([Dense(2)])
        x = Tensor.from_list([[1.0]], shape=(1, 1))
        y = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        with pytest.raises(RuntimeError, match="compiled"):
            model.fit(x, y, epochs=1)

    def test_evaluate_requires_compile(self):
        model = Sequential([Dense(2)])
        x = Tensor.from_list([[1.0]], shape=(1, 1))
        y = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        with pytest.raises(RuntimeError):
            model.evaluate(x, y)


class TestModel:
    def test_functional_api(self):
        inputs = Input(shape=(3,))
        Dense(4, activation="relu")(inputs)
        Dense(2)(inputs)
        model = Model(inputs=inputs, outputs=inputs)
        # The layers should be recorded
        assert len(model.layers) >= 1

    def test_functional_forward_pass(self):
        inputs = Input(shape=(3,))
        Dense(4)(inputs)
        Dense(2)(inputs)
        model = Model(inputs=inputs, outputs=inputs)
        x = Tensor.from_list([[1.0, 2.0, 3.0]], shape=(1, 3))
        y = model(x)
        assert y.shape == (1, 2)

    def test_empty_model(self):
        model = Model()
        assert len(model.layers) == 0


class TestModelFit:
    def test_basic_fit(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse")

        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        y = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        history = model.fit(x, y, epochs=3, batch_size=2, verbose=0)
        assert "loss" in history.history
        assert len(history.history["loss"]) == 3

    def test_fit_loss_decreases(self):
        model = Sequential([Dense(4, activation="relu"), Dense(2)])
        model.compile(optimizer="adam", loss="mse")

        x = Tensor.from_list(
            [[1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.5]],
            shape=(4, 2),
        )
        y = Tensor.from_list(
            [[1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.5, 0.5]],
            shape=(4, 2),
        )
        history = model.fit(x, y, epochs=20, batch_size=4, verbose=0)
        # Loss should generally decrease
        assert history.history["loss"][-1] <= history.history["loss"][0]

    def test_fit_with_validation_data(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse")

        x_train = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        y_train = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        x_val = Tensor.from_list([[0.5, 0.5]], shape=(1, 2))
        y_val = Tensor.from_list([[0.5, 0.5]], shape=(1, 2))

        history = model.fit(
            x_train,
            y_train,
            epochs=2,
            batch_size=2,
            validation_data=(x_val, y_val),
            verbose=0,
        )
        assert "val_loss" in history.history

    def test_fit_with_validation_split(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse")

        x = Tensor.from_list(
            [[1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.0, 0.0]],
            shape=(4, 2),
        )
        y = Tensor.from_list(
            [[1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [0.0, 0.0]],
            shape=(4, 2),
        )
        history = model.fit(
            x,
            y,
            epochs=2,
            batch_size=4,
            validation_split=0.25,
            verbose=0,
        )
        assert "val_loss" in history.history

    def test_fit_with_metrics(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse", metrics=["mse"])

        x = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        y = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        history = model.fit(x, y, epochs=1, batch_size=1, verbose=0)
        assert "mean_squared_error" in history.history

    def test_fit_mini_batches(self):
        model = Sequential([Dense(1)])
        model.compile(optimizer="sgd", loss="mse")

        x = Tensor.from_list([[1.0], [2.0], [3.0], [4.0]], shape=(4, 1))
        y = Tensor.from_list([[2.0], [4.0], [6.0], [8.0]], shape=(4, 1))
        history = model.fit(x, y, epochs=2, batch_size=2, verbose=0)
        assert len(history.history["loss"]) == 2

    def test_evaluate(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse", metrics=["accuracy"])

        x = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        y = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        model(x)  # build

        results = model.evaluate(x, y, verbose=0)
        assert len(results) == 2  # loss + accuracy

    def test_evaluate_verbose(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse")

        x = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        y = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))

        results = model.evaluate(x, y, verbose=1)
        assert len(results) == 1  # just loss

    def test_fit_verbose(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse")

        x = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        y = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        # verbose=1 should print but not crash
        model.fit(x, y, epochs=1, batch_size=1, verbose=1)
