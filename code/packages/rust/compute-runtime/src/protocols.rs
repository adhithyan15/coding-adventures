//! Protocols -- shared types for the compute runtime.
//!
//! # What is a Compute Runtime?
//!
//! A compute runtime is the **software layer between user-facing APIs** (CUDA,
//! OpenCL, Metal, Vulkan) and the **hardware device simulators** (Layer 6).
//!
//! Think of it as the GPU driver's internal machinery:
//!
//! ```text
//! User code:     y = alpha * x + y
//!      |
//! API layer:     cudaMalloc / vkAllocateMemory / MTLBuffer  (Layer 4, future)
//!      |
//! Runtime:       CommandBuffer, Queue, Fence, Memory types   (THIS LAYER)
//!      |
//! Hardware:      NvidiaGPU.launch_kernel, .step(), .run()    (Layer 6)
//! ```
//!
//! The runtime manages:
//! - **Device discovery and selection** (Instance -> PhysicalDevice -> LogicalDevice)
//! - **Memory allocation with types** (DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT)
//! - **Command recording and submission** (CommandBuffer -> CommandQueue)
//! - **Synchronization** (Fence, Semaphore, Event, PipelineBarrier)
//! - **Pipeline and descriptor management** (ShaderModule -> Pipeline -> DescriptorSet)
//!
//! # Why Vulkan-Inspired?
//!
//! Vulkan is the most explicit GPU API -- it exposes every moving part that
//! CUDA, OpenCL, and Metal hide behind convenience wrappers. If we model at
//! Vulkan's level, building the other APIs on top becomes straightforward:
//!
//! ```text
//! Vulkan:   "Here's a command buffer with barriers and descriptor sets"
//! CUDA:     "Here's a kernel launch" (implicitly creates CB, barriers, etc.)
//! Metal:    "Here's a command encoder" (like CB but with Apple conventions)
//! OpenCL:   "Here's a kernel with args" (like CUDA but cross-platform)
//! ```

use std::collections::HashMap;

use bitflags::bitflags;

// =========================================================================
// Device types
// =========================================================================

/// What kind of accelerator this is.
///
/// # The Three Families
///
/// - **GPU**: General-purpose, thread-parallel. Thousands of small cores
///   running the same program on different data (SIMT/SIMD).
///   NVIDIA, AMD, Intel, Apple (GPU portion).
///
/// - **TPU**: Dataflow, matrix-specialized. One large matrix unit (MXU)
///   that processes tiles of matrices in a pipeline.
///   Google TPU.
///
/// - **NPU**: Neural processing unit. Fixed-function for inference,
///   with compiler-generated execution schedules.
///   Apple ANE, Qualcomm Hexagon, Intel NPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    GPU,
    TPU,
    NPU,
}

// =========================================================================
// Queue types
// =========================================================================

/// What kind of work a queue can accept.
///
/// # Why Multiple Queue Types?
///
/// Real GPUs have separate hardware engines for compute and data transfer.
/// While the compute engine runs a kernel, the DMA engine can copy data
/// for the next kernel in parallel. This overlap hides PCIe latency.
///
/// ```text
/// Compute Queue:   [Kernel A]────────[Kernel B]────────
/// Transfer Queue:  ────[Upload B data]────[Upload C data]──
/// ```
///
/// Without separate queues, you'd have to wait:
///
/// ```text
/// Single Queue:    [Upload]──[Kernel A]──[Upload]──[Kernel B]──
///                  ^^^^^^^^               ^^^^^^^^
///                  GPU idle               GPU idle
/// ```
///
/// - `Compute`: Can run kernels (dispatch commands).
/// - `Transfer`: Can copy data (DMA engine).
/// - `ComputeTransfer`: Can do both (most common on simple devices).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueType {
    Compute,
    Transfer,
    ComputeTransfer,
}

impl QueueType {
    /// Convert a string tag to a QueueType.
    ///
    /// Returns `ComputeTransfer` for unknown strings.
    pub fn from_str_tag(s: &str) -> Self {
        match s {
            "compute" => QueueType::Compute,
            "transfer" => QueueType::Transfer,
            _ => QueueType::ComputeTransfer,
        }
    }

    /// Convert to a string tag.
    pub fn as_str(&self) -> &'static str {
        match self {
            QueueType::Compute => "compute",
            QueueType::Transfer => "transfer",
            QueueType::ComputeTransfer => "compute_transfer",
        }
    }
}

// =========================================================================
// Memory types (flags, combinable)
// =========================================================================

bitflags! {
    /// Properties of a memory allocation.
    ///
    /// # Memory Types Explained
    ///
    /// These are flags that can be combined with `|` (bitwise OR) to describe
    /// memory with multiple properties.
    ///
    /// - `DEVICE_LOCAL`: Fast GPU memory (VRAM / HBM). The GPU can access this at full
    ///   bandwidth (1-3 TB/s). The CPU CANNOT directly read/write this unless
    ///   HOST_VISIBLE is also set.
    ///
    /// - `HOST_VISIBLE`: The CPU can map this memory and read/write it. On discrete GPUs,
    ///   this is typically a small pool of system RAM accessible via PCIe.
    ///   On unified memory, all memory is HOST_VISIBLE.
    ///
    /// - `HOST_COHERENT`: CPU writes are immediately visible to the GPU without explicit
    ///   flush. More convenient but may be slower.
    ///
    /// - `HOST_CACHED`: CPU reads are cached (fast read-back). Use for downloading results.
    ///
    /// # Common Combinations
    ///
    /// ```text
    /// DEVICE_LOCAL                          -> GPU-only, fastest
    /// HOST_VISIBLE | HOST_COHERENT          -> staging buffer for uploads
    /// HOST_VISIBLE | HOST_CACHED            -> read-back buffer for downloads
    /// DEVICE_LOCAL | HOST_VISIBLE           -> unified memory (Apple, resizable BAR)
    /// DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT -> zero-copy unified
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryType: u32 {
        const DEVICE_LOCAL  = 0b0001;
        const HOST_VISIBLE  = 0b0010;
        const HOST_COHERENT = 0b0100;
        const HOST_CACHED   = 0b1000;
    }
}

// =========================================================================
// Buffer usage (flags, combinable)
// =========================================================================

bitflags! {
    /// How a buffer will be used.
    ///
    /// # Why Declare Usage?
    ///
    /// Telling the GPU how a buffer will be used enables optimizations:
    /// - STORAGE buffers may be placed in faster memory regions
    /// - TRANSFER_SRC buffers can be DMA-aligned for faster copies
    /// - UNIFORM buffers may be cached in special constant caches
    ///
    /// You must declare all intended usages at allocation time. Using a
    /// buffer in a way not declared is a validation error.
    ///
    /// - `STORAGE`: Shader/kernel can read and write (SSBO in Vulkan, CUDA global mem).
    /// - `UNIFORM`: Shader/kernel can only read. Small, fast (UBO in Vulkan).
    /// - `TRANSFER_SRC`: Can be the source of a copy command.
    /// - `TRANSFER_DST`: Can be the destination of a copy command.
    /// - `INDIRECT`: Contains indirect dispatch parameters (grid dimensions in a buffer).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BufferUsage: u32 {
        const STORAGE      = 0b00001;
        const UNIFORM      = 0b00010;
        const TRANSFER_SRC = 0b00100;
        const TRANSFER_DST = 0b01000;
        const INDIRECT     = 0b10000;
    }
}

// =========================================================================
// Pipeline stages
// =========================================================================

/// Where in the GPU pipeline an operation happens.
///
/// Commands flow through stages in order:
///
/// ```text
/// TOP_OF_PIPE -> TRANSFER -> COMPUTE -> BOTTOM_OF_PIPE
///       |                                    |
///       |          HOST (CPU access)          |
///       +------------------------------------+
/// ```
///
/// When you create a barrier, you specify:
/// - `src_stage`: "wait until this stage finishes"
/// - `dst_stage`: "before this stage starts"
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    TopOfPipe,
    Compute,
    Transfer,
    Host,
    BottomOfPipe,
}

impl PipelineStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            PipelineStage::TopOfPipe => "top_of_pipe",
            PipelineStage::Compute => "compute",
            PipelineStage::Transfer => "transfer",
            PipelineStage::Host => "host",
            PipelineStage::BottomOfPipe => "bottom_of_pipe",
        }
    }
}

// =========================================================================
// Access flags
// =========================================================================

bitflags! {
    /// What kind of memory access an operation performs.
    ///
    /// GPUs have caches. When a kernel writes to a buffer, the data may sit
    /// in L2 cache, not yet visible to a subsequent kernel reading the same
    /// buffer. A memory barrier with the right access flags ensures caches
    /// are flushed (for writes) or invalidated (for reads).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessFlags: u32 {
        const NONE           = 0;
        const SHADER_READ    = 0b000001;
        const SHADER_WRITE   = 0b000010;
        const TRANSFER_READ  = 0b000100;
        const TRANSFER_WRITE = 0b001000;
        const HOST_READ      = 0b010000;
        const HOST_WRITE     = 0b100000;
    }
}

// =========================================================================
// Command buffer state
// =========================================================================

/// Lifecycle state of a command buffer.
///
/// # State Machine
///
/// ```text
/// INITIAL --begin()--> RECORDING --end()--> RECORDED
///     ^                                        |
///     |                                    submit()
///     |                                        |
///     +-------- reset() <-- COMPLETE <-- PENDING
///                                 |
///                                 +-- GPU finished
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferState {
    Initial,
    Recording,
    Recorded,
    Pending,
    Complete,
}

impl CommandBufferState {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandBufferState::Initial => "initial",
            CommandBufferState::Recording => "recording",
            CommandBufferState::Recorded => "recorded",
            CommandBufferState::Pending => "pending",
            CommandBufferState::Complete => "complete",
        }
    }
}

// =========================================================================
// Runtime event types
// =========================================================================

/// Types of events the runtime can produce.
///
/// These are logged in RuntimeTrace for observability -- showing the
/// software-level view of what's happening (as opposed to DeviceTrace
/// which shows hardware cycles).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEventType {
    Submit,
    BeginExecution,
    EndExecution,
    FenceSignal,
    FenceWait,
    SemaphoreSignal,
    SemaphoreWait,
    Barrier,
    MemoryAlloc,
    MemoryFree,
    MemoryMap,
    MemoryTransfer,
}

impl RuntimeEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeEventType::Submit => "SUBMIT",
            RuntimeEventType::BeginExecution => "BEGIN_EXECUTION",
            RuntimeEventType::EndExecution => "END_EXECUTION",
            RuntimeEventType::FenceSignal => "FENCE_SIGNAL",
            RuntimeEventType::FenceWait => "FENCE_WAIT",
            RuntimeEventType::SemaphoreSignal => "SEMAPHORE_SIGNAL",
            RuntimeEventType::SemaphoreWait => "SEMAPHORE_WAIT",
            RuntimeEventType::Barrier => "BARRIER",
            RuntimeEventType::MemoryAlloc => "MEMORY_ALLOC",
            RuntimeEventType::MemoryFree => "MEMORY_FREE",
            RuntimeEventType::MemoryMap => "MEMORY_MAP",
            RuntimeEventType::MemoryTransfer => "MEMORY_TRANSFER",
        }
    }
}

// =========================================================================
// Queue family -- describes what a queue can do
// =========================================================================

/// Describes a family of queues with the same capabilities.
///
/// A GPU might have:
/// - 1 family of 16 compute queues
/// - 1 family of 2 transfer-only queues
///
/// You request queues from families when creating a logical device.
/// All queues in a family have the same capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueFamily {
    pub queue_type: QueueType,
    pub count: usize,
}

// =========================================================================
// Device limits
// =========================================================================

/// Hardware limits of a device.
///
/// These constrain what you can do -- exceeding them is a validation error.
#[derive(Debug, Clone)]
pub struct DeviceLimits {
    pub max_workgroup_size: (usize, usize, usize),
    pub max_workgroup_count: (usize, usize, usize),
    pub max_buffer_size: usize,
    pub max_push_constant_size: usize,
    pub max_descriptor_sets: usize,
    pub max_bindings_per_set: usize,
    pub max_compute_queues: usize,
    pub max_transfer_queues: usize,
}

impl Default for DeviceLimits {
    fn default() -> Self {
        Self {
            max_workgroup_size: (1024, 1024, 64),
            max_workgroup_count: (65535, 65535, 65535),
            max_buffer_size: 2 * 1024 * 1024 * 1024, // 2 GB
            max_push_constant_size: 128,
            max_descriptor_sets: 4,
            max_bindings_per_set: 16,
            max_compute_queues: 16,
            max_transfer_queues: 2,
        }
    }
}

// =========================================================================
// Memory properties
// =========================================================================

/// A physical pool of memory.
///
/// Discrete GPUs typically have two heaps:
/// - VRAM (large, fast, DEVICE_LOCAL)
/// - System RAM (smaller GPU-visible portion, HOST_VISIBLE)
///
/// Unified memory devices have one heap with all flags.
#[derive(Debug, Clone)]
pub struct MemoryHeap {
    pub size: u64,
    pub flags: MemoryType,
}

/// All memory heaps and types available on a device.
#[derive(Debug, Clone)]
pub struct MemoryProperties {
    pub heaps: Vec<MemoryHeap>,
    pub is_unified: bool,
}

// =========================================================================
// Descriptor binding
// =========================================================================

/// One binding slot in a descriptor set layout.
///
/// # What is a Descriptor?
///
/// A descriptor is how you tell a kernel "buffer X is at binding slot 0."
/// The kernel code references bindings by number, and the descriptor set
/// maps those numbers to actual GPU memory addresses.
///
/// ```text
/// Kernel code:    buffer = bindings[0]  // read from binding 0
/// Descriptor set: binding 0 -> buf_x (address 0x1000, size 4096)
/// ```
#[derive(Debug, Clone)]
pub struct DescriptorBinding {
    pub binding: usize,
    pub binding_type: String,
    pub count: usize,
}

impl DescriptorBinding {
    pub fn new(binding: usize) -> Self {
        Self {
            binding,
            binding_type: "storage".to_string(),
            count: 1,
        }
    }

    pub fn with_type(mut self, binding_type: &str) -> Self {
        self.binding_type = binding_type.to_string();
        self
    }
}

// =========================================================================
// Recorded commands -- stored inside command buffers
// =========================================================================

/// A single command recorded into a command buffer.
///
/// This is a simple tagged union: the `command` field identifies the
/// type, and `args` holds command-specific data as string keys to values.
#[derive(Debug, Clone)]
pub struct RecordedCommand {
    pub command: String,
    pub args: HashMap<String, CommandArg>,
}

/// A value that can be stored in a RecordedCommand's args map.
///
/// Because different command types store different kinds of data (integers,
/// bytes, strings), we use an enum rather than a single type.
#[derive(Debug, Clone)]
pub enum CommandArg {
    Int(i64),
    UInt(u64),
    Usize(usize),
    Str(String),
    Bytes(Vec<u8>),
}

impl CommandArg {
    pub fn as_usize(&self) -> usize {
        match self {
            CommandArg::Usize(v) => *v,
            CommandArg::Int(v) => *v as usize,
            CommandArg::UInt(v) => *v as usize,
            _ => panic!("CommandArg is not a numeric type"),
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            CommandArg::Int(v) => *v,
            CommandArg::UInt(v) => *v as i64,
            CommandArg::Usize(v) => *v as i64,
            _ => panic!("CommandArg is not a numeric type"),
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            CommandArg::Int(v) => (*v & 0xFF) as u8,
            CommandArg::UInt(v) => (*v & 0xFF) as u8,
            CommandArg::Usize(v) => (*v & 0xFF) as u8,
            _ => panic!("CommandArg is not a numeric type"),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            CommandArg::Bytes(v) => v,
            _ => panic!("CommandArg is not Bytes"),
        }
    }

    pub fn as_str_val(&self) -> &str {
        match self {
            CommandArg::Str(v) => v,
            _ => panic!("CommandArg is not Str"),
        }
    }
}

// =========================================================================
// Memory barrier
// =========================================================================

/// A memory ordering constraint.
///
/// # Why Barriers?
///
/// GPUs have caches. When kernel A writes to a buffer and kernel B reads
/// from it, the writes may still be in L2 cache -- invisible to kernel B.
/// A memory barrier flushes writes and invalidates read caches.
#[derive(Debug, Clone)]
pub struct MemoryBarrier {
    pub src_access: AccessFlags,
    pub dst_access: AccessFlags,
}

impl Default for MemoryBarrier {
    fn default() -> Self {
        Self {
            src_access: AccessFlags::NONE,
            dst_access: AccessFlags::NONE,
        }
    }
}

/// A barrier targeting a specific buffer.
///
/// Like MemoryBarrier, but scoped to one buffer. More efficient because
/// the GPU only needs to flush/invalidate caches for that buffer.
#[derive(Debug, Clone)]
pub struct BufferBarrier {
    pub buffer_id: usize,
    pub src_access: AccessFlags,
    pub dst_access: AccessFlags,
    pub offset: usize,
    pub size: usize,
}

impl BufferBarrier {
    pub fn new(buffer_id: usize) -> Self {
        Self {
            buffer_id,
            src_access: AccessFlags::NONE,
            dst_access: AccessFlags::NONE,
            offset: 0,
            size: 0,
        }
    }
}

/// A full pipeline barrier with stage and memory constraints.
///
/// # Anatomy of a Barrier
///
/// ```text
/// cmd_dispatch(kernel_A)
/// cmd_pipeline_barrier(PipelineBarrier {
///     src_stage: Compute,        // "wait for compute to finish"
///     dst_stage: Compute,        // "before starting next compute"
///     memory_barriers: vec![     // "and flush/invalidate memory"
///         MemoryBarrier { src: SHADER_WRITE, dst: SHADER_READ },
///     ],
/// })
/// cmd_dispatch(kernel_B)
/// ```
#[derive(Debug, Clone)]
pub struct PipelineBarrier {
    pub src_stage: PipelineStage,
    pub dst_stage: PipelineStage,
    pub memory_barriers: Vec<MemoryBarrier>,
    pub buffer_barriers: Vec<BufferBarrier>,
}

impl Default for PipelineBarrier {
    fn default() -> Self {
        Self {
            src_stage: PipelineStage::TopOfPipe,
            dst_stage: PipelineStage::BottomOfPipe,
            memory_barriers: Vec::new(),
            buffer_barriers: Vec::new(),
        }
    }
}

// =========================================================================
// RuntimeTrace -- submission-level observability
// =========================================================================

/// One runtime-level event.
///
/// # Device Traces vs Runtime Traces
///
/// Device traces (Layer 6) are per-cycle: "SM 7 dispatched warp 42."
/// Runtime traces are per-submission: "CB#1 submitted to compute queue."
///
/// Together they give you the full picture -- what the software did (runtime)
/// and what the hardware did in response (device).
#[derive(Debug, Clone)]
pub struct RuntimeTrace {
    pub timestamp_cycles: u64,
    pub event_type: RuntimeEventType,
    pub description: String,
    pub queue_type: Option<QueueType>,
    pub command_buffer_id: Option<usize>,
    pub fence_id: Option<usize>,
    pub semaphore_id: Option<usize>,
}

impl RuntimeTrace {
    pub fn new(event_type: RuntimeEventType, description: &str) -> Self {
        Self {
            timestamp_cycles: 0,
            event_type,
            description: description.to_string(),
            queue_type: None,
            command_buffer_id: None,
            fence_id: None,
            semaphore_id: None,
        }
    }

    /// Human-readable summary.
    ///
    /// Example: `[T=150 cycles] SUBMIT -- CB#1 to compute queue`
    pub fn format(&self) -> String {
        let mut s = format!(
            "[T={} cycles] {}",
            self.timestamp_cycles,
            self.event_type.as_str()
        );
        if !self.description.is_empty() {
            s.push_str(&format!(" -- {}", self.description));
        }
        s
    }
}

// =========================================================================
// RuntimeStats -- aggregate metrics
// =========================================================================

/// Aggregate statistics for the entire runtime session.
///
/// # Key Metrics
///
/// `gpu_utilization = total_device_cycles / (total_device_cycles + total_idle_cycles)`
///
/// A well-utilized GPU has utilization close to 1.0 -- it's always busy.
/// Low utilization means the CPU is bottlenecking the GPU or synchronization
/// overhead is too high.
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    // Submissions
    pub total_submissions: usize,
    pub total_command_buffers: usize,
    pub total_dispatches: usize,
    pub total_transfers: usize,
    pub total_barriers: usize,

    // Synchronization
    pub total_fence_waits: usize,
    pub total_semaphore_signals: usize,
    pub total_fence_wait_cycles: u64,

    // Memory
    pub total_allocated_bytes: usize,
    pub peak_allocated_bytes: usize,
    pub total_allocations: usize,
    pub total_frees: usize,
    pub total_maps: usize,

    // Timing
    pub total_device_cycles: u64,
    pub total_idle_cycles: u64,
    pub gpu_utilization: f64,

    // Traces
    pub traces: Vec<RuntimeTrace>,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self {
            total_submissions: 0,
            total_command_buffers: 0,
            total_dispatches: 0,
            total_transfers: 0,
            total_barriers: 0,
            total_fence_waits: 0,
            total_semaphore_signals: 0,
            total_fence_wait_cycles: 0,
            total_allocated_bytes: 0,
            peak_allocated_bytes: 0,
            total_allocations: 0,
            total_frees: 0,
            total_maps: 0,
            total_device_cycles: 0,
            total_idle_cycles: 0,
            gpu_utilization: 0.0,
            traces: Vec::new(),
        }
    }
}

impl RuntimeStats {
    /// Recalculate GPU utilization from current counts.
    pub fn update_utilization(&mut self) {
        let total = self.total_device_cycles + self.total_idle_cycles;
        if total > 0 {
            self.gpu_utilization = self.total_device_cycles as f64 / total as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type() {
        assert_eq!(DeviceType::GPU, DeviceType::GPU);
        assert_ne!(DeviceType::GPU, DeviceType::TPU);
    }

    #[test]
    fn test_queue_type_from_str() {
        assert_eq!(QueueType::from_str_tag("compute"), QueueType::Compute);
        assert_eq!(QueueType::from_str_tag("transfer"), QueueType::Transfer);
        assert_eq!(
            QueueType::from_str_tag("unknown"),
            QueueType::ComputeTransfer
        );
    }

    #[test]
    fn test_memory_type_flags() {
        let unified = MemoryType::DEVICE_LOCAL | MemoryType::HOST_VISIBLE | MemoryType::HOST_COHERENT;
        assert!(unified.contains(MemoryType::DEVICE_LOCAL));
        assert!(unified.contains(MemoryType::HOST_VISIBLE));
        assert!(unified.contains(MemoryType::HOST_COHERENT));
        assert!(!unified.contains(MemoryType::HOST_CACHED));
    }

    #[test]
    fn test_buffer_usage_flags() {
        let usage = BufferUsage::STORAGE | BufferUsage::TRANSFER_DST;
        assert!(usage.contains(BufferUsage::STORAGE));
        assert!(usage.contains(BufferUsage::TRANSFER_DST));
        assert!(!usage.contains(BufferUsage::UNIFORM));
    }

    #[test]
    fn test_access_flags() {
        let access = AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE;
        assert!(access.contains(AccessFlags::SHADER_READ));
        assert!(!access.contains(AccessFlags::HOST_READ));
    }

    #[test]
    fn test_command_buffer_state() {
        assert_eq!(CommandBufferState::Initial.as_str(), "initial");
        assert_eq!(CommandBufferState::Recording.as_str(), "recording");
    }

    #[test]
    fn test_device_limits_default() {
        let limits = DeviceLimits::default();
        assert_eq!(limits.max_workgroup_size, (1024, 1024, 64));
        assert_eq!(limits.max_push_constant_size, 128);
    }

    #[test]
    fn test_runtime_trace_format() {
        let trace = RuntimeTrace {
            timestamp_cycles: 150,
            event_type: RuntimeEventType::Submit,
            description: "CB#1 to compute queue".to_string(),
            queue_type: Some(QueueType::Compute),
            command_buffer_id: None,
            fence_id: None,
            semaphore_id: None,
        };
        let formatted = trace.format();
        assert!(formatted.contains("[T=150 cycles]"));
        assert!(formatted.contains("SUBMIT"));
        assert!(formatted.contains("CB#1 to compute queue"));
    }

    #[test]
    fn test_runtime_stats_utilization() {
        let mut stats = RuntimeStats::default();
        stats.total_device_cycles = 800;
        stats.total_idle_cycles = 200;
        stats.update_utilization();
        assert!((stats.gpu_utilization - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_pipeline_barrier_default() {
        let barrier = PipelineBarrier::default();
        assert_eq!(barrier.src_stage, PipelineStage::TopOfPipe);
        assert_eq!(barrier.dst_stage, PipelineStage::BottomOfPipe);
        assert!(barrier.memory_barriers.is_empty());
    }

    #[test]
    fn test_descriptor_binding() {
        let b = DescriptorBinding::new(0).with_type("uniform");
        assert_eq!(b.binding, 0);
        assert_eq!(b.binding_type, "uniform");
        assert_eq!(b.count, 1);
    }

    #[test]
    fn test_command_arg_conversions() {
        assert_eq!(CommandArg::Usize(42).as_usize(), 42);
        assert_eq!(CommandArg::Int(10).as_i64(), 10);
        assert_eq!(CommandArg::UInt(0xFF).as_u8(), 0xFF);
        assert_eq!(CommandArg::Bytes(vec![1, 2, 3]).as_bytes(), &[1, 2, 3]);
        assert_eq!(CommandArg::Str("hello".to_string()).as_str_val(), "hello");
    }
}
