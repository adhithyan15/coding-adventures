"""BLAS Backends — seven interchangeable implementations.

=== The Seven Backends ===

    CpuBlas    — pure Python reference (no GPU, no dependencies)
    CudaBlas   — wraps CUDA Runtime (NVIDIA GPUs)
    OpenClBlas — wraps OpenCL Context (any OpenCL device)
    MetalBlas  — wraps Metal Device (Apple Silicon)
    VulkanBlas — wraps Vulkan Runtime (any Vulkan device)
    WebGpuBlas — wraps WebGPU Device (browsers + native)
    OpenGlBlas — wraps OpenGL Context (OpenGL 4.3+)

All seven implement the ``BlasBackend`` protocol and produce the same
results (within floating-point tolerance). The CPU backend also implements
``MlBlasBackend`` for ML extensions.
"""

from .cpu import CpuBlas
from .cuda import CudaBlas
from .metal import MetalBlas
from .opencl import OpenClBlas
from .opengl import OpenGlBlas
from .vulkan import VulkanBlas
from .webgpu import WebGpuBlas

__all__ = [
    "CpuBlas",
    "CudaBlas",
    "OpenClBlas",
    "MetalBlas",
    "VulkanBlas",
    "WebGpuBlas",
    "OpenGlBlas",
]
