"""
================================================================
TF.KERAS.ACTIVATIONS — ACTIVATION FUNCTIONS AS PLAIN FUNCTIONS
================================================================

This module provides activation functions as simple Python functions,
matching TensorFlow's tf.keras.activations namespace. These are
used internally by layers (e.g., Dense(64, activation='relu'))
and can also be called directly.

=== How Activations Connect to Layers ===

When you write:
    Dense(64, activation='relu')

The Dense layer looks up the activation function by string name
using this module. Internally it's equivalent to:
    output = activation_fn(linear_output)

You can also pass a function directly:
    Dense(64, activation=tf.keras.activations.gelu)

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor
from ml_framework_core import (
    GELUFunction,
    ReLUFunction,
    SigmoidFunction,
    SoftmaxFunction,
    TanhFunction,
)


def relu(x: Tensor) -> Tensor:
    """ReLU activation: max(0, x)."""
    return ReLUFunction.apply(x)


def sigmoid(x: Tensor) -> Tensor:
    """Sigmoid activation: 1 / (1 + exp(-x))."""
    return SigmoidFunction.apply(x)


def tanh(x: Tensor) -> Tensor:
    """Tanh activation: (exp(x) - exp(-x)) / (exp(x) + exp(-x))."""
    return TanhFunction.apply(x)


def softmax(x: Tensor, axis: int = -1) -> Tensor:
    """Softmax activation: exp(x_i) / sum(exp(x_j)) along axis."""
    return SoftmaxFunction.apply(x, axis)


def gelu(x: Tensor) -> Tensor:
    """GELU activation: x * Phi(x), used in transformers."""
    return GELUFunction.apply(x)


def linear(x: Tensor) -> Tensor:
    """Linear (identity) activation: returns input unchanged.

    Used when no activation is desired (e.g., regression output).
    """
    return x


# =========================================================================
# Lookup table: string name → function
# =========================================================================

_ACTIVATION_MAP: dict[str | None, callable] = {
    None: linear,
    "linear": linear,
    "relu": relu,
    "sigmoid": sigmoid,
    "tanh": tanh,
    "softmax": softmax,
    "gelu": gelu,
}


def get(identifier: str | callable | None) -> callable:
    """Look up an activation function by name or return it directly.

    This is used internally by layers:
        activation_fn = activations.get('relu')  # returns relu function
        activation_fn = activations.get(my_func)  # returns my_func as-is

    Args:
        identifier: String name, callable, or None (returns linear).

    Returns:
        The activation function.

    Raises:
        ValueError: If the string name is not recognized.
    """
    if callable(identifier):
        return identifier
    if identifier in _ACTIVATION_MAP:
        return _ACTIVATION_MAP[identifier]
    raise ValueError(
        f"Unknown activation: '{identifier}'. Available: {list(_ACTIVATION_MAP.keys())}"
    )
