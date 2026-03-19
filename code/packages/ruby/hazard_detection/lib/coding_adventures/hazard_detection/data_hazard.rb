# frozen_string_literal: true

# Data hazard detection -- the most common pipeline hazard.
#
# === What Is a Data Hazard? ===
#
# A data hazard occurs when an instruction depends on the result of a
# previous instruction that hasn't finished yet. In a pipelined CPU,
# multiple instructions are "in flight" simultaneously, so an instruction
# might try to READ a register before the previous instruction has
# WRITTEN its result.
#
# This module focuses on RAW (Read After Write) hazards -- the only type
# that occurs in a classic 5-stage in-order pipeline.
#
# === Resolution Strategies ===
#
# 1. FORWARDING: Wire the result directly from EX or MEM back to ID.
# 2. STALLING: Load-use hazard -- the load value isn't ready until after MEM.

module CodingAdventures
  module HazardDetection
    class DataHazardDetector
      # Check for data hazards between the ID stage and later stages.
      #
      # For each source register of the ID-stage instruction:
      #   1. Does it match EX dest_reg?
      #      a. EX is a load? -> STALL (load-use hazard)
      #      b. Otherwise -> FORWARD from EX
      #   2. Does it match MEM dest_reg? -> FORWARD from MEM
      #   3. No match? -> No hazard.
      #
      # If multiple source registers have hazards, the most severe wins:
      #   STALL > FORWARD_FROM_EX > FORWARD_FROM_MEM > NONE
      def detect(id_stage, ex_stage, mem_stage)
        # If ID stage is empty (bubble), nothing to check.
        unless id_stage.valid
          return HazardResult.new(
            action: HazardAction::NONE,
            reason: "ID stage is empty (bubble)"
          )
        end

        # No source registers means no data dependency possible.
        if id_stage.source_regs.empty?
          return HazardResult.new(
            action: HazardAction::NONE,
            reason: "instruction has no source registers"
          )
        end

        # Check each source register; track the worst hazard found.
        worst = HazardResult.new(
          action: HazardAction::NONE,
          reason: "no data dependencies detected"
        )

        id_stage.source_regs.each do |src_reg|
          result = check_single_register(src_reg, ex_stage, mem_stage)
          worst = pick_higher_priority(worst, result)
        end

        worst
      end

      private

      # Check one source register against EX and MEM destinations.
      # EX has priority over MEM (newer instruction in program order).
      def check_single_register(src_reg, ex_stage, mem_stage)
        # --- Check EX stage first (higher priority) ---
        if ex_stage.valid && !ex_stage.dest_reg.nil? && ex_stage.dest_reg == src_reg
          if ex_stage.mem_read
            # Load-use hazard: value not available until after MEM stage.
            return HazardResult.new(
              action: HazardAction::STALL,
              stall_cycles: 1,
              reason: format(
                "load-use hazard: R%d is being loaded by instruction at PC=0x%04X -- must stall 1 cycle",
                src_reg, ex_stage.pc
              )
            )
          end

          # ALU result available now -- forward from EX.
          return HazardResult.new(
            action: HazardAction::FORWARD_FROM_EX,
            forwarded_value: ex_stage.dest_value,
            forwarded_from: "EX",
            reason: format(
              "RAW hazard on R%d: forwarding value %s from EX stage (instruction at PC=0x%04X)",
              src_reg, ex_stage.dest_value.inspect, ex_stage.pc
            )
          )
        end

        # --- Check MEM stage (lower priority) ---
        if mem_stage.valid && !mem_stage.dest_reg.nil? && mem_stage.dest_reg == src_reg
          return HazardResult.new(
            action: HazardAction::FORWARD_FROM_MEM,
            forwarded_value: mem_stage.dest_value,
            forwarded_from: "MEM",
            reason: format(
              "RAW hazard on R%d: forwarding value %s from MEM stage (instruction at PC=0x%04X)",
              src_reg, mem_stage.dest_value.inspect, mem_stage.pc
            )
          )
        end

        # No conflict for this register.
        HazardResult.new(
          action: HazardAction::NONE,
          reason: format("R%d has no pending writes in EX or MEM", src_reg)
        )
      end

      # Return whichever hazard result is more severe.
      def pick_higher_priority(a, b)
        if HazardAction::PRIORITY[b.action] > HazardAction::PRIORITY[a.action]
          b
        else
          a
        end
      end
    end
  end
end
