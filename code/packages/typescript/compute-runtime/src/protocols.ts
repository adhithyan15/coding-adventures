/**
 * Protocols -- shared types for the compute runtime.
 *
 * === What is a Compute Runtime? ===
 *
 * A compute runtime is the **software layer between user-facing APIs** (CUDA,
 * OpenCL, Metal, Vulkan) and the **hardware device simulators** (Layer 6).
 *
 * Think of it as the GPU driver's internal machinery:
 *
 *     User code:     y = alpha * x + y
 *          |
 *     API layer:     cudaMalloc / vkAllocateMemory / MTLBuffer  (Layer 4, future)
 *          |
 *     Runtime:       CommandBuffer, Queue, Fence, Memory types   (THIS LAYER)
 *          |
 *     Hardware:      NvidiaGPU.launchKernel, .step(), .run()     (Layer 6)
 *
 * The runtime manages:
 * - **Device discovery and selection** (Instance -> PhysicalDevice -> LogicalDevice)
 * - **Memory allocation with types** (DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT)
 * - **Command recording and submission** (CommandBuffer -> CommandQueue)
 * - **Synchronization** (Fence, Semaphore, Event, PipelineBarrier)
 * - **Pipeline and descriptor management** (ShaderModule -> Pipeline -> DescriptorSet)
 *
 * === Why Vulkan-Inspired? ===
 *
 * Vulkan is the most explicit GPU API -- it exposes every moving part that
 * CUDA, OpenCL, and Metal hide behind convenience wrappers. If we model at
 * Vulkan's level, building the other APIs on top becomes straightforward:
 *
 *     Vulkan:   "Here's a command buffer with barriers and descriptor sets"
 *     CUDA:     "Here's a kernel launch" (implicitly creates CB, barriers, etc.)
 *     Metal:    "Here's a command encoder" (like CB but with Apple conventions)
 *     OpenCL:   "Here's a kernel with args" (like CUDA but cross-platform)
 *
 * === Design Principle: Enums as Documentation ===
 *
 * Every enum in this file represents a real GPU concept. The variants aren't
 * arbitrary -- they map to actual hardware states, memory types, and pipeline
 * stages that exist in every GPU driver.
 */

// =========================================================================
// Device types
// =========================================================================

/**
 * What kind of accelerator this is.
 *
 * === The Three Families ===
 *
 * GPU:  General-purpose, thread-parallel. Thousands of small cores
 *       running the same program on different data (SIMT/SIMD).
 *       NVIDIA, AMD, Intel, Apple (GPU portion).
 *
 * TPU:  Dataflow, matrix-specialized. One large matrix unit (MXU)
 *       that processes tiles of matrices in a pipeline.
 *       Google TPU.
 *
 * NPU:  Neural processing unit. Fixed-function for inference,
 *       with compiler-generated execution schedules.
 *       Apple ANE, Qualcomm Hexagon, Intel NPU.
 */
export enum DeviceType {
  GPU = "gpu",
  TPU = "tpu",
  NPU = "npu",
}

// =========================================================================
// Queue types
// =========================================================================

/**
 * What kind of work a queue can accept.
 *
 * === Why Multiple Queue Types? ===
 *
 * Real GPUs have separate hardware engines for compute and data transfer.
 * While the compute engine runs a kernel, the DMA engine can copy data
 * for the next kernel in parallel. This overlap hides PCIe latency.
 *
 *     Compute Queue:   [Kernel A]--------[Kernel B]--------
 *     Transfer Queue:  ----[Upload B data]----[Upload C data]--
 *
 * Without separate queues, you'd have to wait:
 *
 *     Single Queue:    [Upload]--[Kernel A]--[Upload]--[Kernel B]--
 *                      ^^^^^^^^               ^^^^^^^^
 *                      GPU idle               GPU idle
 *
 * COMPUTE:          Can run kernels (dispatch commands).
 * TRANSFER:         Can copy data (DMA engine).
 * COMPUTE_TRANSFER: Can do both (most common on simple devices).
 */
export enum QueueType {
  COMPUTE = "compute",
  TRANSFER = "transfer",
  COMPUTE_TRANSFER = "compute_transfer",
}

// =========================================================================
// Memory types (bit flags, combinable)
// =========================================================================

/**
 * Properties of a memory allocation.
 *
 * === Memory Types Explained ===
 *
 * These are bit flags that can be combined with | (bitwise OR) to describe
 * memory with multiple properties.
 *
 * DEVICE_LOCAL:
 *     Fast GPU memory (VRAM / HBM). The GPU can access this at full
 *     bandwidth (1-3 TB/s). The CPU CANNOT directly read/write this
 *     unless HOST_VISIBLE is also set.
 *
 * HOST_VISIBLE:
 *     The CPU can map this memory and read/write it. On discrete GPUs,
 *     this is typically a small pool of system RAM accessible via PCIe.
 *
 * HOST_COHERENT:
 *     CPU writes are immediately visible to the GPU without explicit
 *     flush. More convenient but may be slower.
 *
 * HOST_CACHED:
 *     CPU reads are cached (fast read-back). Without this, every CPU
 *     read goes over PCIe -- very slow. Use for downloading results.
 *
 * === Common Combinations ===
 *
 *     DEVICE_LOCAL                          -> GPU-only, fastest
 *     HOST_VISIBLE | HOST_COHERENT          -> staging buffer for uploads
 *     HOST_VISIBLE | HOST_CACHED            -> read-back buffer for downloads
 *     DEVICE_LOCAL | HOST_VISIBLE           -> unified memory (Apple, resizable BAR)
 *     DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT -> zero-copy unified
 */
export enum MemoryType {
  DEVICE_LOCAL = 1,
  HOST_VISIBLE = 2,
  HOST_COHERENT = 4,
  HOST_CACHED = 8,
}

/**
 * Check if a memory type flags value contains a specific flag.
 *
 * Since TypeScript enums don't support Python's Flag-style `in` operator,
 * we use bitwise AND to check membership.
 */
export function hasMemoryType(flags: number, flag: MemoryType): boolean {
  return (flags & flag) !== 0;
}

// =========================================================================
// Buffer usage (bit flags, combinable)
// =========================================================================

/**
 * How a buffer will be used.
 *
 * === Why Declare Usage? ===
 *
 * Telling the GPU how a buffer will be used enables optimizations:
 * - STORAGE buffers may be placed in faster memory regions
 * - TRANSFER_SRC buffers can be DMA-aligned for faster copies
 * - UNIFORM buffers may be cached in special constant caches
 *
 * STORAGE:       Shader/kernel can read and write (SSBO in Vulkan, CUDA global mem).
 * UNIFORM:       Shader/kernel can only read. Small, fast (UBO in Vulkan).
 * TRANSFER_SRC:  Can be the source of a copy command.
 * TRANSFER_DST:  Can be the destination of a copy command.
 * INDIRECT:      Contains indirect dispatch parameters (grid dimensions in a buffer).
 */
export enum BufferUsage {
  STORAGE = 1,
  UNIFORM = 2,
  TRANSFER_SRC = 4,
  TRANSFER_DST = 8,
  INDIRECT = 16,
}

/**
 * Check if a buffer usage flags value contains a specific flag.
 */
export function hasBufferUsage(flags: number, flag: BufferUsage): boolean {
  return (flags & flag) !== 0;
}

// =========================================================================
// Pipeline stages
// =========================================================================

/**
 * Where in the GPU pipeline an operation happens.
 *
 * === The GPU Pipeline ===
 *
 * Commands flow through stages in order:
 *
 *     TOP_OF_PIPE -> TRANSFER -> COMPUTE -> BOTTOM_OF_PIPE
 *           |                                    |
 *           |          HOST (CPU access)          |
 *           +------------------------------------+
 *
 * When you create a barrier, you specify:
 * - srcStage: "wait until this stage finishes"
 * - dstStage: "before this stage starts"
 */
export enum PipelineStage {
  TOP_OF_PIPE = "top_of_pipe",
  COMPUTE = "compute",
  TRANSFER = "transfer",
  HOST = "host",
  BOTTOM_OF_PIPE = "bottom_of_pipe",
}

// =========================================================================
// Access flags (bit flags)
// =========================================================================

/**
 * What kind of memory access an operation performs.
 *
 * === Why Track Access? ===
 *
 * GPUs have caches. When a kernel writes to a buffer, the data may sit
 * in L2 cache, not yet visible to a subsequent kernel reading the same
 * buffer. A memory barrier with the right access flags ensures caches
 * are flushed (for writes) or invalidated (for reads).
 */
export enum AccessFlags {
  NONE = 0,
  SHADER_READ = 1,
  SHADER_WRITE = 2,
  TRANSFER_READ = 4,
  TRANSFER_WRITE = 8,
  HOST_READ = 16,
  HOST_WRITE = 32,
}

// =========================================================================
// Command buffer state
// =========================================================================

/**
 * Lifecycle state of a command buffer.
 *
 * === State Machine ===
 *
 *     INITIAL --begin()--> RECORDING --end()--> RECORDED
 *         ^                                        |
 *         |                                    submit()
 *         |                                        |
 *         +-------- reset() <-- COMPLETE <-- PENDING
 *                                   |
 *                                   +-- GPU finished
 */
export enum CommandBufferState {
  INITIAL = "initial",
  RECORDING = "recording",
  RECORDED = "recorded",
  PENDING = "pending",
  COMPLETE = "complete",
}

// =========================================================================
// Runtime event types
// =========================================================================

/**
 * Types of events the runtime can produce.
 *
 * These are logged in RuntimeTrace for observability -- showing the
 * software-level view of what's happening (as opposed to DeviceTrace
 * which shows hardware cycles).
 */
export enum RuntimeEventType {
  SUBMIT = "submit",
  BEGIN_EXECUTION = "begin_execution",
  END_EXECUTION = "end_execution",
  FENCE_SIGNAL = "fence_signal",
  FENCE_WAIT = "fence_wait",
  SEMAPHORE_SIGNAL = "semaphore_signal",
  SEMAPHORE_WAIT = "semaphore_wait",
  BARRIER = "barrier",
  MEMORY_ALLOC = "memory_alloc",
  MEMORY_FREE = "memory_free",
  MEMORY_MAP = "memory_map",
  MEMORY_TRANSFER = "memory_transfer",
}

// =========================================================================
// Queue family -- describes what a queue can do
// =========================================================================

/**
 * Describes a family of queues with the same capabilities.
 *
 * A GPU might have:
 * - 1 family of 16 compute queues
 * - 1 family of 2 transfer-only queues
 */
export interface QueueFamily {
  readonly queueType: QueueType;
  readonly count: number;
}

// =========================================================================
// Device limits
// =========================================================================

/**
 * Hardware limits of a device.
 *
 * These constrain what you can do -- exceeding them is a validation error.
 */
export interface DeviceLimits {
  readonly maxWorkgroupSize: readonly [number, number, number];
  readonly maxWorkgroupCount: readonly [number, number, number];
  readonly maxBufferSize: number;
  readonly maxPushConstantSize: number;
  readonly maxDescriptorSets: number;
  readonly maxBindingsPerSet: number;
  readonly maxComputeQueues: number;
  readonly maxTransferQueues: number;
}

/** Create DeviceLimits with sensible defaults. */
export function makeDeviceLimits(
  partial: Partial<DeviceLimits> = {},
): DeviceLimits {
  return {
    maxWorkgroupSize: [1024, 1024, 64],
    maxWorkgroupCount: [65535, 65535, 65535],
    maxBufferSize: 2 * 1024 * 1024 * 1024,
    maxPushConstantSize: 128,
    maxDescriptorSets: 4,
    maxBindingsPerSet: 16,
    maxComputeQueues: 16,
    maxTransferQueues: 2,
    ...partial,
  };
}

// =========================================================================
// Memory properties
// =========================================================================

/**
 * A physical pool of memory.
 *
 * Discrete GPUs typically have two heaps:
 * - VRAM (large, fast, DEVICE_LOCAL)
 * - System RAM (smaller GPU-visible portion, HOST_VISIBLE)
 */
export interface MemoryHeap {
  readonly size: number;
  readonly flags: number; // Combination of MemoryType flags
}

/**
 * All memory heaps and types available on a device.
 */
export interface MemoryProperties {
  readonly heaps: readonly MemoryHeap[];
  readonly isUnified: boolean;
}

// =========================================================================
// Descriptor binding
// =========================================================================

/**
 * One binding slot in a descriptor set layout.
 *
 * A descriptor is how you tell a kernel "buffer X is at binding slot 0."
 */
export interface DescriptorBinding {
  readonly binding: number;
  readonly type: string;
  readonly count: number;
}

/** Create a DescriptorBinding with defaults. */
export function makeDescriptorBinding(
  partial: Partial<DescriptorBinding> & { binding: number },
): DescriptorBinding {
  return {
    type: "storage",
    count: 1,
    ...partial,
  };
}

// =========================================================================
// Recorded commands -- stored inside command buffers
// =========================================================================

/**
 * A single command recorded into a command buffer.
 *
 * This is a simple tagged union: the `command` field identifies the
 * type, and `args` holds command-specific data.
 */
export interface RecordedCommand {
  readonly command: string;
  readonly args: Record<string, unknown>;
}

// =========================================================================
// Memory barrier
// =========================================================================

/**
 * A memory ordering constraint.
 *
 * GPUs have caches. When kernel A writes to a buffer and kernel B reads
 * from it, the writes may still be in L2 cache -- invisible to kernel B.
 * A memory barrier flushes writes and invalidates read caches.
 */
export interface MemoryBarrier {
  readonly srcAccess: AccessFlags;
  readonly dstAccess: AccessFlags;
}

/**
 * A barrier targeting a specific buffer.
 */
export interface BufferBarrier {
  readonly bufferId: number;
  readonly srcAccess: AccessFlags;
  readonly dstAccess: AccessFlags;
  readonly offset: number;
  readonly size: number;
}

/**
 * A full pipeline barrier with stage and memory constraints.
 *
 * === Anatomy of a Barrier ===
 *
 *     cmd_dispatch(kernel_A)
 *     cmd_pipeline_barrier(PipelineBarrier(
 *         srcStage: COMPUTE,        <- "wait for compute to finish"
 *         dstStage: COMPUTE,        <- "before starting next compute"
 *         memoryBarriers: [         <- "and flush/invalidate memory"
 *             { srcAccess: SHADER_WRITE, dstAccess: SHADER_READ },
 *         ],
 *     ))
 *     cmd_dispatch(kernel_B)
 */
export interface PipelineBarrier {
  readonly srcStage: PipelineStage;
  readonly dstStage: PipelineStage;
  readonly memoryBarriers: readonly MemoryBarrier[];
  readonly bufferBarriers: readonly BufferBarrier[];
}

/** Create a PipelineBarrier with defaults. */
export function makePipelineBarrier(
  partial: Partial<PipelineBarrier> = {},
): PipelineBarrier {
  return {
    srcStage: PipelineStage.TOP_OF_PIPE,
    dstStage: PipelineStage.BOTTOM_OF_PIPE,
    memoryBarriers: [],
    bufferBarriers: [],
    ...partial,
  };
}

// =========================================================================
// RuntimeTrace -- submission-level observability
// =========================================================================

/**
 * One runtime-level event.
 *
 * Device traces (Layer 6) are per-cycle: "SM 7 dispatched warp 42."
 * Runtime traces are per-submission: "CB#1 submitted to compute queue."
 */
export interface RuntimeTrace {
  readonly timestampCycles: number;
  readonly eventType: RuntimeEventType;
  readonly description: string;
  readonly queueType: QueueType | null;
  readonly commandBufferId: number | null;
  readonly fenceId: number | null;
  readonly semaphoreId: number | null;
  readonly deviceTraces: readonly unknown[];
}

/** Create a RuntimeTrace with defaults. */
export function makeRuntimeTrace(
  partial: Partial<RuntimeTrace> = {},
): RuntimeTrace {
  return {
    timestampCycles: 0,
    eventType: RuntimeEventType.SUBMIT,
    description: "",
    queueType: null,
    commandBufferId: null,
    fenceId: null,
    semaphoreId: null,
    deviceTraces: [],
    ...partial,
  };
}

/**
 * Format a RuntimeTrace as a human-readable string.
 *
 * Example: "[T=150 cycles] SUBMIT -- CB#1 to compute queue"
 */
export function formatRuntimeTrace(trace: RuntimeTrace): string {
  const parts = [`[T=${trace.timestampCycles} cycles] ${trace.eventType.toUpperCase()}`];
  if (trace.description) {
    parts.push(` — ${trace.description}`);
  }
  return parts.join("");
}

// =========================================================================
// RuntimeStats -- aggregate metrics
// =========================================================================

/**
 * Aggregate statistics for the entire runtime session.
 *
 * gpuUtilization = totalDeviceCycles / (totalDeviceCycles + totalIdleCycles)
 */
export interface RuntimeStats {
  // Submissions
  totalSubmissions: number;
  totalCommandBuffers: number;
  totalDispatches: number;
  totalTransfers: number;
  totalBarriers: number;

  // Synchronization
  totalFenceWaits: number;
  totalSemaphoreSignals: number;
  totalFenceWaitCycles: number;

  // Memory
  totalAllocatedBytes: number;
  peakAllocatedBytes: number;
  totalAllocations: number;
  totalFrees: number;
  totalMaps: number;

  // Timing
  totalDeviceCycles: number;
  totalIdleCycles: number;
  gpuUtilization: number;

  // Traces
  traces: RuntimeTrace[];
}

/** Create RuntimeStats with all zeros. */
export function makeRuntimeStats(): RuntimeStats {
  return {
    totalSubmissions: 0,
    totalCommandBuffers: 0,
    totalDispatches: 0,
    totalTransfers: 0,
    totalBarriers: 0,
    totalFenceWaits: 0,
    totalSemaphoreSignals: 0,
    totalFenceWaitCycles: 0,
    totalAllocatedBytes: 0,
    peakAllocatedBytes: 0,
    totalAllocations: 0,
    totalFrees: 0,
    totalMaps: 0,
    totalDeviceCycles: 0,
    totalIdleCycles: 0,
    gpuUtilization: 0.0,
    traces: [],
  };
}

/** Recalculate GPU utilization from current counts. */
export function updateUtilization(stats: RuntimeStats): void {
  const total = stats.totalDeviceCycles + stats.totalIdleCycles;
  if (total > 0) {
    stats.gpuUtilization = stats.totalDeviceCycles / total;
  }
}
