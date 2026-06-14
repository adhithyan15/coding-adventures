# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module InterpreterIr
    class InterpreterIrTest < Minitest::Test
      def test_instruction_records_feedback_slot
        instr = IIRInstr.new("add", "v0", ["a", "b"], "any")
        instr.record_observation("u8")
        instr.record_observation("u8")

        assert_equal SlotKind::MONOMORPHIC, instr.observed_slot.kind
        assert_equal "u8", instr.observed_type
        assert_equal 2, instr.observation_count

        instr.record_observation("str")
        assert_equal SlotKind::POLYMORPHIC, instr.observed_slot.kind
        assert_equal "polymorphic", instr.observed_type
      end

      def test_function_infers_type_status
        fn = IIRFunction.new(
          name: "main",
          params: [["a", "u8"]],
          instructions: [
            IIRInstr.new("add", "x", ["a", 1], "u8"),
            IIRInstr.new("ret", nil, ["x"], "u8")
          ]
        )

        assert_equal FunctionTypeStatus::FULLY_TYPED, fn.type_status
      end

      def test_module_validation_catches_missing_entry_and_label
        fn = IIRFunction.new(
          name: "main",
          instructions: [IIRInstr.new("jmp", nil, ["missing"], "void")]
        )
        mod = IIRModule.new(name: "m", functions: [fn], entry_point: "main")

        assert_match(/undefined label/, mod.validate.join("\n"))
      end
    end
  end
end
