"""
================================================================
MODELS — THE HEART OF KERAS: COMPILE, FIT, PREDICT
================================================================

A Keras Model brings everything together:
- Layers define the architecture
- compile() connects the optimizer, loss, and metrics
- fit() trains the model (the famous 3-line training loop)
- predict() runs inference
- evaluate() measures performance on test data

=== The Two APIs for Building Models ===

1. Sequential API — for simple stack-of-layers architectures:

    model = Sequential([
        Dense(128, activation="relu"),
        Dropout(0.2),
        Dense(10, activation="softmax"),
    ])

2. Functional API — for complex architectures with branches:

    inputs = Input(shape=(784,))
    x = Dense(128, activation="relu")(inputs)
    outputs = Dense(10, activation="softmax")(x)
    model = Model(inputs=inputs, outputs=outputs)

Both model types share the same compile/fit/evaluate/predict API.

=== The Training Loop (model.fit) ===

    model.fit(x, y, epochs=10, batch_size=32)

Under the hood, this does:
    for epoch in range(epochs):
        for batch in split_data_into_batches(x, y, batch_size):
            predictions = model(batch_x)           # forward pass
            loss = loss_fn(batch_y, predictions)    # compute loss
            loss.backward()                         # compute gradients
            optimizer.apply_gradients(grads, params) # update weights
            zero_all_gradients()                    # reset for next batch

This is the same training loop you'd write in PyTorch, but
abstracted into a single method call. That's the Keras philosophy:
make the common case trivially easy.

================================================================
"""

from __future__ import annotations

from typing import Any

from ml_framework_core import Parameter, Tensor

from .callbacks import Callback, History
from .layers import Input, Layer
from .losses import Loss, get_loss
from .metrics import Metric, get_metric
from .optimizers import Optimizer, get_optimizer


# =========================================================================
# Base Model
# =========================================================================


class BaseModel:
    """Shared logic for Sequential and Functional models.

    Handles compile(), fit(), evaluate(), predict(), summary(),
    and weight management.
    """

    def __init__(self) -> None:
        self._layers: list[Layer] = []
        self._optimizer: Optimizer | None = None
        self._loss_fn: Loss | None = None
        self._metrics: list[Metric] = []
        self._compiled = False

    # ─── Layer / Weight access ───────────────────────────────────

    @property
    def layers(self) -> list[Layer]:
        return list(self._layers)

    @property
    def trainable_weights(self) -> list[Parameter]:
        """Collect all trainable weights from all layers."""
        weights: list[Parameter] = []
        for layer in self._layers:
            weights.extend(layer.trainable_weights)
        return weights

    def count_params(self) -> int:
        """Total number of trainable parameters."""
        return sum(p.numel for p in self.trainable_weights)

    # ─── Forward pass ────────────────────────────────────────────

    def __call__(self, inputs: Tensor, training: bool | None = None) -> Tensor:
        """Run the forward pass through all layers."""
        x = inputs
        for layer in self._layers:
            x = layer(x, training=training)
        return x

    # ─── Compile ─────────────────────────────────────────────────

    def compile(
        self,
        optimizer: str | Optimizer = "adam",
        loss: str | Loss | None = None,
        metrics: list[str | Metric] | None = None,
    ) -> None:
        """Configure the model for training.

        This is where you specify HOW the model should be trained:
        - optimizer: which algorithm updates the weights
        - loss: what the model tries to minimize
        - metrics: what we monitor (for our benefit, not training)

        Args:
            optimizer: Optimizer name or instance. Default: "adam".
            loss: Loss function name or instance.
            metrics: List of metric names or instances.

        Example:
            model.compile(
                optimizer="adam",
                loss="categorical_crossentropy",
                metrics=["accuracy"],
            )
        """
        self._optimizer = get_optimizer(optimizer)
        self._loss_fn = get_loss(loss) if loss is not None else None
        self._metrics = [get_metric(m) for m in (metrics or [])]
        self._compiled = True

    # ─── Zero gradients ──────────────────────────────────────────

    def zero_grad(self) -> None:
        """Reset all parameter gradients to None.

        Must be called before each backward pass to prevent gradient
        accumulation across batches. (In some advanced techniques,
        gradient accumulation is intentional — but not in standard training.)
        """
        for param in self.trainable_weights:
            param.grad = None

    # ─── Fit (training loop) ─────────────────────────────────────

    def fit(
        self,
        x: Tensor,
        y: Tensor,
        epochs: int = 1,
        batch_size: int = 32,
        validation_data: tuple[Tensor, Tensor] | None = None,
        validation_split: float = 0.0,
        callbacks: list[Callback] | None = None,
        verbose: int = 1,
    ) -> History:
        """Train the model on data.

        This is the killer feature of Keras — the entire training loop
        in one method call. Under the hood it runs:

            for epoch in range(epochs):
                for batch in batches:
                    pred = model(x_batch)
                    loss = loss_fn(y_batch, pred)
                    loss.backward()
                    optimizer.apply_gradients(grads, params)

        Args:
            x: Input data tensor.
            y: Target data tensor.
            epochs: Number of full passes through the data.
            batch_size: Number of samples per gradient update.
            validation_data: Tuple of (x_val, y_val) for validation.
            validation_split: Fraction of training data to use as validation.
            callbacks: List of Callback instances.
            verbose: 0 = silent, 1 = progress bar, 2 = one line per epoch.

        Returns:
            History object with training metrics per epoch.
        """
        if not self._compiled:
            raise RuntimeError(
                "Model must be compiled before training. Call model.compile()."
            )
        assert self._optimizer is not None
        assert self._loss_fn is not None

        # ─── Set up callbacks ────────────────────────────────────
        history = History()
        all_callbacks = list(callbacks or [])
        all_callbacks.append(history)

        # Give callbacks a reference to the model
        for cb in all_callbacks:
            if hasattr(cb, "_model"):
                cb._model = self

        # ─── Validation split ────────────────────────────────────
        if validation_split > 0.0 and validation_data is None:
            n = x.shape[0]
            split_idx = int(n * (1 - validation_split))
            # Split tensors manually
            x_train_data = x.data[: split_idx * _features(x)]
            x_val_data = x.data[split_idx * _features(x) :]
            y_train_data = y.data[: split_idx * _features(y)]
            y_val_data = y.data[split_idx * _features(y) :]

            x_shape_train = (split_idx, *x.shape[1:])
            x_shape_val = (n - split_idx, *x.shape[1:])
            y_shape_train = (split_idx, *y.shape[1:])
            y_shape_val = (n - split_idx, *y.shape[1:])

            x = Tensor(x_train_data, x_shape_train)
            y = Tensor(y_train_data, y_shape_train)
            validation_data = (
                Tensor(x_val_data, x_shape_val),
                Tensor(y_val_data, y_shape_val),
            )

        n_samples = x.shape[0]

        # ─── Notify: training begins ────────────────────────────
        for cb in all_callbacks:
            cb.on_train_begin({})

        # ─── Epoch loop ─────────────────────────────────────────
        for epoch in range(epochs):
            for cb in all_callbacks:
                cb.on_epoch_begin(epoch, {})

            epoch_loss = 0.0
            n_batches = 0

            # ─── Mini-batch loop ─────────────────────────────────
            for start in range(0, n_samples, batch_size):
                end = min(start + batch_size, n_samples)
                actual_batch_size = end - start

                # Slice batch from tensors
                x_batch = _slice_batch(x, start, end)
                y_batch = _slice_batch(y, start, end)

                # Forward pass
                pred = self(x_batch, training=True)

                # Compute loss
                loss = self._loss_fn(y_batch, pred)

                # Backward pass
                self.zero_grad()
                loss.backward()

                # Optimizer step
                grads_and_vars = [(p.grad, p) for p in self.trainable_weights]
                self._optimizer.apply_gradients(grads_and_vars)

                # Accumulate epoch loss
                epoch_loss += loss.data[0] * actual_batch_size
                n_batches += 1

            # ─── Epoch metrics ───────────────────────────────────
            logs: dict[str, Any] = {"loss": epoch_loss / n_samples}

            # Compute training metrics
            for metric in self._metrics:
                metric.reset_state()
                # Evaluate on full training data (without grad)
                train_pred = self(x, training=False)
                metric.update_state(y, train_pred)
                logs[metric.name] = metric.result()

            # ─── Validation ──────────────────────────────────────
            if validation_data is not None:
                x_val, y_val = validation_data
                val_pred = self(x_val, training=False)
                val_loss = self._loss_fn(y_val, val_pred)
                logs["val_loss"] = val_loss.data[0]

                for metric in self._metrics:
                    metric.reset_state()
                    metric.update_state(y_val, val_pred)
                    logs[f"val_{metric.name}"] = metric.result()

            # ─── Print progress ──────────────────────────────────
            if verbose >= 1:
                parts = [f"Epoch {epoch + 1}/{epochs}"]
                for key, val in logs.items():
                    if isinstance(val, float):
                        parts.append(f"{key}: {val:.4f}")
                    else:
                        parts.append(f"{key}: {val}")
                print(" - ".join(parts))

            # ─── Notify: epoch ends ──────────────────────────────
            for cb in all_callbacks:
                cb.on_epoch_end(epoch, logs)

            # ─── Check early stopping ────────────────────────────
            if any(getattr(cb, "_stopped", False) for cb in all_callbacks):
                if verbose >= 1:
                    print(f"Early stopping at epoch {epoch + 1}")
                break

        # ─── Notify: training ends ───────────────────────────────
        for cb in all_callbacks:
            cb.on_train_end({})

        return history

    # ─── Evaluate ────────────────────────────────────────────────

    def evaluate(
        self,
        x: Tensor,
        y: Tensor,
        batch_size: int = 32,
        verbose: int = 0,
    ) -> tuple[float, ...]:
        """Evaluate the model on test data.

        Returns the loss and metric values.

        Args:
            x: Test input data.
            y: Test target data.
            batch_size: Batch size for evaluation.
            verbose: Verbosity mode.

        Returns:
            Tuple of (loss, metric1, metric2, ...).
        """
        if self._loss_fn is None:
            raise RuntimeError("Model must be compiled before evaluation.")

        pred = self(x, training=False)
        loss = self._loss_fn(y, pred)

        results = [loss.data[0]]

        for metric in self._metrics:
            metric.reset_state()
            metric.update_state(y, pred)
            results.append(metric.result())

        if verbose >= 1:
            parts = [f"loss: {results[0]:.4f}"]
            for i, metric in enumerate(self._metrics):
                parts.append(f"{metric.name}: {results[i + 1]:.4f}")
            print(" - ".join(parts))

        return tuple(results)

    # ─── Predict ─────────────────────────────────────────────────

    def predict(self, x: Tensor, batch_size: int = 32) -> Tensor:
        """Generate predictions for input data.

        Args:
            x: Input data tensor.
            batch_size: Batch size (currently processes all at once).

        Returns:
            Prediction tensor.
        """
        return self(x, training=False)

    # ─── Summary ─────────────────────────────────────────────────

    def summary(self) -> str:
        """Print a summary of the model architecture.

        Shows each layer, its output shape, and parameter count.
        This is one of Keras's most loved features — a quick
        overview of your model at a glance.

        Returns:
            The summary string (also printed to stdout).
        """
        lines = []
        lines.append("=" * 60)
        lines.append(f"Model: {self.__class__.__name__}")
        lines.append("=" * 60)
        lines.append(f"{'Layer (type)':<30} {'Param #':>10}")
        lines.append("-" * 60)

        total_params = 0
        for layer in self._layers:
            name = f"{layer.__class__.__name__}"
            params = layer.count_params()
            total_params += params
            lines.append(f"{name:<30} {params:>10,}")

        lines.append("=" * 60)
        lines.append(f"Total params: {total_params:,}")
        lines.append("=" * 60)

        summary_str = "\n".join(lines)
        print(summary_str)
        return summary_str


# =========================================================================
# Sequential Model
# =========================================================================


class Sequential(BaseModel):
    """A linear stack of layers.

    The simplest way to build a model in Keras. Layers are added
    in order, and data flows through them one by one:

        model = Sequential([
            Dense(128, activation="relu"),
            Dropout(0.2),
            Dense(10, activation="softmax"),
        ])

    Equivalent to:
        x → Dense(128, relu) → Dropout(0.2) → Dense(10, softmax) → output

    When to use Sequential vs Functional:
    - Sequential: simple feedforward networks (most common)
    - Functional: skip connections, multiple inputs/outputs, branching

    Args:
        layers: List of Layer instances.
    """

    def __init__(self, layers: list[Layer] | None = None) -> None:
        super().__init__()
        if layers:
            for layer in layers:
                self.add(layer)

    def add(self, layer: Layer) -> None:
        """Add a layer to the end of the stack.

        Args:
            layer: A Layer instance to add.
        """
        self._layers.append(layer)


# =========================================================================
# Functional Model
# =========================================================================


class Model(BaseModel):
    """Model created with the Functional API.

    The Functional API lets you build complex architectures by
    explicitly connecting layers:

        inputs = Input(shape=(784,))
        x = Dense(128, activation="relu")(inputs)
        x = Dropout(0.3)(x)
        outputs = Dense(10, activation="softmax")(x)
        model = Model(inputs=inputs, outputs=outputs)

    For our implementation, we support linear chains (same as
    Sequential but built differently). The layers are extracted
    from the Input → ... → output chain.

    Args:
        inputs: An Input instance marking the start of the graph.
        outputs: The final symbolic tensor (layer output).
    """

    def __init__(
        self,
        inputs: Input | None = None,
        outputs: Any = None,
        **kwargs: Any,
    ) -> None:
        super().__init__()

        if inputs is not None and outputs is not None:
            # Extract the chain of layers from the graph
            self._build_from_graph(inputs, outputs)

    def _build_from_graph(self, inputs: Input, outputs: Any) -> None:
        """Extract layers from the symbolic computation graph.

        Walk backwards from outputs to inputs, collecting layers
        in order. This gives us the linear chain for forward pass.
        """
        # The outputs object tracks the chain of layers applied
        # We stored this chain in the Input._layers list during
        # layer __call__ (see the monkey-patched __call__ below)
        if hasattr(inputs, "_layers") and inputs._layers:
            self._layers = list(inputs._layers)


# =========================================================================
# Monkey-patch Layer.__call__ to support Functional API
# =========================================================================
#
# When a layer is called on an Input or _SymbolicTensor, instead of
# doing a real forward pass, we record the layer in the graph.

_original_layer_call = Layer.__call__


def _patched_layer_call(self: Layer, *args: Any, **kwargs: Any) -> Any:
    """Enhanced __call__ that supports both real and symbolic inputs."""
    first_arg = args[0] if args else None

    # If the input is an Input placeholder, record the layer symbolically
    if isinstance(first_arg, Input):
        first_arg._layers.append(self)
        return first_arg  # Return the Input so further layers can chain

    # Otherwise, do the normal forward pass
    return _original_layer_call(self, *args, **kwargs)


Layer.__call__ = _patched_layer_call


# =========================================================================
# Helpers
# =========================================================================


def _features(t: Tensor) -> int:
    """Number of features per sample (product of all dims except batch)."""
    result = 1
    for dim in t.shape[1:]:
        result *= dim
    return result


def _slice_batch(t: Tensor, start: int, end: int) -> Tensor:
    """Slice a tensor along the batch dimension (dim 0).

    For a tensor of shape (N, d1, d2, ...), returns the subtensor
    with samples [start:end], shape (end-start, d1, d2, ...).
    """
    features_per_sample = _features(t)
    data_start = start * features_per_sample
    data_end = end * features_per_sample
    batch_shape = (end - start, *t.shape[1:])
    return Tensor(t.data[data_start:data_end], batch_shape)
