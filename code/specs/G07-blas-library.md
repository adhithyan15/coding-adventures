# G07 — Pluggable BLAS Library

## Overview

This package implements **Layer 3 of the accelerator computing stack** — a
pluggable BLAS (Basic Linear Algebra Subprograms) library with swappable
backend implementations. The BLAS interface defines linear algebra operations
abstractly — the caller writes `blas.sgemm(A, B)` and never knows whether it
runs on CUDA, Metal, OpenCL, Vulkan, WebGPU, OpenGL, or a pure CPU fallback.

Think of it this way: Layer 4 gave us six different restaurant menus (CUDA,
Metal, OpenCL, etc.) that all share one kitchen (Layer 5). Layer 3 gives us
a single **food delivery app** that routes your order to whichever restaurant
is open. You just say "I want matrix multiplication" — the library picks the
backend.

### Real-World Analogs

| Our Backend | Real-World Equivalent | Hardware Target |
|-------------|----------------------|-----------------|
| CpuBlas | OpenBLAS, Reference BLAS | Any CPU |
| CudaBlas | cuBLAS | NVIDIA GPUs |
| OpenClBlas | clBLAS | Any OpenCL device |
| MetalBlas | Accelerate/MPS | Apple Silicon |
| VulkanBlas | (research projects) | Any Vulkan device |
| WebGpuBlas | (emerging, WebGPU compute) | Browsers + native |
| OpenGlBlas | (legacy compute) | OpenGL 4.3+ devices |

The pattern is exactly what the real world uses. When you call `A @ B` in
NumPy, it dispatches to MKL (Intel), OpenBLAS (AMD/generic CPU), or
Accelerate (Apple) depending on what's available. When PyTorch does the same
on a GPU, it dispatches to cuBLAS (NVIDIA) or rocBLAS (AMD). Our library
does the same thing.

## Layer Position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    |
Layer 10: FP Arithmetic (IEEE 754 add/mul/fma)
    |
Layer  9: Accelerator Core (gpu-core) — one core, one instruction at a time
    |
Layer  8: Parallel Execution Engine — warps, wavefronts, systolic arrays
    |
Layer  7: Compute Unit — SM, CU, MXU, XeCore, ANECore
    |
Layer  6: Device Simulator — complete devices with global memory + work dist.
    |
Layer  5: Compute Runtime — Vulkan-inspired explicit GPU API
    |
Layer  4: Vendor API Simulators (CUDA, OpenCL, Metal, Vulkan, WebGPU, OpenGL)
    |
Layer  3: BLAS Library  <-- YOU ARE HERE
    |
    +-->  Abstract Interface (BlasBackend protocol)
    |         |
    |         +-- Level 1: SAXPY, DOT, NRM2, SCAL, ASUM, IAMAX, COPY, SWAP
    |         +-- Level 2: GEMV, GER
    |         +-- Level 3: GEMM, SYMM, Batched GEMM
    |         +-- ML Ext:  ReLU, Softmax, LayerNorm, Conv2D, Attention
    |
    +-->  Backend Implementations (pluggable)
    |     +-- CpuBlas       (pure CPU, no GPU)
    |     +-- CudaBlas      (wraps CUDARuntime)
    |     +-- OpenClBlas    (wraps CLContext)
    |     +-- MetalBlas     (wraps MTLDevice)
    |     +-- VulkanBlas    (wraps VulkanRuntime)
    |     +-- WebGpuBlas    (wraps GPUDevice)
    |     +-- OpenGlBlas    (wraps GLContext)
    |
    +-->  BackendRegistry (select by name or auto-detect)
    |
Layer  2: Tensor + Autograd (future)
    |
Layer  1: ML Framework (future)
```

**Depends on:**
- `vendor-api-simulators` (Layer 4) — for GPU backends
- `compute-runtime` (Layer 5) — transitively
- No dependency for the CPU backend — pure language, no GPU

**Used by:** Tensor + Autograd (Layer 2, future)

## The Big Picture: What is BLAS?

BLAS (Basic Linear Algebra Subprograms) is a **specification**, not a library.
Published in 1979, it defines a standard set of operations for vectors and
matrices. The key insight was separating the **interface** (what operations
exist) from the **implementation** (how they run on specific hardware).

### The Three Levels of BLAS

```
┌─────────────────────────────────────────────────────────────┐
│ BLAS Level 3 (1990) — Matrix × Matrix — O(n³)              │
│                                                             │
│   GEMM: C = αAB + βC    ← THE MOST IMPORTANT FUNCTION     │
│   SYMM: C = αAB + βC (A symmetric)                         │
│   TRMM: B = αAB (A triangular)                             │
│   Batched GEMM: many GEMMs in parallel                     │
│                                                             │
│   ┌─────────────────────────────────────────────────────┐   │
│   │ BLAS Level 2 (1988) — Matrix × Vector — O(n²)      │   │
│   │                                                     │   │
│   │   GEMV: y = αAx + βy                               │   │
│   │   GER:  A = αxy^T + A  (outer product)             │   │
│   │                                                     │   │
│   │   ┌─────────────────────────────────────────────┐   │   │
│   │   │ BLAS Level 1 (1979) — Vector × Vector — O(n)│   │   │
│   │   │                                             │   │   │
│   │   │   SAXPY: y = αx + y  ← OUR STACK'S HELLO   │   │   │
│   │   │   DOT:   s = x·y     WORLD SINCE LAYER 11  │   │   │
│   │   │   NRM2:  s = ||x||₂                        │   │   │
│   │   │   SCAL:  x = αx                            │   │   │
│   │   │   ASUM:  s = Σ|xᵢ|                         │   │   │
│   │   │   IAMAX: i = argmax|xᵢ|                    │   │   │
│   │   │   COPY:  y = x                             │   │   │
│   │   │   SWAP:  x ↔ y                             │   │   │
│   │   └─────────────────────────────────────────────┘   │   │
│   └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### The Naming Convention

BLAS uses a prefix to indicate precision:

| Prefix | Type | Bits | Example |
|--------|------|------|---------|
| **S** | Single precision float | 32 | SGEMM, SAXPY |
| D | Double precision float | 64 | DGEMM, DAXPY |
| C | Single complex | 2×32 | CGEMM |
| Z | Double complex | 2×64 | ZGEMM |

**We implement S-prefix only (f32)**. This is sufficient for ML (most training
uses f32 or lower) and keeps the implementation focused. Adding D-prefix later
is a mechanical duplication.

### Why GEMM Rules Everything

GEMM (`C = αAB + βC`) is the single most optimized function in all of
computing. NVIDIA employs entire teams just to optimize GEMM for each new
GPU architecture. Why? Because almost everything in ML reduces to GEMM:

```
Neural network operation          How it's computed
─────────────────────────         ─────────────────
Linear layer: y = Wx + b     →   GEMM(W, x) + bias
Convolution                   →   im2col reshape, then GEMM
Attention: softmax(QK^T/√d)V →   GEMM(Q, K^T), scale, softmax, GEMM(scores, V)
Batched inference             →   Batched GEMM
Embedding lookup              →   Sparse GEMM
```

Roughly **70-90% of ML training FLOPs** are matrix multiplications. If you
optimize one function in the entire ML stack, make it GEMM.

### SAXPY: Full Circle

Our running example throughout this entire stack — SAXPY (`y = αx + y`) —
is literally **BLAS Level 1 operation #1**. The name stands for:

- **S** — Single precision (f32)
- **A** — Alpha (the scalar multiplier)
- **X** — First vector
- **P** — Plus
- **Y** — Second vector (also the output)

We've been running BLAS operations since Layer 11 without calling them that.
This layer formalizes it and adds Level 2, Level 3, and ML extensions.

## Design Decisions

### Data Types: Matrix and Vector

We define simple container types that hold shape + flat data. These are
**host-side** containers — each backend is responsible for uploading data to
its device and downloading results.

```python
class StorageOrder(Enum):
    """
    ================================================================
    HOW MATRICES ARE STORED IN MEMORY
    ================================================================

    A 2×3 matrix:
        [ 1  2  3 ]
        [ 4  5  6 ]

    Row-major (C convention):    [1, 2, 3, 4, 5, 6]
        A[i][j] = data[i * cols + j]

    Column-major (Fortran/BLAS): [1, 4, 2, 5, 3, 6]
        A[i][j] = data[j * rows + i]

    We default to row-major because Python, C, and most ML frameworks
    use row-major. Traditional BLAS uses column-major (Fortran heritage).
    ================================================================
    """
    ROW_MAJOR = "row_major"
    COLUMN_MAJOR = "column_major"


class Transpose(Enum):
    """
    ================================================================
    TRANSPOSE FLAGS FOR GEMM AND GEMV
    ================================================================

    When computing C = αAB + βC, you often want to use A^T or B^T
    without physically transposing the matrix. The Transpose flag
    tells the backend to "pretend" the matrix is transposed.

    This is a classic BLAS optimization: instead of allocating a new
    matrix and copying transposed data, you just change the access
    pattern. For a row-major matrix with shape (M, N):
      - NO_TRANS: access as (M, N), stride = N
      - TRANS:    access as (N, M), stride = M
    ================================================================
    """
    NO_TRANS = "no_trans"
    TRANS = "trans"


class Side(Enum):
    """
    ================================================================
    WHICH SIDE THE SPECIAL MATRIX IS ON (FOR SYMM, TRMM)
    ================================================================

    SYMM computes C = αAB + βC where A is symmetric.
    If Side.LEFT:  A is on the left  → C = α(A)B + βC
    If Side.RIGHT: A is on the right → C = αB(A) + βC
    ================================================================
    """
    LEFT = "left"
    RIGHT = "right"


@dataclass
class Vector:
    """
    ================================================================
    A 1-D ARRAY OF SINGLE-PRECISION FLOATS
    ================================================================

    This is the simplest possible vector type. It holds:
    - data: a flat list of f32 values
    - size: how many elements

    It is NOT a tensor. It is NOT a GPU buffer. It lives on the host
    (CPU). Each backend copies it to the device when needed and copies
    results back. This keeps the interface dead simple.
    ================================================================
    """
    data: list[float]
    size: int

    def __post_init__(self):
        if len(self.data) != self.size:
            raise ValueError(
                f"Vector data has {len(self.data)} elements but size={self.size}"
            )


@dataclass
class Matrix:
    """
    ================================================================
    A 2-D ARRAY OF SINGLE-PRECISION FLOATS
    ================================================================

    Stored as a flat list in row-major order by default:

        Matrix(data=[1,2,3,4,5,6], rows=2, cols=3)

        represents:  [ 1  2  3 ]
                     [ 4  5  6 ]

        data[i * cols + j] = element at row i, column j

    The Matrix type is deliberately simple — it's a container for
    moving data between the caller and the BLAS backend. The backend
    handles device memory management internally.
    ================================================================
    """
    data: list[float]
    rows: int
    cols: int
    order: StorageOrder = StorageOrder.ROW_MAJOR

    def __post_init__(self):
        if len(self.data) != self.rows * self.cols:
            raise ValueError(
                f"Matrix data has {len(self.data)} elements "
                f"but shape is {self.rows}×{self.cols} = {self.rows * self.cols}"
            )
```

### Memory Management: Caller Owns Data

The caller creates `Matrix` / `Vector` objects on the host (CPU). Each BLAS
operation is **stateless**: data goes in, result comes out. The backend handles
all device memory internally per-call (allocate, upload, compute, download, free).

This is simpler than real cuBLAS (which requires manual `cudaMalloc` /
`cudaMemcpy`), but it's the right abstraction for a portable BLAS: the caller
shouldn't need to know whether data lives on a GPU or CPU.

```
Caller's perspective:

    A = Matrix([1,2,3,4], 2, 2)
    B = Matrix([5,6,7,8], 2, 2)
    C = Matrix([0,0,0,0], 2, 2)

    result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    # result is a new Matrix on the host. Done.

What the CUDA backend does internally:

    1. cuda.malloc(A.size) → d_A
    2. cuda.malloc(B.size) → d_B
    3. cuda.malloc(C.size) → d_C
    4. cuda.memcpy(d_A, A.data, HostToDevice)
    5. cuda.memcpy(d_B, B.data, HostToDevice)
    6. cuda.memcpy(d_C, C.data, HostToDevice)
    7. cuda.launch_kernel(gemm_kernel, grid, block, [d_A, d_B, d_C, ...])
    8. cuda.device_synchronize()
    9. cuda.memcpy(host_result, d_C, DeviceToHost)
    10. cuda.free(d_A), cuda.free(d_B), cuda.free(d_C)
    11. return Matrix(host_result, rows, cols)

What the CPU backend does internally:

    1. Triple nested loop: for i, for j, for k: C[i][j] += A[i][k] * B[k][j]
    2. return Matrix(result, rows, cols)

The caller doesn't know or care which path ran.
```

### Precision: f32 Only

All operations use single-precision floating point (f32). Function names use
the S prefix (SGEMM, SAXPY, etc.). Adding double precision (D prefix) later
is a mechanical duplication of every operation with `float64` instead of
`float32`.

### Row-Major by Default

We use **row-major** storage order by default because:
- Python, C, C++, and most ML frameworks use row-major
- It's more intuitive: `data[i * cols + j]` for element (i, j)

The `StorageOrder` enum supports column-major for interop with traditional
BLAS, but all examples and tests use row-major.

### Error Handling

| Situation | Error Type | Example |
|-----------|-----------|---------|
| Shape mismatch | `ValueError` | GEMM: A is 3×4, B is 5×6 → "A.cols (4) ≠ B.rows (5)" |
| Unsupported operation | `NotImplementedError` | GPU backend doesn't implement `attention()` |
| Backend init failure | `RuntimeError` | No CUDA device available |
| NaN/Inf in inputs | Propagate silently | Matches IEEE 754 and real BLAS behavior |

## The Abstract Interface: BlasBackend Protocol

Every backend implements this protocol. It defines all the operations that
a BLAS library must provide.

```python
class BlasBackend(Protocol):
    """
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
    # LEVEL 1: VECTOR-VECTOR OPERATIONS — O(n)
    # ==========================================================
    #
    # These operate on pairs of vectors. They are the simplest
    # BLAS operations and were the original 1979 specification.

    def saxpy(self, alpha: float, x: Vector, y: Vector) -> Vector:
        """
        SAXPY: y = αx + y

        The most famous BLAS operation. Our running example since
        Layer 11 (logic gates). Each element:
            result[i] = alpha * x[i] + y[i]

        Requires: x.size == y.size
        Returns: new Vector of same size
        """
        ...

    def sdot(self, x: Vector, y: Vector) -> float:
        """
        DOT product: result = x · y = Σ(xᵢ × yᵢ)

        The dot product is the foundation of similarity measures,
        projections, and matrix multiplication (GEMM is just many
        dot products arranged in a grid).

        Requires: x.size == y.size
        Returns: scalar float
        """
        ...

    def snrm2(self, x: Vector) -> float:
        """
        Euclidean norm: result = ||x||₂ = √(Σ xᵢ²)

        Used in normalization (dividing by the norm), convergence
        checks, and regularization.

        Returns: scalar float ≥ 0
        """
        ...

    def sscal(self, alpha: float, x: Vector) -> Vector:
        """
        Scale: result = αx

        Every element multiplied by the scalar alpha.
            result[i] = alpha * x[i]

        Returns: new Vector of same size
        """
        ...

    def sasum(self, x: Vector) -> float:
        """
        Absolute sum: result = Σ|xᵢ|

        Also known as the L1 norm or Manhattan distance.
        Used in L1 regularization (LASSO).

        Returns: scalar float ≥ 0
        """
        ...

    def isamax(self, x: Vector) -> int:
        """
        Index of max absolute value: result = argmax|xᵢ|

        Returns the INDEX (0-based) of the element with the
        largest absolute value. Used in pivoting for numerical
        stability.

        Returns: integer index (0-based)
        """
        ...

    def scopy(self, x: Vector) -> Vector:
        """
        Copy: result = x

        Creates a deep copy of the vector.

        Returns: new Vector with same data
        """
        ...

    def sswap(self, x: Vector, y: Vector) -> tuple[Vector, Vector]:
        """
        Swap: x ↔ y

        Returns: (new_x, new_y) where new_x has y's data and vice versa

        Requires: x.size == y.size
        """
        ...

    # ==========================================================
    # LEVEL 2: MATRIX-VECTOR OPERATIONS — O(n²)
    # ==========================================================
    #
    # These operate on a matrix and a vector. They were added in
    # the 1988 BLAS Level 2 specification.

    def sgemv(self, trans: Transpose, alpha: float, A: Matrix,
              x: Vector, beta: float, y: Vector) -> Vector:
        """
        General Matrix-Vector multiply: y = αAx + βy

        If trans == TRANS, uses A^T instead of A:
            y = α(A^T)x + βy

        The effective dimensions after transpose:
          NO_TRANS: A is (M×N), x must be size N, y must be size M
          TRANS:    A is (M×N), x must be size M, y must be size N

        Returns: new Vector
        """
        ...

    def sger(self, alpha: float, x: Vector, y: Vector, A: Matrix) -> Matrix:
        """
        Outer product (rank-1 update): A = αxy^T + A

        Every element:
            result[i][j] = alpha * x[i] * y[j] + A[i][j]

        Requires: A.rows == x.size, A.cols == y.size
        Returns: new Matrix of same shape as A
        """
        ...

    # ==========================================================
    # LEVEL 3: MATRIX-MATRIX OPERATIONS — O(n³)
    # ==========================================================
    #
    # These operate on pairs of matrices. Added in the 1990 BLAS
    # Level 3 specification. GEMM is the crown jewel.

    def sgemm(self, trans_a: Transpose, trans_b: Transpose,
              alpha: float, A: Matrix, B: Matrix,
              beta: float, C: Matrix) -> Matrix:
        """
        ============================================================
        GENERAL MATRIX MULTIPLY — THE MOST IMPORTANT FUNCTION
        ============================================================

        C = α × op(A) × op(B) + β × C

        where op(X) = X      if trans == NO_TRANS
              op(X) = X^T    if trans == TRANS

        Dimensions after transpose:
          op(A) is (M × K)
          op(B) is (K × N)
          C     is (M × N)

        Common special cases:
          C = AB        → alpha=1, beta=0
          C = A^T × B   → trans_a=TRANS, alpha=1, beta=0
          C += AB       → alpha=1, beta=1
          C = 2AB + 3C  → alpha=2, beta=3

        70-90% of ML training time is spent in this function.
        ============================================================
        """
        ...

    def ssymm(self, side: Side, alpha: float, A: Matrix, B: Matrix,
              beta: float, C: Matrix) -> Matrix:
        """
        Symmetric Matrix Multiply: C = αAB + βC (A is symmetric)

        If side == LEFT:  C = αAB + βC
        If side == RIGHT: C = αBA + βC

        A must be square and symmetric (A[i][j] == A[j][i]).
        The backend only reads the lower triangle of A.

        Returns: new Matrix of same shape as C
        """
        ...

    def sgemm_batched(self, trans_a: Transpose, trans_b: Transpose,
                      alpha: float, As: list[Matrix], Bs: list[Matrix],
                      beta: float, Cs: list[Matrix]) -> list[Matrix]:
        """
        Batched GEMM: multiple independent GEMMs in parallel.

            Cs[i] = α × op(As[i]) × op(Bs[i]) + β × Cs[i]

        All matrices in each batch must have the same dimensions.
        The backend can execute all GEMMs in parallel if the device
        supports it.

        Used for: multi-head attention, batched inference, etc.

        Requires: len(As) == len(Bs) == len(Cs)
        Returns: list of new Matrices
        """
        ...
```

## ML Extensions: MlBlasBackend Protocol

An **optional** extension protocol for operations needed by ML frameworks
but not part of classic BLAS. Backends that implement this protocol can
accelerate neural network operations directly.

```python
class MlBlasBackend(BlasBackend, Protocol):
    """
    ================================================================
    ML EXTENSIONS BEYOND CLASSIC BLAS
    ================================================================

    Classic BLAS handles linear algebra. ML needs additional
    operations: activation functions, normalization, convolution,
    and attention. These operations CAN be built from BLAS primitives
    (attention = two GEMMs + softmax), but dedicated implementations
    are much faster.

    This protocol is OPTIONAL. A backend that only implements
    BlasBackend is still a valid BLAS backend. The registry can
    check whether a backend supports ML extensions:

        if isinstance(backend, MlBlasBackend):
            result = backend.relu(matrix)
        else:
            # Fall back to manual implementation
            result = Matrix([max(0, x) for x in matrix.data], ...)
    ================================================================
    """

    # ==========================================================
    # ACTIVATION FUNCTIONS (element-wise)
    # ==========================================================

    def relu(self, x: Matrix) -> Matrix:
        """ReLU: result[i] = max(0, x[i])"""
        ...

    def gelu(self, x: Matrix) -> Matrix:
        """GELU: result[i] = x[i] × Φ(x[i]) where Φ is the CDF of N(0,1)"""
        ...

    def sigmoid(self, x: Matrix) -> Matrix:
        """Sigmoid: result[i] = 1 / (1 + exp(-x[i]))"""
        ...

    def tanh_activation(self, x: Matrix) -> Matrix:
        """Tanh: result[i] = tanh(x[i])"""
        ...

    # ==========================================================
    # SOFTMAX
    # ==========================================================

    def softmax(self, x: Matrix, axis: int = -1) -> Matrix:
        """
        Softmax along an axis:
            result[i] = exp(x[i]) / Σ exp(x[j])

        The numerically stable version subtracts the max first:
            result[i] = exp(x[i] - max(x)) / Σ exp(x[j] - max(x))

        axis=-1 means last axis (columns for 2D matrix).
        """
        ...

    # ==========================================================
    # NORMALIZATION
    # ==========================================================

    def layer_norm(self, x: Matrix, gamma: Vector, beta: Vector,
                   eps: float = 1e-5) -> Matrix:
        """
        Layer Normalization (Ba et al., 2016):

        For each row (sample):
            mean = Σ x[i] / n
            var  = Σ (x[i] - mean)² / n
            result[i] = gamma[i] * (x[i] - mean) / √(var + eps) + beta[i]

        Used in: Transformers, GPT, BERT
        """
        ...

    def batch_norm(self, x: Matrix, gamma: Vector, beta: Vector,
                   running_mean: Vector, running_var: Vector,
                   eps: float = 1e-5, training: bool = False) -> Matrix:
        """
        Batch Normalization (Ioffe & Szegedy, 2015):

        Across columns (features over the batch):
            mean = Σ x[i] / batch_size
            var  = Σ (x[i] - mean)² / batch_size
            result[i] = gamma * (x[i] - mean) / √(var + eps) + beta

        Used in: CNNs, ResNets
        """
        ...

    # ==========================================================
    # CONVOLUTION
    # ==========================================================

    def conv2d(self, input: Matrix, weight: Matrix, bias: Vector | None = None,
               stride: int = 1, padding: int = 0) -> Matrix:
        """
        2D Convolution via im2col + GEMM:

        1. Reshape input using im2col: extract all patches into columns
        2. Reshape weight into rows
        3. Result = GEMM(weight_matrix, im2col_matrix) + bias

        This is how cuDNN and most frameworks implement convolution.

        input shape:  (batch_size × input_channels × height × width) flattened
        weight shape: (out_channels × in_channels × kH × kW) flattened

        Note: Since our Matrix is 2D, we encode 4D tensors with explicit
        dimension metadata. This is a simplified conv2d for demonstration.
        """
        ...

    # ==========================================================
    # ATTENTION
    # ==========================================================

    def attention(self, Q: Matrix, K: Matrix, V: Matrix,
                  mask: Matrix | None = None,
                  scale: float | None = None) -> Matrix:
        """
        Scaled Dot-Product Attention (Vaswani et al., 2017):

            Attention(Q, K, V) = softmax(QK^T / √d_k) × V

        Steps:
        1. scores = SGEMM(Q, K^T) / scale     ← BLAS Level 3
        2. if mask: scores += mask             ← element-wise
        3. weights = softmax(scores, axis=-1)  ← ML extension
        4. output = SGEMM(weights, V)          ← BLAS Level 3

        This is the core operation of Transformers.

        Q shape: (seq_len × d_k)
        K shape: (seq_len × d_k)
        V shape: (seq_len × d_v)
        Returns: (seq_len × d_v)
        """
        ...
```

## Backend Registry

The registry manages available backends and selects the best one:

```python
class BackendRegistry:
    """
    ================================================================
    BACKEND REGISTRY — FIND AND SELECT BLAS BACKENDS
    ================================================================

    The registry keeps track of which backends are available and
    helps the caller pick one. Three modes of selection:

    1. EXPLICIT:    registry.get("cuda")
    2. AUTO-DETECT: registry.get_best()
    3. CUSTOM:      registry.register("my_backend", MyBlas)

    Auto-detection priority (customizable):
        cuda > metal > vulkan > opencl > webgpu > opengl > cpu

    CUDA is first because it's the most optimized for ML.
    Metal is second because Apple silicon has unified memory.
    CPU is always last — it's the universal fallback.
    ================================================================
    """

    _backends: dict[str, type[BlasBackend]]
    _priority: list[str]
    _default_priority = ["cuda", "metal", "vulkan", "opencl",
                         "webgpu", "opengl", "cpu"]

    def register(self, name: str, backend_class: type[BlasBackend]):
        """Register a backend by name."""
        ...

    def get(self, name: str) -> BlasBackend:
        """Get a specific backend by name. Raises RuntimeError if not found."""
        ...

    def get_best(self) -> BlasBackend:
        """
        Try each backend in priority order. Return the first one
        that successfully initializes. CPU always works.
        """
        ...

    def list_available(self) -> list[str]:
        """List names of all registered backends."""
        ...

    def set_priority(self, priority: list[str]):
        """Change the auto-detection priority order."""
        ...
```

### Convenience API

A module-level API for the simplest possible usage:

```python
# Module-level convenience functions
def create_blas(backend: str = "auto") -> BlasBackend:
    """
    Create a BLAS instance.

    backend="auto" selects the best available.
    backend="cuda" selects CUDA specifically.
    backend="cpu" selects CPU specifically.
    """
    ...

# Context manager for temporary backend selection
@contextmanager
def backend(name: str):
    """
    Temporarily set the active backend:

        with blas.backend("cpu"):
            result = blas.sgemm(A, B)
    """
    ...
```

## The Seven Backends

### Backend 1: CpuBlas — Pure CPU Reference Implementation

The simplest backend. No GPU, no Layer 4 dependency. Pure language-level
loops over arrays. This backend serves two purposes:

1. **Universal fallback** — works everywhere, no hardware requirements
2. **Reference implementation** — other backends are tested against it

```
CpuBlas
    - SAXPY: for i in range(n): y[i] = alpha * x[i] + y[i]
    - GEMM:  triple nested loop with transpose handling
    - DOT:   sum of element-wise products
    - All operations are O(n), O(n²), or O(n³) as expected
    - Implements MlBlasBackend (all ML extensions)
    - No external dependencies
```

The CPU backend implements **all** operations including ML extensions
(using `math.exp`, `math.sqrt`, etc.). It's slow but correct.

### Backend 2: CudaBlas — NVIDIA CUDA

Wraps `CUDARuntime` from Layer 4. The most commonly used GPU backend
for ML workloads.

```
CudaBlas
    For each BLAS operation:
    1. cuda.malloc() for input + output buffers
    2. cuda.memcpy(HostToDevice) to upload data
    3. cuda.launch_kernel(blas_kernel, grid, block, args) to compute
    4. cuda.device_synchronize() to wait
    5. cuda.memcpy(DeviceToHost) to download results
    6. cuda.free() to clean up
    7. Return new Matrix/Vector with results

    Kernel code uses Layer 9 gpu-core instructions (FMA, FMUL, FADD, LOAD, STORE)
```

### Backend 3: OpenClBlas — Portable OpenCL

Wraps `CLContext` + `CLCommandQueue` from Layer 4. The most portable
GPU backend — works on NVIDIA, AMD, Intel, and even CPUs.

```
OpenClBlas
    1. ctx.create_buffer() for inputs + outputs
    2. queue.enqueue_write_buffer() to upload data
    3. program = ctx.create_program_with_source(); program.build()
    4. kernel = program.create_kernel(); kernel.set_arg(...)
    5. queue.enqueue_nd_range_kernel(kernel, global_size, local_size)
    6. queue.enqueue_read_buffer() to download results
    7. queue.finish() to wait
```

### Backend 4: MetalBlas — Apple Metal

Wraps `MTLDevice` from Layer 4. Optimized for Apple Silicon's unified
memory — no host-to-device copies needed.

```
MetalBlas
    1. device.make_buffer() — unified memory, write directly
    2. buf.write_bytes(data) — no cudaMemcpy needed!
    3. Create pipeline state, command buffer, compute encoder
    4. encoder.set_buffer(), encoder.dispatch_threadgroups()
    5. cb.commit(), cb.wait_until_completed()
    6. Read directly from buffer.contents()
```

### Backend 5: VulkanBlas — Vulkan

Wraps `VulkanRuntime` from Layer 4. The most explicit and verbose
backend, but offers maximum control.

```
VulkanBlas
    1. vk_create_buffer() + vk_allocate_memory() + vk_bind_buffer_memory()
    2. vk_map_memory() to write data, vk_unmap_memory()
    3. Create shader module, descriptor set layout, pipeline layout,
       compute pipeline, descriptor set, write descriptors
    4. vk_allocate_command_buffers(), vk_begin_command_buffer()
    5. vk_cmd_bind_pipeline(), vk_cmd_bind_descriptor_sets(), vk_cmd_dispatch()
    6. vk_end_command_buffer(), vk_queue_submit(), vk_wait_for_fences()
    7. Read back results
```

### Backend 6: WebGpuBlas — WebGPU

Wraps `GPUDevice` from Layer 4. Designed for browser-based compute,
with automatic synchronization and simple buffer management.

```
WebGpuBlas
    1. device.create_buffer() with STORAGE | COPY_DST usage
    2. device.queue.write_buffer() to upload data
    3. Create shader module, compute pipeline, bind group
    4. encoder = device.create_command_encoder()
    5. pass.set_pipeline(), pass.set_bind_group(), pass.dispatch_workgroups()
    6. device.queue.submit([encoder.finish()])
    7. Map result buffer and read back
```

### Backend 7: OpenGlBlas — OpenGL Compute

Wraps `GLContext` from Layer 4. Uses the global state machine model
with shader storage buffer objects (SSBOs).

```
OpenGlBlas
    1. gl.gen_buffers(), gl.buffer_data() for SSBOs
    2. gl.bind_buffer_base() to indexed binding points
    3. Create shader + program, gl.use_program()
    4. gl.dispatch_compute()
    5. gl.memory_barrier()
    6. gl.map_buffer_range() to read results
    7. gl.finish()
```

## How Backends Create GPU Kernels

Each BLAS operation translates into a GPU kernel that runs on the Layer 9
`gpu-core` instruction set. Here are the key kernels:

### SAXPY Kernel (Level 1)

```
# Each thread handles one element
# thread_id = global thread index

LOAD  r0, [x_addr + thread_id * 4]     # r0 = x[i]
LOAD  r1, [y_addr + thread_id * 4]     # r1 = y[i]
FMA   r2, alpha, r0, r1                # r2 = alpha * x[i] + y[i]
STORE [out_addr + thread_id * 4], r2   # out[i] = r2
```

### DOT Product Kernel (Level 1)

```
# Each thread computes one partial sum, then reduce
# This is a simplified version — real implementations use
# shared memory reduction trees

LOAD  r0, [x_addr + thread_id * 4]     # r0 = x[i]
LOAD  r1, [y_addr + thread_id * 4]     # r1 = y[i]
FMUL  r2, r0, r1                       # r2 = x[i] * y[i]
STORE [partial_addr + thread_id * 4], r2
# Host sums the partial results
```

### GEMM Kernel (Level 3) — Naive Version

```
# Each thread computes one element C[row][col]
# row = thread_id / N
# col = thread_id % N

sum = 0.0
for k in range(K):
    LOAD  r0, [A_addr + (row * K + k) * 4]   # r0 = A[row][k]
    LOAD  r1, [B_addr + (k * N + col) * 4]   # r1 = B[k][col]
    FMA   sum, r0, r1, sum                    # sum += A[row][k] * B[k][col]

# Apply alpha and beta
FMUL  sum, alpha, sum                         # sum *= alpha
LOAD  r3, [C_addr + (row * N + col) * 4]     # r3 = C[row][col]
FMA   result, beta, r3, sum                   # result = beta * C + alpha * AB
STORE [out_addr + (row * N + col) * 4], result
```

### GEMM Kernel (Level 3) — Tiled Version

Real GEMM implementations use **tiling** to exploit shared memory:

```
For each tile of C (TILE_SIZE × TILE_SIZE):
    1. Load a TILE_SIZE × TILE_SIZE block of A into shared memory
    2. Load a TILE_SIZE × TILE_SIZE block of B into shared memory
    3. Synchronize threads (barrier)
    4. Each thread computes its element's contribution from this tile
    5. Accumulate into a register
    6. Move to next tile along the K dimension
Write final accumulated value to global memory
```

We implement the naive version for correctness. The tiled version can
be added as an optimization later.

## End-to-End Example: GEMM Through All Backends

The capstone test — run the same matrix multiplication through all 7
backends and verify they all produce the same result:

```python
def test_gemm_all_backends():
    """
    Compute C = AB through all 7 backends.
    All must produce identical results (within FP tolerance).

        A = [ 1  2  3 ]    B = [ 7   8  ]    C = [ 58   64  ]
            [ 4  5  6 ]        [ 9  10  ]        [ 139  154 ]
                                [ 11  12 ]
    """
    A = Matrix(data=[1, 2, 3, 4, 5, 6], rows=2, cols=3)
    B = Matrix(data=[7, 8, 9, 10, 11, 12], rows=3, cols=2)
    C = Matrix(data=[0, 0, 0, 0], rows=2, cols=2)

    expected = [58.0, 64.0, 139.0, 154.0]

    backends = ["cpu", "cuda", "opencl", "metal",
                "vulkan", "webgpu", "opengl"]

    for name in backends:
        blas = create_blas(name)
        result = blas.sgemm(
            Transpose.NO_TRANS, Transpose.NO_TRANS,
            1.0, A, B, 0.0, C
        )
        for i, (got, want) in enumerate(zip(result.data, expected)):
            assert abs(got - want) < 1e-5, (
                f"{name}: element {i} = {got}, expected {want}"
            )
```

## Testing Strategy

### Per-Backend Tests (40+ per backend)

| Category | Tests | What They Verify |
|----------|-------|-----------------|
| Level 1 | 8+ | SAXPY, DOT, NRM2, SCAL, ASUM, IAMAX, COPY, SWAP |
| Level 2 | 4+ | GEMV (no trans + trans), GER |
| Level 3 | 8+ | GEMM (4 transpose combos), SYMM (left + right), batched GEMM |
| Edge cases | 6+ | alpha=0, beta=0, beta=1, 1×1 matrices, identity multiply |
| Errors | 5+ | Shape mismatches, invalid dimensions |
| ML extensions | 10+ | ReLU, sigmoid, softmax, layer_norm, attention (CPU + GPU) |

### Cross-Backend Equivalence Test

Run every BLAS operation through all 7 backends and verify matching results
(within 1e-5 relative tolerance for FP ordering differences).

### Registry Tests

- Register and retrieve backends
- Auto-detect returns best available
- Custom priority ordering
- Unknown backend name raises error

Target: **95%+ coverage** for each backend, 40+ tests per backend.

## Package Structure

```
blas-library/
    pyproject.toml
    BUILD
    README.md
    CHANGELOG.md
    src/blas_library/
        __init__.py               # Public API: create_blas(), Matrix, Vector
        _types.py                 # Matrix, Vector, StorageOrder, Transpose, Side
        _protocol.py              # BlasBackend, MlBlasBackend protocols
        _registry.py              # BackendRegistry, auto-detect
        _convenience.py           # Module-level blas.sgemm() style functions
        backends/
            __init__.py           # Re-exports
            cpu.py                # CpuBlas — pure language, no GPU
            cuda.py               # CudaBlas — wraps CUDARuntime
            opencl.py             # OpenClBlas — wraps CLContext
            metal.py              # MetalBlas — wraps MTLDevice
            vulkan.py             # VulkanBlas — wraps VulkanRuntime
            webgpu.py             # WebGpuBlas — wraps GPUDevice
            opengl.py             # OpenGlBlas — wraps GLContext
    tests/
        conftest.py               # Shared fixtures, test matrices
        test_types.py             # Matrix/Vector creation, validation
        test_registry.py          # Backend registration, auto-detect
        test_cpu_blas.py          # Full suite for CPU backend
        test_cuda_blas.py         # Full suite for CUDA backend
        test_opencl_blas.py       # Full suite for OpenCL backend
        test_metal_blas.py        # Full suite for Metal backend
        test_vulkan_blas.py       # Full suite for Vulkan backend
        test_webgpu_blas.py       # Full suite for WebGPU backend
        test_opengl_blas.py       # Full suite for OpenGL backend
        test_ml_extensions.py     # ML ops (CPU + GPU backends)
        test_cross_backend.py     # Same computation through all 7
```

## Implementation Order

1. `_types.py` — Matrix, Vector, enums (no dependencies)
2. `_protocol.py` — BlasBackend and MlBlasBackend protocols
3. `_registry.py` — BackendRegistry
4. `backends/cpu.py` — CpuBlas (reference implementation)
5. `test_cpu_blas.py` — full test suite against the reference
6. `backends/cuda.py` — CudaBlas
7. `backends/metal.py` — MetalBlas
8. `backends/vulkan.py` — VulkanBlas
9. `backends/opencl.py` — OpenClBlas
10. `backends/webgpu.py` — WebGpuBlas
11. `backends/opengl.py` — OpenGlBlas
12. `_convenience.py` + `__init__.py` — public API
13. `test_cross_backend.py` — equivalence test (capstone)

## Dependencies

```
blas-library
    ├── vendor-api-simulators (Layer 4)  # for GPU backends
    │   └── compute-runtime (Layer 5)    # transitively
    │       └── device-simulator (Layer 6)
    │           └── ... (all the way down to logic gates)
    └── (no dependency for CPU backend — pure language)
```
