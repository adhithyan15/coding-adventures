# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module Brainfuck
    class LangVmTest < Minitest::Test
      def test_compiles_to_interpreter_ir
        mod = Brainfuck.compile_to_iir("+.")

        assert_equal "brainfuck", mod.language
        assert_equal ["main"], mod.function_names
        assert mod.get_function("main").instructions.any? { |instr| instr.op == "io_out" }
      end

      def test_executes_on_lang_vm
        result = Brainfuck.execute_on_lang_vm("+++++.")

        assert_equal 5.chr, result.output
        assert_equal 5, result.memory[0]
      end

      def test_loops_on_lang_vm
        result = Brainfuck.execute_on_lang_vm("++[>++<-]>.")

        assert_equal 4.chr, result.output
      end

      def test_lang_vm_supports_input
        result = Brainfuck.execute_on_lang_vm(",.", input: "Z")

        assert_equal "Z", result.output
      end
    end
  end
end
