"""
================================================================
ML FRAMEWORK TF — TENSORFLOW-COMPATIBLE API
================================================================

This package provides a TensorFlow-compatible API built on top of
ml-framework-core. It implements the key TensorFlow abstractions:

1. **tf.constant** / **tf.Variable** — Immutable vs mutable tensors
2. **tf.GradientTape** — Explicit gradient tracking
3. **tf.keras** — High-level layers, models, optimizers, losses
4. **tf.nn** — Neural network activation functions
5. **tf.math** — Element-wise mathematical operations
6. **tf.random** — Random tensor generation
7. **tf.data** — Data pipeline utilities

=== Quick Start ===

    import ml_framework_tf as tf

    # Create variables
    w = tf.Variable([1.0, 2.0, 3.0])

    # Compute gradients
    with tf.GradientTape() as tape:
        y = w * w
        loss = tf.reduce_sum(y)
    grads = tape.gradient(loss, [w])

    # Build and train models
    model = tf.keras.Sequential([
        tf.keras.layers.Dense(64, activation='relu', input_dim=10),
        tf.keras.layers.Dense(1),
    ])
    model.compile(optimizer='adam', loss='mse')
    model.fit(x_train, y_train, epochs=10)

=== TF vs PyTorch: Key Differences ===

| Feature              | TensorFlow (this)           | PyTorch                   |
|----------------------|-----------------------------|---------------------------|
| Gradient tracking    | Explicit (GradientTape)     | Implicit (requires_grad)  |
| Mutable tensors      | tf.Variable                 | Any tensor                |
| Training loop        | model.fit() (batteries)     | Manual loop (flexible)    |
| Axis naming          | axis= parameter             | dim= parameter            |
| Loss arg order       | (y_true, y_pred)            | (pred, target)            |
| Optimizer update     | apply_gradients(zip(...))   | optimizer.step()          |

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor

from . import keras
from . import math_ops as math
from . import nn
from . import random
from .data import Dataset
from .gradient_tape import GradientTape
from .variable import Variable


# =========================================================================
# Top-level tensor creation functions
# =========================================================================


def constant(
    value: list | float | int,
    dtype: str | None = None,
) -> Tensor:
    """Create an immutable tensor (no gradient tracking).

    This is TensorFlow's primary way to create data tensors —
    values that don't change during training (inputs, labels, etc.).

    Unlike tf.Variable, constants cannot be modified after creation,
    and GradientTape does not watch them by default.

    Args:
        value: A Python number, list, or nested list.
        dtype: Optional dtype hint (accepted for API compatibility).

    Returns:
        A Tensor with requires_grad=False.

    Example:
        x = tf.constant([1.0, 2.0, 3.0])
        m = tf.constant([[1.0, 2.0], [3.0, 4.0]])
    """
    if isinstance(value, Tensor):
        return Tensor(list(value.data), value.shape, requires_grad=False)
    if isinstance(value, (int, float)):
        return Tensor([float(value)], (1,), requires_grad=False)
    return Tensor.from_list(value, requires_grad=False)


# =========================================================================
# Top-level factory functions
# =========================================================================


def zeros(shape: tuple[int, ...] | list[int]) -> Tensor:
    """Create a tensor filled with zeros.

    Args:
        shape: Shape of the tensor.

    Example:
        x = tf.zeros((3, 4))  # 3x4 matrix of zeros
    """
    return Tensor.zeros(*tuple(shape))


def ones(shape: tuple[int, ...] | list[int]) -> Tensor:
    """Create a tensor filled with ones.

    Args:
        shape: Shape of the tensor.

    Example:
        x = tf.ones((2, 3))
    """
    return Tensor.ones(*tuple(shape))


def eye(n: int) -> Tensor:
    """Create an n x n identity matrix.

    Args:
        n: Size of the square matrix.

    Example:
        I = tf.eye(3)  # 3x3 identity
    """
    return Tensor.eye(n)


def range_(
    start: float,
    limit: float | None = None,
    delta: float = 1.0,
) -> Tensor:
    """Create a 1-D tensor with evenly spaced values.

    Mimics tf.range(). Note: we name it range_() to avoid
    shadowing Python's built-in range.

    Args:
        start: Start value (or limit if limit is None).
        limit: End value (exclusive).
        delta: Step size. Default: 1.0.

    Example:
        x = tf.range_(0, 5)       # [0, 1, 2, 3, 4]
        x = tf.range_(1, 10, 2)   # [1, 3, 5, 7, 9]
    """
    if limit is None:
        limit = start
        start = 0.0
    return Tensor.arange(start, limit, delta)


# =========================================================================
# Top-level math operations
# =========================================================================


def matmul(a: Tensor, b: Tensor) -> Tensor:
    """Matrix multiplication: C = A @ B.

    Args:
        a: Left matrix of shape (M, K).
        b: Right matrix of shape (K, N).

    Returns:
        Result of shape (M, N).

    Example:
        c = tf.matmul(a, b)  # equivalent to a @ b
    """
    return a @ b


def add(a: Tensor, b: Tensor) -> Tensor:
    """Element-wise addition: C = A + B."""
    return a + b


def multiply(a: Tensor, b: Tensor) -> Tensor:
    """Element-wise multiplication: C = A * B."""
    return a * b


def reduce_sum(x: Tensor, axis: int | None = None, keepdims: bool = False) -> Tensor:
    """Sum elements, optionally along an axis.

    In TensorFlow, 'axis' replaces PyTorch's 'dim':
        tf.reduce_sum(x, axis=1)   # sum along axis 1
        torch.sum(x, dim=1)        # same thing

    Args:
        x: Input tensor.
        axis: Axis to sum along. None = sum all elements.
        keepdims: If True, keep the reduced dimension as size 1.

    Returns:
        Reduced tensor.
    """
    return x.sum(dim=axis, keepdim=keepdims)


def reduce_mean(x: Tensor, axis: int | None = None, keepdims: bool = False) -> Tensor:
    """Mean of elements, optionally along an axis.

    Args:
        x: Input tensor.
        axis: Axis to average along. None = mean all elements.
        keepdims: If True, keep the reduced dimension as size 1.

    Returns:
        Reduced tensor.
    """
    return x.mean(dim=axis, keepdim=keepdims)


def reshape(x: Tensor, shape: tuple[int, ...] | list[int]) -> Tensor:
    """Reshape a tensor to a new shape.

    Args:
        x: Input tensor.
        shape: New shape (total elements must match).

    Returns:
        Reshaped tensor.
    """
    return x.reshape(*tuple(shape))


def transpose(x: Tensor) -> Tensor:
    """Transpose a 2-D tensor (swap rows and columns).

    Args:
        x: 2-D tensor.

    Returns:
        Transposed tensor.
    """
    return x.t()


def clip_by_value(
    x: Tensor,
    clip_value_min: float,
    clip_value_max: float,
) -> Tensor:
    """Clamp tensor values to [min, max].

    Values below min are set to min, above max to max.

    Args:
        x: Input tensor.
        clip_value_min: Minimum value.
        clip_value_max: Maximum value.

    Returns:
        Clamped tensor.
    """
    return x.clamp(clip_value_min, clip_value_max)


# =========================================================================
# Data namespace
# =========================================================================


class data:
    """Namespace for tf.data, providing Dataset."""

    Dataset = Dataset


# =========================================================================
# Public API
# =========================================================================

__all__ = [
    # Tensor creation
    "constant",
    "Variable",
    "zeros",
    "ones",
    "eye",
    "range_",
    # Operations
    "matmul",
    "add",
    "multiply",
    "reduce_sum",
    "reduce_mean",
    "reshape",
    "transpose",
    "clip_by_value",
    # Core
    "GradientTape",
    "Tensor",
    # Submodules
    "nn",
    "math",
    "random",
    "keras",
    "data",
]
