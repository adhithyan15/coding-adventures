"""
================================================================
ACTIVATION FUNCTIONS — NON-LINEARITIES FOR NEURAL NETWORKS
================================================================

Without activation functions, a neural network is just a chain of
matrix multiplies — equivalent to a single linear transformation.
Activation functions add non-linearity, enabling networks to learn
complex patterns.

=== Why Non-Linearity Matters ===

Consider two linear layers:
    y = W2 @ (W1 @ x) = (W2 @ W1) @ x = W_combined @ x

No matter how many linear layers you stack, it collapses to one!
But with ReLU in between:
    y = W2 @ relu(W1 @ x)

This can NOT be simplified to a single linear transformation.
The ReLU "folds" the space, creating piece-wise linear regions
that can approximate any continuous function.

=== Activation Gallery ===

    ReLU:    max(0, x)         — Simple, fast, most popular
    GELU:    x · Phi(x)       — Smooth ReLU, used in transformers
    Sigmoid: 1/(1 + e^(-x))   — Squashes to (0, 1), for probabilities
    Tanh:    (e^x - e^-x)/(e^x + e^-x) — Squashes to (-1, 1)
    Softmax: e^xi / Sum e^xj  — Probability distribution over classes
    LogSoftmax: log(softmax(x)) — Numerically stable log-probabilities

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

from .module import Module


class ReLU(Module):
    """Rectified Linear Unit: y = max(0, x).

    The most widely used activation function. Simple, fast, and works
    well in practice. Its gradient is trivial: 1 if x > 0, else 0.

    One downside: "dying ReLU" — if a neuron always outputs negative
    values, it gets zero gradient forever and never recovers.
    """

    def forward(self, x: Tensor) -> Tensor:
        return ReLUFunction.apply(x)

    def __repr__(self) -> str:
        return "ReLU()"


class GELU(Module):
    """Gaussian Error Linear Unit.

    Approximated as: y = 0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))

    GELU is a smooth approximation of ReLU that:
    - Has a non-zero gradient everywhere (no dying neurons)
    - Weights inputs by their magnitude (probabilistic gating)
    - Used in BERT, GPT, and most modern transformers

    Think of it as: "ReLU, but the transition from 0 to x is smooth
    instead of a sharp kink at zero."
    """

    def forward(self, x: Tensor) -> Tensor:
        return GELUFunction.apply(x)

    def __repr__(self) -> str:
        return "GELU()"


class Sigmoid(Module):
    """Sigmoid: y = 1 / (1 + e^(-x)).

    Squashes any input to the range (0, 1). Used for:
    - Binary classification (output layer)
    - Gates in LSTMs and attention mechanisms
    - Probability estimation

    Historical note: sigmoid was the original activation function
    in neural networks, but was largely replaced by ReLU due to
    the "vanishing gradient" problem (gradients shrink exponentially
    in deep networks because sigmoid's max gradient is only 0.25).
    """

    def forward(self, x: Tensor) -> Tensor:
        return SigmoidFunction.apply(x)

    def __repr__(self) -> str:
        return "Sigmoid()"


class Tanh(Module):
    """Hyperbolic tangent: y = tanh(x).

    Like sigmoid but outputs range from (-1, 1) instead of (0, 1).
    This zero-centered property makes it better than sigmoid for
    hidden layers, but it still suffers from vanishing gradients.

    Used in LSTMs and some RNN architectures.
    """

    def forward(self, x: Tensor) -> Tensor:
        return TanhFunction.apply(x)

    def __repr__(self) -> str:
        return "Tanh()"


class Softmax(Module):
    """Softmax: y_i = exp(x_i) / Sum exp(x_j).

    Converts a vector of raw scores (logits) into a probability
    distribution. All outputs sum to 1 and are non-negative.

    Used as the final layer in multi-class classification:
        logits = model(x)            # raw scores, e.g. [2.1, 0.5, -1.3]
        probs = Softmax(dim=-1)(logits)  # probabilities, e.g. [0.78, 0.16, 0.06]

    Args:
        dim: Dimension along which to compute softmax. Default: -1 (last dim)
    """

    def __init__(self, dim: int = -1) -> None:
        super().__init__()
        object.__setattr__(self, "dim", dim)

    def forward(self, x: Tensor) -> Tensor:
        return SoftmaxFunction.apply(x, self.dim)

    def __repr__(self) -> str:
        return f"Softmax(dim={self.dim})"


class LogSoftmax(Module):
    """Log-Softmax: y = log(softmax(x)).

    Computing log(softmax(x)) directly is numerically unstable because
    softmax can produce very small numbers (underflow -> log(0) = -inf).

    Instead, we use the log-sum-exp trick:
        log_softmax(x_i) = x_i - log(Sum exp(x_j))
                         = x_i - max(x) - log(Sum exp(x_j - max(x)))

    Subtracting max(x) prevents overflow in exp() while preserving
    the mathematical result.

    Used with NLLLoss for classification (equivalent to CrossEntropyLoss).

    Args:
        dim: Dimension along which to compute. Default: -1 (last dim)
    """

    def __init__(self, dim: int = -1) -> None:
        super().__init__()
        object.__setattr__(self, "dim", dim)

    def forward(self, x: Tensor) -> Tensor:
        """Compute log(softmax(x)) using the log-sum-exp trick.

        For numerical stability:
        1. Subtract max value (prevents exp overflow)
        2. Compute log(sum(exp(x - max))) + max
        3. Result: x - log_sum_exp
        """
        # For 1-D tensors
        if x.ndim == 1:
            max_val = max(x.data)
            shifted = [v - max_val for v in x.data]
            log_sum = math.log(sum(math.exp(s) for s in shifted))
            result = [s - log_sum for s in shifted]
            return Tensor(
                result, x.shape, requires_grad=x.requires_grad, device=x.device
            )

        # For 2-D tensors, compute along the specified dim
        dim = self.dim
        if dim < 0:
            dim = x.ndim + dim

        if x.ndim == 2 and dim == 1:
            # Most common case: (batch, classes) -> log_softmax along classes
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

        # Fallback: compute softmax then take log
        softmax_result = SoftmaxFunction.apply(x, self.dim)
        return softmax_result.log()

    def __repr__(self) -> str:
        return f"LogSoftmax(dim={self.dim})"
