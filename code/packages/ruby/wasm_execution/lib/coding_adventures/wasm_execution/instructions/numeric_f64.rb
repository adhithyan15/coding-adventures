# frozen_string_literal: true

# ==========================================================================
# 64-Bit Float Instruction Handlers for WASM
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module NumericF64
        module_function

        def register(vm)
          # 0x44: f64.const
          vm.register_context_opcode(0x44, ->(vm, instr, _code, _ctx) {
            vm.push_typed(WasmExecution.f64(instr.operand))
            vm.advance_pc; "f64.const"
          })

          # 0x61: f64.eq
          vm.register_context_opcode(0x61, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a == b ? 1 : 0)); vm.advance_pc; "f64.eq"
          })

          # 0x62: f64.ne
          vm.register_context_opcode(0x62, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a != b ? 1 : 0)); vm.advance_pc; "f64.ne"
          })

          # 0x63: f64.lt
          vm.register_context_opcode(0x63, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a < b ? 1 : 0)); vm.advance_pc; "f64.lt"
          })

          # 0x64: f64.gt
          vm.register_context_opcode(0x64, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a > b ? 1 : 0)); vm.advance_pc; "f64.gt"
          })

          # 0x65: f64.le
          vm.register_context_opcode(0x65, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a <= b ? 1 : 0)); vm.advance_pc; "f64.le"
          })

          # 0x66: f64.ge
          vm.register_context_opcode(0x66, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a >= b ? 1 : 0)); vm.advance_pc; "f64.ge"
          })

          # 0x99: f64.abs
          vm.register_context_opcode(0x99, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a.abs)); vm.advance_pc; "f64.abs"
          })

          # 0x9A: f64.neg
          vm.register_context_opcode(0x9A, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(-a)); vm.advance_pc; "f64.neg"
          })

          # 0x9B: f64.ceil
          vm.register_context_opcode(0x9B, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a.ceil.to_f)); vm.advance_pc; "f64.ceil"
          })

          # 0x9C: f64.floor
          vm.register_context_opcode(0x9C, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a.floor.to_f)); vm.advance_pc; "f64.floor"
          })

          # 0x9D: f64.trunc
          vm.register_context_opcode(0x9D, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a.truncate.to_f)); vm.advance_pc; "f64.trunc"
          })

          # 0x9E: f64.nearest
          vm.register_context_opcode(0x9E, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a.round(half: :even).to_f)); vm.advance_pc; "f64.nearest"
          })

          # 0x9F: f64.sqrt
          vm.register_context_opcode(0x9F, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(Math.sqrt(a))); vm.advance_pc; "f64.sqrt"
          })

          # 0xA0: f64.add
          vm.register_context_opcode(0xA0, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a + b)); vm.advance_pc; "f64.add"
          })

          # 0xA1: f64.sub
          vm.register_context_opcode(0xA1, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a - b)); vm.advance_pc; "f64.sub"
          })

          # 0xA2: f64.mul
          vm.register_context_opcode(0xA2, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a * b)); vm.advance_pc; "f64.mul"
          })

          # 0xA3: f64.div
          vm.register_context_opcode(0xA3, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(a / b)); vm.advance_pc; "f64.div"
          })

          # 0xA4: f64.min
          vm.register_context_opcode(0xA4, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            result = a.nan? || b.nan? ? Float::NAN : [a, b].min
            vm.push_typed(WasmExecution.f64(result)); vm.advance_pc; "f64.min"
          })

          # 0xA5: f64.max
          vm.register_context_opcode(0xA5, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            result = a.nan? || b.nan? ? Float::NAN : [a, b].max
            vm.push_typed(WasmExecution.f64(result)); vm.advance_pc; "f64.max"
          })

          # 0xA6: f64.copysign
          vm.register_context_opcode(0xA6, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_f64(vm.pop_typed); a = WasmExecution.as_f64(vm.pop_typed)
            sign_b = [b].pack("E").unpack1("Q<") & 0x8000000000000000
            mag_a = [a].pack("E").unpack1("Q<") & 0x7FFFFFFFFFFFFFFF
            result = [sign_b | mag_a].pack("Q<").unpack1("E")
            vm.push_typed(WasmExecution.f64(result)); vm.advance_pc; "f64.copysign"
          })
        end
      end
    end
  end
end
