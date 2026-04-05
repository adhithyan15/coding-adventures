# frozen_string_literal: true

# ==========================================================================
# Type Conversion Instruction Handlers for WASM
# ==========================================================================
#
# WASM is strongly typed. Conversion instructions explicitly convert
# between i32, i64, f32, and f64.
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module Conversion
        module_function

        def register(vm) # rubocop:disable Metrics/MethodLength
          # 0xA7: i32.wrap_i64
          vm.register_context_opcode(0xA7, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(v)); vm.advance_pc; "i32.wrap_i64"
          })

          # 0xA8: i32.trunc_f32_s
          vm.register_context_opcode(0xA8, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f32(vm.pop_typed)
            raise TrapError, "invalid conversion to integer" if v.nan?
            t = v.truncate
            raise TrapError, "integer overflow" if t < -2147483648 || t > 2147483647
            vm.push_typed(WasmExecution.i32(t)); vm.advance_pc; "i32.trunc_f32_s"
          })

          # 0xA9: i32.trunc_f32_u
          vm.register_context_opcode(0xA9, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f32(vm.pop_typed)
            raise TrapError, "invalid conversion to integer" if v.nan?
            t = v.truncate
            raise TrapError, "integer overflow" if t < 0 || t > 4294967295
            vm.push_typed(WasmExecution.i32(t)); vm.advance_pc; "i32.trunc_f32_u"
          })

          # 0xAA: i32.trunc_f64_s
          vm.register_context_opcode(0xAA, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f64(vm.pop_typed)
            raise TrapError, "invalid conversion to integer" if v.nan?
            t = v.truncate
            raise TrapError, "integer overflow" if t < -2147483648 || t > 2147483647
            vm.push_typed(WasmExecution.i32(t)); vm.advance_pc; "i32.trunc_f64_s"
          })

          # 0xAB: i32.trunc_f64_u
          vm.register_context_opcode(0xAB, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f64(vm.pop_typed)
            raise TrapError, "invalid conversion to integer" if v.nan?
            t = v.truncate
            raise TrapError, "integer overflow" if t < 0 || t > 4294967295
            vm.push_typed(WasmExecution.i32(t)); vm.advance_pc; "i32.trunc_f64_u"
          })

          # 0xAC: i64.extend_i32_s
          vm.register_context_opcode(0xAC, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(v)); vm.advance_pc; "i64.extend_i32_s"
          })

          # 0xAD: i64.extend_i32_u
          vm.register_context_opcode(0xAD, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(WasmExecution.to_u32(v))); vm.advance_pc; "i64.extend_i32_u"
          })

          # 0xAE: i64.trunc_f32_s
          vm.register_context_opcode(0xAE, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f32(vm.pop_typed)
            raise TrapError, "invalid conversion to integer" if v.nan?
            raise TrapError, "integer overflow" unless v.finite?
            t = v.truncate
            raise TrapError, "integer overflow" if t < -9223372036854775808 || t > 9223372036854775807
            vm.push_typed(WasmExecution.i64(t)); vm.advance_pc; "i64.trunc_f32_s"
          })

          # 0xAF: i64.trunc_f32_u
          vm.register_context_opcode(0xAF, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f32(vm.pop_typed)
            raise TrapError, "invalid conversion to integer" if v.nan?
            raise TrapError, "integer overflow" unless v.finite?
            t = v.truncate
            raise TrapError, "integer overflow" if t < 0 || t > 18446744073709551615
            vm.push_typed(WasmExecution.i64(t)); vm.advance_pc; "i64.trunc_f32_u"
          })

          # 0xB0: i64.trunc_f64_s
          vm.register_context_opcode(0xB0, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f64(vm.pop_typed)
            raise TrapError, "invalid conversion to integer" if v.nan?
            raise TrapError, "integer overflow" unless v.finite?
            t = v.truncate
            raise TrapError, "integer overflow" if t < -9223372036854775808 || t > 9223372036854775807
            vm.push_typed(WasmExecution.i64(t)); vm.advance_pc; "i64.trunc_f64_s"
          })

          # 0xB1: i64.trunc_f64_u
          vm.register_context_opcode(0xB1, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f64(vm.pop_typed)
            raise TrapError, "invalid conversion to integer" if v.nan?
            raise TrapError, "integer overflow" unless v.finite?
            t = v.truncate
            raise TrapError, "integer overflow" if t < 0 || t > 18446744073709551615
            vm.push_typed(WasmExecution.i64(t)); vm.advance_pc; "i64.trunc_f64_u"
          })

          # 0xB2: f32.convert_i32_s
          vm.register_context_opcode(0xB2, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(v.to_f)); vm.advance_pc; "f32.convert_i32_s"
          })

          # 0xB3: f32.convert_i32_u
          vm.register_context_opcode(0xB3, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(WasmExecution.to_u32(v).to_f)); vm.advance_pc; "f32.convert_i32_u"
          })

          # 0xB4: f32.convert_i64_s
          vm.register_context_opcode(0xB4, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(v.to_f)); vm.advance_pc; "f32.convert_i64_s"
          })

          # 0xB5: f32.convert_i64_u
          vm.register_context_opcode(0xB5, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(WasmExecution.to_u64(v).to_f)); vm.advance_pc; "f32.convert_i64_u"
          })

          # 0xB6: f32.demote_f64
          vm.register_context_opcode(0xB6, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f64(vm.pop_typed)
            vm.push_typed(WasmExecution.f32(v)); vm.advance_pc; "f32.demote_f64"
          })

          # 0xB7: f64.convert_i32_s
          vm.register_context_opcode(0xB7, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(v.to_f)); vm.advance_pc; "f64.convert_i32_s"
          })

          # 0xB8: f64.convert_i32_u
          vm.register_context_opcode(0xB8, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(WasmExecution.to_u32(v).to_f)); vm.advance_pc; "f64.convert_i32_u"
          })

          # 0xB9: f64.convert_i64_s
          vm.register_context_opcode(0xB9, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(v.to_f)); vm.advance_pc; "f64.convert_i64_s"
          })

          # 0xBA: f64.convert_i64_u
          vm.register_context_opcode(0xBA, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(WasmExecution.to_u64(v).to_f)); vm.advance_pc; "f64.convert_i64_u"
          })

          # 0xBB: f64.promote_f32
          vm.register_context_opcode(0xBB, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f32(vm.pop_typed)
            vm.push_typed(WasmExecution.f64(v)); vm.advance_pc; "f64.promote_f32"
          })

          # 0xBC: i32.reinterpret_f32
          vm.register_context_opcode(0xBC, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f32(vm.pop_typed)
            bits = [v].pack("e").unpack1("l<")
            vm.push_typed(WasmExecution.i32(bits)); vm.advance_pc; "i32.reinterpret_f32"
          })

          # 0xBD: i64.reinterpret_f64
          vm.register_context_opcode(0xBD, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_f64(vm.pop_typed)
            bits = [v].pack("E").unpack1("q<")
            vm.push_typed(WasmExecution.i64(bits)); vm.advance_pc; "i64.reinterpret_f64"
          })

          # 0xBE: f32.reinterpret_i32
          vm.register_context_opcode(0xBE, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i32(vm.pop_typed)
            result = [v].pack("l<").unpack1("e")
            vm.push_typed(WasmExecution.f32(result)); vm.advance_pc; "f32.reinterpret_i32"
          })

          # 0xBF: f64.reinterpret_i64
          vm.register_context_opcode(0xBF, ->(vm, _instr, _code, _ctx) {
            v = WasmExecution.as_i64(vm.pop_typed)
            result = [v].pack("q<").unpack1("E")
            vm.push_typed(WasmExecution.f64(result)); vm.advance_pc; "f64.reinterpret_i64"
          })
        end
      end
    end
  end
end
