# ML Framework Core

Shared tensor/autograd engine for PyTorch, TensorFlow, and Keras API layers.
Layer 2 of the accelerator computing stack.

## What This Does

This package provides the **shared computational engine** that all three ML
framework APIs (PyTorch, TensorFlow, Keras) build on:

- **Tensor**: n-dimensional array with automatic differentiation
- **Autograd**: computation graph and `backward()` algorithm (backpropagation)
- **Parameter**: learnable weight tensor (always tracks gradients)
- **Functions**: 20+ differentiable operations (add, matmul, relu, softmax, etc.)
- **DeviceManager**: maps device strings ("cpu", "cuda", "metal") to BLAS backends

## Quick Start

```python
from ml_framework_core import Tensor, Parameter

# Create tensors with gradient tracking
x = Tensor.from_list([[1.0, 2.0], [3.0, 4.0]], requires_grad=True)
w = Parameter(Tensor.randn(2, 2))

# Forward pass (builds computation graph)
y = x @ w
loss = y.sum()

# Backward pass (computes all gradients via chain rule)
loss.backward()

# Gradients are now available
print(w.grad)  # ∂loss/∂w
```

## Architecture

```
PyTorch API  ──┐
TensorFlow API ├──→  ML Framework Core  ──→  BLAS Library  ──→  GPU Stack
Keras API     ──┘       (this package)        (Layer 3)       (Layers 4-11)
```

## Dependencies

- `blas-library` (Layer 3) — all tensor math dispatches here
