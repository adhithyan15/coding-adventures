"""
================================================================
ACTIVATIONS — NONLINEAR FUNCTIONS THAT MAKE NEURAL NETWORKS WORK
================================================================

Without activation functions, a neural network would just be a
stack of linear transformations — which collapses to a single
linear transformation (matrix multiply). Activations add the
nonlinearity that lets networks learn complex patterns.

=== The Activation Zoo ===

| Name    | Formula                          | Range      | Use case                    |
|---------|----------------------------------|------------|-----------------------------|
| ReLU    | max(0, x)                        | [0, +inf)  | Default hidden layer choice |
| Sigmoid | 1 / (1 + e^(-x))                | (0, 1)     | Binary classification       |
| Tanh    | (e^x - e^(-x))/(e^x + e^(-x))   | (-1, 1)    | Centered alternative to sig |
| Softmax | e^xi / sum(e^xj)                | (0, 1)     | Multi-class output layer    |
| GELU    | x * Phi(x)                       | (-0.17,+inf)| Transformers (BERT, GPT)   |

=== String-Based Lookup ===

Keras lets you pass activations as strings to layers:

    Dense(128, activation="relu")       # string form
    Dense(128, activation=relu)         # function form
    Dense(128, activation=None)         # no activation (linear)

The get_activation() function handles this conversion, mapping
strings like "relu" to the corresponding callable function.

================================================================
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ml_framework_core import (
    GELUFunction,
    ReLUFunction,
    SigmoidFunction,
    SoftmaxFunction,
    TanhFunction,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    from ml_framework_core import Tensor


# =========================================================================
# Individual activation functions
# =========================================================================
# Each of these wraps the corresponding autograd Function from
# ml-framework-core. They return a new Tensor with the activation
# applied element-wise (or along an axis, for softmax).


def relu(x: Tensor) -> Tensor:
    """Rectified Linear Unit: max(0, x).

    The most widely used activation function. Simple, fast, and
    its gradient (0 or 1) doesn't suffer from vanishing gradients
    for positive inputs.

    Potential issue: "dead neurons" — if a neuron's output becomes
    permanently negative, its gradient is always 0 and it stops learning.

    Args:
        x: Input tensor.

    Returns:
        Tensor with negative values replaced by 0.
    """
    return ReLUFunction.apply(x)


def sigmoid(x: Tensor) -> Tensor:
    """Sigmoid: 1 / (1 + e^(-x)).

    Squashes any real number into the range (0, 1). Originally the
    default activation, now mainly used for binary classification
    outputs (interpreting the output as a probability).

    Problem: gradients vanish for large |x| because the curve
    flattens out. This makes deep networks hard to train.

    Args:
        x: Input tensor.

    Returns:
        Tensor with values in (0, 1).
    """
    return SigmoidFunction.apply(x)


def tanh(x: Tensor) -> Tensor:
    """Hyperbolic tangent: (e^x - e^(-x)) / (e^x + e^(-x)).

    Like sigmoid but centered at 0 (output range: -1 to 1).
    This zero-centering helps gradient flow in some architectures.

    Still suffers from vanishing gradients for large |x|, but
    less so than sigmoid because it has steeper gradients near 0.

    Args:
        x: Input tensor.

    Returns:
        Tensor with values in (-1, 1).
    """
    return TanhFunction.apply(x)


def softmax(x: Tensor, axis: int = -1) -> Tensor:
    """Softmax: e^xi / sum(e^xj) along an axis.

    Converts a vector of raw scores (logits) into a probability
    distribution — all values are in (0, 1) and sum to 1.

    This is the standard output activation for multi-class
    classification. If you have 10 classes (like MNIST digits),
    softmax gives you a probability for each class.

    Implementation detail: we subtract max(x) before exponentiating
    to prevent numerical overflow. This doesn't change the result
    because softmax(x) = softmax(x - c) for any constant c.

    Args:
        x: Input tensor.
        axis: Dimension along which to compute softmax. Default: -1 (last).

    Returns:
        Tensor where values along `axis` sum to 1.
    """
    return SoftmaxFunction.apply(x, axis)


def gelu(x: Tensor) -> Tensor:
    """Gaussian Error Linear Unit: x * Phi(x).

    A smooth approximation to ReLU that weights inputs by how
    likely they are to be positive (under a Gaussian distribution).
    Used extensively in modern transformers (BERT, GPT, ViT).

    Unlike ReLU, GELU is smooth everywhere — no sharp corner at
    x=0. It also has a small negative region, which can help
    with learning.

    Uses the tanh approximation:
        gelu(x) = 0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715*x^3)))

    Args:
        x: Input tensor.

    Returns:
        Tensor with GELU applied element-wise.
    """
    return GELUFunction.apply(x)


def linear(x: Tensor) -> Tensor:
    """Linear (identity) activation: f(x) = x.

    Used when you want no activation at all. This is the default
    when activation=None in a Dense layer — the layer just performs
    the affine transformation without any nonlinearity.

    Args:
        x: Input tensor.

    Returns:
        The input tensor unchanged.
    """
    return x


# =========================================================================
# String-to-function lookup
# =========================================================================

# Registry mapping string names to activation functions.
# These are the same strings Keras 3 accepts.
_ACTIVATION_REGISTRY: dict[str, Callable] = {
    "relu": relu,
    "sigmoid": sigmoid,
    "tanh": tanh,
    "softmax": softmax,
    "gelu": gelu,
    "linear": linear,
}


def get_activation(identifier: str | Callable | None) -> Callable | None:
    """Convert an activation identifier to a callable function.

    This is the key convenience that makes Keras so user-friendly.
    You can pass activations as strings, functions, or None:

        get_activation("relu")      → relu function
        get_activation(my_fn)       → my_fn (returned as-is)
        get_activation(None)        → None (no activation)

    Args:
        identifier: String name, callable, or None.

    Returns:
        The activation function, or None for no activation.

    Raises:
        ValueError: If the string name is not recognized.
    """
    if identifier is None:
        return None

    if callable(identifier):
        return identifier

    if isinstance(identifier, str):
        key = identifier.lower()
        if key not in _ACTIVATION_REGISTRY:
            raise ValueError(
                f"Unknown activation '{identifier}'. "
                f"Available: {sorted(_ACTIVATION_REGISTRY.keys())}"
            )
        return _ACTIVATION_REGISTRY[key]

    raise TypeError(
        f"Activation must be a string, callable, or None. Got {type(identifier)}"
    )
