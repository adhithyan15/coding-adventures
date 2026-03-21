"""
================================================================
METRICS — TRACK MODEL PERFORMANCE DURING TRAINING
================================================================

Metrics are similar to losses but serve a different purpose:
- Losses are what the optimizer minimizes (must be differentiable)
- Metrics are what humans monitor (can be anything meaningful)

For example, cross-entropy loss is hard to interpret, but accuracy
("what % of predictions are correct?") is easy to understand.

=== The Metric Protocol ===

Every metric follows a stateful update protocol:

    metric = Accuracy()
    metric.update_state(y_true, y_pred)   # accumulate results
    metric.update_state(y_true2, y_pred2) # more results
    value = metric.result()                # compute the metric
    metric.reset_state()                   # start fresh for next epoch

This stateful design lets metrics aggregate over multiple batches
before computing the final value. For accuracy, it tracks total
correct predictions and total predictions, then divides at the end.

=== String-Based Lookup ===

    model.compile(metrics=["accuracy"])              # string form
    model.compile(metrics=[Accuracy()])              # instance form
    model.compile(metrics=["accuracy", "mse"])       # multiple metrics

================================================================
"""

from __future__ import annotations

from typing import Any

from ml_framework_core import Tensor


# =========================================================================
# Base Metric
# =========================================================================


class Metric:
    """Base class for all metrics.

    Subclasses must implement:
    - update_state(y_true, y_pred): accumulate batch results
    - result() → float: compute the metric value
    - reset_state(): clear accumulated state
    """

    def __init__(self, name: str | None = None) -> None:
        self.name = name or self.__class__.__name__.lower()

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        """Accumulate metric state from one batch."""
        raise NotImplementedError

    def result(self) -> float:
        """Compute the current metric value."""
        raise NotImplementedError

    def reset_state(self) -> None:
        """Reset accumulated state for a new epoch."""
        raise NotImplementedError

    def get_config(self) -> dict[str, Any]:
        return {"class_name": self.__class__.__name__, "name": self.name}


# =========================================================================
# Accuracy — Fraction of Correct Predictions
# =========================================================================


class Accuracy(Metric):
    """Simple accuracy: fraction of predictions that exactly match labels.

    Works for both binary and multi-class classification:
    - Binary: y_true and y_pred are single values (0 or 1)
    - Multi-class: y_true and y_pred are class indices

    For probability outputs, predictions are thresholded at 0.5
    (binary) or argmax'd (multi-class).

    This is a "generic" accuracy — for specific variants, see
    BinaryAccuracy and CategoricalAccuracy.
    """

    def __init__(self, name: str | None = None) -> None:
        super().__init__(name or "accuracy")
        self._correct = 0
        self._total = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        """Count correct predictions.

        If y_pred has multiple columns (multi-class), use argmax.
        If y_pred is 1-D or has 1 column (binary), threshold at 0.5.
        """
        if y_pred.ndim == 2 and y_pred.shape[-1] > 1:
            # Multi-class: compare argmax of predictions to true labels
            num_classes = y_pred.shape[-1]
            batch_size = y_pred.shape[0]
            for i in range(batch_size):
                start = i * num_classes
                end = start + num_classes
                row = y_pred.data[start:end]
                pred_class = row.index(max(row))

                # y_true might be one-hot or integer
                if y_true.ndim == 2 and y_true.shape[-1] > 1:
                    true_row = y_true.data[i * num_classes : (i + 1) * num_classes]
                    true_class = true_row.index(max(true_row))
                else:
                    true_class = int(y_true.data[i])

                if pred_class == true_class:
                    self._correct += 1
                self._total += 1
        else:
            # Binary or flat: threshold at 0.5
            for yt, yp in zip(y_true.data, y_pred.data):
                pred = 1.0 if yp >= 0.5 else 0.0
                if pred == yt:
                    self._correct += 1
                self._total += 1

    def result(self) -> float:
        if self._total == 0:
            return 0.0
        return self._correct / self._total

    def reset_state(self) -> None:
        self._correct = 0
        self._total = 0


# =========================================================================
# BinaryAccuracy
# =========================================================================


class BinaryAccuracy(Metric):
    """Accuracy for binary classification with probability outputs.

    Predictions above `threshold` (default 0.5) are classified as 1,
    otherwise as 0. Then compared to y_true.

    Args:
        threshold: Classification threshold. Default: 0.5.
    """

    def __init__(self, threshold: float = 0.5, name: str | None = None) -> None:
        super().__init__(name or "binary_accuracy")
        self.threshold = threshold
        self._correct = 0
        self._total = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        for yt, yp in zip(y_true.data, y_pred.data):
            pred = 1.0 if yp >= self.threshold else 0.0
            if pred == yt:
                self._correct += 1
            self._total += 1

    def result(self) -> float:
        if self._total == 0:
            return 0.0
        return self._correct / self._total

    def reset_state(self) -> None:
        self._correct = 0
        self._total = 0


# =========================================================================
# CategoricalAccuracy
# =========================================================================


class CategoricalAccuracy(Metric):
    """Accuracy for multi-class classification with one-hot labels.

    Compares argmax(y_pred) to argmax(y_true) for each sample.
    Both y_true and y_pred should have shape (batch_size, num_classes).
    """

    def __init__(self, name: str | None = None) -> None:
        super().__init__(name or "categorical_accuracy")
        self._correct = 0
        self._total = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        num_classes = y_pred.shape[-1]
        batch_size = y_pred.shape[0]

        for i in range(batch_size):
            start = i * num_classes
            end = start + num_classes

            pred_row = y_pred.data[start:end]
            true_row = y_true.data[start:end]

            pred_class = pred_row.index(max(pred_row))
            true_class = true_row.index(max(true_row))

            if pred_class == true_class:
                self._correct += 1
            self._total += 1

    def result(self) -> float:
        if self._total == 0:
            return 0.0
        return self._correct / self._total

    def reset_state(self) -> None:
        self._correct = 0
        self._total = 0


# =========================================================================
# MeanSquaredError Metric
# =========================================================================


class MeanSquaredError(Metric):
    """MSE as a metric (not a loss).

    Tracks mean((y_true - y_pred)^2) across batches.
    Unlike the loss version, this doesn't participate in autograd.
    """

    def __init__(self, name: str | None = None) -> None:
        super().__init__(name or "mean_squared_error")
        self._sum = 0.0
        self._count = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        for yt, yp in zip(y_true.data, y_pred.data):
            self._sum += (yt - yp) ** 2
            self._count += 1

    def result(self) -> float:
        if self._count == 0:
            return 0.0
        return self._sum / self._count

    def reset_state(self) -> None:
        self._sum = 0.0
        self._count = 0


# =========================================================================
# MeanAbsoluteError Metric
# =========================================================================


class MeanAbsoluteError(Metric):
    """MAE as a metric.

    Tracks mean(|y_true - y_pred|) across batches.
    """

    def __init__(self, name: str | None = None) -> None:
        super().__init__(name or "mean_absolute_error")
        self._sum = 0.0
        self._count = 0

    def update_state(self, y_true: Tensor, y_pred: Tensor) -> None:
        for yt, yp in zip(y_true.data, y_pred.data):
            self._sum += abs(yt - yp)
            self._count += 1

    def result(self) -> float:
        if self._count == 0:
            return 0.0
        return self._sum / self._count

    def reset_state(self) -> None:
        self._sum = 0.0
        self._count = 0


# =========================================================================
# String-to-instance lookup
# =========================================================================

_METRIC_REGISTRY: dict[str, type[Metric]] = {
    "accuracy": Accuracy,
    "binary_accuracy": BinaryAccuracy,
    "categorical_accuracy": CategoricalAccuracy,
    "mse": MeanSquaredError,
    "mean_squared_error": MeanSquaredError,
    "mae": MeanAbsoluteError,
    "mean_absolute_error": MeanAbsoluteError,
}


def get_metric(identifier: str | Metric) -> Metric:
    """Convert a metric identifier to a Metric instance.

    Args:
        identifier: String name or Metric instance.

    Returns:
        A Metric instance.

    Raises:
        ValueError: If the string name is not recognized.
    """
    if isinstance(identifier, Metric):
        return identifier

    if isinstance(identifier, str):
        key = identifier.lower()
        if key not in _METRIC_REGISTRY:
            raise ValueError(
                f"Unknown metric '{identifier}'. "
                f"Available: {sorted(_METRIC_REGISTRY.keys())}"
            )
        return _METRIC_REGISTRY[key]()

    raise TypeError(
        f"Metric must be a string or Metric instance. Got {type(identifier)}"
    )
