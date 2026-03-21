"""Shared test fixtures and reference data for BLAS library tests.

=== What Lives Here ===

This module provides pytest fixtures that are shared across all test files:

1. Backend fixtures — instantiate each of the 7 backends
2. Reference matrices — known inputs with pre-computed expected results
3. Helper functions — approximate comparison for floating-point results
"""

from __future__ import annotations

import pytest

from blas_library import (
    CpuBlas,
    CudaBlas,
    Matrix,
    MetalBlas,
    OpenClBlas,
    OpenGlBlas,
    Vector,
    VulkanBlas,
    WebGpuBlas,
)

# =========================================================================
# Backend fixtures
# =========================================================================


@pytest.fixture
def cpu_blas() -> CpuBlas:
    """Create a CPU BLAS backend."""
    return CpuBlas()


@pytest.fixture
def cuda_blas() -> CudaBlas:
    """Create a CUDA BLAS backend."""
    return CudaBlas()


@pytest.fixture
def opencl_blas() -> OpenClBlas:
    """Create an OpenCL BLAS backend."""
    return OpenClBlas()


@pytest.fixture
def metal_blas() -> MetalBlas:
    """Create a Metal BLAS backend."""
    return MetalBlas()


@pytest.fixture
def vulkan_blas() -> VulkanBlas:
    """Create a Vulkan BLAS backend."""
    return VulkanBlas()


@pytest.fixture
def webgpu_blas() -> WebGpuBlas:
    """Create a WebGPU BLAS backend."""
    return WebGpuBlas()


@pytest.fixture
def opengl_blas() -> OpenGlBlas:
    """Create an OpenGL BLAS backend."""
    return OpenGlBlas()


@pytest.fixture
def all_backends(
    cpu_blas: CpuBlas,
    cuda_blas: CudaBlas,
    opencl_blas: OpenClBlas,
    metal_blas: MetalBlas,
    vulkan_blas: VulkanBlas,
    webgpu_blas: WebGpuBlas,
    opengl_blas: OpenGlBlas,
) -> list[object]:
    """All 7 backends for cross-backend tests."""
    return [
        cpu_blas,
        cuda_blas,
        opencl_blas,
        metal_blas,
        vulkan_blas,
        webgpu_blas,
        opengl_blas,
    ]


# =========================================================================
# Reference data fixtures
# =========================================================================


@pytest.fixture
def vec_x() -> Vector:
    """Reference vector x = [1, 2, 3, 4]."""
    return Vector(data=[1.0, 2.0, 3.0, 4.0], size=4)


@pytest.fixture
def vec_y() -> Vector:
    """Reference vector y = [5, 6, 7, 8]."""
    return Vector(data=[5.0, 6.0, 7.0, 8.0], size=4)


@pytest.fixture
def mat_a() -> Matrix:
    """Reference matrix A (2x3):
    [ 1  2  3 ]
    [ 4  5  6 ]
    """
    return Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)


@pytest.fixture
def mat_b() -> Matrix:
    """Reference matrix B (3x2):
    [ 7   8  ]
    [ 9  10  ]
    [ 11  12 ]
    """
    return Matrix(data=[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], rows=3, cols=2)


@pytest.fixture
def mat_c_2x2() -> Matrix:
    """Zero matrix C (2x2) for GEMM results."""
    return Matrix(data=[0.0, 0.0, 0.0, 0.0], rows=2, cols=2)


@pytest.fixture
def mat_square() -> Matrix:
    """A 2x2 symmetric matrix:
    [ 1  2 ]
    [ 2  1 ]
    """
    return Matrix(data=[1.0, 2.0, 2.0, 1.0], rows=2, cols=2)


@pytest.fixture
def identity_2x2() -> Matrix:
    """2x2 identity matrix."""
    return Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)


# =========================================================================
# Helper functions
# =========================================================================


def approx_equal(a: float, b: float, tol: float = 1e-5) -> bool:
    """Check if two floats are approximately equal."""
    return abs(a - b) < tol


def approx_list(a: list[float], b: list[float], tol: float = 1e-5) -> bool:
    """Check if two lists of floats are element-wise approximately equal."""
    if len(a) != len(b):
        return False
    return all(abs(x - y) < tol for x, y in zip(a, b, strict=False))
