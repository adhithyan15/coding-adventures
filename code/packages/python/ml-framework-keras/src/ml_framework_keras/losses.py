"""
================================================================
LOSSES — FUNCTIONS THAT MEASURE HOW WRONG THE MODEL IS
================================================================

A loss function takes two inputs:
1. y_true: the correct answer (ground truth)
2. y_pred: the model's prediction

And returns a single scalar number — the "loss" — that measures how
far off the prediction is. The goal of training is to MINIMIZE this
number by adjusting the model's weights.

=== Common Losses and When to Use Them ===

| Loss                          | Task                    | Output activation |
|-------------------------------|-------------------------|-------------------|
| MeanSquaredError (MSE)        | Regression              | Linear (None)     |
| MeanAbsoluteError (MAE)       | Regression (robust)     | Linear (None)     |
| BinaryCrossentropy            | Binary classification   | Sigmoid           |
| CategoricalCrossentropy       | Multi-class (one-hot)   | Softmax           |
| SparseCategoricalCrossentropy | Multi-class (integer)   | Softmax           |

=== The from_logits Parameter ===

Cross-entropy losses have a `from_logits` option:
- from_logits=False (default): y_pred is probabilities (after sigmoid/softmax)
- from_logits=True: y_pred is raw logits (before sigmoid/softmax)

Using from_logits=True is more numerically stable because the loss
function can combine the softmax and log operations, avoiding the
log(exp(x)) → x simplification that floating point misses.

=== String-Based Lookup ===

    model.compile(loss="mse")                    # string form
    model.compile(loss=MeanSquaredError())       # instance form

================================================================
"""

from __future__ import annotations

import math
from typing import Any

from ml_framework_core import Tensor


# =========================================================================
# Base Loss
# =========================================================================


class Loss:
    """Base class for all loss functions.

    Subclasses implement __call__(y_true, y_pred) → scalar Tensor.
    The returned tensor participates in autograd so gradients
    flow back through the model.
    """

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        """Compute the loss.

        Args:
            y_true: Ground truth values.
            y_pred: Model predictions.

        Returns:
            Scalar Tensor containing the loss value.
        """
        raise NotImplementedError

    def get_config(self) -> dict[str, Any]:
        return {"class_name": self.__class__.__name__}


# =========================================================================
# Mean Squared Error
# =========================================================================


class MeanSquaredError(Loss):
    """MSE loss: mean((y_true - y_pred)^2).

    The standard loss for regression tasks. Penalizes large errors
    quadratically — an error of 2 is 4x worse than an error of 1.

    Gradient: ∂MSE/∂y_pred = -2 * (y_true - y_pred) / n

    This quadratic penalty means MSE is sensitive to outliers.
    For robust regression, consider MeanAbsoluteError instead.
    """

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        diff = y_pred - y_true
        squared = diff * diff
        return squared.mean()


# =========================================================================
# Mean Absolute Error
# =========================================================================


class MeanAbsoluteError(Loss):
    """MAE loss: mean(|y_true - y_pred|).

    More robust to outliers than MSE because it penalizes errors
    linearly instead of quadratically. An error of 10 is only 10x
    worse than an error of 1 (vs 100x for MSE).

    Downside: the gradient is ±1 everywhere (except at 0 where
    it's undefined), so it doesn't naturally slow down near the
    optimum. MSE's gradient shrinks near zero, providing a natural
    "braking" effect.
    """

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        diff = y_pred - y_true
        abs_diff = diff.abs()
        return abs_diff.mean()


# =========================================================================
# Binary Cross-Entropy
# =========================================================================


class BinaryCrossentropy(Loss):
    """Binary cross-entropy: -mean(y*log(p) + (1-y)*log(1-p)).

    The standard loss for binary classification (yes/no, spam/not-spam).
    y_true should be 0 or 1, y_pred should be probabilities in (0, 1).

    === Intuition ===

    Cross-entropy measures how "surprised" we are by the prediction:
    - If y_true=1 and y_pred=0.99 → low loss (not surprised)
    - If y_true=1 and y_pred=0.01 → high loss (very surprised!)

    The log() function makes this work: log(0.99) ≈ 0 (small loss),
    but log(0.01) ≈ -4.6 (large loss). The further from correct,
    the exponentially larger the penalty.

    Args:
        from_logits: If True, y_pred is raw logits (pre-sigmoid).
            Default: False (y_pred is probabilities).
    """

    def __init__(self, from_logits: bool = False) -> None:
        self.from_logits = from_logits

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        if self.from_logits:
            # Apply sigmoid to convert logits → probabilities
            from ml_framework_core import SigmoidFunction

            y_pred = SigmoidFunction.apply(y_pred)

        # Clamp predictions to avoid log(0)
        eps = 1e-7
        y_pred = y_pred.clamp(eps, 1.0 - eps)

        # -[y * log(p) + (1 - y) * log(1 - p)]
        term1 = y_true * y_pred.log()
        term2 = (y_true * (-1.0) + 1.0) * (y_pred * (-1.0) + 1.0).log()
        loss = (term1 + term2) * (-1.0)
        return loss.mean()

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config["from_logits"] = self.from_logits
        return config


# =========================================================================
# Categorical Cross-Entropy
# =========================================================================


class CategoricalCrossentropy(Loss):
    """Categorical cross-entropy for one-hot encoded labels.

    For multi-class classification where y_true is one-hot:
        y_true = [0, 0, 1, 0, 0]   (class 2)
        y_pred = [0.1, 0.1, 0.6, 0.1, 0.1]  (model's probabilities)

    Loss = -sum(y_true * log(y_pred))
         = -log(0.6) = 0.51

    Only the predicted probability for the TRUE class matters —
    all other terms are zeroed out by the one-hot y_true.

    Args:
        from_logits: If True, y_pred is raw logits (pre-softmax).
    """

    def __init__(self, from_logits: bool = False) -> None:
        self.from_logits = from_logits

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        if self.from_logits:
            from ml_framework_core import SoftmaxFunction

            y_pred = SoftmaxFunction.apply(y_pred, -1)

        eps = 1e-7
        y_pred = y_pred.clamp(eps, 1.0 - eps)

        # -sum(y_true * log(y_pred)) averaged over batch
        log_pred = y_pred.log()
        elementwise = y_true * log_pred
        # Sum across classes (last dim), then mean across batch
        batch_losses = elementwise.sum(dim=-1)
        return batch_losses.mean() * (-1.0)

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config["from_logits"] = self.from_logits
        return config


# =========================================================================
# Sparse Categorical Cross-Entropy
# =========================================================================


class SparseCategoricalCrossentropy(Loss):
    """Sparse categorical cross-entropy for integer labels.

    Same as CategoricalCrossentropy, but y_true contains integer
    class indices instead of one-hot vectors:

        y_true = [2, 0, 5]           (class indices for 3 samples)
        y_pred = [[0.1, 0.1, 0.8],   (probabilities for 3 classes)
                  [0.7, 0.2, 0.1],
                  [0.1, 0.1, 0.8]]

    This is more memory-efficient than one-hot encoding when you
    have many classes (e.g., 10000 word vocabulary).

    Args:
        from_logits: If True, y_pred is raw logits (pre-softmax).
    """

    def __init__(self, from_logits: bool = False) -> None:
        self.from_logits = from_logits

    def __call__(self, y_true: Tensor, y_pred: Tensor) -> Tensor:
        if self.from_logits:
            from ml_framework_core import SoftmaxFunction

            y_pred = SoftmaxFunction.apply(y_pred, -1)

        eps = 1e-7
        y_pred_clamped = y_pred.clamp(eps, 1.0 - eps)

        batch_size = y_pred.shape[0]
        num_classes = y_pred.shape[-1]

        # For each sample, pick the predicted probability of the true class
        # and compute -log(p_true)
        total_loss = 0.0
        for i in range(batch_size):
            true_class = int(y_true.data[i])
            pred_prob = y_pred_clamped.data[i * num_classes + true_class]
            total_loss += -math.log(pred_prob)

        avg_loss = total_loss / batch_size
        return Tensor([avg_loss], (1,), requires_grad=y_pred.requires_grad)

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config["from_logits"] = self.from_logits
        return config


# =========================================================================
# String-to-instance lookup
# =========================================================================

_LOSS_REGISTRY: dict[str, type[Loss]] = {
    "mse": MeanSquaredError,
    "mean_squared_error": MeanSquaredError,
    "mae": MeanAbsoluteError,
    "mean_absolute_error": MeanAbsoluteError,
    "binary_crossentropy": BinaryCrossentropy,
    "categorical_crossentropy": CategoricalCrossentropy,
    "sparse_categorical_crossentropy": SparseCategoricalCrossentropy,
}


def get_loss(identifier: str | Loss) -> Loss:
    """Convert a loss identifier to a Loss instance.

    Args:
        identifier: String name or Loss instance.

    Returns:
        A Loss instance.

    Raises:
        ValueError: If the string name is not recognized.
    """
    if isinstance(identifier, Loss):
        return identifier

    if isinstance(identifier, str):
        key = identifier.lower()
        if key not in _LOSS_REGISTRY:
            raise ValueError(
                f"Unknown loss '{identifier}'. "
                f"Available: {sorted(_LOSS_REGISTRY.keys())}"
            )
        return _LOSS_REGISTRY[key]()

    raise TypeError(f"Loss must be a string or Loss instance. Got {type(identifier)}")
