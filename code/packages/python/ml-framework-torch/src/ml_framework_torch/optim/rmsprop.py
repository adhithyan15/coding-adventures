"""
================================================================
RMSPROP — ROOT MEAN SQUARE PROPAGATION
================================================================

RMSprop was proposed by Geoffrey Hinton (unpublished, from a Coursera
lecture). It addresses a problem with basic SGD: all parameters use
the same learning rate, even though some gradients are much larger
than others.

RMSprop maintains a running average of squared gradients for each
parameter and divides the gradient by its root mean square:

    v = α * v + (1 - α) * grad²         # running average of grad²
    w = w - lr * grad / (√v + ε)         # adaptive update

=== Intuition ===

If a parameter's gradient has been large recently (high v), the
effective learning rate is reduced. If the gradient has been small
(low v), the effective learning rate is increased.

This is like Adam without the first moment (momentum). Adam is
essentially RMSprop + momentum + bias correction.

=== When to Use RMSprop vs Adam ===

- Adam is generally preferred (combines momentum + adaptive LR)
- RMSprop works well for RNNs and some reinforcement learning tasks
- RMSprop has fewer hyperparameters than Adam

================================================================
"""

from __future__ import annotations

import math
from collections.abc import Iterator

from ml_framework_core import Parameter

from .optimizer import Optimizer


class RMSprop(Optimizer):
    """RMSprop optimizer.

    Args:
        params: Parameters to optimize
        lr: Learning rate (default: 0.01)
        alpha: Smoothing constant for squared gradient average (default: 0.99)
        eps: Numerical stability constant (default: 1e-8)
        weight_decay: L2 regularization (default: 0)
        momentum: Momentum factor (default: 0)

    Example:
        optimizer = RMSprop(model.parameters(), lr=0.01, alpha=0.99)
    """

    def __init__(
        self,
        params: Iterator[Parameter] | list[Parameter],
        lr: float = 0.01,
        alpha: float = 0.99,
        eps: float = 1e-8,
        weight_decay: float = 0.0,
        momentum: float = 0.0,
    ) -> None:
        super().__init__(params, lr)
        self.alpha = alpha
        self.eps = eps
        self.weight_decay = weight_decay
        self.momentum = momentum

        # ─── State: running average of squared gradients ────────
        self._v: list[list[float]] = [[0.0] * len(p.data) for p in self.params]
        # ─── Momentum buffer ───────────────────────────────────
        self._buf: list[list[float]] = [[0.0] * len(p.data) for p in self.params]

    def step(self) -> None:
        """Perform one RMSprop update step.

        1. Update squared gradient average: v = α*v + (1-α)*g²
        2. Compute adaptive step: step = g / (√v + ε)
        3. Apply momentum (if any): buf = μ*buf + step
        4. Update parameter: w = w - lr * buf
        """
        for i, p in enumerate(self.params):
            if p.grad is None:
                continue

            grad_data = p.grad.data

            # ─── Weight decay ───────────────────────────────────
            if self.weight_decay != 0.0:
                grad_data = [
                    g + self.weight_decay * w for g, w in zip(grad_data, p.data)
                ]

            v = self._v[i]
            buf = self._buf[i]

            for j in range(len(v)):
                g = grad_data[j]
                # Running average of squared gradients
                v[j] = self.alpha * v[j] + (1 - self.alpha) * g * g

            if self.momentum != 0.0:
                for j in range(len(buf)):
                    g = grad_data[j]
                    buf[j] = self.momentum * buf[j] + g / (math.sqrt(v[j]) + self.eps)
                p.data = [w - self.lr * b for w, b in zip(p.data, buf)]
            else:
                p.data = [
                    w - self.lr * g / (math.sqrt(v_j) + self.eps)
                    for w, g, v_j in zip(p.data, grad_data, v)
                ]
