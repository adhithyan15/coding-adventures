"""OpenGL Compute Simulator — the legacy global state machine.

=== What is OpenGL? ===

OpenGL is the oldest surviving GPU API (1992). Compute shaders were bolted
on in OpenGL 4.3 (2012), long after the core API was designed around
graphics rendering. This heritage shows: OpenGL uses a **global state
machine** model where you bind things to "current" state and then issue
commands that operate on whatever is currently bound.

=== The State Machine Model ===

Unlike Vulkan (explicit objects) or Metal (scoped encoders), OpenGL
maintains global state:

    glUseProgram(prog)           # Sets "current program" globally
    glBindBufferBase(0, buf_a)   # Sets "buffer at binding 0" globally
    glDispatchCompute(4, 1, 1)   # Uses WHATEVER is currently bound

This is simple for small programs but error-prone for large ones — you
must always remember what's bound. It's like cooking with ingredients
spread across your counter instead of measured in bowls.

=== Integer Handles ===

OpenGL uses integer handles (GLuint) for everything. You never get a
typed object — just a number:

    GLuint shader = glCreateShader(GL_COMPUTE_SHADER)     # Returns 1
    GLuint program = glCreateProgram()                    # Returns 1
    GLuint buffers[2]; glGenBuffers(2, buffers)          # Returns [1, 2]

These integers are essentially IDs in internal lookup tables. Our
simulator maintains dictionaries mapping these IDs to Layer 5 objects.

=== Memory Model ===

OpenGL buffers are created with glGenBuffers + glBufferData. The driver
picks the memory type based on usage hints (GL_STATIC_DRAW, GL_DYNAMIC_DRAW).
You access buffer contents via glMapBufferRange / glUnmapBuffer.
"""

from __future__ import annotations

import struct
from typing import Any

from compute_runtime import (
    BufferUsage,
    DescriptorBinding,
    MemoryType,
    Pipeline,
    ShaderModule,
)
from compute_runtime import Buffer as RuntimeBuffer

from ._base import BaseVendorSimulator


# =========================================================================
# OpenGL constants — module-level, just like real OpenGL
# =========================================================================

# In real OpenGL, these are C preprocessor defines. In Python, we use
# module-level constants with the same values.

# Shader types
GL_COMPUTE_SHADER = 0x91B9

# Buffer targets
GL_SHADER_STORAGE_BUFFER = 0x90D2
GL_ARRAY_BUFFER = 0x8892
GL_UNIFORM_BUFFER = 0x8A11

# Buffer usage hints
GL_STATIC_DRAW = 0x88E4   # Data set once, used many times
GL_DYNAMIC_DRAW = 0x88E8  # Data modified repeatedly, used many times
GL_STREAM_DRAW = 0x88E0   # Data set once, used a few times

# Map access bits
GL_MAP_READ_BIT = 0x0001
GL_MAP_WRITE_BIT = 0x0002

# Memory barrier bits
GL_SHADER_STORAGE_BARRIER_BIT = 0x00002000
GL_BUFFER_UPDATE_BARRIER_BIT = 0x00000200
GL_ALL_BARRIER_BITS = 0xFFFFFFFF

# Sync object results
GL_ALREADY_SIGNALED = 0x911A
GL_CONDITION_SATISFIED = 0x911C
GL_TIMEOUT_EXPIRED = 0x911B
GL_WAIT_FAILED = 0x911D

# Sync flags
GL_SYNC_FLUSH_COMMANDS_BIT = 0x00000001
GL_SYNC_GPU_COMMANDS_COMPLETE = 0x9117


# =========================================================================
# GLContext — the main OpenGL state machine
# =========================================================================


class GLContext(BaseVendorSimulator):
    """OpenGL context — a global state machine for GPU programming.

    === The State Machine ===

    GLContext maintains global state that commands operate on:

    - _current_program:  Which program is currently active (glUseProgram)
    - _bound_buffers:    Which buffers are bound to which targets/indices
    - _programs:         Map of GL handle → Layer 5 Pipeline
    - _shaders:          Map of GL handle → shader source + code
    - _buffers:          Map of GL handle → Layer 5 Buffer
    - _next_id:          Counter for generating unique GL handles

    Every OpenGL call reads and/or modifies this global state.

    === Usage ===

        gl = GLContext()

        # Create shader and program
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "saxpy")
        gl.compile_shader(shader)
        program = gl.create_program()
        gl.attach_shader(program, shader)
        gl.link_program(program)

        # Create and fill buffers
        bufs = gl.gen_buffers(2)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 256, data, GL_DYNAMIC_DRAW)

        # Bind and dispatch
        gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])
        gl.use_program(program)
        gl.dispatch_compute(4, 1, 1)
        gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)
    """

    def __init__(self) -> None:
        """Create a new OpenGL context."""
        super().__init__()

        # === Global State ===
        # These are the mutable state that OpenGL commands operate on.

        # Which program is currently active (set by glUseProgram)
        self._current_program: int | None = None

        # Buffer bindings: (target, index) → GL handle
        # target is e.g. GL_SHADER_STORAGE_BUFFER
        # index is the indexed binding point (0, 1, 2, ...)
        self._bound_buffers: dict[tuple[int, int], int] = {}

        # Current buffer bound to each target (for non-indexed operations)
        self._target_buffers: dict[int, int] = {}

        # === Internal lookup tables ===
        # GL uses integer handles. These dicts map handles to real objects.

        # handle → (source, code, compiled)
        self._shaders: dict[int, dict[str, Any]] = {}

        # handle → (pipeline, attached_shaders)
        self._programs: dict[int, dict[str, Any]] = {}

        # handle → Layer 5 Buffer
        self._buffers: dict[int, RuntimeBuffer] = {}

        # handle → Layer 5 Fence (for sync objects)
        self._syncs: dict[int, Any] = {}

        # Uniform locations: (program, name) → value
        self._uniforms: dict[tuple[int, str], Any] = {}

        # GL handle counter
        self._next_id: int = 1

    def _gen_id(self) -> int:
        """Generate a unique GL handle."""
        handle = self._next_id
        self._next_id += 1
        return handle

    # =================================================================
    # Shader management
    # =================================================================

    def create_shader(self, shader_type: int) -> int:
        """Create a shader object (glCreateShader).

        Args:
            shader_type: Must be GL_COMPUTE_SHADER.

        Returns:
            A GL handle (integer) for the shader.

        Raises:
            ValueError: If shader_type is not GL_COMPUTE_SHADER.
        """
        if shader_type != GL_COMPUTE_SHADER:
            raise ValueError(
                f"Only GL_COMPUTE_SHADER (0x{GL_COMPUTE_SHADER:04X}) is supported, "
                f"got 0x{shader_type:04X}"
            )
        handle = self._gen_id()
        self._shaders[handle] = {
            "source": "",
            "code": None,
            "compiled": False,
            "type": shader_type,
        }
        return handle

    def shader_source(self, shader: int, source: str) -> None:
        """Set the source code for a shader (glShaderSource).

        Args:
            shader: GL shader handle.
            source: Shader source code string.

        Raises:
            ValueError: If shader handle is invalid.
        """
        if shader not in self._shaders:
            raise ValueError(f"Invalid shader handle {shader}")
        self._shaders[shader]["source"] = source

    def compile_shader(self, shader: int) -> None:
        """Compile a shader (glCompileShader).

        In real OpenGL, this invokes the GLSL compiler. In our simulator,
        it just marks the shader as compiled. Actual GPU code is attached
        separately or provided at dispatch time.

        Args:
            shader: GL shader handle.

        Raises:
            ValueError: If shader handle is invalid.
        """
        if shader not in self._shaders:
            raise ValueError(f"Invalid shader handle {shader}")
        self._shaders[shader]["compiled"] = True

    def delete_shader(self, shader: int) -> None:
        """Delete a shader object (glDeleteShader).

        Args:
            shader: GL shader handle.
        """
        self._shaders.pop(shader, None)

    # =================================================================
    # Program management
    # =================================================================

    def create_program(self) -> int:
        """Create a program object (glCreateProgram).

        A program links one or more shaders into a usable pipeline.

        Returns:
            A GL handle for the program.
        """
        handle = self._gen_id()
        self._programs[handle] = {
            "pipeline": None,
            "shaders": [],
            "linked": False,
            "shader_module": None,
        }
        return handle

    def attach_shader(self, program: int, shader: int) -> None:
        """Attach a shader to a program (glAttachShader).

        Args:
            program: GL program handle.
            shader:  GL shader handle.

        Raises:
            ValueError: If either handle is invalid.
        """
        if program not in self._programs:
            raise ValueError(f"Invalid program handle {program}")
        if shader not in self._shaders:
            raise ValueError(f"Invalid shader handle {shader}")
        self._programs[program]["shaders"].append(shader)

    def link_program(self, program: int) -> None:
        """Link a program (glLinkProgram).

        Creates the Layer 5 Pipeline from attached shaders.

        Args:
            program: GL program handle.

        Raises:
            ValueError: If program handle is invalid.
            RuntimeError: If no shaders are attached.
        """
        if program not in self._programs:
            raise ValueError(f"Invalid program handle {program}")

        prog = self._programs[program]
        if not prog["shaders"]:
            raise RuntimeError(f"Program {program} has no attached shaders")

        # Get shader code from the first compute shader
        shader_handle = prog["shaders"][0]
        shader_info = self._shaders[shader_handle]
        code = shader_info.get("code")

        # Create Layer 5 pipeline
        shader = self._logical_device.create_shader_module(code=code)
        ds_layout = self._logical_device.create_descriptor_set_layout([])
        pl_layout = self._logical_device.create_pipeline_layout([ds_layout])
        pipeline = self._logical_device.create_compute_pipeline(shader, pl_layout)

        prog["pipeline"] = pipeline
        prog["shader_module"] = shader
        prog["linked"] = True

    def use_program(self, program: int) -> None:
        """Set the active program (glUseProgram).

        All subsequent dispatch_compute() calls will use this program.

        Args:
            program: GL program handle. 0 = unbind.

        Raises:
            ValueError: If program handle is invalid (and not 0).
        """
        if program == 0:
            self._current_program = None
            return
        if program not in self._programs:
            raise ValueError(f"Invalid program handle {program}")
        if not self._programs[program]["linked"]:
            raise RuntimeError(f"Program {program} is not linked")
        self._current_program = program

    def delete_program(self, program: int) -> None:
        """Delete a program object (glDeleteProgram).

        Args:
            program: GL program handle.
        """
        if self._current_program == program:
            self._current_program = None
        self._programs.pop(program, None)

    # =================================================================
    # Buffer management
    # =================================================================

    def gen_buffers(self, count: int) -> list[int]:
        """Generate buffer objects (glGenBuffers).

        Creates `count` buffer handles. The buffers are empty until
        buffer_data() is called.

        Args:
            count: Number of buffers to create.

        Returns:
            List of GL handles.
        """
        handles = []
        for _ in range(count):
            handle = self._gen_id()
            self._buffers[handle] = None  # type: ignore[assignment]
            handles.append(handle)
        return handles

    def delete_buffers(self, buffers: list[int]) -> None:
        """Delete buffer objects (glDeleteBuffers).

        Args:
            buffers: List of GL handles to delete.
        """
        for handle in buffers:
            if handle in self._buffers and self._buffers[handle] is not None:
                buf = self._buffers[handle]
                if not buf.freed:
                    self._memory_manager.free(buf)
            self._buffers.pop(handle, None)
            # Remove from any bindings
            keys_to_remove = [
                k for k, v in self._bound_buffers.items() if v == handle
            ]
            for k in keys_to_remove:
                del self._bound_buffers[k]
            keys_to_remove = [
                k for k, v in self._target_buffers.items() if v == handle
            ]
            for k in keys_to_remove:
                del self._target_buffers[k]

    def bind_buffer(self, target: int, buffer: int) -> None:
        """Bind a buffer to a target (glBindBuffer).

        Sets which buffer is "current" for the given target. Subsequent
        buffer operations on that target will affect this buffer.

        Args:
            target: Buffer target (e.g., GL_SHADER_STORAGE_BUFFER).
            buffer: GL buffer handle. 0 = unbind.
        """
        if buffer == 0:
            self._target_buffers.pop(target, None)
            return
        if buffer not in self._buffers:
            raise ValueError(f"Invalid buffer handle {buffer}")
        self._target_buffers[target] = buffer

    def buffer_data(
        self, target: int, size: int, data: bytes | None, usage: int
    ) -> None:
        """Allocate and optionally fill a buffer (glBufferData).

        This allocates GPU memory for the buffer currently bound to
        `target`. If `data` is provided, it's uploaded to the buffer.

        Args:
            target: Buffer target.
            size:   Buffer size in bytes.
            data:   Optional initial data.
            usage:  Usage hint (GL_STATIC_DRAW, GL_DYNAMIC_DRAW, etc.).

        Raises:
            RuntimeError: If no buffer is bound to the target.
        """
        if target not in self._target_buffers:
            raise RuntimeError(f"No buffer bound to target 0x{target:04X}")

        handle = self._target_buffers[target]

        # Free old allocation if exists
        if self._buffers[handle] is not None:
            old_buf = self._buffers[handle]
            if not old_buf.freed:
                self._memory_manager.free(old_buf)

        # Allocate new buffer via Layer 5
        mem_type = (
            MemoryType.DEVICE_LOCAL
            | MemoryType.HOST_VISIBLE
            | MemoryType.HOST_COHERENT
        )
        buf_usage = BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST
        buf = self._memory_manager.allocate(size, mem_type, usage=buf_usage)
        self._buffers[handle] = buf

        # Upload initial data if provided
        if data is not None:
            mapped = self._memory_manager.map(buf)
            mapped.write(0, bytes(data[:size]))
            self._memory_manager.unmap(buf)

    def buffer_sub_data(
        self, target: int, offset: int, data: bytes
    ) -> None:
        """Update a portion of a buffer (glBufferSubData).

        Args:
            target: Buffer target.
            offset: Byte offset into the buffer.
            data:   Data to write.

        Raises:
            RuntimeError: If no buffer is bound to the target.
        """
        if target not in self._target_buffers:
            raise RuntimeError(f"No buffer bound to target 0x{target:04X}")
        handle = self._target_buffers[target]
        buf = self._buffers[handle]
        if buf is None:
            raise RuntimeError(f"Buffer {handle} has no data store")

        mapped = self._memory_manager.map(buf)
        mapped.write(offset, data)
        self._memory_manager.unmap(buf)

    def bind_buffer_base(self, target: int, index: int, buffer: int) -> None:
        """Bind a buffer to an indexed binding point (glBindBufferBase).

        This is how you connect buffers to shader binding points:

            gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, buf_x)
            # Now binding 0 in the compute shader references buf_x

        Args:
            target: Buffer target (e.g., GL_SHADER_STORAGE_BUFFER).
            index:  Binding point index (0, 1, 2, ...).
            buffer: GL buffer handle.
        """
        if buffer not in self._buffers:
            raise ValueError(f"Invalid buffer handle {buffer}")
        self._bound_buffers[(target, index)] = buffer

    def map_buffer_range(
        self, target: int, offset: int, length: int, access: int
    ) -> bytearray:
        """Map a buffer region for CPU access (glMapBufferRange).

        Args:
            target: Buffer target.
            offset: Byte offset.
            length: Bytes to map.
            access: Access bits (GL_MAP_READ_BIT, GL_MAP_WRITE_BIT).

        Returns:
            A mutable bytearray of the buffer contents.

        Raises:
            RuntimeError: If no buffer is bound to the target.
        """
        if target not in self._target_buffers:
            raise RuntimeError(f"No buffer bound to target 0x{target:04X}")
        handle = self._target_buffers[target]
        buf = self._buffers[handle]
        if buf is None:
            raise RuntimeError(f"Buffer {handle} has no data store")

        # Invalidate to get latest device data
        self._memory_manager.invalidate(buf)
        data = self._memory_manager._get_buffer_data(buf.buffer_id)
        return bytearray(data[offset : offset + length])

    def unmap_buffer(self, target: int) -> bool:
        """Unmap a buffer (glUnmapBuffer).

        Args:
            target: Buffer target.

        Returns:
            True on success.
        """
        # In our simulator, the map_buffer_range returns a copy, so
        # unmapping is a no-op. In real OpenGL, this would flush writes.
        return True

    # =================================================================
    # Compute dispatch
    # =================================================================

    def dispatch_compute(
        self, num_groups_x: int, num_groups_y: int = 1, num_groups_z: int = 1
    ) -> None:
        """Dispatch compute work groups (glDispatchCompute).

        Uses whatever program and SSBO bindings are currently active.
        This is the OpenGL equivalent of CUDA's kernel<<<>>>().

        Internally: reads current state → builds pipeline + descriptor set
        → creates CB → records bind + dispatch → submits → waits.

        Args:
            num_groups_x: Workgroups in X.
            num_groups_y: Workgroups in Y.
            num_groups_z: Workgroups in Z.

        Raises:
            RuntimeError: If no program is active.
        """
        if self._current_program is None:
            raise RuntimeError("No program is currently active (call use_program first)")

        prog = self._programs[self._current_program]
        device = self._logical_device

        # Get shader code from the program
        shader_code = None
        if prog["shaders"]:
            shader_handle = prog["shaders"][0]
            if shader_handle in self._shaders:
                shader_code = self._shaders[shader_handle].get("code")

        # Find all SSBO bindings
        ssbo_bindings: dict[int, RuntimeBuffer] = {}
        for (target, index), handle in self._bound_buffers.items():
            if target == GL_SHADER_STORAGE_BUFFER and handle in self._buffers:
                buf = self._buffers[handle]
                if buf is not None:
                    ssbo_bindings[index] = buf

        # Create shader module
        shader = device.create_shader_module(code=shader_code)

        # Create descriptor set with SSBO bindings
        bindings = [
            DescriptorBinding(binding=i, type="storage")
            for i in sorted(ssbo_bindings.keys())
        ]
        ds_layout = device.create_descriptor_set_layout(bindings)
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        ds = device.create_descriptor_set(ds_layout)
        for i in sorted(ssbo_bindings.keys()):
            ds.write(i, ssbo_bindings[i])

        # Record and submit
        def record_dispatch(cb: Any) -> None:
            cb.cmd_bind_pipeline(pipeline)
            cb.cmd_bind_descriptor_set(ds)
            cb.cmd_dispatch(num_groups_x, num_groups_y, num_groups_z)

        self._create_and_submit_cb(record_dispatch)

    # =================================================================
    # Synchronization
    # =================================================================

    def memory_barrier(self, barriers: int) -> None:
        """Insert a memory barrier (glMemoryBarrier).

        Ensures that previous writes are visible to subsequent reads.
        In our synchronous simulator, this is largely a no-op, but we
        record it for correctness.

        Args:
            barriers: Barrier bits (GL_SHADER_STORAGE_BARRIER_BIT, etc.).
        """
        # In synchronous execution, barriers are automatically satisfied.
        # We could record a trace event here for debugging.
        pass

    def fence_sync(self) -> int:
        """Create a sync object (glFenceSync).

        Returns:
            A GL handle for the sync object.
        """
        handle = self._gen_id()
        fence = self._logical_device.create_fence(signaled=True)
        self._syncs[handle] = fence
        return handle

    def client_wait_sync(
        self, sync: int, flags: int, timeout: int
    ) -> int:
        """Wait for a sync object (glClientWaitSync).

        Args:
            sync:    GL sync handle.
            flags:   Wait flags (e.g., GL_SYNC_FLUSH_COMMANDS_BIT).
            timeout: Timeout in nanoseconds.

        Returns:
            GL_ALREADY_SIGNALED, GL_CONDITION_SATISFIED, GL_TIMEOUT_EXPIRED,
            or GL_WAIT_FAILED.
        """
        if sync not in self._syncs:
            return GL_WAIT_FAILED

        fence = self._syncs[sync]
        if fence.signaled:
            return GL_ALREADY_SIGNALED

        result = fence.wait(timeout_cycles=timeout)
        if result:
            return GL_CONDITION_SATISFIED
        return GL_TIMEOUT_EXPIRED

    def delete_sync(self, sync: int) -> None:
        """Delete a sync object (glDeleteSync).

        Args:
            sync: GL sync handle.
        """
        self._syncs.pop(sync, None)

    def finish(self) -> None:
        """Block until all GL commands complete (glFinish).

        This is the OpenGL equivalent of cudaDeviceSynchronize() or
        Vulkan's vkDeviceWaitIdle().
        """
        self._logical_device.wait_idle()

    # =================================================================
    # Uniforms (push constants)
    # =================================================================

    def get_uniform_location(self, program: int, name: str) -> int:
        """Get the location of a uniform variable (glGetUniformLocation).

        Args:
            program: GL program handle.
            name:    Uniform variable name.

        Returns:
            Location index (deterministic hash-based).
        """
        if program not in self._programs:
            raise ValueError(f"Invalid program handle {program}")
        # Return a deterministic location based on the name
        return hash(name) & 0x7FFFFFFF

    def uniform_1f(self, location: int, value: float) -> None:
        """Set a float uniform (glUniform1f).

        Args:
            location: Uniform location.
            value:    Float value to set.
        """
        if self._current_program is not None:
            self._uniforms[(self._current_program, str(location))] = value

    def uniform_1i(self, location: int, value: int) -> None:
        """Set an integer uniform (glUniform1i).

        Args:
            location: Uniform location.
            value:    Integer value to set.
        """
        if self._current_program is not None:
            self._uniforms[(self._current_program, str(location))] = value
