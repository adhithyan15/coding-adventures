"""Integration tests: end-to-end model.compile() + model.fit() scenarios."""

from ml_framework_core import Tensor

import ml_framework_keras as keras
from ml_framework_keras.callbacks import EarlyStopping, LearningRateScheduler
from ml_framework_keras.layers import Dense, Dropout
from ml_framework_keras.models import Sequential


class TestEndToEndRegression:
    """Test: train a model to learn y = 2x."""

    def test_linear_regression(self):
        model = Sequential([Dense(1)])
        model.compile(optimizer="sgd", loss="mse")

        # Training data: y = 2x (approximately)
        x = Tensor.from_list([[1.0], [2.0], [3.0], [4.0]], shape=(4, 1))
        y = Tensor.from_list([[2.0], [4.0], [6.0], [8.0]], shape=(4, 1))

        history = model.fit(x, y, epochs=50, batch_size=4, verbose=0)

        # Loss should decrease
        assert history.history["loss"][-1] < history.history["loss"][0]

        # Prediction should be close to 2*5=10
        pred = model.predict(Tensor.from_list([[5.0]], shape=(1, 1)))
        assert pred.shape == (1, 1)


class TestEndToEndClassification:
    """Test: train a simple classifier with softmax output."""

    def test_binary_classification(self):
        model = Sequential(
            [
                Dense(4, activation="relu"),
                Dense(2, activation="softmax"),
            ]
        )
        model.compile(
            optimizer="adam",
            loss="categorical_crossentropy",
            metrics=["accuracy"],
        )

        # Simple 2-class problem
        x = Tensor.from_list(
            [[1.0, 0.0], [0.0, 1.0], [1.0, 0.0], [0.0, 1.0]],
            shape=(4, 2),
        )
        y = Tensor.from_list(
            [[1.0, 0.0], [0.0, 1.0], [1.0, 0.0], [0.0, 1.0]],
            shape=(4, 2),
        )

        history = model.fit(x, y, epochs=30, batch_size=4, verbose=0)
        assert history.history["loss"][-1] < history.history["loss"][0]


class TestEndToEndWithCallbacks:
    """Test: training with callbacks."""

    def test_early_stopping(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse")

        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        y = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))

        es = EarlyStopping(monitor="loss", patience=2)
        history = model.fit(
            x,
            y,
            epochs=100,
            batch_size=2,
            callbacks=[es],
            verbose=0,
        )

        # Should stop before 100 epochs (either by converging or patience)
        assert len(history.history["loss"]) <= 100

    def test_lr_scheduler(self):
        model = Sequential([Dense(2)])
        model.compile(optimizer="sgd", loss="mse")

        x = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))
        y = Tensor.from_list([[1.0, 0.0]], shape=(1, 2))

        def schedule(epoch, lr):
            return lr * 0.9  # decay by 10% each epoch

        lrs = LearningRateScheduler(schedule)
        model.fit(
            x,
            y,
            epochs=5,
            batch_size=1,
            callbacks=[lrs],
            verbose=0,
        )

        # LR should have decayed
        assert model._optimizer.learning_rate < 0.01


class TestEndToEndWithDropout:
    """Test: model with dropout trains without error."""

    def test_dropout_in_model(self):
        model = Sequential(
            [
                Dense(8, activation="relu"),
                Dropout(0.3),
                Dense(2),
            ]
        )
        model.compile(optimizer="adam", loss="mse")

        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))
        y = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]], shape=(2, 2))

        history = model.fit(x, y, epochs=5, batch_size=2, verbose=0)
        assert len(history.history["loss"]) == 5


class TestEndToEndWithValidation:
    """Test: training with validation data and metrics."""

    def test_validation_metrics(self):
        model = Sequential([Dense(4, activation="relu"), Dense(2)])
        model.compile(optimizer="adam", loss="mse", metrics=["accuracy"])

        x_train = Tensor.from_list([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]], shape=(3, 2))
        y_train = Tensor.from_list([[1.0, 0.0], [0.0, 1.0], [0.5, 0.5]], shape=(3, 2))
        x_val = Tensor.from_list([[0.5, 0.5]], shape=(1, 2))
        y_val = Tensor.from_list([[0.5, 0.5]], shape=(1, 2))

        history = model.fit(
            x_train,
            y_train,
            epochs=3,
            batch_size=3,
            validation_data=(x_val, y_val),
            verbose=0,
        )

        assert "val_loss" in history.history
        assert "val_accuracy" in history.history


class TestKerasImportStyle:
    """Test that the keras import style works like real Keras."""

    def test_import_style(self):
        # This should work like: import keras
        assert hasattr(keras, "Sequential")
        assert hasattr(keras, "Model")
        assert hasattr(keras, "layers")
        assert hasattr(keras, "optimizers")
        assert hasattr(keras, "losses")
        assert hasattr(keras, "metrics")
        assert hasattr(keras, "callbacks")
        assert hasattr(keras, "activations")
        assert hasattr(keras, "backend")

    def test_layers_submodule(self):
        assert hasattr(keras.layers, "Dense")
        assert hasattr(keras.layers, "Dropout")
        assert hasattr(keras.layers, "Input")
        assert hasattr(keras.layers, "Flatten")

    def test_optimizers_submodule(self):
        assert hasattr(keras.optimizers, "Adam")
        assert hasattr(keras.optimizers, "SGD")

    def test_losses_submodule(self):
        assert hasattr(keras.losses, "MeanSquaredError")
        assert hasattr(keras.losses, "CategoricalCrossentropy")

    def test_backend_submodule(self):
        assert keras.backend.get_backend() == "ml_framework_core"
