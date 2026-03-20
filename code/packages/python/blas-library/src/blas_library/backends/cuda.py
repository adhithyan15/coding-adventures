"""CudaBlas — NVIDIA CUDA BLAS backend.

=== How CudaBlas Works ===

This backend wraps the ``CUDARuntime`` from Layer 4 (vendor-api-simulators).
For each BLAS operation, it follows the classic CUDA pattern:

    1. cudaMalloc()           — allocate device memory for inputs and output
    2. cudaMemcpy(H2D)       — upload input data from host to device
    3. (compute)             — perform the operation
    4. cudaMemcpy(D2H)       — download results from device to host
    5. cudaFree()            — release device memory

Since our simulator's kernel execution is simplified, the actual arithmetic
is performed by the CPU reference (CpuBlas). The GPU memory pipeline is
fully exercised to demonstrate the CUDA programming pattern.

=== Real cuBLAS ===

In the real world, ``cublasSgemm()`` launches highly optimized CUDA kernels
that tile the computation across thousands of GPU threads, using shared
memory, warp-level primitives, and tensor cores. Our simulator demonstrates
the memory management pattern without that complexity.
"""

from __future__ import annotations

from vendor_api_simulators.cuda import CUDAMemcpyKind, CUDARuntime

from ._gpu_base import GpuBlasBase


class CudaBlas(GpuBlasBase):
    """CUDA BLAS backend — wraps CUDARuntime from Layer 4.

    ================================================================
    CUDA BLAS -- NVIDIA GPU ACCELERATION
    ================================================================

    The most widely used GPU BLAS backend in ML. Real cuBLAS achieves
    near-peak FLOPS on NVIDIA GPUs through:
    - Tiled GEMM with shared memory
    - Tensor Core acceleration (FP16/TF32)
    - Warp-level matrix multiply (WMMA)

    Our simulator demonstrates the memory management pattern:
    cudaMalloc -> cudaMemcpy(H2D) -> compute -> cudaMemcpy(D2H) -> cudaFree

    Usage:
        blas = CudaBlas()
        result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    ================================================================
    """

    def __init__(self) -> None:
        """Initialize the CUDA runtime and allocate resources."""
        super().__init__()
        self._cuda = CUDARuntime()

    @property
    def name(self) -> str:
        """Backend identifier."""
        return "cuda"

    @property
    def device_name(self) -> str:
        """Human-readable device name from CUDA properties."""
        props = self._cuda.get_device_properties()
        return props.name

    def _upload(self, data: bytes) -> object:
        """Allocate GPU memory and upload data via cudaMemcpy(H2D)."""
        ptr = self._cuda.malloc(len(data))
        self._cuda.memcpy(ptr, data, len(data), CUDAMemcpyKind.HostToDevice)
        return ptr

    def _download(self, handle: object, size: int) -> bytes:
        """Download data from GPU via cudaMemcpy(D2H)."""
        host_buf = bytearray(size)
        self._cuda.memcpy(host_buf, handle, size, CUDAMemcpyKind.DeviceToHost)  # type: ignore[arg-type]
        return bytes(host_buf)

    def _free(self, handle: object) -> None:
        """Free GPU memory via cudaFree()."""
        self._cuda.free(handle)  # type: ignore[arg-type]
