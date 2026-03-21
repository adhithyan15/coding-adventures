"""
================================================================
TF.RANDOM — RANDOM TENSOR GENERATION
================================================================

Random number generation is essential for:
1. **Weight initialization** — Neural network weights must start
   with random values (not zeros!) to break symmetry.
2. **Dropout** — Randomly zeroing neurons during training.
3. **Data augmentation** — Random transformations for training data.
4. **Stochastic algorithms** — Sampling in VAEs, GANs, etc.

=== tf.random.normal ===

Generates tensors from a normal (Gaussian) distribution:

    Normal(mean, stddev)

    The probability density function:
        p(x) = (1 / sqrt(2*pi*sigma^2)) * exp(-(x-mu)^2 / (2*sigma^2))

    Default: mean=0.0, stddev=1.0 (standard normal)

This is the most commonly used random initialization. Weight
initialization schemes (Xavier, He, etc.) all start with random
normal values and scale them appropriately.

=== Implementation ===

We delegate to ml-framework-core's Tensor.randn() for generating
standard normal values, then scale and shift as needed:
    result = mean + stddev * randn(shape)

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor


def normal(
    shape: tuple[int, ...] | list[int],
    mean: float = 0.0,
    stddev: float = 1.0,
) -> Tensor:
    """Generate a tensor of random values from a normal distribution.

    Each element is independently sampled from:
        Normal(mean, stddev^2)

    The sampling uses the Box-Muller transform internally
    (implemented in ml-framework-core's Tensor.randn).

    Args:
        shape: Shape of the output tensor, e.g., (2, 3) or [4, 5].
        mean: Mean of the distribution. Default: 0.0.
        stddev: Standard deviation. Default: 1.0.

    Returns:
        Tensor of the given shape with random normal values.
        The tensor has requires_grad=False (it's data, not a parameter).

    Example:
        # Standard normal: mean=0, stddev=1
        x = tf.random.normal((3, 4))

        # Custom distribution: mean=5, stddev=2
        x = tf.random.normal((3, 4), mean=5.0, stddev=2.0)
    """
    shape_tuple = tuple(shape)
    # Generate standard normal values (mean=0, std=1)
    result = Tensor.randn(*shape_tuple)

    # Scale and shift: result = mean + stddev * standard_normal
    # If mean=0 and stddev=1, this is a no-op.
    if stddev != 1.0:
        result.data = [x * stddev for x in result.data]
    if mean != 0.0:
        result.data = [x + mean for x in result.data]

    return result
