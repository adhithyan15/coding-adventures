"""Tests for the BackendRegistry — registration, lookup, auto-detect."""

from __future__ import annotations

import pytest

from blas_library import BackendRegistry, CpuBlas, create_blas, use_backend
from blas_library._registry import global_registry

# =========================================================================
# BackendRegistry tests
# =========================================================================


class TestBackendRegistry:
    """Tests for BackendRegistry."""

    def test_register_and_get(self) -> None:
        """Register a backend and retrieve it."""
        reg = BackendRegistry()
        reg.register("cpu", CpuBlas)
        blas = reg.get("cpu")
        assert blas.name == "cpu"

    def test_get_unknown_raises(self) -> None:
        """Getting an unregistered backend raises RuntimeError."""
        reg = BackendRegistry()
        with pytest.raises(RuntimeError, match="not registered"):
            reg.get("nonexistent")

    def test_list_available(self) -> None:
        """List registered backends."""
        reg = BackendRegistry()
        reg.register("cpu", CpuBlas)
        assert "cpu" in reg.list_available()

    def test_list_available_empty(self) -> None:
        """Empty registry lists nothing."""
        reg = BackendRegistry()
        assert reg.list_available() == []

    def test_get_best_returns_highest_priority(self) -> None:
        """get_best() should return the first available in priority order."""
        reg = BackendRegistry()
        reg.register("cpu", CpuBlas)
        blas = reg.get_best()
        assert blas.name == "cpu"

    def test_get_best_empty_raises(self) -> None:
        """get_best() with no backends raises RuntimeError."""
        reg = BackendRegistry()
        with pytest.raises(RuntimeError, match="No BLAS backend"):
            reg.get_best()

    def test_set_priority(self) -> None:
        """Custom priority order is respected."""
        reg = BackendRegistry()
        reg.register("cpu", CpuBlas)
        reg.set_priority(["cpu"])
        blas = reg.get_best()
        assert blas.name == "cpu"

    def test_multiple_registrations(self) -> None:
        """Multiple backends can be registered."""
        reg = BackendRegistry()
        reg.register("cpu", CpuBlas)
        reg.register("cpu2", CpuBlas)
        assert len(reg.list_available()) == 2

    def test_overwrite_registration(self) -> None:
        """Re-registering a name overwrites the previous backend."""
        reg = BackendRegistry()
        reg.register("test", CpuBlas)
        reg.register("test", CpuBlas)
        assert reg.list_available().count("test") == 1


# =========================================================================
# Global registry tests
# =========================================================================


class TestGlobalRegistry:
    """Tests for the global registry (populated at import time)."""

    def test_all_backends_registered(self) -> None:
        """All 7 backends should be registered globally."""
        available = global_registry.list_available()
        for name in ["cpu", "cuda", "opencl", "metal", "vulkan", "webgpu", "opengl"]:
            assert name in available, f"{name} not in global registry"

    def test_get_cpu(self) -> None:
        """Can get the CPU backend from global registry."""
        blas = global_registry.get("cpu")
        assert blas.name == "cpu"


# =========================================================================
# Convenience API tests
# =========================================================================


class TestConvenienceAPI:
    """Tests for create_blas() and use_backend()."""

    def test_create_blas_cpu(self) -> None:
        """create_blas('cpu') returns a CPU backend."""
        blas = create_blas("cpu")
        assert blas.name == "cpu"

    def test_create_blas_auto(self) -> None:
        """create_blas('auto') returns some backend."""
        blas = create_blas("auto")
        assert blas.name in [
            "cpu",
            "cuda",
            "opencl",
            "metal",
            "vulkan",
            "webgpu",
            "opengl",
        ]

    def test_create_blas_unknown_raises(self) -> None:
        """create_blas with unknown name raises RuntimeError."""
        with pytest.raises(RuntimeError):
            create_blas("nonexistent_backend")

    def test_use_backend_context_manager(self) -> None:
        """use_backend() context manager creates and yields a backend."""
        with use_backend("cpu") as blas:
            assert blas.name == "cpu"

    def test_create_blas_cuda(self) -> None:
        """create_blas('cuda') returns a CUDA backend."""
        blas = create_blas("cuda")
        assert blas.name == "cuda"

    def test_create_blas_metal(self) -> None:
        """create_blas('metal') returns a Metal backend."""
        blas = create_blas("metal")
        assert blas.name == "metal"

    def test_create_blas_vulkan(self) -> None:
        """create_blas('vulkan') returns a Vulkan backend."""
        blas = create_blas("vulkan")
        assert blas.name == "vulkan"

    def test_create_blas_opencl(self) -> None:
        """create_blas('opencl') returns an OpenCL backend."""
        blas = create_blas("opencl")
        assert blas.name == "opencl"

    def test_create_blas_webgpu(self) -> None:
        """create_blas('webgpu') returns a WebGPU backend."""
        blas = create_blas("webgpu")
        assert blas.name == "webgpu"

    def test_create_blas_opengl(self) -> None:
        """create_blas('opengl') returns an OpenGL backend."""
        blas = create_blas("opengl")
        assert blas.name == "opengl"
