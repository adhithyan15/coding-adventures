"""
================================================================
TF.KERAS.OPTIMIZERS — ALGORITHMS FOR UPDATING MODEL WEIGHTS
================================================================

Optimizers update model weights to minimize the loss function.
They implement the core training algorithm:

    for each training step:
        1. Forward pass → compute loss
        2. Backward pass → compute gradients
        3. Optimizer.apply_gradients() → update weights

=== Keras Optimizer API vs PyTorch ===

| Keras (TF)                                | PyTorch                          |
|-------------------------------------------|----------------------------------|
| optimizer = SGD(learning_rate=0.01)       | optimizer = SGD(params, lr=0.01) |
| optimizer.apply_gradients(zip(grads, vars))| optimizer.step()                 |
| No explicit zero_grad needed (tape handles)| optimizer.zero_grad() required   |
| learning_rate parameter name              | lr parameter name                |

The key API difference: Keras optimizers receive (gradient, variable)
pairs via apply_gradients(), while PyTorch optimizers hold references
to parameters and read their .grad attribute in step().

=== Learning Rate ===

The learning rate controls how big each update step is:
- Too high: weights oscillate or diverge
- Too low: training is extremely slow
- Just right: smooth convergence to a minimum

Common starting points:
    SGD:  0.01 - 0.1
    Adam: 0.001 (almost always works)
    AdamW: 0.001 with weight_decay=0.01

================================================================
"""

from __future__ import annotations

import math
from collections.abc import Iterable

from ml_framework_core import Parameter, Tensor


# =========================================================================
# Base Optimizer
# =========================================================================


class Optimizer:
    """Base class for all Keras optimizers.

    Subclasses implement _apply_single_gradient() for their specific
    update rule. The apply_gradients() method iterates over
    (gradient, variable) pairs and delegates to the subclass.

    Args:
        learning_rate: Step size for weight updates. Default: 0.01.
    """

    def __init__(self, learning_rate: float = 0.01) -> None:
        self.learning_rate = learning_rate
        self._iterations = 0

    def apply_gradients(
        self,
        grads_and_vars: Iterable[tuple[Tensor | None, Parameter]],
    ) -> None:
        """Update variables using their gradients.

        This is the main entry point for applying an optimization step.
        It receives an iterable of (gradient, variable) pairs
        (typically from zip(tape.gradient(...), model.trainable_variables)).

        Args:
            grads_and_vars: Iterable of (gradient_tensor, variable) pairs.
                            If gradient is None, the variable is skipped.

        Example:
            grads = tape.gradient(loss, model.trainable_variables)
            optimizer.apply_gradients(zip(grads, model.trainable_variables))
        """
        self._iterations += 1
        for grad, var in grads_and_vars:
            if grad is None:
                continue
            self._apply_single_gradient(grad, var)

    def _apply_single_gradient(self, grad: Tensor, var: Parameter) -> None:
        """Apply gradient to a single variable. Subclasses override."""
        raise NotImplementedError


# =========================================================================
# SGD (Stochastic Gradient Descent)
# =========================================================================


class SGD(Optimizer):
    """Stochastic Gradient Descent with optional momentum.

    Update rule:
        Without momentum: w = w - lr * grad
        With momentum:    v = momentum * v + grad
                          w = w - lr * v

    Args:
        learning_rate: Step size. Default: 0.01.
        momentum: Momentum factor. Default: 0.0.

    Example:
        optimizer = SGD(learning_rate=0.1, momentum=0.9)
        grads = tape.gradient(loss, model.trainable_variables)
        optimizer.apply_gradients(zip(grads, model.trainable_variables))
    """

    def __init__(
        self,
        learning_rate: float = 0.01,
        momentum: float = 0.0,
    ) -> None:
        super().__init__(learning_rate)
        self.momentum = momentum
        # Velocity buffers, keyed by variable id
        self._velocity: dict[int, list[float]] = {}

    def _apply_single_gradient(self, grad: Tensor, var: Parameter) -> None:
        vid = id(var)
        if vid not in self._velocity:
            self._velocity[vid] = [0.0] * len(var.data)

        if self.momentum != 0.0:
            v = self._velocity[vid]
            for j in range(len(v)):
                v[j] = self.momentum * v[j] + grad.data[j]
            var.data = [w - self.learning_rate * vj for w, vj in zip(var.data, v)]
        else:
            var.data = [w - self.learning_rate * g for w, g in zip(var.data, grad.data)]


# =========================================================================
# Adam
# =========================================================================


class Adam(Optimizer):
    """Adam optimizer (Adaptive Moment Estimation).

    Combines momentum (first moment) and RMSprop (second moment):
        m = beta1 * m + (1 - beta1) * grad
        v = beta2 * v + (1 - beta2) * grad^2
        m_hat = m / (1 - beta1^t)
        v_hat = v / (1 - beta2^t)
        w = w - lr * m_hat / (sqrt(v_hat) + epsilon)

    Args:
        learning_rate: Step size. Default: 0.001.
        beta_1: Decay rate for first moment. Default: 0.9.
        beta_2: Decay rate for second moment. Default: 0.999.
        epsilon: Numerical stability. Default: 1e-7.

    Example:
        optimizer = Adam(learning_rate=0.001)
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
        self._m: dict[int, list[float]] = {}
        self._v: dict[int, list[float]] = {}

    def _apply_single_gradient(self, grad: Tensor, var: Parameter) -> None:
        vid = id(var)
        if vid not in self._m:
            self._m[vid] = [0.0] * len(var.data)
            self._v[vid] = [0.0] * len(var.data)

        m = self._m[vid]
        v = self._v[vid]
        t = self._iterations  # already incremented in apply_gradients

        for j in range(len(m)):
            g = grad.data[j]
            m[j] = self.beta_1 * m[j] + (1 - self.beta_1) * g
            v[j] = self.beta_2 * v[j] + (1 - self.beta_2) * g * g

        bc1 = 1.0 - self.beta_1**t
        bc2 = 1.0 - self.beta_2**t

        var.data = [
            w - self.learning_rate * (mj / bc1) / (math.sqrt(vj / bc2) + self.epsilon)
            for w, mj, vj in zip(var.data, m, v)
        ]


# =========================================================================
# RMSprop
# =========================================================================


class RMSprop(Optimizer):
    """RMSprop optimizer.

    Maintains a running average of squared gradients to adapt the
    learning rate per-parameter:
        v = rho * v + (1 - rho) * grad^2
        w = w - lr * grad / (sqrt(v) + epsilon)

    Args:
        learning_rate: Step size. Default: 0.001.
        rho: Decay rate for squared gradient average. Default: 0.9.
        epsilon: Numerical stability. Default: 1e-7.
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

    def _apply_single_gradient(self, grad: Tensor, var: Parameter) -> None:
        vid = id(var)
        if vid not in self._v:
            self._v[vid] = [0.0] * len(var.data)

        v = self._v[vid]
        for j in range(len(v)):
            g = grad.data[j]
            v[j] = self.rho * v[j] + (1 - self.rho) * g * g

        var.data = [
            w - self.learning_rate * g / (math.sqrt(vj) + self.epsilon)
            for w, g, vj in zip(var.data, grad.data, v)
        ]


# =========================================================================
# AdamW
# =========================================================================


class AdamW(Optimizer):
    """AdamW optimizer (Adam with decoupled weight decay).

    Key difference from Adam: weight decay is applied directly to
    weights, not added to gradients. This prevents the adaptive
    learning rate from scaling the regularization.

    Args:
        learning_rate: Step size. Default: 0.001.
        beta_1: First moment decay. Default: 0.9.
        beta_2: Second moment decay. Default: 0.999.
        epsilon: Numerical stability. Default: 1e-7.
        weight_decay: Decoupled weight decay rate. Default: 0.01.
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

    def _apply_single_gradient(self, grad: Tensor, var: Parameter) -> None:
        vid = id(var)
        if vid not in self._m:
            self._m[vid] = [0.0] * len(var.data)
            self._v[vid] = [0.0] * len(var.data)

        # Decoupled weight decay: applied first
        if self.weight_decay != 0.0:
            decay = 1.0 - self.learning_rate * self.weight_decay
            var.data = [w * decay for w in var.data]

        m = self._m[vid]
        v = self._v[vid]
        t = self._iterations

        for j in range(len(m)):
            g = grad.data[j]
            m[j] = self.beta_1 * m[j] + (1 - self.beta_1) * g
            v[j] = self.beta_2 * v[j] + (1 - self.beta_2) * g * g

        bc1 = 1.0 - self.beta_1**t
        bc2 = 1.0 - self.beta_2**t

        var.data = [
            w - self.learning_rate * (mj / bc1) / (math.sqrt(vj / bc2) + self.epsilon)
            for w, mj, vj in zip(var.data, m, v)
        ]
