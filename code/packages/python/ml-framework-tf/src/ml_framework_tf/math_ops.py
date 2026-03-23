"""
================================================================
TF.MATH — MATHEMATICAL OPERATIONS ON TENSORS
================================================================

The tf.math module provides element-wise mathematical functions.
In real TensorFlow, these are backed by XLA (Accelerated Linear
Algebra) compiler optimizations for GPU/TPU execution.

In our implementation, they're thin wrappers around the autograd
functions from ml-framework-core. Each operation is differentiable,
meaning gradients flow through them during backpropagation.

=== TF.MATH vs Tensor Methods ===

TensorFlow provides these as module-level functions:
    tf.math.log(x)    # TF style — functional
    x.log()           # Would be unusual in TF (though possible)

PyTorch provides both styles:
    torch.log(x)      # Functional
    x.log()           # Method

Our implementation mirrors the TF API: use tf.math.log(x), not x.log().

=== Gradient Formulas ===

| Function   | Forward     | Backward (dy/dx)          |
|------------|-------------|---------------------------|
| log(x)     | ln(x)       | 1/x                       |
| exp(x)     | e^x         | e^x (its own derivative!) |
| sqrt(x)    | x^0.5       | 0.5 / sqrt(x)             |
| abs(x)     | |x|         | sign(x)                   |

These are all differentiable (abs has a subgradient at 0), so they
work seamlessly with GradientTape for computing gradients.

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor
from ml_framework_core import AbsFunction, ExpFunction, LogFunction


def log(x: Tensor) -> Tensor:
    """Natural logarithm: y = ln(x), element-wise.

    The natural log is the inverse of exp():
        tf.math.log(tf.math.exp(x)) == x

    Commonly used in loss functions:
        cross_entropy = -sum(target * tf.math.log(prediction))

    Args:
        x: Input tensor. All elements should be positive.
           log(0) = -inf, log(negative) = NaN.

    Returns:
        Tensor with natural log applied element-wise.

    Gradient:
        d(ln(x))/dx = 1/x
    """
    return LogFunction.apply(x)


def exp(x: Tensor) -> Tensor:
    """Exponential: y = e^x, element-wise.

    The exponential function has a beautiful property: it's its own
    derivative! This makes backpropagation through exp() trivial:
        d(e^x)/dx = e^x

    Used extensively in softmax, attention mechanisms, and
    probabilistic models.

    Warning: exp() can overflow for large inputs. In practice,
    always use the log-sum-exp trick for numerical stability:
        log(sum(exp(x))) = max(x) + log(sum(exp(x - max(x))))

    Args:
        x: Input tensor of any shape.

    Returns:
        Tensor with e^x applied element-wise.
    """
    return ExpFunction.apply(x)


def sqrt(x: Tensor) -> Tensor:
    """Square root: y = sqrt(x), element-wise.

    Computed as x^0.5, which means the gradient is:
        d(sqrt(x))/dx = 0.5 / sqrt(x)

    Used in normalization layers (BatchNorm, LayerNorm) to
    compute standard deviation: std = sqrt(variance).

    Args:
        x: Input tensor. Elements should be non-negative.

    Returns:
        Tensor with square root applied element-wise.
    """
    return x**0.5


def abs(x: Tensor) -> Tensor:
    """Absolute value: y = |x|, element-wise.

    Returns the magnitude of each element, discarding sign.

    The gradient is the sign function:
        d|x|/dx = +1 if x > 0
                  -1 if x < 0
                   0 if x = 0 (subgradient)

    Used in L1 loss (Mean Absolute Error) and robust statistics.

    Args:
        x: Input tensor of any shape.

    Returns:
        Tensor with absolute values.
    """
    return AbsFunction.apply(x)
