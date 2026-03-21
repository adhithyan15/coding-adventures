"""
================================================================
CALLBACKS — HOOKS INTO THE TRAINING LOOP
================================================================

Callbacks let you inject custom behavior at specific points during
training WITHOUT modifying the training loop itself. This is the
Observer pattern applied to machine learning.

=== When Callbacks Fire ===

    model.fit(x, y, epochs=10, callbacks=[my_callback])

    on_train_begin(logs)          ← once, before all epochs
      on_epoch_begin(epoch, logs) ← before each epoch
        [training batches]
      on_epoch_end(epoch, logs)   ← after each epoch (with metrics!)
    on_train_end(logs)            ← once, after all epochs

The `logs` dict contains metrics like:
    {"loss": 0.42, "accuracy": 0.87, "val_loss": 0.51}

=== Built-in Callbacks ===

| Callback              | Purpose                                   |
|-----------------------|-------------------------------------------|
| History               | Records loss/metrics per epoch             |
| EarlyStopping         | Stop training when metric stops improving  |
| ModelCheckpoint       | Save model when metric improves            |
| LearningRateScheduler | Adjust learning rate per epoch             |

=== Custom Callbacks ===

You can create your own by subclassing Callback:

    class PrintLoss(Callback):
        def on_epoch_end(self, epoch, logs=None):
            print(f"Epoch {epoch}: loss = {logs.get('loss')}")

================================================================
"""

from __future__ import annotations

from typing import Any


# =========================================================================
# Base Callback
# =========================================================================


class Callback:
    """Base class for training callbacks.

    Override any of the on_* methods to inject behavior at the
    corresponding point in the training loop. All methods receive
    a `logs` dict with current metrics.

    The default implementations do nothing — override only what you need.
    """

    def on_train_begin(self, logs: dict[str, Any] | None = None) -> None:
        """Called at the start of training."""

    def on_train_end(self, logs: dict[str, Any] | None = None) -> None:
        """Called at the end of training."""

    def on_epoch_begin(self, epoch: int, logs: dict[str, Any] | None = None) -> None:
        """Called at the start of each epoch."""

    def on_epoch_end(self, epoch: int, logs: dict[str, Any] | None = None) -> None:
        """Called at the end of each epoch (with metrics in logs)."""


# =========================================================================
# History — Record Training Metrics
# =========================================================================


class History(Callback):
    """Records training metrics for each epoch.

    This callback is automatically added by model.fit() and returned
    as the result. After training:

        history = model.fit(x, y, epochs=10)
        history.history["loss"]      # [0.9, 0.7, 0.5, 0.4, ...]
        history.history["val_loss"]  # [1.0, 0.8, 0.6, 0.5, ...]

    You can plot these to visualize training progress:
        - Is loss decreasing? (learning)
        - Is val_loss increasing while train_loss decreases? (overfitting)
        - Has val_loss plateaued? (time to stop)
    """

    def __init__(self) -> None:
        super().__init__()
        self.history: dict[str, list[float]] = {}
        self.epoch: list[int] = []

    def on_epoch_end(self, epoch: int, logs: dict[str, Any] | None = None) -> None:
        """Record all metrics from this epoch."""
        logs = logs or {}
        self.epoch.append(epoch)
        for key, value in logs.items():
            if key not in self.history:
                self.history[key] = []
            self.history[key].append(float(value))


# =========================================================================
# EarlyStopping — Stop When No Improvement
# =========================================================================


class EarlyStopping(Callback):
    """Stop training when a monitored metric stops improving.

    Training a model too long leads to overfitting — the model
    memorizes the training data instead of learning general patterns.
    EarlyStopping watches a metric (usually val_loss) and stops
    training if it hasn't improved for `patience` epochs.

    === How It Works ===

    1. Track the best value of `monitor` seen so far
    2. After each epoch, check if `monitor` improved
    3. If it didn't improve, increment a counter
    4. If the counter reaches `patience`, stop training
    5. Optionally, restore the model weights from the best epoch

    Args:
        monitor: Metric to watch. Default: "val_loss".
        patience: Number of epochs with no improvement before stopping.
            Default: 5.
        restore_best_weights: If True, restore model to best epoch's weights.
            Default: False.
        min_delta: Minimum change to qualify as an improvement. Default: 0.0.

    Example:
        model.fit(x, y, epochs=100, callbacks=[
            EarlyStopping(monitor="val_loss", patience=10)
        ])
        # Training stops after 10 epochs without val_loss improvement
    """

    def __init__(
        self,
        monitor: str = "val_loss",
        patience: int = 5,
        restore_best_weights: bool = False,
        min_delta: float = 0.0,
    ) -> None:
        super().__init__()
        self.monitor = monitor
        self.patience = patience
        self.restore_best_weights = restore_best_weights
        self.min_delta = min_delta

        # Internal state
        self._best_value: float | None = None
        self._wait = 0
        self._stopped = False
        self._best_weights: list[list[float]] | None = None
        self._model: Any = None

    def on_train_begin(self, logs: dict[str, Any] | None = None) -> None:
        self._best_value = None
        self._wait = 0
        self._stopped = False

    def on_epoch_end(self, epoch: int, logs: dict[str, Any] | None = None) -> None:
        """Check if monitored metric improved."""
        logs = logs or {}
        current = logs.get(self.monitor)
        if current is None:
            return

        current = float(current)

        # First epoch: initialize best value
        if self._best_value is None:
            self._best_value = current
            if self.restore_best_weights and self._model is not None:
                self._best_weights = [
                    list(p.data) for p in self._model.trainable_weights
                ]
            return

        # Check for improvement (lower is better for loss metrics)
        if self._is_improvement(current):
            self._best_value = current
            self._wait = 0
            if self.restore_best_weights and self._model is not None:
                self._best_weights = [
                    list(p.data) for p in self._model.trainable_weights
                ]
        else:
            self._wait += 1
            if self._wait >= self.patience:
                self._stopped = True
                # Restore best weights if requested
                if (
                    self.restore_best_weights
                    and self._best_weights is not None
                    and self._model is not None
                ):
                    for param, best_data in zip(
                        self._model.trainable_weights, self._best_weights
                    ):
                        param.data = list(best_data)

    def _is_improvement(self, current: float) -> bool:
        """Check if current value is better than best (lower is better)."""
        assert self._best_value is not None
        return current < self._best_value - self.min_delta


# =========================================================================
# ModelCheckpoint — Save Model at Best Epoch
# =========================================================================


class ModelCheckpoint(Callback):
    """Save the model when a monitored metric improves.

    In real Keras, this saves model weights to disk. In our
    implementation, we store the weights in memory since we don't
    have a serialization format yet.

    Args:
        filepath: Path to save the model. (Symbolic in our implementation.)
        save_best_only: If True, only save when metric improves. Default: True.
        monitor: Metric to watch. Default: "val_loss".
    """

    def __init__(
        self,
        filepath: str = "model_checkpoint",
        save_best_only: bool = True,
        monitor: str = "val_loss",
    ) -> None:
        super().__init__()
        self.filepath = filepath
        self.save_best_only = save_best_only
        self.monitor = monitor

        self._best_value: float | None = None
        self._saved_weights: list[list[float]] | None = None
        self._model: Any = None

    def on_epoch_end(self, epoch: int, logs: dict[str, Any] | None = None) -> None:
        logs = logs or {}
        current = logs.get(self.monitor)

        if current is None:
            # If monitor metric not available, save anyway if not save_best_only
            if not self.save_best_only and self._model is not None:
                self._saved_weights = [
                    list(p.data) for p in self._model.trainable_weights
                ]
            return

        current = float(current)

        if self._best_value is None or current < self._best_value:
            self._best_value = current
            if self._model is not None:
                self._saved_weights = [
                    list(p.data) for p in self._model.trainable_weights
                ]
        elif not self.save_best_only and self._model is not None:
            self._saved_weights = [list(p.data) for p in self._model.trainable_weights]


# =========================================================================
# LearningRateScheduler
# =========================================================================


class LearningRateScheduler(Callback):
    """Adjust the learning rate according to a schedule function.

    The schedule function receives the current epoch and current
    learning rate, and returns the new learning rate:

        def schedule(epoch, lr):
            if epoch < 10:
                return 0.001
            else:
                return 0.0001

    Common schedules:
    - Step decay: reduce LR by a factor every N epochs
    - Exponential decay: lr = initial_lr * decay^epoch
    - Warmup: linearly increase LR for first few epochs
    - Cosine annealing: oscillate LR following a cosine curve

    Args:
        schedule: Function(epoch, lr) → new_lr.
    """

    def __init__(self, schedule: Any) -> None:
        super().__init__()
        self.schedule = schedule
        self._model: Any = None

    def on_epoch_begin(self, epoch: int, logs: dict[str, Any] | None = None) -> None:
        """Adjust learning rate at the start of each epoch."""
        if self._model is not None and hasattr(self._model, "_optimizer"):
            current_lr = self._model._optimizer.learning_rate
            new_lr = self.schedule(epoch, current_lr)
            self._model._optimizer.learning_rate = new_lr
