# G08 — ML Frameworks: PyTorch, TensorFlow, and Keras

## Overview

Imagine you're building a house. Layers 11–3 gave us the raw materials: bricks
(logic gates), cement (FP arithmetic), walls (BLAS). Now we need to build the
rooms — the frameworks that data scientists actually live in.

This layer implements **three complete ML framework APIs** — PyTorch, TensorFlow,
and Keras — all sharing a single tensor/autograd engine that dispatches to our
BLAS library. A data scientist should be able to write code that *looks* like
real PyTorch or TensorFlow, and it runs on our stack all the way down to the
logic gates.

The key insight: **all three frameworks do the same thing differently.** They all
compute tensor operations, track gradients, and update weights. They just expose
different APIs for doing so:

| Framework | Philosophy | Gradient Style | Model Building |
|-----------|-----------|---------------|----------------|
| PyTorch | Eager-first, Pythonic | `loss.backward()` implicit tape | `nn.Module` subclassing |
| TensorFlow | Graph + eager hybrid | `GradientTape` explicit context | `tf.keras` or raw ops |
| Keras | High-level, declarative | Hidden inside `model.fit()` | `Sequential` / Functional API |

**Real-world analogs:**

| Our Implementation | Industry Equivalent | What It Does |
|-------------------|---------------------|--------------|
| Tensor Engine (shared) | ATen (PyTorch internals) | Tensor ops + autograd |
| `torch` module | PyTorch 2.x | Eager tensor framework |
| `tf` module | TensorFlow 2.x | Graph/eager hybrid framework |
| `keras` module | Keras 3.x | High-level multi-backend API |
| BLAS dispatch | cuBLAS / MKL / Accelerate | Actual computation |

## Layer Position

```
Layer 11:  Logic Gates
Layer 10:  FP Arithmetic + Clock
Layer  9:  GPU Core (Processing Element)
Layer  8:  Parallel Execution Engine
Layer  7:  Compute Unit
Layer  6:  Device Simulator
Layer  5:  Compute Runtime
Layer  4:  Vendor API Simulators
Layer  3:  BLAS Library                    ← computation engine
Layer  2:  ML Frameworks  ← THIS LAYER    ← PyTorch / TF / Keras APIs
Layer  1:  Applications (train models, run inference)
```

**Depends on:**
- `blas-library` (Layer 3) — all tensor math dispatches here
- `loss-functions` — existing loss implementations (MSE, MAE, BCE, CCE)
- `gradient-descent` — existing SGD optimizer
- `matrix` — existing Matrix type (bridged via converters)

**Used by:**
- Layer 1 applications: training neural networks, running inference
- User code that imports `torch`, `tf`, or `keras`

## The Big Picture: What ML Frameworks Actually Do

Every ML framework, from PyTorch to JAX to TensorFlow, does exactly four things:

### 1. Tensor Operations

A tensor is an n-dimensional array with math operations. A scalar is a 0-D
tensor, a vector is 1-D, a matrix is 2-D, and deep learning uses 3-D and 4-D
tensors constantly (batches of images = 4-D: `[batch, channels, height, width]`).

Under the hood, every tensor operation maps to BLAS:
- `tensor + tensor` → element-wise `saxpy` with α=1
- `tensor @ tensor` → `sgemm` (matrix multiply)
- `tensor * scalar` → `sscal`
- `softmax(tensor)` → BLAS ML extension

### 2. Automatic Differentiation (Autograd)

This is the magic that makes deep learning work. When you compute:

```
y = W @ x + b        # forward pass
loss = mse(y, target)
loss.backward()       # backward pass — computes all gradients automatically
```

The framework secretly builds a **computational graph** during the forward pass:

```
x ──→ [matmul] ──→ [add] ──→ [mse] ──→ loss
W ──↗            b ──↗    target ──↗
```

Then `backward()` walks this graph in reverse, applying the **chain rule** at
each node to compute ∂loss/∂W and ∂loss/∂b. This is backpropagation — but
the framework does it automatically for any computation graph.

### 3. Neural Network Layers

Layers are reusable building blocks with learnable parameters:

- `Linear(in, out)` — holds a weight matrix W and bias b, computes `W @ x + b`
- `Conv2d(in_ch, out_ch, kernel)` — convolution with learnable filters
- `LayerNorm(dim)` — normalization with learnable γ and β
- `ReLU()`, `Softmax()` — stateless activation functions

### 4. Optimizers

After `backward()` computes gradients, optimizers update the parameters:

- **SGD**: `w = w - lr * grad` (literally BLAS `saxpy(-lr, grad, w)`)
- **Adam**: Adaptive learning rates using first/second moment estimates
- **RMSprop**: Running average of squared gradients
- **AdamW**: Adam with decoupled weight decay

## Architecture: Shared Engine, Three API Skins

```
┌─────────────────────────────────────────────────────┐
│                    User Code                         │
│  import torch / import tf / import keras             │
├──────────┬──────────────┬───────────────────────────┤
│  torch   │  tensorflow  │         keras              │
│  Module  │  GradientTape│  Sequential / Functional   │
│  nn.*    │  tf.keras.*  │  model.fit() / predict()   │
├──────────┴──────────────┴───────────────────────────┤
│              Shared Tensor Engine                     │
│  ┌──────────┐  ┌───────────┐  ┌──────────────────┐  │
│  │  Tensor   │  │  Autograd  │  │  Device Manager  │  │
│  │  (n-dim)  │  │  (comp.   │  │  (cpu/cuda/      │  │
│  │           │  │   graph)  │  │   metal/...)     │  │
│  └──────────┘  └───────────┘  └──────────────────┘  │
├─────────────────────────────────────────────────────┤
│              BLAS Library (Layer 3)                   │
│  BlasBackend: CPU | CUDA | Metal | Vulkan | ...      │
└─────────────────────────────────────────────────────┘
```

The shared engine provides:
- **Tensor**: n-dimensional array wrapping BLAS Matrix/Vector
- **Autograd**: computational graph with automatic backward pass
- **Device**: backend selection (`"cpu"`, `"cuda"`, `"metal"`, etc.)
- **Parameter**: tensor subclass that tracks gradients for optimizer updates

Each framework API is a thin skin over this shared engine.

## Part 1: Shared Tensor Engine

### Tensor

The fundamental data structure. Wraps BLAS `Matrix` for 2-D data and extends
to arbitrary dimensions.

```python
class Tensor:
    """
    ================================================================
    N-DIMENSIONAL ARRAY WITH AUTOMATIC DIFFERENTIATION
    ================================================================

    A Tensor is the central object in all three frameworks. It holds:

    1. data — the actual numbers (flat list, row-major, like BLAS Matrix)
    2. shape — dimensions tuple, e.g. (2, 3) for a 2×3 matrix
    3. requires_grad — whether to track this tensor in the computation graph
    4. grad — accumulated gradients after backward()
    5. grad_fn — the autograd Function that created this tensor
    6. device — which backend to use ("cpu", "cuda", "metal", etc.)

    Storage is always a flat list[float] in row-major order, matching
    BLAS Matrix format. A (2, 3, 4) tensor has 24 elements stored as:
    [t[0,0,0], t[0,0,1], ..., t[0,0,3], t[0,1,0], ..., t[1,2,3]]

    This means any 2-D tensor can be directly passed to BLAS sgemm
    without copying.
    ================================================================
    """

    # --- Construction ---

    data: list[float]           # Flat storage (row-major)
    shape: tuple[int, ...]      # Dimensions
    requires_grad: bool         # Track in computation graph?
    grad: Tensor | None         # Accumulated gradients
    grad_fn: Function | None    # Autograd node that created this
    device: str                 # "cpu", "cuda", "metal", etc.

    # --- Factory Methods ---

    @staticmethod
    def zeros(*shape, requires_grad=False, device="cpu") -> Tensor
    @staticmethod
    def ones(*shape, requires_grad=False, device="cpu") -> Tensor
    @staticmethod
    def randn(*shape, requires_grad=False, device="cpu") -> Tensor
    @staticmethod
    def from_list(data, shape=None, requires_grad=False) -> Tensor

    # --- Arithmetic (returns new Tensor, tracks grad if needed) ---

    def __add__(self, other: Tensor | float) -> Tensor
    def __sub__(self, other: Tensor | float) -> Tensor
    def __mul__(self, other: Tensor | float) -> Tensor
    def __truediv__(self, other: Tensor | float) -> Tensor
    def __neg__(self) -> Tensor
    def __matmul__(self, other: Tensor) -> Tensor     # @ operator → sgemm
    def __pow__(self, exponent: float) -> Tensor

    # --- Shape Operations ---

    def reshape(self, *shape) -> Tensor
    def transpose(self, dim0: int, dim1: int) -> Tensor
    def t(self) -> Tensor                              # 2-D transpose shortcut
    def flatten(self, start_dim=0, end_dim=-1) -> Tensor
    def unsqueeze(self, dim: int) -> Tensor
    def squeeze(self, dim: int | None = None) -> Tensor

    # --- Reduction ---

    def sum(self, dim=None, keepdim=False) -> Tensor
    def mean(self, dim=None, keepdim=False) -> Tensor
    def max(self, dim=None) -> Tensor
    def min(self, dim=None) -> Tensor

    # --- Comparison ---

    def eq(self, other: Tensor | float) -> Tensor
    def gt(self, other: Tensor | float) -> Tensor
    def lt(self, other: Tensor | float) -> Tensor

    # --- Autograd ---

    def backward(self, gradient: Tensor | None = None) -> None
        """
        ================================================================
        REVERSE-MODE AUTOMATIC DIFFERENTIATION
        ================================================================

        Computes gradients of this tensor with respect to all leaf tensors
        in the computation graph that have requires_grad=True.

        Algorithm:
        1. If this is a scalar, gradient defaults to ones(1)
        2. Topological sort of the computation graph
        3. Walk in reverse order, calling each node's backward function
        4. Accumulate gradients in each leaf tensor's .grad attribute

        After calling backward(), every Parameter in the graph has its
        .grad field populated, ready for optimizer.step().
        ================================================================
        """

    def detach(self) -> Tensor
        """Return a new tensor detached from the computation graph."""

    def item(self) -> float
        """Extract a scalar value from a single-element tensor."""

    # --- Device ---

    def to(self, device: str) -> Tensor
        """Move tensor to a different backend device."""

    # --- BLAS Bridge ---

    def _to_blas_matrix(self) -> blas_library.Matrix
    def _to_blas_vector(self) -> blas_library.Vector

    @staticmethod
    def _from_blas_matrix(m: blas_library.Matrix, ...) -> Tensor
```

### Autograd Engine

The computational graph that makes `backward()` work.

```python
class Function:
    """
    ================================================================
    BASE CLASS FOR AUTOGRAD OPERATIONS
    ================================================================

    Every differentiable operation (add, matmul, relu, etc.) is a
    Function subclass with two methods:

    - forward(*inputs) → output tensor(s)
    - backward(grad_output) → grad_input(s)

    During forward(), the Function saves whatever it needs for backward()
    (input tensors, intermediate values). The autograd engine calls
    backward() during loss.backward().

    Example — addition:
      forward(a, b) → a + b
      backward(grad) → (grad, grad)    # gradient flows equally to both

    Example — matmul:
      forward(A, B) → A @ B
      backward(grad) → (grad @ B.T, A.T @ grad)  # chain rule for matrices
    ================================================================
    """

    def forward(self, *inputs: Tensor) -> Tensor: ...
    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]: ...

# --- Built-in Functions (each wraps a BLAS call) ---

class AddFunction(Function):
    """forward: C = A + B  |  backward: (grad, grad)"""

class SubFunction(Function):
    """forward: C = A - B  |  backward: (grad, -grad)"""

class MulFunction(Function):
    """forward: C = A * B (element-wise)  |  backward: (grad*B, grad*A)"""

class MatMulFunction(Function):
    """
    forward: C = A @ B  →  blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    backward: (grad @ B.T, A.T @ grad)
    Both backward ops also dispatch to sgemm with appropriate transposes.
    """

class PowFunction(Function):
    """forward: y = x^n  |  backward: n * x^(n-1) * grad"""

class SumFunction(Function):
    """forward: y = Σx  |  backward: broadcast grad to input shape"""

class MeanFunction(Function):
    """forward: y = mean(x)  |  backward: grad / n"""

class TransposeFunction(Function):
    """forward: y = x.T  |  backward: grad.T"""

class ReshapeFunction(Function):
    """forward: y = x.reshape(shape)  |  backward: grad.reshape(original_shape)"""

class ReLUFunction(Function):
    """
    forward: y = max(0, x)  →  blas.relu(x)
    backward: grad * (x > 0)
    """

class SigmoidFunction(Function):
    """
    forward: y = σ(x) = 1/(1+exp(-x))  →  blas.sigmoid(x)
    backward: grad * y * (1 - y)
    """

class TanhFunction(Function):
    """
    forward: y = tanh(x)
    backward: grad * (1 - y²)
    """

class SoftmaxFunction(Function):
    """
    forward: y = softmax(x, dim)  →  blas.softmax(x)
    backward: y * (grad - sum(grad * y, dim))
    """

class GELUFunction(Function):
    """
    forward: y = gelu(x)  →  blas.gelu(x)
    backward: Approximated derivative of GELU
    """

class LogFunction(Function):
    """forward: y = log(x)  |  backward: grad / x"""

class ExpFunction(Function):
    """forward: y = exp(x)  |  backward: grad * y"""

class NegFunction(Function):
    """forward: y = -x  |  backward: -grad"""

class DivFunction(Function):
    """forward: y = a / b  |  backward: (grad/b, -grad*a/b²)"""
```

### Parameter

A tensor that is a learnable weight. Always has `requires_grad=True`.

```python
class Parameter(Tensor):
    """
    ================================================================
    LEARNABLE PARAMETER — A TENSOR THAT ACCUMULATES GRADIENTS
    ================================================================

    Parameters are tensors that:
    1. Always have requires_grad=True
    2. Are registered with nn.Module for optimizer access
    3. Accumulate gradients across backward() calls
    4. Get updated by optimizer.step()

    Usage:
        self.weight = Parameter(Tensor.randn(in_features, out_features))
        self.bias = Parameter(Tensor.zeros(out_features))
    ================================================================
    """
    def __init__(self, data: Tensor) -> None: ...
```

### Device Manager

Selects which BLAS backend to use.

```python
class DeviceManager:
    """
    ================================================================
    BACKEND SELECTION AND TENSOR PLACEMENT
    ================================================================

    Maps device strings to BLAS backends:
        "cpu"    → CpuBlas
        "cuda"   → CudaBlas
        "metal"  → MetalBlas
        "vulkan" → VulkanBlas
        "opencl" → OpenClBlas
        "webgpu" → WebGpuBlas
        "opengl" → OpenGlBlas

    Uses blas-library's BackendRegistry under the hood.
    ================================================================
    """

    def get_backend(self, device: str) -> BlasBackend: ...
    def get_default_device(self) -> str: ...
    def set_default_device(self, device: str) -> None: ...
```

## Part 2: PyTorch API (`torch` module)

PyTorch's philosophy: **eager execution, Pythonic API, research-friendly.**
The programmer writes normal Python code and gradients just work.

### torch — Top-Level Functions

```python
# === Tensor Creation ===
torch.tensor(data, requires_grad=False, device="cpu") -> Tensor
torch.zeros(*size, requires_grad=False, device="cpu") -> Tensor
torch.ones(*size, requires_grad=False, device="cpu") -> Tensor
torch.randn(*size, requires_grad=False, device="cpu") -> Tensor
torch.eye(n, device="cpu") -> Tensor
torch.arange(start, end, step=1, device="cpu") -> Tensor
torch.linspace(start, end, steps, device="cpu") -> Tensor

# === Math Ops ===
torch.matmul(a, b) -> Tensor          # → sgemm
torch.add(a, b) -> Tensor
torch.sum(x, dim=None) -> Tensor
torch.mean(x, dim=None) -> Tensor
torch.max(x, dim=None) -> Tensor
torch.clamp(x, min, max) -> Tensor
torch.abs(x) -> Tensor
torch.sqrt(x) -> Tensor
torch.exp(x) -> Tensor
torch.log(x) -> Tensor

# === Context Managers ===
torch.no_grad()     # Disable gradient tracking (inference mode)
torch.enable_grad() # Re-enable gradient tracking
```

### torch.nn — Neural Network Layers

```python
class Module:
    """
    ================================================================
    BASE CLASS FOR ALL NEURAL NETWORK LAYERS
    ================================================================

    Every layer (Linear, Conv2d, etc.) subclasses Module. It provides:

    1. parameter() — iterate over all learnable Parameters
    2. forward() — the computation (subclasses override this)
    3. __call__() — calls forward() with autograd tracking
    4. train() / eval() — toggle training/inference mode
    5. to(device) — move all parameters to a device
    6. zero_grad() — reset all parameter gradients to zero
    7. state_dict() / load_state_dict() — serialization

    Usage:
        class MyModel(torch.nn.Module):
            def __init__(self):
                super().__init__()
                self.linear1 = torch.nn.Linear(784, 128)
                self.relu = torch.nn.ReLU()
                self.linear2 = torch.nn.Linear(128, 10)

            def forward(self, x):
                x = self.linear1(x)
                x = self.relu(x)
                x = self.linear2(x)
                return x
    ================================================================
    """

    def forward(self, *args) -> Tensor: ...
    def parameters(self) -> Iterator[Parameter]: ...
    def named_parameters(self) -> Iterator[tuple[str, Parameter]]: ...
    def train(self, mode=True) -> Module: ...
    def eval(self) -> Module: ...
    def to(self, device: str) -> Module: ...
    def zero_grad(self) -> None: ...
    def state_dict(self) -> dict: ...
    def load_state_dict(self, state: dict) -> None: ...

class Sequential(Module):
    """
    Chain layers sequentially: Sequential(Linear(784,128), ReLU(), Linear(128,10))
    forward() passes input through each layer in order.
    """
    def __init__(self, *layers: Module) -> None: ...
    def forward(self, x: Tensor) -> Tensor: ...

# === Layer Implementations ===

class Linear(Module):
    """
    y = x @ W.T + b
    Dispatches to: blas.sgemm(NO_TRANS, TRANS, 1.0, x, W, 0.0, C) then add b
    Parameters: weight (out × in), bias (out)
    """
    def __init__(self, in_features: int, out_features: int, bias: bool = True): ...

class Conv2d(Module):
    """
    2-D convolution over input (batch, in_ch, H, W).
    Dispatches to: blas.conv2d() or im2col + sgemm
    Parameters: weight (out_ch × in_ch × kH × kW), bias (out_ch)
    """
    def __init__(self, in_channels, out_channels, kernel_size,
                 stride=1, padding=0, bias=True): ...

class BatchNorm1d(Module):
    """Batch normalization. Dispatches to blas.batch_norm()."""
    def __init__(self, num_features, eps=1e-5, momentum=0.1): ...

class BatchNorm2d(Module):
    """Batch normalization for 4-D inputs (N, C, H, W)."""
    def __init__(self, num_features, eps=1e-5, momentum=0.1): ...

class LayerNorm(Module):
    """Layer normalization. Dispatches to blas.layer_norm()."""
    def __init__(self, normalized_shape, eps=1e-5): ...

class Dropout(Module):
    """Randomly zeroes elements during training. No-op during eval."""
    def __init__(self, p=0.5): ...

class Embedding(Module):
    """Lookup table: integer index → dense vector."""
    def __init__(self, num_embeddings: int, embedding_dim: int): ...

class Flatten(Module):
    """Reshape to 2-D: (batch, features)."""
    def __init__(self, start_dim=1, end_dim=-1): ...

# === Activation Functions (stateless) ===

class ReLU(Module):     # max(0, x) → blas.relu()
class GELU(Module):     # GELU(x) → blas.gelu()
class Sigmoid(Module):  # σ(x) → blas.sigmoid()
class Tanh(Module):     # tanh(x) → blas.tanh()
class Softmax(Module):  # softmax(x, dim) → blas.softmax()
class LogSoftmax(Module):

# === Loss Functions ===

class MSELoss(Module):
    """Mean Squared Error: Σ(pred - target)² / n"""
class CrossEntropyLoss(Module):
    """Combines LogSoftmax + NLLLoss. The standard classification loss."""
class BCELoss(Module):
    """Binary Cross Entropy for binary classification."""
class BCEWithLogitsLoss(Module):
    """BCE with built-in sigmoid — numerically stable."""
class NLLLoss(Module):
    """Negative Log Likelihood Loss."""
class L1Loss(Module):
    """Mean Absolute Error: Σ|pred - target| / n"""
```

### torch.nn.functional — Stateless Versions

```python
# Every nn.Module layer also has a functional version:
torch.nn.functional.relu(x) -> Tensor
torch.nn.functional.gelu(x) -> Tensor
torch.nn.functional.sigmoid(x) -> Tensor
torch.nn.functional.softmax(x, dim) -> Tensor
torch.nn.functional.log_softmax(x, dim) -> Tensor
torch.nn.functional.dropout(x, p=0.5, training=True) -> Tensor
torch.nn.functional.linear(x, weight, bias=None) -> Tensor
torch.nn.functional.conv2d(x, weight, bias=None, stride=1, padding=0) -> Tensor
torch.nn.functional.cross_entropy(input, target) -> Tensor
torch.nn.functional.mse_loss(input, target) -> Tensor
torch.nn.functional.binary_cross_entropy(input, target) -> Tensor
torch.nn.functional.layer_norm(x, normalized_shape, weight, bias) -> Tensor
torch.nn.functional.batch_norm(x, running_mean, running_var, weight, bias) -> Tensor
```

### torch.optim — Optimizers

```python
class Optimizer:
    """
    ================================================================
    BASE CLASS FOR ALL OPTIMIZERS
    ================================================================

    Manages parameter groups and provides step() / zero_grad().

    Usage:
        optimizer = torch.optim.Adam(model.parameters(), lr=0.001)

        for batch in dataloader:
            optimizer.zero_grad()           # Reset gradients
            output = model(batch.x)         # Forward pass
            loss = criterion(output, batch.y)
            loss.backward()                 # Compute gradients
            optimizer.step()                # Update parameters
    ================================================================
    """
    def __init__(self, params, lr: float, **kwargs): ...
    def step(self) -> None: ...
    def zero_grad(self) -> None: ...
    def state_dict(self) -> dict: ...
    def load_state_dict(self, state: dict) -> None: ...

class SGD(Optimizer):
    """
    Stochastic Gradient Descent: w = w - lr * grad
    With momentum: v = μv + grad; w = w - lr * v
    Dispatches to: blas.saxpy(-lr, grad, w) for vanilla SGD
    """
    def __init__(self, params, lr: float, momentum=0.0, weight_decay=0.0): ...

class Adam(Optimizer):
    """
    Adaptive Moment Estimation.
    Tracks first moment (mean) and second moment (variance) of gradients.
    m = β1*m + (1-β1)*grad
    v = β2*v + (1-β2)*grad²
    w = w - lr * m̂ / (√v̂ + ε)
    """
    def __init__(self, params, lr=0.001, betas=(0.9, 0.999), eps=1e-8,
                 weight_decay=0.0): ...

class AdamW(Optimizer):
    """Adam with decoupled weight decay regularization."""
    def __init__(self, params, lr=0.001, betas=(0.9, 0.999), eps=1e-8,
                 weight_decay=0.01): ...

class RMSprop(Optimizer):
    """
    Root Mean Square Propagation.
    v = α*v + (1-α)*grad²
    w = w - lr * grad / (√v + ε)
    """
    def __init__(self, params, lr=0.01, alpha=0.99, eps=1e-8,
                 weight_decay=0.0, momentum=0.0): ...
```

### torch.utils.data — Data Loading

```python
class Dataset:
    """Abstract base. Subclass and implement __len__ and __getitem__."""
    def __len__(self) -> int: ...
    def __getitem__(self, idx: int) -> tuple: ...

class TensorDataset(Dataset):
    """Wraps tensors into a dataset. Each sample is a tuple of tensor slices."""
    def __init__(self, *tensors: Tensor): ...

class DataLoader:
    """
    Iterates over a Dataset in batches.
    Handles shuffling, batching, and optional dropping of last incomplete batch.
    """
    def __init__(self, dataset: Dataset, batch_size=1,
                 shuffle=False, drop_last=False): ...
    def __iter__(self) -> Iterator[tuple[Tensor, ...]]: ...
    def __len__(self) -> int: ...
```

## Part 3: TensorFlow API (`tf` module)

TensorFlow's philosophy: **production-ready, graph optimization, explicit control.**
Uses `GradientTape` for explicit gradient tracking.

### tf — Top-Level

```python
# === Tensor Creation ===
tf.constant(value, dtype=None) -> Tensor
tf.Variable(initial_value, trainable=True, name=None) -> Variable
tf.zeros(shape) -> Tensor
tf.ones(shape) -> Tensor
tf.random.normal(shape, mean=0.0, stddev=1.0) -> Tensor
tf.eye(num_rows) -> Tensor
tf.range(start, limit, delta=1) -> Tensor

# === Math Ops ===
tf.matmul(a, b) -> Tensor
tf.add(a, b) -> Tensor
tf.multiply(a, b) -> Tensor
tf.reduce_sum(x, axis=None) -> Tensor
tf.reduce_mean(x, axis=None) -> Tensor
tf.reduce_max(x, axis=None) -> Tensor
tf.nn.relu(x) -> Tensor
tf.nn.sigmoid(x) -> Tensor
tf.nn.softmax(x, axis=-1) -> Tensor
tf.nn.gelu(x) -> Tensor
tf.math.log(x) -> Tensor
tf.math.exp(x) -> Tensor
tf.math.sqrt(x) -> Tensor
tf.reshape(x, shape) -> Tensor
tf.transpose(x, perm=None) -> Tensor
tf.concat(values, axis) -> Tensor
tf.clip_by_value(x, min, max) -> Tensor

# === Variable (mutable, trainable tensor) ===
class Variable(Tensor):
    """
    A tf.Variable is a mutable tensor with a name. Unlike tf.constant,
    variables can be updated in-place via assign() and assign_sub().
    Variables with trainable=True are tracked by GradientTape.
    """
    def assign(self, value: Tensor) -> None: ...
    def assign_sub(self, delta: Tensor) -> None: ...
    def assign_add(self, delta: Tensor) -> None: ...
```

### tf.GradientTape — Explicit Gradient Tracking

```python
class GradientTape:
    """
    ================================================================
    EXPLICIT GRADIENT TRACKING CONTEXT MANAGER
    ================================================================

    TensorFlow's approach to autograd. Unlike PyTorch (where backward()
    is called on the loss), TF uses an explicit tape context:

        x = tf.Variable([1.0, 2.0, 3.0])
        with tf.GradientTape() as tape:
            y = x * x
            loss = tf.reduce_sum(y)
        grads = tape.gradient(loss, [x])  # [2.0, 4.0, 6.0]

    The tape records operations on watched variables inside the `with`
    block. Then tape.gradient() computes gradients via reverse-mode AD.

    Key difference from PyTorch:
    - PyTorch: loss.backward() is implicit, modifies tensor.grad in-place
    - TensorFlow: tape.gradient(loss, vars) is explicit, returns grad list
    ================================================================
    """
    def __enter__(self) -> GradientTape: ...
    def __exit__(self, *args) -> None: ...
    def watch(self, tensor: Tensor) -> None: ...
    def gradient(self, target: Tensor, sources: list[Variable]) -> list[Tensor]: ...
```

### tf.keras — Layers, Models, Optimizers

```python
# === Layers ===
class tf.keras.layers.Dense(units, activation=None, use_bias=True):
    """Fully connected layer. Equivalent to torch.nn.Linear."""

class tf.keras.layers.Conv2D(filters, kernel_size, strides=1,
                              padding="valid", activation=None):
    """2-D convolution. Equivalent to torch.nn.Conv2d."""

class tf.keras.layers.BatchNormalization(epsilon=1e-3, momentum=0.99):
class tf.keras.layers.LayerNormalization(epsilon=1e-6):
class tf.keras.layers.Dropout(rate=0.5):
class tf.keras.layers.Flatten():
class tf.keras.layers.Embedding(input_dim, output_dim):
class tf.keras.layers.ReLU():
class tf.keras.layers.Softmax(axis=-1):

# === Activations (strings or functions) ===
tf.keras.activations.relu(x) -> Tensor
tf.keras.activations.sigmoid(x) -> Tensor
tf.keras.activations.softmax(x, axis=-1) -> Tensor
tf.keras.activations.tanh(x) -> Tensor
tf.keras.activations.gelu(x) -> Tensor

# === Models ===
class tf.keras.Sequential(layers=None):
    """Stack layers linearly. Same concept as torch.nn.Sequential."""
    def add(self, layer) -> None: ...
    def compile(self, optimizer, loss, metrics=None) -> None: ...
    def fit(self, x, y, epochs=1, batch_size=32, validation_data=None,
            callbacks=None, verbose=1) -> History: ...
    def evaluate(self, x, y, batch_size=32) -> tuple[float, ...]: ...
    def predict(self, x, batch_size=32) -> Tensor: ...
    def summary(self) -> None: ...

class tf.keras.Model:
    """
    Functional API model. Connect layers like a graph:
        inputs = tf.keras.Input(shape=(784,))
        x = tf.keras.layers.Dense(128, activation='relu')(inputs)
        outputs = tf.keras.layers.Dense(10, activation='softmax')(x)
        model = tf.keras.Model(inputs=inputs, outputs=outputs)
    """
    def compile(self, optimizer, loss, metrics=None) -> None: ...
    def fit(self, x, y, **kwargs) -> History: ...
    def evaluate(self, x, y, **kwargs) -> tuple[float, ...]: ...
    def predict(self, x, **kwargs) -> Tensor: ...
    def summary(self) -> None: ...

class tf.keras.Input:
    """Symbolic placeholder for model input shape."""
    def __init__(self, shape: tuple[int, ...], name=None): ...

# === Losses ===
tf.keras.losses.MeanSquaredError()
tf.keras.losses.BinaryCrossentropy(from_logits=False)
tf.keras.losses.CategoricalCrossentropy(from_logits=False)
tf.keras.losses.SparseCategoricalCrossentropy(from_logits=False)
tf.keras.losses.MeanAbsoluteError()

# === Optimizers ===
tf.keras.optimizers.SGD(learning_rate=0.01, momentum=0.0)
tf.keras.optimizers.Adam(learning_rate=0.001, beta_1=0.9, beta_2=0.999, epsilon=1e-7)
tf.keras.optimizers.RMSprop(learning_rate=0.001, rho=0.9, epsilon=1e-7)
tf.keras.optimizers.AdamW(learning_rate=0.001, weight_decay=0.01)

# === Metrics ===
tf.keras.metrics.Accuracy()
tf.keras.metrics.BinaryAccuracy()
tf.keras.metrics.CategoricalAccuracy()
tf.keras.metrics.MeanSquaredError()
tf.keras.metrics.MeanAbsoluteError()

# === Callbacks ===
tf.keras.callbacks.EarlyStopping(monitor="val_loss", patience=5)
tf.keras.callbacks.ModelCheckpoint(filepath, save_best_only=True)
tf.keras.callbacks.LearningRateScheduler(schedule)
tf.keras.callbacks.History()   # Returned by fit()

# === Data ===
tf.data.Dataset.from_tensor_slices(tensors) -> Dataset
```

## Part 4: Keras API (`keras` module)

Keras 3's philosophy: **multi-backend, highest abstraction, easiest to use.**
Keras can run on PyTorch, TensorFlow, or JAX as backends. In our case, it
runs on our shared tensor engine (which runs on BLAS).

### keras — Standalone High-Level API

```python
# keras mirrors tf.keras but is backend-agnostic

# === Layers (same API as tf.keras.layers) ===
keras.layers.Dense(units, activation=None, use_bias=True)
keras.layers.Conv2D(filters, kernel_size, strides=1, padding="valid")
keras.layers.BatchNormalization()
keras.layers.LayerNormalization()
keras.layers.Dropout(rate=0.5)
keras.layers.Flatten()
keras.layers.Embedding(input_dim, output_dim)
keras.layers.Input(shape)
keras.layers.MultiHeadAttention(num_heads, key_dim)

# === Models ===
keras.Sequential(layers=None)
keras.Model(inputs, outputs)

# === Training API (the killer feature) ===
model = keras.Sequential([
    keras.layers.Dense(128, activation="relu"),
    keras.layers.Dropout(0.2),
    keras.layers.Dense(10, activation="softmax"),
])

model.compile(
    optimizer="adam",              # String or optimizer instance
    loss="categorical_crossentropy",
    metrics=["accuracy"],
)

history = model.fit(
    x_train, y_train,
    epochs=10,
    batch_size=32,
    validation_split=0.2,
    callbacks=[keras.callbacks.EarlyStopping(patience=3)],
)

loss, accuracy = model.evaluate(x_test, y_test)
predictions = model.predict(x_new)

# === Functional API ===
inputs = keras.layers.Input(shape=(784,))
x = keras.layers.Dense(256, activation="relu")(inputs)
x = keras.layers.Dropout(0.3)(x)
x = keras.layers.Dense(128, activation="relu")(x)
outputs = keras.layers.Dense(10, activation="softmax")(x)
model = keras.Model(inputs=inputs, outputs=outputs)

# === Backend Selection ===
keras.backend.set_backend("torch")   # Use PyTorch engine
keras.backend.set_backend("tf")      # Use TensorFlow engine
keras.backend.set_backend("cpu")     # Direct to CPU BLAS
```

## Part 5: Framework Equivalence

The same model built three ways:

### PyTorch

```python
import torch
import torch.nn as nn
import torch.optim as optim

class MNISTClassifier(nn.Module):
    def __init__(self):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(784, 128),
            nn.ReLU(),
            nn.Dropout(0.2),
            nn.Linear(128, 10),
        )

    def forward(self, x):
        return self.net(x)

model = MNISTClassifier()
optimizer = optim.Adam(model.parameters(), lr=0.001)
criterion = nn.CrossEntropyLoss()

for epoch in range(10):
    for x_batch, y_batch in dataloader:
        optimizer.zero_grad()
        output = model(x_batch)
        loss = criterion(output, y_batch)
        loss.backward()
        optimizer.step()
```

### TensorFlow

```python
import tf

model = tf.keras.Sequential([
    tf.keras.layers.Dense(128, activation="relu"),
    tf.keras.layers.Dropout(0.2),
    tf.keras.layers.Dense(10),
])

optimizer = tf.keras.optimizers.Adam(learning_rate=0.001)
loss_fn = tf.keras.losses.SparseCategoricalCrossentropy(from_logits=True)

for epoch in range(10):
    for x_batch, y_batch in dataset:
        with tf.GradientTape() as tape:
            logits = model(x_batch)
            loss = loss_fn(y_batch, logits)
        grads = tape.gradient(loss, model.trainable_variables)
        optimizer.apply_gradients(zip(grads, model.trainable_variables))
```

### Keras

```python
import keras

model = keras.Sequential([
    keras.layers.Dense(128, activation="relu"),
    keras.layers.Dropout(0.2),
    keras.layers.Dense(10, activation="softmax"),
])

model.compile(
    optimizer="adam",
    loss="categorical_crossentropy",
    metrics=["accuracy"],
)

model.fit(x_train, y_train, epochs=10, batch_size=32, validation_split=0.2)
```

**Same neural network. Same BLAS operations. Three different APIs.**

## Package Structure

### Shared Engine (one package per language)

```
ml-framework-core/
├── src/
│   ├── tensor.py          # Tensor class with shape, device, grad
│   ├── autograd.py        # Function base, computation graph, backward()
│   ├── functions.py       # Built-in autograd Functions (Add, MatMul, ReLU...)
│   ├── parameter.py       # Parameter (tensor with requires_grad=True)
│   └── device.py          # DeviceManager — maps strings to BLAS backends
```

### PyTorch API

```
ml-framework-torch/
├── src/
│   ├── __init__.py        # torch.tensor(), torch.zeros(), etc.
│   ├── nn/
│   │   ├── module.py      # Module base class
│   │   ├── sequential.py  # Sequential container
│   │   ├── linear.py      # Linear layer
│   │   ├── conv.py        # Conv2d
│   │   ├── normalization.py  # BatchNorm, LayerNorm
│   │   ├── activation.py  # ReLU, GELU, Sigmoid, Softmax
│   │   ├── dropout.py     # Dropout
│   │   ├── loss.py        # CrossEntropyLoss, MSELoss, etc.
│   │   └── functional.py  # Stateless versions of all ops
│   ├── optim/
│   │   ├── optimizer.py   # Optimizer base
│   │   ├── sgd.py         # SGD
│   │   ├── adam.py         # Adam, AdamW
│   │   └── rmsprop.py     # RMSprop
│   └── utils/
│       └── data.py        # Dataset, DataLoader
```

### TensorFlow API

```
ml-framework-tf/
├── src/
│   ├── __init__.py        # tf.constant(), tf.Variable(), etc.
│   ├── nn.py              # tf.nn.relu(), tf.nn.softmax()
│   ├── math.py            # tf.math.log(), tf.math.exp()
│   ├── gradient_tape.py   # GradientTape context manager
│   ├── keras/
│   │   ├── layers.py      # Dense, Conv2D, BatchNorm, etc.
│   │   ├── models.py      # Sequential, Model, Input
│   │   ├── optimizers.py  # SGD, Adam, RMSprop, AdamW
│   │   ├── losses.py      # MSE, BCE, CCE, etc.
│   │   ├── metrics.py     # Accuracy, MSE, MAE
│   │   ├── callbacks.py   # EarlyStopping, ModelCheckpoint, History
│   │   └── activations.py # relu, sigmoid, softmax, gelu
│   └── data.py            # tf.data.Dataset
```

### Keras API

```
ml-framework-keras/
├── src/
│   ├── __init__.py        # keras.Sequential, keras.Model
│   ├── layers.py          # Dense, Conv2D, etc. (delegates to core)
│   ├── models.py          # Sequential, Model, Input
│   ├── optimizers.py      # SGD, Adam, etc.
│   ├── losses.py          # MSE, BCE, CCE
│   ├── metrics.py         # Accuracy, MSE
│   ├── callbacks.py       # EarlyStopping, etc.
│   ├── activations.py     # relu, sigmoid, etc.
│   └── backend.py         # set_backend(), get_backend()
```

## Cross-Language Implementation

| | Python | TypeScript | Rust | Go | Ruby |
|---|---|---|---|---|---|
| Shared Engine | `ml_framework_core` | `ml-framework-core` | `ml-framework-core` | `ml-framework-core` | `ml_framework_core` |
| PyTorch API | `ml_framework_torch` | `ml-framework-torch` | `ml-framework-torch` | `ml-framework-torch` | `ml_framework_torch` |
| TensorFlow API | `ml_framework_tf` | `ml-framework-tf` | `ml-framework-tf` | `ml-framework-tf` | `ml_framework_tf` |
| Keras API | `ml_framework_keras` | `ml-framework-keras` | `ml-framework-keras` | `ml-framework-keras` | `ml_framework_keras` |

Each language gets **4 packages**: core engine + 3 framework APIs.

## Implementation Order

1. **Spec** — This document (commit first)
2. **Shared Engine** (Python first) — Tensor, Autograd, Parameter, Device
3. **PyTorch API** (Python) — Module, nn layers, optimizers, data loading
4. **TensorFlow API** (Python) — Variable, GradientTape, tf.keras
5. **Keras API** (Python) — Sequential, compile/fit/predict
6. **Verification** — Train MNIST with all 3 frameworks, same results
7. **Port to TypeScript, Rust, Go, Ruby** — One language at a time

## BLAS Operations Needed (Already Available)

Every framework operation maps to existing BLAS:

| Framework Op | BLAS Call | Level |
|-------------|-----------|-------|
| `Linear` forward | `sgemm` | L3 |
| `Linear` backward (dW) | `sgemm` (with transpose) | L3 |
| `Linear` backward (dx) | `sgemm` (with transpose) | L3 |
| Weight update (SGD) | `saxpy(-lr, grad, w)` | L1 |
| Vector dot product | `sdot` | L1 |
| Norm computation | `snrm2` | L1 |
| Scale gradients | `sscal` | L1 |
| ReLU / GELU / Sigmoid | ML extensions | ext |
| Softmax | ML extension | ext |
| LayerNorm / BatchNorm | ML extensions | ext |
| Conv2d | ML extension (or im2col + sgemm) | ext/L3 |
| Attention | ML extension | ext |

## Additions Needed in Lower Layers

The following additions may be needed in the BLAS library (Layer 3):

| Operation | Why | BLAS Level |
|-----------|-----|-----------|
| `sger` (rank-1 update) | Outer product in attention backward pass | L2 |
| `strsm` (triangular solve) | Matrix inversion for some optimizers | L3 |
| `element_wise_mul` | Hadamard product for gradient masking | ext |
| `element_wise_div` | Adam optimizer: m / (sqrt(v) + eps) | ext |
| `sqrt` (element-wise) | Adam optimizer: sqrt(v) | ext |
| `exp` / `log` (element-wise) | Softmax backward, cross-entropy | ext |
| `clamp` (element-wise) | Gradient clipping, numerical stability | ext |
| `where` (conditional select) | Dropout mask, ReLU backward | ext |
| `broadcasting` | Adding bias to batched matmul output | core |

These should be added to the `MlBlasBackend` protocol as needed.

## Verification Plan

For each language, verify:

1. **Unit tests** — Each autograd Function tested independently
2. **Gradient checks** — Numerical gradient ≈ autograd gradient (finite differences)
3. **Framework equivalence** — Same model + data → same loss trajectory across all 3 APIs
4. **MNIST training** — All 3 frameworks train to >90% accuracy on MNIST
5. **Coverage** — 95%+ for core engine, 90%+ for framework APIs
