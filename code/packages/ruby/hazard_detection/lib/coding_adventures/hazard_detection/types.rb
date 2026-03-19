# frozen_string_literal: true

# Shared data types for pipeline hazard detection.
#
# === Why These Types Exist ===
#
# A CPU pipeline is like an assembly line: each stage works on a different
# instruction simultaneously. But sometimes instructions interfere with each
# other -- one instruction needs a result that another hasn't produced yet,
# or two instructions fight over the same hardware resource.
#
# The hazard detection unit needs to know what each pipeline stage is doing
# WITHOUT knowing the specifics of the instruction set. It doesn't care
# whether you're running ARM, RISC-V, or x86 -- it only needs to know:
#
#   1. Which registers does this instruction READ?
#   2. Which register does it WRITE?
#   3. Is it a branch? Was it predicted correctly?
#   4. What hardware resources does it need (ALU, FP unit, memory)?
#
# === The Pipeline Stages (5-Stage Classic) ===
#
#     IF -> ID -> EX -> MEM -> WB
#     |     |     |      |      |
#     |     |     |      |      +-- Write Back: write result to register file
#     |     |     |      +-- Memory: load/store data from/to memory
#     |     |     +-- Execute: ALU computes result
#     |     +-- Instruction Decode: read registers, detect hazards
#     +-- Instruction Fetch: grab instruction from memory
#
# The hazard unit sits between ID and EX. It peeks at what's in each stage
# and decides: "Can ID proceed, or do we need to stall/forward/flush?"

module CodingAdventures
  module HazardDetection
    # ---------------------------------------------------------------------------
    # HazardAction -- what the hazard unit tells the pipeline to do
    # ---------------------------------------------------------------------------
    #
    # Think of these as traffic signals for the pipeline:
    #
    # NONE (Green Light):
    #   Everything is fine. The pipeline flows normally.
    #
    # FORWARD_FROM_EX (Yellow Shortcut from EX):
    #   "The value you need is right HERE in the EX stage -- grab it!"
    #
    # FORWARD_FROM_MEM (Yellow Shortcut from MEM):
    #   Same idea, but the value comes from the MEM stage.
    #
    # STALL (Red Light):
    #   "STOP! You can't proceed yet." Typically a load-use hazard.
    #
    # FLUSH (Emergency Stop):
    #   "WRONG WAY! Throw out everything!" A branch was mispredicted.
    module HazardAction
      NONE = :none
      FORWARD_FROM_EX = :forward_ex
      FORWARD_FROM_MEM = :forward_mem
      STALL = :stall
      FLUSH = :flush

      # Priority map for comparing actions. Higher number = more severe.
      PRIORITY = {
        NONE => 0,
        FORWARD_FROM_MEM => 1,
        FORWARD_FROM_EX => 2,
        STALL => 3,
        FLUSH => 4
      }.freeze
    end

    # ---------------------------------------------------------------------------
    # PipelineSlot -- what the hazard unit sees in each pipeline stage
    # ---------------------------------------------------------------------------
    #
    # This is ISA-independent. Whatever decoder is plugged in extracts this
    # info from raw instruction bits. The hazard unit only cares about register
    # numbers and resource usage, not opcodes.
    #
    # === Example: Encoding "ADD R1, R2, R3" ===
    #
    #   PipelineSlot.new(
    #     valid: true, pc: 0x1000,
    #     source_regs: [2, 3],   # reads R2 and R3
    #     dest_reg: 1,           # writes R1
    #     uses_alu: true
    #   )
    class PipelineSlot
      attr_reader :valid, :pc, :source_regs, :dest_reg, :dest_value,
        :is_branch, :branch_taken, :branch_predicted_taken,
        :mem_read, :mem_write, :uses_alu, :uses_fp

      def initialize(
        valid: false, pc: 0, source_regs: [], dest_reg: nil,
        dest_value: nil, is_branch: false, branch_taken: false,
        branch_predicted_taken: false, mem_read: false, mem_write: false,
        uses_alu: true, uses_fp: false
      )
        @valid = valid
        @pc = pc
        @source_regs = source_regs.freeze
        @dest_reg = dest_reg
        @dest_value = dest_value
        @is_branch = is_branch
        @branch_taken = branch_taken
        @branch_predicted_taken = branch_predicted_taken
        @mem_read = mem_read
        @mem_write = mem_write
        @uses_alu = uses_alu
        @uses_fp = uses_fp
      end
    end

    # ---------------------------------------------------------------------------
    # HazardResult -- the complete verdict from hazard detection
    # ---------------------------------------------------------------------------
    #
    # A simple "stall or not" boolean isn't enough. The pipeline needs to know:
    # - WHAT action to take (forward? stall? flush?)
    # - The forwarded VALUE (if forwarding)
    # - WHERE it came from (for debugging)
    # - HOW MANY cycles to stall
    # - HOW MANY stages to flush
    # - WHY (human-readable explanation)
    class HazardResult
      attr_reader :action, :forwarded_value, :forwarded_from,
        :stall_cycles, :flush_count, :reason

      def initialize(
        action: HazardAction::NONE, forwarded_value: nil,
        forwarded_from: "", stall_cycles: 0, flush_count: 0, reason: ""
      )
        @action = action
        @forwarded_value = forwarded_value
        @forwarded_from = forwarded_from
        @stall_cycles = stall_cycles
        @flush_count = flush_count
        @reason = reason
      end
    end
  end
end
