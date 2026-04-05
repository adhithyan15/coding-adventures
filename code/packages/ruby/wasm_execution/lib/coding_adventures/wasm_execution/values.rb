# frozen_string_literal: true

# ==========================================================================
# WASM Values --- Typed Numeric Values for the Execution Engine
# ==========================================================================
#
# Every value in WebAssembly is *typed* --- it carries both a raw payload
# and a type tag. An i32(42) and an f64(42.0) are DIFFERENT values, even
# though the numeric payloads look similar.
#
# The four WASM 1.0 value types:
#
#   +------+------------------------------------------------------+
#   | Type | Description                                          |
#   +------+------------------------------------------------------+
#   | i32  | 32-bit integer (stored as Ruby Integer)              |
#   | i64  | 64-bit integer (stored as Ruby Integer)              |
#   | f32  | 32-bit IEEE 754 float (stored as Ruby Float)         |
#   | f64  | 64-bit IEEE 754 float (stored as Ruby Float)         |
#   +------+------------------------------------------------------+
#
# Ruby's Integer has arbitrary precision, so we must explicitly wrap values
# to their correct bit widths. Ruby's Float is always 64-bit (IEEE 754
# double), so f32 values need rounding via Array#pack/unpack.
#
# ==========================================================================
# WRAPPING SEMANTICS
# ==========================================================================
#
# WASM integers have fixed bit widths. When arithmetic produces a result
# outside the representable range, the value wraps (modular arithmetic):
#
#   - i32: wraps to [-2^31, 2^31 - 1] using two's complement
#     Formula: val = ((val + 0x80000000) % 0x100000000) - 0x80000000
#
#   - i64: wraps to [-2^63, 2^63 - 1] using two's complement
#     Formula: val = ((val + 0x8000000000000000) % 0x10000000000000000) - 0x8000000000000000
#
#   - f32: rounds to nearest IEEE 754 single-precision float
#     Trick: [val].pack('e').unpack1('e') round-trips through 32-bit
#
#   - f64: no conversion needed (Ruby Float is already 64-bit double)
#
# ==========================================================================

module CodingAdventures
  module WasmExecution
    # WasmValue is an alias for the GenericVM's TypedVMValue.
    #
    # We reuse the same struct so that push_typed/pop_typed work seamlessly.
    # The `type` field is a VALUE_TYPE constant (0x7F for i32, etc.) and
    # the `value` field holds the raw Ruby numeric payload.
    WasmValue = CodingAdventures::VirtualMachine::TypedVMValue

    # ── i32 Constants ──────────────────────────────────────────────────

    # The bit mask for 32-bit unsigned values: 2^32 = 4294967296.
    I32_MOD = 0x100000000

    # The offset for converting unsigned to signed: 2^31 = 2147483648.
    I32_SIGN = 0x80000000

    # The minimum signed 32-bit integer: -2,147,483,648.
    I32_MIN = -2147483648

    # The maximum signed 32-bit integer: 2,147,483,647.
    I32_MAX = 2147483647

    # ── i64 Constants ──────────────────────────────────────────────────

    I64_MOD = 0x10000000000000000
    I64_SIGN = 0x8000000000000000

    # ── Constructor Functions ──────────────────────────────────────────

    # Create an i32 (32-bit integer) WASM value with proper wrapping.
    #
    # The wrapping formula converts any Ruby Integer to a signed 32-bit
    # integer in the range [-2^31, 2^31 - 1]:
    #
    #   +-----------------------+--------+------------------------+
    #   | Input                 | Result | Why?                   |
    #   +-----------------------+--------+------------------------+
    #   | 42                    | 42     | Fits in i32            |
    #   | -1                    | -1     | Already valid          |
    #   | 0xFFFFFFFF (2^32 - 1) | -1     | Wraps to signed        |
    #   | 0x100000000 (2^32)    | 0      | Truncates to 32 bits   |
    #   +-----------------------+--------+------------------------+
    #
    # @param value [Integer] any Ruby integer
    # @return [WasmValue] an i32-typed WASM value
    def self.i32(value)
      wrapped = ((value.to_i + I32_SIGN) % I32_MOD) - I32_SIGN
      WasmValue.new(WasmTypes::VALUE_TYPE[:i32], wrapped)
    end

    # Create an i64 (64-bit integer) WASM value with proper wrapping.
    #
    # Same idea as i32 but for the 64-bit range [-2^63, 2^63 - 1].
    # Ruby Integers are arbitrary precision, so we must explicitly wrap.
    #
    # @param value [Integer] any Ruby integer
    # @return [WasmValue] an i64-typed WASM value
    def self.i64(value)
      wrapped = ((value.to_i + I64_SIGN) % I64_MOD) - I64_SIGN
      WasmValue.new(WasmTypes::VALUE_TYPE[:i64], wrapped)
    end

    # Create an f32 (32-bit float) WASM value.
    #
    # Ruby Floats are always 64-bit doubles. To get proper 32-bit
    # float semantics, we round-trip through the 'e' pack format
    # (little-endian single-precision IEEE 754):
    #
    #   [val].pack('e').unpack1('e')
    #
    # This produces the same result as JavaScript's Math.fround().
    #
    # @param value [Numeric] any Ruby numeric
    # @return [WasmValue] an f32-typed WASM value
    def self.f32(value)
      rounded = [value.to_f].pack("e").unpack1("e")
      WasmValue.new(WasmTypes::VALUE_TYPE[:f32], rounded)
    end

    # Create an f64 (64-bit float) WASM value.
    #
    # Ruby Floats are already 64-bit doubles, so no conversion needed.
    #
    # @param value [Numeric] any Ruby numeric
    # @return [WasmValue] an f64-typed WASM value
    def self.f64(value)
      WasmValue.new(WasmTypes::VALUE_TYPE[:f64], value.to_f)
    end

    # ── Default Value ──────────────────────────────────────────────────

    # Create a zero-initialized WasmValue for a given type code.
    #
    # When a WASM function is called, all local variables are initialized
    # to the "default value" for their type (the respective zero):
    #
    #   +------+-------------------+
    #   | Type | Default           |
    #   +------+-------------------+
    #   | i32  | 0                 |
    #   | i64  | 0                 |
    #   | f32  | 0.0               |
    #   | f64  | 0.0               |
    #   +------+-------------------+
    #
    # @param type_code [Integer] one of the VALUE_TYPE constants
    # @return [WasmValue] a zero-initialized value of the given type
    def self.default_value(type_code)
      case type_code
      when WasmTypes::VALUE_TYPE[:i32] then i32(0)
      when WasmTypes::VALUE_TYPE[:i64] then i64(0)
      when WasmTypes::VALUE_TYPE[:f32] then f32(0)
      when WasmTypes::VALUE_TYPE[:f64] then f64(0)
      else
        raise TrapError, "Unknown value type: 0x#{type_code.to_s(16)}"
      end
    end

    # ── Type Extraction Helpers ────────────────────────────────────────

    # Human-readable names for WASM value types, used in error messages.
    TYPE_NAMES = {
      WasmTypes::VALUE_TYPE[:i32] => "i32",
      WasmTypes::VALUE_TYPE[:i64] => "i64",
      WasmTypes::VALUE_TYPE[:f32] => "f32",
      WasmTypes::VALUE_TYPE[:f64] => "f64"
    }.freeze

    # Extract the raw Integer from an i32 WasmValue.
    # Traps if the value is not actually an i32.
    #
    # @param v [WasmValue] a WASM value that must be i32-typed
    # @return [Integer] the raw integer payload
    def self.as_i32(v)
      unless v.type == WasmTypes::VALUE_TYPE[:i32]
        raise TrapError,
              "Type mismatch: expected i32, got #{TYPE_NAMES[v.type] || "0x#{v.type.to_s(16)}"}"
      end
      v.value
    end

    # Extract the raw Integer from an i64 WasmValue.
    # Traps if the value is not actually an i64.
    def self.as_i64(v)
      unless v.type == WasmTypes::VALUE_TYPE[:i64]
        raise TrapError,
              "Type mismatch: expected i64, got #{TYPE_NAMES[v.type] || "0x#{v.type.to_s(16)}"}"
      end
      v.value
    end

    # Extract the raw Float from an f32 WasmValue.
    # Traps if the value is not actually an f32.
    def self.as_f32(v)
      unless v.type == WasmTypes::VALUE_TYPE[:f32]
        raise TrapError,
              "Type mismatch: expected f32, got #{TYPE_NAMES[v.type] || "0x#{v.type.to_s(16)}"}"
      end
      v.value
    end

    # Extract the raw Float from an f64 WasmValue.
    # Traps if the value is not actually an f64.
    def self.as_f64(v)
      unless v.type == WasmTypes::VALUE_TYPE[:f64]
        raise TrapError,
              "Type mismatch: expected f64, got #{TYPE_NAMES[v.type] || "0x#{v.type.to_s(16)}"}"
      end
      v.value
    end

    # ── Unsigned Interpretation Helpers ─────────────────────────────────

    # Interpret a signed i32 value as unsigned (for unsigned comparisons).
    #
    # Ruby's integers are arbitrary precision, so we just mask to 32 bits.
    #   to_u32(-1) => 4294967295
    #   to_u32(42) => 42
    #
    def self.to_u32(val)
      val & 0xFFFFFFFF
    end

    # Interpret a signed i64 value as unsigned.
    def self.to_u64(val)
      val & 0xFFFFFFFFFFFFFFFF
    end
  end
end
