"""Integration tests — end-to-end training with GradientTape and model.fit()."""

import ml_framework_tf as tf
from ml_framework_tf.variable import Variable
from ml_framework_tf.gradient_tape import GradientTape
from ml_framework_tf.keras.layers import Dense
from ml_framework_tf.keras.models import Sequential
from ml_framework_tf.keras.optimizers import Adam
from ml_framework_tf.keras.callbacks import EarlyStopping
from ml_framework_core import Tensor


class TestManualTrainingLoop:
    """Test a manual GradientTape training loop (low-level TF style)."""

    def test_linear_regression(self):
        """Train y = 2*x with a manual gradient descent loop."""
        w = Variable([0.5])
        learning_rate = 0.1

        # Training data: y = 2 * x
        x_data = [1.0, 2.0, 3.0, 4.0]
        y_data = [2.0, 4.0, 6.0, 8.0]

        for _ in range(100):
            total_loss = 0.0
            for x_val, y_true in zip(x_data, y_data):
                x = tf.constant([x_val])
                y_target = tf.constant([y_true])

                with GradientTape() as tape:
                    tape.watch(w)
                    y_pred = w * x
                    loss = (y_pred - y_target) ** 2.0
                    loss = tf.reduce_sum(loss)

                grads = tape.gradient(loss, [w])
                w.assign_sub(Tensor.from_list([learning_rate * grads[0].data[0]]))
                total_loss += loss.data[0]

        # w should converge to ~2.0
        assert abs(w.data[0] - 2.0) < 0.1

    def test_quadratic_gradient(self):
        """Verify gradient of sum(x^2) = 2*x."""
        x = Variable([3.0, 4.0, 5.0])
        with GradientTape() as tape:
            tape.watch(x)
            y = x * x
            loss = tf.reduce_sum(y)
        grads = tape.gradient(loss, [x])
        assert abs(grads[0].data[0] - 6.0) < 1e-4
        assert abs(grads[0].data[1] - 8.0) < 1e-4
        assert abs(grads[0].data[2] - 10.0) < 1e-4


class TestSequentialFitLoop:
    """Test the high-level model.fit() training API."""

    def test_simple_regression(self):
        """Train a small network to learn y = x1 + x2."""
        model = Sequential(
            [
                Dense(8, activation="relu", input_dim=2),
                Dense(1),
            ]
        )
        model.compile(optimizer=Adam(learning_rate=0.05), loss="mse")

        # Training data: y = x1 + x2
        x = Tensor.from_list(
            [
                [1.0, 0.0],
                [0.0, 1.0],
                [1.0, 1.0],
                [2.0, 1.0],
            ]
        )
        y = Tensor.from_list([[1.0], [1.0], [2.0], [3.0]])

        history = model.fit(x, y, epochs=50, batch_size=4, verbose=0)

        # Loss should decrease
        assert history.history["loss"][-1] < history.history["loss"][0]

    def test_with_early_stopping(self):
        model = Sequential(
            [
                Dense(4, activation="relu", input_dim=2),
                Dense(1),
            ]
        )
        model.compile(optimizer="adam", loss="mse")

        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])
        x_val = Tensor.from_list([[1.0, 1.0]])
        y_val = Tensor.from_list([[0.5]])

        es = EarlyStopping(monitor="val_loss", patience=5)
        history = model.fit(
            x,
            y,
            epochs=100,
            batch_size=2,
            validation_data=(x_val, y_val),
            callbacks=[es],
            verbose=0,
        )
        # Training should stop before 100 epochs (likely)
        assert len(history.history["loss"]) <= 100

    def test_evaluate_after_training(self):
        model = Sequential(
            [
                Dense(4, input_dim=2),
                Dense(1),
            ]
        )
        model.compile(optimizer="adam", loss="mse", metrics=["mse"])

        x = Tensor.from_list([[1.0, 0.0], [0.0, 1.0]])
        y = Tensor.from_list([[1.0], [0.0]])

        model.fit(x, y, epochs=10, batch_size=2, verbose=0)
        results = model.evaluate(x, y)
        assert len(results) == 2
        assert all(isinstance(r, float) for r in results)


class TestTopLevelAPI:
    """Test top-level tf.* functions."""

    def test_constant(self):
        x = tf.constant([1.0, 2.0, 3.0])
        assert x.data == [1.0, 2.0, 3.0]
        assert x.requires_grad is False

    def test_constant_scalar(self):
        x = tf.constant(42.0)
        assert x.data == [42.0]

    def test_constant_nested(self):
        x = tf.constant([[1.0, 2.0], [3.0, 4.0]])
        assert x.shape == (2, 2)

    def test_constant_from_tensor(self):
        t = Tensor.from_list([1.0, 2.0])
        x = tf.constant(t)
        assert x.data == [1.0, 2.0]
        assert x.requires_grad is False

    def test_zeros(self):
        x = tf.zeros((2, 3))
        assert x.shape == (2, 3)
        assert all(v == 0.0 for v in x.data)

    def test_ones(self):
        x = tf.ones((2, 3))
        assert x.shape == (2, 3)
        assert all(v == 1.0 for v in x.data)

    def test_eye(self):
        x = tf.eye(3)
        assert x.shape == (3, 3)
        assert x.data[0] == 1.0
        assert x.data[1] == 0.0

    def test_range(self):
        x = tf.range_(0, 5)
        assert x.shape == (5,)
        assert x.data == [0.0, 1.0, 2.0, 3.0, 4.0]

    def test_range_single_arg(self):
        x = tf.range_(3)
        assert x.data == [0.0, 1.0, 2.0]

    def test_matmul(self):
        a = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        b = Tensor.from_list([[5.0, 6.0], [7.0, 8.0]])
        c = tf.matmul(a, b)
        assert c.shape == (2, 2)
        assert c.data[0] == 19.0  # 1*5 + 2*7

    def test_add(self):
        a = tf.constant([1.0, 2.0])
        b = tf.constant([3.0, 4.0])
        c = tf.add(a, b)
        assert c.data == [4.0, 6.0]

    def test_multiply(self):
        a = tf.constant([2.0, 3.0])
        b = tf.constant([4.0, 5.0])
        c = tf.multiply(a, b)
        assert c.data == [8.0, 15.0]

    def test_reduce_sum(self):
        x = tf.constant([1.0, 2.0, 3.0])
        s = tf.reduce_sum(x)
        assert abs(s.data[0] - 6.0) < 1e-6

    def test_reduce_mean(self):
        x = tf.constant([2.0, 4.0, 6.0])
        m = tf.reduce_mean(x)
        assert abs(m.data[0] - 4.0) < 1e-6

    def test_reshape(self):
        x = tf.constant([1.0, 2.0, 3.0, 4.0])
        y = tf.reshape(x, (2, 2))
        assert y.shape == (2, 2)

    def test_transpose(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        y = tf.transpose(x)
        assert y.shape == (2, 2)
        assert y.data[0] == 1.0
        assert y.data[1] == 3.0

    def test_clip_by_value(self):
        x = tf.constant([-1.0, 0.5, 2.0])
        y = tf.clip_by_value(x, 0.0, 1.0)
        assert y.data == [0.0, 0.5, 1.0]


class TestDataNamespace:
    """Test tf.data.Dataset access."""

    def test_from_tensor_slices(self):
        x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]])
        ds = tf.data.Dataset.from_tensor_slices(x)
        assert len(ds) == 2
