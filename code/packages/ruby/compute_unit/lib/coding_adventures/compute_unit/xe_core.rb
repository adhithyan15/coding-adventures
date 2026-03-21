# frozen_string_literal: true

# ---------------------------------------------------------------------------
# XeCore -- Intel Xe Core simulator.
# ---------------------------------------------------------------------------
#
# === What is an Xe Core? ===
#
# Intel's Xe Core is a hybrid: it combines SIMD execution units (like AMD)
# with hardware threads (like NVIDIA), wrapped in a unique organizational
# structure. It's the building block of Intel's Arc GPUs and Data Center
# GPUs (Ponte Vecchio, Flex series).
#
# === Architecture ===
#
# An Xe Core contains:
# - **Execution Units (EUs)**: 8-16 per Xe Core, each with its own ALU
# - **Hardware threads**: 7 threads per EU for latency hiding
# - **SIMD width**: SIMD8 (or SIMD16/32 on newer architectures)
# - **SLM (Shared Local Memory)**: 64 KB, similar to NVIDIA's shared memory
# - **Thread dispatcher**: distributes work to EU threads
#
#     XeCore
#     +---------------------------------------------------------------+
#     |  Thread Dispatcher                                            |
#     |  +----------------------------------------------------------+ |
#     |  | Dispatches work to available EU thread slots               | |
#     |  +----------------------------------------------------------+ |
#     |                                                               |
#     |  +------------------+ +------------------+                    |
#     |  | EU 0             | | EU 1             |                    |
#     |  | Thread 0: SIMD8  | | Thread 0: SIMD8  |                    |
#     |  | Thread 1: SIMD8  | | Thread 1: SIMD8  |                    |
#     |  | ...              | | ...              |                    |
#     |  | Thread 6: SIMD8  | | Thread 6: SIMD8  |                    |
#     |  | Thread Arbiter   | | Thread Arbiter   |                    |
#     |  +------------------+ +------------------+                    |
#     |  ... (EU 2 through EU 15)                                     |
#     |                                                               |
#     |  Shared Local Memory (SLM): 64 KB                             |
#     |  L1 Cache: 192 KB                                             |
#     +---------------------------------------------------------------+
#
# === How Xe Differs from NVIDIA and AMD ===
#
#     NVIDIA SM:  4 schedulers, each manages many warps
#     AMD CU:     4 SIMD units, each runs wavefronts
#     Intel Xe:   8-16 EUs, each has 7 threads, each thread does SIMD8
#
# The key insight: Intel puts the thread-level parallelism INSIDE each EU
# (7 threads per EU), while NVIDIA puts it across warps (64 warps per SM).

module CodingAdventures
  module ComputeUnit
    # -----------------------------------------------------------------------
    # XeCoreConfig -- configuration for an Intel Xe Core
    # -----------------------------------------------------------------------
    #
    # Real-world Xe Core configurations:
    #
    #     Parameter           | Xe-LP (iGPU) | Xe-HPG (Arc)  | Xe-HPC
    #     --------------------+--------------+---------------+---------
    #     EUs per Xe Core     | 16           | 16            | 16
    #     Threads per EU      | 7            | 8             | 8
    #     SIMD width          | 8            | 8 (or 16)     | 8/16/32
    #     GRF per EU          | 128          | 128           | 128
    #     SLM size            | 64 KB        | 64 KB         | 128 KB
    #     L1 cache            | 192 KB       | 192 KB        | 384 KB
    XeCoreConfig = Data.define(
      :num_eus,
      :threads_per_eu,
      :simd_width,
      :grf_per_eu,
      :slm_size,
      :l1_cache_size,
      :instruction_cache_size,
      :scheduling_policy,
      :float_format,
      :isa,
      :memory_latency_cycles
    ) do
      def initialize(
        num_eus: 16,
        threads_per_eu: 7,
        simd_width: 8,
        grf_per_eu: 128,
        slm_size: 65_536,
        l1_cache_size: 196_608,
        instruction_cache_size: 65_536,
        scheduling_policy: :round_robin,
        float_format: FpArithmetic::FP32,
        isa: nil,
        memory_latency_cycles: 200
      )
        super(
          num_eus: num_eus,
          threads_per_eu: threads_per_eu,
          simd_width: simd_width,
          grf_per_eu: grf_per_eu,
          slm_size: slm_size,
          l1_cache_size: l1_cache_size,
          instruction_cache_size: instruction_cache_size,
          scheduling_policy: scheduling_policy,
          float_format: float_format,
          isa: isa || GpuCore::GenericISA.new,
          memory_latency_cycles: memory_latency_cycles
        )
      end
    end

    # -----------------------------------------------------------------------
    # XeCore -- the main Intel Xe Core simulator
    # -----------------------------------------------------------------------
    #
    # Manages Execution Units (EUs) with hardware threads, SLM, and a
    # thread dispatcher that distributes work across EU threads.
    #
    # === How Work Distribution Works ===
    #
    # When a work group is dispatched to an Xe Core:
    # 1. The thread dispatcher calculates how many EU threads are needed
    # 2. Each thread gets a portion of the work (SIMD8 of the total)
    # 3. The EU's thread arbiter round-robins among active threads
    # 4. SLM is shared among all threads in the work group
    class XeCore
      attr_reader :config, :slm, :engine

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0

        # SLM (Shared Local Memory)
        @slm = SharedMemory.new(size: config.slm_size)

        # SubsliceEngine handles the EU + thread hierarchy
        @engine = ParallelExecutionEngine::SubsliceEngine.new(
          ParallelExecutionEngine::SubsliceConfig.new(
            num_eus: config.num_eus,
            threads_per_eu: config.threads_per_eu,
            simd_width: config.simd_width,
            grf_size: config.grf_per_eu,
            slm_size: config.slm_size,
            float_format: config.float_format,
            isa: config.isa
          ),
          clock
        )

        @idle_flag = true
        @work_items = []
      end

      # --- Properties ---

      def name
        "XeCore"
      end

      def architecture
        :intel_xe_core
      end

      # True if no work remains.
      def idle?
        if @work_items.empty? && @idle_flag
          return true
        end
        @idle_flag && @engine.halted?
      end

      # --- Dispatch ---

      # Dispatch a work group to this Xe Core.
      #
      # Loads the program into the SubsliceEngine and sets per-thread
      # register values.
      #
      # @param work [WorkItem] The WorkItem to dispatch.
      def dispatch(work)
        @work_items << work
        @idle_flag = false

        @engine.load_program(work.program) if work.program

        # Set per-thread data across EUs
        work.per_thread_data.each do |global_tid, regs|
          total_lanes = @config.simd_width
          thread_total = total_lanes * @config.threads_per_eu
          eu_id = global_tid / thread_total
          remainder = global_tid % thread_total
          thread_id = remainder / total_lanes
          lane = remainder % total_lanes

          if eu_id < @config.num_eus
            regs.each do |reg, val|
              @engine.set_eu_thread_lane_register(eu_id, thread_id, lane, reg, val)
            end
          end
        end
      end

      # --- Execution ---

      # Advance one cycle.
      #
      # Delegates to the SubsliceEngine which manages EU thread arbitration.
      #
      # @param clock_edge [ClockEdge] The clock edge that triggered this step.
      # @return [ComputeUnitTrace] A trace for this cycle.
      def step(clock_edge)
        @cycle += 1

        engine_trace = @engine.step(clock_edge)

        @idle_flag = true if @engine.halted?

        active = engine_trace.active_count

        ComputeUnitTrace.new(
          cycle: @cycle,
          unit_name: name,
          architecture: architecture,
          scheduler_action: engine_trace.description,
          active_warps: active > 0 ? 1 : 0,
          total_warps: 1,
          engine_traces: {0 => engine_trace},
          shared_memory_used: 0,
          shared_memory_total: @config.slm_size,
          register_file_used: @config.grf_per_eu * @config.num_eus,
          register_file_total: @config.grf_per_eu * @config.num_eus,
          occupancy: active > 0 ? 1.0 : 0.0
        )
      end

      # Run until all work completes or max_cycles.
      def run(max_cycles: 100_000)
        traces = []
        (1..max_cycles).each do |cycle_num|
          edge = Clock::ClockEdge.new(
            cycle: cycle_num, value: 1,
            "rising?": true, "falling?": false
          )
          trace = step(edge)
          traces << trace
          break if idle?
        end
        traces
      end

      # Reset all state.
      def reset
        @engine.reset
        @slm.reset
        @work_items.clear
        @idle_flag = true
        @cycle = 0
      end

      def to_s
        "XeCore(eus=#{@config.num_eus}, " \
          "threads_per_eu=#{@config.threads_per_eu}, " \
          "idle=#{idle?})"
      end

      def inspect
        to_s
      end
    end
  end
end
