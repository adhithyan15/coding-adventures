"""CUDA Runtime Simulator — NVIDIA's "just launch it" GPU programming model.

=== What is CUDA? ===

CUDA (Compute Unified Device Architecture) is NVIDIA's proprietary GPU
computing platform. It's the most popular GPU programming API, used by
PyTorch, TensorFlow, and virtually all ML research.

CUDA's design philosophy is **"make the common case easy."** The common
case for GPU programming is:

    1. Allocate memory on the GPU          --> cudaMalloc()
    2. Copy data from CPU to GPU           --> cudaMemcpy(HostToDevice)
    3. Launch a kernel                     --> kernel<<<grid, block>>>(args)
    4. Copy results back                   --> cudaMemcpy(DeviceToHost)
    5. Free memory                         --> cudaFree()

Each of these is a single function call. Compare this to Vulkan, where
launching a kernel requires creating a pipeline, descriptor set, command
buffer, recording commands, submitting, and waiting.

=== How CUDA Hides Complexity ===

When you write `kernel<<<grid, block>>>(args)` in CUDA, here's what
happens internally (and what our simulator does):

    1. Create a Pipeline from the kernel's code
    2. Create a DescriptorSet and bind the argument buffers
    3. Create a CommandBuffer
    4. Record: bind_pipeline, bind_descriptor_set, dispatch
    5. Submit the CommandBuffer to the default stream's queue
    6. Wait for completion (synchronous in default stream)

You never see steps 1-6. That's the magic of CUDA — it feels like
calling a function, but underneath it's the full Vulkan-style pipeline.

=== Streams ===

CUDA streams are independent execution queues. The default stream (stream 0)
is synchronous — every operation completes before the next starts. Additional
streams can overlap:

    Stream 0 (default):  [kernel A]──[kernel B]──[kernel C]
    Stream 1:            ──[upload]──[kernel D]──[download]

Operations in the same stream are sequential. Operations in different
streams can overlap. This maps directly to Layer 5's CommandQueue concept.

=== Memory Model ===

CUDA simplifies memory into two main types:

    cudaMalloc():        GPU-only memory (DEVICE_LOCAL in Layer 5)
    cudaMallocManaged(): Unified memory accessible from both CPU and GPU
                         (DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT)

The memcpy() function handles transfers between these memory types.
"""

from __future__ import annotations

from collections import namedtuple
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any

from compute_runtime import (
    BufferUsage,
    CommandQueue,
    DescriptorBinding,
    Fence,
    MemoryType,
)
from compute_runtime import Buffer as RuntimeBuffer

from ._base import BaseVendorSimulator


# =========================================================================
# CUDA-specific types
# =========================================================================


# dim3 — the classic CUDA grid/block dimension type.
#
# In real CUDA, dim3 is a struct with x, y, z fields. When you write
# kernel<<<dim3(4, 1, 1), dim3(64, 1, 1)>>>, you're saying:
#   "Launch 4 blocks of 64 threads each, in 1D."
#
# We use a namedtuple for simplicity — same semantics, Python-idiomatic.
dim3 = namedtuple("dim3", ["x", "y", "z"])


class CUDAMemcpyKind(Enum):
    """Direction of a CUDA memory copy.

    === The Four Copy Directions ===

        HostToDevice:    CPU RAM → GPU VRAM (upload)
        DeviceToHost:    GPU VRAM → CPU RAM (download)
        DeviceToDevice:  GPU VRAM → GPU VRAM (on-device copy)
        HostToHost:      CPU RAM → CPU RAM (plain memcpy)

    In real CUDA, these map to different DMA engine configurations:
    - HostToDevice uses the PCIe DMA engine (CPU→GPU direction)
    - DeviceToHost uses the PCIe DMA engine (GPU→CPU direction)
    - DeviceToDevice uses the internal GPU copy engine
    - HostToHost uses plain CPU memcpy (no GPU involvement)
    """

    HostToDevice = auto()
    DeviceToHost = auto()
    DeviceToDevice = auto()
    HostToHost = auto()


@dataclass
class CUDADeviceProperties:
    """Properties of a CUDA device, similar to cudaDeviceProp.

    In real CUDA, you query these with cudaGetDeviceProperties(). They
    tell you what the GPU can do — how much memory, how many threads,
    what compute capability.

    Fields:
        name:                  GPU name ("NVIDIA H100", etc.)
        total_global_mem:      Total VRAM in bytes
        shared_mem_per_block:  Shared memory per thread block (48KB typical)
        max_threads_per_block: Maximum threads in one block (1024 typical)
        max_grid_size:         Maximum grid dimensions per axis
        warp_size:             Threads per warp (always 32 on NVIDIA)
        compute_capability:    (major, minor) version tuple
    """

    name: str = ""
    total_global_mem: int = 0
    shared_mem_per_block: int = 49152  # 48 KB
    max_threads_per_block: int = 1024
    max_grid_size: tuple[int, int, int] = (65535, 65535, 65535)
    warp_size: int = 32
    compute_capability: tuple[int, int] = (8, 0)


@dataclass
class CUDAKernel:
    """A CUDA kernel — compiled GPU code ready to launch.

    In real CUDA, kernels are C++ functions decorated with __global__.
    In our simulator, a kernel wraps a list of GPU instructions from
    the gpu-core package (Layer 9).

    Fields:
        code:  List of GPU instructions to execute.
        name:  Kernel name for debugging/profiling.
    """

    code: list[Any]
    name: str = "unnamed_kernel"


@dataclass
class CUDADevicePtr:
    """A CUDA device pointer — a handle to GPU memory.

    In real CUDA, cudaMalloc() returns a void* pointer to device memory.
    You can't dereference it on the CPU — it's only valid on the GPU.

    In our simulator, CUDADevicePtr wraps a Layer 5 Buffer object and
    exposes its device_address and size.

    Fields:
        _buffer:        The underlying Layer 5 Buffer.
        device_address: The "pointer" value (address on device).
        size:           Size of the allocation in bytes.
    """

    _buffer: RuntimeBuffer
    device_address: int = 0
    size: int = 0


class CUDAStream:
    """A CUDA stream — an independent execution queue.

    === What is a Stream? ===

    A stream is a sequence of GPU operations that execute in order.
    Operations in the same stream are guaranteed to execute sequentially.
    Operations in different streams MAY execute concurrently.

    The default stream (stream 0) has special semantics — it synchronizes
    with all other streams. Our simulator models each stream as a separate
    Layer 5 CommandQueue.

    === Internal State ===

    _queue:         The Layer 5 CommandQueue this stream wraps.
    _pending_fence: The fence from the most recent submission, used for
                   stream_synchronize().
    """

    def __init__(self, queue: CommandQueue) -> None:
        self._queue = queue
        self._pending_fence: Fence | None = None


class CUDAEvent:
    """A CUDA event — a timestamp marker in a stream.

    === What is an Event? ===

    Events are used for two things in CUDA:
    1. GPU timing — record event before and after a kernel, measure elapsed
    2. Stream synchronization — one stream can wait for another's event

    In our simulator, an event wraps a Layer 5 Fence with a timestamp.

    _fence:     The underlying Fence for synchronization.
    _timestamp: Device cycle count when this event was recorded.
    _recorded:  Whether record_event() has been called.
    """

    def __init__(self, fence: Fence) -> None:
        self._fence = fence
        self._timestamp: int = 0
        self._recorded: bool = False


# =========================================================================
# CUDARuntime — the main simulator class
# =========================================================================


class CUDARuntime(BaseVendorSimulator):
    """CUDA runtime simulator — wraps Layer 5 with CUDA semantics.

    === Usage ===

    This is the main entry point for CUDA-style programming:

        cuda = CUDARuntime()

        # Allocate, copy, launch, synchronize — just like real CUDA
        d_x = cuda.malloc(1024)
        cuda.memcpy(d_x, host_data, 1024, CUDAMemcpyKind.HostToDevice)
        cuda.launch_kernel(kernel, grid=dim3(4,1,1), block=dim3(64,1,1), args=[d_x])
        cuda.device_synchronize()
        cuda.free(d_x)

    === What This Hides ===

    Each CUDA call translates to multiple Layer 5 operations:
    - malloc() → allocate() with DEVICE_LOCAL memory type
    - memcpy() → map/write/unmap or cmd_copy_buffer
    - launch_kernel() → create pipeline + descriptor set + CB + submit
    - device_synchronize() → wait_idle() on all queues
    """

    def __init__(self) -> None:
        """Initialize CUDA runtime, selecting an NVIDIA GPU."""
        super().__init__(vendor_hint="nvidia")
        self._device_id = 0
        self._streams: list[CUDAStream] = []
        self._events: list[CUDAEvent] = []

    # =================================================================
    # Device management
    # =================================================================

    def set_device(self, device_id: int) -> None:
        """Select which GPU to use (cudaSetDevice).

        In multi-GPU systems, this switches the "current" device. In our
        simulator, we only model one device, so this validates the ID.

        Args:
            device_id: Device index (0-based).

        Raises:
            ValueError: If device_id is out of range.
        """
        if device_id < 0 or device_id >= len(self._physical_devices):
            raise ValueError(
                f"Invalid device ID {device_id}. "
                f"Available: 0-{len(self._physical_devices) - 1}"
            )
        self._device_id = device_id

    def get_device(self) -> int:
        """Get the current device ID (cudaGetDevice).

        Returns:
            The current device index.
        """
        return self._device_id

    def get_device_properties(self) -> CUDADeviceProperties:
        """Query device properties (cudaGetDeviceProperties).

        Returns a CUDADeviceProperties dataclass with information about
        the current device — name, memory size, limits, etc.

        Returns:
            Device properties for the current GPU.
        """
        pd = self._physical_device
        mem_size = sum(h.size for h in pd.memory_properties.heaps)
        return CUDADeviceProperties(
            name=pd.name,
            total_global_mem=mem_size,
            max_threads_per_block=pd.limits.max_workgroup_size[0],
            max_grid_size=pd.limits.max_workgroup_count,
        )

    def device_synchronize(self) -> None:
        """Wait for all GPU work to complete (cudaDeviceSynchronize).

        This is the bluntest synchronization tool — it blocks the CPU
        until every kernel, every copy, every operation on every stream
        has finished. Use sparingly in performance-critical code.

        Maps to: LogicalDevice.wait_idle()
        """
        self._logical_device.wait_idle()

    def device_reset(self) -> None:
        """Reset the device (cudaDeviceReset).

        Destroys all allocations, streams, and state. In real CUDA,
        this is used for cleanup at program exit.

        Maps to: LogicalDevice.reset()
        """
        self._logical_device.reset()
        self._streams.clear()
        self._events.clear()

    # =================================================================
    # Memory management
    # =================================================================

    def malloc(self, size: int) -> CUDADevicePtr:
        """Allocate device memory (cudaMalloc).

        Allocates GPU-only memory (DEVICE_LOCAL). The CPU cannot read or
        write this memory directly — you must use memcpy() to transfer
        data to/from it.

        This is the fastest memory type for GPU computation because it
        uses high-bandwidth VRAM (HBM on datacenter GPUs, GDDR6 on
        consumer GPUs).

        Args:
            size: Number of bytes to allocate.

        Returns:
            A CUDADevicePtr handle to the allocated memory.

        Maps to: memory_manager.allocate(size, DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT)

        Note: We use HOST_VISIBLE | HOST_COHERENT for simulation convenience
        so we can actually read/write data. Real CUDA DEVICE_LOCAL memory
        would not have these flags on discrete GPUs.
        """
        buf = self._memory_manager.allocate(
            size,
            MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST,
        )
        return CUDADevicePtr(
            _buffer=buf,
            device_address=buf.device_address,
            size=size,
        )

    def malloc_managed(self, size: int) -> CUDADevicePtr:
        """Allocate unified/managed memory (cudaMallocManaged).

        Managed memory is accessible from both CPU and GPU. The CUDA
        runtime handles page migration automatically — if the GPU reads
        a page that's on the CPU, it migrates it to GPU memory, and
        vice versa.

        In our simulator, managed memory is simply allocated with all
        visibility flags, since our Layer 5 already supports unified
        memory semantics.

        Args:
            size: Number of bytes to allocate.

        Returns:
            A CUDADevicePtr handle to the unified memory allocation.

        Maps to: allocate(size, DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT)
        """
        buf = self._memory_manager.allocate(
            size,
            MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT,
            usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST,
        )
        return CUDADevicePtr(
            _buffer=buf,
            device_address=buf.device_address,
            size=size,
        )

    def free(self, ptr: CUDADevicePtr) -> None:
        """Free device memory (cudaFree).

        Returns the memory to the allocator. Using the pointer after
        free is undefined behavior (in real CUDA, it's a crash).

        Args:
            ptr: The device pointer to free.

        Raises:
            ValueError: If the pointer has already been freed.
        """
        self._memory_manager.free(ptr._buffer)

    def memcpy(
        self,
        dst: CUDADevicePtr | bytearray,
        src: CUDADevicePtr | bytes | bytearray,
        size: int,
        kind: CUDAMemcpyKind,
    ) -> None:
        """Copy memory between host and device (cudaMemcpy).

        === The Four Copy Directions ===

        HostToDevice:    src is bytes/bytearray (CPU), dst is CUDADevicePtr (GPU)
        DeviceToHost:    src is CUDADevicePtr (GPU), dst is bytearray (CPU)
        DeviceToDevice:  both src and dst are CUDADevicePtr
        HostToHost:      both are bytes/bytearray (no GPU involvement)

        Each direction translates to different Layer 5 operations:
        - HostToDevice: map buffer, write data, unmap
        - DeviceToHost: map buffer, read data, unmap, copy to dst
        - DeviceToDevice: cmd_copy_buffer in a CB
        - HostToHost: plain Python memcpy

        Args:
            dst:  Destination — CUDADevicePtr or bytearray.
            src:  Source — CUDADevicePtr, bytes, or bytearray.
            size: Number of bytes to copy.
            kind: Copy direction.

        Raises:
            TypeError: If src/dst types don't match the specified kind.
        """
        if kind == CUDAMemcpyKind.HostToDevice:
            # CPU → GPU: map the device buffer, write host data, unmap
            if not isinstance(dst, CUDADevicePtr):
                raise TypeError("dst must be CUDADevicePtr for HostToDevice")
            if not isinstance(src, (bytes, bytearray)):
                raise TypeError("src must be bytes for HostToDevice")
            mapped = self._memory_manager.map(dst._buffer)
            mapped.write(0, bytes(src[:size]))
            self._memory_manager.unmap(dst._buffer)

        elif kind == CUDAMemcpyKind.DeviceToHost:
            # GPU → CPU: map the device buffer, read data, unmap
            if not isinstance(src, CUDADevicePtr):
                raise TypeError("src must be CUDADevicePtr for DeviceToHost")
            if not isinstance(dst, bytearray):
                raise TypeError("dst must be bytearray for DeviceToHost")
            # Sync from device first so we read latest GPU-written data
            self._memory_manager.invalidate(src._buffer)
            mapped = self._memory_manager.map(src._buffer)
            data = mapped.read(0, size)
            self._memory_manager.unmap(src._buffer)
            dst[:size] = data

        elif kind == CUDAMemcpyKind.DeviceToDevice:
            # GPU → GPU: use a command buffer with cmd_copy_buffer
            if not isinstance(dst, CUDADevicePtr):
                raise TypeError("dst must be CUDADevicePtr for DeviceToDevice")
            if not isinstance(src, CUDADevicePtr):
                raise TypeError("src must be CUDADevicePtr for DeviceToDevice")

            def record_copy(cb: Any) -> None:
                cb.cmd_copy_buffer(src._buffer, dst._buffer, size)

            self._create_and_submit_cb(record_copy)

        elif kind == CUDAMemcpyKind.HostToHost:
            # CPU → CPU: plain memory copy, no GPU involvement
            if not isinstance(dst, bytearray):
                raise TypeError("dst must be bytearray for HostToHost")
            if not isinstance(src, (bytes, bytearray)):
                raise TypeError("src must be bytes for HostToHost")
            dst[:size] = src[:size]

    def memset(self, ptr: CUDADevicePtr, value: int, size: int) -> None:
        """Set device memory to a value (cudaMemset).

        Fills the first `size` bytes of device memory with the byte
        value `value`. Commonly used to zero-initialize buffers:

            cuda.memset(d_output, 0, 1024)  # Zero 1024 bytes

        Args:
            ptr:   Device pointer to fill.
            value: Byte value (0-255).
            size:  Number of bytes to fill.
        """
        def record_fill(cb: Any) -> None:
            cb.cmd_fill_buffer(ptr._buffer, value, 0, size)

        self._create_and_submit_cb(record_fill)

    # =================================================================
    # Kernel launch — the heart of CUDA
    # =================================================================

    def launch_kernel(
        self,
        kernel: CUDAKernel,
        grid: dim3,
        block: dim3,
        args: list[CUDADevicePtr] | None = None,
        shared_mem: int = 0,
        stream: CUDAStream | None = None,
    ) -> None:
        """Launch a CUDA kernel (the <<<grid, block>>> operator).

        === What Happens Internally ===

        This single call hides the entire Vulkan-style pipeline:

        1. Create a ShaderModule from the kernel's code, with the
           block dimensions as the local workgroup size.
        2. Create a DescriptorSetLayout and PipelineLayout.
        3. Create a Pipeline binding the shader to the layout.
        4. Create a DescriptorSet and bind the argument buffers.
        5. Create a CommandBuffer.
        6. Record: bind_pipeline → bind_descriptor_set → dispatch.
        7. Submit to the queue (default or specified stream).
        8. Wait for completion.

        In real CUDA, the driver caches pipelines and descriptor sets
        to avoid recreating them every launch. Our simulator creates
        fresh objects each time for simplicity.

        Args:
            kernel:     The CUDAKernel containing GPU instructions.
            grid:       Grid dimensions (number of thread blocks).
            block:      Block dimensions (threads per block).
            args:       List of CUDADevicePtr arguments to the kernel.
            shared_mem: Dynamic shared memory per block (bytes). (Unused
                       in our simulation, but part of the real API.)
            stream:     Optional CUDA stream. If None, uses default.
        """
        device = self._logical_device
        arg_list = args or []

        # Step 1: Create shader module with the kernel's code
        shader = device.create_shader_module(
            code=kernel.code,
            local_size=(block.x, block.y, block.z),
        )

        # Step 2: Create descriptor set layout with one binding per argument
        bindings = [
            DescriptorBinding(binding=i, type="storage")
            for i in range(len(arg_list))
        ]
        ds_layout = device.create_descriptor_set_layout(bindings)
        pl_layout = device.create_pipeline_layout([ds_layout])

        # Step 3: Create the compute pipeline
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        # Step 4: Create and populate descriptor set
        ds = device.create_descriptor_set(ds_layout)
        for i, arg in enumerate(arg_list):
            ds.write(i, arg._buffer)

        # Step 5-8: Record and submit via helper
        def record_dispatch(cb: Any) -> None:
            cb.cmd_bind_pipeline(pipeline)
            cb.cmd_bind_descriptor_set(ds)
            cb.cmd_dispatch(grid.x, grid.y, grid.z)

        queue = stream._queue if stream else None
        self._create_and_submit_cb(record_dispatch, queue=queue)

    # =================================================================
    # Streams
    # =================================================================

    def create_stream(self) -> CUDAStream:
        """Create a new CUDA stream (cudaStreamCreate).

        A stream is an independent execution queue. Operations enqueued
        to different streams can overlap (execute concurrently on the GPU).

        In Layer 5 terms, each stream maps to an additional CommandQueue.
        We create a fresh queue request on the logical device.

        Returns:
            A new CUDAStream.
        """
        # In our simulator, we reuse the compute queue since multiple
        # queues from the same family are functionally equivalent in
        # our synchronous simulation.
        stream = CUDAStream(self._compute_queue)
        self._streams.append(stream)
        return stream

    def destroy_stream(self, stream: CUDAStream) -> None:
        """Destroy a CUDA stream (cudaStreamDestroy).

        The stream must have no pending work. After destruction, the
        stream handle is invalid.

        Args:
            stream: The stream to destroy.

        Raises:
            ValueError: If the stream is not found.
        """
        if stream not in self._streams:
            raise ValueError("Stream not found or already destroyed")
        self._streams.remove(stream)

    def stream_synchronize(self, stream: CUDAStream) -> None:
        """Wait for all operations in a stream (cudaStreamSynchronize).

        Blocks the CPU until all previously enqueued operations in this
        stream have completed.

        Args:
            stream: The stream to synchronize.
        """
        if stream._pending_fence is not None:
            stream._pending_fence.wait()

    # =================================================================
    # Events (for GPU timing)
    # =================================================================

    def create_event(self) -> CUDAEvent:
        """Create a CUDA event (cudaEventCreate).

        Events are markers that can be inserted into a stream. They're
        used for:
        - GPU timing (record start, record end, measure elapsed)
        - Inter-stream synchronization

        Returns:
            A new CUDAEvent.
        """
        fence = self._logical_device.create_fence()
        event = CUDAEvent(fence)
        self._events.append(event)
        return event

    def record_event(
        self, event: CUDAEvent, stream: CUDAStream | None = None
    ) -> None:
        """Record an event in a stream (cudaEventRecord).

        Places a timestamp marker at the current position in the stream.
        When the GPU reaches this point, the event is "recorded" and its
        timestamp is captured.

        Args:
            event:  The event to record.
            stream: Which stream to record in. None = default stream.
        """
        queue = stream._queue if stream else self._compute_queue
        event._timestamp = queue.total_cycles
        event._fence.signal()
        event._recorded = True

    def synchronize_event(self, event: CUDAEvent) -> None:
        """Wait for an event to complete (cudaEventSynchronize).

        Blocks the CPU until the event has been recorded (i.e., the GPU
        has reached the point where the event was placed in the stream).

        Args:
            event: The event to wait for.

        Raises:
            RuntimeError: If the event was never recorded.
        """
        if not event._recorded:
            raise RuntimeError("Event was never recorded")
        event._fence.wait()

    def elapsed_time(self, start: CUDAEvent, end: CUDAEvent) -> float:
        """Measure elapsed GPU time between two events (cudaEventElapsedTime).

        Returns the time in milliseconds between when the start event
        and end event were recorded on the GPU.

        Args:
            start: The start event (must have been recorded).
            end:   The end event (must have been recorded).

        Returns:
            Elapsed time in milliseconds (simulated from cycle counts).

        Raises:
            RuntimeError: If either event was not recorded.
        """
        if not start._recorded:
            raise RuntimeError("Start event was never recorded")
        if not end._recorded:
            raise RuntimeError("End event was never recorded")
        # Convert cycle difference to milliseconds (assume 1 GHz clock)
        cycles = end._timestamp - start._timestamp
        return cycles / 1_000_000.0  # 1 GHz → 1 cycle = 1 ns = 0.000001 ms
