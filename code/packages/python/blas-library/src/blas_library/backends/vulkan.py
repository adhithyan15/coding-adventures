"""VulkanBlas — explicit Vulkan BLAS backend.

=== How VulkanBlas Works ===

This backend wraps the Vulkan API from Layer 4. Vulkan is the most verbose
GPU API — you explicitly manage everything: buffer creation, memory
allocation, binding, mapping, and unmapping.

For each BLAS operation, we allocate VkDeviceMemory, write data via the
underlying memory manager's map/write/unmap cycle, and read it back the
same way. Since the Vulkan simulator's ``vk_map_memory()`` returns a
snapshot bytearray (not a live reference), we use the underlying Layer 5
memory manager directly for writes.
"""

from __future__ import annotations

from vendor_api_simulators.vulkan import (
    VkInstance,
    VkMemoryAllocateInfo,
)

from ._gpu_base import GpuBlasBase


class VulkanBlas(GpuBlasBase):
    """Vulkan BLAS backend — wraps VkDevice from Layer 4.

    ================================================================
    VULKAN BLAS -- MAXIMUM CONTROL GPU ACCELERATION
    ================================================================

    Vulkan forces you to be explicit about everything:
    - Buffer creation with usage flags
    - Memory allocation with property flags
    - Explicit map/unmap for data transfer

    The reward is maximum performance and predictability — the driver
    does exactly what you say, nothing more.

    Usage:
        blas = VulkanBlas()
        result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    ================================================================
    """

    def __init__(self) -> None:
        """Initialize the Vulkan device via the full Vulkan setup ceremony."""
        super().__init__()
        self._vk_instance = VkInstance()
        physical_devices = self._vk_instance.vk_enumerate_physical_devices()
        self._vk_device = self._vk_instance.vk_create_device(physical_devices[0])

    @property
    def name(self) -> str:
        """Backend identifier."""
        return "vulkan"

    @property
    def device_name(self) -> str:
        """Human-readable device name."""
        return "Vulkan Device"

    def _upload(self, data: bytes) -> object:
        """Allocate Vulkan device memory and write data.

        We use VkDeviceMemory which wraps a Layer 5 Buffer. The write
        goes through the memory manager's map/write/unmap cycle to
        actually persist data to the device buffer.
        """
        alloc_info = VkMemoryAllocateInfo(
            size=len(data),
            memory_type_index=0,
        )
        memory = self._vk_device.vk_allocate_memory(alloc_info)

        # Write through the underlying memory manager (Layer 5)
        mm = memory._mm
        mapped = mm.map(memory._buffer)
        mapped.write(0, bytes(data))
        mm.unmap(memory._buffer)

        return memory

    def _download(self, handle: object, size: int) -> bytes:
        """Read data from Vulkan device memory."""
        memory = handle
        mm = memory._mm  # type: ignore[union-attr]
        mm.invalidate(memory._buffer)  # type: ignore[union-attr]
        mapped = mm.map(memory._buffer)  # type: ignore[union-attr]
        data = mapped.read(0, size)
        mm.unmap(memory._buffer)  # type: ignore[union-attr]
        return bytes(data)

    def _free(self, handle: object) -> None:
        """In our simulator, memory is freed by garbage collection."""
        pass
