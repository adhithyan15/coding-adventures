"""Convenience API — simple module-level functions for common usage.

=== The Simplest Way to Use BLAS ===

Instead of manually creating backends and calling methods:

    from blas_library.backends.cpu import CpuBlas
    blas = CpuBlas()
    result = blas.sgemm(...)

You can use the convenience API:

    from blas_library import create_blas, backend

    blas = create_blas("auto")     # Best available backend
    blas = create_blas("cuda")     # Specific backend
    blas = create_blas("cpu")      # CPU fallback

    # Temporary backend switching
    with backend("cpu") as blas:
        result = blas.sgemm(...)
"""

from __future__ import annotations

from contextlib import contextmanager
from typing import TYPE_CHECKING

from ._registry import global_registry

if TYPE_CHECKING:
    from collections.abc import Generator

    from ._protocol import BlasBackend


def create_blas(backend_name: str = "auto") -> BlasBackend:
    """Create a BLAS instance with the specified backend.

    ================================================================
    CREATE A BLAS BACKEND INSTANCE
    ================================================================

    This is the main entry point for the BLAS library. It creates
    and returns a backend instance:

        "auto"   — selects the best available backend by priority
        "cuda"   — NVIDIA GPU
        "metal"  — Apple Silicon
        "vulkan" — any Vulkan-capable GPU
        "opencl" — any OpenCL device
        "webgpu" — WebGPU-capable device
        "opengl" — OpenGL 4.3+ device
        "cpu"    — pure Python fallback (always works)

    Args:
        backend_name: Which backend to use. Default "auto".

    Returns:
        An instantiated BlasBackend.

    Raises:
        RuntimeError: If the requested backend is not available.
    ================================================================
    """
    if backend_name == "auto":
        return global_registry.get_best()
    return global_registry.get(backend_name)


@contextmanager
def use_backend(name: str) -> Generator[BlasBackend, None, None]:
    """Context manager for temporary backend selection.

    ================================================================
    TEMPORARY BACKEND SWITCHING
    ================================================================

    Use this when you want to temporarily switch backends:

        with use_backend("cpu") as blas:
            result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)

    The backend is created on entry and goes out of scope on exit.
    This is useful for testing (compare results across backends)
    or for fallback handling (try GPU, fall back to CPU).
    ================================================================
    """
    blas = create_blas(name)
    yield blas
