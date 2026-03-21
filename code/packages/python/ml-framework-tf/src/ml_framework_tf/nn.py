"""
================================================================
TF.NN — NEURAL NETWORK ACTIVATION FUNCTIONS
================================================================

The tf.nn module provides standalone activation functions that operate
on Tensors. These are the "functional" versions — they take a Tensor
and return a Tensor, without maintaining any internal state.

=== tf.nn vs tf.keras.layers ===

TensorFlow provides activations in two forms:
1. **tf.nn.relu(x)** — stateless function, just computes the math
2. **tf.keras.layers.ReLU()** — a Layer object that wraps tf.nn.relu

The functional form (tf.nn) is used when you're building custom
training loops or low-level models. The Layer form (tf.keras) is
used with Sequential/Model APIs.

=== Why These Four Activations? ===

| Activation | Formula                        | Use Case                    |
|------------|--------------------------------|-----------------------------|
| ReLU       | max(0, x)                      | Hidden layers (default)     |
| Sigmoid    | 1 / (1 + e^(-x))              | Binary classification output|
| Softmax    | e^xi / sum(e^xj)              | Multi-class output          |
| GELU       | x * Phi(x)                     | Transformers (BERT, GPT)    |

ReLU is the workhorse — fast, simple, and works well for most hidden
layers. Sigmoid squashes to (0,1) for probabilities. Softmax produces
a probability distribution (sums to 1). GELU is the modern choice
for transformer architectures.

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor
from ml_framework_core import (
    GELUFunction,
    ReLUFunction,
    SigmoidFunction,
    SoftmaxFunction,
)


def relu(x: Tensor) -> Tensor:
    """Rectified Linear Unit: y = max(0, x).

    The most widely used activation function. It's simple, fast,
    and empirically works well for deep networks.

    The key property: ReLU is piecewise linear, so it doesn't
    suffer from the vanishing gradient problem that plagues
    sigmoid/tanh in deep networks.

    Args:
        x: Input tensor of any shape.

    Returns:
        Tensor with negative values replaced by zero.

    Example:
        >>> x = tf.constant([-2.0, -1.0, 0.0, 1.0, 2.0])
        >>> tf.nn.relu(x)
        Tensor([0.0, 0.0, 0.0, 1.0, 2.0])
    """
    return ReLUFunction.apply(x)


def sigmoid(x: Tensor) -> Tensor:
    """Sigmoid: y = 1 / (1 + e^(-x)).

    Squashes any real number into the range (0, 1). Commonly used
    as the output activation for binary classification:
        probability = tf.nn.sigmoid(logits)

    Properties:
    - sigmoid(0) = 0.5 (uncertain)
    - sigmoid(large positive) → 1.0
    - sigmoid(large negative) → 0.0

    Args:
        x: Input tensor of any shape.

    Returns:
        Tensor with values in (0, 1).
    """
    return SigmoidFunction.apply(x)


def softmax(x: Tensor, axis: int = -1) -> Tensor:
    """Softmax: y_i = e^(x_i) / sum(e^(x_j)) along axis.

    Converts a vector of raw scores (logits) into a probability
    distribution that sums to 1.0. Used as the output activation
    for multi-class classification.

    TensorFlow uses 'axis' instead of PyTorch's 'dim':
        tf.nn.softmax(logits, axis=-1)   # TF style
        torch.softmax(logits, dim=-1)    # PyTorch style

    Args:
        x: Input tensor.
        axis: Dimension along which softmax is computed.
              Default: -1 (last dimension).

    Returns:
        Tensor of same shape with values that sum to 1 along axis.

    Example:
        >>> logits = tf.constant([2.0, 1.0, 0.1])
        >>> tf.nn.softmax(logits)
        Tensor([0.659, 0.242, 0.099])  # sums to 1.0
    """
    return SoftmaxFunction.apply(x, axis)


def gelu(x: Tensor) -> Tensor:
    """Gaussian Error Linear Unit: y = x * Phi(x).

    A smooth approximation of ReLU that's used extensively in
    modern transformer architectures (BERT, GPT, ViT).

    Unlike ReLU (which has a hard zero below 0), GELU smoothly
    gates values based on how likely they are under a Gaussian:
    - Large positive values pass through (~ReLU)
    - Values near 0 are partially attenuated
    - Large negative values are suppressed (~0)

    Args:
        x: Input tensor of any shape.

    Returns:
        Tensor with GELU activation applied element-wise.
    """
    return GELUFunction.apply(x)
