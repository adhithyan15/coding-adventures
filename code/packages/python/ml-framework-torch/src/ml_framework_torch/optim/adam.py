"""
================================================================
ADAM — ADAPTIVE MOMENT ESTIMATION
================================================================

Adam is the most popular optimizer for deep learning. It combines
two ideas:

1. **Momentum** (first moment): Track a running average of gradients
   to smooth out noise → like SGD with momentum

2. **RMSprop** (second moment): Track a running average of squared
   gradients to adapt learning rate per-parameter → features with
   small gradients get larger updates

=== The Algorithm ===

For each parameter at step t:

    m = β₁ * m + (1 - β₁) * grad          # first moment (mean)
    v = β₂ * v + (1 - β₂) * grad²         # second moment (variance)

    m̂ = m / (1 - β₁ᵗ)                     # bias correction
    v̂ = v / (1 - β₂ᵗ)                     # bias correction

    w = w - lr * m̂ / (√v̂ + ε)            # update

=== Why Bias Correction? ===

m and v are initialized to zero. In early steps, they're biased
toward zero because they haven't accumulated enough history.
Dividing by (1 - βᵗ) corrects for this initialization bias.

At step 1: m̂ = m / (1 - 0.9) = 10 * m → big correction
At step 100: m̂ = m / (1 - 0.9¹⁰⁰) ≈ m → negligible correction

=== AdamW (Decoupled Weight Decay) ===

The original Adam paper mixed weight decay with gradient updates,
but this interacts poorly with the adaptive learning rate. AdamW
decouples them:

    w = w - lr * (m̂ / (√v̂ + ε) + weight_decay * w)

This gives better generalization in practice.

================================================================
"""

from __future__ import annotations

import math
from collections.abc import Iterator

from ml_framework_core import Parameter

from .optimizer import Optimizer


class Adam(Optimizer):
    """Adam optimizer (Adaptive Moment Estimation).

    Args:
        params: Parameters to optimize
        lr: Learning rate (default: 0.001)
        betas: Coefficients for moment estimates (default: (0.9, 0.999))
        eps: Numerical stability constant (default: 1e-8)
        weight_decay: L2 regularization (default: 0)

    The default hyperparameters (lr=0.001, betas=(0.9, 0.999)) work
    well for most deep learning tasks. They rarely need tuning.

    Example:
        optimizer = Adam(model.parameters(), lr=0.001)
    """

    def __init__(
        self,
        params: Iterator[Parameter] | list[Parameter],
        lr: float = 0.001,
        betas: tuple[float, float] = (0.9, 0.999),
        eps: float = 1e-8,
        weight_decay: float = 0.0,
    ) -> None:
        super().__init__(params, lr)
        self.beta1, self.beta2 = betas
        self.eps = eps
        self.weight_decay = weight_decay

        # ─── State for each parameter ───────────────────────────
        # m: first moment (running mean of gradients)
        # v: second moment (running mean of squared gradients)
        # t: step counter (for bias correction)
        self._m: list[list[float]] = [[0.0] * len(p.data) for p in self.params]
        self._v: list[list[float]] = [[0.0] * len(p.data) for p in self.params]
        self._t = 0

    def step(self) -> None:
        """Perform one Adam update step.

        For each parameter:
        1. Update first moment (momentum): m = β₁m + (1-β₁)g
        2. Update second moment (RMSprop): v = β₂v + (1-β₂)g²
        3. Bias-correct both moments
        4. Update: w = w - lr * m̂ / (√v̂ + ε)
        """
        self._t += 1

        for i, p in enumerate(self.params):
            if p.grad is None:
                continue

            grad_data = p.grad.data

            # ─── L2 regularization (added to gradient) ──────────
            if self.weight_decay != 0.0:
                grad_data = [
                    g + self.weight_decay * w for g, w in zip(grad_data, p.data)
                ]

            m = self._m[i]
            v = self._v[i]

            for j in range(len(m)):
                g = grad_data[j]

                # ─── Update moments ─────────────────────────────
                m[j] = self.beta1 * m[j] + (1 - self.beta1) * g
                v[j] = self.beta2 * v[j] + (1 - self.beta2) * g * g

            # ─── Bias correction ────────────────────────────────
            # In early steps, m and v are biased toward zero.
            # This correction factor grows from ~10x to ~1x.
            bc1 = 1.0 - self.beta1**self._t
            bc2 = 1.0 - self.beta2**self._t

            # ─── Update parameters ──────────────────────────────
            p.data = [
                w - self.lr * (m_j / bc1) / (math.sqrt(v_j / bc2) + self.eps)
                for w, m_j, v_j in zip(p.data, m, v)
            ]


class AdamW(Optimizer):
    """AdamW optimizer (Adam with decoupled weight decay).

    The key difference from Adam: weight decay is applied directly
    to the weights, NOT added to the gradient. This prevents the
    adaptive learning rate from scaling the regularization.

    Adam:  w = w - lr * (m̂/(√v̂+ε) + wd*w)    ← wd is scaled by Adam
    AdamW: w = w*(1-lr*wd) - lr * m̂/(√v̂+ε)   ← wd is independent

    This subtle change significantly improves generalization,
    especially with large models.

    Args:
        params: Parameters to optimize
        lr: Learning rate (default: 0.001)
        betas: Coefficients for moment estimates (default: (0.9, 0.999))
        eps: Numerical stability constant (default: 1e-8)
        weight_decay: Decoupled weight decay coefficient (default: 0.01)
    """

    def __init__(
        self,
        params: Iterator[Parameter] | list[Parameter],
        lr: float = 0.001,
        betas: tuple[float, float] = (0.9, 0.999),
        eps: float = 1e-8,
        weight_decay: float = 0.01,
    ) -> None:
        super().__init__(params, lr)
        self.beta1, self.beta2 = betas
        self.eps = eps
        self.weight_decay = weight_decay

        self._m: list[list[float]] = [[0.0] * len(p.data) for p in self.params]
        self._v: list[list[float]] = [[0.0] * len(p.data) for p in self.params]
        self._t = 0

    def step(self) -> None:
        """Perform one AdamW update step.

        Key difference from Adam: weight decay is decoupled.
        1. Apply weight decay directly: w = w * (1 - lr * wd)
        2. Then apply Adam update: w = w - lr * m̂ / (√v̂ + ε)
        """
        self._t += 1

        for i, p in enumerate(self.params):
            if p.grad is None:
                continue

            # ─── Decoupled weight decay (applied first!) ────────
            if self.weight_decay != 0.0:
                decay_factor = 1.0 - self.lr * self.weight_decay
                p.data = [w * decay_factor for w in p.data]

            grad_data = p.grad.data
            m = self._m[i]
            v = self._v[i]

            for j in range(len(m)):
                g = grad_data[j]
                m[j] = self.beta1 * m[j] + (1 - self.beta1) * g
                v[j] = self.beta2 * v[j] + (1 - self.beta2) * g * g

            bc1 = 1.0 - self.beta1**self._t
            bc2 = 1.0 - self.beta2**self._t

            p.data = [
                w - self.lr * (m_j / bc1) / (math.sqrt(v_j / bc2) + self.eps)
                for w, m_j, v_j in zip(p.data, m, v)
            ]
