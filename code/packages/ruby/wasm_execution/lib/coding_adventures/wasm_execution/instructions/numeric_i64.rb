# frozen_string_literal: true

# ==========================================================================
# 64-Bit Integer Instruction Handlers for WASM
# ==========================================================================
#
# Similar to i32 but for 64-bit integers. Ruby's arbitrary-precision
# integers make this straightforward --- we just need to wrap to 64 bits.
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module NumericI64
        module_function

        def clz64(value)
          v = value & 0xFFFFFFFFFFFFFFFF
          return 64 if v == 0
          count = 0
          bit = 1 << 63
          while (v & bit) == 0
            count += 1
            bit >>= 1
          end
          count
        end

        def ctz64(value)
          return 64 if value == 0
          v = value & 0xFFFFFFFFFFFFFFFF
          count = 0
          while (v & 1) == 0
            count += 1
            v >>= 1
          end
          count
        end

        def popcnt64(value)
          (value & 0xFFFFFFFFFFFFFFFF).to_s(2).count("1")
        end

        I64_MIN = -9223372036854775808

        def register(vm)
          # 0x42: i64.const
          vm.register_context_opcode(0x42, ->(vm, instr, _code, _ctx) {
            vm.push_typed(WasmExecution.i64(instr.operand))
            vm.advance_pc
            "i64.const"
          })

          # 0x50: i64.eqz
          vm.register_context_opcode(0x50, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a == 0 ? 1 : 0))
            vm.advance_pc
            "i64.eqz"
          })

          # 0x51: i64.eq
          vm.register_context_opcode(0x51, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a == b ? 1 : 0)); vm.advance_pc; "i64.eq"
          })

          # 0x52: i64.ne
          vm.register_context_opcode(0x52, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a != b ? 1 : 0)); vm.advance_pc; "i64.ne"
          })

          # 0x53: i64.lt_s
          vm.register_context_opcode(0x53, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a < b ? 1 : 0)); vm.advance_pc; "i64.lt_s"
          })

          # 0x54: i64.lt_u
          vm.register_context_opcode(0x54, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            a = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            vm.push_typed(WasmExecution.i32(a < b ? 1 : 0)); vm.advance_pc; "i64.lt_u"
          })

          # 0x55: i64.gt_s
          vm.register_context_opcode(0x55, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a > b ? 1 : 0)); vm.advance_pc; "i64.gt_s"
          })

          # 0x56: i64.gt_u
          vm.register_context_opcode(0x56, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            a = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            vm.push_typed(WasmExecution.i32(a > b ? 1 : 0)); vm.advance_pc; "i64.gt_u"
          })

          # 0x57: i64.le_s
          vm.register_context_opcode(0x57, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a <= b ? 1 : 0)); vm.advance_pc; "i64.le_s"
          })

          # 0x58: i64.le_u
          vm.register_context_opcode(0x58, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            a = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            vm.push_typed(WasmExecution.i32(a <= b ? 1 : 0)); vm.advance_pc; "i64.le_u"
          })

          # 0x59: i64.ge_s
          vm.register_context_opcode(0x59, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a >= b ? 1 : 0)); vm.advance_pc; "i64.ge_s"
          })

          # 0x5A: i64.ge_u
          vm.register_context_opcode(0x5A, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            a = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            vm.push_typed(WasmExecution.i32(a >= b ? 1 : 0)); vm.advance_pc; "i64.ge_u"
          })

          # 0x79: i64.clz
          vm.register_context_opcode(0x79, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(NumericI64.clz64(a))); vm.advance_pc; "i64.clz"
          })

          # 0x7A: i64.ctz
          vm.register_context_opcode(0x7A, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(NumericI64.ctz64(a))); vm.advance_pc; "i64.ctz"
          })

          # 0x7B: i64.popcnt
          vm.register_context_opcode(0x7B, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(NumericI64.popcnt64(a))); vm.advance_pc; "i64.popcnt"
          })

          # 0x7C: i64.add
          vm.register_context_opcode(0x7C, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(a + b)); vm.advance_pc; "i64.add"
          })

          # 0x7D: i64.sub
          vm.register_context_opcode(0x7D, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(a - b)); vm.advance_pc; "i64.sub"
          })

          # 0x7E: i64.mul
          vm.register_context_opcode(0x7E, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(a * b)); vm.advance_pc; "i64.mul"
          })

          # 0x7F: i64.div_s
          vm.register_context_opcode(0x7F, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            raise TrapError, "integer divide by zero" if b == 0
            raise TrapError, "integer overflow" if a == I64_MIN && b == -1
            result = a.fdiv(b).truncate
            vm.push_typed(WasmExecution.i64(result)); vm.advance_pc; "i64.div_s"
          })

          # 0x80: i64.div_u
          vm.register_context_opcode(0x80, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            a = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            raise TrapError, "integer divide by zero" if b == 0
            vm.push_typed(WasmExecution.i64(a / b)); vm.advance_pc; "i64.div_u"
          })

          # 0x81: i64.rem_s
          vm.register_context_opcode(0x81, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            raise TrapError, "integer divide by zero" if b == 0
            result = (a == I64_MIN && b == -1) ? 0 : a.remainder(b)
            vm.push_typed(WasmExecution.i64(result)); vm.advance_pc; "i64.rem_s"
          })

          # 0x82: i64.rem_u
          vm.register_context_opcode(0x82, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            a = WasmExecution.to_u64(WasmExecution.as_i64(vm.pop_typed))
            raise TrapError, "integer divide by zero" if b == 0
            vm.push_typed(WasmExecution.i64(a % b)); vm.advance_pc; "i64.rem_u"
          })

          # 0x83-0x85: i64 bitwise
          vm.register_context_opcode(0x83, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(a & b)); vm.advance_pc; "i64.and"
          })
          vm.register_context_opcode(0x84, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(a | b)); vm.advance_pc; "i64.or"
          })
          vm.register_context_opcode(0x85, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            vm.push_typed(WasmExecution.i64(a ^ b)); vm.advance_pc; "i64.xor"
          })

          # 0x86: i64.shl
          vm.register_context_opcode(0x86, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            n = b & 63
            vm.push_typed(WasmExecution.i64(WasmExecution.to_u64(a) << n)); vm.advance_pc; "i64.shl"
          })

          # 0x87: i64.shr_s
          vm.register_context_opcode(0x87, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            n = b & 63
            vm.push_typed(WasmExecution.i64(a >> n)); vm.advance_pc; "i64.shr_s"
          })

          # 0x88: i64.shr_u
          vm.register_context_opcode(0x88, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            n = b & 63
            vm.push_typed(WasmExecution.i64(WasmExecution.to_u64(a) >> n)); vm.advance_pc; "i64.shr_u"
          })

          # 0x89: i64.rotl
          vm.register_context_opcode(0x89, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            n = b & 63; ua = WasmExecution.to_u64(a)
            result = ((ua << n) | (ua >> (64 - n))) & 0xFFFFFFFFFFFFFFFF
            vm.push_typed(WasmExecution.i64(result)); vm.advance_pc; "i64.rotl"
          })

          # 0x8A: i64.rotr
          vm.register_context_opcode(0x8A, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i64(vm.pop_typed); a = WasmExecution.as_i64(vm.pop_typed)
            n = b & 63; ua = WasmExecution.to_u64(a)
            result = ((ua >> n) | (ua << (64 - n))) & 0xFFFFFFFFFFFFFFFF
            vm.push_typed(WasmExecution.i64(result)); vm.advance_pc; "i64.rotr"
          })
        end
      end
    end
  end
end
