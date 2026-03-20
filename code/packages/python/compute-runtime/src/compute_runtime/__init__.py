"""Compute Runtime — Layer 5 of the accelerator computing stack.

A low-level Vulkan-inspired compute runtime that provides the software
infrastructure between user-facing APIs (CUDA, OpenCL, Metal, Vulkan)
and the hardware device simulators (Layer 6).

=== Quick Start ===

    from compute_runtime import (
        RuntimeInstance, MemoryType, BufferUsage, PipelineStage,
    )
    from gpu_core import limm, halt

    # 1. Discover devices
    instance = RuntimeInstance()
    devices = instance.enumerate_physical_devices()
    nvidia = next(d for d in devices if d.vendor == "nvidia")

    # 2. Create logical device
    device = instance.create_logical_device(nvidia)
    queue = device.queues["compute"][0]
    mm = device.memory_manager

    # 3. Allocate buffers
    buf = mm.allocate(256, MemoryType.DEVICE_LOCAL | MemoryType.HOST_VISIBLE,
                      usage=BufferUsage.STORAGE)

    # 4. Create pipeline
    shader = device.create_shader_module(code=[limm(0, 42.0), halt()])
    ds_layout = device.create_descriptor_set_layout([])
    pl_layout = device.create_pipeline_layout([ds_layout])
    pipeline = device.create_compute_pipeline(shader, pl_layout)

    # 5. Record and submit commands
    cb = device.create_command_buffer()
    cb.begin()
    cb.cmd_bind_pipeline(pipeline)
    cb.cmd_dispatch(1, 1, 1)
    cb.end()

    fence = device.create_fence()
    queue.submit([cb], fence=fence)
    fence.wait()

=== Architecture ===

    RuntimeInstance
    ├── enumerate_physical_devices() → PhysicalDevice[]
    └── create_logical_device() → LogicalDevice
        ├── queues: CommandQueue[]
        ├── memory_manager: MemoryManager
        ├── create_command_buffer() → CommandBuffer
        ├── create_compute_pipeline() → Pipeline
        ├── create_fence() → Fence
        └── create_semaphore() → Semaphore
"""

# Protocols and types
from .protocols import (
    AccessFlags,
    BufferBarrier,
    BufferUsage,
    CommandBufferState,
    DescriptorBinding,
    DeviceLimits,
    DeviceType,
    MemoryBarrier,
    MemoryHeap,
    MemoryProperties,
    MemoryType,
    PipelineBarrier,
    PipelineStage,
    QueueFamily,
    QueueType,
    RecordedCommand,
    RuntimeEventType,
    RuntimeStats,
    RuntimeTrace,
)

# Instance and device management
from .instance import (
    LogicalDevice,
    PhysicalDevice,
    RuntimeInstance,
)

# Memory management
from .memory import (
    Buffer,
    MappedMemory,
    MemoryManager,
)

# Command recording and submission
from .command_buffer import CommandBuffer
from .command_queue import CommandQueue

# Pipeline and descriptors
from .pipeline import (
    DescriptorSet,
    DescriptorSetLayout,
    Pipeline,
    PipelineLayout,
    ShaderModule,
)

# Synchronization
from .sync import (
    Event,
    Fence,
    Semaphore,
)

# Validation
from .validation import (
    ValidationError,
    ValidationLayer,
)

__all__ = [
    # Enums and flags
    "AccessFlags",
    "BufferUsage",
    "CommandBufferState",
    "DeviceType",
    "MemoryType",
    "PipelineStage",
    "QueueType",
    "RuntimeEventType",
    # Data types
    "BufferBarrier",
    "DescriptorBinding",
    "DeviceLimits",
    "MemoryBarrier",
    "MemoryHeap",
    "MemoryProperties",
    "PipelineBarrier",
    "QueueFamily",
    "RecordedCommand",
    "RuntimeStats",
    "RuntimeTrace",
    # Instance and device
    "LogicalDevice",
    "PhysicalDevice",
    "RuntimeInstance",
    # Memory
    "Buffer",
    "MappedMemory",
    "MemoryManager",
    # Commands
    "CommandBuffer",
    "CommandQueue",
    # Pipeline
    "DescriptorSet",
    "DescriptorSetLayout",
    "Pipeline",
    "PipelineLayout",
    "ShaderModule",
    # Synchronization
    "Event",
    "Fence",
    "Semaphore",
    # Validation
    "ValidationError",
    "ValidationLayer",
]
