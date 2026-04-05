# frozen_string_literal: true

# ==========================================================================
# 32-Bit Float Instruction Handlers for WASM
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module NumericF32
        module_function

        def register(vm)
          # 0x43: f32.const
          vm.register_context_opcode(0x43, ->(vm, instr, _code, _ctx) {
            vm.push_typed(WasmExecution.f32(instr.operand))
            vm.advance_pc; "f32.const"
          })

          # 0x5B: f32.eq
          vm.register_context_opcode(0x5B, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a == b ? 1 : 0)); vm.advance_pc; "f32.eq"
          })

          # 0x5C: f32.ne
          vm.register_context_opcode(0x5C, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a != b ? 1 : 0)); vm.advance_pc; "f32.ne"
          })

          # 0x5D: f32.lt
          vm.register_context_opcode(0x5D, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a < b ? 1 : 0)); vm.advance_pc; "f32.lt"
          })

          # 0x5E: f32.gt
          vm.register_context_opcode(0x5E, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a > b ? 1 : 0)); vm.advance_pc; "f32.gt"
          })

          # 0x5F: f32.le
          vm.register_context_opcode(0x5F, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a <= b ? 1 : 0)); vm.advance_pc; "f32.le"
          })

          # 0x60: f32.ge
          vm.register_context_opcode(0x60, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a >= b ? 1 : 0)); vm.advance_pc; "f32.ge"
          })

          # 0x8B: f32.abs
          vm.register_context_opcode(0x8B, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a.abs)); vm.advance_pc; "f32.abs"
          })

          # 0x8C: f32.neg
          vm.register_context_opcode(0x8C, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(-a)); vm.advance_pc; "f32.neg"
          })

          # 0x8D: f32.ceil
          vm.register_context_opcode(0x8D, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a.ceil.to_f)); vm.advance_pc; "f32.ceil"
          })

          # 0x8E: f32.floor
          vm.register_context_opcode(0x8E, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a.floor.to_f)); vm.advance_pc; "f32.floor"
          })

          # 0x8F: f32.trunc
          vm.register_context_opcode(0x8F, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a.truncate.to_f)); vm.advance_pc; "f32.trunc"
          })

          # 0x90: f32.nearest
          vm.register_context_opcode(0x90, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a.round(half: :even).to_f)); vm.advance_pc; "f32.nearest"
          })

          # 0x91: f32.sqrt
          vm.register_context_opcode(0x91, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(Math.sqrt(a))); vm.advance_pc; "f32.sqrt"
          })

          # 0x92: f32.add
          vm.register_context_opcode(0x92, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a + b)); vm.advance_pc; "f32.add"
          })

          # 0x93: f32.sub
          vm.register_context_opcode(0x93, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a - b)); vm.advance_pc; "f32.sub"
          })

          # 0x94: f32.mul
          vm.register_context_opcode(0x94, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a * b)); vm.advance_pc; "f32.mul"
          })

          # 0x95: f32.div
          vm.register_context_opcode(0x95, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(a / b)); vm.advance_pc; "f32.div"
          })

          # 0x96: f32.min
          vm.register_context_opcode(0x96, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            result = a.nan? || b.nan? ? Float::NAN : [a, b].min
            vm.push_typed(WasmExecution.f32(result)); vm.advance_pc; "f32.min"
          })

          # 0x97: f32.max
          vm.register_context_opcode(0x97, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            result = a.nan? || b.nan? ? Float::NAN : [a, b].max
            vm.push_typed(WasmExecution.f32(result)); vm.advance_pc; "f32.max"
          })

          # 0x98: f32.copysign
          vm.register_context_opcode(0x98, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f32(vm.pop_typed); a = WasmExecution.as_f32(vm.pop_typed)
            # Copy sign of b to magnitude of a
            sign_b = [b].pack("e").unpack1("V") & 0x80000000
            mag_a = [a].pack("e").unpack1("V") & 0x7FFFFFFF
            result = [sign_b | mag_a].pack("V").unpack1("e")
            vm.push_typed(WasmExecution.f32(result)); vm.advance_pc; "f32.copysign"
          })
        end
      end
    end
  end
end
