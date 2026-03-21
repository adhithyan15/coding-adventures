"""
================================================================
ML FRAMEWORK CORE — SHARED TENSOR/AUTOGRAD ENGINE
================================================================

This is the shared engine that PyTorch, TensorFlow, and Keras API
layers all build on. It provides:

1. Tensor — n-dimensional array with automatic differentiation
2. Autograd — computation graph and backward() algorithm
3. Parameter — learnable tensor (always requires_grad=True)
4. Functions — built-in differentiable operations (add, matmul, relu...)
5. DeviceManager — maps device strings to BLAS backends

Usage:
    from ml_framework_core import Tensor, Parameter

    # Create tensors
    x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]], requires_grad=True)
    w = Parameter(Tensor.randn(2, 2))

    # Compute (builds computation graph)
    y = x @ w
    loss = y.sum()

    # Backpropagate (walks graph, computes gradients)
    loss.backward()

    # Gradients are now available
    print(w.grad)  # ∂loss/∂w
================================================================
"""

from .autograd import Function, is_grad_enabled, no_grad
from .device import DeviceManager
from .functions import (
    AbsFunction,
    AddFunction,
    ClampFunction,
    DivFunction,
    ExpFunction,
    GELUFunction,
    LogFunction,
    MatMulFunction,
    MeanFunction,
    MulFunction,
    NegFunction,
    PowFunction,
    ReLUFunction,
    ReshapeFunction,
    SigmoidFunction,
    SoftmaxFunction,
    SubFunction,
    SumFunction,
    TanhFunction,
    TransposeFunction,
)
from .parameter import Parameter
from .tensor import Tensor

__all__ = [
    # Core
    "Tensor",
    "Parameter",
    "Function",
    # Autograd
    "no_grad",
    "is_grad_enabled",
    # Device
    "DeviceManager",
    # Functions (for extension/testing)
    "AbsFunction",
    "AddFunction",
    "ClampFunction",
    "DivFunction",
    "ExpFunction",
    "GELUFunction",
    "LogFunction",
    "MatMulFunction",
    "MeanFunction",
    "MulFunction",
    "NegFunction",
    "PowFunction",
    "ReLUFunction",
    "ReshapeFunction",
    "SigmoidFunction",
    "SoftmaxFunction",
    "SubFunction",
    "SumFunction",
    "TanhFunction",
    "TransposeFunction",
]
