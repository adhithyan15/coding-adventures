"""Tests for the CUDA runtime simulator."""

import pytest
from gpu_core import limm, halt

from vendor_api_simulators.cuda import (
    CUDARuntime,
    CUDAKernel,
    CUDADevicePtr,
    CUDAStream,
    CUDAEvent,
    CUDAMemcpyKind,
    CUDADeviceProperties,
    dim3,
)


class TestCUDADeviceManagement:
    """Test device discovery and management."""

    def test_create_runtime(self):
        """CUDARuntime initializes and selects an NVIDIA device."""
        cuda = CUDARuntime()
        assert cuda._physical_device is not None
        assert cuda._logical_device is not None

    def test_get_device(self):
        """get_device() returns the current device ID."""
        cuda = CUDARuntime()
        assert cuda.get_device() == 0

    def test_set_device_valid(self):
        """set_device() with a valid ID succeeds."""
        cuda = CUDARuntime()
        cuda.set_device(0)
        assert cuda.get_device() == 0

    def test_set_device_invalid(self):
        """set_device() with invalid ID raises ValueError."""
        cuda = CUDARuntime()
        with pytest.raises(ValueError, match="Invalid device ID"):
            cuda.set_device(999)

    def test_set_device_negative(self):
        """set_device() with negative ID raises ValueError."""
        cuda = CUDARuntime()
        with pytest.raises(ValueError):
            cuda.set_device(-1)

    def test_get_device_properties(self):
        """get_device_properties() returns a CUDADeviceProperties."""
        cuda = CUDARuntime()
        props = cuda.get_device_properties()
        assert isinstance(props, CUDADeviceProperties)
        assert len(props.name) > 0
        assert props.total_global_mem > 0
        assert props.max_threads_per_block > 0
        assert props.warp_size == 32

    def test_device_synchronize(self):
        """device_synchronize() completes without error."""
        cuda = CUDARuntime()
        cuda.device_synchronize()  # Should not raise

    def test_device_reset(self):
        """device_reset() clears streams and events."""
        cuda = CUDARuntime()
        stream = cuda.create_stream()
        event = cuda.create_event()
        cuda.device_reset()
        assert len(cuda._streams) == 0
        assert len(cuda._events) == 0


class TestCUDAMemory:
    """Test memory allocation and transfers."""

    def test_malloc(self):
        """malloc() returns a CUDADevicePtr."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(256)
        assert isinstance(ptr, CUDADevicePtr)
        assert ptr.size == 256
        assert ptr.device_address >= 0

    def test_malloc_managed(self):
        """malloc_managed() returns a CUDADevicePtr with unified memory."""
        cuda = CUDARuntime()
        ptr = cuda.malloc_managed(512)
        assert isinstance(ptr, CUDADevicePtr)
        assert ptr.size == 512

    def test_free(self):
        """free() releases allocated memory."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(128)
        cuda.free(ptr)
        assert ptr._buffer.freed

    def test_double_free_raises(self):
        """Freeing already-freed memory raises ValueError."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(128)
        cuda.free(ptr)
        with pytest.raises(ValueError):
            cuda.free(ptr)

    def test_memcpy_host_to_device(self):
        """memcpy HostToDevice transfers data to GPU."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(16)
        data = b"\x01\x02\x03\x04" * 4
        cuda.memcpy(ptr, data, 16, CUDAMemcpyKind.HostToDevice)
        # Verify by reading back
        result = bytearray(16)
        cuda.memcpy(result, ptr, 16, CUDAMemcpyKind.DeviceToHost)
        assert result == data

    def test_memcpy_device_to_host(self):
        """memcpy DeviceToHost transfers data from GPU."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(8)
        data = b"\xAA\xBB\xCC\xDD\xEE\xFF\x00\x11"
        cuda.memcpy(ptr, data, 8, CUDAMemcpyKind.HostToDevice)
        result = bytearray(8)
        cuda.memcpy(result, ptr, 8, CUDAMemcpyKind.DeviceToHost)
        assert result == bytearray(data)

    def test_memcpy_device_to_device(self):
        """memcpy DeviceToDevice copies between GPU buffers."""
        cuda = CUDARuntime()
        src = cuda.malloc(16)
        dst = cuda.malloc(16)
        data = b"\x42" * 16
        cuda.memcpy(src, data, 16, CUDAMemcpyKind.HostToDevice)
        cuda.memcpy(dst, src, 16, CUDAMemcpyKind.DeviceToDevice)
        result = bytearray(16)
        cuda.memcpy(result, dst, 16, CUDAMemcpyKind.DeviceToHost)
        assert result == bytearray(data)

    def test_memcpy_host_to_host(self):
        """memcpy HostToHost copies between CPU buffers."""
        cuda = CUDARuntime()
        src = b"\x01\x02\x03\x04"
        dst = bytearray(4)
        cuda.memcpy(dst, src, 4, CUDAMemcpyKind.HostToHost)
        assert dst == bytearray(src)

    def test_memcpy_wrong_types_h2d(self):
        """memcpy HostToDevice with wrong types raises TypeError."""
        cuda = CUDARuntime()
        with pytest.raises(TypeError):
            cuda.memcpy(bytearray(4), b"\x00" * 4, 4, CUDAMemcpyKind.HostToDevice)

    def test_memcpy_wrong_src_h2d(self):
        """memcpy HostToDevice with non-bytes src raises TypeError."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(4)
        with pytest.raises(TypeError):
            cuda.memcpy(ptr, ptr, 4, CUDAMemcpyKind.HostToDevice)

    def test_memcpy_wrong_types_d2h(self):
        """memcpy DeviceToHost with wrong dst type raises TypeError."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(4)
        with pytest.raises(TypeError):
            cuda.memcpy(ptr, ptr, 4, CUDAMemcpyKind.DeviceToHost)

    def test_memcpy_wrong_src_d2h(self):
        """memcpy DeviceToHost with wrong src type raises TypeError."""
        cuda = CUDARuntime()
        with pytest.raises(TypeError):
            cuda.memcpy(bytearray(4), b"\x00", 4, CUDAMemcpyKind.DeviceToHost)

    def test_memcpy_wrong_types_d2d(self):
        """memcpy DeviceToDevice with wrong types raises TypeError."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(4)
        with pytest.raises(TypeError):
            cuda.memcpy(bytearray(4), ptr, 4, CUDAMemcpyKind.DeviceToDevice)

    def test_memcpy_wrong_src_d2d(self):
        """memcpy DeviceToDevice with wrong src type raises TypeError."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(4)
        with pytest.raises(TypeError):
            cuda.memcpy(ptr, b"\x00", 4, CUDAMemcpyKind.DeviceToDevice)

    def test_memcpy_wrong_types_h2h_dst(self):
        """memcpy HostToHost with wrong dst type raises TypeError."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(4)
        with pytest.raises(TypeError):
            cuda.memcpy(ptr, b"\x00", 4, CUDAMemcpyKind.HostToHost)

    def test_memcpy_wrong_types_h2h_src(self):
        """memcpy HostToHost with wrong src type raises TypeError."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(4)
        with pytest.raises(TypeError):
            cuda.memcpy(bytearray(4), ptr, 4, CUDAMemcpyKind.HostToHost)

    def test_memset(self):
        """memset fills device memory with a byte value."""
        cuda = CUDARuntime()
        ptr = cuda.malloc(16)
        cuda.memset(ptr, 0xAB, 16)
        result = bytearray(16)
        cuda.memcpy(result, ptr, 16, CUDAMemcpyKind.DeviceToHost)
        assert result == bytearray(b"\xAB" * 16)


class TestCUDAKernelLaunch:
    """Test kernel dispatch."""

    def test_launch_simple_kernel(self, simple_instructions):
        """Launch a simple kernel with one workgroup."""
        cuda = CUDARuntime()
        kernel = CUDAKernel(code=simple_instructions, name="simple")
        cuda.launch_kernel(kernel, grid=dim3(1, 1, 1), block=dim3(32, 1, 1))
        cuda.device_synchronize()

    def test_launch_with_args(self, simple_instructions):
        """Launch a kernel with buffer arguments."""
        cuda = CUDARuntime()
        d_x = cuda.malloc(128)
        d_y = cuda.malloc(128)
        kernel = CUDAKernel(code=simple_instructions, name="with_args")
        cuda.launch_kernel(
            kernel,
            grid=dim3(1, 1, 1),
            block=dim3(32, 1, 1),
            args=[d_x, d_y],
        )
        cuda.device_synchronize()

    def test_launch_on_stream(self, simple_instructions):
        """Launch a kernel on a non-default stream."""
        cuda = CUDARuntime()
        stream = cuda.create_stream()
        kernel = CUDAKernel(code=simple_instructions, name="stream_kernel")
        cuda.launch_kernel(
            kernel,
            grid=dim3(1, 1, 1),
            block=dim3(32, 1, 1),
            stream=stream,
        )
        cuda.stream_synchronize(stream)

    def test_launch_multi_workgroup(self, simple_instructions):
        """Launch with multiple workgroups."""
        cuda = CUDARuntime()
        kernel = CUDAKernel(code=simple_instructions, name="multi")
        cuda.launch_kernel(kernel, grid=dim3(4, 2, 1), block=dim3(32, 1, 1))
        cuda.device_synchronize()

    def test_dim3_fields(self):
        """dim3 has x, y, z fields."""
        d = dim3(4, 2, 1)
        assert d.x == 4
        assert d.y == 2
        assert d.z == 1

    def test_kernel_name(self):
        """CUDAKernel stores its name."""
        kernel = CUDAKernel(code=[halt()], name="test_kernel")
        assert kernel.name == "test_kernel"

    def test_launch_no_args(self, nop_instructions):
        """Launch with no buffer args."""
        cuda = CUDARuntime()
        kernel = CUDAKernel(code=nop_instructions, name="no_args")
        cuda.launch_kernel(kernel, grid=dim3(1, 1, 1), block=dim3(32, 1, 1))


class TestCUDAStreams:
    """Test stream management."""

    def test_create_stream(self):
        """create_stream() returns a CUDAStream."""
        cuda = CUDARuntime()
        stream = cuda.create_stream()
        assert isinstance(stream, CUDAStream)

    def test_destroy_stream(self):
        """destroy_stream() removes the stream."""
        cuda = CUDARuntime()
        stream = cuda.create_stream()
        cuda.destroy_stream(stream)
        assert stream not in cuda._streams

    def test_destroy_invalid_stream(self):
        """destroy_stream() with unknown stream raises ValueError."""
        cuda = CUDARuntime()
        stream = cuda.create_stream()
        cuda.destroy_stream(stream)
        with pytest.raises(ValueError):
            cuda.destroy_stream(stream)

    def test_stream_synchronize(self):
        """stream_synchronize() waits for stream to complete."""
        cuda = CUDARuntime()
        stream = cuda.create_stream()
        cuda.stream_synchronize(stream)  # No pending work, should not raise

    def test_multiple_streams(self):
        """Can create multiple independent streams."""
        cuda = CUDARuntime()
        s1 = cuda.create_stream()
        s2 = cuda.create_stream()
        assert s1 is not s2
        assert len(cuda._streams) == 2


class TestCUDAEvents:
    """Test event timing and synchronization."""

    def test_create_event(self):
        """create_event() returns a CUDAEvent."""
        cuda = CUDARuntime()
        event = cuda.create_event()
        assert isinstance(event, CUDAEvent)

    def test_record_event(self):
        """record_event() marks the event as recorded."""
        cuda = CUDARuntime()
        event = cuda.create_event()
        cuda.record_event(event)
        assert event._recorded

    def test_record_event_on_stream(self):
        """record_event() on a specific stream."""
        cuda = CUDARuntime()
        stream = cuda.create_stream()
        event = cuda.create_event()
        cuda.record_event(event, stream)
        assert event._recorded

    def test_synchronize_event(self):
        """synchronize_event() waits for a recorded event."""
        cuda = CUDARuntime()
        event = cuda.create_event()
        cuda.record_event(event)
        cuda.synchronize_event(event)

    def test_synchronize_unrecorded_event(self):
        """synchronize_event() on unrecorded event raises RuntimeError."""
        cuda = CUDARuntime()
        event = cuda.create_event()
        with pytest.raises(RuntimeError, match="never recorded"):
            cuda.synchronize_event(event)

    def test_elapsed_time(self):
        """elapsed_time() returns a float >= 0."""
        cuda = CUDARuntime()
        start = cuda.create_event()
        end = cuda.create_event()
        cuda.record_event(start)
        cuda.record_event(end)
        elapsed = cuda.elapsed_time(start, end)
        assert isinstance(elapsed, float)
        assert elapsed >= 0.0

    def test_elapsed_time_unrecorded_start(self):
        """elapsed_time() with unrecorded start raises RuntimeError."""
        cuda = CUDARuntime()
        start = cuda.create_event()
        end = cuda.create_event()
        cuda.record_event(end)
        with pytest.raises(RuntimeError, match="Start event"):
            cuda.elapsed_time(start, end)

    def test_elapsed_time_unrecorded_end(self):
        """elapsed_time() with unrecorded end raises RuntimeError."""
        cuda = CUDARuntime()
        start = cuda.create_event()
        end = cuda.create_event()
        cuda.record_event(start)
        with pytest.raises(RuntimeError, match="End event"):
            cuda.elapsed_time(start, end)
