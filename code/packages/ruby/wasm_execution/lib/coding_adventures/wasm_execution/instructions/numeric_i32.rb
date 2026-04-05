# frozen_string_literal: true

# ==========================================================================
# 32-Bit Integer Instruction Handlers for WASM
# ==========================================================================
#
# WebAssembly's i32 type represents 32-bit integers. WASM makes no
# distinction between signed and unsigned at the storage level --- both
# are 32 bits. The *interpretation* depends on the instruction:
#
#   i32.lt_s treats operands as signed (two's complement).
#   i32.lt_u treats operands as unsigned.
#   i32.add doesn't care --- addition is the same for both!
#
# Pop order: b FIRST (top of stack), THEN a. The operation is a <op> b.
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module Instructions
      module NumericI32
        module_function

        # Count trailing zeros in a 32-bit integer.
        def ctz32(value)
          return 32 if value == 0
          v = value & 0xFFFFFFFF
          count = 0
          while (v & 1) == 0
            count += 1
            v >>= 1
          end
          count
        end

        # Population count (number of set bits) in a 32-bit integer.
        def popcnt32(value)
          (value & 0xFFFFFFFF).to_s(2).count("1")
        end

        # Count leading zeros in a 32-bit integer.
        def clz32(value)
          v = value & 0xFFFFFFFF
          return 32 if v == 0
          count = 0
          bit = 1 << 31
          while (v & bit) == 0
            count += 1
            bit >>= 1
          end
          count
        end

        # Register all 33 i32 numeric instruction handlers.
        def register(vm)
          # 0x41: i32.const --- Push an i32 constant
          vm.register_context_opcode(0x41, ->(vm, instr, _code, _ctx) {
            vm.push_typed(WasmExecution.i32(instr.operand))
            vm.advance_pc
            "i32.const"
          })

          # 0x45: i32.eqz --- Test if zero
          vm.register_context_opcode(0x45, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a == 0 ? 1 : 0))
            vm.advance_pc
            "i32.eqz"
          })

          # 0x46: i32.eq
          vm.register_context_opcode(0x46, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a == b ? 1 : 0))
            vm.advance_pc
            "i32.eq"
          })

          # 0x47: i32.ne
          vm.register_context_opcode(0x47, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a != b ? 1 : 0))
            vm.advance_pc
            "i32.ne"
          })

          # 0x48: i32.lt_s --- Signed less than
          vm.register_context_opcode(0x48, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a < b ? 1 : 0))
            vm.advance_pc
            "i32.lt_s"
          })

          # 0x49: i32.lt_u --- Unsigned less than
          vm.register_context_opcode(0x49, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            a = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            vm.push_typed(WasmExecution.i32(a < b ? 1 : 0))
            vm.advance_pc
            "i32.lt_u"
          })

          # 0x4A: i32.gt_s
          vm.register_context_opcode(0x4A, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a > b ? 1 : 0))
            vm.advance_pc
            "i32.gt_s"
          })

          # 0x4B: i32.gt_u
          vm.register_context_opcode(0x4B, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            a = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            vm.push_typed(WasmExecution.i32(a > b ? 1 : 0))
            vm.advance_pc
            "i32.gt_u"
          })

          # 0x4C: i32.le_s
          vm.register_context_opcode(0x4C, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a <= b ? 1 : 0))
            vm.advance_pc
            "i32.le_s"
          })

          # 0x4D: i32.le_u
          vm.register_context_opcode(0x4D, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            a = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            vm.push_typed(WasmExecution.i32(a <= b ? 1 : 0))
            vm.advance_pc
            "i32.le_u"
          })

          # 0x4E: i32.ge_s
          vm.register_context_opcode(0x4E, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a >= b ? 1 : 0))
            vm.advance_pc
            "i32.ge_s"
          })

          # 0x4F: i32.ge_u
          vm.register_context_opcode(0x4F, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            a = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            vm.push_typed(WasmExecution.i32(a >= b ? 1 : 0))
            vm.advance_pc
            "i32.ge_u"
          })

          # 0x67: i32.clz
          vm.register_context_opcode(0x67, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(NumericI32.clz32(a)))
            vm.advance_pc
            "i32.clz"
          })

          # 0x68: i32.ctz
          vm.register_context_opcode(0x68, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(NumericI32.ctz32(a)))
            vm.advance_pc
            "i32.ctz"
          })

          # 0x69: i32.popcnt
          vm.register_context_opcode(0x69, ->(vm, _instr, _code, _ctx) {
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(NumericI32.popcnt32(a)))
            vm.advance_pc
            "i32.popcnt"
          })

          # 0x6A: i32.add
          vm.register_context_opcode(0x6A, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a + b))
            vm.advance_pc
            "i32.add"
          })

          # 0x6B: i32.sub
          vm.register_context_opcode(0x6B, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a - b))
            vm.advance_pc
            "i32.sub"
          })

          # 0x6C: i32.mul
          vm.register_context_opcode(0x6C, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a * b))
            vm.advance_pc
            "i32.mul"
          })

          # 0x6D: i32.div_s --- Signed division (trapping)
          vm.register_context_opcode(0x6D, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            raise TrapError, "integer divide by zero" if b == 0
            raise TrapError, "integer overflow" if a == I32_MIN && b == -1
            # Ruby's Integer#/ truncates toward negative infinity; we need toward zero.
            result = a.fdiv(b).truncate
            vm.push_typed(WasmExecution.i32(result))
            vm.advance_pc
            "i32.div_s"
          })

          # 0x6E: i32.div_u --- Unsigned division
          vm.register_context_opcode(0x6E, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            a = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            raise TrapError, "integer divide by zero" if b == 0
            vm.push_typed(WasmExecution.i32(a / b))
            vm.advance_pc
            "i32.div_u"
          })

          # 0x6F: i32.rem_s --- Signed remainder
          vm.register_context_opcode(0x6F, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            raise TrapError, "integer divide by zero" if b == 0
            if a == I32_MIN && b == -1
              vm.push_typed(WasmExecution.i32(0))
            else
              # Ruby's % gives result with sign of divisor; we need sign of dividend
              result = a.remainder(b)
              vm.push_typed(WasmExecution.i32(result))
            end
            vm.advance_pc
            "i32.rem_s"
          })

          # 0x70: i32.rem_u --- Unsigned remainder
          vm.register_context_opcode(0x70, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            a = WasmExecution.to_u32(WasmExecution.as_i32(vm.pop_typed))
            raise TrapError, "integer divide by zero" if b == 0
            vm.push_typed(WasmExecution.i32(a % b))
            vm.advance_pc
            "i32.rem_u"
          })

          # 0x71: i32.and
          vm.register_context_opcode(0x71, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a & b))
            vm.advance_pc
            "i32.and"
          })

          # 0x72: i32.or
          vm.register_context_opcode(0x72, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a | b))
            vm.advance_pc
            "i32.or"
          })

          # 0x73: i32.xor
          vm.register_context_opcode(0x73, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            vm.push_typed(WasmExecution.i32(a ^ b))
            vm.advance_pc
            "i32.xor"
          })

          # 0x74: i32.shl --- Shift left (mod 32)
          vm.register_context_opcode(0x74, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            n = b & 31
            vm.push_typed(WasmExecution.i32((WasmExecution.to_u32(a) << n)))
            vm.advance_pc
            "i32.shl"
          })

          # 0x75: i32.shr_s --- Arithmetic shift right
          vm.register_context_opcode(0x75, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            n = b & 31
            vm.push_typed(WasmExecution.i32(a >> n))
            vm.advance_pc
            "i32.shr_s"
          })

          # 0x76: i32.shr_u --- Logical shift right
          vm.register_context_opcode(0x76, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            n = b & 31
            vm.push_typed(WasmExecution.i32(WasmExecution.to_u32(a) >> n))
            vm.advance_pc
            "i32.shr_u"
          })

          # 0x77: i32.rotl --- Rotate left
          vm.register_context_opcode(0x77, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            n = b & 31
            ua = WasmExecution.to_u32(a)
            result = ((ua << n) | (ua >> (32 - n))) & 0xFFFFFFFF
            vm.push_typed(WasmExecution.i32(result))
            vm.advance_pc
            "i32.rotl"
          })

          # 0x78: i32.rotr --- Rotate right
          vm.register_context_opcode(0x78, ->(vm, _instr, _code, _ctx) {
            b = WasmExecution.as_i32(vm.pop_typed)
            a = WasmExecution.as_i32(vm.pop_typed)
            n = b & 31
            ua = WasmExecution.to_u32(a)
            result = ((ua >> n) | (ua << (32 - n))) & 0xFFFFFFFF
            vm.push_typed(WasmExecution.i32(result))
            vm.advance_pc
            "i32.rotr"
          })
        end
      end
    end
  end
end
