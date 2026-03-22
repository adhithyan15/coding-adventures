# ml-framework-core

Core ML framework abstractions shared by the TensorFlow, PyTorch, and Keras API layers.

## What It Provides

1. **Tensor** -- N-dimensional array with automatic differentiation support
2. **Autograd** -- Computation graph and backward() algorithm (reverse-mode AD)
3. **Parameter** -- Learnable tensor (always requires_grad=True)
4. **Functions** -- Built-in differentiable operations (add, matmul, relu, softmax, etc.)
5. **DeviceManager** -- Maps device strings ("cpu", "cuda") to backends

## Architecture

This is the **engine** that all three API layers build on:

```
ml-framework-core   = the engine (tensors, autograd, math)
ml-framework-tf     = TensorFlow API (GradientTape, tf.keras)
ml-framework-torch  = PyTorch API (nn.Module, optim)
ml-framework-keras  = Keras 3 API (Sequential, compile/fit)
```

## Usage

```rust
use ml_framework_core::{Tensor, Parameter};

// Create tensors
let x = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0], &[2, 2], true, "cpu");
let w = Parameter::new(Tensor::randn(&[2, 2], "cpu"));

// Compute (builds computation graph)
let y = x.matmul(&w.tensor);
let loss = y.sum(None, false);

// Backpropagate
loss.backward(None);
```

## How Autograd Works

Every operation on tensors creates a node in a computation graph. When you call
`backward()`, the engine walks the graph in reverse topological order, applying
the chain rule to compute gradients for all leaf tensors.

## Storage Layout

Data is stored as a flat `Vec<f64>` in row-major (C) order. A shape of `[2, 3]`
means 2 rows, 3 columns:

```
data = [a, b, c, d, e, f]
represents: [[a, b, c],
             [d, e, f]]
```
