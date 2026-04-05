# frozen_string_literal: true

# ==========================================================================
# Parametric Instruction Handlers for WASM (drop, select)
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module Parametric
        module_function

        def register(vm)
          # 0x1A: drop --- discard the top stack value
          vm.register_context_opcode(0x1A, ->(vm, _instr, _code, _ctx) {
            vm.pop_typed
            vm.advance_pc; "drop"
          })

          # 0x1B: select --- ternary pick
          # Pop order: condition (i32), val2, val1
          # Push val1 if condition != 0, else val2
          vm.register_context_opcode(0x1B, ->(vm, _instr, _code, _ctx) {
            condition = vm.pop_typed
            val2 = vm.pop_typed
            val1 = vm.pop_typed
            vm.push_typed(condition.value != 0 ? val1 : val2)
            vm.advance_pc; "select"
          })
        end
      end
    end
  end
end
