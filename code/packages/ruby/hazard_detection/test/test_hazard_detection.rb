# frozen_string_literal: true

require "test_helper"

module CodingAdventures
  module HazardDetection
    # =========================================================================
    # Helper: shorthand for creating PipelineSlots
    # =========================================================================
    def self.slot(**kwargs)
      PipelineSlot.new(**kwargs)
    end

    def self.empty_slot
      PipelineSlot.new(valid: false)
    end

    # =========================================================================
    # DataHazardDetector Tests
    # =========================================================================
    class TestDataHazardDetector < Minitest::Test
      def setup
        @detector = DataHazardDetector.new
      end

      def test_no_hazard_when_id_is_empty
        id = HazardDetection.empty_slot
        ex = HazardDetection.slot(valid: true, dest_reg: 1, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::NONE, result.action
      end

      def test_no_hazard_when_no_source_regs
        id = HazardDetection.slot(valid: true, source_regs: [], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 1, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::NONE, result.action
      end

      def test_no_hazard_when_no_dependency
        id = HazardDetection.slot(valid: true, source_regs: [2, 3], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 5, uses_alu: false)
        mem = HazardDetection.slot(valid: true, dest_reg: 6, uses_alu: false)
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::NONE, result.action
      end

      def test_forward_from_ex
        # ADD R1, R2, R3 in EX; SUB R4, R1, R5 in ID
        id = HazardDetection.slot(valid: true, source_regs: [1, 5], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 1, dest_value: 42, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::FORWARD_FROM_EX, result.action
        assert_equal 42, result.forwarded_value
        assert_equal "EX", result.forwarded_from
      end

      def test_forward_from_mem
        id = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        ex = HazardDetection.empty_slot
        mem = HazardDetection.slot(valid: true, dest_reg: 1, dest_value: 99, uses_alu: false)
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::FORWARD_FROM_MEM, result.action
        assert_equal 99, result.forwarded_value
        assert_equal "MEM", result.forwarded_from
      end

      def test_load_use_stall
        # LW R1, [addr] in EX (mem_read=true); ADD R4, R1, R5 in ID
        id = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 1, mem_read: true, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::STALL, result.action
        assert_equal 1, result.stall_cycles
      end

      def test_ex_has_priority_over_mem
        # Both EX and MEM write R1; EX is newer so its value should be forwarded.
        id = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 1, dest_value: 10, uses_alu: false)
        mem = HazardDetection.slot(valid: true, dest_reg: 1, dest_value: 20, uses_alu: false)
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::FORWARD_FROM_EX, result.action
        assert_equal 10, result.forwarded_value
      end

      def test_multiple_source_regs_worst_wins
        # R1 forwards from MEM, R2 forwards from EX -> EX wins (higher priority).
        id = HazardDetection.slot(valid: true, source_regs: [1, 2], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 2, dest_value: 55, uses_alu: false)
        mem = HazardDetection.slot(valid: true, dest_reg: 1, dest_value: 77, uses_alu: false)
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::FORWARD_FROM_EX, result.action
      end

      def test_stall_beats_forward
        # R1 has load-use stall from EX; R2 forwards from MEM -> stall wins.
        id = HazardDetection.slot(valid: true, source_regs: [1, 2], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 1, mem_read: true, uses_alu: false)
        mem = HazardDetection.slot(valid: true, dest_reg: 2, dest_value: 77, uses_alu: false)
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::STALL, result.action
      end

      def test_no_hazard_when_ex_dest_reg_nil
        id = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: nil, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::NONE, result.action
      end

      def test_no_hazard_when_ex_invalid
        id = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        ex = HazardDetection.slot(valid: false, dest_reg: 1, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = @detector.detect(id, ex, mem)
        assert_equal HazardAction::NONE, result.action
      end
    end

    # =========================================================================
    # ControlHazardDetector Tests
    # =========================================================================
    class TestControlHazardDetector < Minitest::Test
      def setup
        @detector = ControlHazardDetector.new
      end

      def test_no_hazard_when_ex_empty
        ex = HazardDetection.empty_slot
        result = @detector.detect(ex)
        assert_equal HazardAction::NONE, result.action
      end

      def test_no_hazard_when_not_branch
        ex = HazardDetection.slot(valid: true, is_branch: false, uses_alu: false)
        result = @detector.detect(ex)
        assert_equal HazardAction::NONE, result.action
      end

      def test_no_hazard_when_correctly_predicted_taken
        ex = HazardDetection.slot(
          valid: true, is_branch: true,
          branch_taken: true, branch_predicted_taken: true,
          uses_alu: false
        )
        result = @detector.detect(ex)
        assert_equal HazardAction::NONE, result.action
      end

      def test_no_hazard_when_correctly_predicted_not_taken
        ex = HazardDetection.slot(
          valid: true, is_branch: true,
          branch_taken: false, branch_predicted_taken: false,
          uses_alu: false
        )
        result = @detector.detect(ex)
        assert_equal HazardAction::NONE, result.action
      end

      def test_flush_when_predicted_not_taken_but_taken
        ex = HazardDetection.slot(
          valid: true, is_branch: true, pc: 0x100,
          branch_taken: true, branch_predicted_taken: false,
          uses_alu: false
        )
        result = @detector.detect(ex)
        assert_equal HazardAction::FLUSH, result.action
        assert_equal 2, result.flush_count
        assert_includes result.reason, "not-taken, actually taken"
      end

      def test_flush_when_predicted_taken_but_not_taken
        ex = HazardDetection.slot(
          valid: true, is_branch: true, pc: 0x200,
          branch_taken: false, branch_predicted_taken: true,
          uses_alu: false
        )
        result = @detector.detect(ex)
        assert_equal HazardAction::FLUSH, result.action
        assert_equal 2, result.flush_count
        assert_includes result.reason, "taken, actually not-taken"
      end
    end

    # =========================================================================
    # StructuralHazardDetector Tests
    # =========================================================================
    class TestStructuralHazardDetector < Minitest::Test
      def test_no_hazard_with_enough_alus
        detector = StructuralHazardDetector.new(num_alus: 2)
        id = HazardDetection.slot(valid: true, uses_alu: true)
        ex = HazardDetection.slot(valid: true, uses_alu: true)
        result = detector.detect(id, ex)
        assert_equal HazardAction::NONE, result.action
      end

      def test_alu_conflict_with_one_alu
        detector = StructuralHazardDetector.new(num_alus: 1)
        id = HazardDetection.slot(valid: true, uses_alu: true)
        ex = HazardDetection.slot(valid: true, uses_alu: true)
        result = detector.detect(id, ex)
        assert_equal HazardAction::STALL, result.action
        assert_equal 1, result.stall_cycles
      end

      def test_fp_conflict_with_one_fp_unit
        detector = StructuralHazardDetector.new(num_fp_units: 1)
        id = HazardDetection.slot(valid: true, uses_alu: false, uses_fp: true)
        ex = HazardDetection.slot(valid: true, uses_alu: false, uses_fp: true)
        result = detector.detect(id, ex)
        assert_equal HazardAction::STALL, result.action
      end

      def test_no_fp_conflict_with_two_fp_units
        detector = StructuralHazardDetector.new(num_fp_units: 2)
        id = HazardDetection.slot(valid: true, uses_alu: false, uses_fp: true)
        ex = HazardDetection.slot(valid: true, uses_alu: false, uses_fp: true)
        result = detector.detect(id, ex)
        assert_equal HazardAction::NONE, result.action
      end

      def test_no_conflict_when_id_empty
        detector = StructuralHazardDetector.new(num_alus: 1)
        id = HazardDetection.empty_slot
        ex = HazardDetection.slot(valid: true, uses_alu: true)
        result = detector.detect(id, ex)
        assert_equal HazardAction::NONE, result.action
      end

      def test_no_conflict_when_ex_empty
        detector = StructuralHazardDetector.new(num_alus: 1)
        id = HazardDetection.slot(valid: true, uses_alu: true)
        ex = HazardDetection.empty_slot
        result = detector.detect(id, ex)
        assert_equal HazardAction::NONE, result.action
      end

      def test_memory_port_conflict_shared_cache
        detector = StructuralHazardDetector.new(split_caches: false)
        id = HazardDetection.slot(valid: true, uses_alu: false)
        ex = HazardDetection.slot(valid: true, uses_alu: false)
        if_stage = HazardDetection.slot(valid: true, pc: 0x10, uses_alu: false)
        mem_stage = HazardDetection.slot(valid: true, pc: 0x04, mem_read: true, uses_alu: false)
        result = detector.detect(id, ex, if_stage: if_stage, mem_stage: mem_stage)
        assert_equal HazardAction::STALL, result.action
      end

      def test_no_memory_conflict_split_cache
        detector = StructuralHazardDetector.new(split_caches: true)
        id = HazardDetection.slot(valid: true, uses_alu: false)
        ex = HazardDetection.slot(valid: true, uses_alu: false)
        if_stage = HazardDetection.slot(valid: true, uses_alu: false)
        mem_stage = HazardDetection.slot(valid: true, mem_read: true, uses_alu: false)
        result = detector.detect(id, ex, if_stage: if_stage, mem_stage: mem_stage)
        assert_equal HazardAction::NONE, result.action
      end

      def test_memory_port_conflict_store
        detector = StructuralHazardDetector.new(split_caches: false)
        id = HazardDetection.slot(valid: true, uses_alu: false)
        ex = HazardDetection.slot(valid: true, uses_alu: false)
        if_stage = HazardDetection.slot(valid: true, uses_alu: false)
        mem_stage = HazardDetection.slot(valid: true, mem_write: true, uses_alu: false)
        result = detector.detect(id, ex, if_stage: if_stage, mem_stage: mem_stage)
        assert_equal HazardAction::STALL, result.action
      end

      def test_no_memory_conflict_when_mem_not_accessing
        detector = StructuralHazardDetector.new(split_caches: false)
        id = HazardDetection.slot(valid: true, uses_alu: false)
        ex = HazardDetection.slot(valid: true, uses_alu: false)
        if_stage = HazardDetection.slot(valid: true, uses_alu: false)
        mem_stage = HazardDetection.slot(valid: true, uses_alu: false)
        result = detector.detect(id, ex, if_stage: if_stage, mem_stage: mem_stage)
        assert_equal HazardAction::NONE, result.action
      end
    end

    # =========================================================================
    # HazardUnit Tests
    # =========================================================================
    class TestHazardUnit < Minitest::Test
      def test_no_hazard
        unit = HazardUnit.new(num_alus: 2)
        if_s = HazardDetection.slot(valid: true, uses_alu: false)
        id = HazardDetection.slot(valid: true, source_regs: [2], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 5, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = unit.check(if_s, id, ex, mem)
        assert_equal HazardAction::NONE, result.action
      end

      def test_data_forwarding
        unit = HazardUnit.new(num_alus: 2)
        if_s = HazardDetection.slot(valid: true, uses_alu: false)
        id = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 1, dest_value: 42, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = unit.check(if_s, id, ex, mem)
        assert_equal HazardAction::FORWARD_FROM_EX, result.action
        assert_equal 42, result.forwarded_value
      end

      def test_control_flush_beats_data_forward
        unit = HazardUnit.new(num_alus: 2)
        if_s = HazardDetection.slot(valid: true, uses_alu: false)
        id = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        # EX has a mispredicted branch AND writes R1
        ex = HazardDetection.slot(
          valid: true, dest_reg: 1, dest_value: 42,
          is_branch: true, branch_taken: true, branch_predicted_taken: false,
          uses_alu: false
        )
        mem = HazardDetection.empty_slot
        result = unit.check(if_s, id, ex, mem)
        assert_equal HazardAction::FLUSH, result.action
      end

      def test_stall_beats_forward
        unit = HazardUnit.new(num_alus: 2)
        if_s = HazardDetection.slot(valid: true, uses_alu: false)
        id = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 1, mem_read: true, uses_alu: false)
        mem = HazardDetection.empty_slot
        result = unit.check(if_s, id, ex, mem)
        assert_equal HazardAction::STALL, result.action
      end

      def test_statistics_tracking
        unit = HazardUnit.new(num_alus: 2)
        empty = HazardDetection.empty_slot

        # Cycle 1: no hazard
        if_s = HazardDetection.slot(valid: true, uses_alu: false)
        id = HazardDetection.slot(valid: true, source_regs: [2], uses_alu: false)
        ex = HazardDetection.slot(valid: true, dest_reg: 5, uses_alu: false)
        unit.check(if_s, id, ex, empty)

        # Cycle 2: forward from EX
        id2 = HazardDetection.slot(valid: true, source_regs: [1], uses_alu: false)
        ex2 = HazardDetection.slot(valid: true, dest_reg: 1, dest_value: 10, uses_alu: false)
        unit.check(if_s, id2, ex2, empty)

        # Cycle 3: flush
        ex3 = HazardDetection.slot(
          valid: true, is_branch: true,
          branch_taken: true, branch_predicted_taken: false,
          uses_alu: false
        )
        unit.check(if_s, empty, ex3, empty)

        assert_equal 3, unit.history.length
        assert_equal 0, unit.stall_count
        assert_equal 1, unit.flush_count
        assert_equal 1, unit.forward_count
      end

      def test_structural_stall_with_one_alu
        unit = HazardUnit.new(num_alus: 1)
        if_s = HazardDetection.slot(valid: true, uses_alu: false)
        id = HazardDetection.slot(valid: true, source_regs: [], uses_alu: true)
        ex = HazardDetection.slot(valid: true, dest_reg: 5, uses_alu: true)
        mem = HazardDetection.empty_slot
        result = unit.check(if_s, id, ex, mem)
        assert_equal HazardAction::STALL, result.action
      end

      def test_forward_from_mem_via_unit
        unit = HazardUnit.new(num_alus: 2)
        if_s = HazardDetection.slot(valid: true, uses_alu: false)
        id = HazardDetection.slot(valid: true, source_regs: [3], uses_alu: false)
        ex = HazardDetection.empty_slot
        mem = HazardDetection.slot(valid: true, dest_reg: 3, dest_value: 88, uses_alu: false)
        result = unit.check(if_s, id, ex, mem)
        assert_equal HazardAction::FORWARD_FROM_MEM, result.action
        assert_equal 88, result.forwarded_value
      end

      def test_all_empty_stages
        unit = HazardUnit.new
        empty = HazardDetection.empty_slot
        result = unit.check(empty, empty, empty, empty)
        assert_equal HazardAction::NONE, result.action
      end
    end
  end
end
