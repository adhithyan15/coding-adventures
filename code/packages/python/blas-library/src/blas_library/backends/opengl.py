"""OpenGlBlas — legacy OpenGL compute BLAS backend.

=== How OpenGlBlas Works ===

This backend wraps ``GLContext`` from Layer 4. OpenGL uses a global state
machine model — you bind things to "current" state and then issue commands
that operate on whatever is currently bound.

For each BLAS operation:
    1. gl.gen_buffers()    — generate buffer IDs
    2. gl.buffer_data()    — allocate and upload data
    3. (compute)           — perform operation
    4. gl.map_buffer_range() — map buffer for reading
    5. gl.delete_buffers() — free buffers

OpenGL compute shaders (4.3+) use Shader Storage Buffer Objects (SSBOs)
for GPU-accessible storage.
"""

from __future__ import annotations

from vendor_api_simulators.opengl import (
    GL_MAP_READ_BIT,
    GL_SHADER_STORAGE_BUFFER,
    GL_STATIC_DRAW,
    GLContext,
)

from ._gpu_base import GpuBlasBase


class OpenGlBlas(GpuBlasBase):
    """OpenGL BLAS backend — wraps GLContext from Layer 4.

    ================================================================
    OPENGL BLAS -- LEGACY STATE MACHINE GPU ACCELERATION
    ================================================================

    OpenGL is the oldest surviving GPU API (1992). Compute shaders
    were added in OpenGL 4.3 (2012), bolted onto the existing state
    machine model.

    The state machine means:
    - glBindBuffer(target, id)  sets "current buffer" globally
    - glBufferData(target, ...) operates on WHATEVER is currently bound
    - You must remember what's bound at all times

    Simple for small programs, error-prone for large ones.

    Usage:
        blas = OpenGlBlas()
        result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C)
    ================================================================
    """

    def __init__(self) -> None:
        """Initialize the OpenGL context."""
        super().__init__()
        self._gl = GLContext()

    @property
    def name(self) -> str:
        """Backend identifier."""
        return "opengl"

    @property
    def device_name(self) -> str:
        """Human-readable device name."""
        return "OpenGL Device"

    def _upload(self, data: bytes) -> object:
        """Create an OpenGL SSBO and upload data."""
        buf_id = self._gl.gen_buffers(1)[0]
        self._gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, buf_id)
        self._gl.buffer_data(GL_SHADER_STORAGE_BUFFER, len(data), data, GL_STATIC_DRAW)
        return buf_id

    def _download(self, handle: object, size: int) -> bytes:
        """Map the OpenGL buffer for reading and copy data out."""
        self._gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, handle)  # type: ignore[arg-type]
        mapped = self._gl.map_buffer_range(
            GL_SHADER_STORAGE_BUFFER, 0, size, GL_MAP_READ_BIT
        )
        data = bytes(mapped[:size])
        self._gl.unmap_buffer(GL_SHADER_STORAGE_BUFFER)
        return data

    def _free(self, handle: object) -> None:
        """Delete the OpenGL buffer."""
        self._gl.delete_buffers([handle])  # type: ignore[list-item]
