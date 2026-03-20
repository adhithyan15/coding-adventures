"""WebGPU Runtime Simulator — safe, browser-first GPU programming.

=== What is WebGPU? ===

WebGPU is the modern web GPU API, designed to run safely in browsers.
It sits on top of Vulkan (Linux/Windows/Android), Metal (macOS/iOS),
or D3D12 (Windows), providing a safe, portable abstraction.

=== Key Simplifications Over Vulkan ===

1. **Single queue** — `device.queue` is all you get. No queue families,
   no multiple queues. The runtime handles parallelism internally.

2. **Automatic barriers** — no manual pipeline barriers. The runtime
   tracks buffer usage and inserts barriers as needed.

3. **No memory types** — just usage flags. The runtime picks the
   optimal memory type for you.

4. **Always validated** — every operation is checked. You can't cause
   undefined behavior (unlike Vulkan where invalid usage = crash).

5. **Immutable command buffers** — once encoder.finish() is called,
   the GPUCommandBuffer cannot be modified or re-recorded.

=== The WebGPU Object Hierarchy ===

    GPU (navigator.gpu in browsers)
    └── GPUAdapter (represents a physical device)
        └── GPUDevice (the usable handle)
            ├── device.queue (GPUQueue — single queue!)
            ├── createBuffer() → GPUBuffer
            ├── createShaderModule() → GPUShaderModule
            ├── createComputePipeline() → GPUComputePipeline
            ├── createBindGroup() → GPUBindGroup
            └── createCommandEncoder() → GPUCommandEncoder
                └── beginComputePass() → GPUComputePassEncoder
                    ├── setPipeline()
                    ├── setBindGroup()
                    ├── dispatchWorkgroups()
                    └── end()
                └── finish() → GPUCommandBuffer (frozen!)

=== Bind Groups (WebGPU's Descriptor Sets) ===

WebGPU uses "bind groups" instead of "descriptor sets" — same concept,
friendlier name. A bind group maps binding indices to buffers:

    bind_group = device.createBindGroup({
        layout: pipeline.getBindGroupLayout(0),
        entries: [
            { binding: 0, resource: { buffer: buf_x } },
            { binding: 1, resource: { buffer: buf_y } },
        ]
    })
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Flag, auto
from typing import Any

from compute_runtime import (
    BufferUsage,
    DescriptorBinding,
    DescriptorSet,
    DescriptorSetLayout,
    Fence,
    MemoryType,
    Pipeline,
    PipelineLayout,
    ShaderModule,
)
from compute_runtime import Buffer as RuntimeBuffer

from ._base import BaseVendorSimulator


# =========================================================================
# WebGPU flags
# =========================================================================


class GPUBufferUsage(Flag):
    """WebGPU buffer usage flags.

    MAP_READ:   Buffer can be mapped for reading by the CPU.
    MAP_WRITE:  Buffer can be mapped for writing by the CPU.
    COPY_SRC:   Buffer can be the source of a copy operation.
    COPY_DST:   Buffer can be the destination of a copy operation.
    STORAGE:    Buffer can be used as a storage buffer in shaders.
    UNIFORM:    Buffer can be used as a uniform buffer in shaders.
    """

    MAP_READ = auto()
    MAP_WRITE = auto()
    COPY_SRC = auto()
    COPY_DST = auto()
    STORAGE = auto()
    UNIFORM = auto()


class GPUMapMode(Flag):
    """WebGPU buffer map modes.

    READ:  Map for CPU reading.
    WRITE: Map for CPU writing.
    """

    READ = auto()
    WRITE = auto()


# =========================================================================
# WebGPU descriptor types
# =========================================================================


@dataclass
class GPUBufferDescriptor:
    """Parameters for creating a GPUBuffer.

    Fields:
        size:               Buffer size in bytes.
        usage:              Usage flags (GPUBufferUsage).
        mapped_at_creation: If True, buffer starts mapped for CPU writing.
    """

    size: int = 0
    usage: GPUBufferUsage = GPUBufferUsage.STORAGE
    mapped_at_creation: bool = False


@dataclass
class GPUShaderModuleDescriptor:
    """Parameters for creating a GPUShaderModule.

    Fields:
        code: WGSL shader source (or GPU instructions in our simulator).
    """

    code: Any = None


@dataclass
class GPUProgrammableStage:
    """Shader stage specification for pipeline creation.

    Fields:
        module:      The shader module.
        entry_point: Function name in the shader.
    """

    module: GPUShaderModule | None = None
    entry_point: str = "main"


@dataclass
class GPUComputePipelineDescriptor:
    """Parameters for creating a compute pipeline.

    Fields:
        layout:  Pipeline layout ("auto" for automatic inference).
        compute: Programmable stage configuration.
    """

    layout: str | GPUPipelineLayout = "auto"
    compute: GPUProgrammableStage | None = None


@dataclass
class GPUBufferBindingLayout:
    """Layout for a buffer binding in a bind group.

    Fields:
        type: "storage", "uniform", or "read-only-storage".
    """

    type: str = "storage"


@dataclass
class GPUBindGroupLayoutEntry:
    """One entry in a bind group layout.

    Fields:
        binding:    Binding index.
        visibility: Which shader stages can see this binding.
        buffer:     Buffer binding layout details.
    """

    binding: int = 0
    visibility: int = 0x04  # COMPUTE stage
    buffer: GPUBufferBindingLayout = field(
        default_factory=GPUBufferBindingLayout
    )


@dataclass
class GPUBindGroupLayoutDescriptor:
    """Parameters for creating a bind group layout.

    Fields:
        entries: List of binding entries.
    """

    entries: list[GPUBindGroupLayoutEntry] = field(default_factory=list)


@dataclass
class GPUBindGroupEntry:
    """One entry in a bind group (binding index → buffer).

    Fields:
        binding:  Binding index.
        resource: The buffer to bind.
    """

    binding: int = 0
    resource: GPUBuffer | None = None


@dataclass
class GPUBindGroupDescriptor:
    """Parameters for creating a bind group.

    Fields:
        layout:  The bind group layout.
        entries: Binding entries (buffer assignments).
    """

    layout: GPUBindGroupLayout | None = None
    entries: list[GPUBindGroupEntry] = field(default_factory=list)


@dataclass
class GPUPipelineLayoutDescriptor:
    """Parameters for creating a pipeline layout.

    Fields:
        bind_group_layouts: List of bind group layouts.
    """

    bind_group_layouts: list[GPUBindGroupLayout] = field(default_factory=list)


@dataclass
class GPURequestAdapterOptions:
    """Options for adapter selection.

    Fields:
        power_preference: "low-power" or "high-performance".
    """

    power_preference: str = "high-performance"


@dataclass
class GPUDeviceDescriptor:
    """Parameters for requesting a device.

    Fields:
        required_features: Features the device must support.
    """

    required_features: list[str] = field(default_factory=list)


@dataclass
class GPUAdapterLimits:
    """Hardware limits reported by an adapter.

    Fields:
        max_buffer_size: Maximum buffer allocation size.
        max_compute_workgroup_size_x: Max workgroup X dimension.
    """

    max_buffer_size: int = 2 * 1024 * 1024 * 1024
    max_compute_workgroup_size_x: int = 1024


@dataclass
class GPUDeviceLimits:
    """Device limits (same as adapter limits in our simulator)."""

    max_buffer_size: int = 2 * 1024 * 1024 * 1024
    max_compute_workgroup_size_x: int = 1024


@dataclass
class GPUComputePassDescriptor:
    """Parameters for beginning a compute pass (currently empty)."""

    label: str = ""


@dataclass
class GPUCommandEncoderDescriptor:
    """Parameters for creating a command encoder (currently empty)."""

    label: str = ""


# =========================================================================
# WebGPU wrapper objects
# =========================================================================


class GPUBuffer:
    """A WebGPU buffer — memory on the device.

    WebGPU buffers don't expose memory types. You specify usage flags,
    and the runtime picks the optimal memory type. Mapping is async
    (simulated as sync in our simulator).
    """

    def __init__(
        self,
        buffer: RuntimeBuffer,
        memory_manager: Any,
        size: int,
        usage: GPUBufferUsage,
    ) -> None:
        self._buffer = buffer
        self._mm = memory_manager
        self._size = size
        self._usage = usage
        self._mapped = False
        self._mapped_data: bytearray | None = None
        self._destroyed = False

    @property
    def size(self) -> int:
        """Buffer size in bytes."""
        return self._size

    @property
    def usage(self) -> GPUBufferUsage:
        """Buffer usage flags."""
        return self._usage

    def map_async(
        self,
        mode: GPUMapMode,
        offset: int = 0,
        size: int | None = None,
    ) -> None:
        """Map the buffer for CPU access (simulated as synchronous).

        In real WebGPU, this is async and returns a Promise. In our
        simulator, it completes immediately.

        Args:
            mode:   READ or WRITE.
            offset: Byte offset to map from.
            size:   Bytes to map. None = entire buffer.
        """
        if self._destroyed:
            raise RuntimeError("Cannot map a destroyed buffer")
        actual_size = size if size is not None else self._size
        self._mm.invalidate(self._buffer)
        data = self._mm._get_buffer_data(self._buffer.buffer_id)
        self._mapped_data = bytearray(data[offset : offset + actual_size])
        self._mapped = True

    def get_mapped_range(
        self, offset: int = 0, size: int | None = None
    ) -> bytearray:
        """Get a view of the mapped buffer data.

        Must be called after map_async() completes.

        Args:
            offset: Byte offset within the mapped range.
            size:   Bytes to return. None = entire mapped range.

        Returns:
            A mutable bytearray of the buffer contents.

        Raises:
            RuntimeError: If buffer is not mapped.
        """
        if not self._mapped or self._mapped_data is None:
            raise RuntimeError("Buffer is not mapped. Call map_async() first.")
        actual_size = size if size is not None else len(self._mapped_data)
        return self._mapped_data[offset : offset + actual_size]

    def unmap(self) -> None:
        """Unmap the buffer, making it usable by the GPU again.

        If the buffer was mapped for writing, the data is synced back
        to the device.
        """
        if not self._mapped:
            raise RuntimeError("Buffer is not mapped")
        if self._mapped_data is not None:
            # Write mapped data back to the buffer
            mapped = self._mm.map(self._buffer)
            mapped.write(0, bytes(self._mapped_data))
            self._mm.unmap(self._buffer)
        self._mapped = False
        self._mapped_data = None

    def destroy(self) -> None:
        """Destroy this buffer, releasing its memory."""
        if not self._destroyed:
            self._mm.free(self._buffer)
            self._destroyed = True


class GPUShaderModule:
    """A WebGPU shader module — wraps Layer 5 ShaderModule."""

    def __init__(self, shader: ShaderModule) -> None:
        self._shader = shader


class GPUBindGroupLayout:
    """A WebGPU bind group layout — wraps Layer 5 DescriptorSetLayout."""

    def __init__(self, layout: DescriptorSetLayout) -> None:
        self._layout = layout


class GPUPipelineLayout:
    """A WebGPU pipeline layout — wraps Layer 5 PipelineLayout."""

    def __init__(self, layout: PipelineLayout) -> None:
        self._layout = layout


class GPUComputePipeline:
    """A WebGPU compute pipeline — wraps Layer 5 Pipeline.

    Supports get_bind_group_layout() for automatic layout inference.
    """

    def __init__(
        self,
        pipeline: Pipeline,
        bind_group_layouts: list[GPUBindGroupLayout],
    ) -> None:
        self._pipeline = pipeline
        self._bind_group_layouts = bind_group_layouts

    def get_bind_group_layout(self, index: int) -> GPUBindGroupLayout:
        """Get the bind group layout at a given index.

        When a pipeline is created with layout="auto", the runtime
        infers bind group layouts from the shader. This method lets
        you query those inferred layouts.

        Args:
            index: Bind group index.

        Returns:
            The GPUBindGroupLayout at that index.
        """
        if index < len(self._bind_group_layouts):
            return self._bind_group_layouts[index]
        raise IndexError(f"Bind group layout index {index} out of range")


class GPUBindGroup:
    """A WebGPU bind group — wraps Layer 5 DescriptorSet."""

    def __init__(self, ds: DescriptorSet) -> None:
        self._ds = ds


class GPUCommandBuffer:
    """A frozen WebGPU command buffer — immutable after finish().

    Once an encoder's finish() is called, the resulting GPUCommandBuffer
    cannot be modified. It can only be submitted to a queue.
    """

    def __init__(self, cb: Any) -> None:
        self._cb = cb


# =========================================================================
# GPUComputePassEncoder — records compute commands
# =========================================================================


class GPUComputePassEncoder:
    """A WebGPU compute pass encoder — records compute commands.

    Similar to Metal's compute command encoder but with WebGPU naming.
    A compute pass is a scope for compute operations.
    """

    def __init__(self, encoder: GPUCommandEncoder) -> None:
        self._encoder = encoder
        self._pipeline: GPUComputePipeline | None = None
        self._bind_groups: dict[int, GPUBindGroup] = {}

    def set_pipeline(self, pipeline: GPUComputePipeline) -> None:
        """Set the compute pipeline for this pass.

        Args:
            pipeline: The compute pipeline to use.
        """
        self._pipeline = pipeline

    def set_bind_group(self, index: int, bind_group: GPUBindGroup) -> None:
        """Set a bind group at the given index.

        Args:
            index:      Bind group index.
            bind_group: The bind group to set.
        """
        self._bind_groups[index] = bind_group

    def dispatch_workgroups(
        self, x: int, y: int = 1, z: int = 1
    ) -> None:
        """Dispatch compute workgroups.

        Args:
            x: Workgroups in X dimension.
            y: Workgroups in Y dimension.
            z: Workgroups in Z dimension.
        """
        if self._pipeline is None:
            raise RuntimeError("No pipeline set")

        cb = self._encoder._cb
        cb.cmd_bind_pipeline(self._pipeline._pipeline)
        for _idx, bg in sorted(self._bind_groups.items()):
            cb.cmd_bind_descriptor_set(bg._ds)
        cb.cmd_dispatch(x, y, z)

    def end(self) -> None:
        """End this compute pass."""
        pass


# =========================================================================
# GPUCommandEncoder — records commands into a command buffer
# =========================================================================


class GPUCommandEncoder:
    """A WebGPU command encoder — builds a GPUCommandBuffer.

    The encoder is the recording interface. Once finish() is called,
    it produces a frozen GPUCommandBuffer that can be submitted.
    """

    def __init__(self, device: GPUDevice) -> None:
        self._device = device
        self._cb = device._logical_device.create_command_buffer()
        self._cb.begin()

    def begin_compute_pass(
        self, descriptor: GPUComputePassDescriptor | None = None
    ) -> GPUComputePassEncoder:
        """Begin a compute pass.

        Returns:
            A new GPUComputePassEncoder.
        """
        return GPUComputePassEncoder(self)

    def copy_buffer_to_buffer(
        self,
        source: GPUBuffer,
        source_offset: int,
        destination: GPUBuffer,
        destination_offset: int,
        size: int,
    ) -> None:
        """Copy data between buffers.

        Args:
            source:             Source buffer.
            source_offset:      Byte offset in source.
            destination:        Destination buffer.
            destination_offset: Byte offset in destination.
            size:               Bytes to copy.
        """
        self._cb.cmd_copy_buffer(
            source._buffer, destination._buffer, size,
            source_offset, destination_offset,
        )

    def finish(self) -> GPUCommandBuffer:
        """Finish recording and produce a frozen command buffer.

        Returns:
            An immutable GPUCommandBuffer ready for submission.
        """
        self._cb.end()
        return GPUCommandBuffer(self._cb)


# =========================================================================
# GPUQueue — the single submission queue
# =========================================================================


class GPUQueue:
    """A WebGPU queue — the only queue on the device.

    WebGPU simplifies queues to just one: device.queue. All submissions
    go through this single queue. The runtime handles internal scheduling.
    """

    def __init__(self, device: GPUDevice) -> None:
        self._device = device

    def submit(self, command_buffers: list[GPUCommandBuffer]) -> None:
        """Submit command buffers for execution.

        In WebGPU, there's no explicit fence — the runtime tracks
        completion internally.

        Args:
            command_buffers: List of frozen command buffers.
        """
        queue = self._device._compute_queue
        for gpu_cb in command_buffers:
            fence = self._device._logical_device.create_fence()
            queue.submit([gpu_cb._cb], fence=fence)
            fence.wait()

    def write_buffer(
        self,
        buffer: GPUBuffer,
        buffer_offset: int,
        data: bytes | bytearray,
    ) -> None:
        """Write data to a buffer (convenience method).

        This combines mapping, writing, and unmapping into a single call.
        In real WebGPU, this is implemented as an internal staging upload.

        Args:
            buffer:        Destination buffer.
            buffer_offset: Byte offset in the buffer.
            data:          Data to write.
        """
        mm = self._device._memory_manager
        mapped = mm.map(buffer._buffer)
        mapped.write(buffer_offset, bytes(data))
        mm.unmap(buffer._buffer)


# =========================================================================
# GPUDevice — the main WebGPU device
# =========================================================================


class GPUDevice(BaseVendorSimulator):
    """A WebGPU device — the main entry point for GPU programming.

    The device provides:
    - device.queue: The single submission queue
    - Factory methods for all GPU resources
    """

    def __init__(self, physical_device: Any = None) -> None:
        if physical_device:
            super().__init__(vendor_hint=physical_device.vendor)
        else:
            super().__init__()
        self.queue = GPUQueue(self)
        self.features: set[str] = {"compute"}
        self.limits = GPUDeviceLimits()

    def create_buffer(self, descriptor: GPUBufferDescriptor) -> GPUBuffer:
        """Create a buffer.

        WebGPU picks the optimal memory type based on usage flags.

        Args:
            descriptor: Buffer parameters.

        Returns:
            A new GPUBuffer.
        """
        mem_type = (
            MemoryType.DEVICE_LOCAL
            | MemoryType.HOST_VISIBLE
            | MemoryType.HOST_COHERENT
        )
        usage = BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST

        buf = self._memory_manager.allocate(
            descriptor.size, mem_type, usage=usage
        )
        gpu_buf = GPUBuffer(
            buf, self._memory_manager, descriptor.size, descriptor.usage
        )

        # If mapped at creation, pre-map for writing
        if descriptor.mapped_at_creation:
            gpu_buf.map_async(GPUMapMode.WRITE)

        return gpu_buf

    def create_shader_module(
        self, descriptor: GPUShaderModuleDescriptor
    ) -> GPUShaderModule:
        """Create a shader module.

        Args:
            descriptor: Shader parameters.

        Returns:
            A new GPUShaderModule.
        """
        code = descriptor.code if isinstance(descriptor.code, list) else None
        shader = self._logical_device.create_shader_module(code=code)
        return GPUShaderModule(shader)

    def create_compute_pipeline(
        self, descriptor: GPUComputePipelineDescriptor
    ) -> GPUComputePipeline:
        """Create a compute pipeline.

        Supports layout="auto" for automatic bind group layout inference.

        Args:
            descriptor: Pipeline parameters.

        Returns:
            A new GPUComputePipeline.
        """
        shader = (
            descriptor.compute.module._shader
            if descriptor.compute and descriptor.compute.module
            else self._logical_device.create_shader_module()
        )

        # Create an empty default layout for "auto"
        ds_layout = self._logical_device.create_descriptor_set_layout([])
        pl_layout = self._logical_device.create_pipeline_layout([ds_layout])
        pipeline = self._logical_device.create_compute_pipeline(shader, pl_layout)

        bg_layout = GPUBindGroupLayout(ds_layout)
        return GPUComputePipeline(pipeline, [bg_layout])

    def create_bind_group_layout(
        self, descriptor: GPUBindGroupLayoutDescriptor
    ) -> GPUBindGroupLayout:
        """Create a bind group layout.

        Args:
            descriptor: Layout parameters.

        Returns:
            A new GPUBindGroupLayout.
        """
        bindings = [
            DescriptorBinding(
                binding=e.binding,
                type=e.buffer.type if e.buffer else "storage",
            )
            for e in descriptor.entries
        ]
        layout = self._logical_device.create_descriptor_set_layout(bindings)
        return GPUBindGroupLayout(layout)

    def create_pipeline_layout(
        self, descriptor: GPUPipelineLayoutDescriptor
    ) -> GPUPipelineLayout:
        """Create a pipeline layout.

        Args:
            descriptor: Layout parameters.

        Returns:
            A new GPUPipelineLayout.
        """
        layouts = [bg._layout for bg in descriptor.bind_group_layouts]
        pl = self._logical_device.create_pipeline_layout(layouts)
        return GPUPipelineLayout(pl)

    def create_bind_group(
        self, descriptor: GPUBindGroupDescriptor
    ) -> GPUBindGroup:
        """Create a bind group (WebGPU's descriptor set).

        Args:
            descriptor: Bind group parameters with buffer entries.

        Returns:
            A new GPUBindGroup.
        """
        layout = (
            descriptor.layout._layout
            if descriptor.layout
            else self._logical_device.create_descriptor_set_layout([])
        )
        ds = self._logical_device.create_descriptor_set(layout)
        for entry in descriptor.entries:
            if entry.resource is not None:
                ds.write(entry.binding, entry.resource._buffer)
        return GPUBindGroup(ds)

    def create_command_encoder(
        self, descriptor: GPUCommandEncoderDescriptor | None = None
    ) -> GPUCommandEncoder:
        """Create a command encoder.

        Returns:
            A new GPUCommandEncoder ready for recording.
        """
        return GPUCommandEncoder(self)

    def destroy(self) -> None:
        """Destroy this device and release all resources."""
        self._logical_device.wait_idle()


# =========================================================================
# GPUAdapter — physical device wrapper
# =========================================================================


class GPUAdapter:
    """A WebGPU adapter — represents a physical GPU.

    The adapter lets you inspect hardware capabilities before creating
    a device. In real WebGPU, adapter selection is async.
    """

    def __init__(self, physical_device: Any) -> None:
        self._physical = physical_device
        self.features: set[str] = {"compute"}
        self.limits = GPUAdapterLimits()

    @property
    def name(self) -> str:
        """Adapter name."""
        return self._physical.name

    def request_device(
        self, descriptor: GPUDeviceDescriptor | None = None
    ) -> GPUDevice:
        """Request a device from this adapter.

        Args:
            descriptor: Optional device requirements.

        Returns:
            A new GPUDevice.
        """
        return GPUDevice(self._physical)


# =========================================================================
# GPU — the top-level WebGPU entry point
# =========================================================================


class GPU:
    """The WebGPU entry point — like navigator.gpu in browsers.

    In a browser, you access GPU via navigator.gpu. In our simulator,
    you create a GPU() object directly.
    """

    def __init__(self) -> None:
        self._instance = __import__("compute_runtime").RuntimeInstance()
        self._physical_devices = self._instance.enumerate_physical_devices()

    def request_adapter(
        self, options: GPURequestAdapterOptions | None = None
    ) -> GPUAdapter:
        """Request a GPU adapter.

        Args:
            options: Adapter selection preferences.

        Returns:
            A GPUAdapter wrapping a physical device.
        """
        if not self._physical_devices:
            raise RuntimeError("No GPU adapters available")

        # Pick based on power preference
        if options and options.power_preference == "low-power":
            # Prefer integrated / low-power devices
            for pd in self._physical_devices:
                if pd.memory_properties.is_unified:
                    return GPUAdapter(pd)

        return GPUAdapter(self._physical_devices[0])
