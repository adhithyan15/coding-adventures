"""
================================================================
FLATTEN — RESHAPE MULTI-DIMENSIONAL INPUT TO 2-D
================================================================

Flatten converts a multi-dimensional tensor into a 2-D tensor,
typically used between convolutional layers and linear layers:

    Input:  (batch_size, channels, height, width) = (32, 3, 28, 28)
    Output: (batch_size, channels * height * width) = (32, 2352)

The first dimension (batch) is preserved; everything else is
flattened into a single feature dimension.

Usage:
    model = Sequential(
        ...some conv layers...,
        Flatten(),           # (batch, C, H, W) → (batch, C*H*W)
        Linear(C*H*W, 128),
    )

================================================================
"""

from __future__ import annotations

from ml_framework_core import Tensor

from .module import Module


class Flatten(Module):
    """Flatten dimensions from start_dim to end_dim into one.

    Args:
        start_dim: First dimension to flatten. Default: 1
        end_dim: Last dimension to flatten. Default: -1

    With default args, preserves batch dimension and flattens the rest:
        (batch, d1, d2, ...) → (batch, d1 * d2 * ...)

    Example:
        x = Tensor.randn(4, 3, 8)     # (4, 3, 8)
        Flatten()(x)                    # (4, 24)
        Flatten(start_dim=0)(x)         # (96,) — flatten everything
    """

    def __init__(self, start_dim: int = 1, end_dim: int = -1) -> None:
        super().__init__()
        object.__setattr__(self, "start_dim", start_dim)
        object.__setattr__(self, "end_dim", end_dim)

    def forward(self, x: Tensor) -> Tensor:
        return x.flatten(self.start_dim, self.end_dim)

    def __repr__(self) -> str:
        return f"Flatten(start_dim={self.start_dim}, end_dim={self.end_dim})"
