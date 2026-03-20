"""MetalBlas — Apple Metal BLAS backend.

=== How MetalBlas Works ===

This backend wraps ``MTLDevice`` from Layer 4. Metal's key advantage is
**unified memory** — on Apple Silicon, CPU and GPU share the same RAM.
This means no host-to-device copies:

    CUDA:   cudaMalloc -> cudaMemcpy(H2D) -> compute -> cudaMemcpy(D2H) -> cudaFree
    Metal:  makeBuffer -> write_bytes       -> compute -> contents()

The buffer is always accessible from both CPU and GPU, so writes are
immediate and reads require no copy.

=== Real Accelerate/MPS ===

On real Apple hardware, Metal Performance Shaders (MPS) provides optimized
BLAS operations that leverage the Apple GPU's unified memory architecture.
PyTorch MPS backend uses this.
"""

from __future__ import annotations

from vendor_api_simulators.metal import MTLDevice

from ._gpu_base import GpuBlasBase


class MetalBlas(GpuBlasBase):
    """Metal BLAS backend — wraps MTLDevice from Layer 4.

    ================================================================
    METAL BLAS -- APPLE SILICON UNIFIED MEMORY
    ================================================================

    Metal's unified memory model eliminates host-device copies:
    - make_buffer() allocates memory visible to both CPU and GPU
    - write_bytes() writes directly (no staging buffer needed)
    - contents() reads directly (no download needed)

    This is the biggest ergonomic advantage of Apple Silicon for GPU
    computing.

    Usage:
        blas = MetalBlas()
        result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    ================================================================
    """

    def __init__(self) -> None:
        """Initialize the Metal device."""
        super().__init__()
        self._device = MTLDevice()

    @property
    def name(self) -> str:
        """Backend identifier."""
        return "metal"

    @property
    def device_name(self) -> str:
        """Human-readable device name."""
        return self._device.name

    def _upload(self, data: bytes) -> object:
        """Create a Metal buffer with unified memory and write data."""
        buf = self._device.make_buffer(len(data))
        buf.write_bytes(data)
        return buf

    def _download(self, handle: object, size: int) -> bytes:
        """Read directly from the Metal buffer (unified memory)."""
        contents = handle.contents()  # type: ignore[union-attr]
        return bytes(contents[:size])

    def _free(self, handle: object) -> None:
        """Metal buffers are freed when they go out of scope.

        In our simulator, Metal uses ARC-style memory management.
        The buffer will be deallocated when no references remain.
        """
        pass  # Metal uses automatic reference counting
