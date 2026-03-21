# frozen_string_literal: true

# Pipeline snapshot and statistics types.
#
# These types capture the pipeline's state at a moment in time and track
# cumulative performance metrics across the entire execution.

module CodingAdventures
  module CpuPipeline
    # =====================================================================
    # PipelineSnapshot -- the complete state of the pipeline at one moment
    # =====================================================================
    #
    # Captures the full state of the pipeline at a single point in time
    # (one clock cycle). Think of it as a photograph of the assembly line:
    # you can see what instruction is at each station.
    #
    # Snapshots are used for:
    #   - Debugging: "What was in the EX stage at cycle 7?"
    #   - Visualization: drawing pipeline diagrams
    #   - Testing: verifying that the pipeline behaves correctly
    #
    # Example snapshot for a 5-stage pipeline at cycle 7:
    #
    #   Cycle 7:
    #     IF:  instr@28  (fetching instruction at PC=28)
    #     ID:  ADD@24    (decoding an ADD instruction)
    #     EX:  SUB@20    (executing a SUB)
    #     MEM: ---       (bubble -- pipeline was stalled here)
    #     WB:  LDR@12    (writing back a load result)
    class PipelineSnapshot
      # The clock cycle number when this snapshot was taken.
      # Cycles count from 1 (the first call to step is cycle 1).
      attr_accessor :cycle

      # Hash mapping stage name to the token currently occupying that stage.
      # A nil value means the stage is empty (only during pipeline warmup).
      # A token with is_bubble=true means the stage holds a bubble/NOP.
      attr_accessor :stages

      # True if the pipeline was stalled during this cycle.
      # During a stall, earlier stages are frozen and a bubble is inserted.
      attr_accessor :stalled

      # True if a pipeline flush occurred during this cycle.
      # During a flush, speculative instructions are replaced with bubbles.
      attr_accessor :flushing

      # The current program counter (address of next fetch).
      attr_accessor :pc

      def initialize(cycle: 0, stages: {}, stalled: false, flushing: false, pc: 0)
        @cycle = cycle
        @stages = stages
        @stalled = stalled
        @flushing = flushing
        @pc = pc
      end

      # Returns a compact representation of the pipeline state.
      def to_s
        "[cycle #{@cycle}] PC=#{@pc} stalled=#{@stalled} flushing=#{@flushing}"
      end
    end

    # =====================================================================
    # PipelineStats -- execution statistics
    # =====================================================================
    #
    # Tracks performance statistics across the pipeline's execution.
    # These statistics are the same ones that hardware performance counters
    # measure in real CPUs.
    #
    # == Key Metrics
    #
    # IPC (Instructions Per Cycle): The most important pipeline metric.
    #
    #   IPC = instructions_completed / total_cycles
    #
    #   Ideal:       IPC = 1.0 (one instruction completes every cycle)
    #   With stalls: IPC < 1.0 (some cycles are wasted)
    #   Superscalar: IPC > 1.0 (multiple instructions per cycle)
    #
    # CPI (Cycles Per Instruction): The inverse of IPC.
    #
    #   CPI = total_cycles / instructions_completed
    #
    #   Ideal:   CPI = 1.0
    #   Typical: CPI = 1.2-2.0 for real workloads
    #
    # == Breakdown of Wasted Cycles
    #
    #   Total cycles = Useful cycles + Stall cycles + Flush cycles + Bubble cycles
    #
    #   Stall cycles:  Caused by data hazards (load-use dependencies)
    #   Flush cycles:  Caused by branch mispredictions
    #   Bubble cycles: Cycles where at least one stage held a bubble
    class PipelineStats
      # The number of clock cycles the pipeline has executed.
      attr_accessor :total_cycles

      # The number of non-bubble instructions that have reached the final
      # (writeback) stage.
      attr_accessor :instructions_completed

      # The number of cycles where the pipeline was stalled.
      # During a stall, no new instruction enters the pipeline.
      attr_accessor :stall_cycles

      # The number of cycles where a flush occurred.
      # Each flush discards one or more speculative instructions.
      attr_accessor :flush_cycles

      # Counts the total number of stage-cycles occupied by bubbles.
      # For example, if 3 stages hold bubbles for 1 cycle, that
      # contributes 3 to bubble_cycles.
      attr_accessor :bubble_cycles

      def initialize
        @total_cycles = 0
        @instructions_completed = 0
        @stall_cycles = 0
        @flush_cycles = 0
        @bubble_cycles = 0
      end

      # Returns the instructions per cycle (IPC).
      #
      # IPC is the primary measure of pipeline efficiency:
      #   - IPC = 1.0: perfect pipeline utilization (ideal)
      #   - IPC < 1.0: some cycles are wasted (stalls, flushes)
      #   - IPC > 1.0: superscalar execution (multiple instructions per cycle)
      #
      # Returns 0.0 if no cycles have been executed (avoids division by zero).
      def ipc
        return 0.0 if @total_cycles == 0

        @instructions_completed.to_f / @total_cycles
      end

      # Returns cycles per instruction (inverse of IPC).
      #
      # CPI tells you how many clock cycles each instruction takes, on average:
      #   - CPI = 1.0: one cycle per instruction (ideal for scalar pipeline)
      #   - CPI = 1.5: 50% overhead from stalls and flushes
      #   - CPI = 0.5: two instructions per cycle (superscalar)
      #
      # Returns 0.0 if no instructions have completed (avoids division by zero).
      def cpi
        return 0.0 if @instructions_completed == 0

        @total_cycles.to_f / @instructions_completed
      end

      # Returns a formatted summary of pipeline statistics.
      def to_s
        format(
          "PipelineStats{cycles=%d, completed=%d, IPC=%.3f, CPI=%.3f, stalls=%d, flushes=%d, bubbles=%d}",
          @total_cycles,
          @instructions_completed,
          ipc,
          cpi,
          @stall_cycles,
          @flush_cycles,
          @bubble_cycles
        )
      end
    end

    # =====================================================================
    # HazardAction -- what the hazard detector tells the pipeline to do
    # =====================================================================
    #
    # These are "traffic signals" for the pipeline:
    #
    #   NONE:             Green light -- pipeline flows normally
    #   STALL:            Red light -- freeze earlier stages, insert bubble
    #   FLUSH:            Emergency stop -- discard speculative instructions
    #   FORWARD_FROM_EX:  Shortcut -- grab value from EX stage output
    #   FORWARD_FROM_MEM: Shortcut -- grab value from MEM stage output
    #
    # Priority: FLUSH > STALL > FORWARD > NONE
    module HazardAction
      NONE             = :none
      FORWARD_FROM_EX  = :forward_from_ex
      FORWARD_FROM_MEM = :forward_from_mem
      STALL            = :stall
      FLUSH            = :flush

      # Returns a human-readable name for the hazard action.
      def self.to_s(action)
        case action
        when NONE             then "NONE"
        when FORWARD_FROM_EX  then "FORWARD_FROM_EX"
        when FORWARD_FROM_MEM then "FORWARD_FROM_MEM"
        when STALL            then "STALL"
        when FLUSH            then "FLUSH"
        else "UNKNOWN"
        end
      end
    end

    # =====================================================================
    # HazardResponse -- the full response from the hazard detection callback
    # =====================================================================
    #
    # Tells the pipeline what to do and provides additional context
    # (forwarded values, stall duration, flush target).
    class HazardResponse
      # The hazard action to take.
      attr_accessor :action

      # The value to forward (only used for FORWARD actions).
      attr_accessor :forward_value

      # The stage that provided the forwarded value.
      attr_accessor :forward_source

      # The number of stages to stall (typically the index of the stall point).
      attr_accessor :stall_stages

      # The number of stages to flush on a misprediction.
      attr_accessor :flush_count

      # The correct PC to fetch from after a flush.
      # Only meaningful when action == FLUSH.
      attr_accessor :redirect_pc

      def initialize(
        action: HazardAction::NONE,
        forward_value: 0,
        forward_source: "",
        stall_stages: 0,
        flush_count: 0,
        redirect_pc: 0
      )
        @action = action
        @forward_value = forward_value
        @forward_source = forward_source
        @stall_stages = stall_stages
        @flush_count = flush_count
        @redirect_pc = redirect_pc
      end
    end
  end
end
