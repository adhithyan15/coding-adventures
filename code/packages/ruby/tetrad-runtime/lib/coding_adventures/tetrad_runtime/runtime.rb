# frozen_string_literal: true

require "coding_adventures_codegen_core"
require "coding_adventures_jit_core"
require "coding_adventures_vm_core"
require_relative "compiler"

module CodingAdventures
  module TetradRuntime
    class Runtime
      attr_reader :last_module, :last_vm

      def compile(source)
        Compiler.compile_source(source)
      end

      def run(source, jit: false)
        @last_module = compile(source)
        @last_vm = CodingAdventures::VmCore::VMCore.new(u8_wrap: true)
        if jit
          CodingAdventures::JitCore::JITCore.new(@last_vm).execute_with_jit(@last_module)
        else
          @last_vm.execute(@last_module)
        end
      end

      def emit(source, target:)
        CodingAdventures::CodegenCore::BackendRegistry.default.compile(compile(source), target: target)
      end
    end
  end
end
