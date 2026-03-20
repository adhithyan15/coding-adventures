"""BLAS Library — pluggable linear algebra with 7 swappable backends.

=== Quick Start ===

    from blas_library import create_blas, Matrix, Vector, Transpose

    # Create a BLAS instance (auto-selects best backend)
    blas = create_blas("auto")

    # Or pick a specific backend
    blas = create_blas("cpu")    # Pure Python reference
    blas = create_blas("cuda")   # NVIDIA GPU
    blas = create_blas("metal")  # Apple Silicon

    # Matrix multiply: C = A * B
    A = Matrix(data=[1, 2, 3, 4, 5, 6], rows=2, cols=3)
    B = Matrix(data=[7, 8, 9, 10, 11, 12], rows=3, cols=2)
    C = Matrix(data=[0, 0, 0, 0], rows=2, cols=2)

    result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, A, B, 0.0, C)
    # result.data == [58.0, 64.0, 139.0, 154.0]

=== What is BLAS? ===

BLAS (Basic Linear Algebra Subprograms) is a specification for standard
linear algebra operations. Published in 1979, it defines vector and matrix
operations at three levels:

    Level 1 (1979): Vector-Vector — O(n)    — SAXPY, DOT, NRM2, SCAL...
    Level 2 (1988): Matrix-Vector — O(n^2)  — GEMV, GER
    Level 3 (1990): Matrix-Matrix — O(n^3)  — GEMM, SYMM, Batched GEMM

Plus ML extensions: ReLU, softmax, layer norm, attention, conv2d.

The key insight: separate the INTERFACE (what operations exist) from the
IMPLEMENTATION (how they run on specific hardware). Write ``blas.sgemm()``
once, run it on any GPU or CPU.
"""

# Types
# Convenience
from ._convenience import create_blas, use_backend

# Protocols
from ._protocol import BlasBackend, MlBlasBackend

# Registry
from ._registry import BackendRegistry, global_registry
from ._types import (
    Matrix,
    Side,
    StorageOrder,
    Transpose,
    Vector,
    from_matrix_pkg,
    to_matrix_pkg,
)

# Backends
from .backends import (
    CpuBlas,
    CudaBlas,
    MetalBlas,
    OpenClBlas,
    OpenGlBlas,
    VulkanBlas,
    WebGpuBlas,
)

# =========================================================================
# Register all backends in the global registry
# =========================================================================
#
# This happens at import time. Each backend class is registered by name.
# When create_blas("cuda") is called, the registry instantiates CudaBlas().
# If instantiation fails (e.g., no GPU), get_best() silently skips it.

global_registry.register("cpu", CpuBlas)
global_registry.register("cuda", CudaBlas)
global_registry.register("opencl", OpenClBlas)
global_registry.register("metal", MetalBlas)
global_registry.register("vulkan", VulkanBlas)
global_registry.register("webgpu", WebGpuBlas)
global_registry.register("opengl", OpenGlBlas)

__all__ = [
    # Types
    "Matrix",
    "Vector",
    "StorageOrder",
    "Transpose",
    "Side",
    "from_matrix_pkg",
    "to_matrix_pkg",
    # Protocols
    "BlasBackend",
    "MlBlasBackend",
    # Registry
    "BackendRegistry",
    "global_registry",
    # Convenience
    "create_blas",
    "use_backend",
    # Backends
    "CpuBlas",
    "CudaBlas",
    "OpenClBlas",
    "MetalBlas",
    "VulkanBlas",
    "WebGpuBlas",
    "OpenGlBlas",
]
