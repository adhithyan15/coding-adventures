"""Metal Runtime Simulator — Apple's unified memory GPU programming model.

=== What is Metal? ===

Metal is Apple's GPU API, designed exclusively for Apple hardware (macOS,
iOS, iPadOS, tvOS). Its key innovation is **unified memory** — on Apple
Silicon (M1/M2/M3/M4), the CPU and GPU share the same physical RAM. This
eliminates the host-to-device copies that CUDA and OpenCL require.

=== The Command Encoder Model ===

Metal uses a distinctive pattern for recording GPU commands:

    1. Get a command buffer from the command queue
    2. Create a **command encoder** (compute, blit, render)
    3. Record commands into the encoder
    4. End the encoder
    5. Commit the command buffer

The encoder adds a layer of scoping that Vulkan doesn't have:

    Vulkan:   cb.begin() → cmd_bind_pipeline() → cmd_dispatch() → cb.end()
    Metal:    cb → encoder = cb.make_compute_command_encoder()
                  encoder.set_compute_pipeline_state(pso)
                  encoder.dispatch_threadgroups(...)
                  encoder.end_encoding()
              cb.commit()

This scoping makes it clear what type of commands are being recorded.
You can't accidentally mix compute and blit commands in one encoder.

=== Unified Memory ===

On Apple Silicon, all memory is both CPU-accessible and GPU-accessible:

    CUDA:   cudaMalloc → device-only, need cudaMemcpy to access from CPU
    Metal:  makeBuffer → unified, buffer.contents() gives CPU access directly

This is modeled using DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT in
Layer 5. The buffer can be mapped at any time, and writes are immediately
visible to both CPU and GPU.

=== No Physical/Logical Split ===

Unlike Vulkan (and our Layer 5), Metal doesn't separate physical and logical
devices. There's just MTLDevice — you get it, you use it. This simplifies
the API but doesn't change the underlying architecture.
"""

from __future__ import annotations

from collections import namedtuple
from dataclasses import dataclass
from enum import Enum, auto
from typing import Any, Callable

from compute_runtime import (
    BufferUsage,
    DescriptorBinding,
    Fence,
    MemoryType,
)
from compute_runtime import Buffer as RuntimeBuffer

from ._base import BaseVendorSimulator


# =========================================================================
# Metal-specific types
# =========================================================================

# MTLSize — grid/threadgroup dimensions in Metal.
#
# Metal uses (width, height, depth) instead of (x, y, z). Same concept,
# different naming — Apple convention for consistency with their graphics API.
MTLSize = namedtuple("MTLSize", ["width", "height", "depth"])


class MTLResourceOptions(Enum):
    """Metal storage mode options for buffers.

    storageModeShared:  CPU + GPU access (default on Apple Silicon).
                       Both sides see the same memory. No copies needed.

    storageModePrivate: GPU-only access. Fastest for GPU-only buffers.
                       CPU cannot read or write. Data must be copied
                       via blit encoder.

    storageModeManaged: CPU + GPU with explicit synchronization (macOS only).
                       Like shared, but you must explicitly call
                       didModifyRange() after CPU writes. Allows the
                       driver to optimize caching.
    """

    storageModeShared = auto()
    storageModePrivate = auto()
    storageModeManaged = auto()


class MTLCommandBufferStatus(Enum):
    """Status of a Metal command buffer in its lifecycle.

    notEnqueued → enqueued → committed → scheduled → completed
                                                  or → error

    notEnqueued: Just created, not yet submitted.
    enqueued:    Added to the command queue.
    committed:   commit() was called, ready for GPU.
    scheduled:   GPU has started processing.
    completed:   GPU finished successfully.
    error:       Something went wrong.
    """

    notEnqueued = auto()
    enqueued = auto()
    committed = auto()
    scheduled = auto()
    completed = auto()
    error = auto()


# =========================================================================
# MTLBuffer — unified memory buffer
# =========================================================================


class MTLBuffer:
    """A Metal buffer — always accessible from both CPU and GPU.

    === Unified Memory in Action ===

    Because Apple Silicon uses unified memory, you can:

        buf = device.make_buffer(1024)
        buf.write_bytes(data)           # CPU writes directly
        # ... GPU computes on buf ...
        result = bytes(buf.contents())  # CPU reads directly

    No staging buffers, no memcpy, no map/unmap ceremony. This is the
    biggest ergonomic advantage of Metal over Vulkan/CUDA on Apple hardware.
    """

    def __init__(
        self,
        buffer: RuntimeBuffer,
        memory_manager: Any,
        length: int,
    ) -> None:
        self._buffer = buffer
        self._mm = memory_manager
        self._length = length

    @property
    def length(self) -> int:
        """Buffer size in bytes."""
        return self._length

    def contents(self) -> bytearray:
        """Get CPU-accessible view of the buffer contents.

        In real Metal, this returns a raw pointer to the shared memory.
        The CPU can read and write it directly, and the GPU sees the
        same bytes (for storageModeShared).

        In our simulator, we invalidate (pull from device), then return
        the buffer's data as a bytearray.

        Returns:
            A mutable bytearray with the buffer's current contents.
        """
        self._mm.invalidate(self._buffer)
        return self._mm._get_buffer_data(self._buffer.buffer_id)

    def write_bytes(self, data: bytes, offset: int = 0) -> None:
        """Write bytes to the buffer from CPU side.

        Convenience method that maps, writes, and unmaps in one call.
        Since Metal uses unified memory, this is equivalent to a direct
        memcpy into the shared address space.

        Args:
            data:   Bytes to write.
            offset: Byte offset into the buffer.
        """
        mapped = self._mm.map(self._buffer)
        mapped.write(offset, data)
        self._mm.unmap(self._buffer)


# =========================================================================
# MTLFunction and MTLLibrary — shader management
# =========================================================================


class MTLFunction:
    """A Metal shader function extracted from a library.

    In real Metal, functions are written in the Metal Shading Language (MSL)
    and compiled into a MTLLibrary. You extract functions by name.

    In our simulator, a function wraps the shader code and name.
    """

    def __init__(self, name: str, code: list[Any] | None = None) -> None:
        self._name = name
        self._code = code

    @property
    def name(self) -> str:
        """Function name."""
        return self._name


class MTLLibrary:
    """A Metal shader library — a collection of compiled functions.

    In real Metal, a library is compiled from MSL source code:

        library = device.makeLibrary(source: metalSource)
        function = library.makeFunction(name: "saxpy")

    In our simulator, the "source" is a label, and functions carry
    optional GPU instruction code.
    """

    def __init__(self, source: str, functions: dict[str, list[Any] | None] | None = None) -> None:
        self._source = source
        self._functions = functions or {}

    def make_function(self, name: str) -> MTLFunction:
        """Extract a function from the library by name.

        Args:
            name: Function name.

        Returns:
            A MTLFunction.
        """
        code = self._functions.get(name)
        return MTLFunction(name=name, code=code)


# =========================================================================
# MTLComputePipelineState — compiled compute pipeline
# =========================================================================


class MTLComputePipelineState:
    """A compiled Metal compute pipeline state.

    In Metal, a pipeline state object (PSO) encapsulates the compiled
    kernel function ready for dispatch. It's similar to Vulkan's Pipeline
    but doesn't require an explicit layout — Metal infers the layout
    from the function's buffer bindings.
    """

    def __init__(self, function: MTLFunction, device: Any) -> None:
        self._function = function
        self._device = device

        # Create Layer 5 pipeline from the function
        shader = device.create_shader_module(code=function._code)
        ds_layout = device.create_descriptor_set_layout([])
        pl_layout = device.create_pipeline_layout([ds_layout])
        self._pipeline = device.create_compute_pipeline(shader, pl_layout)

    @property
    def max_total_threads_per_threadgroup(self) -> int:
        """Maximum threads per threadgroup for this pipeline."""
        return 1024


# =========================================================================
# MTLComputeCommandEncoder — records compute commands
# =========================================================================


class MTLComputeCommandEncoder:
    """A Metal compute command encoder — records compute commands.

    === The Encoder Pattern ===

    Instead of recording commands directly into a command buffer (Vulkan
    style), Metal uses typed encoders that scope commands by type:

        encoder = command_buffer.make_compute_command_encoder()
        encoder.set_compute_pipeline_state(pso)
        encoder.set_buffer(buf_x, offset=0, index=0)
        encoder.set_buffer(buf_y, offset=0, index=1)
        encoder.dispatch_threadgroups(groups, threads_per_group)
        encoder.end_encoding()

    The encoder internally creates descriptor sets and records into the
    underlying Layer 5 command buffer.
    """

    def __init__(self, command_buffer: MTLCommandBuffer) -> None:
        self._command_buffer = command_buffer
        self._pipeline_state: MTLComputePipelineState | None = None
        self._buffers: dict[int, MTLBuffer] = {}
        self._push_data: dict[int, bytes] = {}
        self._ended = False

    def set_compute_pipeline_state(self, pso: MTLComputePipelineState) -> None:
        """Set which compute pipeline to use for dispatches.

        Args:
            pso: The pipeline state object.
        """
        self._pipeline_state = pso

    def set_buffer(self, buffer: MTLBuffer, offset: int, index: int) -> None:
        """Bind a buffer to an argument index.

        In Metal, buffers are bound by index, not by descriptor sets.
        This is simpler than Vulkan's descriptor model:

            encoder.set_buffer(buf_x, offset=0, index=0)  # arg 0
            encoder.set_buffer(buf_y, offset=0, index=1)  # arg 1

        Args:
            buffer: The buffer to bind.
            offset: Byte offset into the buffer.
            index:  Argument index (0, 1, 2, ...).
        """
        self._buffers[index] = buffer

    def set_bytes(self, data: bytes, index: int) -> None:
        """Set inline bytes as a kernel argument (push constants).

        For small data like scalar alpha values, you can pass bytes
        directly instead of creating a buffer:

            encoder.set_bytes(struct.pack('f', 2.0), index=2)

        Args:
            data:  Raw bytes.
            index: Argument index.
        """
        self._push_data[index] = data

    def dispatch_threadgroups(
        self,
        threadgroups_per_grid: MTLSize,
        threads_per_threadgroup: MTLSize,
    ) -> None:
        """Dispatch a compute kernel with explicit threadgroup count.

        Args:
            threadgroups_per_grid:   Number of threadgroups (grid dimensions).
            threads_per_threadgroup: Threads per threadgroup (block dimensions).
        """
        if self._pipeline_state is None:
            raise RuntimeError("No compute pipeline state set")

        cb = self._command_buffer._cb
        device = self._command_buffer._device

        # Create a fresh pipeline with the correct local size
        pso = self._pipeline_state
        shader = device.create_shader_module(
            code=pso._function._code,
            local_size=(
                threads_per_threadgroup.width,
                threads_per_threadgroup.height,
                threads_per_threadgroup.depth,
            ),
        )

        # Build descriptor set from bound buffers
        bindings = [
            DescriptorBinding(binding=i, type="storage")
            for i in sorted(self._buffers.keys())
        ]
        ds_layout = device.create_descriptor_set_layout(bindings)
        pl_layout = device.create_pipeline_layout([ds_layout])
        pipeline = device.create_compute_pipeline(shader, pl_layout)

        ds = device.create_descriptor_set(ds_layout)
        for i in sorted(self._buffers.keys()):
            ds.write(i, self._buffers[i]._buffer)

        # Record into the command buffer
        cb.cmd_bind_pipeline(pipeline)
        cb.cmd_bind_descriptor_set(ds)
        cb.cmd_dispatch(
            threadgroups_per_grid.width,
            threadgroups_per_grid.height,
            threadgroups_per_grid.depth,
        )

    def dispatch_threads(
        self,
        threads_per_grid: MTLSize,
        threads_per_threadgroup: MTLSize,
    ) -> None:
        """Dispatch with total thread count (Metal calculates grid).

        A convenience that calculates threadgroup count from total threads.

        Args:
            threads_per_grid:        Total threads in each dimension.
            threads_per_threadgroup: Threads per threadgroup.
        """
        groups = MTLSize(
            width=max(
                1,
                (threads_per_grid.width + threads_per_threadgroup.width - 1)
                // threads_per_threadgroup.width,
            ),
            height=max(
                1,
                (threads_per_grid.height + threads_per_threadgroup.height - 1)
                // threads_per_threadgroup.height,
            ),
            depth=max(
                1,
                (threads_per_grid.depth + threads_per_threadgroup.depth - 1)
                // threads_per_threadgroup.depth,
            ),
        )
        self.dispatch_threadgroups(groups, threads_per_threadgroup)

    def end_encoding(self) -> None:
        """End recording into this encoder.

        After this call, no more commands can be recorded into this
        encoder. The command buffer can then commit more encoders
        or be committed for execution.
        """
        self._ended = True


# =========================================================================
# MTLBlitCommandEncoder — records data transfer commands
# =========================================================================


class MTLBlitCommandEncoder:
    """A Metal blit command encoder — records copy/fill operations.

    "Blit" stands for "block image transfer" — a term from early computer
    graphics for bulk memory copies. In Metal, blit encoders handle:
    - Buffer-to-buffer copies
    - Buffer fills
    - Texture copies (not implemented in our compute-only simulator)
    """

    def __init__(self, command_buffer: MTLCommandBuffer) -> None:
        self._command_buffer = command_buffer
        self._ended = False

    def copy_from_buffer(
        self,
        src: MTLBuffer,
        src_offset: int,
        to_buffer: MTLBuffer,
        dst_offset: int,
        size: int,
    ) -> None:
        """Copy data between buffers.

        Args:
            src:        Source buffer.
            src_offset: Byte offset in source.
            to_buffer:  Destination buffer.
            dst_offset: Byte offset in destination.
            size:       Bytes to copy.
        """
        cb = self._command_buffer._cb
        cb.cmd_copy_buffer(
            src._buffer, to_buffer._buffer, size, src_offset, dst_offset
        )

    def fill_buffer(
        self, buffer: MTLBuffer, fill_range: range, value: int
    ) -> None:
        """Fill a buffer region with a byte value.

        Args:
            buffer:     Buffer to fill.
            fill_range: Range of bytes to fill.
            value:      Byte value (0-255).
        """
        cb = self._command_buffer._cb
        cb.cmd_fill_buffer(
            buffer._buffer, value, fill_range.start,
            fill_range.stop - fill_range.start,
        )

    def end_encoding(self) -> None:
        """End recording into this blit encoder."""
        self._ended = True


# =========================================================================
# MTLCommandBuffer — wraps Layer 5 CommandBuffer with encoder model
# =========================================================================


class MTLCommandBuffer:
    """A Metal command buffer — records and submits GPU work.

    Metal command buffers use the encoder model:

        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        # ... record commands ...
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

    Internally, this wraps a Layer 5 CommandBuffer. The commit() method
    submits it to the queue, and wait_until_completed() waits via a fence.
    """

    def __init__(self, queue: MTLCommandQueue) -> None:
        self._queue = queue
        self._device = queue._device._logical_device
        self._cb = self._device.create_command_buffer()
        self._cb.begin()
        self._fence = self._device.create_fence()
        self._status = MTLCommandBufferStatus.notEnqueued
        self._completed_handlers: list[Callable[[], None]] = []

    @property
    def status(self) -> MTLCommandBufferStatus:
        """Current command buffer status."""
        return self._status

    def make_compute_command_encoder(self) -> MTLComputeCommandEncoder:
        """Create a compute command encoder for this command buffer.

        Returns:
            A new MTLComputeCommandEncoder ready for recording.
        """
        return MTLComputeCommandEncoder(self)

    def make_blit_command_encoder(self) -> MTLBlitCommandEncoder:
        """Create a blit (copy/fill) command encoder.

        Returns:
            A new MTLBlitCommandEncoder ready for recording.
        """
        return MTLBlitCommandEncoder(self)

    def commit(self) -> None:
        """Submit this command buffer for execution (commit).

        This ends recording and submits the command buffer to the
        queue for GPU execution.
        """
        self._cb.end()
        self._status = MTLCommandBufferStatus.committed
        self._queue._queue.submit([self._cb], fence=self._fence)
        self._status = MTLCommandBufferStatus.completed
        for handler in self._completed_handlers:
            handler()

    def wait_until_completed(self) -> None:
        """Block until the command buffer finishes execution.

        In real Metal, this blocks the CPU thread. In our synchronous
        simulator, commit() already runs everything to completion, so
        this just checks the fence.
        """
        self._fence.wait()

    def add_completed_handler(self, handler: Callable[[], None]) -> None:
        """Register a callback to be called when execution completes.

        Args:
            handler: Callable with no arguments.
        """
        self._completed_handlers.append(handler)


# =========================================================================
# MTLCommandQueue — creates command buffers
# =========================================================================


class MTLCommandQueue:
    """A Metal command queue — creates command buffers for submission.

    In Metal, the command queue is simpler than in Vulkan — you just
    create command buffers from it. The queue handles scheduling internally.
    """

    def __init__(self, device: MTLDevice) -> None:
        self._device = device
        self._queue = device._compute_queue

    def make_command_buffer(self) -> MTLCommandBuffer:
        """Create a new command buffer for this queue.

        Returns:
            A new MTLCommandBuffer ready for encoding.
        """
        return MTLCommandBuffer(self)


# =========================================================================
# MTLDevice — the main Metal device object
# =========================================================================


class MTLDevice(BaseVendorSimulator):
    """A Metal device — the main entry point for Metal programming.

    === Apple's Simplified Model ===

    In Vulkan, you have PhysicalDevice (read-only) and LogicalDevice (usable).
    In Metal, there's just MTLDevice — it's both. You get properties AND
    you create resources from it.

    Metal always uses unified memory. All buffers are CPU-accessible
    (storageModeShared by default), so there's no need for staging
    buffers or explicit host-device transfers.

    === Usage ===

        device = MTLDevice()
        queue = device.make_command_queue()

        buf = device.make_buffer(1024)
        buf.write_bytes(data)  # Direct CPU write!

        library = device.make_library(source="my_shader")
        function = library.make_function("compute_fn")
        pso = device.make_compute_pipeline_state(function)

        cb = queue.make_command_buffer()
        encoder = cb.make_compute_command_encoder()
        encoder.set_compute_pipeline_state(pso)
        encoder.set_buffer(buf, offset=0, index=0)
        encoder.dispatch_threadgroups(MTLSize(4,1,1), MTLSize(64,1,1))
        encoder.end_encoding()
        cb.commit()
        cb.wait_until_completed()

        result = bytes(buf.contents())  # Direct CPU read!
    """

    def __init__(self) -> None:
        """Create a Metal device, preferring Apple hardware."""
        super().__init__(vendor_hint="apple")

    @property
    def name(self) -> str:
        """Device name."""
        return self._physical_device.name

    def make_command_queue(self) -> MTLCommandQueue:
        """Create a command queue for this device.

        Returns:
            A new MTLCommandQueue.
        """
        return MTLCommandQueue(self)

    def make_buffer(
        self,
        length: int,
        options: MTLResourceOptions = MTLResourceOptions.storageModeShared,
    ) -> MTLBuffer:
        """Allocate a buffer on the device.

        All Metal buffers use unified memory by default (storageModeShared).
        This means both CPU and GPU can access them without copies.

        Args:
            length:  Buffer size in bytes.
            options: Storage mode. Default is shared (unified memory).

        Returns:
            A new MTLBuffer.
        """
        # Unified memory: all flags for CPU + GPU access
        mem_type = (
            MemoryType.DEVICE_LOCAL
            | MemoryType.HOST_VISIBLE
            | MemoryType.HOST_COHERENT
        )
        usage = BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST

        buf = self._memory_manager.allocate(length, mem_type, usage=usage)
        return MTLBuffer(buf, self._memory_manager, length)

    def make_library(self, source: str) -> MTLLibrary:
        """Create a shader library from source code.

        In real Metal, this compiles MSL (Metal Shading Language) source.
        In our simulator, the source is a label.

        Args:
            source: Shader source code (label in simulator).

        Returns:
            A MTLLibrary containing compiled functions.
        """
        return MTLLibrary(source)

    def make_compute_pipeline_state(
        self, function: MTLFunction
    ) -> MTLComputePipelineState:
        """Create a compute pipeline state from a shader function.

        Args:
            function: The compiled shader function.

        Returns:
            A MTLComputePipelineState ready for use in encoders.
        """
        return MTLComputePipelineState(function, self._logical_device)
