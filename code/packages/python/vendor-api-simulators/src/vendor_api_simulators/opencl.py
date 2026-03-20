"""OpenCL Runtime Simulator — cross-platform "portable compute" model.

=== What is OpenCL? ===

OpenCL (Open Computing Language) is the Khronos Group's cross-platform
compute API. Unlike CUDA (NVIDIA only), OpenCL runs on any vendor's GPU,
and even on CPUs and FPGAs. The tradeoff is more boilerplate — you must
explicitly manage platforms, devices, contexts, and command queues.

=== The OpenCL Object Hierarchy ===

    CLPlatform          "Which vendor's implementation?"
        └── CLDevice    "Which specific GPU/CPU?"
    CLContext            "A group of devices I want to use together"
        ├── CLBuffer     "Memory on one of the context's devices"
        ├── CLProgram    "Source code, not yet compiled"
        │   └── CLKernel "Compiled function, ready to dispatch"
        └── CLCommandQueue "Where I enqueue operations"
                └── CLEvent "Dependency token for operation ordering"

=== Event-Based Dependencies ===

OpenCL's most distinctive feature is its event model. Every enqueue
operation returns a CLEvent. You can pass event lists to subsequent
operations to create dependency chains:

    ev1 = queue.enqueue_write_buffer(buf_x, data_x)
    ev2 = queue.enqueue_write_buffer(buf_y, data_y)
    ev3 = queue.enqueue_nd_range_kernel(kernel, wait_list=[ev1, ev2])
    ev4 = queue.enqueue_read_buffer(buf_y, wait_list=[ev3])

This is more flexible than CUDA's stream model because dependencies
can form arbitrary DAGs, not just linear sequences.

=== Memory Model ===

OpenCL uses simple flags instead of Vulkan's memory types:

    READ_WRITE:    GPU can read and write (most common)
    READ_ONLY:     GPU can only read (compiler can optimize)
    WRITE_ONLY:    GPU can only write (compiler can optimize)
    COPY_HOST_PTR: Initialize buffer contents from host memory
    ALLOC_HOST_PTR: Allocate in host-accessible memory
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum, Flag, auto
from typing import Any

from compute_runtime import (
    BufferUsage,
    DescriptorBinding,
    Fence,
    MemoryType,
    PhysicalDevice,
)
from compute_runtime import Buffer as RuntimeBuffer

from ._base import BaseVendorSimulator


# =========================================================================
# OpenCL enums and flags
# =========================================================================


class CLDeviceType(Enum):
    """OpenCL device types for filtering during discovery.

    GPU:         Graphics processing unit
    CPU:         Central processing unit (OpenCL can run on CPUs too!)
    ACCELERATOR: Custom accelerator (FPGA, DSP, etc.)
    ALL:         Any device type
    """

    GPU = "gpu"
    CPU = "cpu"
    ACCELERATOR = "accelerator"
    ALL = "all"


class CLMemFlags(Flag):
    """OpenCL memory flags — simpler than Vulkan's memory types.

    READ_WRITE:    Default. GPU can read and write this buffer.
    READ_ONLY:     GPU can only read. Allows compiler optimization.
    WRITE_ONLY:    GPU can only write. Allows compiler optimization.
    COPY_HOST_PTR: Initialize buffer contents from provided host data.
    USE_HOST_PTR:  Use the host pointer directly (zero-copy if possible).
    ALLOC_HOST_PTR: Allocate in host-visible memory for CPU access.
    """

    READ_WRITE = auto()
    READ_ONLY = auto()
    WRITE_ONLY = auto()
    COPY_HOST_PTR = auto()
    USE_HOST_PTR = auto()
    ALLOC_HOST_PTR = auto()


class CLBuildStatus(Enum):
    """Build status of a CLProgram.

    SUCCESS:     Build completed successfully.
    ERROR:       Build failed (syntax errors, unsupported features).
    IN_PROGRESS: Build is currently running (async builds).
    NONE:        Program hasn't been built yet.
    """

    SUCCESS = "success"
    ERROR = "error"
    IN_PROGRESS = "in_progress"
    NONE = "none"


class CLEventStatus(Enum):
    """Status of an OpenCL event.

    QUEUED:    Operation is in the command queue but hasn't started.
    SUBMITTED: Operation has been sent to the device.
    RUNNING:   Operation is currently executing on the device.
    COMPLETE:  Operation has finished.
    """

    QUEUED = "queued"
    SUBMITTED = "submitted"
    RUNNING = "running"
    COMPLETE = "complete"


class CLDeviceInfo(Enum):
    """Device info parameter IDs for CLDevice.get_info()."""

    NAME = "name"
    TYPE = "type"
    MAX_COMPUTE_UNITS = "max_compute_units"
    MAX_WORK_GROUP_SIZE = "max_work_group_size"
    GLOBAL_MEM_SIZE = "global_mem_size"


# =========================================================================
# CLEvent — dependency token
# =========================================================================


class CLEvent:
    """An OpenCL event — a dependency token for operation ordering.

    Every enqueue operation returns a CLEvent. You can:
    - Wait on it (blocking the CPU)
    - Pass it in wait_list to another operation (GPU-side dependency)
    - Query its status

    Internally wraps a Layer 5 Fence.
    """

    def __init__(self, fence: Fence) -> None:
        self._fence = fence

    def wait(self) -> None:
        """Block until this event completes (clWaitForEvents with one event)."""
        self._fence.wait()

    @property
    def status(self) -> CLEventStatus:
        """Query the current status of this event."""
        if self._fence.signaled:
            return CLEventStatus.COMPLETE
        return CLEventStatus.QUEUED


# =========================================================================
# CLDevice — wraps PhysicalDevice
# =========================================================================


class CLDevice:
    """An OpenCL device — a specific piece of hardware.

    Wraps a Layer 5 PhysicalDevice with OpenCL-style property queries.
    """

    def __init__(self, physical_device: PhysicalDevice) -> None:
        self._physical = physical_device

    @property
    def name(self) -> str:
        """Device name string."""
        return self._physical.name

    @property
    def device_type(self) -> CLDeviceType:
        """The device type (GPU, CPU, etc.)."""
        dt = self._physical.device_type.value
        if dt == "gpu":
            return CLDeviceType.GPU
        if dt == "tpu":
            return CLDeviceType.ACCELERATOR
        if dt == "npu":
            return CLDeviceType.ACCELERATOR
        return CLDeviceType.GPU

    @property
    def max_compute_units(self) -> int:
        """Number of compute units (SMs on NVIDIA, CUs on AMD)."""
        return 4  # Simplified — real value would come from device config

    @property
    def max_work_group_size(self) -> int:
        """Maximum work items per work group."""
        return self._physical.limits.max_workgroup_size[0]

    @property
    def global_mem_size(self) -> int:
        """Total global memory in bytes."""
        return sum(h.size for h in self._physical.memory_properties.heaps)

    def get_info(self, param: CLDeviceInfo) -> Any:
        """Query device information by parameter ID.

        Args:
            param: Which property to query.

        Returns:
            The requested value.
        """
        info_map: dict[CLDeviceInfo, Any] = {
            CLDeviceInfo.NAME: self.name,
            CLDeviceInfo.TYPE: self.device_type,
            CLDeviceInfo.MAX_COMPUTE_UNITS: self.max_compute_units,
            CLDeviceInfo.MAX_WORK_GROUP_SIZE: self.max_work_group_size,
            CLDeviceInfo.GLOBAL_MEM_SIZE: self.global_mem_size,
        }
        return info_map[param]


# =========================================================================
# CLBuffer — wraps Buffer
# =========================================================================


class CLBuffer:
    """An OpenCL buffer — memory allocated on a device.

    Wraps a Layer 5 Buffer with OpenCL memory flag semantics.
    """

    def __init__(self, buffer: RuntimeBuffer, size: int, flags: CLMemFlags) -> None:
        self._buffer = buffer
        self._size = size
        self._flags = flags

    @property
    def size(self) -> int:
        """Buffer size in bytes."""
        return self._size

    @property
    def flags(self) -> CLMemFlags:
        """Memory flags this buffer was created with."""
        return self._flags


# =========================================================================
# CLKernel — a compiled kernel function
# =========================================================================


class CLKernel:
    """An OpenCL kernel — a compiled function extracted from a CLProgram.

    In OpenCL, kernel arguments are set one at a time with set_arg().
    This is different from CUDA (args passed at launch) and Vulkan
    (args bound via descriptor sets).
    """

    def __init__(self, name: str, code: list[Any] | None = None) -> None:
        self._name = name
        self._code = code
        self._args: dict[int, CLBuffer | int | float | bytes] = {}

    @property
    def name(self) -> str:
        """Kernel function name."""
        return self._name

    def set_arg(self, index: int, value: CLBuffer | int | float | bytes) -> None:
        """Set a kernel argument at the given index.

        In OpenCL, arguments are set individually before enqueueing:

            kernel.set_arg(0, buf_x)    # binding 0 = input X
            kernel.set_arg(1, buf_y)    # binding 1 = output Y
            kernel.set_arg(2, alpha)    # binding 2 = scalar alpha

        Args:
            index: Argument index (0-based).
            value: The argument value — a CLBuffer, scalar, or bytes.
        """
        self._args[index] = value


# =========================================================================
# CLProgram — source code + compilation
# =========================================================================


class CLProgram:
    """An OpenCL program — source code that can be compiled for a device.

    OpenCL uses runtime compilation: you provide kernel source as a string,
    call build(), and the OpenCL implementation compiles it for the target
    device. This is how OpenCL achieves portability — the same source code
    is compiled for NVIDIA, AMD, Intel, etc.

    In our simulator, "compilation" creates a Layer 5 ShaderModule.
    """

    def __init__(
        self,
        source: str,
        context: CLContext,
    ) -> None:
        self._source = source
        self._context = context
        self._build_status = CLBuildStatus.NONE
        self._kernels: dict[str, list[Any] | None] = {}

    @property
    def build_status(self) -> CLBuildStatus:
        """Current build status."""
        return self._build_status

    def build(
        self,
        devices: list[CLDevice] | None = None,
        options: str = "",
    ) -> None:
        """Compile the program for the target device(s).

        In real OpenCL, this invokes the vendor's compiler to produce
        device-specific binary code. In our simulator, we just mark the
        program as built — the "compilation" happens when we create a
        ShaderModule at kernel launch time.

        Args:
            devices: Target devices. None = all devices in context.
            options: Compiler options string (ignored in simulator).
        """
        self._build_status = CLBuildStatus.SUCCESS

    def create_kernel(self, name: str) -> CLKernel:
        """Extract a kernel function from the compiled program.

        Args:
            name: The kernel function name.

        Returns:
            A CLKernel ready for argument binding and dispatch.

        Raises:
            RuntimeError: If the program hasn't been built yet.
        """
        if self._build_status != CLBuildStatus.SUCCESS:
            raise RuntimeError(
                f"Program not built (status: {self._build_status.value}). "
                "Call program.build() first."
            )
        return CLKernel(name=name, code=self._kernels.get(name))


# =========================================================================
# CLCommandQueue — enqueue operations with event dependencies
# =========================================================================


class CLCommandQueue:
    """An OpenCL command queue — where operations are enqueued.

    Every operation returns a CLEvent for dependency tracking. You can
    pass event wait_lists to subsequent operations to create execution
    order dependencies.
    """

    def __init__(self, context: CLContext, device: CLDevice) -> None:
        self._context = context
        self._device = device

    def enqueue_nd_range_kernel(
        self,
        kernel: CLKernel,
        global_size: tuple[int, ...],
        local_size: tuple[int, ...] | None = None,
        wait_list: list[CLEvent] | None = None,
    ) -> CLEvent:
        """Enqueue a kernel for execution (clEnqueueNDRangeKernel).

        === The Core Dispatch ===

        This is OpenCL's equivalent of CUDA's kernel<<<>>>() operator.
        The key differences:

        1. Arguments were set beforehand via kernel.set_arg()
        2. Returns a CLEvent for dependency tracking
        3. Can wait on other events before executing

        The global_size specifies total work items. If local_size is None,
        the OpenCL runtime picks an optimal workgroup size.

        Args:
            kernel:      The kernel to execute.
            global_size: Total number of work items per dimension.
            local_size:  Work items per workgroup. None = auto-select.
            wait_list:   Events to wait for before executing.

        Returns:
            A CLEvent that signals when the kernel completes.
        """
        # Wait for dependency events
        for event in (wait_list or []):
            event.wait()

        device = self._context._logical_device
        mm = self._context._memory_manager

        # Determine local size (workgroup size)
        if local_size is None:
            local = (32, 1, 1)
        else:
            local = (
                local_size[0],
                local_size[1] if len(local_size) > 1 else 1,
                local_size[2] if len(local_size) > 2 else 1,
            )

        # Calculate grid dimensions (number of workgroups)
        grid_x = max(1, (global_size[0] + local[0] - 1) // local[0])
        grid_y = (
            max(1, (global_size[1] + local[1] - 1) // local[1])
            if len(global_size) > 1
            else 1
        )
        grid_z = (
            max(1, (global_size[2] + local[2] - 1) // local[2])
            if len(global_size) > 2
            else 1
        )

        # Create shader module from kernel code
        shader = device.create_shader_module(
            code=kernel._code,
            local_size=local,
        )

        # Build descriptor set from kernel arguments
        buffer_args = {
            i: arg
            for i, arg in kernel._args.items()
            if isinstance(arg, CLBuffer)
        }
        bindings = [
            DescriptorBinding(binding=i, type="storage")
            for i in sorted(buffer_args.keys())
        ]
        ds_layout = device.create_descriptor_set_layout(bindings)
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        ds = device.create_descriptor_set(ds_layout)
        for i in sorted(buffer_args.keys()):
            ds.write(i, buffer_args[i]._buffer)

        # Record and submit
        fence = device.create_fence()
        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_bind_descriptor_set(ds)
        cb.cmd_dispatch(grid_x, grid_y, grid_z)
        cb.end()

        queue = self._context._compute_queue
        queue.submit([cb], fence=fence)
        fence.wait()

        return CLEvent(fence)

    def enqueue_write_buffer(
        self,
        buffer: CLBuffer,
        offset: int,
        size: int,
        host_ptr: bytes | bytearray,
        wait_list: list[CLEvent] | None = None,
    ) -> CLEvent:
        """Write host data to a device buffer (clEnqueueWriteBuffer).

        Args:
            buffer:   Destination device buffer.
            offset:   Byte offset in the buffer.
            size:     Bytes to write.
            host_ptr: Source host data.
            wait_list: Events to wait for first.

        Returns:
            CLEvent signaling when the write is complete.
        """
        for event in (wait_list or []):
            event.wait()

        mm = self._context._memory_manager
        mapped = mm.map(buffer._buffer)
        mapped.write(offset, bytes(host_ptr[:size]))
        mm.unmap(buffer._buffer)

        fence = self._context._logical_device.create_fence(signaled=True)
        return CLEvent(fence)

    def enqueue_read_buffer(
        self,
        buffer: CLBuffer,
        offset: int,
        size: int,
        host_ptr: bytearray,
        wait_list: list[CLEvent] | None = None,
    ) -> CLEvent:
        """Read device buffer data to host memory (clEnqueueReadBuffer).

        Args:
            buffer:   Source device buffer.
            offset:   Byte offset in the buffer.
            size:     Bytes to read.
            host_ptr: Destination host buffer.
            wait_list: Events to wait for first.

        Returns:
            CLEvent signaling when the read is complete.
        """
        for event in (wait_list or []):
            event.wait()

        mm = self._context._memory_manager
        mm.invalidate(buffer._buffer)
        mapped = mm.map(buffer._buffer)
        data = mapped.read(offset, size)
        mm.unmap(buffer._buffer)
        host_ptr[:size] = data

        fence = self._context._logical_device.create_fence(signaled=True)
        return CLEvent(fence)

    def enqueue_copy_buffer(
        self,
        src: CLBuffer,
        dst: CLBuffer,
        size: int,
        wait_list: list[CLEvent] | None = None,
    ) -> CLEvent:
        """Copy between two device buffers (clEnqueueCopyBuffer).

        Args:
            src:  Source buffer.
            dst:  Destination buffer.
            size: Bytes to copy.
            wait_list: Events to wait for first.

        Returns:
            CLEvent signaling when the copy is complete.
        """
        for event in (wait_list or []):
            event.wait()

        device = self._context._logical_device
        fence = device.create_fence()
        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_copy_buffer(src._buffer, dst._buffer, size)
        cb.end()
        self._context._compute_queue.submit([cb], fence=fence)
        fence.wait()

        return CLEvent(fence)

    def enqueue_fill_buffer(
        self,
        buffer: CLBuffer,
        pattern: bytes,
        offset: int,
        size: int,
    ) -> CLEvent:
        """Fill a buffer with a pattern (clEnqueueFillBuffer).

        Args:
            buffer:  Buffer to fill.
            pattern: Byte pattern to repeat.
            offset:  Start offset.
            size:    Bytes to fill.

        Returns:
            CLEvent signaling when the fill is complete.
        """
        device = self._context._logical_device
        fence = device.create_fence()
        cb = device.create_command_buffer()
        cb.begin()
        cb.cmd_fill_buffer(buffer._buffer, pattern[0] if pattern else 0, offset, size)
        cb.end()
        self._context._compute_queue.submit([cb], fence=fence)
        fence.wait()

        return CLEvent(fence)

    def finish(self) -> None:
        """Block until all enqueued operations complete (clFinish).

        This is the OpenCL equivalent of cudaDeviceSynchronize() — it
        waits for everything in this queue to finish.
        """
        self._context._logical_device.wait_idle()

    def flush(self) -> None:
        """Ensure all enqueued operations are submitted (clFlush).

        In real OpenCL, flush() ensures that all operations have been
        submitted to the device, but doesn't wait for them to complete.
        In our synchronous simulator, this is a no-op because operations
        complete immediately upon enqueue.
        """
        pass


# =========================================================================
# CLContext — the OpenCL execution context
# =========================================================================


class CLContext(BaseVendorSimulator):
    """An OpenCL context — groups devices and manages shared resources.

    In OpenCL, a context is the scope for resource sharing. Buffers and
    programs are created within a context and can be used on any device
    in that context.

    Our simulator creates a context with a single device, wrapping the
    Layer 5 LogicalDevice.
    """

    def __init__(self, devices: list[CLDevice] | None = None) -> None:
        """Create an OpenCL context.

        Args:
            devices: Devices to include. If None, uses the first GPU.
        """
        if devices:
            vendor = devices[0]._physical.vendor
            super().__init__(vendor_hint=vendor)
            self._devices = devices
        else:
            super().__init__()
            self._devices = [
                CLDevice(pd) for pd in self._physical_devices
            ]

    def create_buffer(
        self,
        flags: CLMemFlags,
        size: int,
        host_ptr: bytes | None = None,
    ) -> CLBuffer:
        """Create a device buffer (clCreateBuffer).

        Maps OpenCL memory flags to Layer 5 memory types:
        - READ_WRITE, READ_ONLY, WRITE_ONLY → DEVICE_LOCAL + HOST_VISIBLE
        - ALLOC_HOST_PTR → HOST_VISIBLE + HOST_COHERENT
        - COPY_HOST_PTR → allocate + write initial data

        Args:
            flags:    Memory flags.
            size:     Buffer size in bytes.
            host_ptr: Optional initial data (used with COPY_HOST_PTR).

        Returns:
            A new CLBuffer.
        """
        mem_type = (
            MemoryType.DEVICE_LOCAL
            | MemoryType.HOST_VISIBLE
            | MemoryType.HOST_COHERENT
        )
        usage = BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST

        buf = self._memory_manager.allocate(size, mem_type, usage=usage)
        cl_buf = CLBuffer(buf, size, flags)

        # If COPY_HOST_PTR, write the initial data
        if host_ptr is not None and CLMemFlags.COPY_HOST_PTR in flags:
            mapped = self._memory_manager.map(buf)
            mapped.write(0, bytes(host_ptr[:size]))
            self._memory_manager.unmap(buf)

        return cl_buf

    def create_program_with_source(self, source: str) -> CLProgram:
        """Create a program from source code (clCreateProgramWithSource).

        In real OpenCL, this takes OpenCL C source code. In our simulator,
        the "source" is a string label that identifies which kernel to
        use. Actual GPU instructions are provided via CLKernel.

        Args:
            source: Kernel source code (or name label in our simulator).

        Returns:
            A CLProgram ready for build().
        """
        return CLProgram(source, self)

    def create_command_queue(
        self,
        device: CLDevice | None = None,
        properties: int = 0,
    ) -> CLCommandQueue:
        """Create a command queue for a device (clCreateCommandQueue).

        Args:
            device:     Target device. None = first device in context.
            properties: Queue properties (ignored in simulator).

        Returns:
            A new CLCommandQueue.
        """
        dev = device if device else self._devices[0]
        return CLCommandQueue(self, dev)


# =========================================================================
# CLPlatform — the top-level discovery object
# =========================================================================


class CLPlatform:
    """An OpenCL platform — represents a vendor's OpenCL implementation.

    In real OpenCL, your system might have multiple platforms:
    - NVIDIA's OpenCL implementation
    - Intel's OpenCL implementation
    - AMD's OpenCL implementation

    Each platform has its own set of devices. In our simulator, there's
    one platform wrapping our Layer 5 runtime.
    """

    def __init__(self) -> None:
        self._instance = BaseVendorSimulator.__new__(BaseVendorSimulator)
        self._instance._instance = __import__(
            "compute_runtime"
        ).RuntimeInstance()
        self._instance._physical_devices = (
            self._instance._instance.enumerate_physical_devices()
        )
        self._name = "Coding Adventures Compute Platform"
        self._vendor = "Coding Adventures"
        self._version = "OpenCL 3.0"

    @classmethod
    def get_platforms(cls) -> list[CLPlatform]:
        """Enumerate available OpenCL platforms (clGetPlatformIDs).

        Returns a list of platforms. In our simulator, there's always
        exactly one platform.

        Returns:
            List containing one CLPlatform.
        """
        return [cls()]

    @property
    def name(self) -> str:
        """Platform name."""
        return self._name

    @property
    def vendor(self) -> str:
        """Platform vendor."""
        return self._vendor

    @property
    def version(self) -> str:
        """Platform version string."""
        return self._version

    def get_devices(
        self, device_type: CLDeviceType = CLDeviceType.ALL
    ) -> list[CLDevice]:
        """Get devices of a specific type on this platform.

        Args:
            device_type: Filter by device type. ALL = return everything.

        Returns:
            List of matching CLDevice objects.
        """
        devices = [
            CLDevice(pd) for pd in self._instance._physical_devices
        ]
        if device_type == CLDeviceType.ALL:
            return devices
        return [d for d in devices if d.device_type == device_type]
