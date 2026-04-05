# frozen_string_literal: true

# ==========================================================================
# Central Registration of All WASM Instruction Handlers
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module Dispatch
        module_function

        # Register all non-control-flow WASM instruction handlers.
        def register_all(vm)
          NumericI32.register(vm)
          NumericI64.register(vm)
          NumericF32.register(vm)
          NumericF64.register(vm)
          Conversion.register(vm)
          Variable.register(vm)
          Parametric.register(vm)
          Memory.register(vm)
        end
      end
    end
  end
end
