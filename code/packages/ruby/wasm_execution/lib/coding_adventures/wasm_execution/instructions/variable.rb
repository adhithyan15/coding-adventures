# frozen_string_literal: true

# ==========================================================================
# Variable Access Instruction Handlers for WASM
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module Variable
        module_function

        def register(vm)
          # 0x20: local.get
          vm.register_context_opcode(0x20, ->(vm, instr, _code, ctx) {
            index = instr.operand
            vm.push_typed(ctx[:typed_locals][index])
            vm.advance_pc; "local.get"
          })

          # 0x21: local.set
          vm.register_context_opcode(0x21, ->(vm, instr, _code, ctx) {
            index = instr.operand
            ctx[:typed_locals][index] = vm.pop_typed
            vm.advance_pc; "local.set"
          })

          # 0x22: local.tee --- write WITHOUT popping
          vm.register_context_opcode(0x22, ->(vm, instr, _code, ctx) {
            index = instr.operand
            ctx[:typed_locals][index] = vm.peek_typed
            vm.advance_pc; "local.tee"
          })

          # 0x23: global.get
          vm.register_context_opcode(0x23, ->(vm, instr, _code, ctx) {
            index = instr.operand
            vm.push_typed(ctx[:globals][index])
            vm.advance_pc; "global.get"
          })

          # 0x24: global.set
          vm.register_context_opcode(0x24, ->(vm, instr, _code, ctx) {
            index = instr.operand
            ctx[:globals][index] = vm.pop_typed
            vm.advance_pc; "global.set"
          })
        end
      end
    end
  end
end
