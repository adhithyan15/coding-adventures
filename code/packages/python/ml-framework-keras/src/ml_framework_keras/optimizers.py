"""
================================================================
OPTIMIZERS — UPDATE WEIGHTS TO MINIMIZE LOSS
================================================================

After the backward pass computes gradients (∂loss/∂weight for each
weight), the optimizer uses those gradients to update the weights.
The simplest update rule is:

    weight = weight - learning_rate * gradient

But more sophisticated optimizers adapt the learning rate per-parameter
and use momentum to smooth out noisy gradients.

=== Keras Optimizer API ===

Keras optimizers differ from PyTorch's in one key way:

    PyTorch: optimizer.step()                 — uses stored param refs
    Keras:   optimizer.apply_gradients(...)    — receives (grad, param) pairs

The Keras style is more explicit — you pass the gradients and
parameters directly, rather than relying on the optimizer holding
references to the parameters.

=== String-Based Lookup ===

Keras lets you pass optimizer names as strings to model.compile():

    model.compile(optimizer="adam", ...)      # string form
    model.compile(optimizer=Adam(lr=0.01))   # instance form

The get_optimizer() function handles this conversion.

================================================================
"""

from __future__ import annotations

import math
from typing import Any

from ml_framework_core import Parameter, Tensor


# =========================================================================
# Base Optimizer
# =========================================================================


class Optimizer:
    """Base class for all Keras optimizers.

    All optimizers must implement apply_gradients(), which takes
    a list of (gradient, parameter) pairs and updates the parameters.

    The optimizer also tracks iteration count for features like
    learning rate warmup and Adam's bias correction.
    """

    def __init__(self, learning_rate: float = 0.01) -> None:
        self.learning_rate = learning_rate
        self._iterations = 0

    def apply_gradients(
        self, grads_and_vars: list[tuple[Tensor | None, Parameter]]
    ) -> None:
        """Update parameters using their gradients.

        Args:
            grads_and_vars: List of (gradient, parameter) tuples.
                If gradient is None, that parameter is skipped.
        """
        raise NotImplementedError

    def get_config(self) -> dict[str, Any]:
        """Return optimizer configuration for serialization."""
        return {
            "class_name": self.__class__.__name__,
            "learning_rate": self.learning_rate,
        }


# =========================================================================
# SGD — Stochastic Gradient Descent
# =========================================================================


class SGD(Optimizer):
    """Stochastic Gradient Descent with optional momentum.

    The most basic and interpretable optimizer.

    Without momentum:
        w = w - lr * grad

    With momentum (default β = 0.0, typical β = 0.9):
        v = β * v + grad
        w = w - lr * v

    Momentum keeps a running average of past gradients, smoothing
    out noise and accelerating convergence in consistent directions.

    Args:
        learning_rate: Step size for updates. Default: 0.01.
        momentum: Momentum factor. Default: 0.0 (no momentum).

    Example:
        model.compile(optimizer=SGD(learning_rate=0.1, momentum=0.9))
    """

    def __init__(
        self,
        learning_rate: float = 0.01,
        momentum: float = 0.0,
    ) -> None:
        super().__init__(learning_rate)
        self.momentum = momentum
        self._velocities: dict[int, list[float]] = {}

    def apply_gradients(
        self, grads_and_vars: list[tuple[Tensor | None, Parameter]]
    ) -> None:
        """Apply SGD update to each parameter.

        The update is computed in-place by directly modifying
        parameter.data — this is equivalent to a no_grad() context.
        """
        self._iterations += 1

        for grad, param in grads_and_vars:
            if grad is None:
                continue

            param_id = id(param)

            if self.momentum != 0.0:
                # Initialize velocity buffer on first use
                if param_id not in self._velocities:
                    self._velocities[param_id] = [0.0] * len(param.data)

                v = self._velocities[param_id]
                for j in range(len(v)):
                    v[j] = self.momentum * v[j] + grad.data[j]

                param.data = [
                    w - self.learning_rate * v_j for w, v_j in zip(param.data, v)
                ]
            else:
                param.data = [
                    w - self.learning_rate * g for w, g in zip(param.data, grad.data)
                ]

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config["momentum"] = self.momentum
        return config


# =========================================================================
# Adam — Adaptive Moment Estimation
# =========================================================================


class Adam(Optimizer):
    """Adam optimizer — the workhorse of deep learning.

    Combines two ideas:
    1. Momentum (first moment): running mean of gradients
    2. RMSprop (second moment): running mean of squared gradients

    The algorithm (for each parameter):
        m = β₁ * m + (1 - β₁) * grad           # first moment
        v = β₂ * v + (1 - β₂) * grad²          # second moment
        m_hat = m / (1 - β₁^t)                  # bias correction
        v_hat = v / (1 - β₂^t)                  # bias correction
        w = w - lr * m_hat / (√v_hat + ε)        # update

    The bias correction is crucial in early steps: since m and v
    start at zero, they'd be too small without correction.

    Args:
        learning_rate: Step size. Default: 0.001.
        beta_1: Exponential decay rate for first moment. Default: 0.9.
        beta_2: Exponential decay rate for second moment. Default: 0.999.
        epsilon: Numerical stability constant. Default: 1e-7.

    Example:
        model.compile(optimizer=Adam(learning_rate=0.001))
    """

    def __init__(
        self,
        learning_rate: float = 0.001,
        beta_1: float = 0.9,
        beta_2: float = 0.999,
        epsilon: float = 1e-7,
    ) -> None:
        super().__init__(learning_rate)
        self.beta_1 = beta_1
        self.beta_2 = beta_2
        self.epsilon = epsilon

        # Per-parameter state: first and second moment estimates
        self._m: dict[int, list[float]] = {}
        self._v: dict[int, list[float]] = {}

    def apply_gradients(
        self, grads_and_vars: list[tuple[Tensor | None, Parameter]]
    ) -> None:
        self._iterations += 1
        t = self._iterations

        for grad, param in grads_and_vars:
            if grad is None:
                continue

            param_id = id(param)

            # Initialize moment buffers on first use
            if param_id not in self._m:
                self._m[param_id] = [0.0] * len(param.data)
                self._v[param_id] = [0.0] * len(param.data)

            m = self._m[param_id]
            v = self._v[param_id]

            for j in range(len(m)):
                g = grad.data[j]
                # Update biased first moment estimate
                m[j] = self.beta_1 * m[j] + (1 - self.beta_1) * g
                # Update biased second moment estimate
                v[j] = self.beta_2 * v[j] + (1 - self.beta_2) * g * g

            # Bias correction factors
            bc1 = 1.0 - self.beta_1**t
            bc2 = 1.0 - self.beta_2**t

            # Update parameters
            param.data = [
                w
                - self.learning_rate
                * (m_j / bc1)
                / (math.sqrt(v_j / bc2) + self.epsilon)
                for w, m_j, v_j in zip(param.data, m, v)
            ]

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config.update(
            {
                "beta_1": self.beta_1,
                "beta_2": self.beta_2,
                "epsilon": self.epsilon,
            }
        )
        return config


# =========================================================================
# RMSprop — Root Mean Square Propagation
# =========================================================================


class RMSprop(Optimizer):
    """RMSprop optimizer — adapts learning rate per-parameter.

    The idea: parameters with large recent gradients get smaller
    learning rates, and vice versa. This helps with features that
    have very different gradient magnitudes.

    Algorithm:
        v = ρ * v + (1 - ρ) * grad²         # running mean of squared gradients
        w = w - lr * grad / (√v + ε)          # update with adapted rate

    RMSprop was proposed by Geoffrey Hinton in his Coursera lectures
    (never formally published!). It's the precursor to Adam.

    Args:
        learning_rate: Base learning rate. Default: 0.001.
        rho: Decay factor for running average. Default: 0.9.
        epsilon: Numerical stability constant. Default: 1e-7.
    """

    def __init__(
        self,
        learning_rate: float = 0.001,
        rho: float = 0.9,
        epsilon: float = 1e-7,
    ) -> None:
        super().__init__(learning_rate)
        self.rho = rho
        self.epsilon = epsilon
        self._v: dict[int, list[float]] = {}

    def apply_gradients(
        self, grads_and_vars: list[tuple[Tensor | None, Parameter]]
    ) -> None:
        self._iterations += 1

        for grad, param in grads_and_vars:
            if grad is None:
                continue

            param_id = id(param)
            if param_id not in self._v:
                self._v[param_id] = [0.0] * len(param.data)

            v = self._v[param_id]

            for j in range(len(v)):
                g = grad.data[j]
                v[j] = self.rho * v[j] + (1 - self.rho) * g * g

            param.data = [
                w - self.learning_rate * g / (math.sqrt(v_j) + self.epsilon)
                for w, g, v_j in zip(param.data, grad.data, v)
            ]

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config.update({"rho": self.rho, "epsilon": self.epsilon})
        return config


# =========================================================================
# AdamW — Adam with Decoupled Weight Decay
# =========================================================================


class AdamW(Optimizer):
    """AdamW optimizer — Adam with decoupled weight decay.

    Regular Adam applies weight decay through the gradient, which
    interacts with the adaptive learning rate. AdamW decouples them:

    Adam:  effective_decay = lr * wd / adaptive_lr  (varies per param!)
    AdamW: effective_decay = lr * wd                (consistent)

    This subtle difference significantly improves generalization,
    especially for large models like transformers.

    Algorithm:
        w = w * (1 - lr * weight_decay)       # 1. Decay weights first
        m = β₁ * m + (1 - β₁) * grad         # 2. Update moments
        v = β₂ * v + (1 - β₂) * grad²
        w = w - lr * m_hat / (√v_hat + ε)     # 3. Adam update

    Args:
        learning_rate: Step size. Default: 0.001.
        beta_1: First moment decay. Default: 0.9.
        beta_2: Second moment decay. Default: 0.999.
        epsilon: Numerical stability. Default: 1e-7.
        weight_decay: Decoupled weight decay coefficient. Default: 0.01.
    """

    def __init__(
        self,
        learning_rate: float = 0.001,
        beta_1: float = 0.9,
        beta_2: float = 0.999,
        epsilon: float = 1e-7,
        weight_decay: float = 0.01,
    ) -> None:
        super().__init__(learning_rate)
        self.beta_1 = beta_1
        self.beta_2 = beta_2
        self.epsilon = epsilon
        self.weight_decay = weight_decay

        self._m: dict[int, list[float]] = {}
        self._v: dict[int, list[float]] = {}

    def apply_gradients(
        self, grads_and_vars: list[tuple[Tensor | None, Parameter]]
    ) -> None:
        self._iterations += 1
        t = self._iterations

        for grad, param in grads_and_vars:
            if grad is None:
                continue

            param_id = id(param)

            # Step 1: Decoupled weight decay (applied BEFORE Adam update)
            if self.weight_decay != 0.0:
                decay = 1.0 - self.learning_rate * self.weight_decay
                param.data = [w * decay for w in param.data]

            # Initialize moment buffers
            if param_id not in self._m:
                self._m[param_id] = [0.0] * len(param.data)
                self._v[param_id] = [0.0] * len(param.data)

            m = self._m[param_id]
            v = self._v[param_id]

            for j in range(len(m)):
                g = grad.data[j]
                m[j] = self.beta_1 * m[j] + (1 - self.beta_1) * g
                v[j] = self.beta_2 * v[j] + (1 - self.beta_2) * g * g

            bc1 = 1.0 - self.beta_1**t
            bc2 = 1.0 - self.beta_2**t

            # Step 2: Adam update
            param.data = [
                w
                - self.learning_rate
                * (m_j / bc1)
                / (math.sqrt(v_j / bc2) + self.epsilon)
                for w, m_j, v_j in zip(param.data, m, v)
            ]

    def get_config(self) -> dict[str, Any]:
        config = super().get_config()
        config.update(
            {
                "beta_1": self.beta_1,
                "beta_2": self.beta_2,
                "epsilon": self.epsilon,
                "weight_decay": self.weight_decay,
            }
        )
        return config


# =========================================================================
# String-to-instance lookup
# =========================================================================

_OPTIMIZER_REGISTRY: dict[str, type[Optimizer]] = {
    "sgd": SGD,
    "adam": Adam,
    "rmsprop": RMSprop,
    "adamw": AdamW,
}


def get_optimizer(identifier: str | Optimizer) -> Optimizer:
    """Convert an optimizer identifier to an Optimizer instance.

    Accepts either a string name or an Optimizer instance:

        get_optimizer("adam")           → Adam()
        get_optimizer(Adam(lr=0.01))   → Adam(lr=0.01) (returned as-is)

    Args:
        identifier: String name or Optimizer instance.

    Returns:
        An Optimizer instance.

    Raises:
        ValueError: If the string name is not recognized.
    """
    if isinstance(identifier, Optimizer):
        return identifier

    if isinstance(identifier, str):
        key = identifier.lower()
        if key not in _OPTIMIZER_REGISTRY:
            raise ValueError(
                f"Unknown optimizer '{identifier}'. "
                f"Available: {sorted(_OPTIMIZER_REGISTRY.keys())}"
            )
        return _OPTIMIZER_REGISTRY[key]()

    raise TypeError(
        f"Optimizer must be a string or Optimizer instance. Got {type(identifier)}"
    )
