"""WebGpuBlas — browser-friendly WebGPU BLAS backend.

=== How WebGpuBlas Works ===

This backend wraps ``GPUDevice`` from Layer 4. WebGPU is designed for
safe, browser-based GPU compute with automatic synchronization.

For each BLAS operation:
    1. device.create_buffer(STORAGE | COPY_DST)  — allocate with usage flags
    2. device.queue.write_buffer()               — upload data
    3. (compute)                                  — perform operation
    4. Create a MAP_READ staging buffer, copy, map, read
    5. Buffer goes out of scope (auto-freed)

WebGPU's key simplification: a single queue (``device.queue``) handles
everything. No queue families, no multiple queues.
"""

from __future__ import annotations

from vendor_api_simulators.webgpu import (
    GPU,
    GPUBufferDescriptor,
    GPUBufferUsage,
)

from ._gpu_base import GpuBlasBase


class WebGpuBlas(GpuBlasBase):
    """WebGPU BLAS backend — wraps GPUDevice from Layer 4.

    ================================================================
    WEBGPU BLAS -- SAFE BROWSER-FIRST GPU ACCELERATION
    ================================================================

    WebGPU provides a safe, validated GPU API designed for browsers:
    - Single queue (device.queue)
    - Automatic barriers (no manual synchronization)
    - Usage-based buffer creation (STORAGE, COPY_SRC, COPY_DST, MAP_READ)

    Usage:
        blas = WebGpuBlas()
        result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    ================================================================
    """

    def __init__(self) -> None:
        """Initialize WebGPU adapter and device."""
        super().__init__()
        gpu = GPU()
        adapter = gpu.request_adapter()
        self._device = adapter.request_device()

    @property
    def name(self) -> str:
        """Backend identifier."""
        return "webgpu"

    @property
    def device_name(self) -> str:
        """Human-readable device name."""
        return "WebGPU Device"

    def _upload(self, data: bytes) -> object:
        """Create a WebGPU buffer with STORAGE usage and write data."""
        desc = GPUBufferDescriptor(
            size=len(data),
            usage=GPUBufferUsage.STORAGE
            | GPUBufferUsage.COPY_DST
            | GPUBufferUsage.COPY_SRC,
        )
        buf = self._device.create_buffer(desc)
        self._device.queue.write_buffer(buf, 0, data)
        return buf

    def _download(self, handle: object, size: int) -> bytes:
        """Create a MAP_READ staging buffer, copy, and read."""
        # Create a staging buffer for readback
        staging_desc = GPUBufferDescriptor(
            size=size,
            usage=GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
        )
        staging = self._device.create_buffer(staging_desc)

        # Copy from source to staging
        encoder = self._device.create_command_encoder()
        encoder.copy_buffer_to_buffer(handle, 0, staging, 0, size)  # type: ignore[arg-type]
        cmd_buf = encoder.finish()
        self._device.queue.submit([cmd_buf])

        # Map and read
        staging.map_async("read")
        data = staging.get_mapped_range(0, size)
        staging.unmap()
        return bytes(data)

    def _free(self, handle: object) -> None:
        """WebGPU buffers are freed via destroy() or garbage collection."""
        handle.destroy()  # type: ignore[union-attr]
