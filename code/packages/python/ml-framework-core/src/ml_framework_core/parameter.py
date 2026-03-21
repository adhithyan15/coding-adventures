"""
================================================================
PARAMETER — A TENSOR THAT IS A LEARNABLE WEIGHT
================================================================

In neural networks, some tensors are special: they hold the learned
weights and biases that the optimizer updates during training. These
are Parameters.

A Parameter is just a Tensor with:
1. requires_grad=True (always — that's the whole point)
2. Registration with nn.Module so the optimizer can find them

When you define a layer:
    self.weight = Parameter(Tensor.randn(128, 784))
    self.bias = Parameter(Tensor.zeros(128))

The optimizer iterates over model.parameters() and updates each one:
    for param in model.parameters():
        param.data = [w - lr * g for w, g in zip(param.data, param.grad.data)]

This is literally BLAS saxpy: w_new = w + (-lr) * grad
================================================================
"""

from __future__ import annotations

from .tensor import Tensor


class Parameter(Tensor):
    """A tensor that always requires gradient computation.

    Parameters are the learnable weights of neural network layers.
    They are always tracked by the autograd engine and get their
    gradients populated during backward().
    """

    def __init__(
        self,
        data: Tensor | None = None,
        requires_grad: bool = True,
    ) -> None:
        if data is None:
            # Default to empty 0-D parameter
            super().__init__([0.0], (1,), requires_grad=requires_grad)
        else:
            super().__init__(
                list(data.data),
                data.shape,
                requires_grad=requires_grad,
                device=data.device,
            )

    def __repr__(self) -> str:
        if self.numel <= 6:
            data_str = str(self.data)
        else:
            data_str = f"[{self.data[0]:.4f}, ..., {self.data[-1]:.4f}]"
        return f"Parameter({data_str}, shape={self.shape})"
