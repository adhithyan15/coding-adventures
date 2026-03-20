"""Tests for the OpenCL runtime simulator."""

import pytest
from gpu_core import limm, halt

from vendor_api_simulators.opencl import (
    CLPlatform,
    CLDevice,
    CLContext,
    CLCommandQueue,
    CLProgram,
    CLKernel,
    CLBuffer,
    CLEvent,
    CLMemFlags,
    CLDeviceType,
    CLBuildStatus,
    CLEventStatus,
    CLDeviceInfo,
)


class TestCLPlatform:
    """Test platform discovery."""

    def test_get_platforms(self):
        """get_platforms() returns a non-empty list."""
        platforms = CLPlatform.get_platforms()
        assert len(platforms) >= 1

    def test_platform_properties(self):
        """Platform has name, vendor, version."""
        platform = CLPlatform.get_platforms()[0]
        assert len(platform.name) > 0
        assert len(platform.vendor) > 0
        assert len(platform.version) > 0

    def test_get_devices_all(self):
        """get_devices(ALL) returns all devices."""
        platform = CLPlatform.get_platforms()[0]
        devices = platform.get_devices(CLDeviceType.ALL)
        assert len(devices) >= 1

    def test_get_devices_gpu(self):
        """get_devices(GPU) returns GPU devices."""
        platform = CLPlatform.get_platforms()[0]
        devices = platform.get_devices(CLDeviceType.GPU)
        assert len(devices) >= 1
        for dev in devices:
            assert dev.device_type == CLDeviceType.GPU

    def test_get_devices_accelerator(self):
        """get_devices(ACCELERATOR) filters to accelerators."""
        platform = CLPlatform.get_platforms()[0]
        devices = platform.get_devices(CLDeviceType.ACCELERATOR)
        # May or may not have accelerators
        for dev in devices:
            assert dev.device_type == CLDeviceType.ACCELERATOR


class TestCLDevice:
    """Test device properties."""

    def test_device_name(self):
        """Device has a non-empty name."""
        platform = CLPlatform.get_platforms()[0]
        devices = platform.get_devices(CLDeviceType.ALL)
        assert len(devices[0].name) > 0

    def test_device_info(self):
        """get_info() returns expected values."""
        platform = CLPlatform.get_platforms()[0]
        device = platform.get_devices(CLDeviceType.ALL)[0]
        assert device.get_info(CLDeviceInfo.NAME) == device.name
        assert device.get_info(CLDeviceInfo.MAX_WORK_GROUP_SIZE) > 0
        assert device.get_info(CLDeviceInfo.GLOBAL_MEM_SIZE) > 0
        assert device.get_info(CLDeviceInfo.MAX_COMPUTE_UNITS) > 0

    def test_device_properties(self):
        """Device exposes max_work_group_size and global_mem_size."""
        platform = CLPlatform.get_platforms()[0]
        device = platform.get_devices(CLDeviceType.ALL)[0]
        assert device.max_work_group_size > 0
        assert device.global_mem_size > 0
        assert device.max_compute_units > 0


class TestCLContext:
    """Test context and resource creation."""

    def test_create_context(self):
        """CLContext initializes successfully."""
        ctx = CLContext()
        assert ctx._logical_device is not None

    def test_create_context_with_devices(self):
        """CLContext with explicit devices."""
        platform = CLPlatform.get_platforms()[0]
        devices = platform.get_devices(CLDeviceType.ALL)
        ctx = CLContext(devices[:1])
        assert len(ctx._devices) >= 1

    def test_create_buffer_read_write(self):
        """create_buffer with READ_WRITE flag."""
        ctx = CLContext()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 256)
        assert isinstance(buf, CLBuffer)
        assert buf.size == 256
        assert CLMemFlags.READ_WRITE in buf.flags

    def test_create_buffer_read_only(self):
        """create_buffer with READ_ONLY flag."""
        ctx = CLContext()
        buf = ctx.create_buffer(CLMemFlags.READ_ONLY, 128)
        assert CLMemFlags.READ_ONLY in buf.flags

    def test_create_buffer_copy_host_ptr(self):
        """create_buffer with COPY_HOST_PTR initializes data."""
        ctx = CLContext()
        data = b"\xAA\xBB\xCC\xDD" * 4
        buf = ctx.create_buffer(
            CLMemFlags.READ_WRITE | CLMemFlags.COPY_HOST_PTR,
            16,
            host_ptr=data,
        )
        assert buf.size == 16

    def test_create_program(self):
        """create_program_with_source returns a CLProgram."""
        ctx = CLContext()
        prog = ctx.create_program_with_source("saxpy")
        assert isinstance(prog, CLProgram)

    def test_create_command_queue(self):
        """create_command_queue returns a CLCommandQueue."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        assert isinstance(queue, CLCommandQueue)

    def test_create_command_queue_with_device(self):
        """create_command_queue with specific device."""
        ctx = CLContext()
        queue = ctx.create_command_queue(ctx._devices[0])
        assert isinstance(queue, CLCommandQueue)


class TestCLProgram:
    """Test program compilation."""

    def test_build_status_initial(self):
        """Newly created program has NONE build status."""
        ctx = CLContext()
        prog = ctx.create_program_with_source("test")
        assert prog.build_status == CLBuildStatus.NONE

    def test_build_success(self):
        """build() sets status to SUCCESS."""
        ctx = CLContext()
        prog = ctx.create_program_with_source("test")
        prog.build()
        assert prog.build_status == CLBuildStatus.SUCCESS

    def test_create_kernel_before_build(self):
        """create_kernel() before build raises RuntimeError."""
        ctx = CLContext()
        prog = ctx.create_program_with_source("test")
        with pytest.raises(RuntimeError, match="not built"):
            prog.create_kernel("main")

    def test_create_kernel_after_build(self):
        """create_kernel() after build returns a CLKernel."""
        ctx = CLContext()
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("main")
        assert isinstance(kernel, CLKernel)
        assert kernel.name == "main"


class TestCLKernel:
    """Test kernel argument setting."""

    def test_set_arg_buffer(self):
        """set_arg with a CLBuffer."""
        ctx = CLContext()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 256)
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel.set_arg(0, buf)
        assert kernel._args[0] is buf

    def test_set_arg_scalar(self):
        """set_arg with a scalar value."""
        ctx = CLContext()
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel.set_arg(0, 42)
        assert kernel._args[0] == 42

    def test_set_multiple_args(self):
        """Set multiple args at different indices."""
        ctx = CLContext()
        buf1 = ctx.create_buffer(CLMemFlags.READ_WRITE, 64)
        buf2 = ctx.create_buffer(CLMemFlags.READ_WRITE, 64)
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel.set_arg(0, buf1)
        kernel.set_arg(1, buf2)
        assert len(kernel._args) == 2


class TestCLCommandQueue:
    """Test command queue operations."""

    def test_enqueue_write_buffer(self):
        """enqueue_write_buffer writes data and returns CLEvent."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 16)
        data = b"\x01\x02\x03\x04" * 4
        event = queue.enqueue_write_buffer(buf, 0, 16, data)
        assert isinstance(event, CLEvent)

    def test_enqueue_read_buffer(self):
        """enqueue_read_buffer reads data back."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 8)

        # Write data first
        data = b"\xAA\xBB\xCC\xDD\xEE\xFF\x00\x11"
        queue.enqueue_write_buffer(buf, 0, 8, data)

        # Read it back
        result = bytearray(8)
        event = queue.enqueue_read_buffer(buf, 0, 8, result)
        assert isinstance(event, CLEvent)
        assert result == bytearray(data)

    def test_enqueue_nd_range_kernel(self, simple_instructions):
        """enqueue_nd_range_kernel dispatches and returns CLEvent."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel._code = simple_instructions
        event = queue.enqueue_nd_range_kernel(
            kernel, global_size=(32,), local_size=(32,)
        )
        assert isinstance(event, CLEvent)

    def test_enqueue_with_wait_list(self, simple_instructions):
        """enqueue_nd_range_kernel respects wait_list."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 64)

        # Write data, then dispatch waiting on the write
        ev_write = queue.enqueue_write_buffer(buf, 0, 64, b"\x00" * 64)
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel._code = simple_instructions
        kernel.set_arg(0, buf)
        ev_kernel = queue.enqueue_nd_range_kernel(
            kernel, global_size=(32,), local_size=(32,),
            wait_list=[ev_write],
        )
        assert ev_kernel.status == CLEventStatus.COMPLETE

    def test_enqueue_nd_range_auto_local(self, simple_instructions):
        """enqueue_nd_range_kernel with local_size=None (auto-select)."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel._code = simple_instructions
        event = queue.enqueue_nd_range_kernel(
            kernel, global_size=(64,), local_size=None
        )
        assert isinstance(event, CLEvent)

    def test_enqueue_copy_buffer(self):
        """enqueue_copy_buffer copies between buffers."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        src = ctx.create_buffer(CLMemFlags.READ_WRITE, 16)
        dst = ctx.create_buffer(CLMemFlags.READ_WRITE, 16)
        queue.enqueue_write_buffer(src, 0, 16, b"\x42" * 16)
        event = queue.enqueue_copy_buffer(src, dst, 16)
        assert isinstance(event, CLEvent)

    def test_enqueue_fill_buffer(self):
        """enqueue_fill_buffer fills a buffer."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 16)
        event = queue.enqueue_fill_buffer(buf, b"\xFF", 0, 16)
        assert isinstance(event, CLEvent)

    def test_finish(self):
        """finish() completes without error."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        queue.finish()

    def test_flush(self):
        """flush() completes without error."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        queue.flush()


class TestCLEvent:
    """Test event status and waiting."""

    def test_event_complete_status(self):
        """Completed event has COMPLETE status."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 8)
        event = queue.enqueue_write_buffer(buf, 0, 8, b"\x00" * 8)
        assert event.status == CLEventStatus.COMPLETE

    def test_event_wait(self):
        """event.wait() completes without error."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 8)
        event = queue.enqueue_write_buffer(buf, 0, 8, b"\x00" * 8)
        event.wait()  # Should not raise

    def test_event_dependency_chain(self, simple_instructions):
        """Events can form dependency chains."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        buf = ctx.create_buffer(CLMemFlags.READ_WRITE, 32)

        ev1 = queue.enqueue_write_buffer(buf, 0, 32, b"\x00" * 32)
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel._code = simple_instructions
        kernel.set_arg(0, buf)
        ev2 = queue.enqueue_nd_range_kernel(
            kernel, global_size=(32,), wait_list=[ev1]
        )
        result = bytearray(32)
        ev3 = queue.enqueue_read_buffer(buf, 0, 32, result, wait_list=[ev2])
        assert ev3.status == CLEventStatus.COMPLETE

    def test_multidim_global_size(self, simple_instructions):
        """enqueue_nd_range_kernel with 2D and 3D global sizes."""
        ctx = CLContext()
        queue = ctx.create_command_queue()
        prog = ctx.create_program_with_source("test")
        prog.build()
        kernel = prog.create_kernel("test")
        kernel._code = simple_instructions
        # 2D
        ev = queue.enqueue_nd_range_kernel(
            kernel, global_size=(32, 32), local_size=(8, 8)
        )
        assert isinstance(ev, CLEvent)
        # 3D
        ev = queue.enqueue_nd_range_kernel(
            kernel, global_size=(16, 16, 4), local_size=(4, 4, 2)
        )
        assert isinstance(ev, CLEvent)
