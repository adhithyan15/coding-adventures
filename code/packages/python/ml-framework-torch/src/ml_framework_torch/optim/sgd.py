"""
================================================================
SGD — STOCHASTIC GRADIENT DESCENT (WITH OPTIONAL MOMENTUM)
================================================================

The simplest and most foundational optimizer. The basic update rule:

    w_new = w - lr * grad

This moves each parameter in the direction that reduces the loss,
scaled by the learning rate.

=== With Momentum ===

Plain SGD can oscillate in ravines (narrow valleys in the loss
landscape). Momentum smooths this by maintaining a velocity:

    v = momentum * v + grad           (exponential moving average)
    w = w - lr * v                    (update using smoothed gradient)

Think of momentum like a ball rolling downhill:
- Without momentum: the ball can get stuck bouncing between walls
- With momentum: the ball builds up speed in consistent directions

Typical momentum value: 0.9 (retains 90% of previous velocity)

=== With Weight Decay ===

Weight decay (L2 regularization) penalizes large weights:
    w = w - lr * (grad + weight_decay * w)

This prevents overfitting by keeping weights small. Unlike L2
regularization in the loss function, SGD weight decay is applied
directly during the update step.

================================================================
"""

from __future__ import annotations

from collections.abc import Iterator

from ml_framework_core import Parameter

from .optimizer import Optimizer


class SGD(Optimizer):
    """Stochastic Gradient Descent with momentum and weight decay.

    Args:
        params: Parameters to optimize
        lr: Learning rate (default: 0.01)
        momentum: Momentum factor (default: 0, no momentum)
        weight_decay: L2 regularization coefficient (default: 0)

    Example:
        optimizer = SGD(model.parameters(), lr=0.1, momentum=0.9)

        for epoch in range(100):
            optimizer.zero_grad()
            loss = compute_loss(model, data)
            loss.backward()
            optimizer.step()
    """

    def __init__(
        self,
        params: Iterator[Parameter] | list[Parameter],
        lr: float = 0.01,
        momentum: float = 0.0,
        weight_decay: float = 0.0,
    ) -> None:
        super().__init__(params, lr)
        self.momentum = momentum
        self.weight_decay = weight_decay

        # ─── Velocity buffers for momentum ──────────────────────
        # Each parameter gets its own velocity vector, initialized to zero.
        # These persist across steps to accumulate momentum.
        self._velocity: list[list[float]] = [[0.0] * len(p.data) for p in self.params]

    def step(self) -> None:
        """Perform one optimization step.

        For each parameter with a gradient:
        1. Add weight decay to gradient (if any)
        2. Update velocity with momentum (if any)
        3. Update parameter: w = w - lr * v

        Without momentum: w = w - lr * grad
        """
        for i, p in enumerate(self.params):
            if p.grad is None:
                continue

            grad_data = p.grad.data

            # ─── Weight decay: add L2 penalty ───────────────────
            if self.weight_decay != 0.0:
                grad_data = [
                    g + self.weight_decay * w for g, w in zip(grad_data, p.data)
                ]

            # ─── Momentum: smooth gradient with velocity ────────
            if self.momentum != 0.0:
                v = self._velocity[i]
                for j in range(len(v)):
                    v[j] = self.momentum * v[j] + grad_data[j]
                update = v
            else:
                update = grad_data

            # ─── Update parameter ───────────────────────────────
            # w_new = w - lr * update
            p.data = [w - self.lr * u for w, u in zip(p.data, update)]
