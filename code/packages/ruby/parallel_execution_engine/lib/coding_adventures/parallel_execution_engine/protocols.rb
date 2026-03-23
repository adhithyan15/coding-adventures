# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Protocols -- the unified interface for all parallel execution engines.
# ---------------------------------------------------------------------------
#
# === What is a Parallel Execution Engine? ===
#
# At Layer 9 (gpu-core), we built a single processing element -- one tiny
# compute unit that executes one instruction at a time. Useful for learning,
# but real accelerators never run just ONE core. They run THOUSANDS in parallel.
#
# Layer 8 is where parallelism happens. It takes many Layer 9 cores (or
# simpler processing elements) and orchestrates them to execute together.
# But HOW they're orchestrated differs fundamentally across architectures:
#
#     NVIDIA GPU:   32 threads in a "warp" -- each has its own registers,
#                   but they execute the same instruction (SIMT).
#
#     AMD GPU:      32/64 "lanes" in a "wavefront" -- one instruction stream,
#                   one wide vector ALU, explicit execution mask (SIMD).
#
#     Google TPU:   NxN grid of multiply-accumulate units -- data FLOWS
#                   through the array, no instructions at all (Systolic).
#
#     Apple NPU:    Array of MACs driven by a compiler-generated schedule --
#                   no runtime scheduler, just a fixed plan (Scheduled MAC).
#
#     Intel GPU:    SIMD8 execution units with multiple hardware threads --
#                   a hybrid of SIMD and multi-threading (Subslice).
#
# Despite these radical differences, ALL of them share a common interface:
# "advance one clock cycle, tell me what happened, report utilization."
# That common interface is the ParallelExecutionEngine duck type.
#
# === Flynn's Taxonomy -- A Quick Refresher ===
#
# In 1966, Michael Flynn classified computer architectures:
#
#     +-------------------+-----------------+---------------------+
#     |                   | Single Data     | Multiple Data        |
#     +-------------------+-----------------+---------------------+
#     | Single Instr.     | SISD (old CPU)  | SIMD (vector proc.) |
#     | Multiple Instr.   | MISD (rare)     | MIMD (multi-core)   |
#     +-------------------+-----------------+---------------------+
#
# Modern accelerators don't fit neatly into these boxes:
# - NVIDIA coined "SIMT" because warps are neither pure SIMD nor pure MIMD.
# - Systolic arrays don't have "instructions" at all.
# - NPU scheduled arrays are driven by static compiler schedules.
#
# Our EXECUTION_MODELS list captures these real-world execution models.

module CodingAdventures
  module ParallelExecutionEngine
    # -----------------------------------------------------------------------
    # ExecutionModel -- the five parallel execution paradigms
    # -----------------------------------------------------------------------
    #
    # We use Ruby symbols as our "enum" type. Each symbol represents a
    # fundamentally different way to organize parallel computation:
    #
    #     :simt          "Give every thread its own identity, execute together"
    #     :simd          "One instruction, wide ALU, explicit masking"
    #     :systolic      "Data flows through a grid -- no instructions needed"
    #     :scheduled_mac "Compiler decides everything -- hardware just executes"
    #     :vliw          "Pack multiple ops into one wide instruction word"
    #
    # Comparison table:
    #
    #     Model          | Has PC? | Has threads? | Divergence?     | Used by
    #     ---------------+---------+--------------+-----------------+---------
    #     :simt          | Yes*    | Yes          | HW-managed      | NVIDIA
    #     :simd          | Yes     | No (lanes)   | Explicit mask   | AMD
    #     :systolic      | No      | No           | N/A             | Google TPU
    #     :scheduled_mac | No      | No           | Compile-time    | Apple NPU
    #     :vliw          | Yes     | No           | Predicated      | Qualcomm
    #
    #     * SIMT: each thread logically has its own PC, but they usually share one.
    EXECUTION_MODELS = %i[simt simd systolic scheduled_mac vliw].freeze

    # -----------------------------------------------------------------------
    # DivergenceInfo -- tracking branch divergence (SIMT/SIMD only)
    # -----------------------------------------------------------------------
    #
    # === What is Divergence? ===
    #
    # When a group of threads/lanes encounters a branch (if/else), some may
    # take the "true" path and others the "false" path. This is called
    # "divergence" -- the threads are no longer executing in lockstep.
    #
    #     Before branch:    All 8 threads active: [T, T, T, T, T, T, T, T]
    #     Branch condition:  thread_id < 4?
    #     After branch:     Only 4 active:        [T, T, T, T, F, F, F, F]
    #                       The other 4 will run later.
    #
    # Divergence is the enemy of GPU performance. When half the threads are
    # masked off, you're wasting half your hardware. Real GPU code tries to
    # minimize divergence by ensuring threads in the same warp/wavefront
    # take the same path.
    #
    # Fields:
    #     active_mask_before: Which units were active BEFORE the branch.
    #     active_mask_after:  Which units are active AFTER the branch.
    #     reconvergence_pc:   The instruction address where all units rejoin.
    #                         -1 if not applicable (e.g., SIMD explicit mask).
    #     divergence_depth:   How many nested divergent branches we're inside.
    #                         0 means no divergence. Higher = more serialization.
    DivergenceInfo = Data.define(
      :active_mask_before,
      :active_mask_after,
      :reconvergence_pc,
      :divergence_depth
    ) do
      def initialize(
        active_mask_before:,
        active_mask_after:,
        reconvergence_pc: -1,
        divergence_depth: 0
      )
        super
      end
    end

    # -----------------------------------------------------------------------
    # DataflowInfo -- tracking data movement (Systolic only)
    # -----------------------------------------------------------------------
    #
    # === What is Dataflow Execution? ===
    #
    # In a systolic array, there are no instructions. Instead, data "flows"
    # through a grid of processing elements, like water flowing through pipes.
    # Each PE does a multiply-accumulate and passes data to its neighbor.
    #
    # This Data.define tracks the state of every PE in the grid so we can
    # visualize how data pulses through the array cycle by cycle.
    #
    # Fields:
    #     pe_states:      2D grid of PE state descriptions.
    #                     pe_states[row][col] = "acc=3.14, in=2.0"
    #     data_positions: Where each input value currently is in the array.
    #                     Maps input_id to [row, col] position.
    DataflowInfo = Data.define(:pe_states, :data_positions) do
      def initialize(pe_states:, data_positions: {})
        super
      end
    end

    # -----------------------------------------------------------------------
    # EngineTrace -- the unified trace record for all engines
    # -----------------------------------------------------------------------
    #
    # Every engine -- warp, wavefront, systolic, MAC array -- produces one
    # EngineTrace per clock cycle. This lets higher layers (and tests, and
    # visualization tools) treat all engines uniformly.
    #
    # The trace captures:
    # 1. WHAT happened (description, per-unit details)
    # 2. WHO was active (active_mask, utilization)
    # 3. HOW efficient it was (active_count / total_count)
    # 4. Engine-specific details (divergence for SIMT, dataflow for systolic)
    #
    # Example trace from a 4-thread warp:
    #
    #     EngineTrace.new(
    #       cycle: 3,
    #       engine_name: "WarpEngine",
    #       execution_model: :simt,
    #       description: "FADD R2, R0, R1 -- 3/4 threads active",
    #       unit_traces: {
    #         0 => "R2 = 1.0 + 2.0 = 3.0",
    #         1 => "R2 = 3.0 + 4.0 = 7.0",
    #         2 => "(masked -- diverged)",
    #         3 => "R2 = 5.0 + 6.0 = 11.0",
    #       },
    #       active_mask: [true, true, false, true],
    #       active_count: 3,
    #       total_count: 4,
    #       utilization: 0.75,
    #     )
    EngineTrace = Data.define(
      :cycle,
      :engine_name,
      :execution_model,
      :description,
      :unit_traces,
      :active_mask,
      :active_count,
      :total_count,
      :utilization,
      :divergence_info,
      :dataflow_info
    ) do
      def initialize(
        cycle:,
        engine_name:,
        execution_model:,
        description:,
        unit_traces:,
        active_mask:,
        active_count:,
        total_count:,
        utilization:,
        divergence_info: nil,
        dataflow_info: nil
      )
        super
      end

      # Pretty-print the trace for educational display.
      #
      # Returns a multi-line string showing the cycle, engine, utilization,
      # and per-unit details. Example output:
      #
      #     [Cycle 3] WarpEngine (SIMT) -- 75.0% utilization (3/4 active)
      #       FADD R2, R0, R1 -- 3/4 threads active
      #       Unit 0: R2 = 1.0 + 2.0 = 3.0
      #       Unit 1: R2 = 3.0 + 4.0 = 7.0
      #       Unit 2: (masked -- diverged)
      #       Unit 3: R2 = 5.0 + 6.0 = 11.0
      def format
        pct = "#{(utilization * 100).round(1)}%"
        lines = [
          "[Cycle #{cycle}] #{engine_name} " \
          "(#{execution_model.to_s.upcase}) " \
          "-- #{pct} utilization (#{active_count}/#{total_count} active)"
        ]
        lines << "  #{description}"

        unit_traces.keys.sort.each do |unit_id|
          lines << "  Unit #{unit_id}: #{unit_traces[unit_id]}"
        end

        if divergence_info
          di = divergence_info
          lines << "  Divergence: depth=#{di.divergence_depth}, " \
                   "reconvergence_pc=#{di.reconvergence_pc}"
        end

        lines.join("\n")
      end
    end

    # -----------------------------------------------------------------------
    # ParallelExecutionEngine -- the duck type all engines implement
    # -----------------------------------------------------------------------
    #
    # In Ruby, we don't need a formal Protocol/Interface class. Instead, we
    # use "duck typing" -- any object that responds to these methods is a
    # valid parallel execution engine:
    #
    #     engine.name              -> String
    #     engine.width             -> Integer
    #     engine.execution_model   -> Symbol (:simt, :simd, etc.)
    #     engine.step(clock_edge)  -> EngineTrace
    #     engine.halted?           -> Boolean
    #     engine.reset             -> nil
    #
    # This is Ruby's structural subtyping. If it steps like an engine and
    # traces like an engine, it IS an engine. No inheritance required.
  end
end
