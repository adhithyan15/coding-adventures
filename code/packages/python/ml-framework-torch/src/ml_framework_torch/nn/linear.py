"""
================================================================
LINEAR LAYER — FULLY CONNECTED (DENSE) LAYER
================================================================

The Linear layer is the most fundamental neural network building block.
It computes:

    y = x @ W^T + b

Where:
    x = input tensor of shape (batch_size, in_features)
    W = weight matrix of shape (out_features, in_features)
    b = bias vector of shape (out_features,)
    y = output tensor of shape (batch_size, out_features)

=== Why W^T? ===

We store W as (out_features, in_features) because:
1. Each ROW of W corresponds to one output neuron's weights
2. This matches PyTorch convention
3. For a single input x (1-D), output_j = dot(W[j], x)

But matrix multiply requires shapes to align:
    x:   (batch, in_features)
    W.T: (in_features, out_features)
    result: (batch, out_features)  ← what we want!

=== Xavier Initialization ===

If weights are initialized too large, activations explode.
If too small, gradients vanish. Xavier initialization sets:

    stddev = 1 / sqrt(in_features)

This keeps the variance of activations roughly constant across
layers, which helps training converge faster.

================================================================
"""

from __future__ import annotations

import math

from ml_framework_core import Parameter, Tensor

from .module import Module


class Linear(Module):
    """Fully connected layer: y = x @ W.T + b.

    Args:
        in_features: Size of each input sample
        out_features: Size of each output sample
        bias: If True, adds a learnable bias. Default: True

    Example:
        layer = Linear(784, 128)   # 784 inputs → 128 outputs
        x = Tensor.randn(32, 784)  # batch of 32 samples
        y = layer(x)               # shape: (32, 128)
    """

    def __init__(
        self,
        in_features: int,
        out_features: int,
        bias: bool = True,
    ) -> None:
        super().__init__()

        # Store dimensions for repr
        object.__setattr__(self, "in_features", in_features)
        object.__setattr__(self, "out_features", out_features)

        # ─── Xavier initialization ──────────────────────────────
        # Scale random weights by 1/sqrt(fan_in) to keep the
        # variance of activations stable across layers.
        stddev = 1.0 / math.sqrt(in_features)
        self.weight = Parameter(Tensor.randn(out_features, in_features) * stddev)

        # ─── Optional bias ──────────────────────────────────────
        # Some architectures skip bias (e.g., before BatchNorm).
        # We track whether bias is enabled separately, and use
        # object.__setattr__ for None to avoid registering it.
        object.__setattr__(self, "_bias_enabled", bias)
        if bias:
            self.bias = Parameter(Tensor.zeros(out_features))
        else:
            object.__setattr__(self, "bias", None)

    def forward(self, x: Tensor) -> Tensor:
        """Compute y = x @ W.T + b.

        Steps:
        1. Transpose weight: (out, in) → (in, out)
        2. Matmul: (batch, in) @ (in, out) → (batch, out)
        3. Add bias: broadcast (out,) across batch dimension

        The matmul and addition are both autograd-tracked, so
        gradients flow back through both W and b during backward().
        """
        # x @ W.T
        output = x @ self.weight.t()

        # Add bias if present (broadcast across batch dimension)
        if self.bias is not None:
            # Our Tensor doesn't support broadcasting, so we need to
            # manually broadcast bias (out_features,) to (batch, out_features).
            #
            # The trick: use matmul to broadcast through the autograd graph.
            #   ones(batch, 1) @ bias.reshape(1, out) → (batch, out)
            #
            # This creates a proper autograd path: gradients flow through
            # the MatMulFunction back to self.bias, so the optimizer can
            # update it during training.
            batch_size = x.shape[0]
            out_features = self.bias.shape[0]
            ones_col = Tensor.ones(batch_size, 1)
            bias_row = self.bias.reshape(1, out_features)
            bias_broadcast = ones_col @ bias_row
            output = output + bias_broadcast

        return output

    def __repr__(self) -> str:
        return (
            f"Linear(in_features={self.in_features}, "
            f"out_features={self.out_features}, "
            f"bias={self._bias_enabled})"
        )
