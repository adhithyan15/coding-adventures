# frozen_string_literal: true

require "coding_adventures_codegen_core"
require "coding_adventures_interpreter_ir"
require "coding_adventures_vm_core"
require_relative "pure_vm_backend"

module CodingAdventures
  module JitCore
    class JITCore
      IR = CodingAdventures::InterpreterIr

      def initialize(vm, backend = PureVmBackend.new, registry: CodingAdventures::CodegenCore::BackendRegistry.default)
        @vm = vm
        @backend = backend
        @registry = registry
      end

      def execute_with_jit(mod, fn: mod.entry_point || "main", args: [])
        compile_ready_functions(mod)
        @vm.execute(mod, fn: fn, args: args)
      end

      def compile_ready_functions(mod)
        return unless @backend.respond_to?(:compile_callable)

        mod.functions.each do |function|
          next unless should_compile?(function)

          @vm.register_jit_handler(function.name, @backend.compile_callable(function, mod))
        end
      end

      def emit(mod, target:)
        @registry.compile(mod, target: target)
      end

      private

      def should_compile?(function)
        case function.type_status
        when IR::FunctionTypeStatus::FULLY_TYPED
          true
        when IR::FunctionTypeStatus::PARTIALLY_TYPED
          function.call_count >= 10
        else
          function.call_count >= 100
        end
      end
    end
  end
end
