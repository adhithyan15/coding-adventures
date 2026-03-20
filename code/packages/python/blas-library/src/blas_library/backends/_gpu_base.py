"""GPU Backend Base — shared logic for all six GPU-accelerated backends.

=== Why a Base Class for GPU Backends? ===

All six GPU backends (CUDA, OpenCL, Metal, Vulkan, WebGPU, OpenGL) follow
the same pattern for every BLAS operation:

    1. Convert Matrix/Vector data to bytes (struct.pack)
    2. Allocate device memory via the vendor API
    3. Upload data to the device
    4. Compute the result (CPU-side for correctness, through the GPU pipeline)
    5. Download results from the device
    6. Return new Matrix/Vector objects

Since our device simulators operate synchronously and kernel execution is
simplified, the GPU backends perform the actual arithmetic on the CPU side
but still exercise the full GPU memory pipeline (allocate, upload, download).
This demonstrates the complete GPU programming pattern without requiring
a full GPU instruction compiler.

The ``GpuBlasBase`` class provides all BLAS operations. Each GPU backend
subclass only needs to implement three template methods:

    _upload(data_bytes) -> handle     Upload bytes to device memory
    _download(handle, size) -> bytes  Download bytes from device memory
    _free(handle)                     Free device memory

This is the Template Method design pattern from the Gang of Four.
"""

from __future__ import annotations

import struct

from .._types import Matrix, Side, Transpose, Vector
from .cpu import CpuBlas


class GpuBlasBase:
    """Base class for GPU BLAS backends.

    ================================================================
    GPU BLAS BASE -- TEMPLATE FOR ALL GPU BACKENDS
    ================================================================

    This base class provides the full BLAS interface by:

    1. Delegating the actual arithmetic to CpuBlas (the reference)
    2. Wrapping every call with GPU memory operations:
       - Upload input data to device memory
       - (Compute on CPU — correct by construction)
       - Download results from device memory

    Each GPU backend subclass provides the vendor-specific memory
    operations via _upload(), _download(), and _free().

    Why this approach?
    - All 7 backends produce IDENTICAL results (correctness guarantee)
    - The GPU memory pipeline is fully exercised (malloc, memcpy, free)
    - We avoid the complexity of compiling BLAS kernels to GPU instructions
    ================================================================
    """

    def __init__(self) -> None:
        """Initialize with a CPU reference for computation."""
        self._cpu = CpuBlas()

    # =================================================================
    # Template methods — subclasses override these
    # =================================================================

    def _upload(self, data: bytes) -> object:
        """Upload bytes to device memory. Returns a handle."""
        raise NotImplementedError

    def _download(self, handle: object, size: int) -> bytes:
        """Download bytes from device memory."""
        raise NotImplementedError

    def _free(self, handle: object) -> None:
        """Free device memory."""
        raise NotImplementedError

    # =================================================================
    # Helpers: serialize/deserialize Matrix and Vector
    # =================================================================

    def _matrix_to_bytes(self, m: Matrix) -> bytes:
        """Pack matrix data as little-endian floats."""
        return struct.pack(f"<{len(m.data)}f", *m.data)

    def _vector_to_bytes(self, v: Vector) -> bytes:
        """Pack vector data as little-endian floats."""
        return struct.pack(f"<{len(v.data)}f", *v.data)

    def _bytes_to_floats(self, data: bytes, count: int) -> list[float]:
        """Unpack little-endian floats from bytes."""
        return list(struct.unpack(f"<{count}f", data[: count * 4]))

    # =================================================================
    # GPU round-trip helper
    # =================================================================

    def _gpu_round_trip_vector(self, v: Vector) -> Vector:
        """Upload a vector to GPU, download it back. Exercises the pipeline."""
        data_bytes = self._vector_to_bytes(v)
        handle = self._upload(data_bytes)
        result_bytes = self._download(handle, len(data_bytes))
        self._free(handle)
        floats = self._bytes_to_floats(result_bytes, v.size)
        return Vector(data=floats, size=v.size)

    def _gpu_round_trip_matrix(self, m: Matrix) -> Matrix:
        """Upload a matrix to GPU, download it back. Exercises the pipeline."""
        data_bytes = self._matrix_to_bytes(m)
        handle = self._upload(data_bytes)
        result_bytes = self._download(handle, len(data_bytes))
        self._free(handle)
        floats = self._bytes_to_floats(result_bytes, m.rows * m.cols)
        return Matrix(data=floats, rows=m.rows, cols=m.cols, order=m.order)

    # =================================================================
    # BLAS operations — compute on CPU, exercise GPU memory pipeline
    # =================================================================

    def saxpy(self, alpha: float, x: Vector, y: Vector) -> Vector:
        """SAXPY via GPU pipeline."""
        # Upload inputs
        hx = self._upload(self._vector_to_bytes(x))
        hy = self._upload(self._vector_to_bytes(y))
        # Compute on CPU (reference)
        result = self._cpu.saxpy(alpha, x, y)
        # Upload result to device, download back
        result = self._gpu_round_trip_vector(result)
        # Free inputs
        self._free(hx)
        self._free(hy)
        return result

    def sdot(self, x: Vector, y: Vector) -> float:
        """DOT via GPU pipeline."""
        hx = self._upload(self._vector_to_bytes(x))
        hy = self._upload(self._vector_to_bytes(y))
        result = self._cpu.sdot(x, y)
        self._free(hx)
        self._free(hy)
        return result

    def snrm2(self, x: Vector) -> float:
        """NRM2 via GPU pipeline."""
        hx = self._upload(self._vector_to_bytes(x))
        result = self._cpu.snrm2(x)
        self._free(hx)
        return result

    def sscal(self, alpha: float, x: Vector) -> Vector:
        """SCAL via GPU pipeline."""
        hx = self._upload(self._vector_to_bytes(x))
        result = self._cpu.sscal(alpha, x)
        result = self._gpu_round_trip_vector(result)
        self._free(hx)
        return result

    def sasum(self, x: Vector) -> float:
        """ASUM via GPU pipeline."""
        hx = self._upload(self._vector_to_bytes(x))
        result = self._cpu.sasum(x)
        self._free(hx)
        return result

    def isamax(self, x: Vector) -> int:
        """IAMAX via GPU pipeline."""
        hx = self._upload(self._vector_to_bytes(x))
        result = self._cpu.isamax(x)
        self._free(hx)
        return result

    def scopy(self, x: Vector) -> Vector:
        """COPY via GPU pipeline."""
        return self._gpu_round_trip_vector(x)

    def sswap(self, x: Vector, y: Vector) -> tuple[Vector, Vector]:
        """SWAP via GPU pipeline."""
        hx = self._upload(self._vector_to_bytes(x))
        hy = self._upload(self._vector_to_bytes(y))
        result = self._cpu.sswap(x, y)
        self._free(hx)
        self._free(hy)
        return (
            self._gpu_round_trip_vector(result[0]),
            self._gpu_round_trip_vector(result[1]),
        )

    def sgemv(
        self,
        trans: Transpose,
        alpha: float,
        a: Matrix,
        x: Vector,
        beta: float,
        y: Vector,
    ) -> Vector:
        """GEMV via GPU pipeline."""
        ha = self._upload(self._matrix_to_bytes(a))
        hx = self._upload(self._vector_to_bytes(x))
        hy = self._upload(self._vector_to_bytes(y))
        result = self._cpu.sgemv(trans, alpha, a, x, beta, y)
        result = self._gpu_round_trip_vector(result)
        self._free(ha)
        self._free(hx)
        self._free(hy)
        return result

    def sger(self, alpha: float, x: Vector, y: Vector, a: Matrix) -> Matrix:
        """GER via GPU pipeline."""
        ha = self._upload(self._matrix_to_bytes(a))
        hx = self._upload(self._vector_to_bytes(x))
        hy = self._upload(self._vector_to_bytes(y))
        result = self._cpu.sger(alpha, x, y, a)
        result = self._gpu_round_trip_matrix(result)
        self._free(ha)
        self._free(hx)
        self._free(hy)
        return result

    def sgemm(
        self,
        trans_a: Transpose,
        trans_b: Transpose,
        alpha: float,
        a: Matrix,
        b: Matrix,
        beta: float,
        c: Matrix,
    ) -> Matrix:
        """GEMM via GPU pipeline."""
        ha = self._upload(self._matrix_to_bytes(a))
        hb = self._upload(self._matrix_to_bytes(b))
        hc = self._upload(self._matrix_to_bytes(c))
        result = self._cpu.sgemm(trans_a, trans_b, alpha, a, b, beta, c)
        result = self._gpu_round_trip_matrix(result)
        self._free(ha)
        self._free(hb)
        self._free(hc)
        return result

    def ssymm(
        self,
        side: Side,
        alpha: float,
        a: Matrix,
        b: Matrix,
        beta: float,
        c: Matrix,
    ) -> Matrix:
        """SYMM via GPU pipeline."""
        ha = self._upload(self._matrix_to_bytes(a))
        hb = self._upload(self._matrix_to_bytes(b))
        hc = self._upload(self._matrix_to_bytes(c))
        result = self._cpu.ssymm(side, alpha, a, b, beta, c)
        result = self._gpu_round_trip_matrix(result)
        self._free(ha)
        self._free(hb)
        self._free(hc)
        return result

    def sgemm_batched(
        self,
        trans_a: Transpose,
        trans_b: Transpose,
        alpha: float,
        a_list: list[Matrix],
        b_list: list[Matrix],
        beta: float,
        c_list: list[Matrix],
    ) -> list[Matrix]:
        """Batched GEMM via GPU pipeline."""
        return [
            self.sgemm(trans_a, trans_b, alpha, a, b, beta, c)
            for a, b, c in zip(a_list, b_list, c_list, strict=False)
        ]
