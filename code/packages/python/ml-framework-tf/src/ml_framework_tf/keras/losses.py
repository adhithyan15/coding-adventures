"""
================================================================
TF.KERAS.LOSSES — LOSS FUNCTIONS FOR MODEL TRAINING
================================================================

Loss functions measure how far the model's predictions are from
the ground truth. The optimizer's job is to minimize this loss.

=== Keras Loss API ===

In Keras, losses are callable objects:
    loss_fn = MeanSquaredError()
    loss = loss_fn(y_true, y_pred)

Note the argument order: (y_true, y_pred) — TensorFlow puts the
ground truth FIRST, while PyTorch puts predictions first.
This is a common source of bugs when switching frameworks!

    TF:      loss_fn(y_true, y_pred)     # true first
    PyTorch: loss_fn(prediction, target)  # pred first

=== Loss Function Overview ===

| Loss                       | Use Case                  | Output Layer    |
|----------------------------|---------------------------|-----------------|
| MeanSquaredError           | Regression                | Linear          |
| MeanAbsoluteError          | Robust regression         | Linear          |
| BinaryCrossentropy         | Binary classification     | Sigmoid         |
| CategoricalCrossentropy    | Multi-class (one-hot)     | Softmax         |
| SparseCategoricalCrossent. | Multi-class (int labels)  | Softmax         |

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor
from ml_framework_core import SoftmaxFunction


class MeanSquaredError:
    """MSE Loss: mean((y_true - y_pred)^2).

    The standard loss for regression tasks. Penalizes large errors
    quadratically, making it sensitive to outliers.

    Example:
        loss_fn = MeanSquaredError()
        loss = loss_fn(y_true, y_pred)  # scalar tensor
    """

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        diff = y_pred - y_true
        squared = diff**2
        return squared.mean()

    def __repr__(self) -> str:
        return "MeanSquaredError()"


class MeanAbsoluteError:
    """MAE Loss: mean(|y_true - y_pred|).

    More robust to outliers than MSE because large errors are
    penalized linearly, not quadratically.

    Example:
        loss_fn = MeanAbsoluteError()
        loss = loss_fn(y_true, y_pred)
    """

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        diff = y_pred - y_true
        return diff.abs().mean()

    def __repr__(self) -> str:
        return "MeanAbsoluteError()"


class BinaryCrossentropy:
    """Binary cross-entropy for binary classification.

    Formula: -mean(y_true * log(y_pred) + (1-y_true) * log(1-y_pred))

    Predictions should be probabilities in (0, 1).
    Use from_logits=True if predictions are raw logits.

    Args:
        from_logits: If True, apply sigmoid internally. Default: False.

    Example:
        loss_fn = BinaryCrossentropy()
        loss = loss_fn(y_true, y_pred)  # y_pred should be probabilities
    """

    def __init__(self, from_logits: bool = False) -> None:
        self.from_logits = from_logits

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        if self.from_logits:
            from ml_framework_core import SigmoidFunction

            y_pred = SigmoidFunction.apply(y_pred)

        eps = 1e-7
        pred_clamped = y_pred.clamp(eps, 1.0 - eps)
        log_pred = pred_clamped.log()
        log_one_minus = (1.0 - pred_clamped).log()
        loss = -(y_true * log_pred + (1.0 - y_true) * log_one_minus)
        return loss.mean()

    def __repr__(self) -> str:
        return f"BinaryCrossentropy(from_logits={self.from_logits})"


class CategoricalCrossentropy:
    """Categorical cross-entropy for multi-class classification.

    Expects y_true as one-hot encoded vectors:
        y_true = [[1, 0, 0], [0, 1, 0], [0, 0, 1]]

    Formula: -mean(sum(y_true * log(y_pred), axis=-1))

    Args:
        from_logits: If True, apply softmax internally. Default: False.

    Example:
        loss_fn = CategoricalCrossentropy()
        loss = loss_fn(y_true_onehot, y_pred_probs)
    """

    def __init__(self, from_logits: bool = False) -> None:
        self.from_logits = from_logits

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        if self.from_logits:
            y_pred = SoftmaxFunction.apply(y_pred, -1)

        eps = 1e-7
        pred_clamped = y_pred.clamp(eps, 1.0 - eps)
        log_pred = pred_clamped.log()

        # -sum(y_true * log(y_pred)) per sample, then mean
        elementwise = y_true * log_pred
        neg = -elementwise
        # Sum over classes (dim=1), then mean over batch
        per_sample = neg.sum(dim=1)
        return per_sample.mean()

    def __repr__(self) -> str:
        return f"CategoricalCrossentropy(from_logits={self.from_logits})"


class SparseCategoricalCrossentropy:
    """Sparse categorical cross-entropy — integer labels instead of one-hot.

    Same as CategoricalCrossentropy but y_true contains integer class
    indices instead of one-hot vectors. More memory efficient.

    y_true = [0, 1, 2]  (class indices)
    instead of:
    y_true = [[1,0,0], [0,1,0], [0,0,1]]  (one-hot)

    Args:
        from_logits: If True, apply softmax internally. Default: False.

    Example:
        loss_fn = SparseCategoricalCrossentropy(from_logits=True)
        loss = loss_fn(y_true_ints, logits)
    """

    def __init__(self, from_logits: bool = False) -> None:
        self.from_logits = from_logits

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        if y_pred.ndim != 2:
            raise ValueError(
                f"y_pred must be 2-D (batch, classes), got {y_pred.ndim}-D"
            )

        batch_size, num_classes = y_pred.shape

        if self.from_logits:
            y_pred = SoftmaxFunction.apply(y_pred, -1)

        eps = 1e-7
        pred_clamped = y_pred.clamp(eps, 1.0 - eps)
        log_pred = pred_clamped.log()

        # Build one-hot from integer labels
        one_hot_data = [0.0] * (batch_size * num_classes)
        for i in range(batch_size):
            class_idx = int(y_true.data[i])
            one_hot_data[i * num_classes + class_idx] = 1.0

        one_hot = Tensor(one_hot_data, y_pred.shape, device=y_pred.device)
        elementwise = one_hot * log_pred
        neg = -elementwise
        total = neg.sum()
        return total / float(batch_size)

    def __repr__(self) -> str:
        return f"SparseCategoricalCrossentropy(from_logits={self.from_logits})"


# =========================================================================
# String-to-loss lookup (used by model.compile)
# =========================================================================

_LOSS_MAP: dict[str, type] = {
    "mse": MeanSquaredError,
    "mean_squared_error": MeanSquaredError,
    "mae": MeanAbsoluteError,
    "mean_absolute_error": MeanAbsoluteError,
    "binary_crossentropy": BinaryCrossentropy,
    "categorical_crossentropy": CategoricalCrossentropy,
    "sparse_categorical_crossentropy": SparseCategoricalCrossentropy,
}


def get(identifier: str | object) -> object:
    """Look up a loss by string name or return it as-is.

    Used by model.compile():
        model.compile(loss='mse')  # looks up MeanSquaredError
    """
    if isinstance(identifier, str):
        if identifier in _LOSS_MAP:
            return _LOSS_MAP[identifier]()
        raise ValueError(
            f"Unknown loss: '{identifier}'. Available: {list(_LOSS_MAP.keys())}"
        )
    return identifier
