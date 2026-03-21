"""
================================================================
ML FRAMEWORK TORCH — PyTorch-COMPATIBLE API LAYER
================================================================

This package provides a PyTorch-compatible API built on top of
ml-framework-core. It implements the familiar PyTorch interfaces:

    import ml_framework_torch as torch

    # Create tensors (delegates to ml-framework-core Tensor)
    x = torch.tensor([[1.0, 2.0], [3.0, 4.0]])
    w = torch.randn(4, 2, requires_grad=True)

    # Build neural networks
    model = torch.nn.Sequential(
        torch.nn.Linear(2, 16),
        torch.nn.ReLU(),
        torch.nn.Linear(16, 1),
    )

    # Train with optimizers
    optimizer = torch.optim.SGD(model.parameters(), lr=0.01)
    loss_fn = torch.nn.MSELoss()

    output = model(x)
    loss = loss_fn(output, target)
    loss.backward()
    optimizer.step()

=== Architecture ===

This package is a THIN WRAPPER. Almost all computation is done by
ml-framework-core. This layer adds:

1. nn.Module — base class for layers with parameter registration
2. nn.Linear, nn.ReLU, etc. — standard neural network layers
3. nn.MSELoss, nn.CrossEntropyLoss — loss functions
4. optim.SGD, optim.Adam — parameter update algorithms
5. utils.data — Dataset, DataLoader for batch training

Think of it like this:
    ml-framework-core = the engine (tensors, autograd, math)
    ml-framework-torch = the car (layers, optimizers, training loop)
================================================================
"""

from ml_framework_core import Parameter, Tensor, is_grad_enabled, no_grad

# =====================================================================
# Top-level tensor creation functions
# =====================================================================
# These mirror PyTorch's torch.tensor(), torch.zeros(), etc.
# They're thin wrappers around ml-framework-core's Tensor factory methods.


def tensor(
    data: list,
    requires_grad: bool = False,
    device: str = "cpu",
) -> Tensor:
    """Create a tensor from a (possibly nested) Python list.

    This is the PyTorch-style entry point for creating tensors:
        x = torch.tensor([[1.0, 2.0], [3.0, 4.0]])

    Under the hood, it delegates to Tensor.from_list() which
    flattens the nested list and infers the shape.
    """
    return Tensor.from_list(data, requires_grad=requires_grad, device=device)


def zeros(
    *shape: int,
    requires_grad: bool = False,
    device: str = "cpu",
) -> Tensor:
    """Create a tensor filled with zeros.

    Example: torch.zeros(2, 3) → 2×3 tensor of zeros
    """
    return Tensor.zeros(*shape, requires_grad=requires_grad, device=device)


def ones(
    *shape: int,
    requires_grad: bool = False,
    device: str = "cpu",
) -> Tensor:
    """Create a tensor filled with ones.

    Example: torch.ones(3, 4) → 3×4 tensor of ones
    """
    return Tensor.ones(*shape, requires_grad=requires_grad, device=device)


def randn(
    *shape: int,
    requires_grad: bool = False,
    device: str = "cpu",
) -> Tensor:
    """Create a tensor with random normal values (mean=0, std=1).

    Example: torch.randn(2, 3) → 2×3 tensor of random values
    """
    return Tensor.randn(*shape, requires_grad=requires_grad, device=device)


def eye(
    n: int,
    requires_grad: bool = False,
    device: str = "cpu",
) -> Tensor:
    """Create an n×n identity matrix.

    Example: torch.eye(3) → 3×3 identity matrix
    """
    return Tensor.eye(n, requires_grad=requires_grad, device=device)


def arange(
    start: float,
    end: float,
    step: float = 1.0,
    requires_grad: bool = False,
    device: str = "cpu",
) -> Tensor:
    """Create a 1-D tensor with values from start to end (exclusive).

    Example: torch.arange(0, 5) → Tensor([0, 1, 2, 3, 4])
    """
    return Tensor.arange(start, end, step, requires_grad=requires_grad, device=device)


def full(
    shape: tuple[int, ...],
    fill_value: float,
    requires_grad: bool = False,
    device: str = "cpu",
) -> Tensor:
    """Create a tensor filled with a constant value.

    Example: torch.full((2, 3), 7.0) → 2×3 tensor of 7.0s
    """
    return Tensor.full(shape, fill_value, requires_grad=requires_grad, device=device)


# =====================================================================
# Re-export subpackages for PyTorch-style namespace
# =====================================================================
# Users expect: torch.nn.Linear, torch.optim.SGD, etc.

from ml_framework_torch import nn, optim  # noqa: E402
from ml_framework_torch.utils import data  # noqa: E402

__all__ = [
    # Tensor creation
    "tensor",
    "zeros",
    "ones",
    "randn",
    "eye",
    "arange",
    "full",
    # Re-exports from core
    "Tensor",
    "Parameter",
    "no_grad",
    "is_grad_enabled",
    # Subpackages
    "nn",
    "optim",
    "data",
]
