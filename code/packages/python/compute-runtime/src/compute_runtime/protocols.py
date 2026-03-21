"""Protocols — shared types for the compute runtime.

=== What is a Compute Runtime? ===

A compute runtime is the **software layer between user-facing APIs** (CUDA,
OpenCL, Metal, Vulkan) and the **hardware device simulators** (Layer 6).

Think of it as the GPU driver's internal machinery:

    User code:     y = alpha * x + y
         │
    API layer:     cudaMalloc / vkAllocateMemory / MTLBuffer  (Layer 4, future)
         │
    Runtime:       CommandBuffer, Queue, Fence, Memory types   (THIS LAYER)
         │
    Hardware:      NvidiaGPU.launch_kernel, .step(), .run()    (Layer 6)

The runtime manages:
- **Device discovery and selection** (Instance → PhysicalDevice → LogicalDevice)
- **Memory allocation with types** (DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT)
- **Command recording and submission** (CommandBuffer → CommandQueue)
- **Synchronization** (Fence, Semaphore, Event, PipelineBarrier)
- **Pipeline and descriptor management** (ShaderModule → Pipeline → DescriptorSet)

=== Why Vulkan-Inspired? ===

Vulkan is the most explicit GPU API — it exposes every moving part that
CUDA, OpenCL, and Metal hide behind convenience wrappers. If we model at
Vulkan's level, building the other APIs on top becomes straightforward:

    Vulkan:   "Here's a command buffer with barriers and descriptor sets"
    CUDA:     "Here's a kernel launch" (implicitly creates CB, barriers, etc.)
    Metal:    "Here's a command encoder" (like CB but with Apple conventions)
    OpenCL:   "Here's a kernel with args" (like CUDA but cross-platform)

=== Design Principle: Enums as Documentation ===

Every enum in this file represents a real GPU concept. The variants aren't
arbitrary — they map to actual hardware states, memory types, and pipeline
stages that exist in every GPU driver.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, Flag, auto
from typing import TYPE_CHECKING, Any, Protocol, runtime_checkable

if TYPE_CHECKING:
    from device_simulator import AcceleratorDevice, DeviceTrace


# =========================================================================
# Device types
# =========================================================================


class DeviceType(Enum):
    """What kind of accelerator this is.

    === The Three Families ===

    GPU:  General-purpose, thread-parallel. Thousands of small cores
          running the same program on different data (SIMT/SIMD).
          NVIDIA, AMD, Intel, Apple (GPU portion).

    TPU:  Dataflow, matrix-specialized. One large matrix unit (MXU)
          that processes tiles of matrices in a pipeline.
          Google TPU.

    NPU:  Neural processing unit. Fixed-function for inference,
          with compiler-generated execution schedules.
          Apple ANE, Qualcomm Hexagon, Intel NPU.
    """

    GPU = "gpu"
    TPU = "tpu"
    NPU = "npu"


# =========================================================================
# Queue types
# =========================================================================


class QueueType(Enum):
    """What kind of work a queue can accept.

    === Why Multiple Queue Types? ===

    Real GPUs have separate hardware engines for compute and data transfer.
    While the compute engine runs a kernel, the DMA engine can copy data
    for the next kernel in parallel. This overlap hides PCIe latency.

        Compute Queue:   [Kernel A]────────[Kernel B]────────
        Transfer Queue:  ────[Upload B data]────[Upload C data]──

    Without separate queues, you'd have to wait:

        Single Queue:    [Upload]──[Kernel A]──[Upload]──[Kernel B]──
                         ^^^^^^^^               ^^^^^^^^
                         GPU idle               GPU idle

    COMPUTE:          Can run kernels (dispatch commands).
    TRANSFER:         Can copy data (DMA engine).
    COMPUTE_TRANSFER: Can do both (most common on simple devices).
    """

    COMPUTE = "compute"
    TRANSFER = "transfer"
    COMPUTE_TRANSFER = "compute_transfer"


# =========================================================================
# Memory types (flags, combinable)
# =========================================================================


class MemoryType(Flag):
    """Properties of a memory allocation.

    === Memory Types Explained ===

    These are flags that can be combined with | (bitwise OR) to describe
    memory with multiple properties.

    DEVICE_LOCAL:
        Fast GPU memory (VRAM / HBM). The GPU can access this at full
        bandwidth (1-3 TB/s). The CPU CANNOT directly read/write this
        unless HOST_VISIBLE is also set.

        On discrete GPUs (NVIDIA, AMD): physically separate chip (GDDR6/HBM).
        On unified memory (Apple): same physical RAM, but flagged as GPU-preferred.

    HOST_VISIBLE:
        The CPU can map this memory and read/write it. On discrete GPUs,
        this is typically a small pool of system RAM accessible via PCIe.
        On unified memory, all memory is HOST_VISIBLE.

    HOST_COHERENT:
        CPU writes are immediately visible to the GPU without explicit
        flush. More convenient but may be slower. Without this flag, you
        must call flush() after writing and invalidate() before reading.

    HOST_CACHED:
        CPU reads are cached (fast read-back). Without this, every CPU
        read goes over PCIe — very slow. Use for downloading results.

    === Common Combinations ===

        DEVICE_LOCAL                          → GPU-only, fastest
        HOST_VISIBLE | HOST_COHERENT          → staging buffer for uploads
        HOST_VISIBLE | HOST_CACHED            → read-back buffer for downloads
        DEVICE_LOCAL | HOST_VISIBLE           → unified memory (Apple, resizable BAR)
        DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT → zero-copy unified
    """

    DEVICE_LOCAL = auto()
    HOST_VISIBLE = auto()
    HOST_COHERENT = auto()
    HOST_CACHED = auto()


# =========================================================================
# Buffer usage (flags, combinable)
# =========================================================================


class BufferUsage(Flag):
    """How a buffer will be used.

    === Why Declare Usage? ===

    Telling the GPU how a buffer will be used enables optimizations:
    - STORAGE buffers may be placed in faster memory regions
    - TRANSFER_SRC buffers can be DMA-aligned for faster copies
    - UNIFORM buffers may be cached in special constant caches

    You must declare all intended usages at allocation time. Using a
    buffer in a way not declared is a validation error.

    STORAGE:       Shader/kernel can read and write (SSBO in Vulkan, CUDA global mem).
    UNIFORM:       Shader/kernel can only read. Small, fast (UBO in Vulkan).
    TRANSFER_SRC:  Can be the source of a copy command.
    TRANSFER_DST:  Can be the destination of a copy command.
    INDIRECT:      Contains indirect dispatch parameters (grid dimensions in a buffer).
    """

    STORAGE = auto()
    UNIFORM = auto()
    TRANSFER_SRC = auto()
    TRANSFER_DST = auto()
    INDIRECT = auto()


# =========================================================================
# Pipeline stages
# =========================================================================


class PipelineStage(Enum):
    """Where in the GPU pipeline an operation happens.

    === The GPU Pipeline ===

    Commands flow through stages in order:

        TOP_OF_PIPE → TRANSFER → COMPUTE → BOTTOM_OF_PIPE
              │                                    │
              │          HOST (CPU access)          │
              └────────────────────────────────────┘

    When you create a barrier, you specify:
    - src_stage: "wait until this stage finishes"
    - dst_stage: "before this stage starts"

    Example: after a kernel writes results, before the CPU reads them:
        src_stage = COMPUTE     (wait for kernel to finish)
        dst_stage = HOST        (before CPU access)

    TOP_OF_PIPE:    Virtual stage at the very beginning. No actual work.
    COMPUTE:        Compute shader / kernel execution.
    TRANSFER:       Copy / fill / update buffer operations.
    HOST:           CPU access (map, read, write).
    BOTTOM_OF_PIPE: Virtual stage at the very end. All work done.
    """

    TOP_OF_PIPE = "top_of_pipe"
    COMPUTE = "compute"
    TRANSFER = "transfer"
    HOST = "host"
    BOTTOM_OF_PIPE = "bottom_of_pipe"


# =========================================================================
# Access flags
# =========================================================================


class AccessFlags(Flag):
    """What kind of memory access an operation performs.

    === Why Track Access? ===

    GPUs have caches. When a kernel writes to a buffer, the data may sit
    in L2 cache, not yet visible to a subsequent kernel reading the same
    buffer. A memory barrier with the right access flags ensures caches
    are flushed (for writes) or invalidated (for reads).

    SHADER_READ:     Compute kernel reads from a buffer.
    SHADER_WRITE:    Compute kernel writes to a buffer.
    TRANSFER_READ:   Copy command reads from source buffer.
    TRANSFER_WRITE:  Copy command writes to destination buffer.
    HOST_READ:       CPU reads mapped buffer.
    HOST_WRITE:      CPU writes mapped buffer.
    NONE:            No access (used for layout transitions).
    """

    NONE = 0
    SHADER_READ = auto()
    SHADER_WRITE = auto()
    TRANSFER_READ = auto()
    TRANSFER_WRITE = auto()
    HOST_READ = auto()
    HOST_WRITE = auto()


# =========================================================================
# Command buffer state
# =========================================================================


class CommandBufferState(Enum):
    """Lifecycle state of a command buffer.

    === State Machine ===

        INITIAL ──begin()──► RECORDING ──end()──► RECORDED
            ▲                                        │
            │                                    submit()
            │                                        │
            └──────── reset() ◄── COMPLETE ◄── PENDING
                                      │
                                      └── GPU finished

    You can only record commands in RECORDING state.
    You can only submit in RECORDED state.
    After GPU finishes, the CB moves to COMPLETE.
    Call reset() to reuse it.
    """

    INITIAL = "initial"
    RECORDING = "recording"
    RECORDED = "recorded"
    PENDING = "pending"
    COMPLETE = "complete"


# =========================================================================
# Runtime event types
# =========================================================================


class RuntimeEventType(Enum):
    """Types of events the runtime can produce.

    These are logged in RuntimeTrace for observability — showing the
    software-level view of what's happening (as opposed to DeviceTrace
    which shows hardware cycles).
    """

    SUBMIT = "submit"
    BEGIN_EXECUTION = "begin_execution"
    END_EXECUTION = "end_execution"
    FENCE_SIGNAL = "fence_signal"
    FENCE_WAIT = "fence_wait"
    SEMAPHORE_SIGNAL = "semaphore_signal"
    SEMAPHORE_WAIT = "semaphore_wait"
    BARRIER = "barrier"
    MEMORY_ALLOC = "memory_alloc"
    MEMORY_FREE = "memory_free"
    MEMORY_MAP = "memory_map"
    MEMORY_TRANSFER = "memory_transfer"


# =========================================================================
# Queue family — describes what a queue can do
# =========================================================================


@dataclass(frozen=True)
class QueueFamily:
    """Describes a family of queues with the same capabilities.

    === Queue Families ===

    A GPU might have:
    - 1 family of 16 compute queues
    - 1 family of 2 transfer-only queues

    You request queues from families when creating a logical device.
    All queues in a family have the same capabilities.

    Fields:
        queue_type:  What kind of work this family can do.
        count:       How many queues are available in this family.
    """

    queue_type: QueueType
    count: int


# =========================================================================
# Device limits
# =========================================================================


@dataclass(frozen=True)
class DeviceLimits:
    """Hardware limits of a device.

    These constrain what you can do — exceeding them is a validation error.

    max_workgroup_size:        Maximum threads in one workgroup (e.g., 1024).
    max_workgroup_count:       Maximum workgroups per dispatch dimension.
    max_buffer_size:           Largest single buffer allocation.
    max_push_constant_size:    Max bytes for push constants (small inline data).
    max_descriptor_sets:       Max descriptor sets bound simultaneously.
    max_bindings_per_set:      Max bindings in one descriptor set.
    max_compute_queues:        Max compute queues.
    max_transfer_queues:       Max transfer-only queues.
    """

    max_workgroup_size: tuple[int, int, int] = (1024, 1024, 64)
    max_workgroup_count: tuple[int, int, int] = (65535, 65535, 65535)
    max_buffer_size: int = 2 * 1024 * 1024 * 1024  # 2 GB
    max_push_constant_size: int = 128
    max_descriptor_sets: int = 4
    max_bindings_per_set: int = 16
    max_compute_queues: int = 16
    max_transfer_queues: int = 2


# =========================================================================
# Memory properties
# =========================================================================


@dataclass(frozen=True)
class MemoryHeap:
    """A physical pool of memory.

    Discrete GPUs typically have two heaps:
    - VRAM (large, fast, DEVICE_LOCAL)
    - System RAM (smaller GPU-visible portion, HOST_VISIBLE)

    Unified memory devices have one heap with all flags.

    Fields:
        size:  Total size in bytes.
        flags: What memory types this heap supports.
    """

    size: int
    flags: MemoryType


@dataclass(frozen=True)
class MemoryProperties:
    """All memory heaps and types available on a device.

    Fields:
        heaps:          List of physical memory pools.
        is_unified:     True if CPU and GPU share memory (Apple).
    """

    heaps: tuple[MemoryHeap, ...]
    is_unified: bool = False


# =========================================================================
# Descriptor binding
# =========================================================================


@dataclass(frozen=True)
class DescriptorBinding:
    """One binding slot in a descriptor set layout.

    === What is a Descriptor? ===

    A descriptor is how you tell a kernel "buffer X is at binding slot 0."
    The kernel code references bindings by number, and the descriptor set
    maps those numbers to actual GPU memory addresses.

        Kernel code:    buffer = bindings[0]  // read from binding 0
        Descriptor set: binding 0 → buf_x (address 0x1000, size 4096)

    Fields:
        binding:   Slot number (0, 1, 2, ...).
        type:      "storage" (read/write) or "uniform" (read-only).
        count:     Number of buffers at this binding (usually 1).
    """

    binding: int
    type: str = "storage"
    count: int = 1


# =========================================================================
# Recorded commands — stored inside command buffers
# =========================================================================


@dataclass(frozen=True)
class RecordedCommand:
    """A single command recorded into a command buffer.

    This is a simple tagged union: the `command` field identifies the
    type, and `args` holds command-specific data.

    Example:
        RecordedCommand("dispatch", {"group_x": 4, "group_y": 1, "group_z": 1})
        RecordedCommand("copy_buffer", {"src_id": 1, "dst_id": 2, "size": 4096})
        RecordedCommand("bind_pipeline", {"pipeline_id": 0})
    """

    command: str
    args: dict[str, Any] = field(default_factory=dict)


# =========================================================================
# Memory barrier
# =========================================================================


@dataclass(frozen=True)
class MemoryBarrier:
    """A memory ordering constraint.

    === Why Barriers? ===

    GPUs have caches. When kernel A writes to a buffer and kernel B reads
    from it, the writes may still be in L2 cache — invisible to kernel B.
    A memory barrier flushes writes and invalidates read caches.

        cmd_dispatch(kernel_A)      # writes to buf
        cmd_pipeline_barrier(       # flush writes, invalidate reads
            src_access=SHADER_WRITE,
            dst_access=SHADER_READ,
        )
        cmd_dispatch(kernel_B)      # reads from buf — sees A's writes

    Fields:
        src_access: What the previous operation did (WRITE, TRANSFER_WRITE, ...).
        dst_access: What the next operation will do (READ, TRANSFER_READ, ...).
    """

    src_access: AccessFlags = AccessFlags.NONE
    dst_access: AccessFlags = AccessFlags.NONE


@dataclass(frozen=True)
class BufferBarrier:
    """A barrier targeting a specific buffer.

    Like MemoryBarrier, but scoped to one buffer. More efficient because
    the GPU only needs to flush/invalidate caches for that buffer.

    Fields:
        buffer_id:  Which buffer this barrier applies to.
        src_access: Previous access type.
        dst_access: Next access type.
        offset:     Start of affected region within the buffer.
        size:       Size of affected region (0 = whole buffer).
    """

    buffer_id: int
    src_access: AccessFlags = AccessFlags.NONE
    dst_access: AccessFlags = AccessFlags.NONE
    offset: int = 0
    size: int = 0


@dataclass(frozen=True)
class PipelineBarrier:
    """A full pipeline barrier with stage and memory constraints.

    === Anatomy of a Barrier ===

        cmd_dispatch(kernel_A)
        cmd_pipeline_barrier(PipelineBarrier(
            src_stage=COMPUTE,        ← "wait for compute to finish"
            dst_stage=COMPUTE,        ← "before starting next compute"
            memory_barriers=[         ← "and flush/invalidate memory"
                MemoryBarrier(SHADER_WRITE, SHADER_READ),
            ],
        ))
        cmd_dispatch(kernel_B)

    Fields:
        src_stage:        Wait until this stage completes.
        dst_stage:        Before this stage begins.
        memory_barriers:  Global memory ordering.
        buffer_barriers:  Per-buffer memory ordering.
    """

    src_stage: PipelineStage = PipelineStage.TOP_OF_PIPE
    dst_stage: PipelineStage = PipelineStage.BOTTOM_OF_PIPE
    memory_barriers: tuple[MemoryBarrier, ...] = ()
    buffer_barriers: tuple[BufferBarrier, ...] = ()


# =========================================================================
# RuntimeTrace — submission-level observability
# =========================================================================


@dataclass(frozen=True)
class RuntimeTrace:
    """One runtime-level event.

    === Device Traces vs Runtime Traces ===

    Device traces (Layer 6) are per-cycle: "SM 7 dispatched warp 42."
    Runtime traces are per-submission: "CB#1 submitted to compute queue."

    Together they give you the full picture — what the software did (runtime)
    and what the hardware did in response (device).

    Fields:
        timestamp_cycles:   When this event occurred (cumulative device cycles).
        event_type:         What happened (SUBMIT, FENCE, BARRIER, etc.).
        description:        Human-readable summary.
        queue_type:         Which queue was involved (if any).
        command_buffer_id:  Which command buffer (if any).
        fence_id:           Which fence (if any).
        semaphore_id:       Which semaphore (if any).
        device_traces:      Hardware traces generated by this event (if any).
    """

    timestamp_cycles: int = 0
    event_type: RuntimeEventType = RuntimeEventType.SUBMIT
    description: str = ""
    queue_type: QueueType | None = None
    command_buffer_id: int | None = None
    fence_id: int | None = None
    semaphore_id: int | None = None
    device_traces: tuple[Any, ...] = ()

    def format(self) -> str:
        """Human-readable summary.

        Example:
            [T=150 cycles] SUBMIT — CB#1 to compute queue
        """
        parts = [f"[T={self.timestamp_cycles} cycles] {self.event_type.value.upper()}"]
        if self.description:
            parts.append(f" — {self.description}")
        return "".join(parts)


# =========================================================================
# RuntimeStats — aggregate metrics
# =========================================================================


@dataclass
class RuntimeStats:
    """Aggregate statistics for the entire runtime session.

    === Key Metrics ===

    gpu_utilization = total_device_cycles / (total_device_cycles + total_idle_cycles)

    A well-utilized GPU has utilization close to 1.0 — it's always busy.
    Low utilization means the CPU is bottlenecking the GPU (not submitting
    work fast enough) or synchronization overhead is too high.

    Fields:
        total_submissions:     Number of queue.submit() calls.
        total_command_buffers: Command buffers submitted.
        total_dispatches:      Kernel dispatch commands executed.
        total_transfers:       Copy/fill commands executed.
        total_barriers:        Pipeline barriers inserted.
        total_fence_waits:     Times CPU waited on a fence.
        total_semaphore_signals: Semaphores signaled.
        total_fence_wait_cycles: Cycles CPU spent blocked on fences.
        total_allocated_bytes: Cumulative bytes allocated.
        peak_allocated_bytes:  High-water mark of memory usage.
        total_allocations:     Number of allocate() calls.
        total_frees:           Number of free() calls.
        total_maps:            Number of map() calls.
        total_device_cycles:   Cycles the device was executing work.
        total_idle_cycles:     Cycles between submissions.
        gpu_utilization:       busy / (busy + idle).
        traces:                All runtime events logged.
    """

    # Submissions
    total_submissions: int = 0
    total_command_buffers: int = 0
    total_dispatches: int = 0
    total_transfers: int = 0
    total_barriers: int = 0

    # Synchronization
    total_fence_waits: int = 0
    total_semaphore_signals: int = 0
    total_fence_wait_cycles: int = 0

    # Memory
    total_allocated_bytes: int = 0
    peak_allocated_bytes: int = 0
    total_allocations: int = 0
    total_frees: int = 0
    total_maps: int = 0

    # Timing
    total_device_cycles: int = 0
    total_idle_cycles: int = 0
    gpu_utilization: float = 0.0

    # Traces
    traces: list[RuntimeTrace] = field(default_factory=list)

    def update_utilization(self) -> None:
        """Recalculate GPU utilization from current counts."""
        total = self.total_device_cycles + self.total_idle_cycles
        if total > 0:
            self.gpu_utilization = self.total_device_cycles / total
