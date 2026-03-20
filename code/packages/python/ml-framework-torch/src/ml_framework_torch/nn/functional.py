"""
================================================================
FUNCTIONAL API — STATELESS OPERATIONS (torch.nn.functional)
================================================================

In PyTorch, there are two ways to apply operations:

1. Module-based (stateful): torch.nn.ReLU() — an object you call
2. Functional (stateless): torch.nn.functional.relu(x) — a function

The functional API is useful when:
- You don't need learnable parameters (activations)
- You want more control in custom forward() methods
- You're writing loss computation inline

Usage:
    import ml_framework_torch.nn.functional as F

    # Activations
    y = F.relu(x)
    y = F.gelu(x)
    y = F.sigmoid(x)
    y = F.softmax(x, dim=-1)

    # Linear
    y = F.linear(x, weight, bias)

    # Loss
    loss = F.mse_loss(pred, target)
    loss = F.cross_entropy(logits, labels)

================================================================
"""

from __future__ import annotations

import math

from ml_framework_core import Tensor
from ml_framework_core import (
    GELUFunction,
    ReLUFunction,
    SigmoidFunction,
    SoftmaxFunction,
    TanhFunction,
)


# =====================================================================
# Activation functions
# =====================================================================


def relu(x: Tensor) -> Tensor:
    """Apply ReLU: max(0, x) element-wise.

    Example:
        y = F.relu(Tensor.from_list([-1.0, 0.0, 1.0, 2.0]))
        # [0.0, 0.0, 1.0, 2.0]
    """
    return ReLUFunction.apply(x)


def gelu(x: Tensor) -> Tensor:
    """Apply GELU activation (used in transformers)."""
    return GELUFunction.apply(x)


def sigmoid(x: Tensor) -> Tensor:
    """Apply sigmoid: 1 / (1 + exp(-x))."""
    return SigmoidFunction.apply(x)


def tanh(x: Tensor) -> Tensor:
    """Apply hyperbolic tangent."""
    return TanhFunction.apply(x)


def softmax(x: Tensor, dim: int = -1) -> Tensor:
    """Apply softmax along the specified dimension.

    Converts logits to probabilities (all non-negative, sum to 1).

    Example:
        probs = F.softmax(Tensor.from_list([1.0, 2.0, 3.0]), dim=0)
        # [0.09, 0.24, 0.67] (approximately)
    """
    return SoftmaxFunction.apply(x, dim)


def log_softmax(x: Tensor, dim: int = -1) -> Tensor:
    """Compute log(softmax(x)) with numerical stability.

    Uses the log-sum-exp trick to avoid computing softmax first
    (which could underflow).
    """
    if x.ndim == 1:
        max_val = max(x.data)
        shifted = [v - max_val for v in x.data]
        log_sum = math.log(sum(math.exp(s) for s in shifted))
        result = [s - log_sum for s in shifted]
        return Tensor(result, x.shape, requires_grad=x.requires_grad, device=x.device)

    actual_dim = dim if dim >= 0 else x.ndim + dim

    if x.ndim == 2 and actual_dim == 1:
        rows, cols = x.shape
        result_data: list[float] = []
        for i in range(rows):
            row_start = i * cols
            row = x.data[row_start : row_start + cols]
            max_val = max(row)
            shifted = [v - max_val for v in row]
            log_sum = math.log(sum(math.exp(s) for s in shifted))
            result_data.extend(s - log_sum for s in shifted)
        return Tensor(
            result_data,
            x.shape,
            requires_grad=x.requires_grad,
            device=x.device,
        )

    # Fallback
    return SoftmaxFunction.apply(x, dim).log()


# =====================================================================
# Linear transformation
# =====================================================================


def linear(
    x: Tensor,
    weight: Tensor,
    bias: Tensor | None = None,
) -> Tensor:
    """Apply a linear transformation: y = x @ W.T + b.

    This is the functional equivalent of nn.Linear:
        y = F.linear(x, layer.weight, layer.bias)

    Args:
        x: Input tensor of shape (batch, in_features)
        weight: Weight matrix of shape (out_features, in_features)
        bias: Optional bias of shape (out_features,)

    Returns:
        Output tensor of shape (batch, out_features)
    """
    output = x @ weight.t()

    if bias is not None:
        batch_size = x.shape[0]
        out_features = bias.shape[0]
        ones_col = Tensor.ones(batch_size, 1)
        bias_row = bias.reshape(1, out_features)
        bias_broadcast = ones_col @ bias_row
        output = output + bias_broadcast

    return output


# =====================================================================
# Loss functions
# =====================================================================


def mse_loss(
    prediction: Tensor,
    target: Tensor,
    reduction: str = "mean",
) -> Tensor:
    """Mean Squared Error loss.

    L = mean((prediction - target)^2)

    Args:
        prediction: Model output
        target: Ground truth
        reduction: "mean", "sum", or "none"
    """
    diff = prediction - target
    squared = diff**2

    if reduction == "mean":
        return squared.mean()
    elif reduction == "sum":
        return squared.sum()
    else:
        return squared


def l1_loss(
    prediction: Tensor,
    target: Tensor,
    reduction: str = "mean",
) -> Tensor:
    """L1 (Mean Absolute Error) loss.

    L = mean(|prediction - target|)
    """
    diff = prediction - target
    abs_diff = diff.abs()

    if reduction == "mean":
        return abs_diff.mean()
    elif reduction == "sum":
        return abs_diff.sum()
    else:
        return abs_diff


def cross_entropy(
    prediction: Tensor,
    target: Tensor,
    reduction: str = "mean",
) -> Tensor:
    """Cross-entropy loss (LogSoftmax + NLLLoss).

    Args:
        prediction: (batch, classes) raw logits
        target: (batch,) integer class labels
        reduction: "mean", "sum", or "none"
    """
    from .loss import CrossEntropyLoss

    loss_fn = CrossEntropyLoss(reduction=reduction)
    return loss_fn(prediction, target)


def binary_cross_entropy(
    prediction: Tensor,
    target: Tensor,
    reduction: str = "mean",
) -> Tensor:
    """Binary cross-entropy loss.

    prediction must be probabilities in (0, 1).
    """
    from .loss import BCELoss

    loss_fn = BCELoss(reduction=reduction)
    return loss_fn(prediction, target)


def nll_loss(
    prediction: Tensor,
    target: Tensor,
    reduction: str = "mean",
) -> Tensor:
    """Negative log-likelihood loss.

    prediction should be log-probabilities (from LogSoftmax).
    """
    from .loss import NLLLoss

    loss_fn = NLLLoss(reduction=reduction)
    return loss_fn(prediction, target)
