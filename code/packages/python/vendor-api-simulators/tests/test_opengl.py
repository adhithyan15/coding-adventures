"""Tests for the OpenGL compute simulator."""

import pytest
from gpu_core import limm, halt

from vendor_api_simulators.opengl import (
    GLContext,
    GL_COMPUTE_SHADER,
    GL_SHADER_STORAGE_BUFFER,
    GL_ARRAY_BUFFER,
    GL_STATIC_DRAW,
    GL_DYNAMIC_DRAW,
    GL_MAP_READ_BIT,
    GL_MAP_WRITE_BIT,
    GL_SHADER_STORAGE_BARRIER_BIT,
    GL_BUFFER_UPDATE_BARRIER_BIT,
    GL_ALL_BARRIER_BITS,
    GL_ALREADY_SIGNALED,
    GL_CONDITION_SATISFIED,
    GL_TIMEOUT_EXPIRED,
    GL_WAIT_FAILED,
    GL_SYNC_FLUSH_COMMANDS_BIT,
)


class TestGLContextCreation:
    """Test context initialization."""

    def test_create_context(self):
        """GLContext initializes successfully."""
        gl = GLContext()
        assert gl._logical_device is not None

    def test_initial_state(self):
        """Initial state has no program, no bindings."""
        gl = GLContext()
        assert gl._current_program is None
        assert len(gl._bound_buffers) == 0
        assert len(gl._target_buffers) == 0


class TestGLShaderManagement:
    """Test shader creation, compilation, and deletion."""

    def test_create_shader(self):
        """create_shader returns a valid handle."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        assert isinstance(shader, int)
        assert shader > 0

    def test_create_shader_invalid_type(self):
        """create_shader with invalid type raises ValueError."""
        gl = GLContext()
        with pytest.raises(ValueError, match="GL_COMPUTE_SHADER"):
            gl.create_shader(0x8B31)  # GL_VERTEX_SHADER

    def test_shader_source(self):
        """shader_source stores source string."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "my_kernel")
        assert gl._shaders[shader]["source"] == "my_kernel"

    def test_shader_source_invalid_handle(self):
        """shader_source with invalid handle raises ValueError."""
        gl = GLContext()
        with pytest.raises(ValueError, match="Invalid shader"):
            gl.shader_source(999, "test")

    def test_compile_shader(self):
        """compile_shader marks shader as compiled."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        assert gl._shaders[shader]["compiled"]

    def test_compile_invalid_shader(self):
        """compile_shader with invalid handle raises ValueError."""
        gl = GLContext()
        with pytest.raises(ValueError, match="Invalid shader"):
            gl.compile_shader(999)

    def test_delete_shader(self):
        """delete_shader removes the shader."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.delete_shader(shader)
        assert shader not in gl._shaders

    def test_delete_nonexistent_shader(self):
        """delete_shader with invalid handle is a no-op."""
        gl = GLContext()
        gl.delete_shader(999)  # Should not raise


class TestGLProgramManagement:
    """Test program creation, linking, and usage."""

    def test_create_program(self):
        """create_program returns a valid handle."""
        gl = GLContext()
        prog = gl.create_program()
        assert isinstance(prog, int)
        assert prog > 0

    def test_attach_shader(self):
        """attach_shader adds shader to program."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        assert shader in gl._programs[prog]["shaders"]

    def test_attach_shader_invalid_program(self):
        """attach_shader with invalid program raises ValueError."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        with pytest.raises(ValueError, match="Invalid program"):
            gl.attach_shader(999, shader)

    def test_attach_shader_invalid_shader(self):
        """attach_shader with invalid shader raises ValueError."""
        gl = GLContext()
        prog = gl.create_program()
        with pytest.raises(ValueError, match="Invalid shader"):
            gl.attach_shader(prog, 999)

    def test_link_program(self):
        """link_program creates pipeline."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)
        assert gl._programs[prog]["linked"]
        assert gl._programs[prog]["pipeline"] is not None

    def test_link_program_invalid(self):
        """link_program with invalid handle raises ValueError."""
        gl = GLContext()
        with pytest.raises(ValueError, match="Invalid program"):
            gl.link_program(999)

    def test_link_program_no_shaders(self):
        """link_program with no shaders raises RuntimeError."""
        gl = GLContext()
        prog = gl.create_program()
        with pytest.raises(RuntimeError, match="no attached shaders"):
            gl.link_program(prog)

    def test_use_program(self):
        """use_program sets current program."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)
        gl.use_program(prog)
        assert gl._current_program == prog

    def test_use_program_zero_unbinds(self):
        """use_program(0) unbinds current program."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)
        gl.use_program(prog)
        gl.use_program(0)
        assert gl._current_program is None

    def test_use_program_invalid(self):
        """use_program with invalid handle raises ValueError."""
        gl = GLContext()
        with pytest.raises(ValueError, match="Invalid program"):
            gl.use_program(999)

    def test_use_program_unlinked(self):
        """use_program with unlinked program raises RuntimeError."""
        gl = GLContext()
        prog = gl.create_program()
        with pytest.raises(RuntimeError, match="not linked"):
            gl.use_program(prog)

    def test_delete_program(self):
        """delete_program removes the program."""
        gl = GLContext()
        prog = gl.create_program()
        gl.delete_program(prog)
        assert prog not in gl._programs

    def test_delete_current_program(self):
        """delete_program unbinds if it's the current program."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)
        gl.use_program(prog)
        gl.delete_program(prog)
        assert gl._current_program is None


class TestGLBufferManagement:
    """Test buffer creation, data, and binding."""

    def test_gen_buffers(self):
        """gen_buffers returns unique handles."""
        gl = GLContext()
        bufs = gl.gen_buffers(3)
        assert len(bufs) == 3
        assert len(set(bufs)) == 3  # All unique

    def test_bind_buffer(self):
        """bind_buffer sets current buffer for target."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        assert gl._target_buffers[GL_SHADER_STORAGE_BUFFER] == bufs[0]

    def test_bind_buffer_zero_unbinds(self):
        """bind_buffer(target, 0) unbinds."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, 0)
        assert GL_SHADER_STORAGE_BUFFER not in gl._target_buffers

    def test_bind_buffer_invalid(self):
        """bind_buffer with invalid handle raises ValueError."""
        gl = GLContext()
        with pytest.raises(ValueError, match="Invalid buffer"):
            gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, 999)

    def test_buffer_data(self):
        """buffer_data allocates and fills a buffer."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, b"\xAA" * 16, GL_DYNAMIC_DRAW)
        assert gl._buffers[bufs[0]] is not None

    def test_buffer_data_no_bind(self):
        """buffer_data without bound buffer raises RuntimeError."""
        gl = GLContext()
        with pytest.raises(RuntimeError, match="No buffer bound"):
            gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, None, GL_STATIC_DRAW)

    def test_buffer_data_null_data(self):
        """buffer_data with None data allocates without filling."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 32, None, GL_STATIC_DRAW)
        assert gl._buffers[bufs[0]] is not None

    def test_buffer_sub_data(self):
        """buffer_sub_data updates a portion of the buffer."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, b"\x00" * 16, GL_DYNAMIC_DRAW)
        gl.buffer_sub_data(GL_SHADER_STORAGE_BUFFER, 4, b"\xFF\xFF")

    def test_buffer_sub_data_no_bind(self):
        """buffer_sub_data without bound buffer raises."""
        gl = GLContext()
        with pytest.raises(RuntimeError, match="No buffer bound"):
            gl.buffer_sub_data(GL_SHADER_STORAGE_BUFFER, 0, b"\x00")

    def test_buffer_sub_data_no_data_store(self):
        """buffer_sub_data on uninitialized buffer raises."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        with pytest.raises(RuntimeError, match="no data store"):
            gl.buffer_sub_data(GL_SHADER_STORAGE_BUFFER, 0, b"\x00")

    def test_bind_buffer_base(self):
        """bind_buffer_base binds to indexed binding point."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, None, GL_DYNAMIC_DRAW)
        gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])
        assert gl._bound_buffers[(GL_SHADER_STORAGE_BUFFER, 0)] == bufs[0]

    def test_bind_buffer_base_invalid(self):
        """bind_buffer_base with invalid handle raises."""
        gl = GLContext()
        with pytest.raises(ValueError, match="Invalid buffer"):
            gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, 999)

    def test_delete_buffers(self):
        """delete_buffers removes buffers."""
        gl = GLContext()
        bufs = gl.gen_buffers(2)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 32, None, GL_STATIC_DRAW)
        gl.delete_buffers(bufs)
        assert bufs[0] not in gl._buffers

    def test_map_buffer_range(self):
        """map_buffer_range returns buffer data."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, b"\xAA" * 16, GL_DYNAMIC_DRAW)
        data = gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, 16, GL_MAP_READ_BIT)
        assert isinstance(data, bytearray)
        assert len(data) == 16

    def test_map_buffer_range_no_bind(self):
        """map_buffer_range without bound buffer raises."""
        gl = GLContext()
        with pytest.raises(RuntimeError, match="No buffer bound"):
            gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, 16, GL_MAP_READ_BIT)

    def test_unmap_buffer(self):
        """unmap_buffer returns True."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 16, None, GL_STATIC_DRAW)
        result = gl.unmap_buffer(GL_SHADER_STORAGE_BUFFER)
        assert result is True

    def test_map_buffer_no_data_store(self):
        """map_buffer_range on buffer without data store raises."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        with pytest.raises(RuntimeError, match="no data store"):
            gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, 16, GL_MAP_READ_BIT)

    def test_buffer_data_replace(self):
        """buffer_data on an already-allocated buffer replaces it."""
        gl = GLContext()
        bufs = gl.gen_buffers(1)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 32, None, GL_STATIC_DRAW)
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, None, GL_DYNAMIC_DRAW)
        assert gl._buffers[bufs[0]].size == 64


class TestGLDispatch:
    """Test compute dispatch."""

    def _setup_program(self, gl, instructions=None):
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        if instructions:
            gl._shaders[shader]["code"] = instructions
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)
        gl.use_program(prog)
        return prog

    def test_dispatch_compute(self, simple_instructions):
        """dispatch_compute dispatches work."""
        gl = GLContext()
        self._setup_program(gl, simple_instructions)
        gl.dispatch_compute(1, 1, 1)

    def test_dispatch_without_program(self):
        """dispatch_compute without active program raises."""
        gl = GLContext()
        with pytest.raises(RuntimeError, match="No program"):
            gl.dispatch_compute(1, 1, 1)

    def test_dispatch_with_ssbo(self, simple_instructions):
        """dispatch_compute with bound SSBOs."""
        gl = GLContext()
        self._setup_program(gl, simple_instructions)

        bufs = gl.gen_buffers(2)
        for i, buf in enumerate(bufs):
            gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, buf)
            gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, b"\x00" * 64, GL_DYNAMIC_DRAW)
            gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, i, buf)

        gl.dispatch_compute(1, 1, 1)

    def test_dispatch_multi_workgroup(self, simple_instructions):
        """dispatch_compute with multiple workgroups."""
        gl = GLContext()
        self._setup_program(gl, simple_instructions)
        gl.dispatch_compute(4, 2, 1)


class TestGLSynchronization:
    """Test synchronization primitives."""

    def test_memory_barrier(self):
        """memory_barrier completes without error."""
        gl = GLContext()
        gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)
        gl.memory_barrier(GL_BUFFER_UPDATE_BARRIER_BIT)
        gl.memory_barrier(GL_ALL_BARRIER_BITS)

    def test_fence_sync(self):
        """fence_sync returns a valid handle."""
        gl = GLContext()
        sync = gl.fence_sync()
        assert isinstance(sync, int)
        assert sync > 0

    def test_client_wait_sync(self):
        """client_wait_sync on signaled fence returns ALREADY_SIGNALED."""
        gl = GLContext()
        sync = gl.fence_sync()
        result = gl.client_wait_sync(sync, GL_SYNC_FLUSH_COMMANDS_BIT, 1000)
        assert result == GL_ALREADY_SIGNALED

    def test_client_wait_sync_invalid(self):
        """client_wait_sync with invalid handle returns WAIT_FAILED."""
        gl = GLContext()
        result = gl.client_wait_sync(999, 0, 1000)
        assert result == GL_WAIT_FAILED

    def test_delete_sync(self):
        """delete_sync removes the sync object."""
        gl = GLContext()
        sync = gl.fence_sync()
        gl.delete_sync(sync)
        assert sync not in gl._syncs

    def test_finish(self):
        """finish() completes without error."""
        gl = GLContext()
        gl.finish()


class TestGLUniforms:
    """Test uniform (push constant) management."""

    def test_get_uniform_location(self):
        """get_uniform_location returns an integer location."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)
        loc = gl.get_uniform_location(prog, "alpha")
        assert isinstance(loc, int)

    def test_get_uniform_location_invalid_program(self):
        """get_uniform_location with invalid program raises."""
        gl = GLContext()
        with pytest.raises(ValueError, match="Invalid program"):
            gl.get_uniform_location(999, "alpha")

    def test_uniform_1f(self):
        """uniform_1f sets a float uniform."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)
        gl.use_program(prog)
        loc = gl.get_uniform_location(prog, "alpha")
        gl.uniform_1f(loc, 2.5)

    def test_uniform_1i(self):
        """uniform_1i sets an integer uniform."""
        gl = GLContext()
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)
        gl.use_program(prog)
        loc = gl.get_uniform_location(prog, "count")
        gl.uniform_1i(loc, 42)

    def test_uniform_without_program(self):
        """uniform_1f without active program is safe (no-op)."""
        gl = GLContext()
        gl.uniform_1f(0, 1.0)  # Should not raise


class TestGLFullPipeline:
    """End-to-end OpenGL workflow tests."""

    def test_full_compute_workflow(self, simple_instructions):
        """Full pipeline: shader → program → buffers → dispatch."""
        gl = GLContext()

        # Create shader and program
        shader = gl.create_shader(GL_COMPUTE_SHADER)
        gl.shader_source(shader, "test")
        gl.compile_shader(shader)
        gl._shaders[shader]["code"] = simple_instructions
        prog = gl.create_program()
        gl.attach_shader(prog, shader)
        gl.link_program(prog)

        # Create and fill buffers
        bufs = gl.gen_buffers(2)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[0])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, b"\x00" * 64, GL_DYNAMIC_DRAW)
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[1])
        gl.buffer_data(GL_SHADER_STORAGE_BUFFER, 64, b"\x00" * 64, GL_DYNAMIC_DRAW)

        # Bind SSBOs
        gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 0, bufs[0])
        gl.bind_buffer_base(GL_SHADER_STORAGE_BUFFER, 1, bufs[1])

        # Dispatch
        gl.use_program(prog)
        gl.dispatch_compute(1, 1, 1)
        gl.memory_barrier(GL_SHADER_STORAGE_BARRIER_BIT)

        # Read back
        gl.bind_buffer(GL_SHADER_STORAGE_BUFFER, bufs[1])
        data = gl.map_buffer_range(GL_SHADER_STORAGE_BUFFER, 0, 64, GL_MAP_READ_BIT)
        gl.unmap_buffer(GL_SHADER_STORAGE_BUFFER)

        # Cleanup
        gl.finish()
        gl.delete_buffers(bufs)
        gl.delete_program(prog)
        gl.delete_shader(shader)
