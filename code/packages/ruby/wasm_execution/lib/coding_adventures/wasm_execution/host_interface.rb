# frozen_string_literal: true

# ==========================================================================
# HostInterface --- Contract for Resolving WASM Imports
# ==========================================================================
#
# WASM modules interact with the outside world through *imports* ---
# functions, globals, memories, and tables provided by the host. The
# HostInterface defines what methods a host must implement to provide
# these imported values.
#
# A HostFunction is a callable provided by the host for imported functions.
# It carries a FuncType (the type signature) and a call method.
# ==========================================================================

module CodingAdventures
  module WasmExecution
    # HostFunction --- a callable function provided by the host environment.
    #
    # When a WASM module imports a function, the host must provide an object
    # with a `func_type` (FuncType) and a `call` method that accepts an
    # array of WasmValues and returns an array of WasmValues.
    #
    # Example:
    #   host_fn = HostFunction.new(
    #     func_type: FuncType.new([VALUE_TYPE[:i32]], []),
    #     implementation: ->(args) { puts WasmExecution.as_i32(args[0]); [] }
    #   )
    #
    HostFunction = Struct.new(:func_type, :implementation, keyword_init: true) do
      def call(args)
        implementation.call(args)
      end
    end

    # HostInterface --- the contract for resolving WASM imports.
    #
    # Implement this module's methods to provide host functions, globals,
    # memories, and tables. Return nil if an import cannot be resolved.
    #
    # Example:
    #   class MyHost
    #     include CodingAdventures::WasmExecution::HostInterface
    #
    #     def resolve_function(module_name, name)
    #       return my_print_fn if module_name == "env" && name == "print"
    #       nil
    #     end
    #     ...
    #   end
    #
    module HostInterface
      def resolve_function(_module_name, _name) = nil
      def resolve_global(_module_name, _name) = nil
      def resolve_memory(_module_name, _name) = nil
      def resolve_table(_module_name, _name) = nil
    end
  end
end
