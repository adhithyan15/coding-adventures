# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module VmCore
    IR = CodingAdventures::InterpreterIr

    class VMCoreTest < Minitest::Test
      def test_executes_arithmetic_function
        mod = IR::IIRModule.new(
          name: "m",
          functions: [
            IR::IIRFunction.new(
              name: "main",
              params: [["a", "u8"], ["b", "u8"]],
              return_type: "u8",
              instructions: [
                IR::IIRInstr.new("add", "x", ["a", "b"], "u8"),
                IR::IIRInstr.new("ret", nil, ["x"], "u8")
              ]
            )
          ]
        )

        assert_equal 7, VMCore.new(u8_wrap: true).execute(mod, args: [3, 4])
      end

      def test_branches_and_metrics
        fn = IR::IIRFunction.new(
          name: "main",
          instructions: [
            IR::IIRInstr.new("const", "x", [true], "bool"),
            IR::IIRInstr.new("jmp_if_true", nil, ["x", "done"], "void"),
            IR::IIRInstr.new("const", "r", [0], "u8"),
            IR::IIRInstr.new("label", nil, ["done"], "void"),
            IR::IIRInstr.new("const", "r", [1], "u8"),
            IR::IIRInstr.new("ret", nil, ["r"], "u8")
          ]
        )
        vm = VMCore.new
        assert_equal 1, vm.execute(IR::IIRModule.new(name: "m", functions: [fn]))
        assert_equal 1, vm.branch_profile("main", 1).taken_count
      end

      def test_builtin_and_io
        fn = IR::IIRFunction.new(
          name: "main",
          instructions: [
            IR::IIRInstr.new("const", "x", [65], "u8"),
            IR::IIRInstr.new("io_out", nil, ["x"], "void"),
            IR::IIRInstr.new("call_builtin", "ok", ["assert_eq", "x", 65], "bool"),
            IR::IIRInstr.new("ret", nil, ["ok"], "bool")
          ]
        )
        vm = VMCore.new
        assert_equal true, vm.execute(IR::IIRModule.new(name: "m", functions: [fn]))
        assert_equal "A", vm.output.string
      end
    end
  end
end
