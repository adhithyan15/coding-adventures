# frozen_string_literal: true

# ==========================================================================
# Constant Expression Evaluator
# ==========================================================================
#
# In WASM, some values must be known at module *instantiation* time.
# These values are specified as "constant expressions" --- tiny programs
# using a restricted set of opcodes that produce a single value.
#
# Constant expressions appear in:
#   1. Global initializers
#   2. Data segment offsets
#   3. Element segment offsets
#
# Allowed opcodes:
#   0x41 i32.const, 0x42 i64.const, 0x43 f32.const, 0x44 f64.const,
#   0x23 global.get, 0x0B end
# ==========================================================================

module CodingAdventures
  module WasmExecution
    module ConstExpr
      module_function

      # Evaluate a WASM constant expression and return its result.
      #
      # @param expr [String] binary-encoded constant expression bytes
      # @param globals [Array<WasmValue>] available globals for global.get
      # @return [WasmValue]
      def evaluate(expr, globals = [])
        bytes = expr.is_a?(String) ? expr.bytes : expr
        result = nil
        pos = 0

        while pos < bytes.length
          opcode = bytes[pos]
          pos += 1

          case opcode
          when 0x41 # i32.const
            value, consumed = CodingAdventures::WasmLeb128.decode_signed(bytes, pos)
            pos += consumed
            result = WasmExecution.i32(value)

          when 0x42 # i64.const
            value, consumed = Decoder.decode_signed_64(bytes, pos)
            pos += consumed
            result = WasmExecution.i64(value)

          when 0x43 # f32.const
            raise TrapError, "f32.const: not enough bytes" if pos + 4 > bytes.length
            val = bytes[pos, 4].pack("C*").unpack1("e")
            pos += 4
            result = WasmExecution.f32(val)

          when 0x44 # f64.const
            raise TrapError, "f64.const: not enough bytes" if pos + 8 > bytes.length
            val = bytes[pos, 8].pack("C*").unpack1("E")
            pos += 8
            result = WasmExecution.f64(val)

          when 0x23 # global.get
            global_index, consumed = CodingAdventures::WasmLeb128.decode_unsigned(bytes, pos)
            pos += consumed
            raise TrapError, "global.get: index #{global_index} out of bounds" if global_index >= globals.length
            result = globals[global_index]

          when 0x0B # end
            raise TrapError, "Constant expression produced no value" if result.nil?
            return result

          else
            raise TrapError,
                  "Illegal opcode 0x#{opcode.to_s(16).rjust(2, "0")} in constant expression"
          end
        end

        raise TrapError, "Constant expression missing end opcode (0x0B)"
      end
    end
  end
end
