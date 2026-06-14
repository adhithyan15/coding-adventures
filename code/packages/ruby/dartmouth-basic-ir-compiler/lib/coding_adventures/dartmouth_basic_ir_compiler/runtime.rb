# frozen_string_literal: true

require "stringio"
require "coding_adventures_codegen_core"
require "coding_adventures_jit_core"
require "coding_adventures_vm_core"
require_relative "compiler"

module CodingAdventures
  module DartmouthBasicIrCompiler
    class Runtime
      attr_reader :last_result, :last_vm

      def compile(source)
        Compiler.compile_source(source)
      end

      def run(source, jit: false)
        @last_result = compile(source)
        @last_vm = CodingAdventures::VmCore::VMCore.new
        output = StringIO.new
        @last_vm.register_builtin("__basic_print") do |args|
          output << args[0].to_s << "\n"
          nil
        end
        if jit
          CodingAdventures::JitCore::JITCore.new(@last_vm).execute_with_jit(@last_result.module)
        else
          @last_vm.execute(@last_result.module)
        end
        output.string
      end

      def emit(source, target:)
        CodingAdventures::CodegenCore::BackendRegistry.default.compile(compile(source).module, target: target)
      end
    end
  end
end
