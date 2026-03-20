# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Protocols -- shared types for all compute unit simulators.
# ---------------------------------------------------------------------------
#
# === What is a Compute Unit? ===
#
# A compute unit is the organizational structure that wraps execution engines
# (Layer 8) with scheduling, shared memory, register files, and caches to form
# a complete computational building block. Think of it as the "factory floor":
#
#     Workers         = execution engines (warps, wavefronts, systolic arrays)
#     Floor manager   = warp/wavefront scheduler
#     Shared toolbox  = shared memory / LDS (data accessible to all teams)
#     Supply closet   = L1 cache (recent data kept nearby)
#     Filing cabinets = register file (massive, partitioned among teams)
#     Work orders     = thread blocks / work groups queued for execution
#
# Every vendor has a different name for this level of the hierarchy:
#
#     NVIDIA:   Streaming Multiprocessor (SM)
#     AMD:      Compute Unit (CU) / Work Group Processor (WGP in RDNA)
#     Intel:    Xe Core (or Subslice in older gen)
#     Google:   Matrix Multiply Unit (MXU) + Vector/Scalar units
#     Apple:    Neural Engine Core
#
# Despite the naming differences, they all serve the same purpose: take
# execution engines, add scheduling and shared resources, and present a
# coherent compute unit to the device layer above.
#
# === Duck Type Design ===
#
# Just like Layer 8 (parallel-execution-engine), we use Ruby duck typing
# to define a common interface that all compute units implement:
#
#     unit.name           -> String
#     unit.architecture   -> Symbol
#     unit.dispatch(work) -> nil
#     unit.step(edge)     -> ComputeUnitTrace
#     unit.run(...)       -> Array<ComputeUnitTrace>
#     unit.idle?          -> Boolean
#     unit.reset          -> nil
#
# Any object that responds to these methods is a valid compute unit.

module CodingAdventures
  module ComputeUnit
    # -----------------------------------------------------------------------
    # Architecture -- which vendor's compute unit this is
    # -----------------------------------------------------------------------
    #
    # Each architecture represents a fundamentally different approach to
    # organizing parallel computation. We use Ruby symbols as our "enum":
    #
    #     Architecture      | Scheduling    | Memory Model  | Execution
    #     ------------------+---------------+---------------+-----------
    #     :nvidia_sm        | Warp sched.   | Shared mem    | SIMT warps
    #     :amd_cu           | Wave sched.   | LDS           | SIMD wavefronts
    #     :google_mxu       | Compile-time  | Weight buffer | Systolic array
    #     :intel_xe_core    | Thread disp.  | SLM           | SIMD + threads
    #     :apple_ane_core   | Compiler      | SRAM + DMA    | Scheduled MAC
    ARCHITECTURES = %i[nvidia_sm amd_cu google_mxu intel_xe_core apple_ane_core].freeze

    # -----------------------------------------------------------------------
    # WarpState -- possible states of a warp in the scheduler
    # -----------------------------------------------------------------------
    #
    # A warp moves through these states during its lifetime:
    #
    #     READY --> RUNNING --> READY (if more instructions)
    #       |                     |
    #       |       +-------------+
    #       |       |
    #       +-> STALLED_MEMORY --> READY (when data arrives)
    #       +-> STALLED_BARRIER --> READY (when all warps reach barrier)
    #       +-> STALLED_DEPENDENCY --> READY (when register available)
    #       +-> COMPLETED
    #
    # The scheduler's job is to find a READY warp and issue it to an engine.
    # When a warp stalls (e.g., on a memory access), the scheduler switches
    # to another READY warp -- this is how GPUs hide latency.
    WARP_STATES = %i[
      ready running stalled_memory stalled_barrier stalled_dependency completed
    ].freeze

    # -----------------------------------------------------------------------
    # SchedulingPolicy -- how the scheduler picks which warp to issue
    # -----------------------------------------------------------------------
    #
    # Real GPUs use sophisticated scheduling policies that balance throughput,
    # fairness, and latency hiding:
    #
    #     Policy       | Strategy              | Used by
    #     -------------+-----------------------+-----------
    #     :round_robin | Fair rotation         | Teaching, some AMD
    #     :greedy      | Most-ready-first      | Throughput-focused
    #     :oldest_first| Longest-waiting-first | Fairness-focused
    #     :gto         | Same warp til stall   | NVIDIA (common)
    #     :lrr         | Skip-stalled rotation | AMD (common)
    #
    # GTO (Greedy-Then-Oldest) is particularly interesting: it keeps issuing
    # from the same warp until it stalls, then switches to the oldest ready
    # warp. This reduces context-switch overhead because warps that don't
    # stall get maximum throughput.
    SCHEDULING_POLICIES = %i[round_robin greedy oldest_first gto lrr].freeze

    # -----------------------------------------------------------------------
    # WorkItem -- a unit of parallel work dispatched to a compute unit
    # -----------------------------------------------------------------------
    #
    # In CUDA terms, this is a **thread block** (or cooperative thread array).
    # In OpenCL terms, this is a **work group**.
    # In TPU terms, this is a **tile** of a matrix operation.
    # In NPU terms, this is an **inference tile**.
    #
    # The WorkItem is the bridge between the application (which says "compute
    # this") and the hardware (which says "here are my execution engines").
    # The compute unit takes a WorkItem and decomposes it into warps/wavefronts
    # /tiles that can run on the engines.
    #
    # === Thread Block Decomposition (NVIDIA example) ===
    #
    # A WorkItem with thread_count=256 on an NVIDIA SM:
    #
    #     WorkItem(thread_count=256)
    #     +-- Warp 0:  threads 0-31    (first 32 threads)
    #     +-- Warp 1:  threads 32-63
    #     +-- Warp 2:  threads 64-95
    #     +-- ...
    #     +-- Warp 7:  threads 224-255 (last 32 threads)
    #
    # All 8 warps share the same shared memory and can synchronize with
    # __syncthreads(). This is how threads cooperate on shared data.
    #
    # Fields:
    #   work_id:              Unique identifier for this work item.
    #   program:              Instruction list for instruction-stream architectures.
    #   thread_count:         Number of parallel threads/lanes in this block.
    #   per_thread_data:      Per-thread initial register values.
    #                         per_thread_data[thread_id][register_index] = value
    #   input_data:           Activation matrix for dataflow architectures (TPU/NPU).
    #   weight_data:          Weight matrix for dataflow architectures.
    #   schedule:             MAC schedule for NPU-style architectures.
    #   shared_mem_bytes:     Shared memory requested by this work item.
    #   registers_per_thread: Registers needed per thread (for occupancy calc).
    WorkItem = Data.define(
      :work_id,
      :program,
      :thread_count,
      :per_thread_data,
      :input_data,
      :weight_data,
      :schedule,
      :shared_mem_bytes,
      :registers_per_thread
    ) do
      def initialize(
        work_id:,
        program: nil,
        thread_count: 32,
        per_thread_data: {},
        input_data: nil,
        weight_data: nil,
        schedule: nil,
        shared_mem_bytes: 0,
        registers_per_thread: 32
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # ComputeUnitTrace -- record of one clock cycle across the compute unit
    # -----------------------------------------------------------------------
    #
    # Captures scheduler decisions, engine activity, memory accesses, and
    # resource utilization -- everything needed to understand what the compute
    # unit did in one cycle.
    #
    # === Why Trace Everything? ===
    #
    # Tracing is how you learn what GPUs actually do. Without traces, a GPU
    # is a black box: data in, data out, who knows what happened inside.
    # With traces, you can see:
    #
    # - Which warp the scheduler picked and why
    # - How many warps are stalled on memory
    # - What occupancy looks like cycle by cycle
    # - Where bank conflicts happen in shared memory
    #
    # This is the same information that tools like NVIDIA Nsight Compute
    # show for real GPUs. Our traces are simpler but serve the same
    # educational purpose.
    ComputeUnitTrace = Data.define(
      :cycle,
      :unit_name,
      :architecture,
      :scheduler_action,
      :active_warps,
      :total_warps,
      :engine_traces,
      :shared_memory_used,
      :shared_memory_total,
      :register_file_used,
      :register_file_total,
      :occupancy,
      :l1_hits,
      :l1_misses
    ) do
      def initialize(
        cycle:,
        unit_name:,
        architecture:,
        scheduler_action:,
        active_warps:,
        total_warps:,
        engine_traces:,
        shared_memory_used:,
        shared_memory_total:,
        register_file_used:,
        register_file_total:,
        occupancy:,
        l1_hits: 0,
        l1_misses: 0
      )
        super
      end

      # Pretty-print the trace for educational display.
      #
      # Returns a multi-line string showing scheduler action, occupancy,
      # resource usage, and per-engine details.
      #
      # Example output:
      #
      #     [Cycle 5] SM (nvidia_sm) -- 75.0% occupancy (48/64 warps)
      #       Scheduler: issued warp 3 (GTO policy)
      #       Shared memory: 49152/98304 bytes (50.0%)
      #       Registers: 32768/65536 (50.0%)
      #       Engine 0: FMUL R2, R0, R1 -- 32/32 threads active
      #       Engine 1: (idle)
      def format
        occ_pct = "#{(occupancy * 100).round(1)}%"
        lines = [
          "[Cycle #{cycle}] #{unit_name} " \
          "(#{architecture}) " \
          "-- #{occ_pct} occupancy " \
          "(#{active_warps}/#{total_warps} warps)"
        ]
        lines << "  Scheduler: #{scheduler_action}"

        if shared_memory_total > 0
          smem_pct = (shared_memory_used.to_f / shared_memory_total * 100).round(1)
          lines << "  Shared memory: #{shared_memory_used}" \
                   "/#{shared_memory_total} bytes (#{smem_pct}%)"
        end

        if register_file_total > 0
          reg_pct = (register_file_used.to_f / register_file_total * 100).round(1)
          lines << "  Registers: #{register_file_used}" \
                   "/#{register_file_total} (#{reg_pct}%)"
        end

        engine_traces.keys.sort.each do |eid|
          lines << "  Engine #{eid}: #{engine_traces[eid].description}"
        end

        lines.join("\n")
      end
    end

    # -----------------------------------------------------------------------
    # SharedMemory -- programmer-visible scratchpad with bank conflict detection
    # -----------------------------------------------------------------------
    #
    # === What is Shared Memory? ===
    #
    # Shared memory is a small, fast, programmer-managed scratchpad that's
    # visible to all threads in a thread block. It's the GPU equivalent of
    # a team whiteboard -- everyone on the team can read and write to it.
    #
    # Performance comparison:
    #
    #     Memory Level      | Latency    | Bandwidth
    #     ------------------+------------+----------
    #     Registers         | 0 cycles   | unlimited
    #     Shared memory     | ~1-4 cycles| ~10 TB/s
    #     L1 cache          | ~30 cycles | ~2 TB/s
    #     Global (VRAM)     | ~400 cycles| ~1 TB/s
    #
    # That's a 100x latency difference between shared memory and global
    # memory. Kernels that reuse data should load it into shared memory
    # once and access it from there.
    #
    # === Bank Conflicts -- The Hidden Performance Trap ===
    #
    # Shared memory is divided into **banks** (typically 32). Each bank can
    # serve one request per cycle. If two threads access the same bank but
    # at different addresses, they **serialize** -- this is a bank conflict.
    #
    # Bank mapping (32 banks, 4 bytes per bank):
    #
    #     Address 0x00 -> Bank 0    Address 0x04 -> Bank 1    ...
    #     Address 0x80 -> Bank 0    Address 0x84 -> Bank 1    ...
    #
    # The bank for an address is: (address / bank_width) % num_banks
    class SharedMemory
      attr_reader :size, :num_banks, :bank_width

      def initialize(size:, num_banks: 32, bank_width: 4)
        @size = size
        @num_banks = num_banks
        @bank_width = bank_width
        @data = "\x00".b * size
        @total_accesses = 0
        @total_conflicts = 0
      end

      # Read a 4-byte float from shared memory.
      #
      # @param address [Integer] Byte address to read from (must be 4-byte aligned).
      # @param thread_id [Integer] Which thread is reading (for conflict tracking).
      # @return [Float] The float value at that address.
      # @raise [IndexError] If address is out of range.
      def read(address, thread_id)
        if address < 0 || address + 4 > @size
          raise IndexError,
            "Shared memory address #{address} out of range [0, #{@size})"
        end
        @total_accesses += 1
        @data[address, 4].unpack1("e")
      end

      # Write a 4-byte float to shared memory.
      #
      # @param address [Integer] Byte address to write to (must be 4-byte aligned).
      # @param value [Float] The float value to write.
      # @param thread_id [Integer] Which thread is writing (for conflict tracking).
      # @raise [IndexError] If address is out of range.
      def write(address, value, thread_id)
        if address < 0 || address + 4 > @size
          raise IndexError,
            "Shared memory address #{address} out of range [0, #{@size})"
        end
        @total_accesses += 1
        @data[address, 4] = [value].pack("e")
      end

      # Detect bank conflicts for a set of simultaneous accesses.
      #
      # Given a list of addresses (one per thread), determine which
      # accesses conflict (hit the same bank). Returns a list of conflict
      # groups -- each group is a list of thread indices that conflict.
      #
      # === How Bank Conflict Detection Works ===
      #
      # 1. Compute the bank for each address:
      #    bank = (address / bank_width) % num_banks
      #
      # 2. Group threads by bank.
      #
      # 3. Any bank accessed by more than one thread is a conflict.
      #    The threads in that bank must serialize -- taking N cycles
      #    for N conflicting accesses instead of 1 cycle.
      #
      # @param addresses [Array<Integer>] Byte addresses, one per thread.
      # @return [Array<Array<Integer>>] Conflict groups (only groups of size > 1).
      #
      # Example:
      #   smem = SharedMemory.new(size: 1024)
      #   # Threads 0 and 2 both hit bank 0 (addresses 0 and 128)
      #   smem.check_bank_conflicts([0, 4, 128, 12])
      #   # => [[0, 2]]  threads 0 and 2 conflict on bank 0
      def check_bank_conflicts(addresses)
        bank_to_threads = {}
        addresses.each_with_index do |addr, thread_idx|
          bank = (addr / @bank_width) % @num_banks
          bank_to_threads[bank] ||= []
          bank_to_threads[bank] << thread_idx
        end

        conflicts = []
        bank_to_threads.each_value do |threads|
          if threads.length > 1
            conflicts << threads
            @total_conflicts += threads.length - 1
          end
        end

        conflicts
      end

      # Clear all data and reset statistics.
      def reset
        @data = "\x00".b * @size
        @total_accesses = 0
        @total_conflicts = 0
      end

      # Total number of read/write accesses.
      def total_accesses
        @total_accesses
      end

      # Total bank conflicts detected.
      def total_conflicts
        @total_conflicts
      end
    end

    # -----------------------------------------------------------------------
    # ResourceError -- raised when dispatch fails due to resource limits
    # -----------------------------------------------------------------------
    #
    # This happens when the compute unit doesn't have enough registers,
    # shared memory, or warp slots to fit the requested work item. In real
    # CUDA, this would manifest as a launch failure or reduced occupancy.
    class ResourceError < StandardError; end
  end
end
