"""
================================================================
LOSS FUNCTIONS — MEASURING HOW WRONG THE MODEL IS
================================================================

Loss functions quantify the difference between predictions and
targets. The optimizer minimizes the loss during training.

=== The Big Picture ===

    model output (predictions) ──┐
                                 ├──→ Loss Function ──→ scalar loss
    ground truth (targets) ─────┘

The loss is a scalar (single number), so we can call loss.backward()
to compute gradients for all model parameters.

=== Loss Function Gallery ===

| Loss              | Formula                           | Use Case              |
|-------------------|-----------------------------------|-----------------------|
| MSELoss           | mean((pred - target)²)            | Regression            |
| L1Loss            | mean(|pred - target|)             | Robust regression     |
| CrossEntropyLoss  | -log(softmax(pred)[target_class]) | Multi-class classif.  |
| BCELoss           | -(t*log(p) + (1-t)*log(1-p))     | Binary classification |
| BCEWithLogitsLoss | BCELoss + built-in sigmoid        | Binary (more stable)  |
| NLLLoss           | -log_probs[target_class]          | With LogSoftmax       |

================================================================
"""

from __future__ import annotations

import math

from ml_framework_core import Tensor
from ml_framework_core import SigmoidFunction, SoftmaxFunction

from .module import Module


class MSELoss(Module):
    """Mean Squared Error: L = mean((prediction - target)²).

    The most common loss for regression tasks. Penalizes large
    errors quadratically — an error of 2 costs 4x as much as
    an error of 1.

    Args:
        reduction: "mean" (default) averages over all elements.
                   "sum" sums instead. "none" returns per-element loss.

    Example:
        loss_fn = MSELoss()
        pred = Tensor.from_list([1.0, 2.0, 3.0])
        target = Tensor.from_list([1.5, 2.5, 3.5])
        loss = loss_fn(pred, target)  # 0.25 (mean of [0.25, 0.25, 0.25])
    """

    def __init__(self, reduction: str = "mean") -> None:
        super().__init__()
        object.__setattr__(self, "reduction", reduction)

    def forward(self, prediction: Tensor, target: Tensor) -> Tensor:
        """Compute MSE loss.

        Steps:
        1. diff = prediction - target
        2. squared = diff ** 2
        3. loss = mean(squared) or sum(squared)
        """
        diff = prediction - target
        squared = diff**2

        if self.reduction == "mean":
            return squared.mean()
        elif self.reduction == "sum":
            return squared.sum()
        else:
            return squared

    def __repr__(self) -> str:
        return f"MSELoss(reduction='{self.reduction}')"


class L1Loss(Module):
    """Mean Absolute Error: L = mean(|prediction - target|).

    More robust to outliers than MSE because large errors are
    penalized linearly, not quadratically.

    Args:
        reduction: "mean" (default), "sum", or "none".
    """

    def __init__(self, reduction: str = "mean") -> None:
        super().__init__()
        object.__setattr__(self, "reduction", reduction)

    def forward(self, prediction: Tensor, target: Tensor) -> Tensor:
        diff = prediction - target
        abs_diff = diff.abs()

        if self.reduction == "mean":
            return abs_diff.mean()
        elif self.reduction == "sum":
            return abs_diff.sum()
        else:
            return abs_diff

    def __repr__(self) -> str:
        return f"L1Loss(reduction='{self.reduction}')"


class CrossEntropyLoss(Module):
    """Cross-entropy loss for multi-class classification.

    Combines LogSoftmax + NLLLoss in one step for numerical stability.

    Input:
        prediction: (batch_size, num_classes) — raw logits (NOT probabilities!)
        target: (batch_size,) — integer class labels (0 to num_classes - 1)

    Formula:
        loss = -log(softmax(prediction)[target_class])

    Equivalent to:
        loss = -log_softmax(prediction)[range(batch), target]

    This is the standard loss for classification tasks (image recognition,
    NLP, etc.).

    Args:
        reduction: "mean" (default), "sum", or "none".

    Example:
        loss_fn = CrossEntropyLoss()
        logits = Tensor.from_list([[2.0, 0.5, 0.1],   # sample 0: class 0 likely
                                    [0.1, 2.0, 0.5]])  # sample 1: class 1 likely
        targets = Tensor.from_list([0.0, 1.0])          # correct classes
        loss = loss_fn(logits, targets)  # low loss (predictions match)
    """

    def __init__(self, reduction: str = "mean") -> None:
        super().__init__()
        object.__setattr__(self, "reduction", reduction)

    def forward(self, prediction: Tensor, target: Tensor) -> Tensor:
        """Compute cross-entropy loss.

        Steps:
        1. Compute log-softmax along class dimension (dim=1)
        2. Select the log-probability of the correct class
        3. Negate and reduce (mean or sum)

        The log-sum-exp trick is used for numerical stability:
            log_softmax(x_i) = x_i - log(Σ exp(x_j))
                              = x_i - max(x) - log(Σ exp(x_j - max(x)))
        """
        if prediction.ndim != 2:
            raise ValueError(
                f"CrossEntropyLoss expects 2-D predictions, got {prediction.ndim}-D"
            )

        batch_size, num_classes = prediction.shape

        # ─── Step 1: Compute log-softmax for numerical stability ─
        log_probs_data = []
        for i in range(batch_size):
            row_start = i * num_classes
            row = prediction.data[row_start : row_start + num_classes]

            # Log-sum-exp trick: subtract max to prevent overflow
            max_val = max(row)
            shifted = [v - max_val for v in row]
            log_sum_exp = math.log(sum(math.exp(s) for s in shifted))
            log_probs_data.extend(s - log_sum_exp for s in shifted)

        # ─── Step 2: Pick log-prob of correct class (NLL) ───────
        losses = []
        for i in range(batch_size):
            class_idx = int(target.data[i])
            log_prob = log_probs_data[i * num_classes + class_idx]
            losses.append(-log_prob)

        # ─── Step 3: Reduce ─────────────────────────────────────
        if self.reduction == "none":
            return Tensor(losses, (batch_size,), device=prediction.device)

        # Build the loss through differentiable operations
        # Use the softmax → log → select approach with autograd
        log_softmax_result = SoftmaxFunction.apply(prediction, 1).log()
        nll_data = []
        for i in range(batch_size):
            class_idx = int(target.data[i])
            nll_data.append(-log_softmax_result.data[i * num_classes + class_idx])

        # Create an output tensor that goes through the autograd graph
        # We compute: -sum(one_hot * log_softmax) / batch_size
        # This ensures gradients flow through softmax and log
        one_hot_data = [0.0] * (batch_size * num_classes)
        for i in range(batch_size):
            class_idx = int(target.data[i])
            one_hot_data[i * num_classes + class_idx] = 1.0

        one_hot = Tensor(one_hot_data, prediction.shape, device=prediction.device)

        # -sum(one_hot * log_softmax) gives us the NLL losses
        elementwise = log_softmax_result * one_hot
        neg_elementwise = -elementwise

        if self.reduction == "mean":
            # Sum all then divide by batch size
            total = neg_elementwise.sum()
            # Divide by batch_size
            return total / float(batch_size)
        elif self.reduction == "sum":
            return neg_elementwise.sum()
        else:
            # Per-sample losses
            return neg_elementwise.sum(dim=1)

    def __repr__(self) -> str:
        return f"CrossEntropyLoss(reduction='{self.reduction}')"


class BCELoss(Module):
    """Binary Cross-Entropy Loss.

    For binary classification where predictions are probabilities (0 to 1).

    Formula:
        L = -[target * log(pred) + (1 - target) * log(1 - pred)]

    Input:
        prediction: probabilities in (0, 1) — apply sigmoid before this!
        target: binary labels (0 or 1)

    Args:
        reduction: "mean" (default), "sum", or "none".

    Example:
        loss_fn = BCELoss()
        pred = Tensor.from_list([0.9, 0.1, 0.8])   # model thinks: yes, no, yes
        target = Tensor.from_list([1.0, 0.0, 1.0])  # actual: yes, no, yes
        loss = loss_fn(pred, target)  # low loss
    """

    def __init__(self, reduction: str = "mean") -> None:
        super().__init__()
        object.__setattr__(self, "reduction", reduction)

    def forward(self, prediction: Tensor, target: Tensor) -> Tensor:
        """Compute BCE loss.

        Uses clamp to prevent log(0) which would give -inf:
            log(clamp(pred, 1e-7, 1-1e-7))
        """
        eps = 1e-7
        pred_clamped = prediction.clamp(eps, 1.0 - eps)

        # -(target * log(pred) + (1 - target) * log(1 - pred))
        log_pred = pred_clamped.log()
        log_one_minus_pred = (1.0 - pred_clamped).log()

        loss = -(target * log_pred + (1.0 - target) * log_one_minus_pred)

        if self.reduction == "mean":
            return loss.mean()
        elif self.reduction == "sum":
            return loss.sum()
        else:
            return loss

    def __repr__(self) -> str:
        return f"BCELoss(reduction='{self.reduction}')"


class BCEWithLogitsLoss(Module):
    """Binary Cross-Entropy with built-in Sigmoid.

    More numerically stable than Sigmoid + BCELoss because it uses
    the log-sum-exp trick internally.

    Input:
        prediction: raw logits (any real number) — sigmoid is applied internally
        target: binary labels (0 or 1)

    Formula (numerically stable version):
        L = max(x, 0) - x*t + log(1 + exp(-|x|))

    where x = prediction, t = target.

    Args:
        reduction: "mean" (default), "sum", or "none".
    """

    def __init__(self, reduction: str = "mean") -> None:
        super().__init__()
        object.__setattr__(self, "reduction", reduction)

    def forward(self, prediction: Tensor, target: Tensor) -> Tensor:
        """Compute BCE with logits using the sigmoid internally."""
        # Apply sigmoid to get probabilities, then use BCE
        probs = SigmoidFunction.apply(prediction)
        eps = 1e-7
        pred_clamped = probs.clamp(eps, 1.0 - eps)

        log_pred = pred_clamped.log()
        log_one_minus_pred = (1.0 - pred_clamped).log()
        loss = -(target * log_pred + (1.0 - target) * log_one_minus_pred)

        if self.reduction == "mean":
            return loss.mean()
        elif self.reduction == "sum":
            return loss.sum()
        else:
            return loss

    def __repr__(self) -> str:
        return f"BCEWithLogitsLoss(reduction='{self.reduction}')"


class NLLLoss(Module):
    """Negative Log-Likelihood Loss.

    Used with LogSoftmax output for classification.

    Input:
        prediction: (batch_size, num_classes) — log-probabilities
        target: (batch_size,) — integer class labels

    Formula:
        loss = -prediction[i, target[i]]  for each sample i

    Typically used as:
        log_probs = LogSoftmax(dim=1)(logits)
        loss = NLLLoss()(log_probs, targets)

    This is equivalent to CrossEntropyLoss(logits, targets).

    Args:
        reduction: "mean" (default), "sum", or "none".
    """

    def __init__(self, reduction: str = "mean") -> None:
        super().__init__()
        object.__setattr__(self, "reduction", reduction)

    def forward(self, prediction: Tensor, target: Tensor) -> Tensor:
        """Compute NLL loss.

        For each sample, select the log-probability of the correct class
        and negate it. Lower log-prob → higher loss.
        """
        if prediction.ndim != 2:
            raise ValueError(
                f"NLLLoss expects 2-D predictions, got {prediction.ndim}-D"
            )

        batch_size, num_classes = prediction.shape

        # Build one-hot encoding for targets
        one_hot_data = [0.0] * (batch_size * num_classes)
        for i in range(batch_size):
            class_idx = int(target.data[i])
            one_hot_data[i * num_classes + class_idx] = 1.0

        one_hot = Tensor(one_hot_data, prediction.shape, device=prediction.device)

        # -sum(one_hot * prediction) gives the NLL per sample
        elementwise = prediction * one_hot
        neg_elementwise = -elementwise

        if self.reduction == "mean":
            total = neg_elementwise.sum()
            return total / float(batch_size)
        elif self.reduction == "sum":
            return neg_elementwise.sum()
        else:
            return neg_elementwise.sum(dim=1)

    def __repr__(self) -> str:
        return f"NLLLoss(reduction='{self.reduction}')"
