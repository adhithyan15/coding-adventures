# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Protocols -- shared types for the compute runtime.
# ---------------------------------------------------------------------------
#
# === What is a Compute Runtime? ===
#
# A compute runtime is the **software layer between user-facing APIs** (CUDA,
# OpenCL, Metal, Vulkan) and the **hardware device simulators** (Layer 6).
#
# Think of it as the GPU driver's internal machinery:
#
#     User code:     y = alpha * x + y
#          |
#     API layer:     cudaMalloc / vkAllocateMemory / MTLBuffer  (Layer 4, future)
#          |
#     Runtime:       CommandBuffer, Queue, Fence, Memory types   (THIS LAYER)
#          |
#     Hardware:      NvidiaGPU.launch_kernel, .step(), .run()    (Layer 6)
#
# The runtime manages:
# - **Device discovery and selection** (Instance -> PhysicalDevice -> LogicalDevice)
# - **Memory allocation with types** (DEVICE_LOCAL, HOST_VISIBLE, HOST_COHERENT)
# - **Command recording and submission** (CommandBuffer -> CommandQueue)
# - **Synchronization** (Fence, Semaphore, Event, PipelineBarrier)
# - **Pipeline and descriptor management** (ShaderModule -> Pipeline -> DescriptorSet)
#
# === Why Vulkan-Inspired? ===
#
# Vulkan is the most explicit GPU API -- it exposes every moving part that
# CUDA, OpenCL, and Metal hide behind convenience wrappers. If we model at
# Vulkan's level, building the other APIs on top becomes straightforward:
#
#     Vulkan:   "Here's a command buffer with barriers and descriptor sets"
#     CUDA:     "Here's a kernel launch" (implicitly creates CB, barriers, etc.)
#     Metal:    "Here's a command encoder" (like CB but with Apple conventions)
#     OpenCL:   "Here's a kernel with args" (like CUDA but cross-platform)
#
# === Design Principle: Symbols as Documentation ===
#
# Every symbol/constant in this file represents a real GPU concept. The variants
# aren't arbitrary -- they map to actual hardware states, memory types, and
# pipeline stages that exist in every GPU driver.

module CodingAdventures
  module ComputeRuntime
    # =====================================================================
    # Device types
    # =====================================================================

    # What kind of accelerator this is.
    #
    # === The Three Families ===
    #
    # :gpu  -- General-purpose, thread-parallel. Thousands of small cores
    #          running the same program on different data (SIMT/SIMD).
    #          NVIDIA, AMD, Intel, Apple (GPU portion).
    #
    # :tpu  -- Dataflow, matrix-specialized. One large matrix unit (MXU)
    #          that processes tiles of matrices in a pipeline.
    #          Google TPU.
    #
    # :npu  -- Neural processing unit. Fixed-function for inference,
    #          with compiler-generated execution schedules.
    #          Apple ANE, Qualcomm Hexagon, Intel NPU.
    DEVICE_TYPES = %i[gpu tpu npu].freeze

    # =====================================================================
    # Queue types
    # =====================================================================

    # What kind of work a queue can accept.
    #
    # === Why Multiple Queue Types? ===
    #
    # Real GPUs have separate hardware engines for compute and data transfer.
    # While the compute engine runs a kernel, the DMA engine can copy data
    # for the next kernel in parallel. This overlap hides PCIe latency.
    #
    #     Compute Queue:   [Kernel A]--------[Kernel B]--------
    #     Transfer Queue:  ----[Upload B data]----[Upload C data]--
    #
    # Without separate queues, you'd have to wait:
    #
    #     Single Queue:    [Upload]--[Kernel A]--[Upload]--[Kernel B]--
    #                      ^^^^^^^^               ^^^^^^^^
    #                      GPU idle               GPU idle
    #
    # :compute          -- Can run kernels (dispatch commands).
    # :transfer         -- Can copy data (DMA engine).
    # :compute_transfer -- Can do both (most common on simple devices).
    QUEUE_TYPES = %i[compute transfer compute_transfer].freeze

    # =====================================================================
    # Memory types (bit flags, combinable with |)
    # =====================================================================

    # Properties of a memory allocation.
    #
    # === Memory Types Explained ===
    #
    # These are bit flags that can be combined with | (bitwise OR) to describe
    # memory with multiple properties.
    #
    # DEVICE_LOCAL (1):
    #     Fast GPU memory (VRAM / HBM). The GPU can access this at full
    #     bandwidth (1-3 TB/s). The CPU CANNOT directly read/write this
    #     unless HOST_VISIBLE is also set.
    #
    # HOST_VISIBLE (2):
    #     The CPU can map this memory and read/write it. On discrete GPUs,
    #     this is typically a small pool of system RAM accessible via PCIe.
    #     On unified memory, all memory is HOST_VISIBLE.
    #
    # HOST_COHERENT (4):
    #     CPU writes are immediately visible to the GPU without explicit
    #     flush. More convenient but may be slower. Without this flag, you
    #     must call flush() after writing and invalidate() before reading.
    #
    # HOST_CACHED (8):
    #     CPU reads are cached (fast read-back). Without this, every CPU
    #     read goes over PCIe -- very slow. Use for downloading results.
    #
    # === Common Combinations ===
    #
    #     DEVICE_LOCAL                          -> GPU-only, fastest
    #     HOST_VISIBLE | HOST_COHERENT          -> staging buffer for uploads
    #     HOST_VISIBLE | HOST_CACHED            -> read-back buffer for downloads
    #     DEVICE_LOCAL | HOST_VISIBLE           -> unified memory (Apple, resizable BAR)
    #     DEVICE_LOCAL | HOST_VISIBLE | HOST_COHERENT -> zero-copy unified
    module MemoryType
      DEVICE_LOCAL  = 1
      HOST_VISIBLE  = 2
      HOST_COHERENT = 4
      HOST_CACHED   = 8
    end

    # =====================================================================
    # Buffer usage (bit flags, combinable with |)
    # =====================================================================

    # How a buffer will be used.
    #
    # === Why Declare Usage? ===
    #
    # Telling the GPU how a buffer will be used enables optimizations:
    # - STORAGE buffers may be placed in faster memory regions
    # - TRANSFER_SRC buffers can be DMA-aligned for faster copies
    # - UNIFORM buffers may be cached in special constant caches
    #
    # You must declare all intended usages at allocation time. Using a
    # buffer in a way not declared is a validation error.
    #
    # STORAGE:       Shader/kernel can read and write (SSBO in Vulkan).
    # UNIFORM:       Shader/kernel can only read. Small, fast (UBO).
    # TRANSFER_SRC:  Can be the source of a copy command.
    # TRANSFER_DST:  Can be the destination of a copy command.
    # INDIRECT:      Contains indirect dispatch parameters.
    module BufferUsage
      STORAGE      = 1
      UNIFORM      = 2
      TRANSFER_SRC = 4
      TRANSFER_DST = 8
      INDIRECT     = 16
    end

    # =====================================================================
    # Pipeline stages
    # =====================================================================

    # Where in the GPU pipeline an operation happens.
    #
    # === The GPU Pipeline ===
    #
    # Commands flow through stages in order:
    #
    #     TOP_OF_PIPE -> TRANSFER -> COMPUTE -> BOTTOM_OF_PIPE
    #           |                                    |
    #           |          HOST (CPU access)          |
    #           +------------------------------------+
    #
    # When you create a barrier, you specify:
    # - src_stage: "wait until this stage finishes"
    # - dst_stage: "before this stage starts"
    PIPELINE_STAGES = %i[top_of_pipe compute transfer host bottom_of_pipe].freeze

    # =====================================================================
    # Access flags (bit flags)
    # =====================================================================

    # What kind of memory access an operation performs.
    #
    # === Why Track Access? ===
    #
    # GPUs have caches. When a kernel writes to a buffer, the data may sit
    # in L2 cache, not yet visible to a subsequent kernel reading the same
    # buffer. A memory barrier with the right access flags ensures caches
    # are flushed (for writes) or invalidated (for reads).
    module AccessFlags
      NONE           = 0
      SHADER_READ    = 1
      SHADER_WRITE   = 2
      TRANSFER_READ  = 4
      TRANSFER_WRITE = 8
      HOST_READ      = 16
      HOST_WRITE     = 32
    end

    # =====================================================================
    # Command buffer state
    # =====================================================================

    # Lifecycle state of a command buffer.
    #
    # === State Machine ===
    #
    #     INITIAL --begin()--> RECORDING --end()--> RECORDED
    #         ^                                        |
    #         |                                    submit()
    #         |                                        |
    #         +-------- reset() <-- COMPLETE <-- PENDING
    #                                      |
    #                                      +-- GPU finished
    COMMAND_BUFFER_STATES = %i[initial recording recorded pending complete].freeze

    # =====================================================================
    # Runtime event types
    # =====================================================================

    # Types of events the runtime can produce for observability.
    RUNTIME_EVENT_TYPES = %i[
      submit begin_execution end_execution
      fence_signal fence_wait
      semaphore_signal semaphore_wait
      barrier
      memory_alloc memory_free memory_map memory_transfer
    ].freeze

    # =====================================================================
    # QueueFamily -- describes what a queue can do
    # =====================================================================
    #
    # A GPU might have:
    # - 1 family of 16 compute queues
    # - 1 family of 2 transfer-only queues
    #
    # You request queues from families when creating a logical device.
    # All queues in a family have the same capabilities.
    #
    # Fields:
    #   queue_type: Symbol -- what kind of work this family can do.
    #   count:      Integer -- how many queues are available in this family.
    QueueFamily = Data.define(:queue_type, :count)

    # =====================================================================
    # DeviceLimits -- hardware limits of a device
    # =====================================================================
    #
    # These constrain what you can do -- exceeding them is a validation error.
    #
    # max_workgroup_size:      Maximum threads in one workgroup (e.g., [1024, 1024, 64]).
    # max_workgroup_count:     Maximum workgroups per dispatch dimension.
    # max_buffer_size:         Largest single buffer allocation.
    # max_push_constant_size:  Max bytes for push constants (small inline data).
    # max_descriptor_sets:     Max descriptor sets bound simultaneously.
    # max_bindings_per_set:    Max bindings in one descriptor set.
    # max_compute_queues:      Max compute queues.
    # max_transfer_queues:     Max transfer-only queues.
    DeviceLimits = Data.define(
      :max_workgroup_size,
      :max_workgroup_count,
      :max_buffer_size,
      :max_push_constant_size,
      :max_descriptor_sets,
      :max_bindings_per_set,
      :max_compute_queues,
      :max_transfer_queues
    ) do
      def initialize(
        max_workgroup_size: [1024, 1024, 64],
        max_workgroup_count: [65535, 65535, 65535],
        max_buffer_size: 2 * 1024 * 1024 * 1024,
        max_push_constant_size: 128,
        max_descriptor_sets: 4,
        max_bindings_per_set: 16,
        max_compute_queues: 16,
        max_transfer_queues: 2
      )
        super
      end
    end

    # =====================================================================
    # MemoryHeap -- a physical pool of memory
    # =====================================================================
    #
    # Discrete GPUs typically have two heaps:
    # - VRAM (large, fast, DEVICE_LOCAL)
    # - System RAM (smaller GPU-visible portion, HOST_VISIBLE)
    #
    # Unified memory devices have one heap with all flags.
    #
    # Fields:
    #   size:  Integer -- total size in bytes.
    #   flags: Integer -- bitwise OR of MemoryType flags.
    MemoryHeap = Data.define(:size, :flags)

    # =====================================================================
    # MemoryProperties -- all memory heaps and types available on a device
    # =====================================================================
    #
    # Fields:
    #   heaps:      Array<MemoryHeap> -- list of physical memory pools.
    #   is_unified: Boolean -- true if CPU and GPU share memory (Apple).
    MemoryProperties = Data.define(:heaps, :is_unified) do
      def initialize(heaps:, is_unified: false)
        super
      end
    end

    # =====================================================================
    # DescriptorBinding -- one binding slot in a descriptor set layout
    # =====================================================================
    #
    # === What is a Descriptor? ===
    #
    # A descriptor is how you tell a kernel "buffer X is at binding slot 0."
    # The kernel code references bindings by number, and the descriptor set
    # maps those numbers to actual GPU memory addresses.
    #
    #     Kernel code:    buffer = bindings[0]  // read from binding 0
    #     Descriptor set: binding 0 -> buf_x (address 0x1000, size 4096)
    #
    # Fields:
    #   binding: Integer -- slot number (0, 1, 2, ...).
    #   type:    String  -- "storage" (read/write) or "uniform" (read-only).
    #   count:   Integer -- number of buffers at this binding (usually 1).
    DescriptorBinding = Data.define(:binding, :type, :count) do
      def initialize(binding:, type: "storage", count: 1)
        super
      end
    end

    # =====================================================================
    # RecordedCommand -- stored inside command buffers
    # =====================================================================
    #
    # A single command recorded into a command buffer.
    #
    # This is a simple tagged union: the `command` field identifies the
    # type, and `args` holds command-specific data.
    #
    # Example:
    #   RecordedCommand.new(command: "dispatch", args: { group_x: 4, group_y: 1, group_z: 1 })
    #   RecordedCommand.new(command: "copy_buffer", args: { src_id: 1, dst_id: 2, size: 4096 })
    RecordedCommand = Data.define(:command, :args) do
      def initialize(command:, args: {})
        super
      end
    end

    # =====================================================================
    # MemoryBarrier -- a memory ordering constraint
    # =====================================================================
    #
    # === Why Barriers? ===
    #
    # GPUs have caches. When kernel A writes to a buffer and kernel B reads
    # from it, the writes may still be in L2 cache -- invisible to kernel B.
    # A memory barrier flushes writes and invalidates read caches.
    #
    # Fields:
    #   src_access: Integer -- AccessFlags bits for previous operation.
    #   dst_access: Integer -- AccessFlags bits for next operation.
    MemoryBarrier = Data.define(:src_access, :dst_access) do
      def initialize(src_access: AccessFlags::NONE, dst_access: AccessFlags::NONE)
        super
      end
    end

    # =====================================================================
    # BufferBarrier -- a barrier targeting a specific buffer
    # =====================================================================
    #
    # Like MemoryBarrier, but scoped to one buffer. More efficient because
    # the GPU only needs to flush/invalidate caches for that buffer.
    #
    # Fields:
    #   buffer_id:  Integer -- which buffer this barrier applies to.
    #   src_access: Integer -- previous access type.
    #   dst_access: Integer -- next access type.
    #   offset:     Integer -- start of affected region within the buffer.
    #   size:       Integer -- size of affected region (0 = whole buffer).
    BufferBarrier = Data.define(:buffer_id, :src_access, :dst_access, :offset, :size) do
      def initialize(buffer_id:, src_access: AccessFlags::NONE,
        dst_access: AccessFlags::NONE, offset: 0, size: 0)
        super
      end
    end

    # =====================================================================
    # PipelineBarrier -- a full pipeline barrier
    # =====================================================================
    #
    # === Anatomy of a Barrier ===
    #
    #     cmd_dispatch(kernel_A)
    #     cmd_pipeline_barrier(PipelineBarrier.new(
    #         src_stage: :compute,         <- "wait for compute to finish"
    #         dst_stage: :compute,         <- "before starting next compute"
    #         memory_barriers: [           <- "and flush/invalidate memory"
    #             MemoryBarrier.new(src_access: SHADER_WRITE, dst_access: SHADER_READ),
    #         ],
    #     ))
    #     cmd_dispatch(kernel_B)
    #
    # Fields:
    #   src_stage:        Symbol -- wait until this stage completes.
    #   dst_stage:        Symbol -- before this stage begins.
    #   memory_barriers:  Array<MemoryBarrier> -- global memory ordering.
    #   buffer_barriers:  Array<BufferBarrier> -- per-buffer memory ordering.
    PipelineBarrier = Data.define(:src_stage, :dst_stage, :memory_barriers, :buffer_barriers) do
      def initialize(
        src_stage: :top_of_pipe,
        dst_stage: :bottom_of_pipe,
        memory_barriers: [],
        buffer_barriers: []
      )
        super
      end
    end

    # =====================================================================
    # RuntimeTrace -- submission-level observability
    # =====================================================================
    #
    # === Device Traces vs Runtime Traces ===
    #
    # Device traces (Layer 6) are per-cycle: "SM 7 dispatched warp 42."
    # Runtime traces are per-submission: "CB#1 submitted to compute queue."
    #
    # Together they give you the full picture -- what the software did (runtime)
    # and what the hardware did in response (device).
    #
    # Fields:
    #   timestamp_cycles:   Integer -- when this event occurred.
    #   event_type:         Symbol  -- what happened.
    #   description:        String  -- human-readable summary.
    #   queue_type:         Symbol or nil -- which queue was involved.
    #   command_buffer_id:  Integer or nil -- which CB.
    #   fence_id:           Integer or nil -- which fence.
    #   semaphore_id:       Integer or nil -- which semaphore.
    #   device_traces:      Array -- hardware traces generated by this event.
    RuntimeTrace = Data.define(
      :timestamp_cycles,
      :event_type,
      :description,
      :queue_type,
      :command_buffer_id,
      :fence_id,
      :semaphore_id,
      :device_traces
    ) do
      def initialize(
        timestamp_cycles: 0,
        event_type: :submit,
        description: "",
        queue_type: nil,
        command_buffer_id: nil,
        fence_id: nil,
        semaphore_id: nil,
        device_traces: []
      )
        super
      end

      # Human-readable summary.
      #
      # Example:
      #     [T=150 cycles] SUBMIT -- CB#1 to compute queue
      def format
        parts = "[T=#{timestamp_cycles} cycles] #{event_type.to_s.upcase}"
        parts += " -- #{description}" unless description.empty?
        parts
      end
    end

    # =====================================================================
    # RuntimeStats -- aggregate metrics
    # =====================================================================
    #
    # === Key Metrics ===
    #
    # gpu_utilization = total_device_cycles / (total_device_cycles + total_idle_cycles)
    #
    # A well-utilized GPU has utilization close to 1.0 -- it's always busy.
    # Low utilization means the CPU is bottlenecking the GPU (not submitting
    # work fast enough) or synchronization overhead is too high.
    class RuntimeStats
      attr_accessor :total_submissions, :total_command_buffers,
        :total_dispatches, :total_transfers, :total_barriers,
        :total_fence_waits, :total_semaphore_signals, :total_fence_wait_cycles,
        :total_allocated_bytes, :peak_allocated_bytes,
        :total_allocations, :total_frees, :total_maps,
        :total_device_cycles, :total_idle_cycles, :gpu_utilization,
        :traces

      def initialize
        @total_submissions = 0
        @total_command_buffers = 0
        @total_dispatches = 0
        @total_transfers = 0
        @total_barriers = 0
        @total_fence_waits = 0
        @total_semaphore_signals = 0
        @total_fence_wait_cycles = 0
        @total_allocated_bytes = 0
        @peak_allocated_bytes = 0
        @total_allocations = 0
        @total_frees = 0
        @total_maps = 0
        @total_device_cycles = 0
        @total_idle_cycles = 0
        @gpu_utilization = 0.0
        @traces = []
      end

      # Recalculate GPU utilization from current counts.
      def update_utilization
        total = @total_device_cycles + @total_idle_cycles
        @gpu_utilization = @total_device_cycles.to_f / total if total > 0
      end
    end
  end
end
