"""
================================================================
DEVICE MANAGER — MAPS DEVICE STRINGS TO BLAS BACKENDS
================================================================

When you write tensor.to("cuda"), something needs to know that
"cuda" means "use the CudaBlas backend from the BLAS library."
That's what DeviceManager does.

Device strings map 1:1 to BLAS backends:
    "cpu"    → CpuBlas   (pure Python reference, always available)
    "cuda"   → CudaBlas  (NVIDIA GPUs)
    "metal"  → MetalBlas (Apple Silicon)
    "vulkan" → VulkanBlas
    "opencl" → OpenClBlas
    "webgpu" → WebGpuBlas
    "opengl" → OpenGlBlas

The default device is "cpu" — it always works, no hardware needed.
================================================================
"""

from __future__ import annotations

from typing import Any


class DeviceManager:
    """Manages device-to-backend mapping using the BLAS library's registry."""

    _default_device: str = "cpu"
    _backends: dict[str, Any] = {}

    @classmethod
    def get_backend(cls, device: str) -> Any:
        """Get a BLAS backend for the given device string.

        Lazily creates backends on first request.
        """
        if device not in cls._backends:
            cls._backends[device] = cls._create_backend(device)
        return cls._backends[device]

    @classmethod
    def _create_backend(cls, device: str) -> Any:
        """Create a BLAS backend instance for the given device."""
        try:
            from blas_library import create_blas

            return create_blas(device)
        except ImportError:
            # If blas_library isn't available, return None
            # Operations will fall back to pure Python
            return None

    @classmethod
    def get_default_device(cls) -> str:
        """Get the current default device."""
        return cls._default_device

    @classmethod
    def set_default_device(cls, device: str) -> None:
        """Set the default device for new tensors."""
        cls._default_device = device

    @classmethod
    def reset(cls) -> None:
        """Reset to defaults (useful for testing)."""
        cls._default_device = "cpu"
        cls._backends.clear()
