# frozen_string_literal: true

# Structural hazard detection -- when hardware resources collide.
#
# === What Is a Structural Hazard? ===
#
# A structural hazard occurs when two instructions need the same hardware
# resource in the same clock cycle. It's like two people trying to use
# the same bathroom at the same time -- someone has to wait.
#
# === Configurability ===
#
# The detector is configurable:
# - num_alus: How many ALU units are available (default: 1)
# - num_fp_units: How many FP units are available (default: 1)
# - split_caches: Whether L1I and L1D are separate (default: true)

module CodingAdventures
  module HazardDetection
    class StructuralHazardDetector
      # Configure the structural hazard detector.
      #
      # @param num_alus [Integer] Number of integer ALU units (default: 1)
      # @param num_fp_units [Integer] Number of floating-point units (default: 1)
      # @param split_caches [Boolean] Whether L1I and L1D are separate (default: true)
      def initialize(num_alus: 1, num_fp_units: 1, split_caches: true)
        @num_alus = num_alus
        @num_fp_units = num_fp_units
        @split_caches = split_caches
      end

      # Check for structural hazards between pipeline stages.
      #
      # @param id_stage [PipelineSlot] Instruction about to enter EX
      # @param ex_stage [PipelineSlot] Instruction currently in EX
      # @param if_stage [PipelineSlot, nil] Instruction being fetched
      # @param mem_stage [PipelineSlot, nil] Instruction accessing memory
      def detect(id_stage, ex_stage, if_stage: nil, mem_stage: nil)
        # Check execution unit conflicts first.
        exec_result = check_execution_unit_conflict(id_stage, ex_stage)
        return exec_result if exec_result.action != HazardAction::NONE

        # Check memory port conflicts.
        if if_stage && mem_stage
          mem_result = check_memory_port_conflict(if_stage, mem_stage)
          return mem_result if mem_result.action != HazardAction::NONE
        end

        HazardResult.new(
          action: HazardAction::NONE,
          reason: "no structural hazards -- all resources available"
        )
      end

      private

      # Check if ID and EX need the same execution unit.
      #
      # === Truth Table for ALU Conflict (1 ALU) ===
      #
      #   ID.uses_alu | EX.uses_alu | Conflict?
      #   -----------+-----------+----------
      #   false      | false     | No
      #   false      | true      | No
      #   true       | false     | No
      #   true       | true      | YES
      def check_execution_unit_conflict(id_stage, ex_stage)
        unless id_stage.valid && ex_stage.valid
          return HazardResult.new(
            action: HazardAction::NONE,
            reason: "one or both stages are empty (bubble)"
          )
        end

        # ALU conflict: both need ALU, but we only have 1.
        if id_stage.uses_alu && ex_stage.uses_alu && @num_alus < 2
          return HazardResult.new(
            action: HazardAction::STALL,
            stall_cycles: 1,
            reason: format(
              "structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) need the ALU, but only %d ALU available",
              id_stage.pc, ex_stage.pc, @num_alus
            )
          )
        end

        # FP unit conflict: both need FP, but we only have 1.
        if id_stage.uses_fp && ex_stage.uses_fp && @num_fp_units < 2
          return HazardResult.new(
            action: HazardAction::STALL,
            stall_cycles: 1,
            reason: format(
              "structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) need the FP unit, but only %d FP unit available",
              id_stage.pc, ex_stage.pc, @num_fp_units
            )
          )
        end

        HazardResult.new(
          action: HazardAction::NONE,
          reason: "no execution unit conflict"
        )
      end

      # Check if IF and MEM both need the memory bus.
      # Only matters when split_caches is false (shared L1 cache).
      def check_memory_port_conflict(if_stage, mem_stage)
        if @split_caches
          return HazardResult.new(
            action: HazardAction::NONE,
            reason: "split caches -- no memory port conflict"
          )
        end

        if if_stage.valid && mem_stage.valid && (mem_stage.mem_read || mem_stage.mem_write)
          access_type = mem_stage.mem_read ? "load" : "store"
          return HazardResult.new(
            action: HazardAction::STALL,
            stall_cycles: 1,
            reason: format(
              "structural hazard: IF (fetch at PC=0x%04X) and MEM (%s at PC=0x%04X) both need the shared memory bus",
              if_stage.pc, access_type, mem_stage.pc
            )
          )
        end

        HazardResult.new(
          action: HazardAction::NONE,
          reason: "no memory port conflict"
        )
      end
    end
  end
end
