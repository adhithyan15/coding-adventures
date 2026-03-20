"""BLAS Backend Protocols — the contracts every backend must fulfill.

=== What is a Protocol? ===

A Protocol (PEP 544) defines a set of methods that a class must implement
to be considered "compatible." Unlike abstract base classes, protocols use
structural subtyping — you don't need to inherit from anything. If your
class has the right methods with the right signatures, it satisfies the
protocol.

    class BlasBackend(Protocol):
        def saxpy(self, alpha, x, y) -> Vector: ...

    class MyCoolBackend:          # No inheritance needed!
        def saxpy(self, alpha, x, y) -> Vector:
            return ...  # just implement the method

    backend: BlasBackend = MyCoolBackend()  # Type-checks fine!

=== Two Protocols ===

1. ``BlasBackend`` — the core BLAS operations (Levels 1, 2, 3).
   Every backend MUST implement this.

2. ``MlBlasBackend`` — extends BlasBackend with ML operations
   (activations, softmax, normalization, conv2d, attention).
   This is OPTIONAL. The CPU backend implements it as a reference.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol, runtime_checkable

if TYPE_CHECKING:
    from ._types import Matrix, Side, Transpose, Vector

# =========================================================================
# BlasBackend — the core protocol
# =========================================================================


@runtime_checkable
class BlasBackend(Protocol):
    """The BLAS backend protocol — the contract every backend must fulfill.

    ================================================================
    THE BLAS BACKEND PROTOCOL
    ================================================================

    This is the contract every backend must fulfill. Whether you're
    running on an NVIDIA GPU, an Apple M4, or a Raspberry Pi CPU,
    if you implement this protocol, you're a valid BLAS backend.

    The design follows Python's "duck typing" philosophy:
    - If it implements sgemm(), it's a BLAS backend
    - No inheritance required
    - No registration required (though the registry helps)

    All operations return NEW Matrix/Vector objects. They do not
    mutate inputs. This is cleaner for testing and avoids aliasing
    bugs. Real BLAS mutates in-place for performance, but we
    optimize for clarity.
    ================================================================
    """

    @property
    def name(self) -> str:
        """Backend identifier: 'cpu', 'cuda', 'metal', etc."""
        ...

    @property
    def device_name(self) -> str:
        """Human-readable device name: 'NVIDIA H100', 'Apple M4', 'CPU', etc."""
        ...

    # ==========================================================
    # LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
    # ==========================================================

    def saxpy(self, alpha: float, x: Vector, y: Vector) -> Vector:
        """SAXPY: y = alpha * x + y

        The most famous BLAS operation. Each element:
            result[i] = alpha * x[i] + y[i]

        Requires: x.size == y.size
        Returns: new Vector of same size
        """
        ...

    def sdot(self, x: Vector, y: Vector) -> float:
        """DOT product: result = x . y = sum(x_i * y_i)

        Requires: x.size == y.size
        Returns: scalar float
        """
        ...

    def snrm2(self, x: Vector) -> float:
        """Euclidean norm: result = ||x||_2 = sqrt(sum(x_i^2))

        Returns: scalar float >= 0
        """
        ...

    def sscal(self, alpha: float, x: Vector) -> Vector:
        """Scale: result = alpha * x

        Returns: new Vector of same size
        """
        ...

    def sasum(self, x: Vector) -> float:
        """Absolute sum: result = sum(|x_i|)

        Returns: scalar float >= 0
        """
        ...

    def isamax(self, x: Vector) -> int:
        """Index of max absolute value: result = argmax(|x_i|)

        Returns: integer index (0-based)
        """
        ...

    def scopy(self, x: Vector) -> Vector:
        """Copy: result = x (deep copy)

        Returns: new Vector with same data
        """
        ...

    def sswap(self, x: Vector, y: Vector) -> tuple[Vector, Vector]:
        """Swap: x <-> y

        Returns: (new_x with y's data, new_y with x's data)
        Requires: x.size == y.size
        """
        ...

    # ==========================================================
    # LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
    # ==========================================================

    def sgemv(
        self,
        trans: Transpose,
        alpha: float,
        a: Matrix,
        x: Vector,
        beta: float,
        y: Vector,
    ) -> Vector:
        """General Matrix-Vector multiply: y = alpha * op(A) * x + beta * y

        If trans == TRANS, uses A^T instead of A.

        The effective dimensions after transpose:
          NO_TRANS: A is (M x N), x must be size N, y must be size M
          TRANS:    A is (M x N), x must be size M, y must be size N

        Returns: new Vector
        """
        ...

    def sger(self, alpha: float, x: Vector, y: Vector, a: Matrix) -> Matrix:
        """Outer product (rank-1 update): A = alpha * x * y^T + A

        Every element:
            result[i][j] = alpha * x[i] * y[j] + A[i][j]

        Requires: A.rows == x.size, A.cols == y.size
        Returns: new Matrix of same shape as A
        """
        ...

    # ==========================================================
    # LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
    # ==========================================================

    def sgemm(
        self,
        trans_a: Transpose,
        trans_b: Transpose,
        alpha: float,
        a: Matrix,
        b: Matrix,
        beta: float,
        c: Matrix,
    ) -> Matrix:
        """General Matrix Multiply: C = alpha * op(A) * op(B) + beta * C

        where op(X) = X      if trans == NO_TRANS
              op(X) = X^T    if trans == TRANS

        Dimensions after transpose:
          op(A) is (M x K)
          op(B) is (K x N)
          C     is (M x N)

        Returns: new Matrix of same shape as C
        """
        ...

    def ssymm(
        self,
        side: Side,
        alpha: float,
        a: Matrix,
        b: Matrix,
        beta: float,
        c: Matrix,
    ) -> Matrix:
        """Symmetric Matrix Multiply: C = alpha * A * B + beta * C (A symmetric)

        If side == LEFT:  C = alpha * A * B + beta * C
        If side == RIGHT: C = alpha * B * A + beta * C

        A must be square and symmetric.
        Returns: new Matrix of same shape as C
        """
        ...

    def sgemm_batched(
        self,
        trans_a: Transpose,
        trans_b: Transpose,
        alpha: float,
        a_list: list[Matrix],
        b_list: list[Matrix],
        beta: float,
        c_list: list[Matrix],
    ) -> list[Matrix]:
        """Batched GEMM: multiple independent GEMMs.

            Cs[i] = alpha * op(As[i]) * op(Bs[i]) + beta * Cs[i]

        Requires: len(As) == len(Bs) == len(Cs)
        Returns: list of new Matrices
        """
        ...


# =========================================================================
# MlBlasBackend — optional ML extensions
# =========================================================================


@runtime_checkable
class MlBlasBackend(BlasBackend, Protocol):
    """ML extensions beyond classic BLAS.

    ================================================================
    ML EXTENSIONS BEYOND CLASSIC BLAS
    ================================================================

    Classic BLAS handles linear algebra. ML needs additional
    operations: activation functions, normalization, convolution,
    and attention. These operations CAN be built from BLAS primitives
    (attention = two GEMMs + softmax), but dedicated implementations
    are much faster.

    This protocol is OPTIONAL. A backend that only implements
    BlasBackend is still a valid BLAS backend.
    ================================================================
    """

    def relu(self, x: Matrix) -> Matrix:
        """ReLU: result[i] = max(0, x[i])"""
        ...

    def gelu(self, x: Matrix) -> Matrix:
        """GELU: result[i] = x[i] * Phi(x[i]) where Phi is CDF of N(0,1)"""
        ...

    def sigmoid(self, x: Matrix) -> Matrix:
        """Sigmoid: result[i] = 1 / (1 + exp(-x[i]))"""
        ...

    def tanh_activation(self, x: Matrix) -> Matrix:
        """Tanh: result[i] = tanh(x[i])"""
        ...

    def softmax(self, x: Matrix, axis: int = -1) -> Matrix:
        """Softmax along an axis (numerically stable)."""
        ...

    def layer_norm(
        self,
        x: Matrix,
        gamma: Vector,
        beta: Vector,
        eps: float = 1e-5,
    ) -> Matrix:
        """Layer Normalization (Ba et al., 2016)."""
        ...

    def batch_norm(
        self,
        x: Matrix,
        gamma: Vector,
        beta: Vector,
        running_mean: Vector,
        running_var: Vector,
        eps: float = 1e-5,
        training: bool = False,
    ) -> Matrix:
        """Batch Normalization (Ioffe & Szegedy, 2015)."""
        ...

    def conv2d(
        self,
        input_mat: Matrix,
        weight: Matrix,
        bias: Vector | None = None,
        stride: int = 1,
        padding: int = 0,
    ) -> Matrix:
        """2D Convolution via im2col + GEMM."""
        ...

    def attention(
        self,
        q: Matrix,
        k: Matrix,
        v: Matrix,
        mask: Matrix | None = None,
        scale: float | None = None,
    ) -> Matrix:
        """Scaled Dot-Product Attention (Vaswani et al., 2017)."""
        ...
