"""
================================================================
TF.KERAS.MODELS — HIGH-LEVEL MODEL ABSTRACTIONS
================================================================

Keras models combine layers, a loss function, an optimizer, and
metrics into a single object that handles the entire training loop.

=== The Keras Training API ===

The flagship feature of Keras is the compile/fit/evaluate/predict API:

    model = Sequential([
        Dense(128, activation='relu', input_dim=784),
        Dense(10, activation='softmax'),
    ])

    model.compile(
        optimizer='adam',
        loss='sparse_categorical_crossentropy',
        metrics=['accuracy'],
    )

    history = model.fit(x_train, y_train, epochs=10, batch_size=32)
    loss, accuracy = model.evaluate(x_test, y_test)
    predictions = model.predict(x_new)

This is dramatically more concise than PyTorch's manual training loop.
The tradeoff: less flexibility for custom training (though TF's
GradientTape provides that escape hatch).

=== Sequential vs Functional API ===

**Sequential**: Stack layers linearly, one after another.
    Simple and sufficient for ~80% of use cases.

**Functional API (Model class)**: Connect layers in arbitrary
    topologies (multi-input, multi-output, skip connections).
    Used for complex architectures like ResNet, Inception, etc.

=== The fit() Training Loop ===

Under the hood, model.fit() does this:

    for epoch in range(epochs):
        for x_batch, y_batch in batches:
            with GradientTape() as tape:
                predictions = model(x_batch)
                loss = loss_fn(y_batch, predictions)
            gradients = tape.gradient(loss, model.trainable_variables)
            optimizer.apply_gradients(zip(gradients, model.trainable_variables))
            update_metrics(y_batch, predictions)
        log_epoch_results()

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor

from .callbacks import (
    Callback,
    EarlyStopping,
    History,
    LearningRateScheduler,
)
from .layers import Layer
from . import losses as losses_module
from . import metrics as metrics_module
from . import optimizers as optimizers_module


# =========================================================================
# Sequential Model
# =========================================================================


class Sequential:
    """A linear stack of layers.

    The simplest Keras model: data flows through layers in order,
    each layer's output becoming the next layer's input.

    Args:
        layers: Optional list of Layer objects.

    Example:
        model = Sequential([
            Dense(128, activation='relu', input_dim=784),
            Dropout(0.3),
            Dense(10, activation='softmax'),
        ])
    """

    def __init__(self, layers: list[Layer] | None = None) -> None:
        self._layers: list[Layer] = layers or []
        self._optimizer = None
        self._loss_fn = None
        self._metrics: list = []
        self._compiled = False

    def add(self, layer: Layer) -> None:
        """Add a layer to the model.

        Layers are added in sequence: the first layer's output
        feeds into the second, and so on.
        """
        self._layers.append(layer)

    def __call__(self, x: Tensor) -> Tensor:
        """Run data through all layers in sequence."""
        for layer in self._layers:
            x = layer(x)
        return x

    @property
    def trainable_variables(self) -> list:
        """All learnable parameters across all layers."""
        params = []
        for layer in self._layers:
            params.extend(layer.trainable_weights)
        return params

    @property
    def layers(self) -> list[Layer]:
        """The list of layers in this model."""
        return list(self._layers)

    # =================================================================
    # compile — configure the training process
    # =================================================================

    def compile(
        self,
        optimizer: str | optimizers_module.Optimizer = "adam",
        loss: str | object = "mse",
        metrics: list[str | object] | None = None,
    ) -> None:
        """Configure the model for training.

        This sets up the optimizer, loss function, and metrics
        that will be used by fit(), evaluate(), and predict().

        Args:
            optimizer: String name or Optimizer instance.
            loss: String name or loss function instance.
            metrics: List of metric names or Metric instances.

        Example:
            model.compile(
                optimizer='adam',
                loss='sparse_categorical_crossentropy',
                metrics=['accuracy'],
            )
        """
        # ─── Resolve optimizer ───────────────────────────────────
        if isinstance(optimizer, str):
            opt_map = {
                "sgd": optimizers_module.SGD,
                "adam": optimizers_module.Adam,
                "rmsprop": optimizers_module.RMSprop,
                "adamw": optimizers_module.AdamW,
            }
            if optimizer not in opt_map:
                raise ValueError(f"Unknown optimizer: '{optimizer}'")
            self._optimizer = opt_map[optimizer]()
        else:
            self._optimizer = optimizer

        # ─── Resolve loss ────────────────────────────────────────
        self._loss_fn = losses_module.get(loss)

        # ─── Resolve metrics ─────────────────────────────────────
        self._metrics = []
        if metrics:
            for m in metrics:
                self._metrics.append(metrics_module.get(m))

        self._compiled = True

    # =================================================================
    # fit — the training loop
    # =================================================================

    def fit(
        self,
        x: Tensor,
        y: Tensor,
        epochs: int = 1,
        batch_size: int = 32,
        validation_data: tuple[Tensor, Tensor] | None = None,
        callbacks: list[Callback] | None = None,
        verbose: int = 1,
    ) -> History:
        """Train the model on data.

        This is the heart of Keras — a single method call that
        handles the entire training loop: batching, forward pass,
        loss computation, backward pass, and weight updates.

        Args:
            x: Input data tensor of shape (num_samples, ...).
            y: Target data tensor.
            epochs: Number of passes over the entire dataset.
            batch_size: Number of samples per gradient update.
            validation_data: Optional (x_val, y_val) tuple.
            callbacks: List of Callback objects.
            verbose: 0 = silent, 1 = progress bar, 2 = one line/epoch.

        Returns:
            History object containing per-epoch loss and metrics.

        Example:
            history = model.fit(x_train, y_train, epochs=10, batch_size=32)
            print(history.history['loss'])
        """
        if not self._compiled:
            raise RuntimeError(
                "Model must be compiled before training. Call model.compile() first."
            )

        # ─── Import GradientTape here to avoid circular imports ──
        from ..gradient_tape import GradientTape

        # ─── Set up callbacks ────────────────────────────────────
        history = History()
        all_callbacks = [history]
        if callbacks:
            all_callbacks.extend(callbacks)

        # Link LR schedulers to the optimizer
        for cb in all_callbacks:
            if isinstance(cb, LearningRateScheduler):
                cb.set_optimizer(self._optimizer)

        # ─── Fire on_train_begin ─────────────────────────────────
        for cb in all_callbacks:
            cb.on_train_begin()

        n = x.shape[0]

        # ─── Training loop ───────────────────────────────────────
        for epoch in range(epochs):
            # Fire on_epoch_begin
            for cb in all_callbacks:
                cb.on_epoch_begin(epoch)

            epoch_loss = 0.0
            num_batches = 0

            # Reset metrics for this epoch
            for m in self._metrics:
                m.reset_state()

            # ─── Mini-batch iteration ────────────────────────────
            for start in range(0, n, batch_size):
                end = min(start + batch_size, n)
                x_batch = _slice_batch(x, start, end)
                y_batch = _slice_batch(y, start, end)

                # Forward + backward via GradientTape
                with GradientTape() as tape:
                    # Watch all trainable variables
                    for var in self.trainable_variables:
                        tape.watch(var)

                    pred = self(x_batch)
                    loss = self._loss_fn(y_batch, pred)

                grads = tape.gradient(loss, self.trainable_variables)
                self._optimizer.apply_gradients(zip(grads, self.trainable_variables))

                # Accumulate loss
                batch_loss = (
                    loss.data[0] if loss.numel == 1 else sum(loss.data) / loss.numel
                )
                epoch_loss += batch_loss
                num_batches += 1

                # Update metrics
                for m in self._metrics:
                    m.update_state(y_batch, pred)

            # ─── Compute epoch metrics ───────────────────────────
            avg_loss = epoch_loss / max(num_batches, 1)
            logs: dict[str, float] = {"loss": avg_loss}

            for m in self._metrics:
                logs[m.name] = m.result()

            # ─── Validation ──────────────────────────────────────
            if validation_data is not None:
                x_val, y_val = validation_data
                val_pred = self(x_val)
                val_loss_tensor = self._loss_fn(y_val, val_pred)
                val_loss = (
                    val_loss_tensor.data[0]
                    if val_loss_tensor.numel == 1
                    else sum(val_loss_tensor.data) / val_loss_tensor.numel
                )
                logs["val_loss"] = val_loss

                # Compute validation metrics
                for m in self._metrics:
                    m.reset_state()
                    m.update_state(y_val, val_pred)
                    logs[f"val_{m.name}"] = m.result()

            # ─── Print progress ──────────────────────────────────
            if verbose >= 1:
                metrics_str = " - ".join(f"{k}: {v:.4f}" for k, v in logs.items())
                print(f"Epoch {epoch + 1}/{epochs} - {metrics_str}")

            # ─── Fire on_epoch_end ───────────────────────────────
            for cb in all_callbacks:
                cb.on_epoch_end(epoch, logs)

            # ─── Check early stopping ────────────────────────────
            for cb in all_callbacks:
                if isinstance(cb, EarlyStopping) and cb.stop_training:
                    if verbose >= 1:
                        print(f"Early stopping at epoch {epoch + 1}")
                    # Fire on_train_end before returning
                    for cb2 in all_callbacks:
                        cb2.on_train_end()
                    return history

        # ─── Fire on_train_end ───────────────────────────────────
        for cb in all_callbacks:
            cb.on_train_end()

        return history

    # =================================================================
    # evaluate — measure performance on test data
    # =================================================================

    def evaluate(
        self,
        x: Tensor,
        y: Tensor,
        verbose: int = 0,
    ) -> tuple[float, ...]:
        """Evaluate the model on test data.

        Returns the loss and any compiled metrics as a tuple:
            (loss, metric1, metric2, ...)

        Args:
            x: Test input data.
            y: Test target data.
            verbose: Verbosity level.

        Returns:
            Tuple of (loss, *metrics).
        """
        if not self._compiled:
            raise RuntimeError("Model must be compiled before evaluation.")

        pred = self(x)
        loss_tensor = self._loss_fn(y, pred)
        loss_val = (
            loss_tensor.data[0]
            if loss_tensor.numel == 1
            else sum(loss_tensor.data) / loss_tensor.numel
        )

        results = [loss_val]
        for m in self._metrics:
            m.reset_state()
            m.update_state(y, pred)
            results.append(m.result())

        if verbose >= 1:
            metrics_str = " - ".join(
                f"{k}: {v:.4f}"
                for k, v in zip(
                    ["loss"] + [m.name for m in self._metrics],
                    results,
                )
            )
            print(f"Evaluate: {metrics_str}")

        return tuple(results)

    # =================================================================
    # predict — generate output for new data
    # =================================================================

    def predict(self, x: Tensor) -> Tensor:
        """Generate predictions for input data.

        Runs the forward pass without gradient tracking.

        Args:
            x: Input data tensor.

        Returns:
            Model output tensor.
        """
        return self(x)

    # =================================================================
    # summary — print model architecture
    # =================================================================

    def summary(self) -> None:
        """Print a summary of the model architecture.

        Shows each layer's type and parameter count, similar to
        Keras' model.summary() output.
        """
        print("=" * 60)
        print("Model Summary")
        print("=" * 60)
        total_params = 0
        for i, layer in enumerate(self._layers):
            num_params = sum(p.numel for p in layer.trainable_weights)
            total_params += num_params
            print(f"Layer {i}: {layer!r}  —  {num_params} params")
        print("-" * 60)
        print(f"Total trainable parameters: {total_params}")
        print("=" * 60)


# =========================================================================
# Functional API Model
# =========================================================================


class Model(Sequential):
    """Model with support for the functional API.

    For simple use, Model behaves like Sequential. For the functional
    API, you pass inputs and outputs:

        inputs = Input(shape=(784,))
        x = Dense(128, activation='relu')(inputs)
        outputs = Dense(10, activation='softmax')(x)
        model = Model(inputs=inputs, outputs=outputs)

    Our simplified implementation stores the layers list from
    Sequential and adds compile/fit/evaluate/predict from Sequential.
    The functional API tracking is simplified.
    """

    def __init__(
        self,
        inputs: object | None = None,
        outputs: object | None = None,
        layers: list[Layer] | None = None,
    ) -> None:
        super().__init__(layers=layers)
        self._inputs = inputs
        self._outputs = outputs


# =========================================================================
# Helpers
# =========================================================================


def _slice_batch(t: Tensor, start: int, end: int) -> Tensor:
    """Extract a batch slice from a tensor along the first dimension.

    For a tensor of shape (N, D1, D2, ...):
        Returns a tensor of shape (end-start, D1, D2, ...)
    """
    if t.ndim == 1:
        return Tensor(t.data[start:end], (end - start,), device=t.device)

    inner_size = 1
    for s in t.shape[1:]:
        inner_size *= s

    flat_start = start * inner_size
    flat_end = end * inner_size
    new_shape = (end - start, *t.shape[1:])
    return Tensor(t.data[flat_start:flat_end], new_shape, device=t.device)
