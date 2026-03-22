"""
================================================================
TENSOR — N-DIMENSIONAL ARRAY WITH AUTOMATIC DIFFERENTIATION
================================================================

This is the central data structure of every ML framework. A Tensor is:

1. A container for numbers (like a matrix, but any number of dimensions)
2. Aware of its computation history (for automatic gradient computation)
3. Tied to a device (CPU, CUDA, Metal, etc.) for hardware acceleration

=== How It Works ===

Every Tensor stores:
- data: flat list of floats in row-major order (matches BLAS format)
- shape: tuple of dimension sizes, e.g. (2, 3) for a 2×3 matrix
- requires_grad: if True, operations on this tensor build a computation graph
- grad: after backward(), this holds ∂loss/∂(this tensor)
- _grad_fn: the autograd Function that created this tensor (or None for leaves)

Example:
    x = Tensor.from_list([1.0, 2.0, 3.0], shape=(3,), requires_grad=True)
    y = x * 2.0      # y.data = [2, 4, 6], y._grad_fn = MulFunction
    z = y.sum()       # z.data = [12], z._grad_fn = SumFunction
    z.backward()      # Walks graph: Sum → Mul → x
    print(x.grad)     # Tensor([2.0, 2.0, 2.0])  — ∂z/∂x = 2 everywhere

=== Storage Layout ===

Data is always a flat list[float] in row-major (C) order. This matches
BLAS Matrix format exactly, so 2-D tensors can be passed directly to
sgemm without copying.

A shape of (2, 3) means 2 rows, 3 columns:
    data = [a, b, c, d, e, f]
    represents: [[a, b, c],
                 [d, e, f]]

A shape of (2, 3, 4) means 2 "pages" of 3×4 matrices:
    Total elements = 2 * 3 * 4 = 24
    Index (i, j, k) maps to flat index: i*12 + j*4 + k

================================================================
"""

from __future__ import annotations

import math
import random
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .autograd import Function

# =========================================================================
# Helper: compute flat index from multi-dimensional indices
# =========================================================================


def _flat_index(indices: tuple[int, ...], shape: tuple[int, ...]) -> int:
    """Convert multi-dimensional indices to a flat index (row-major).

    Example:
        shape = (2, 3, 4)
        indices = (1, 2, 3)
        flat = 1*12 + 2*4 + 3 = 23
    """
    idx = 0
    stride = 1
    for i in range(len(shape) - 1, -1, -1):
        idx += indices[i] * stride
        stride *= shape[i]
    return idx


def _compute_strides(shape: tuple[int, ...]) -> tuple[int, ...]:
    """Compute row-major strides for a shape.

    Example:
        shape = (2, 3, 4) → strides = (12, 4, 1)
        shape = (3, 5) → strides = (5, 1)
    """
    if not shape:
        return ()
    strides = [1] * len(shape)
    for i in range(len(shape) - 2, -1, -1):
        strides[i] = strides[i + 1] * shape[i + 1]
    return tuple(strides)


def _numel(shape: tuple[int, ...]) -> int:
    """Total number of elements for a given shape."""
    result = 1
    for s in shape:
        result *= s
    return result


# =========================================================================
# Tensor class
# =========================================================================


class Tensor:
    """
    N-dimensional array with automatic differentiation support.

    See module docstring for full explanation.
    """

    __slots__ = (
        "data",
        "shape",
        "requires_grad",
        "grad",
        "_grad_fn",
        "_device",
    )

    def __init__(
        self,
        data: list[float],
        shape: tuple[int, ...],
        requires_grad: bool = False,
        device: str = "cpu",
    ) -> None:
        expected = _numel(shape)
        if len(data) != expected:
            raise ValueError(
                f"Data length {len(data)} doesn't match shape {shape} "
                f"(expected {expected} elements)"
            )
        self.data = data
        self.shape = shape
        self.requires_grad = requires_grad
        self.grad: Tensor | None = None
        self._grad_fn: Function | None = None
        self._device = device

    # =====================================================================
    # Properties
    # =====================================================================

    @property
    def device(self) -> str:
        return self._device

    @property
    def grad_fn(self) -> Function | None:
        return self._grad_fn

    @property
    def ndim(self) -> int:
        """Number of dimensions."""
        return len(self.shape)

    @property
    def numel(self) -> int:
        """Total number of elements."""
        return _numel(self.shape)

    @property
    def is_leaf(self) -> bool:
        """A tensor is a leaf if it was created by the user (not by an op)."""
        return self._grad_fn is None

    # =====================================================================
    # Factory methods
    # =====================================================================

    @staticmethod
    def zeros(
        *shape: int, requires_grad: bool = False, device: str = "cpu"
    ) -> Tensor:
        """Create a tensor filled with zeros."""
        n = _numel(shape)
        return Tensor([0.0] * n, shape, requires_grad, device)

    @staticmethod
    def ones(
        *shape: int, requires_grad: bool = False, device: str = "cpu"
    ) -> Tensor:
        """Create a tensor filled with ones."""
        n = _numel(shape)
        return Tensor([1.0] * n, shape, requires_grad, device)

    @staticmethod
    def full(
        shape: tuple[int, ...],
        fill_value: float,
        requires_grad: bool = False,
        device: str = "cpu",
    ) -> Tensor:
        """Create a tensor filled with a constant value."""
        n = _numel(shape)
        return Tensor([fill_value] * n, shape, requires_grad, device)

    @staticmethod
    def randn(
        *shape: int, requires_grad: bool = False, device: str = "cpu"
    ) -> Tensor:
        """Create a tensor with random normal values (mean=0, std=1).

        Uses Box-Muller transform for normal distribution.
        """
        n = _numel(shape)
        data: list[float] = []
        for _ in range(n):
            # Box-Muller transform: two uniform → two normal
            u1 = random.random()
            u2 = random.random()
            # Avoid log(0)
            while u1 == 0.0:
                u1 = random.random()
            data.append(math.sqrt(-2.0 * math.log(u1)) * math.cos(2.0 * math.pi * u2))
        return Tensor(data, shape, requires_grad, device)

    @staticmethod
    def eye(
        n: int, requires_grad: bool = False, device: str = "cpu"
    ) -> Tensor:
        """Create an n×n identity matrix."""
        data = [0.0] * (n * n)
        for i in range(n):
            data[i * n + i] = 1.0
        return Tensor(data, (n, n), requires_grad, device)

    @staticmethod
    def arange(
        start: float,
        end: float,
        step: float = 1.0,
        requires_grad: bool = False,
        device: str = "cpu",
    ) -> Tensor:
        """Create a 1-D tensor with values from start to end (exclusive)."""
        data: list[float] = []
        val = start
        while val < end:
            data.append(val)
            val += step
        return Tensor(data, (len(data),), requires_grad, device)

    @staticmethod
    def from_list(
        data: list,
        shape: tuple[int, ...] | None = None,
        requires_grad: bool = False,
        device: str = "cpu",
    ) -> Tensor:
        """Create a tensor from a (possibly nested) list.

        If shape is not given, it's inferred from the nesting structure.
        """
        flat, inferred_shape = _flatten_nested(data)
        if shape is None:
            shape = inferred_shape
        return Tensor(flat, shape, requires_grad, device)

    # =====================================================================
    # Arithmetic operators (each creates an autograd-tracked result)
    # =====================================================================

    def __add__(self, other: Tensor | float) -> Tensor:
        from .functions import AddFunction

        if isinstance(other, (int, float)):
            other = Tensor.full(self.shape, float(other), device=self._device)
        return AddFunction.apply(self, other)

    def __radd__(self, other: float) -> Tensor:
        return self.__add__(other)

    def __sub__(self, other: Tensor | float) -> Tensor:
        from .functions import SubFunction

        if isinstance(other, (int, float)):
            other = Tensor.full(self.shape, float(other), device=self._device)
        return SubFunction.apply(self, other)

    def __rsub__(self, other: float) -> Tensor:
        from .functions import SubFunction

        other_t = Tensor.full(self.shape, float(other), device=self._device)
        return SubFunction.apply(other_t, self)

    def __mul__(self, other: Tensor | float) -> Tensor:
        from .functions import MulFunction

        if isinstance(other, (int, float)):
            other = Tensor.full(self.shape, float(other), device=self._device)
        return MulFunction.apply(self, other)

    def __rmul__(self, other: float) -> Tensor:
        return self.__mul__(other)

    def __truediv__(self, other: Tensor | float) -> Tensor:
        from .functions import DivFunction

        if isinstance(other, (int, float)):
            other = Tensor.full(self.shape, float(other), device=self._device)
        return DivFunction.apply(self, other)

    def __neg__(self) -> Tensor:
        from .functions import NegFunction

        return NegFunction.apply(self)

    def __matmul__(self, other: Tensor) -> Tensor:
        from .functions import MatMulFunction

        return MatMulFunction.apply(self, other)

    def __pow__(self, exponent: float) -> Tensor:
        from .functions import PowFunction

        return PowFunction.apply(self, exponent)

    # =====================================================================
    # Shape operations
    # =====================================================================

    def reshape(self, *shape: int) -> Tensor:
        """Return a tensor with the same data but different shape."""
        from .functions import ReshapeFunction

        return ReshapeFunction.apply(self, shape)

    def transpose(self, dim0: int, dim1: int) -> Tensor:
        """Swap two dimensions."""
        from .functions import TransposeFunction

        return TransposeFunction.apply(self, dim0, dim1)

    def t(self) -> Tensor:
        """Transpose a 2-D tensor (shortcut for transpose(0, 1))."""
        if self.ndim != 2:
            raise ValueError(f"t() expects a 2-D tensor, got {self.ndim}-D")
        return self.transpose(0, 1)

    def flatten(self, start_dim: int = 0, end_dim: int = -1) -> Tensor:
        """Flatten dimensions from start_dim to end_dim into one."""
        if end_dim < 0:
            end_dim = self.ndim + end_dim
        new_shape = (
            list(self.shape[:start_dim])
            + [_numel(self.shape[start_dim : end_dim + 1])]
            + list(self.shape[end_dim + 1 :])
        )
        return self.reshape(*new_shape)

    def unsqueeze(self, dim: int) -> Tensor:
        """Add a dimension of size 1 at the given position."""
        if dim < 0:
            dim = self.ndim + 1 + dim
        new_shape = list(self.shape)
        new_shape.insert(dim, 1)
        return self.reshape(*new_shape)

    def squeeze(self, dim: int | None = None) -> Tensor:
        """Remove dimensions of size 1."""
        if dim is not None:
            if self.shape[dim] != 1:
                return self.reshape(*self.shape)
            new_shape = list(self.shape)
            new_shape.pop(dim)
            return self.reshape(*new_shape)
        new_shape = [s for s in self.shape if s != 1]
        if not new_shape:
            new_shape = [1]
        return self.reshape(*new_shape)

    # =====================================================================
    # Reduction operations
    # =====================================================================

    def sum(self, dim: int | None = None, keepdim: bool = False) -> Tensor:
        """Sum elements, optionally along a dimension."""
        from .functions import SumFunction

        return SumFunction.apply(self, dim, keepdim)

    def mean(self, dim: int | None = None, keepdim: bool = False) -> Tensor:
        """Mean of elements, optionally along a dimension."""
        from .functions import MeanFunction

        return MeanFunction.apply(self, dim, keepdim)

    # =====================================================================
    # Element-wise math
    # =====================================================================

    def exp(self) -> Tensor:
        from .functions import ExpFunction

        return ExpFunction.apply(self)

    def log(self) -> Tensor:
        from .functions import LogFunction

        return LogFunction.apply(self)

    def sqrt(self) -> Tensor:
        return self ** 0.5

    def abs(self) -> Tensor:
        from .functions import AbsFunction

        return AbsFunction.apply(self)

    def clamp(
        self,
        min_val: float | None = None,
        max_val: float | None = None,
    ) -> Tensor:
        from .functions import ClampFunction

        return ClampFunction.apply(self, min_val, max_val)

    # =====================================================================
    # Comparison (returns non-grad tensors)
    # =====================================================================

    def eq(self, other: Tensor | float) -> Tensor:
        if isinstance(other, (int, float)):
            data = [1.0 if x == other else 0.0 for x in self.data]
        else:
            data = [
                1.0 if a == b else 0.0
                for a, b in zip(self.data, other.data, strict=False)
            ]
        return Tensor(data, self.shape, device=self._device)

    def gt(self, other: Tensor | float) -> Tensor:
        if isinstance(other, (int, float)):
            data = [1.0 if x > other else 0.0 for x in self.data]
        else:
            data = [
                1.0 if a > b else 0.0
                for a, b in zip(self.data, other.data, strict=False)
            ]
        return Tensor(data, self.shape, device=self._device)

    def lt(self, other: Tensor | float) -> Tensor:
        if isinstance(other, (int, float)):
            data = [1.0 if x < other else 0.0 for x in self.data]
        else:
            data = [
                1.0 if a < b else 0.0
                for a, b in zip(self.data, other.data, strict=False)
            ]
        return Tensor(data, self.shape, device=self._device)

    # =====================================================================
    # Autograd
    # =====================================================================

    def backward(self, gradient: Tensor | None = None) -> None:
        """
        ================================================================
        REVERSE-MODE AUTOMATIC DIFFERENTIATION (BACKPROPAGATION)
        ================================================================

        This is the core algorithm that makes deep learning work.

        When you call loss.backward(), it:
        1. Starts from this tensor (usually a scalar loss)
        2. Builds a topological ordering of the computation graph
        3. Walks the graph in reverse, calling each node's backward()
        4. Accumulates gradients in each leaf tensor's .grad field

        After backward(), every Parameter (leaf tensor with requires_grad)
        has its .grad populated, ready for optimizer.step().

        The chain rule is applied automatically:
            If z = f(y) and y = g(x), then ∂z/∂x = ∂z/∂y · ∂y/∂x

        Each autograd Function knows its local derivative (∂y/∂x),
        and backward() chains them together.
        ================================================================
        """
        from .autograd import backward

        backward(self, gradient)

    def detach(self) -> Tensor:
        """Return a new tensor detached from the computation graph."""
        return Tensor(
            list(self.data), self.shape, requires_grad=False, device=self._device
        )

    def item(self) -> float:
        """Extract a scalar value from a single-element tensor."""
        if self.numel != 1:
            raise ValueError(
                f"item() only works on single-element tensors, got {self.numel}"
            )
        return self.data[0]

    # =====================================================================
    # Device management
    # =====================================================================

    def to(self, device: str) -> Tensor:
        """Move tensor to a different backend device."""
        if device == self._device:
            return self
        return Tensor(
            list(self.data),
            self.shape,
            self.requires_grad,
            device,
        )

    # =====================================================================
    # BLAS bridge
    # =====================================================================

    def _to_blas_matrix(self):
        """Convert 2-D tensor to BLAS Matrix for sgemm etc."""
        from blas_library import Matrix

        if self.ndim != 2:
            raise ValueError(f"Cannot convert {self.ndim}-D tensor to BLAS Matrix")
        return Matrix(data=list(self.data), rows=self.shape[0], cols=self.shape[1])

    def _to_blas_vector(self):
        """Convert 1-D tensor to BLAS Vector."""
        from blas_library import Vector

        if self.ndim != 1:
            raise ValueError(f"Cannot convert {self.ndim}-D tensor to BLAS Vector")
        return Vector(data=list(self.data), size=self.shape[0])

    @staticmethod
    def _from_blas_matrix(
        m,
        requires_grad: bool = False,
        device: str = "cpu",
    ) -> Tensor:
        """Create a 2-D tensor from a BLAS Matrix."""
        return Tensor(list(m.data), (m.rows, m.cols), requires_grad, device)

    @staticmethod
    def _from_blas_vector(
        v,
        requires_grad: bool = False,
        device: str = "cpu",
    ) -> Tensor:
        """Create a 1-D tensor from a BLAS Vector."""
        return Tensor(list(v.data), (v.size,), requires_grad, device)

    # =====================================================================
    # Display
    # =====================================================================

    def __repr__(self) -> str:
        grad_str = ", requires_grad=True" if self.requires_grad else ""
        fn_str = f", grad_fn={self._grad_fn}" if self._grad_fn else ""
        if self.numel <= 10:
            data_str = str(self.data)
        else:
            data_str = f"[{self.data[0]}, {self.data[1]}, ..., {self.data[-1]}]"
        return f"Tensor({data_str}, shape={self.shape}{grad_str}{fn_str})"

    def __len__(self) -> int:
        if not self.shape:
            raise TypeError("len() of a 0-d tensor")
        return self.shape[0]

    # =====================================================================
    # Indexing (basic integer indexing for first dimension)
    # =====================================================================

    def __getitem__(self, idx: int) -> Tensor:
        """Basic integer indexing along the first dimension."""
        if not isinstance(idx, int):
            raise TypeError("Only integer indexing is supported")
        if idx < 0:
            idx = self.shape[0] + idx
        if idx < 0 or idx >= self.shape[0]:
            raise IndexError(f"Index {idx} out of range for dim 0 size {self.shape[0]}")
        if self.ndim == 1:
            return Tensor([self.data[idx]], (1,), device=self._device)
        # Slice along first dimension
        stride = _numel(self.shape[1:])
        start = idx * stride
        end = start + stride
        new_shape = self.shape[1:]
        return Tensor(self.data[start:end], new_shape, device=self._device)


# =========================================================================
# Helper: flatten nested lists
# =========================================================================


def _flatten_nested(data: list | float) -> tuple[list[float], tuple[int, ...]]:
    """Flatten a (possibly nested) list and infer shape.

    Examples:
        [1, 2, 3] → ([1.0, 2.0, 3.0], (3,))
        [[1, 2], [3, 4]] → ([1.0, 2.0, 3.0, 4.0], (2, 2))
        [[[1], [2]], [[3], [4]]] → ([1.0, 2.0, 3.0, 4.0], (2, 2, 1))
    """
    if isinstance(data, (int, float)):
        return [float(data)], (1,)
    if not isinstance(data, list) or len(data) == 0:
        return [], (0,)
    if isinstance(data[0], (int, float)):
        return [float(x) for x in data], (len(data),)
    # Nested list — recurse
    sublists = [_flatten_nested(sub) for sub in data]
    sub_shape = sublists[0][1]
    flat = []
    for sub_flat, s in sublists:
        if s != sub_shape:
            raise ValueError(f"Inconsistent shapes: {s} vs {sub_shape}")
        flat.extend(sub_flat)
    return flat, (len(data), *sub_shape)
