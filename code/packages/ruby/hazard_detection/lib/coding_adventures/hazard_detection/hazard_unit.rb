# frozen_string_literal: true

# Combined hazard detection unit -- the pipeline's traffic controller.
#
# === Priority System ===
#
#   FLUSH > STALL > FORWARD > NONE
#
# The hazard unit runs ALL detectors every clock cycle and returns ONE
# decision. It also tracks statistics for performance analysis.

module CodingAdventures
  module HazardDetection
    class HazardUnit
      attr_reader :history

      # Create a hazard unit with configurable hardware resources.
      #
      # @param num_alus [Integer] Number of integer ALUs
      # @param num_fp_units [Integer] Number of floating-point units
      # @param split_caches [Boolean] Whether L1I and L1D caches are separate
      def initialize(num_alus: 1, num_fp_units: 1, split_caches: true)
        @data_detector = DataHazardDetector.new
        @control_detector = ControlHazardDetector.new
        @structural_detector = StructuralHazardDetector.new(
          num_alus: num_alus,
          num_fp_units: num_fp_units,
          split_caches: split_caches
        )
        @history = []
      end

      # Run all hazard detectors and return the highest-priority action.
      #
      # Called once per clock cycle.
      def check(if_stage, id_stage, ex_stage, mem_stage)
        # 1. Control hazards (highest priority)
        control_result = @control_detector.detect(ex_stage)

        # 2. Data hazards
        data_result = @data_detector.detect(id_stage, ex_stage, mem_stage)

        # 3. Structural hazards
        structural_result = @structural_detector.detect(
          id_stage, ex_stage,
          if_stage: if_stage, mem_stage: mem_stage
        )

        # Pick highest-priority result.
        final = pick_highest_priority(control_result, data_result, structural_result)
        @history << final
        final
      end

      # Total stall cycles across all checks.
      def stall_count
        @history.sum(&:stall_cycles)
      end

      # Total pipeline flushes (branch mispredictions).
      def flush_count
        @history.count { |r| r.action == HazardAction::FLUSH }
      end

      # Total forwarding operations.
      def forward_count
        @history.count { |r|
          r.action == HazardAction::FORWARD_FROM_EX ||
            r.action == HazardAction::FORWARD_FROM_MEM
        }
      end

      private

      # Return the hazard result with the highest-priority action.
      def pick_highest_priority(*results)
        results.max_by { |r| HazardAction::PRIORITY[r.action] }
      end
    end
  end
end
