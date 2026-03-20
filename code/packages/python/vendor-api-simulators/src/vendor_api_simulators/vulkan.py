"""Vulkan Runtime Simulator — the thinnest wrapper over Layer 5.

=== What is Vulkan? ===

Vulkan is the Khronos Group's low-level, cross-platform GPU API. It's the
most explicit GPU API — you manage everything: memory types, command buffer
recording, queue submission, synchronization barriers, descriptor set layouts.

Because our Layer 5 compute runtime is already Vulkan-inspired, this
simulator is the **thinnest wrapper** of all six. It mainly adds:

1. Vulkan naming conventions (the `vk_` prefix on all methods)
2. Vulkan-specific structures (VkBufferCreateInfo, VkSubmitInfo, etc.)
3. VkResult return codes instead of Python exceptions
4. VkCommandPool for grouping command buffers

=== Why Vulkan is So Verbose ===

Vulkan forces you to be explicit about everything because:

1. **No hidden allocations** — you control every byte of memory
2. **No implicit sync** — you insert every barrier yourself
3. **No automatic resource tracking** — you free what you allocate
4. **No driver guessing** — you tell the driver exactly what you need

The reward is maximum performance and predictability. The Vulkan driver
is thin — it does exactly what you say, no more. This is why AAA game
engines and professional compute use Vulkan.

=== Structure of a Vulkan Program ===

    1. VkInstance → VkPhysicalDevice → VkDevice → VkQueue
    2. VkBuffer + VkDeviceMemory (allocate + bind)
    3. VkShaderModule → VkPipeline + VkDescriptorSet
    4. VkCommandPool → VkCommandBuffer (record commands)
    5. vkQueueSubmit() + VkFence (execute + synchronize)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, Flag, auto
from typing import Any

from compute_runtime import (
    Buffer,
    CommandBuffer,
    DescriptorBinding,
    DescriptorSet,
    DescriptorSetLayout,
    Fence,
    LogicalDevice,
    MemoryType,
    BufferUsage,
    PhysicalDevice,
    Pipeline,
    PipelineLayout,
    RuntimeInstance,
    Semaphore,
    ShaderModule,
    CommandQueue,
    PipelineBarrier,
)

from ._base import BaseVendorSimulator


# =========================================================================
# Vulkan enums
# =========================================================================


class VkResult(Enum):
    """Vulkan function return codes.

    SUCCESS:                     Operation completed successfully.
    NOT_READY:                   Fence/query not yet signaled.
    TIMEOUT:                     Wait timed out.
    ERROR_OUT_OF_DEVICE_MEMORY:  GPU ran out of memory.
    ERROR_DEVICE_LOST:           GPU crashed or was removed.
    ERROR_INITIALIZATION_FAILED: Failed to create instance/device.
    """

    SUCCESS = 0
    NOT_READY = 1
    TIMEOUT = 2
    ERROR_OUT_OF_DEVICE_MEMORY = -3
    ERROR_DEVICE_LOST = -4
    ERROR_INITIALIZATION_FAILED = -5


class VkPipelineBindPoint(Enum):
    """Which pipeline type to bind — compute or graphics.

    We only support COMPUTE in our simulator.
    """

    COMPUTE = auto()


class VkBufferUsageFlagBits(Flag):
    """Vulkan buffer usage flags — how the buffer will be used.

    STORAGE_BUFFER: Shader storage buffer object (SSBO).
    UNIFORM_BUFFER: Uniform buffer object (UBO).
    TRANSFER_SRC:   Source of a copy/blit operation.
    TRANSFER_DST:   Destination of a copy/blit operation.
    """

    STORAGE_BUFFER = auto()
    UNIFORM_BUFFER = auto()
    TRANSFER_SRC = auto()
    TRANSFER_DST = auto()


class VkMemoryPropertyFlagBits(Flag):
    """Vulkan memory property flags — where and how memory is accessible.

    DEVICE_LOCAL:  Fast GPU memory (VRAM).
    HOST_VISIBLE:  CPU can map and access.
    HOST_COHERENT: CPU writes immediately visible to GPU.
    HOST_CACHED:   CPU reads are cached (fast readback).
    """

    DEVICE_LOCAL = auto()
    HOST_VISIBLE = auto()
    HOST_COHERENT = auto()
    HOST_CACHED = auto()


class VkSharingMode(Enum):
    """Whether a resource is used by one queue family or multiple.

    EXCLUSIVE:  Used by one queue family (most common).
    CONCURRENT: Shared across multiple queue families.
    """

    EXCLUSIVE = auto()
    CONCURRENT = auto()


# =========================================================================
# Vulkan create-info structures
# =========================================================================

# In Vulkan, every resource is created via a "create info" structure.
# This pattern is verbose but has important benefits:
#
# 1. All parameters are named and explicit (no positional ambiguity)
# 2. Extensions can add new fields without breaking the API
# 3. The driver can validate all parameters at once
# 4. Create-info structs can be cached and reused


@dataclass
class VkBufferCreateInfo:
    """Parameters for creating a VkBuffer.

    Fields:
        size:         Buffer size in bytes.
        usage:        How the buffer will be used (STORAGE, TRANSFER, etc.).
        sharing_mode: EXCLUSIVE or CONCURRENT queue access.
    """

    size: int = 0
    usage: VkBufferUsageFlagBits = VkBufferUsageFlagBits.STORAGE_BUFFER
    sharing_mode: VkSharingMode = VkSharingMode.EXCLUSIVE


@dataclass
class VkMemoryAllocateInfo:
    """Parameters for allocating device memory.

    Fields:
        size:              Bytes to allocate.
        memory_type_index: Which memory type to use (index into device's
                          memory type array). 0 = DEVICE_LOCAL, 1 = HOST_VISIBLE.
    """

    size: int = 0
    memory_type_index: int = 0


@dataclass
class VkShaderModuleCreateInfo:
    """Parameters for creating a shader module.

    Fields:
        code: The compiled shader code (GPU instructions in our simulator).
    """

    code: list[Any] | None = None


@dataclass
class VkComputePipelineCreateInfo:
    """Parameters for creating a compute pipeline.

    Fields:
        shader_stage: The shader stage configuration.
        layout:       The pipeline layout (descriptor sets + push constants).
    """

    shader_stage: VkPipelineShaderStageCreateInfo | None = None
    layout: VkPipelineLayout | None = None


@dataclass
class VkPipelineShaderStageCreateInfo:
    """Shader stage configuration for pipeline creation.

    Fields:
        stage:       Which pipeline stage (always COMPUTE for us).
        module:      The compiled shader module.
        entry_point: Function name in the shader (typically "main").
    """

    stage: str = "compute"
    module: VkShaderModule | None = None
    entry_point: str = "main"


@dataclass
class VkSubmitInfo:
    """Parameters for a queue submission.

    Fields:
        command_buffers:   CBs to execute.
        wait_semaphores:   Semaphores to wait on before starting.
        signal_semaphores: Semaphores to signal after completion.
    """

    command_buffers: list[VkCommandBuffer] = field(default_factory=list)
    wait_semaphores: list[VkSemaphore] = field(default_factory=list)
    signal_semaphores: list[VkSemaphore] = field(default_factory=list)


@dataclass
class VkBufferCopy:
    """Region to copy between buffers.

    Fields:
        src_offset: Byte offset in the source buffer.
        dst_offset: Byte offset in the destination buffer.
        size:       Number of bytes to copy.
    """

    src_offset: int = 0
    dst_offset: int = 0
    size: int = 0


@dataclass
class VkWriteDescriptorSet:
    """Describes a write to a descriptor set (binding a buffer).

    Fields:
        dst_set:         Target descriptor set.
        dst_binding:     Which binding slot to write.
        descriptor_type: Type of descriptor ("storage" or "uniform").
        buffer_info:     Buffer binding information.
    """

    dst_set: VkDescriptorSet | None = None
    dst_binding: int = 0
    descriptor_type: str = "storage"
    buffer_info: VkDescriptorBufferInfo | None = None


@dataclass
class VkDescriptorBufferInfo:
    """Buffer reference for descriptor set writes.

    Fields:
        buffer: The VkBuffer to bind.
        offset: Byte offset into the buffer.
        range:  Number of bytes to expose (0 = whole buffer).
    """

    buffer: VkBuffer | None = None
    offset: int = 0
    range: int = 0


@dataclass
class VkCommandPoolCreateInfo:
    """Parameters for creating a command pool.

    Fields:
        queue_family_index: Which queue family this pool allocates for.
    """

    queue_family_index: int = 0


@dataclass
class VkDescriptorSetLayoutCreateInfo:
    """Parameters for creating a descriptor set layout.

    Fields:
        bindings: List of binding slot descriptions.
    """

    bindings: list[VkDescriptorSetLayoutBinding] = field(default_factory=list)


@dataclass
class VkDescriptorSetLayoutBinding:
    """One binding slot in a descriptor set layout.

    Fields:
        binding:          Slot number (0, 1, 2, ...).
        descriptor_type:  "storage" or "uniform".
        descriptor_count: How many descriptors at this binding.
    """

    binding: int = 0
    descriptor_type: str = "storage"
    descriptor_count: int = 1


@dataclass
class VkPipelineLayoutCreateInfo:
    """Parameters for creating a pipeline layout.

    Fields:
        set_layouts:         Descriptor set layouts used by this pipeline.
        push_constant_size:  Max push constant bytes.
    """

    set_layouts: list[VkDescriptorSetLayout] = field(default_factory=list)
    push_constant_size: int = 0


@dataclass
class VkDescriptorSetAllocateInfo:
    """Parameters for allocating descriptor sets.

    Fields:
        set_layouts: Layouts to allocate sets from.
    """

    set_layouts: list[VkDescriptorSetLayout] = field(default_factory=list)


# =========================================================================
# Vulkan wrapper objects — thin wrappers over Layer 5
# =========================================================================


class VkPhysicalDevice:
    """Vulkan physical device — wraps Layer 5 PhysicalDevice."""

    def __init__(self, physical: PhysicalDevice) -> None:
        self._physical = physical

    def vk_get_physical_device_properties(self) -> dict[str, Any]:
        """Query device properties (vkGetPhysicalDeviceProperties)."""
        return {
            "device_name": self._physical.name,
            "device_type": self._physical.device_type.value,
            "vendor": self._physical.vendor,
        }

    def vk_get_physical_device_memory_properties(self) -> dict[str, Any]:
        """Query memory properties (vkGetPhysicalDeviceMemoryProperties)."""
        mp = self._physical.memory_properties
        return {
            "heap_count": len(mp.heaps),
            "heaps": [{"size": h.size, "flags": str(h.flags)} for h in mp.heaps],
            "is_unified": mp.is_unified,
        }

    def vk_get_physical_device_queue_family_properties(
        self,
    ) -> list[dict[str, Any]]:
        """Query queue family properties."""
        return [
            {"queue_type": qf.queue_type.value, "queue_count": qf.count}
            for qf in self._physical.queue_families
        ]


class VkBuffer:
    """Vulkan buffer — wraps Layer 5 Buffer."""

    def __init__(self, buffer: Buffer) -> None:
        self._buffer = buffer

    @property
    def size(self) -> int:
        """Buffer size in bytes."""
        return self._buffer.size


class VkDeviceMemory:
    """Vulkan device memory — wraps Layer 5 Buffer's memory.

    In Vulkan, memory and buffers are separate concepts:
    - VkDeviceMemory is a raw allocation
    - VkBuffer is a view into that allocation
    - vkBindBufferMemory connects them

    In our simulator, Layer 5's Buffer combines both, so VkDeviceMemory
    wraps the same Buffer.
    """

    def __init__(self, buffer: Buffer, memory_manager: Any) -> None:
        self._buffer = buffer
        self._mm = memory_manager


class VkShaderModule:
    """Vulkan shader module — wraps Layer 5 ShaderModule."""

    def __init__(self, shader: ShaderModule) -> None:
        self._shader = shader


class VkPipeline:
    """Vulkan pipeline — wraps Layer 5 Pipeline."""

    def __init__(self, pipeline: Pipeline) -> None:
        self._pipeline = pipeline


class VkDescriptorSetLayout:
    """Vulkan descriptor set layout — wraps Layer 5 DescriptorSetLayout."""

    def __init__(self, layout: DescriptorSetLayout) -> None:
        self._layout = layout


class VkPipelineLayout:
    """Vulkan pipeline layout — wraps Layer 5 PipelineLayout."""

    def __init__(self, layout: PipelineLayout) -> None:
        self._layout = layout


class VkDescriptorSet:
    """Vulkan descriptor set — wraps Layer 5 DescriptorSet."""

    def __init__(self, descriptor_set: DescriptorSet) -> None:
        self._ds = descriptor_set


class VkFence:
    """Vulkan fence — wraps Layer 5 Fence."""

    def __init__(self, fence: Fence) -> None:
        self._fence = fence

    @property
    def signaled(self) -> bool:
        """Whether the fence has been signaled."""
        return self._fence.signaled


class VkSemaphore:
    """Vulkan semaphore — wraps Layer 5 Semaphore."""

    def __init__(self, semaphore: Semaphore) -> None:
        self._semaphore = semaphore


class VkCommandPool:
    """Vulkan command pool — groups command buffers.

    In Vulkan, command buffers are allocated from pools. Pools can be
    reset to recycle all their command buffers at once. This is more
    efficient than individual CB management.
    """

    def __init__(self, device: VkDevice, queue_family_index: int) -> None:
        self._device = device
        self._queue_family_index = queue_family_index
        self._command_buffers: list[VkCommandBuffer] = []

    def vk_allocate_command_buffers(self, count: int) -> list[VkCommandBuffer]:
        """Allocate command buffers from this pool.

        Args:
            count: Number of command buffers to allocate.

        Returns:
            List of new VkCommandBuffer objects.
        """
        cbs = []
        for _ in range(count):
            inner_cb = self._device._logical.create_command_buffer()
            vk_cb = VkCommandBuffer(inner_cb)
            cbs.append(vk_cb)
            self._command_buffers.append(vk_cb)
        return cbs

    def vk_reset_command_pool(self) -> None:
        """Reset all command buffers in this pool."""
        for vk_cb in self._command_buffers:
            vk_cb._cb.reset()

    def vk_free_command_buffers(self, buffers: list[VkCommandBuffer]) -> None:
        """Free specific command buffers back to this pool.

        Args:
            buffers: Command buffers to free.
        """
        for buf in buffers:
            if buf in self._command_buffers:
                self._command_buffers.remove(buf)


class VkCommandBuffer:
    """Vulkan command buffer — wraps Layer 5 CommandBuffer with vk_ prefix."""

    def __init__(self, cb: CommandBuffer) -> None:
        self._cb = cb

    def vk_begin_command_buffer(self, flags: int = 0) -> None:
        """Begin recording (vkBeginCommandBuffer)."""
        self._cb.begin()

    def vk_end_command_buffer(self) -> None:
        """End recording (vkEndCommandBuffer)."""
        self._cb.end()

    def vk_cmd_bind_pipeline(
        self, bind_point: VkPipelineBindPoint, pipeline: VkPipeline
    ) -> None:
        """Bind a pipeline (vkCmdBindPipeline)."""
        self._cb.cmd_bind_pipeline(pipeline._pipeline)

    def vk_cmd_bind_descriptor_sets(
        self,
        bind_point: VkPipelineBindPoint,
        layout: VkPipelineLayout,
        descriptor_sets: list[VkDescriptorSet],
    ) -> None:
        """Bind descriptor sets (vkCmdBindDescriptorSets)."""
        for ds in descriptor_sets:
            self._cb.cmd_bind_descriptor_set(ds._ds)

    def vk_cmd_push_constants(
        self, layout: VkPipelineLayout, offset: int, data: bytes
    ) -> None:
        """Set push constants (vkCmdPushConstants)."""
        self._cb.cmd_push_constants(offset, data)

    def vk_cmd_dispatch(self, x: int, y: int = 1, z: int = 1) -> None:
        """Dispatch compute work (vkCmdDispatch)."""
        self._cb.cmd_dispatch(x, y, z)

    def vk_cmd_copy_buffer(
        self, src: VkBuffer, dst: VkBuffer, regions: list[VkBufferCopy]
    ) -> None:
        """Copy between buffers (vkCmdCopyBuffer)."""
        for region in regions:
            self._cb.cmd_copy_buffer(
                src._buffer, dst._buffer, region.size,
                region.src_offset, region.dst_offset,
            )

    def vk_cmd_fill_buffer(
        self, buffer: VkBuffer, offset: int, size: int, data: int
    ) -> None:
        """Fill buffer with a value (vkCmdFillBuffer)."""
        self._cb.cmd_fill_buffer(buffer._buffer, data, offset, size)

    def vk_cmd_pipeline_barrier(
        self,
        src_stage: str,
        dst_stage: str,
        buffer_barriers: list[Any] | None = None,
    ) -> None:
        """Insert a pipeline barrier (vkCmdPipelineBarrier)."""
        from compute_runtime import PipelineStage

        barrier = PipelineBarrier(
            src_stage=PipelineStage(src_stage),
            dst_stage=PipelineStage(dst_stage),
        )
        self._cb.cmd_pipeline_barrier(barrier)


class VkQueue:
    """Vulkan queue — wraps Layer 5 CommandQueue."""

    def __init__(self, queue: CommandQueue) -> None:
        self._queue = queue

    def vk_queue_submit(
        self,
        submits: list[VkSubmitInfo],
        fence: VkFence | None = None,
    ) -> VkResult:
        """Submit work to the queue (vkQueueSubmit).

        Args:
            submits: List of VkSubmitInfo with command buffers.
            fence:   Optional fence to signal on completion.

        Returns:
            VkResult.SUCCESS on success.
        """
        for submit in submits:
            cbs = [vk_cb._cb for vk_cb in submit.command_buffers]
            wait_sems = [s._semaphore for s in submit.wait_semaphores]
            signal_sems = [s._semaphore for s in submit.signal_semaphores]

            self._queue.submit(
                cbs,
                wait_semaphores=wait_sems or None,
                signal_semaphores=signal_sems or None,
                fence=fence._fence if fence else None,
            )
        return VkResult.SUCCESS

    def vk_queue_wait_idle(self) -> None:
        """Wait for all queue work to complete (vkQueueWaitIdle)."""
        self._queue.wait_idle()


# =========================================================================
# VkDevice — wraps LogicalDevice
# =========================================================================


class VkDevice:
    """Vulkan logical device — wraps Layer 5 LogicalDevice with vk_ API.

    This is the most verbose of all six simulators because Vulkan exposes
    every operation as a separate, explicit call.
    """

    def __init__(self, logical: LogicalDevice) -> None:
        self._logical = logical

    def vk_get_device_queue(
        self, family_index: int, queue_index: int
    ) -> VkQueue:
        """Get a queue from the device (vkGetDeviceQueue).

        Args:
            family_index: Queue family (0 = compute, 1 = transfer).
            queue_index:  Queue index within the family.

        Returns:
            A VkQueue.
        """
        family_name = "compute" if family_index == 0 else "transfer"
        if family_name in self._logical.queues:
            queues = self._logical.queues[family_name]
            if queue_index < len(queues):
                return VkQueue(queues[queue_index])
        return VkQueue(self._logical.queues["compute"][0])

    def vk_create_command_pool(
        self, create_info: VkCommandPoolCreateInfo
    ) -> VkCommandPool:
        """Create a command pool (vkCreateCommandPool)."""
        return VkCommandPool(self, create_info.queue_family_index)

    def vk_allocate_memory(
        self, alloc_info: VkMemoryAllocateInfo
    ) -> VkDeviceMemory:
        """Allocate device memory (vkAllocateMemory).

        Maps memory_type_index to Layer 5 MemoryType:
        - 0 → DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT
        - 1 → HOST_VISIBLE | HOST_COHERENT

        Args:
            alloc_info: Allocation parameters.

        Returns:
            A VkDeviceMemory handle.
        """
        if alloc_info.memory_type_index == 0:
            mem_type = (
                MemoryType.DEVICE_LOCAL
                | MemoryType.HOST_VISIBLE
                | MemoryType.HOST_COHERENT
            )
        else:
            mem_type = MemoryType.HOST_VISIBLE | MemoryType.HOST_COHERENT

        buf = self._logical.memory_manager.allocate(
            alloc_info.size, mem_type,
            usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST,
        )
        return VkDeviceMemory(buf, self._logical.memory_manager)

    def vk_create_buffer(
        self, create_info: VkBufferCreateInfo
    ) -> VkBuffer:
        """Create a buffer (vkCreateBuffer).

        In real Vulkan, this creates a buffer object WITHOUT backing memory.
        You must separately allocate memory and bind it with
        vk_bind_buffer_memory(). In our simulator, the buffer is backed
        by the allocated VkDeviceMemory.

        Args:
            create_info: Buffer parameters.

        Returns:
            A VkBuffer (not yet backed by memory).
        """
        # Allocate underlying storage
        mem_type = (
            MemoryType.DEVICE_LOCAL
            | MemoryType.HOST_VISIBLE
            | MemoryType.HOST_COHERENT
        )
        buf = self._logical.memory_manager.allocate(
            create_info.size, mem_type,
            usage=BufferUsage.STORAGE | BufferUsage.TRANSFER_SRC | BufferUsage.TRANSFER_DST,
        )
        return VkBuffer(buf)

    def vk_bind_buffer_memory(
        self, buffer: VkBuffer, memory: VkDeviceMemory, offset: int
    ) -> None:
        """Bind memory to a buffer (vkBindBufferMemory).

        In real Vulkan, this connects a buffer object to backing memory.
        In our simulator, buffers are already backed, so this is a no-op.
        """
        pass

    def vk_map_memory(
        self, memory: VkDeviceMemory, offset: int, size: int
    ) -> bytearray:
        """Map device memory for CPU access (vkMapMemory).

        Args:
            memory: The memory to map.
            offset: Byte offset (ignored in simulator).
            size:   Bytes to map.

        Returns:
            A mutable bytearray view of the memory.
        """
        mapped = memory._mm.map(memory._buffer)
        return bytearray(mapped.get_data())

    def vk_unmap_memory(self, memory: VkDeviceMemory) -> None:
        """Unmap device memory (vkUnmapMemory)."""
        if memory._buffer.mapped:
            memory._mm.unmap(memory._buffer)

    def vk_create_shader_module(
        self, create_info: VkShaderModuleCreateInfo
    ) -> VkShaderModule:
        """Create a shader module (vkCreateShaderModule)."""
        shader = self._logical.create_shader_module(code=create_info.code)
        return VkShaderModule(shader)

    def vk_create_descriptor_set_layout(
        self, create_info: VkDescriptorSetLayoutCreateInfo
    ) -> VkDescriptorSetLayout:
        """Create a descriptor set layout (vkCreateDescriptorSetLayout)."""
        bindings = [
            DescriptorBinding(
                binding=b.binding,
                type=b.descriptor_type,
                count=b.descriptor_count,
            )
            for b in create_info.bindings
        ]
        layout = self._logical.create_descriptor_set_layout(bindings)
        return VkDescriptorSetLayout(layout)

    def vk_create_pipeline_layout(
        self, create_info: VkPipelineLayoutCreateInfo
    ) -> VkPipelineLayout:
        """Create a pipeline layout (vkCreatePipelineLayout)."""
        layouts = [sl._layout for sl in create_info.set_layouts]
        pl = self._logical.create_pipeline_layout(
            layouts, create_info.push_constant_size
        )
        return VkPipelineLayout(pl)

    def vk_create_compute_pipelines(
        self, create_infos: list[VkComputePipelineCreateInfo]
    ) -> list[VkPipeline]:
        """Create compute pipelines (vkCreateComputePipelines)."""
        pipelines = []
        for ci in create_infos:
            shader = ci.shader_stage.module._shader if ci.shader_stage and ci.shader_stage.module else None
            layout = ci.layout._layout if ci.layout else None
            if shader and layout:
                p = self._logical.create_compute_pipeline(shader, layout)
                pipelines.append(VkPipeline(p))
        return pipelines

    def vk_allocate_descriptor_sets(
        self, alloc_info: VkDescriptorSetAllocateInfo
    ) -> list[VkDescriptorSet]:
        """Allocate descriptor sets (vkAllocateDescriptorSets)."""
        sets = []
        for sl in alloc_info.set_layouts:
            ds = self._logical.create_descriptor_set(sl._layout)
            sets.append(VkDescriptorSet(ds))
        return sets

    def vk_update_descriptor_sets(
        self, writes: list[VkWriteDescriptorSet]
    ) -> None:
        """Write buffer bindings to descriptor sets (vkUpdateDescriptorSets)."""
        for write in writes:
            if write.dst_set and write.buffer_info and write.buffer_info.buffer:
                write.dst_set._ds.write(
                    write.dst_binding,
                    write.buffer_info.buffer._buffer,
                )

    def vk_create_fence(self, flags: int = 0) -> VkFence:
        """Create a fence (vkCreateFence).

        Args:
            flags: Fence creation flags. 1 = signaled initially.
        """
        signaled = bool(flags & 1)
        fence = self._logical.create_fence(signaled=signaled)
        return VkFence(fence)

    def vk_create_semaphore(self) -> VkSemaphore:
        """Create a semaphore (vkCreateSemaphore)."""
        sem = self._logical.create_semaphore()
        return VkSemaphore(sem)

    def vk_wait_for_fences(
        self,
        fences: list[VkFence],
        wait_all: bool,
        timeout: int,
    ) -> VkResult:
        """Wait for fences (vkWaitForFences).

        Args:
            fences:   Fences to wait on.
            wait_all: If True, wait for ALL fences. If False, any one.
            timeout:  Timeout in nanoseconds.

        Returns:
            VkResult.SUCCESS or VkResult.NOT_READY.
        """
        for f in fences:
            if f._fence.signaled:
                if not wait_all:
                    return VkResult.SUCCESS
            elif wait_all:
                return VkResult.NOT_READY
        return VkResult.SUCCESS

    def vk_reset_fences(self, fences: list[VkFence]) -> None:
        """Reset fences to unsignaled state (vkResetFences)."""
        for f in fences:
            f._fence.reset()

    def vk_device_wait_idle(self) -> None:
        """Wait for all work to complete (vkDeviceWaitIdle)."""
        self._logical.wait_idle()


# =========================================================================
# VkInstance — the Vulkan entry point
# =========================================================================


class VkInstance(BaseVendorSimulator):
    """Vulkan instance — the entry point for device discovery.

    Unlike CUDA (which auto-selects NVIDIA) or Metal (which auto-selects
    Apple), Vulkan gives you all devices and lets you choose.
    """

    def __init__(self) -> None:
        super().__init__()

    def vk_enumerate_physical_devices(self) -> list[VkPhysicalDevice]:
        """Enumerate all physical devices (vkEnumeratePhysicalDevices).

        Returns:
            List of VkPhysicalDevice wrappers.
        """
        return [VkPhysicalDevice(pd) for pd in self._physical_devices]

    def vk_create_device(
        self, physical_device: VkPhysicalDevice
    ) -> VkDevice:
        """Create a logical device (vkCreateDevice).

        Args:
            physical_device: The physical device to create a handle for.

        Returns:
            A VkDevice ready for use.
        """
        logical = self._instance.create_logical_device(physical_device._physical)
        return VkDevice(logical)
