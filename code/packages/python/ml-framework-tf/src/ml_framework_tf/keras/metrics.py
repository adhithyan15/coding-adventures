"""
================================================================
TF.KERAS.METRICS — MEASURING MODEL PERFORMANCE
================================================================

Metrics track how well a model performs during training and evaluation.
Unlike loss functions (which are optimized by gradient descent),
metrics are computed for human consumption — they tell us whether
the model is actually good at its task.

=== Metrics vs Losses ===

- **Loss** (e.g., cross-entropy): What the optimizer minimizes.
  Must be differentiable. Values aren't always intuitive.
- **Metric** (e.g., accuracy): What WE care about.
  Doesn't need to be differentiable. Human-readable.

You might train with cross-entropy loss but evaluate with accuracy.

=== Stateful Metrics ===

Keras metrics are stateful: they accumulate values across batches,
then compute a final result. This is important because the metric
for the full epoch should reflect ALL batches, not just the last one.

    metric = Accuracy()
    metric.update_state(y_true_batch1, y_pred_batch1)
    metric.update_state(y_true_batch2, y_pred_batch2)
    final_accuracy = metric.result()  # across both batches
    metric.reset_state()

================================================================
"""

from __future__ import annotations

import builtins as _builtins

from ml_framework_core import Tensor

# We use Python's built-in abs to avoid confusion with tf.math.abs
_builtin_abs = _builtins.abs


class Metric:
    """Base class for all Keras metrics.

    Subclasses implement:
    - update_state(): accumulate values from a batch
    - result(): compute the final metric value
    - reset_state(): clear accumulated values
    """

    def __init__(self, name: str = "metric") -> None:
        self._name = name

    @property
    def name(self) -> str:
        return self._name

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        raise NotImplementedError

    def result(self) -> float:
        raise NotImplementedError

    def reset_state(self) -> None:
        raise NotImplementedError


class Accuracy(Metric):
    """Accuracy: fraction of predictions that match the true labels.

    For regression-like outputs, compares argmax of predictions
    with argmax of targets (or integer targets directly).

    Accuracy = correct_predictions / total_predictions

    This is the most intuitive metric: "what fraction did the model
    get right?"

    Example:
        acc = Accuracy()
        acc.update_state(
            Tensor.from_list([0.0, 1.0, 2.0]),     # true labels
            Tensor.from_list([0.0, 1.0, 1.0]),      # predictions
        )
        print(acc.result())  # 0.6667
    """

    def __init__(self) -> None:
        super().__init__(name="accuracy")
        self._correct = 0
        self._total = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        """Count correct predictions in this batch.

        If y_pred is 2-D (batch, classes), uses argmax along dim 1.
        If y_pred is 1-D, compares values directly (rounded).
        """
        if y_pred.ndim == 2:
            # Multi-class: argmax prediction
            batch_size, num_classes = y_pred.shape
            for i in range(batch_size):
                row_start = i * num_classes
                row = y_pred.data[row_start : row_start + num_classes]
                pred_class = row.index(max(row))

                if y_true.ndim == 2:
                    true_row = y_true.data[i * num_classes : (i + 1) * num_classes]
                    true_class = true_row.index(max(true_row))
                else:
                    true_class = int(y_true.data[i])

                if pred_class == true_class:
                    self._correct += 1
                self._total += 1
        else:
            # Binary or exact match
            for i in range(len(y_true.data)):
                if round(y_pred.data[i]) == round(y_true.data[i]):
                    self._correct += 1
                self._total += 1

    def result(self) -> float:
        if self._total == 0:
            return 0.0
        return self._correct / self._total

    def reset_state(self) -> None:
        self._correct = 0
        self._total = 0


class BinaryAccuracy(Metric):
    """Binary accuracy with a configurable threshold.

    Predictions above the threshold are treated as class 1,
    below as class 0.

    Args:
        threshold: Decision boundary. Default: 0.5.
    """

    def __init__(self, threshold: float = 0.5) -> None:
        super().__init__(name="binary_accuracy")
        self.threshold = threshold
        self._correct = 0
        self._total = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        for i in range(len(y_true.data)):
            pred_class = 1.0 if y_pred.data[i] >= self.threshold else 0.0
            if pred_class == round(y_true.data[i]):
                self._correct += 1
            self._total += 1

    def result(self) -> float:
        if self._total == 0:
            return 0.0
        return self._correct / self._total

    def reset_state(self) -> None:
        self._correct = 0
        self._total = 0


class CategoricalAccuracy(Metric):
    """Accuracy for one-hot encoded targets.

    Both y_true and y_pred are 2-D: (batch, num_classes).
    Compares argmax of each.
    """

    def __init__(self) -> None:
        super().__init__(name="categorical_accuracy")
        self._correct = 0
        self._total = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        if y_true.ndim != 2 or y_pred.ndim != 2:
            raise ValueError("CategoricalAccuracy expects 2-D tensors")

        batch_size, num_classes = y_pred.shape
        for i in range(batch_size):
            pred_row = y_pred.data[i * num_classes : (i + 1) * num_classes]
            true_row = y_true.data[i * num_classes : (i + 1) * num_classes]
            if pred_row.index(max(pred_row)) == true_row.index(max(true_row)):
                self._correct += 1
            self._total += 1

    def result(self) -> float:
        if self._total == 0:
            return 0.0
        return self._correct / self._total

    def reset_state(self) -> None:
        self._correct = 0
        self._total = 0


class MeanSquaredError(Metric):
    """MSE metric: tracks running average of (y_true - y_pred)^2."""

    def __init__(self) -> None:
        super().__init__(name="mean_squared_error")
        self._sum = 0.0
        self._count = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        for t, p in zip(y_true.data, y_pred.data):
            self._sum += (t - p) ** 2
            self._count += 1

    def result(self) -> float:
        if self._count == 0:
            return 0.0
        return self._sum / self._count

    def reset_state(self) -> None:
        self._sum = 0.0
        self._count = 0


class MeanAbsoluteError(Metric):
    """MAE metric: tracks running average of |y_true - y_pred|."""

    def __init__(self) -> None:
        super().__init__(name="mean_absolute_error")
        self._sum = 0.0
        self._count = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        for t, p in zip(y_true.data, y_pred.data):
            self._sum += _builtin_abs(t - p)
            self._count += 1

    def result(self) -> float:
        if self._count == 0:
            return 0.0
        return self._sum / self._count

    def reset_state(self) -> None:
        self._sum = 0.0
        self._count = 0


# =========================================================================
# String-to-metric lookup
# =========================================================================

_METRIC_MAP: dict[str, type] = {
    "accuracy": Accuracy,
    "binary_accuracy": BinaryAccuracy,
    "categorical_accuracy": CategoricalAccuracy,
    "mse": MeanSquaredError,
    "mean_squared_error": MeanSquaredError,
    "mae": MeanAbsoluteError,
    "mean_absolute_error": MeanAbsoluteError,
}


def get(identifier: str | object) -> Metric:
    """Look up a metric by string name or return it as-is."""
    if isinstance(identifier, str):
        if identifier in _METRIC_MAP:
            return _METRIC_MAP[identifier]()
        raise ValueError(
            f"Unknown metric: '{identifier}'. Available: {list(_METRIC_MAP.keys())}"
        )
    return identifier
