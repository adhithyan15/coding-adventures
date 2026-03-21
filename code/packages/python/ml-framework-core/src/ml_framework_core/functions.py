"""
================================================================
BUILT-IN AUTOGRAD FUNCTIONS — THE BACKWARD RULES
================================================================

Each operation (add, mul, matmul, relu, etc.) has a Function subclass
that knows:
1. How to compute the forward pass
2. How to compute the backward pass (local gradients)

=== The Chain Rule in Action ===

If z = f(y) and y = g(x), the chain rule says:
    ∂z/∂x = ∂z/∂y · ∂y/∂x

Each Function provides ∂output/∂input (the local gradient).
The autograd engine chains them together by passing ∂loss/∂output
(grad_output) into backward(), which returns ∂loss/∂input.

=== Gradient Formulas ===

| Operation       | Forward          | Backward (∂output/∂input)          |
|-----------------|------------------|------------------------------------|
| add(a, b)       | a + b            | (grad, grad)                       |
| sub(a, b)       | a - b            | (grad, -grad)                      |
| mul(a, b)       | a * b            | (grad*b, grad*a)                   |
| div(a, b)       | a / b            | (grad/b, -grad*a/b²)              |
| neg(a)          | -a               | (-grad,)                           |
| pow(a, n)       | a^n              | (n * a^(n-1) * grad,)              |
| matmul(A, B)    | A @ B            | (grad @ B.T, A.T @ grad)          |
| sum(a)          | Σa               | broadcast grad                     |
| mean(a)         | Σa / n           | grad / n                           |
| relu(a)         | max(0, a)        | grad * (a > 0)                     |
| sigmoid(a)      | σ(a)             | grad * σ(a) * (1 - σ(a))          |
| tanh(a)         | tanh(a)          | grad * (1 - tanh²(a))             |
| exp(a)          | e^a              | grad * e^a                         |
| log(a)          | ln(a)            | grad / a                           |

================================================================
"""

from __future__ import annotations

import math

from .autograd import Function
from .tensor import Tensor, _compute_strides, _numel

# =========================================================================
# Arithmetic Functions
# =========================================================================


class AddFunction(Function):
    """Element-wise addition: C = A + B.

    Backward: ∂L/∂A = grad, ∂L/∂B = grad
    Both inputs receive the same gradient (addition distributes equally).
    """

    def forward(self, a: Tensor, b: Tensor) -> Tensor:
        self.save_for_backward(a, b)
        data = [x + y for x, y in zip(a.data, b.data, strict=False)]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        a, b = self.saved_tensors
        grad_a = grad_output if a.requires_grad else None
        grad_b = grad_output if b.requires_grad else None
        return (grad_a, grad_b)


class SubFunction(Function):
    """Element-wise subtraction: C = A - B.

    Backward: ∂L/∂A = grad, ∂L/∂B = -grad
    """

    def forward(self, a: Tensor, b: Tensor) -> Tensor:
        self.save_for_backward(a, b)
        data = [x - y for x, y in zip(a.data, b.data, strict=False)]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        a, b = self.saved_tensors
        grad_a = grad_output if a.requires_grad else None
        grad_b = (
            Tensor([-g for g in grad_output.data], grad_output.shape, device=b.device)
            if b.requires_grad
            else None
        )
        return (grad_a, grad_b)


class MulFunction(Function):
    """Element-wise multiplication: C = A * B.

    Backward: ∂L/∂A = grad * B, ∂L/∂B = grad * A
    This is the product rule: d(fg)/dx = f'g + fg'
    """

    def forward(self, a: Tensor, b: Tensor) -> Tensor:
        self.save_for_backward(a, b)
        data = [x * y for x, y in zip(a.data, b.data, strict=False)]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        a, b = self.saved_tensors
        grad_a = (
            Tensor(
                [g * bv for g, bv in zip(grad_output.data, b.data, strict=False)],
                a.shape,
                device=a.device,
            )
            if a.requires_grad
            else None
        )
        grad_b = (
            Tensor(
                [g * av for g, av in zip(grad_output.data, a.data, strict=False)],
                b.shape,
                device=b.device,
            )
            if b.requires_grad
            else None
        )
        return (grad_a, grad_b)


class DivFunction(Function):
    """Element-wise division: C = A / B.

    Backward: ∂L/∂A = grad / B, ∂L/∂B = -grad * A / B²
    This is the quotient rule.
    """

    def forward(self, a: Tensor, b: Tensor) -> Tensor:
        self.save_for_backward(a, b)
        data = [x / y for x, y in zip(a.data, b.data, strict=False)]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        a, b = self.saved_tensors
        grad_a = (
            Tensor(
                [g / bv for g, bv in zip(grad_output.data, b.data, strict=False)],
                a.shape,
                device=a.device,
            )
            if a.requires_grad
            else None
        )
        grad_b = (
            Tensor(
                [
                    -g * av / (bv * bv)
                    for g, av, bv in zip(grad_output.data, a.data, b.data, strict=False)
                ],
                b.shape,
                device=b.device,
            )
            if b.requires_grad
            else None
        )
        return (grad_a, grad_b)


class NegFunction(Function):
    """Negation: C = -A.

    Backward: ∂L/∂A = -grad
    """

    def forward(self, a: Tensor) -> Tensor:
        self.save_for_backward(a)
        data = [-x for x in a.data]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        return (
            Tensor(
                [-g for g in grad_output.data],
                grad_output.shape,
                device=grad_output.device,
            ),
        )


class PowFunction(Function):
    """Power: C = A ^ n (element-wise).

    Backward: ∂L/∂A = n * A^(n-1) * grad
    This is the power rule: d(x^n)/dx = n * x^(n-1)
    """

    def forward(self, a: Tensor, exponent: float) -> Tensor:
        self.save_for_backward(a)
        self.saved_metadata["exponent"] = exponent
        data = [x**exponent for x in a.data]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        n = self.saved_metadata["exponent"]
        pairs = zip(a.data, grad_output.data, strict=False)
        grad_a = Tensor(
            [n * (x ** (n - 1)) * g for x, g in pairs],
            a.shape,
            device=a.device,
        )
        return (grad_a,)


# =========================================================================
# Matrix multiplication
# =========================================================================


class MatMulFunction(Function):
    """Matrix multiplication: C = A @ B.

    For 2-D tensors:
        forward: C[i,j] = Σ_k A[i,k] * B[k,j]   → blas.sgemm()
        backward: ∂L/∂A = grad @ B.T
                  ∂L/∂B = A.T @ grad

    Why these formulas?
    - If C = A @ B, then dC/dA = I ⊗ B.T (Kronecker product)
    - Simplifies to: grad_A = grad_C @ B.T
    - Similarly: grad_B = A.T @ grad_C

    Both backward operations are also matrix multiplications,
    so they dispatch to sgemm too.
    """

    def forward(self, a: Tensor, b: Tensor) -> Tensor:
        self.save_for_backward(a, b)
        if a.ndim != 2 or b.ndim != 2:
            raise ValueError(
                f"matmul requires 2-D tensors, got {a.ndim}-D and {b.ndim}-D"
            )
        if a.shape[1] != b.shape[0]:
            raise ValueError(
                f"matmul shape mismatch: {a.shape} @ {b.shape}"
            )
        m, k = a.shape
        _, n = b.shape
        # Naive matmul (BLAS sgemm dispatch could be added here)
        data = [0.0] * (m * n)
        for i in range(m):
            for j in range(n):
                s = 0.0
                for p in range(k):
                    s += a.data[i * k + p] * b.data[p * n + j]
                data[i * n + j] = s
        return Tensor(data, (m, n), device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        a, b = self.saved_tensors
        grad_a = None
        grad_b = None

        if a.requires_grad:
            # grad_A = grad_output @ B.T
            grad_a = _matmul_2d(grad_output, _transpose_2d(b))
            grad_a._device = a.device

        if b.requires_grad:
            # grad_B = A.T @ grad_output
            grad_b = _matmul_2d(_transpose_2d(a), grad_output)
            grad_b._device = b.device

        return (grad_a, grad_b)


# =========================================================================
# Shape operations
# =========================================================================


class ReshapeFunction(Function):
    """Reshape: change the shape without changing data.

    Backward: reshape gradient back to original shape.
    """

    def forward(self, a: Tensor, new_shape: tuple[int, ...]) -> Tensor:
        self.save_for_backward(a)
        self.saved_metadata["original_shape"] = a.shape
        return Tensor(list(a.data), new_shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        original_shape = self.saved_metadata["original_shape"]
        return (
            Tensor(list(grad_output.data), original_shape, device=grad_output.device),
        )


class TransposeFunction(Function):
    """Transpose: swap two dimensions.

    For 2-D: swap rows and columns.
    Backward: transpose the gradient with the same dims.
    """

    def forward(self, a: Tensor, dim0: int, dim1: int) -> Tensor:
        self.save_for_backward(a)
        self.saved_metadata["dim0"] = dim0
        self.saved_metadata["dim1"] = dim1

        if a.ndim == 2 and dim0 == 0 and dim1 == 1:
            # Optimized 2-D transpose
            rows, cols = a.shape
            data = [0.0] * (rows * cols)
            for i in range(rows):
                for j in range(cols):
                    data[j * rows + i] = a.data[i * cols + j]
            return Tensor(data, (cols, rows), device=a.device)

        # General n-D transpose
        new_shape = list(a.shape)
        new_shape[dim0], new_shape[dim1] = new_shape[dim1], new_shape[dim0]

        old_strides = _compute_strides(a.shape)
        new_strides = list(old_strides)
        new_strides[dim0], new_strides[dim1] = new_strides[dim1], new_strides[dim0]

        n = _numel(a.shape)
        data = [0.0] * n
        result_strides = _compute_strides(tuple(new_shape))

        for flat_idx in range(n):
            # Compute multi-dim index in result
            remaining = flat_idx
            old_flat = 0
            for d in range(len(new_shape)):
                idx_d = remaining // result_strides[d]
                remaining %= result_strides[d]
                old_flat += idx_d * new_strides[d]
            data[flat_idx] = a.data[old_flat]

        return Tensor(data, tuple(new_shape), device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        dim0 = self.saved_metadata["dim0"]
        dim1 = self.saved_metadata["dim1"]
        # Transpose is its own inverse
        return (TransposeFunction.apply(grad_output, dim0, dim1),)


# =========================================================================
# Reduction operations
# =========================================================================


class SumFunction(Function):
    """Sum elements, optionally along a dimension.

    Backward: broadcast the gradient back to the input shape.
    If sum reduces dim d, grad_output has that dim removed (or kept as 1).
    We expand it back by repeating along that dimension.
    """

    def forward(
        self, a: Tensor, dim: int | None, keepdim: bool
    ) -> Tensor:
        self.save_for_backward(a)
        self.saved_metadata["dim"] = dim
        self.saved_metadata["keepdim"] = keepdim

        if dim is None:
            # Sum all elements → scalar
            total = sum(a.data)
            return Tensor([total], (1,), device=a.device)

        if dim < 0:
            dim = a.ndim + dim

        # Sum along a specific dimension
        list(a.shape)
        stride = _compute_strides(a.shape)
        a.shape[dim]

        if keepdim:
            result_shape = list(a.shape)
            result_shape[dim] = 1
        else:
            result_shape = [s for i, s in enumerate(a.shape) if i != dim]

        result_numel = _numel(tuple(result_shape)) if result_shape else 1
        result_data = [0.0] * result_numel

        # For each element in the result, sum over the reduced dimension
        for flat_idx in range(len(a.data)):
            # Compute multi-dim index
            remaining = flat_idx
            indices = []
            for d in range(a.ndim):
                indices.append(remaining // stride[d])
                remaining %= stride[d]

            # Compute result index (skip the reduced dimension)
            if keepdim:
                res_indices = list(indices)
                res_indices[dim] = 0
                res_shape_tuple = tuple(result_shape)
            else:
                res_indices = [idx for i, idx in enumerate(indices) if i != dim]
                res_shape_tuple = tuple(result_shape)

            if not res_shape_tuple:
                res_flat = 0
            else:
                res_strides = _compute_strides(res_shape_tuple)
                res_flat = sum(
                    idx * s for idx, s in zip(res_indices, res_strides, strict=False)
                )
            result_data[res_flat] += a.data[flat_idx]

        return Tensor(
            result_data, tuple(result_shape) if result_shape else (1,), device=a.device
        )

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        dim = self.saved_metadata["dim"]

        if dim is None:
            # Scalar sum: broadcast gradient to all elements
            return (
                Tensor(
                    [grad_output.data[0]] * a.numel,
                    a.shape,
                    device=a.device,
                ),
            )

        if dim < 0:
            dim = a.ndim + dim

        # Expand gradient along the reduced dimension
        grad_data = [0.0] * a.numel
        strides = _compute_strides(a.shape)

        for flat_idx in range(a.numel):
            remaining = flat_idx
            indices = []
            for d in range(a.ndim):
                indices.append(remaining // strides[d])
                remaining %= strides[d]

            # Index into grad_output (dimension dim is collapsed)
            grad_indices = [idx for i, idx in enumerate(indices) if i != dim]
            if not grad_indices:
                grad_flat = 0
            else:
                grad_shape = tuple(s for i, s in enumerate(a.shape) if i != dim)
                grad_strides = _compute_strides(grad_shape)
                grad_flat = sum(
                    idx * s for idx, s in zip(grad_indices, grad_strides, strict=False)
                )
            grad_data[flat_idx] = grad_output.data[grad_flat]

        return (Tensor(grad_data, a.shape, device=a.device),)


class MeanFunction(Function):
    """Mean of elements: sum / count.

    Backward: grad / count (each element contributes equally).
    """

    def forward(
        self, a: Tensor, dim: int | None, keepdim: bool
    ) -> Tensor:
        self.save_for_backward(a)
        self.saved_metadata["dim"] = dim

        if dim is None:
            total = sum(a.data)
            return Tensor([total / a.numel], (1,), device=a.device)

        # Use SumFunction then divide
        sum_result = SumFunction.apply(a, dim, keepdim)
        count = a.shape[dim] if dim < a.ndim else 1
        return Tensor(
            [x / count for x in sum_result.data],
            sum_result.shape,
            device=a.device,
        )

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        dim = self.saved_metadata["dim"]

        if dim is None:
            n = a.numel
            return (
                Tensor(
                    [grad_output.data[0] / n] * n,
                    a.shape,
                    device=a.device,
                ),
            )

        if dim < 0:
            dim = a.ndim + dim
        count = a.shape[dim]

        # Expand gradient (same as SumFunction) then divide by count
        sum_grad_fn = SumFunction()
        sum_grad_fn.saved_tensors = [a]
        sum_grad_fn.saved_metadata = {"dim": dim, "keepdim": False}
        (expanded,) = sum_grad_fn.backward(grad_output)
        return (
            Tensor(
                [x / count for x in expanded.data],
                expanded.shape,
                device=a.device,
            ),
        )


# =========================================================================
# Element-wise math functions
# =========================================================================


class ExpFunction(Function):
    """Exponential: y = e^x.

    Backward: ∂L/∂x = grad * e^x = grad * y
    The exponential is its own derivative!
    """

    def forward(self, a: Tensor) -> Tensor:
        data = [math.exp(x) for x in a.data]
        result = Tensor(data, a.shape, device=a.device)
        self.save_for_backward(a)
        self.saved_metadata["output"] = data
        return result

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        output = self.saved_metadata["output"]
        return (
            Tensor(
                [g * y for g, y in zip(grad_output.data, output, strict=False)],
                grad_output.shape,
                device=grad_output.device,
            ),
        )


class LogFunction(Function):
    """Natural log: y = ln(x).

    Backward: ∂L/∂x = grad / x
    """

    def forward(self, a: Tensor) -> Tensor:
        self.save_for_backward(a)
        data = [math.log(x) if x > 0 else float("-inf") for x in a.data]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        return (
            Tensor(
                [
                    g / x if x != 0 else 0.0
                    for g, x in zip(
                        grad_output.data, a.data, strict=False
                    )
                ],
                a.shape,
                device=a.device,
            ),
        )


class AbsFunction(Function):
    """Absolute value: y = |x|.

    Backward: ∂L/∂x = grad * sign(x)
    """

    def forward(self, a: Tensor) -> Tensor:
        self.save_for_backward(a)
        data = [abs(x) for x in a.data]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        return (
            Tensor(
                [
                    g * (1.0 if x > 0 else (-1.0 if x < 0 else 0.0))
                    for g, x in zip(grad_output.data, a.data, strict=False)
                ],
                a.shape,
                device=a.device,
            ),
        )


class ClampFunction(Function):
    """Clamp values to [min, max].

    Backward: grad passes through where value is in range, zero otherwise.
    """

    def forward(
        self, a: Tensor, min_val: float | None, max_val: float | None
    ) -> Tensor:
        self.save_for_backward(a)
        self.saved_metadata["min_val"] = min_val
        self.saved_metadata["max_val"] = max_val
        data = list(a.data)
        if min_val is not None:
            data = [max(x, min_val) for x in data]
        if max_val is not None:
            data = [min(x, max_val) for x in data]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        min_val = self.saved_metadata["min_val"]
        max_val = self.saved_metadata["max_val"]
        grad_data = []
        for x, g in zip(a.data, grad_output.data, strict=False):
            clamped_low = min_val is not None and x <= min_val
            clamped_high = max_val is not None and x >= max_val
            if clamped_low or clamped_high:
                grad_data.append(0.0)
            else:
                grad_data.append(g)
        return (Tensor(grad_data, a.shape, device=a.device),)


# =========================================================================
# Activation functions
# =========================================================================


class ReLUFunction(Function):
    """ReLU: y = max(0, x).

    Backward: grad * (x > 0)
    Gradient is 1 where input was positive, 0 where it was negative.
    This is why ReLU is so popular: the gradient is trivial to compute.
    """

    def forward(self, a: Tensor) -> Tensor:
        self.save_for_backward(a)
        data = [max(0.0, x) for x in a.data]
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        return (
            Tensor(
                [
                    g * (1.0 if x > 0 else 0.0)
                    for g, x in zip(
                        grad_output.data, a.data, strict=False
                    )
                ],
                a.shape,
                device=a.device,
            ),
        )


class SigmoidFunction(Function):
    """Sigmoid: y = 1 / (1 + e^(-x)).

    Backward: grad * y * (1 - y)
    Beautiful property: the derivative depends only on the output!
    """

    def forward(self, a: Tensor) -> Tensor:
        self.save_for_backward(a)
        data = [1.0 / (1.0 + math.exp(-x)) for x in a.data]
        self.saved_metadata["output"] = data
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        output = self.saved_metadata["output"]
        return (
            Tensor(
                [
                    g * y * (1.0 - y)
                    for g, y in zip(
                        grad_output.data, output, strict=False
                    )
                ],
                grad_output.shape,
                device=grad_output.device,
            ),
        )


class TanhFunction(Function):
    """Tanh: y = tanh(x) = (e^x - e^(-x)) / (e^x + e^(-x)).

    Backward: grad * (1 - y²)
    Like sigmoid, the derivative depends only on the output.
    """

    def forward(self, a: Tensor) -> Tensor:
        self.save_for_backward(a)
        data = [math.tanh(x) for x in a.data]
        self.saved_metadata["output"] = data
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        output = self.saved_metadata["output"]
        return (
            Tensor(
                [
                    g * (1.0 - y * y)
                    for g, y in zip(
                        grad_output.data, output, strict=False
                    )
                ],
                grad_output.shape,
                device=grad_output.device,
            ),
        )


class GELUFunction(Function):
    """GELU: y ≈ 0.5 * x * (1 + tanh(√(2/π) * (x + 0.044715 * x³))).

    The Gaussian Error Linear Unit, used in transformers (BERT, GPT).
    Uses the tanh approximation for both forward and backward.
    """

    _SQRT_2_PI = math.sqrt(2.0 / math.pi)
    _COEFF = 0.044715

    def forward(self, a: Tensor) -> Tensor:
        self.save_for_backward(a)
        data = []
        for x in a.data:
            inner = self._SQRT_2_PI * (x + self._COEFF * x * x * x)
            data.append(0.5 * x * (1.0 + math.tanh(inner)))
        return Tensor(data, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        grad_data = []
        for x, g in zip(a.data, grad_output.data, strict=False):
            inner = self._SQRT_2_PI * (x + self._COEFF * x * x * x)
            tanh_val = math.tanh(inner)
            # Derivative of GELU using chain rule
            sech2 = 1.0 - tanh_val * tanh_val
            d_inner = self._SQRT_2_PI * (1.0 + 3.0 * self._COEFF * x * x)
            grad_data.append(
                g * (0.5 * (1.0 + tanh_val) + 0.5 * x * sech2 * d_inner)
            )
        return (Tensor(grad_data, a.shape, device=a.device),)


class SoftmaxFunction(Function):
    """Softmax: y_i = exp(x_i) / Σexp(x_j) along a dimension.

    Backward: y * (grad - Σ(grad * y))
    This elegant formula avoids computing the full Jacobian matrix.
    """

    def forward(self, a: Tensor, dim: int) -> Tensor:
        self.save_for_backward(a)
        self.saved_metadata["dim"] = dim

        if dim < 0:
            dim = a.ndim + dim

        if a.ndim == 1:
            max_val = max(a.data)
            exps = [math.exp(x - max_val) for x in a.data]
            total = sum(exps)
            data = [e / total for e in exps]
            self.saved_metadata["output"] = data
            return Tensor(data, a.shape, device=a.device)

        # General n-D softmax along specified dimension
        strides = _compute_strides(a.shape)
        data = list(a.data)
        dim_size = a.shape[dim]
        strides[dim]

        # Process each "slice" along the softmax dimension
        # Group elements that share the same index on all dims except `dim`
        outer_size = a.numel // dim_size
        result = [0.0] * a.numel

        for outer_idx in range(outer_size):
            # Compute the base flat index for this slice
            remaining = outer_idx
            base_indices = []
            for d in range(a.ndim):
                if d == dim:
                    base_indices.append(0)
                    continue
                stride_without_dim = 1
                for d2 in range(d + 1, a.ndim):
                    if d2 != dim:
                        stride_without_dim *= a.shape[d2]
                base_indices.append(remaining // stride_without_dim)
                remaining %= stride_without_dim

            # Gather elements along dim
            indices_list = []
            for k in range(dim_size):
                idx = list(base_indices)
                idx[dim] = k
                flat = sum(i * s for i, s in zip(idx, strides, strict=False))
                indices_list.append(flat)

            vals = [data[fi] for fi in indices_list]
            max_v = max(vals)
            exps = [math.exp(v - max_v) for v in vals]
            total = sum(exps)
            for k, fi in enumerate(indices_list):
                result[fi] = exps[k] / total

        self.saved_metadata["output"] = result
        return Tensor(result, a.shape, device=a.device)

    def backward(self, grad_output: Tensor) -> tuple[Tensor | None, ...]:
        (a,) = self.saved_tensors
        output = self.saved_metadata["output"]
        dim = self.saved_metadata["dim"]

        if dim < 0:
            dim = a.ndim + dim

        if a.ndim == 1:
            # y * (grad - sum(grad * y))
            dot_product = sum(
                g * y for g, y in zip(grad_output.data, output, strict=False)
            )
            grad_data = [
                y * (g - dot_product)
                for y, g in zip(output, grad_output.data, strict=False)
            ]
            return (Tensor(grad_data, a.shape, device=a.device),)

        # General n-D case
        strides = _compute_strides(a.shape)
        dim_size = a.shape[dim]
        outer_size = a.numel // dim_size
        grad_data = [0.0] * a.numel

        for outer_idx in range(outer_size):
            remaining = outer_idx
            base_indices = []
            for d in range(a.ndim):
                if d == dim:
                    base_indices.append(0)
                    continue
                stride_without_dim = 1
                for d2 in range(d + 1, a.ndim):
                    if d2 != dim:
                        stride_without_dim *= a.shape[d2]
                base_indices.append(remaining // stride_without_dim)
                remaining %= stride_without_dim

            indices_list = []
            for k in range(dim_size):
                idx = list(base_indices)
                idx[dim] = k
                flat = sum(i * s for i, s in zip(idx, strides, strict=False))
                indices_list.append(flat)

            y_vals = [output[fi] for fi in indices_list]
            g_vals = [grad_output.data[fi] for fi in indices_list]
            dot_product = sum(g * y for g, y in zip(g_vals, y_vals, strict=False))
            for k, fi in enumerate(indices_list):
                grad_data[fi] = y_vals[k] * (g_vals[k] - dot_product)

        return (Tensor(grad_data, a.shape, device=a.device),)


# =========================================================================
# Helpers
# =========================================================================


def _matmul_2d(a: Tensor, b: Tensor) -> Tensor:
    """Simple 2-D matrix multiply without autograd tracking."""
    m, k = a.shape
    _, n = b.shape
    data = [0.0] * (m * n)
    for i in range(m):
        for j in range(n):
            s = 0.0
            for p in range(k):
                s += a.data[i * k + p] * b.data[p * n + j]
            data[i * n + j] = s
    return Tensor(data, (m, n), device=a.device)


def _transpose_2d(a: Tensor) -> Tensor:
    """Simple 2-D transpose without autograd tracking."""
    rows, cols = a.shape
    data = [0.0] * (rows * cols)
    for i in range(rows):
        for j in range(cols):
            data[j * rows + i] = a.data[i * cols + j]
    return Tensor(data, (cols, rows), device=a.device)
