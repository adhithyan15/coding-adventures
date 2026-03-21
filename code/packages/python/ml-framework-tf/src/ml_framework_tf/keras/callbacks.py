"""
================================================================
TF.KERAS.CALLBACKS — HOOKS INTO THE TRAINING LOOP
================================================================

Callbacks are objects that perform actions at various stages of
training. They're TensorFlow's way of adding custom behavior
without modifying the training loop itself.

=== When Callbacks Fire ===

    model.fit(x, y, callbacks=[my_callback])

    on_train_begin()                    # once at start
    for epoch in range(epochs):
        on_epoch_begin(epoch)           # start of each epoch
        for batch in batches:
            on_train_batch_begin(batch) # start of each batch
            ... training step ...
            on_train_batch_end(batch)   # end of each batch
        on_epoch_end(epoch, logs)       # end of each epoch
    on_train_end()                      # once at end

=== Built-in Callbacks ===

| Callback              | Purpose                                |
|-----------------------|----------------------------------------|
| History               | Records loss/metrics per epoch         |
| EarlyStopping         | Stops training when metric plateaus    |
| ModelCheckpoint       | Saves model at best performance        |
| LearningRateScheduler | Adjusts learning rate during training  |

=== History Callback ===

History is automatically added to every fit() call. It stores
per-epoch metrics in a dictionary:

    history = model.fit(x, y, epochs=10)
    history.history['loss']       # [0.5, 0.3, 0.2, ...]
    history.history['accuracy']   # [0.7, 0.8, 0.85, ...]

================================================================
"""

from __future__ import annotations


class Callback:
    """Base class for all callbacks.

    Subclasses override the on_* methods they care about.
    All methods receive relevant context (epoch number, logs dict).

    The logs dict contains metric values for the current epoch:
        {'loss': 0.25, 'accuracy': 0.92, 'val_loss': 0.30}
    """

    def on_train_begin(self, logs: dict | None = None) -> None:
        """Called once at the start of training."""
        pass

    def on_train_end(self, logs: dict | None = None) -> None:
        """Called once at the end of training."""
        pass

    def on_epoch_begin(self, epoch: int, logs: dict | None = None) -> None:
        """Called at the start of each epoch."""
        pass

    def on_epoch_end(self, epoch: int, logs: dict | None = None) -> None:
        """Called at the end of each epoch."""
        pass


class History(Callback):
    """Records training metrics per epoch.

    This callback is automatically created by model.fit() and
    returned as the fit() return value.

    Attributes:
        history: Dict mapping metric names to lists of per-epoch values.
                 Example: {'loss': [0.5, 0.3], 'accuracy': [0.8, 0.9]}

    Example:
        result = model.fit(x, y, epochs=5)
        plt.plot(result.history['loss'])  # plot training curve
    """

    def __init__(self) -> None:
        super().__init__()
        self.history: dict[str, list[float]] = {}

    def on_epoch_end(self, epoch: int, logs: dict | None = None) -> None:
        """Append this epoch's metrics to the history."""
        if logs is None:
            return
        for key, value in logs.items():
            if key not in self.history:
                self.history[key] = []
            self.history[key].append(value)


class EarlyStopping(Callback):
    """Stop training when a monitored metric has stopped improving.

    This prevents overfitting by halting training when the validation
    loss (or another metric) stops decreasing.

    How it works:
    1. After each epoch, check the monitored metric
    2. If it improved, reset the patience counter
    3. If it didn't improve for `patience` epochs, stop training

    Args:
        monitor: Metric to watch. Default: 'val_loss'.
        patience: Number of epochs with no improvement before stopping.
                  Default: 5.
        min_delta: Minimum change to qualify as an improvement.
                   Default: 0.0.
        restore_best_weights: If True, restore model weights from the
                              epoch with the best value. Default: False.

    Example:
        early_stop = EarlyStopping(monitor='val_loss', patience=3)
        model.fit(x, y, callbacks=[early_stop])
    """

    def __init__(
        self,
        monitor: str = "val_loss",
        patience: int = 5,
        min_delta: float = 0.0,
        restore_best_weights: bool = False,
    ) -> None:
        super().__init__()
        self.monitor = monitor
        self.patience = patience
        self.min_delta = min_delta
        self.restore_best_weights = restore_best_weights
        self._best: float | None = None
        self._wait = 0
        self.stopped_epoch = 0
        self.stop_training = False

    def on_train_begin(self, logs: dict | None = None) -> None:
        self._best = None
        self._wait = 0
        self.stop_training = False

    def on_epoch_end(self, epoch: int, logs: dict | None = None) -> None:
        if logs is None:
            return

        current = logs.get(self.monitor)
        if current is None:
            return

        if self._best is None or current < self._best - self.min_delta:
            self._best = current
            self._wait = 0
        else:
            self._wait += 1
            if self._wait >= self.patience:
                self.stop_training = True
                self.stopped_epoch = epoch


class ModelCheckpoint(Callback):
    """Save the model when a monitored metric improves.

    In our simplified implementation, this records the best metric
    value and the epoch at which it occurred (since we don't have
    file I/O for actual model saving).

    Args:
        filepath: Path to save the model (recorded but not used).
        monitor: Metric to watch. Default: 'val_loss'.
        save_best_only: If True, only save when metric improves.
                        Default: True.

    Example:
        checkpoint = ModelCheckpoint('best_model.h5')
        model.fit(x, y, callbacks=[checkpoint])
    """

    def __init__(
        self,
        filepath: str = "model_checkpoint",
        monitor: str = "val_loss",
        save_best_only: bool = True,
    ) -> None:
        super().__init__()
        self.filepath = filepath
        self.monitor = monitor
        self.save_best_only = save_best_only
        self.best: float | None = None
        self.best_epoch: int = 0

    def on_epoch_end(self, epoch: int, logs: dict | None = None) -> None:
        if logs is None:
            return

        current = logs.get(self.monitor)
        if current is None:
            return

        if self.best is None or current < self.best:
            self.best = current
            self.best_epoch = epoch


class LearningRateScheduler(Callback):
    """Adjust the learning rate according to a schedule function.

    The schedule function receives the epoch number and current
    learning rate, and returns the new learning rate:

        def schedule(epoch, lr):
            if epoch < 10:
                return lr
            return lr * 0.9  # decay by 10% per epoch after 10

    Args:
        schedule: A function (epoch, lr) → new_lr.

    Example:
        scheduler = LearningRateScheduler(lambda e, lr: lr * 0.95)
        model.fit(x, y, callbacks=[scheduler])
    """

    def __init__(self, schedule: callable) -> None:
        super().__init__()
        self.schedule = schedule
        self._optimizer = None

    def set_optimizer(self, optimizer: object) -> None:
        """Link the scheduler to the optimizer being used."""
        self._optimizer = optimizer

    def on_epoch_begin(self, epoch: int, logs: dict | None = None) -> None:
        if self._optimizer is None:
            return
        current_lr = self._optimizer.learning_rate
        new_lr = self.schedule(epoch, current_lr)
        self._optimizer.learning_rate = new_lr
