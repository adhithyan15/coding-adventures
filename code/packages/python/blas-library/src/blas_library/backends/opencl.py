"""OpenClBlas — portable OpenCL BLAS backend.

=== How OpenClBlas Works ===

This backend wraps ``CLContext`` and ``CLCommandQueue`` from Layer 4.
OpenCL's distinctive feature is event-based dependencies — every enqueue
operation returns a CLEvent that subsequent operations can wait on.

For each BLAS operation:
    1. ctx.create_buffer()            — allocate device memory
    2. queue.enqueue_write_buffer()   — upload data (returns event)
    3. (compute)                      — perform the operation
    4. queue.enqueue_read_buffer()    — download results (waits on compute)
    5. queue.finish()                 — wait for all operations

OpenCL is the most portable GPU API — it runs on NVIDIA, AMD, Intel GPUs,
and even CPUs and FPGAs.
"""

from __future__ import annotations

from vendor_api_simulators.opencl import CLContext, CLMemFlags

from ._gpu_base import GpuBlasBase


class OpenClBlas(GpuBlasBase):
    """OpenCL BLAS backend — wraps CLContext from Layer 4.

    ================================================================
    OPENCL BLAS -- PORTABLE GPU ACCELERATION
    ================================================================

    OpenCL (Open Computing Language) is the Khronos Group's cross-platform
    compute API. Unlike CUDA (NVIDIA only), OpenCL runs on any vendor's
    GPU and even on CPUs.

    Our simulator exercises the OpenCL memory pipeline:
    create_buffer -> enqueue_write -> compute -> enqueue_read -> finish

    Usage:
        blas = OpenClBlas()
        result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    ================================================================
    """

    def __init__(self) -> None:
        """Initialize OpenCL context and command queue."""
        super().__init__()
        self._ctx = CLContext()
        self._queue = self._ctx.create_command_queue()

    @property
    def name(self) -> str:
        """Backend identifier."""
        return "opencl"

    @property
    def device_name(self) -> str:
        """Human-readable device name."""
        return self._ctx._devices[0].name

    def _upload(self, data: bytes) -> object:
        """Create a CLBuffer and upload data via enqueue_write_buffer."""
        buf = self._ctx.create_buffer(CLMemFlags.READ_WRITE, len(data))
        self._queue.enqueue_write_buffer(buf, 0, len(data), data)
        return buf

    def _download(self, handle: object, size: int) -> bytes:
        """Download data via enqueue_read_buffer."""
        host_buf = bytearray(size)
        self._queue.enqueue_read_buffer(handle, 0, size, host_buf)  # type: ignore[arg-type]
        self._queue.finish()
        return bytes(host_buf)

    def _free(self, handle: object) -> None:
        """OpenCL buffers are freed when the context is destroyed.

        In our simulator, there's no explicit free for CLBuffer.
        The buffer will be garbage collected with the context.
        """
        pass  # CLBuffer doesn't have an explicit free
