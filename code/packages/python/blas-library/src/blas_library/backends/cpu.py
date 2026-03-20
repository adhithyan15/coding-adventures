"""CpuBlas — pure Python reference implementation of BLAS.

=== Why a CPU Backend? ===

The CPU backend serves two critical purposes:

1. **Universal fallback** — it works everywhere, on any machine, with no
   GPU drivers or hardware requirements. If everything else fails, CPU works.

2. **Reference implementation** — every other backend (CUDA, Metal, etc.) is
   tested against the CPU backend's results. If CudaBlas and CpuBlas disagree,
   the bug is in CudaBlas.

=== How It Works ===

Every BLAS operation is implemented with explicit Python loops. No NumPy,
no C extensions, no tricks — just ``for`` loops and arithmetic. This makes
every operation completely transparent:

    SAXPY:  for i in range(n): result[i] = alpha * x[i] + y[i]
    GEMM:   for i, for j, for k: C[i][j] += A[i][k] * B[k][j]
    DOT:    sum(x[i] * y[i] for i in range(n))

=== Performance ===

The CPU backend is SLOW. O(n^3) for GEMM with Python loop overhead on every
element. A 1000x1000 matrix multiply takes seconds. But that's fine — the
CPU backend optimizes for **clarity**, not speed. The GPU backends optimize
for speed.

=== ML Extensions ===

The CPU backend implements ALL ML extensions (activation functions, softmax,
layer normalization, batch normalization, conv2d, attention). These use the
``math`` module for exp, sqrt, tanh, etc.
"""

from __future__ import annotations

import math

from .._types import Matrix, Side, Transpose, Vector

# =========================================================================
# Helper: access a matrix element respecting transpose
# =========================================================================


def _get_element(m: Matrix, row: int, col: int, trans: Transpose) -> float:
    """Access matrix element, respecting the transpose flag.

    ================================================================
    VIRTUAL TRANSPOSE -- NO COPY NEEDED
    ================================================================

    Instead of physically transposing a matrix (allocating new memory
    and rearranging elements), we just swap the row/col indices:

        NO_TRANS: A[row][col] = data[row * cols + col]
        TRANS:    A[row][col] = data[col * cols + row]
                  (swap row and col, keep the original cols stride)

    This is how real BLAS libraries handle transpose — the data stays
    in place, only the access pattern changes.
    ================================================================
    """
    if trans == Transpose.TRANS:
        # Transposed: logical (row, col) maps to physical (col, row)
        return m.data[col * m.cols + row]
    # Not transposed: direct access
    return m.data[row * m.cols + col]


def _effective_shape(m: Matrix, trans: Transpose) -> tuple[int, int]:
    """Get the effective (rows, cols) after applying the transpose flag.

    A 2x3 matrix transposed becomes 3x2:
        NO_TRANS: (2, 3) -> (2, 3)
        TRANS:    (2, 3) -> (3, 2)
    """
    if trans == Transpose.TRANS:
        return (m.cols, m.rows)
    return (m.rows, m.cols)


class CpuBlas:
    """Pure Python BLAS implementation — the reference backend.

    ================================================================
    CPU BLAS -- THE REFERENCE IMPLEMENTATION
    ================================================================

    This class implements both BlasBackend and MlBlasBackend protocols
    using nothing but Python loops and the ``math`` standard library.

    Every other backend's correctness is measured against this one.
    If CudaBlas.sgemm() and CpuBlas.sgemm() disagree on the result,
    the bug is in CudaBlas, not CpuBlas.

    Usage:
        blas = CpuBlas()
        result = blas.saxpy(2.0, x, y)
        result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    ================================================================
    """

    # =================================================================
    # Identity properties
    # =================================================================

    @property
    def name(self) -> str:
        """Backend identifier."""
        return "cpu"

    @property
    def device_name(self) -> str:
        """Human-readable device name."""
        return "CPU (pure Python)"

    # =================================================================
    # LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
    # =================================================================

    def saxpy(self, alpha: float, x: Vector, y: Vector) -> Vector:
        """SAXPY: result = alpha * x + y

        ================================================================
        SAXPY -- THE HELLO WORLD OF BLAS
        ================================================================

        S = Single precision, A = Alpha, X = vector X, P = Plus, Y = vector Y

        This is the simplest BLAS operation and our running example
        since Layer 11 (logic gates). Each element:

            result[i] = alpha * x[i] + y[i]

        Time complexity: O(n) -- one pass through both vectors.
        ================================================================
        """
        if x.size != y.size:
            raise ValueError(
                f"SAXPY dimension mismatch: x.size={x.size} != y.size={y.size}"
            )
        result = [alpha * x.data[i] + y.data[i] for i in range(x.size)]
        return Vector(data=result, size=x.size)

    def sdot(self, x: Vector, y: Vector) -> float:
        """DOT product: result = sum(x[i] * y[i])

        ================================================================
        DOT PRODUCT -- FOUNDATION OF SIMILARITY
        ================================================================

        The dot product measures how "aligned" two vectors are:
        - Parallel vectors: large positive dot product
        - Perpendicular vectors: dot product = 0
        - Anti-parallel: large negative dot product

        It's also the building block of matrix multiply (GEMM is
        just a grid of dot products).

        Time complexity: O(n)
        ================================================================
        """
        if x.size != y.size:
            raise ValueError(
                f"DOT dimension mismatch: x.size={x.size} != y.size={y.size}"
            )
        return sum(x.data[i] * y.data[i] for i in range(x.size))

    def snrm2(self, x: Vector) -> float:
        """Euclidean norm: result = sqrt(sum(x[i]^2))

        ================================================================
        EUCLIDEAN NORM (L2 NORM)
        ================================================================

        The "length" of a vector in Euclidean space. Used for:
        - Normalizing vectors (dividing by the norm to get unit vectors)
        - Convergence checks (is the gradient small enough?)
        - Regularization (keeping weights small)

        Numerically: sqrt(x[0]^2 + x[1]^2 + ... + x[n-1]^2)

        Time complexity: O(n)
        ================================================================
        """
        return math.sqrt(sum(xi * xi for xi in x.data))

    def sscal(self, alpha: float, x: Vector) -> Vector:
        """Scale: result = alpha * x

        Multiply every element by the scalar alpha.
        Time complexity: O(n)
        """
        return Vector(data=[alpha * xi for xi in x.data], size=x.size)

    def sasum(self, x: Vector) -> float:
        """Absolute sum (L1 norm): result = sum(|x[i]|)

        Also called the Manhattan distance or taxicab norm. Used in
        L1 regularization (LASSO) which encourages sparsity.

        Time complexity: O(n)
        """
        return sum(abs(xi) for xi in x.data)

    def isamax(self, x: Vector) -> int:
        """Index of maximum absolute value: argmax(|x[i]|)

        Returns the 0-based index of the element with the largest
        absolute value. Used in partial pivoting for LU decomposition
        to improve numerical stability.

        Time complexity: O(n)
        """
        if x.size == 0:
            return 0
        max_idx = 0
        max_val = abs(x.data[0])
        for i in range(1, x.size):
            val = abs(x.data[i])
            if val > max_val:
                max_val = val
                max_idx = i
        return max_idx

    def scopy(self, x: Vector) -> Vector:
        """Copy: result = x (deep copy)

        Creates a completely independent copy. Modifying the result
        does not affect the original.

        Time complexity: O(n)
        """
        return Vector(data=list(x.data), size=x.size)

    def sswap(self, x: Vector, y: Vector) -> tuple[Vector, Vector]:
        """Swap: exchange the contents of x and y.

        Returns (new_x, new_y) where new_x has y's data and new_y
        has x's data. The originals are not modified.

        Time complexity: O(n)
        """
        if x.size != y.size:
            raise ValueError(
                f"SWAP dimension mismatch: x.size={x.size} != y.size={y.size}"
            )
        return (
            Vector(data=list(y.data), size=y.size),
            Vector(data=list(x.data), size=x.size),
        )

    # =================================================================
    # LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
    # =================================================================

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

        ================================================================
        GEMV -- MATRIX TIMES VECTOR
        ================================================================

        op(A) is the matrix A, optionally transposed:
            NO_TRANS: op(A) = A        (M x N)
            TRANS:    op(A) = A^T      (N x M)

        After applying the transpose:
            op(A) has shape (m x n)
            x must have size n
            y must have size m
            result has size m

        Each element of the result:
            result[i] = alpha * sum(op(A)[i][k] * x[k], k=0..n-1) + beta * y[i]

        Time complexity: O(M * N)
        ================================================================
        """
        m, n = _effective_shape(a, trans)

        if x.size != n:
            raise ValueError(
                f"GEMV dimension mismatch: op(A) is {m}x{n} but x.size={x.size}"
            )
        if y.size != m:
            raise ValueError(
                f"GEMV dimension mismatch: op(A) is {m}x{n} but y.size={y.size}"
            )

        result = [0.0] * m
        for i in range(m):
            s = 0.0
            for k in range(n):
                s += _get_element(a, i, k, trans) * x.data[k]
            result[i] = alpha * s + beta * y.data[i]

        return Vector(data=result, size=m)

    def sger(self, alpha: float, x: Vector, y: Vector, a: Matrix) -> Matrix:
        """Outer product (rank-1 update): A = alpha * x * y^T + A

        ================================================================
        GER -- OUTER PRODUCT
        ================================================================

        The outer product of two vectors creates a matrix:

            x = [a, b]     y = [c, d, e]

            x * y^T = [ a*c  a*d  a*e ]
                      [ b*c  b*d  b*e ]

        Then we scale by alpha and add to the existing matrix A.
        Each element: result[i][j] = alpha * x[i] * y[j] + A[i][j]

        Time complexity: O(M * N)
        ================================================================
        """
        if a.rows != x.size:
            raise ValueError(
                f"GER dimension mismatch: A.rows={a.rows} != x.size={x.size}"
            )
        if a.cols != y.size:
            raise ValueError(
                f"GER dimension mismatch: A.cols={a.cols} != y.size={y.size}"
            )

        result = list(a.data)  # copy
        for i in range(a.rows):
            for j in range(a.cols):
                result[i * a.cols + j] += alpha * x.data[i] * y.data[j]

        return Matrix(data=result, rows=a.rows, cols=a.cols, order=a.order)

    # =================================================================
    # LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
    # =================================================================

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

        ================================================================
        GEMM -- THE MOST IMPORTANT FUNCTION IN ALL OF COMPUTING
        ================================================================

        This is the function that NVIDIA employs entire teams to
        optimize. 70-90% of ML training time is spent here.

        C = alpha * op(A) * op(B) + beta * C

        where:
            op(A) has shape (M x K)
            op(B) has shape (K x N)
            C     has shape (M x N)

        The triple nested loop:
            for i in range(M):          # row of C
                for j in range(N):      # col of C
                    sum = 0.0
                    for k in range(K):  # shared dimension
                        sum += op(A)[i][k] * op(B)[k][j]
                    C[i][j] = alpha * sum + beta * C[i][j]

        Common special cases:
            C = A * B        -> alpha=1, beta=0
            C = A^T * B      -> trans_a=TRANS, alpha=1, beta=0
            C += A * B       -> alpha=1, beta=1
            C = 2*A*B + 3*C  -> alpha=2, beta=3

        Time complexity: O(M * N * K)
        ================================================================
        """
        # Determine effective shapes after transpose
        m, k_a = _effective_shape(a, trans_a)
        k_b, n = _effective_shape(b, trans_b)

        # The inner dimensions must match
        if k_a != k_b:
            raise ValueError(
                f"GEMM dimension mismatch: op(A) is {m}x{k_a}, "
                f"op(B) is {k_b}x{n}. Inner dimensions {k_a} != {k_b}"
            )
        k = k_a

        # C must have shape (M x N)
        if c.rows != m or c.cols != n:
            raise ValueError(
                f"GEMM dimension mismatch: result should be {m}x{n} "
                f"but C is {c.rows}x{c.cols}"
            )

        # The triple nested loop — the heart of linear algebra
        result = [0.0] * (m * n)
        for i in range(m):
            for j in range(n):
                s = 0.0
                for kk in range(k):
                    s += _get_element(a, i, kk, trans_a) * _get_element(
                        b, kk, j, trans_b
                    )
                result[i * n + j] = alpha * s + beta * c.data[i * c.cols + j]

        return Matrix(data=result, rows=m, cols=n, order=c.order)

    def ssymm(
        self,
        side: Side,
        alpha: float,
        a: Matrix,
        b: Matrix,
        beta: float,
        c: Matrix,
    ) -> Matrix:
        """Symmetric Matrix Multiply.

        ================================================================
        SYMM -- SYMMETRIC MATRIX MULTIPLY
        ================================================================

        Like GEMM, but exploits the fact that A is symmetric (A = A^T).
        The backend only needs to read half of A.

        LEFT:  C = alpha * A * B + beta * C
        RIGHT: C = alpha * B * A + beta * C

        A must be square (rows == cols).
        ================================================================
        """
        if a.rows != a.cols:
            raise ValueError(f"SSYMM: A must be square but is {a.rows}x{a.cols}")

        if side == Side.LEFT:
            # C = alpha * A * B + beta * C
            # A is (M x M), B is (M x N), C is (M x N)
            m = a.rows
            n = b.cols
            if b.rows != m:
                raise ValueError(f"SSYMM LEFT: A is {m}x{m} but B.rows={b.rows}")
        else:
            # C = alpha * B * A + beta * C
            # B is (M x N), A is (N x N), C is (M x N)
            m = b.rows
            n = a.rows
            if b.cols != n:
                raise ValueError(f"SSYMM RIGHT: A is {n}x{n} but B.cols={b.cols}")

        if c.rows != m or c.cols != n:
            raise ValueError(f"SSYMM: C should be {m}x{n} but is {c.rows}x{c.cols}")

        # Use sgemm with NO_TRANS for both — A is symmetric so A = A^T
        if side == Side.LEFT:
            return self.sgemm(
                Transpose.NO_TRANS, Transpose.NO_TRANS, alpha, a, b, beta, c
            )
        else:
            return self.sgemm(
                Transpose.NO_TRANS, Transpose.NO_TRANS, alpha, b, a, beta, c
            )

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

        ================================================================
        BATCHED GEMM -- MANY MATRIX MULTIPLIES AT ONCE
        ================================================================

        Used for multi-head attention (each head is a separate GEMM),
        batched inference (each sample is a separate GEMM), and more.

        On a GPU, all GEMMs can run in parallel. On CPU, we just loop.
        ================================================================
        """
        if len(a_list) != len(b_list) or len(b_list) != len(c_list):
            raise ValueError(
                f"Batched GEMM: batch sizes don't match: "
                f"A={len(a_list)}, B={len(b_list)}, C={len(c_list)}"
            )
        return [
            self.sgemm(trans_a, trans_b, alpha, a, b, beta, c)
            for a, b, c in zip(a_list, b_list, c_list, strict=False)
        ]

    # =================================================================
    # ML EXTENSIONS: Activation Functions
    # =================================================================

    def relu(self, x: Matrix) -> Matrix:
        """ReLU activation: max(0, x)

        ================================================================
        RELU -- RECTIFIED LINEAR UNIT
        ================================================================

        The most common activation function in deep learning:
            relu(x) = max(0, x)

        Truth table for a single element:
            x < 0  -> 0.0    (negative inputs are zeroed)
            x >= 0 -> x      (positive inputs pass through)

        ReLU is popular because:
        1. It's extremely fast to compute (just a comparison)
        2. It doesn't saturate for positive values (no vanishing gradient)
        3. It produces sparse activations (many zeros)
        ================================================================
        """
        return Matrix(
            data=[max(0.0, v) for v in x.data],
            rows=x.rows,
            cols=x.cols,
            order=x.order,
        )

    def gelu(self, x: Matrix) -> Matrix:
        """GELU activation: x * Phi(x) where Phi is the CDF of N(0,1).

        ================================================================
        GELU -- GAUSSIAN ERROR LINEAR UNIT
        ================================================================

        Used in GPT, BERT, and modern Transformers. Unlike ReLU which
        has a hard cutoff at 0, GELU smoothly transitions:

            gelu(x) = x * 0.5 * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))

        This approximation (from Hendrycks & Gimpel, 2016) is what
        PyTorch and TensorFlow use.
        ================================================================
        """
        sqrt_2_over_pi = math.sqrt(2.0 / math.pi)
        result = []
        for v in x.data:
            inner = sqrt_2_over_pi * (v + 0.044715 * v * v * v)
            result.append(0.5 * v * (1.0 + math.tanh(inner)))
        return Matrix(data=result, rows=x.rows, cols=x.cols, order=x.order)

    def sigmoid(self, x: Matrix) -> Matrix:
        """Sigmoid activation: 1 / (1 + exp(-x))

        ================================================================
        SIGMOID -- THE LOGISTIC FUNCTION
        ================================================================

        Maps any real number to the range (0, 1):
            sigmoid(-inf) -> 0
            sigmoid(0)    -> 0.5
            sigmoid(+inf) -> 1

        Used as the output activation for binary classification and
        as a gate in LSTMs.

        Numerically stable implementation: for large negative x,
        exp(-x) overflows. We use: if x >= 0, compute as 1/(1+exp(-x));
        if x < 0, compute as exp(x)/(1+exp(x)).
        ================================================================
        """
        result = []
        for v in x.data:
            if v >= 0:
                result.append(1.0 / (1.0 + math.exp(-v)))
            else:
                ev = math.exp(v)
                result.append(ev / (1.0 + ev))
        return Matrix(data=result, rows=x.rows, cols=x.cols, order=x.order)

    def tanh_activation(self, x: Matrix) -> Matrix:
        """Tanh activation: tanh(x)

        Maps any real number to (-1, 1). Used in RNNs and as an
        activation function. Related to sigmoid: tanh(x) = 2*sigmoid(2x) - 1.
        """
        return Matrix(
            data=[math.tanh(v) for v in x.data],
            rows=x.rows,
            cols=x.cols,
            order=x.order,
        )

    # =================================================================
    # ML EXTENSIONS: Softmax
    # =================================================================

    def softmax(self, x: Matrix, axis: int = -1) -> Matrix:
        """Numerically stable softmax along an axis.

        ================================================================
        SOFTMAX -- PROBABILITY DISTRIBUTION OVER A VECTOR
        ================================================================

        Converts a vector of real numbers into a probability distribution:
            softmax(x)[i] = exp(x[i]) / sum(exp(x[j]))

        The NAIVE implementation overflows for large x because exp(710)
        is infinity in float64. The STABLE version subtracts the max first:
            softmax(x)[i] = exp(x[i] - max(x)) / sum(exp(x[j] - max(x)))

        This works because softmax is invariant to constant shifts:
            softmax(x + c) = softmax(x)  for any constant c

        axis=-1 means "along the last dimension" (columns for 2D).
        For a 2D matrix, this means each ROW becomes a probability
        distribution that sums to 1.0.
        ================================================================
        """
        # Normalize axis
        if axis == -1:
            axis = 1  # last axis for 2D matrix = columns

        if axis == 1:
            # Softmax along each row
            result = []
            for i in range(x.rows):
                row = x.data[i * x.cols : (i + 1) * x.cols]
                max_val = max(row)
                exps = [math.exp(v - max_val) for v in row]
                total = sum(exps)
                result.extend(e / total for e in exps)
        else:
            # axis == 0: softmax along each column
            result = list(x.data)
            for j in range(x.cols):
                col = [x.data[i * x.cols + j] for i in range(x.rows)]
                max_val = max(col)
                exps = [math.exp(v - max_val) for v in col]
                total = sum(exps)
                for i in range(x.rows):
                    result[i * x.cols + j] = exps[i] / total

        return Matrix(data=result, rows=x.rows, cols=x.cols, order=x.order)

    # =================================================================
    # ML EXTENSIONS: Normalization
    # =================================================================

    def layer_norm(
        self,
        x: Matrix,
        gamma: Vector,
        beta: Vector,
        eps: float = 1e-5,
    ) -> Matrix:
        """Layer Normalization (Ba et al., 2016).

        ================================================================
        LAYER NORM -- NORMALIZE EACH SAMPLE INDEPENDENTLY
        ================================================================

        For each row (sample) in the matrix:
            1. Compute mean: mu = sum(x) / n
            2. Compute variance: var = sum((x - mu)^2) / n
            3. Normalize: x_hat = (x - mu) / sqrt(var + eps)
            4. Scale and shift: result = gamma * x_hat + beta

        gamma and beta are learnable parameters (one per feature).

        Used in: Transformers, GPT, BERT (before every attention/FFN block)
        ================================================================
        """
        if gamma.size != x.cols:
            raise ValueError(f"LayerNorm: gamma.size={gamma.size} != x.cols={x.cols}")
        if beta.size != x.cols:
            raise ValueError(f"LayerNorm: beta.size={beta.size} != x.cols={x.cols}")

        result = [0.0] * (x.rows * x.cols)
        n = x.cols

        for i in range(x.rows):
            row = x.data[i * n : (i + 1) * n]

            # Step 1: mean
            mean = sum(row) / n

            # Step 2: variance
            var = sum((v - mean) ** 2 for v in row) / n

            # Step 3 & 4: normalize, scale, shift
            inv_std = 1.0 / math.sqrt(var + eps)
            for j in range(n):
                x_hat = (row[j] - mean) * inv_std
                result[i * n + j] = gamma.data[j] * x_hat + beta.data[j]

        return Matrix(data=result, rows=x.rows, cols=x.cols, order=x.order)

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
        """Batch Normalization (Ioffe & Szegedy, 2015).

        ================================================================
        BATCH NORM -- NORMALIZE EACH FEATURE ACROSS THE BATCH
        ================================================================

        Unlike layer norm (which normalizes each sample), batch norm
        normalizes each FEATURE across all samples in the batch:

        Training mode:
            mean_j = sum(x[i][j] for i in batch) / batch_size
            var_j  = sum((x[i][j] - mean_j)^2 for i in batch) / batch_size
            x_hat[i][j] = (x[i][j] - mean_j) / sqrt(var_j + eps)
            result[i][j] = gamma[j] * x_hat[i][j] + beta[j]

        Inference mode:
            Uses running_mean and running_var instead of batch statistics.

        Used in: CNNs, ResNets, most non-Transformer architectures
        ================================================================
        """
        if gamma.size != x.cols:
            raise ValueError(f"BatchNorm: gamma.size={gamma.size} != x.cols={x.cols}")
        if beta.size != x.cols:
            raise ValueError(f"BatchNorm: beta.size={beta.size} != x.cols={x.cols}")

        result = [0.0] * (x.rows * x.cols)
        batch_size = x.rows
        n_features = x.cols

        if training:
            # Compute batch statistics
            for j in range(n_features):
                col = [x.data[i * n_features + j] for i in range(batch_size)]
                mean = sum(col) / batch_size
                var = sum((v - mean) ** 2 for v in col) / batch_size
                inv_std = 1.0 / math.sqrt(var + eps)
                for i in range(batch_size):
                    x_hat = (col[i] - mean) * inv_std
                    result[i * n_features + j] = gamma.data[j] * x_hat + beta.data[j]
        else:
            # Use running statistics
            for j in range(n_features):
                mean = running_mean.data[j]
                var = running_var.data[j]
                inv_std = 1.0 / math.sqrt(var + eps)
                for i in range(batch_size):
                    x_hat = (x.data[i * n_features + j] - mean) * inv_std
                    result[i * n_features + j] = gamma.data[j] * x_hat + beta.data[j]

        return Matrix(data=result, rows=x.rows, cols=x.cols, order=x.order)

    # =================================================================
    # ML EXTENSIONS: Convolution
    # =================================================================

    def conv2d(
        self,
        input_mat: Matrix,
        weight: Matrix,
        bias: Vector | None = None,
        stride: int = 1,
        padding: int = 0,
    ) -> Matrix:
        """2D Convolution via im2col + GEMM.

        ================================================================
        CONV2D -- SIMPLIFIED 2D CONVOLUTION
        ================================================================

        We treat input_mat as a 2D spatial feature map (height x width)
        and weight as a 2D filter (kH x kW). This is a simplified
        single-channel convolution for demonstration.

        Steps:
        1. Apply padding if needed
        2. Extract all patches (im2col style) into columns
        3. Flatten weight into a row vector
        4. Compute dot product of weight with each patch

        The output has shape:
            out_h = (height + 2*padding - kH) / stride + 1
            out_w = (width + 2*padding - kW) / stride + 1
        ================================================================
        """
        h_in = input_mat.rows
        w_in = input_mat.cols
        k_h = weight.rows
        k_w = weight.cols

        # Output dimensions
        out_h = (h_in + 2 * padding - k_h) // stride + 1
        out_w = (w_in + 2 * padding - k_w) // stride + 1

        if out_h <= 0 or out_w <= 0:
            raise ValueError(
                f"Conv2d: output dimensions are non-positive: {out_h}x{out_w}"
            )

        # Create padded input if needed
        if padding > 0:
            padded_h = h_in + 2 * padding
            padded_w = w_in + 2 * padding
            padded = [0.0] * (padded_h * padded_w)
            for i in range(h_in):
                for j in range(w_in):
                    padded[(i + padding) * padded_w + (j + padding)] = input_mat.data[
                        i * w_in + j
                    ]
        else:
            padded_h = h_in
            padded_w = w_in
            padded = list(input_mat.data)

        # Compute convolution
        result = [0.0] * (out_h * out_w)
        weight_flat = weight.data

        for oh in range(out_h):
            for ow in range(out_w):
                s = 0.0
                for kh in range(k_h):
                    for kw in range(k_w):
                        ih = oh * stride + kh
                        iw = ow * stride + kw
                        s += padded[ih * padded_w + iw] * weight_flat[kh * k_w + kw]
                if bias is not None:
                    # For simplified single-filter case, use bias[0]
                    s += bias.data[0] if bias.size > 0 else 0.0
                result[oh * out_w + ow] = s

        return Matrix(data=result, rows=out_h, cols=out_w)

    # =================================================================
    # ML EXTENSIONS: Attention
    # =================================================================

    def attention(
        self,
        q: Matrix,
        k: Matrix,
        v: Matrix,
        mask: Matrix | None = None,
        scale: float | None = None,
    ) -> Matrix:
        """Scaled Dot-Product Attention (Vaswani et al., 2017).

        ================================================================
        ATTENTION -- THE CORE OF TRANSFORMERS
        ================================================================

        Attention(Q, K, V) = softmax(Q * K^T / sqrt(d_k)) * V

        Steps:
        1. scores = Q * K^T                     (SGEMM, Level 3)
        2. scores = scores / scale               (SSCAL-like)
        3. if mask: scores = scores + mask        (element-wise)
        4. weights = softmax(scores, axis=-1)    (ML extension)
        5. output = weights * V                  (SGEMM, Level 3)

        Q shape: (seq_len x d_k)
        K shape: (seq_len x d_k)
        V shape: (seq_len x d_v)
        Returns: (seq_len x d_v)

        This is the function that enables GPT, BERT, and every
        Transformer model to attend to different parts of the input.
        ================================================================
        """
        d_k = q.cols
        if scale is None:
            scale = math.sqrt(float(d_k))

        # Step 1: scores = Q * K^T using SGEMM
        # Q is (seq x d_k), K is (seq x d_k), K^T is (d_k x seq)
        # scores = Q * K^T is (seq x seq)
        seq_len = q.rows
        scores_c = Matrix(data=[0.0] * (seq_len * k.rows), rows=seq_len, cols=k.rows)
        scores = self.sgemm(
            Transpose.NO_TRANS, Transpose.TRANS, 1.0, q, k, 0.0, scores_c
        )

        # Step 2: scale
        scaled_data = [v / scale for v in scores.data]

        # Step 3: apply mask (additive, typically -inf for masked positions)
        if mask is not None:
            for i in range(len(scaled_data)):
                scaled_data[i] += mask.data[i]

        scores_matrix = Matrix(data=scaled_data, rows=scores.rows, cols=scores.cols)

        # Step 4: softmax along the last dimension (each row)
        weights = self.softmax(scores_matrix, axis=-1)

        # Step 5: output = weights * V using SGEMM
        output_c = Matrix(
            data=[0.0] * (weights.rows * v.cols),
            rows=weights.rows,
            cols=v.cols,
        )
        return self.sgemm(
            Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, weights, v, 0.0, output_c
        )
