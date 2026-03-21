# frozen_string_literal: true

# ---------------------------------------------------------------------------
# SubsliceEngine -- Intel Xe hybrid SIMD execution engine.
# ---------------------------------------------------------------------------
#
# === What is a Subslice? ===
#
# Intel's GPU architecture uses a hierarchical organization that's different
# from both NVIDIA's SIMT warps and AMD's SIMD wavefronts. The basic unit
# is the "subslice."
#
# A subslice contains:
# - Multiple Execution Units (EUs), typically 8
# - Each EU runs multiple hardware threads, typically 7
# - Each thread processes SIMD8 (8-wide vector) instructions
#
#     +------------------------------------------------------+
#     |  Subslice                                             |
#     |                                                       |
#     |  +----------------------+  +----------------------+   |
#     |  |  EU 0                |  |  EU 1                |   |
#     |  |  +----------------+  |  |  +----------------+  |   |
#     |  |  | Thread 0: SIMD8|  |  |  | Thread 0: SIMD8|  |   |
#     |  |  | Thread 1: SIMD8|  |  |  | Thread 1: SIMD8|  |   |
#     |  |  | ...            |  |  |  | ...            |  |   |
#     |  |  | Thread 6: SIMD8|  |  |  | Thread 6: SIMD8|  |   |
#     |  |  +----------------+  |  |  +----------------+  |   |
#     |  |  Thread Arbiter      |  |  Thread Arbiter      |   |
#     |  +----------------------+  +----------------------+   |
#     |                                                       |
#     |  Shared Local Memory (SLM): 64 KB                     |
#     +------------------------------------------------------+
#
# === Why Multiple Threads Per EU? ===
#
# This is Intel's approach to latency hiding. When one thread is stalled
# (waiting for memory), the EU's thread arbiter switches to another ready
# thread. This keeps the SIMD ALU busy even when individual threads are
# blocked.
#
# === Total Parallelism ===
#
# One subslice: 8 EUs x 7 threads x 8 SIMD lanes = 448 operations per cycle.

module CodingAdventures
  module ParallelExecutionEngine
    # -----------------------------------------------------------------------
    # SubsliceConfig -- configuration for an Intel Xe-style SIMD subslice
    # -----------------------------------------------------------------------
    #
    # Real-world reference values:
    #
    #     Architecture   | EUs/subslice | Threads/EU | SIMD Width | GRF
    #     ---------------+--------------+------------+------------+-----
    #     Intel Xe-LP    | 16           | 7          | 8          | 128
    #     Intel Xe-HPG   | 16           | 8          | 8/16       | 128
    #     Our default    | 8            | 7          | 8          | 128
    SubsliceConfig = Data.define(
      :num_eus,
      :threads_per_eu,
      :simd_width,
      :grf_size,
      :slm_size,
      :float_format,
      :isa
    ) do
      def initialize(
        num_eus: 8,
        threads_per_eu: 7,
        simd_width: 8,
        grf_size: 128,
        slm_size: 65_536,
        float_format: FpArithmetic::FP32,
        isa: nil
      )
        super(
          num_eus: num_eus,
          threads_per_eu: threads_per_eu,
          simd_width: simd_width,
          grf_size: grf_size,
          slm_size: slm_size,
          float_format: float_format,
          isa: isa || GpuCore::GenericISA.new
        )
      end
    end

    # -----------------------------------------------------------------------
    # ExecutionUnit -- one EU in the subslice
    # -----------------------------------------------------------------------
    #
    # Each EU has multiple hardware threads and a thread arbiter that picks
    # one ready thread to execute per cycle. Each thread runs SIMD8
    # instructions, simulated with one GPUCore per SIMD lane.
    class ExecutionUnit
      attr_reader :eu_id, :threads

      def initialize(eu_id:, config:)
        @eu_id = eu_id
        @config = config
        @current_thread = 0

        # Each thread has `simd_width` SIMD lanes, each backed by a GPUCore.
        @threads = Array.new(config.threads_per_eu) do
          Array.new(config.simd_width) do
            GpuCore::GPUCore.new(
              isa: config.isa,
              fmt: config.float_format,
              num_registers: [config.grf_size, 256].min,
              memory_size: config.slm_size / [config.threads_per_eu, 1].max
            )
          end
        end

        @thread_active = Array.new(config.threads_per_eu, false)
        @program = []
      end

      # Load a program into all threads of this EU.
      def load_program(program)
        @program = program.dup
        @config.threads_per_eu.times do |thread_id|
          @threads[thread_id].each { |lane_core| lane_core.load_program(@program) }
          @thread_active[thread_id] = true
        end
        @current_thread = 0
      end

      # Set a register value for a specific lane of a specific thread.
      def set_thread_lane_register(thread_id, lane, reg, value)
        @threads[thread_id][lane].registers.write_float(reg, value)
      end

      # Execute one cycle using the thread arbiter.
      # Returns a hash mapping thread_id to trace description.
      def step
        traces = {}

        thread_id = find_ready_thread
        return traces if thread_id.nil?

        # Execute SIMD instruction on all lanes of the selected thread
        lane_descriptions = []
        @threads[thread_id].each do |lane_core|
          unless lane_core.halted?
            begin
              trace = lane_core.step
              lane_descriptions << trace.description
            rescue RuntimeError
              lane_descriptions << "(error)"
            end
          end
        end

        # Check if all lanes of this thread are halted
        @thread_active[thread_id] = false if @threads[thread_id].all?(&:halted?)

        if lane_descriptions.any?
          traces[thread_id] =
            "Thread #{thread_id}: SIMD#{@config.simd_width} " \
            "-- #{lane_descriptions[0]}"
        end

        traces
      end

      # True if all threads on this EU are done.
      def all_halted?
        @thread_active.none?
      end

      # Reset all threads on this EU.
      def reset
        @config.threads_per_eu.times do |thread_id|
          @threads[thread_id].each do |lane_core|
            lane_core.reset
            lane_core.load_program(@program) if @program.any?
          end
          @thread_active[thread_id] = @program.any?
        end
        @current_thread = 0
      end

      private

      # Find the next ready thread using round-robin arbitration.
      def find_ready_thread
        @config.threads_per_eu.times do |offset|
          tid = (@current_thread + offset) % @config.threads_per_eu
          if @thread_active[tid] && @threads[tid].any? { |c| !c.halted? }
            @current_thread = (tid + 1) % @config.threads_per_eu
            return tid
          end
        end
        nil
      end
    end

    # -----------------------------------------------------------------------
    # SubsliceEngine -- the hybrid SIMD execution engine
    # -----------------------------------------------------------------------
    #
    # Manages multiple EUs, each with multiple hardware threads, each
    # processing SIMD8 vectors. The thread arbiter in each EU selects
    # one ready thread per cycle.
    #
    # === Parallelism Hierarchy ===
    #
    #     Subslice (this engine)
    #     +-- EU 0
    #     |   +-- Thread 0: SIMD8 [lane0, lane1, ..., lane7]
    #     |   +-- Thread 1: SIMD8 [lane0, lane1, ..., lane7]
    #     |   +-- ... (threads_per_eu threads)
    #     +-- EU 1
    #     |   +-- Thread 0: SIMD8
    #     |   +-- ...
    #     +-- ... (num_eus EUs)
    #
    # Total parallelism = num_eus * threads_per_eu * simd_width
    class SubsliceEngine
      attr_reader :config, :eus

      def initialize(config, clock)
        @config = config
        @clock = clock
        @cycle = 0
        @program = []

        @eus = Array.new(config.num_eus) do |i|
          ExecutionUnit.new(eu_id: i, config: config)
        end

        @all_halted = false
      end

      # --- Properties (duck type interface) ---

      def name
        "SubsliceEngine"
      end

      def width
        @config.num_eus * @config.threads_per_eu * @config.simd_width
      end

      def execution_model
        :simd
      end

      def halted?
        @all_halted
      end

      # --- Program loading ---

      def load_program(program)
        @program = program.dup
        @eus.each { |eu| eu.load_program(program) }
        @all_halted = false
        @cycle = 0
      end

      # Set a register for a specific lane of a specific thread on a specific EU.
      def set_eu_thread_lane_register(eu_id, thread_id, lane, reg, value)
        @eus[eu_id].set_thread_lane_register(thread_id, lane, reg, value)
      end

      # --- Execution ---

      # Execute one cycle: each EU's arbiter picks one thread.
      def step(clock_edge)
        @cycle += 1

        return make_halted_trace if @all_halted

        all_traces = {}
        active_count = 0

        @eus.each do |eu|
          unless eu.all_halted?
            eu_traces = eu.step
            eu_traces.each do |thread_id, desc|
              flat_id = eu.eu_id * @config.threads_per_eu + thread_id
              all_traces[flat_id] = "EU#{eu.eu_id}/#{desc}"
              active_count += @config.simd_width
            end
          end
        end

        # Check if all EUs are done
        @all_halted = true if @eus.all?(&:all_halted?)

        total = width

        # Build active mask
        active_mask = Array.new(total, false)
        [active_count, total].min.times { |i| active_mask[i] = true }

        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "Subslice step -- #{active_count}/#{total} lanes active " \
                       "across #{@config.num_eus} EUs",
          unit_traces: all_traces,
          active_mask: active_mask,
          active_count: active_count,
          total_count: total,
          utilization: total > 0 ? active_count.to_f / total : 0.0
        )
      end

      # Run until all EUs are done or max_cycles reached.
      def run(max_cycles: 10_000)
        traces = []
        (1..max_cycles).each do |cycle_num|
          edge = Clock::ClockEdge.new(
            cycle: cycle_num, value: 1,
            "rising?": true, "falling?": false
          )
          trace = step(edge)
          traces << trace
          break if @all_halted
        end

        if !@all_halted && traces.length >= max_cycles
          raise RuntimeError, "SubsliceEngine: max_cycles (#{max_cycles}) reached"
        end

        traces
      end

      # Reset all EUs to initial state.
      def reset
        @eus.each(&:reset)
        @all_halted = false
        @cycle = 0
      end

      def to_s
        active_eus = @eus.count { |eu| !eu.all_halted? }
        "SubsliceEngine(eus=#{@config.num_eus}, " \
          "active_eus=#{active_eus}, halted=#{@all_halted})"
      end

      def inspect
        to_s
      end

      private

      def make_halted_trace
        total = width
        EngineTrace.new(
          cycle: @cycle,
          engine_name: name,
          execution_model: execution_model,
          description: "All EUs halted",
          unit_traces: {},
          active_mask: Array.new(total, false),
          active_count: 0,
          total_count: total,
          utilization: 0.0
        )
      end
    end
  end
end
