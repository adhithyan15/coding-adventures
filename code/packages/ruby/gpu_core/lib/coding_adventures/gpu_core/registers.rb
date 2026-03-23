# frozen_string_literal: true

# ---------------------------------------------------------------------------
# FPRegisterFile -- floating-point register storage for GPU cores.
# ---------------------------------------------------------------------------
#
# === What is a Register File? ===
#
# A register file is the fastest storage in a processor -- faster than cache,
# faster than RAM. It's where the processor keeps the values it's currently
# working with. Think of it like the handful of numbers you can keep in your
# head while doing mental math.
#
#     Register file (in your head):
#         "first number"  = 3.14
#         "second number" = 2.71
#         "result"        = ???
#
#     Register file (in a GPU core):
#         R0  = 3.14  (FloatBits: sign=0, exp=[...], mantissa=[...])
#         R1  = 2.71  (FloatBits: sign=0, exp=[...], mantissa=[...])
#         R2  = 0.00  (will hold the result)
#
# === GPU vs CPU Register Files ===
#
# CPU registers hold integers (32 or 64 bits of binary). GPU registers hold
# floating-point numbers (IEEE 754 FloatBits). This reflects their different
# purposes:
#
#     CPU: general-purpose computation (loops, pointers, addresses -> integers)
#     GPU: parallel numeric computation (vertices, pixels, gradients -> floats)
#
# === Why Configurable? ===
#
# Different GPU vendors use different register counts:
#
#     NVIDIA CUDA Core:    up to 255 registers per thread
#     AMD Stream Processor: 256 VGPRs (Vector General Purpose Registers)
#     Intel Vector Engine:  128 GRF entries (General Register File)
#     ARM Mali:            64 registers per thread
#
# By making the register count a constructor parameter, the same GPUCore
# class can simulate any vendor's register architecture.
#
# === Register File Diagram ===
#
#     +------------------------------------------+
#     |           FP Register File               |
#     |         (32 registers x FP32)            |
#     +------------------------------------------+
#     |  R0:  [0][01111111][00000000000...0]     |  = +1.0
#     |  R1:  [0][10000000][00000000000...0]     |  = +2.0
#     |  R2:  [0][00000000][00000000000...0]     |  = +0.0
#     |  ...                                     |
#     |  R31: [0][00000000][00000000000...0]     |  = +0.0
#     +------------------------------------------+
#
#     Each register stores a FloatBits value:
#         sign (1 bit) + exponent (8 bits for FP32) + mantissa (23 bits for FP32)

module CodingAdventures
  module GpuCore
    # A configurable floating-point register file.
    #
    # Stores FloatBits values (from the fp_arithmetic package) in a fixed
    # number of registers. Provides both raw FloatBits and convenience float
    # interfaces for reading and writing.
    #
    # @param num_registers [Integer] How many registers (default 32, max 256).
    # @param fmt [FloatFormat] The floating-point format (FP32, FP16, BF16).
    class FPRegisterFile
      attr_reader :num_registers, :fmt

      def initialize(num_registers: 32, fmt: FpArithmetic::FP32)
        if num_registers < 1 || num_registers > 256
          raise ArgumentError, "num_registers must be 1-256, got #{num_registers}"
        end

        @num_registers = num_registers
        @fmt = fmt

        # Initialize all registers to +0.0 in the specified format.
        @zero = FpArithmetic.float_to_bits(0.0, fmt)
        @values = Array.new(num_registers, @zero)
      end

      # Validate a register index, raising IndexError if out of bounds.
      #
      # This is called before every read and write to catch bugs early.
      # In real hardware, accessing an invalid register would cause undefined
      # behavior -- we prefer a clear error message.
      def check_index(index)
        if index < 0 || index >= @num_registers
          raise IndexError, "Register index #{index} out of range [0, #{@num_registers - 1}]"
        end
      end

      # Read a register as a FloatBits value.
      #
      # @param index [Integer] Register number (0 to num_registers-1).
      # @return [FloatBits] The FloatBits value stored in that register.
      # @raise [IndexError] If index is out of range.
      def read(index)
        check_index(index)
        @values[index]
      end

      # Write a FloatBits value to a register.
      #
      # @param index [Integer] Register number (0 to num_registers-1).
      # @param value [FloatBits] The FloatBits value to store.
      # @raise [IndexError] If index is out of range.
      def write(index, value)
        check_index(index)
        @values[index] = value
      end

      # Convenience: read a register as a Ruby float.
      #
      # This decodes the FloatBits back to a float, which is useful for
      # inspection and testing but loses the bit-level detail.
      def read_float(index)
        FpArithmetic.bits_to_float(read(index))
      end

      # Convenience: write a Ruby float to a register.
      #
      # This encodes the float as FloatBits in the register file's format,
      # then stores it. Useful for setting up test inputs.
      def write_float(index, value)
        write(index, FpArithmetic.float_to_bits(value, @fmt))
      end

      # Return all register values as a hash of "R{n}" => float.
      #
      # Useful for debugging and test assertions. Only includes non-zero
      # registers to reduce noise.
      #
      # @return [Hash<String, Float>] Register names to float values.
      def dump
        result = {}
        @num_registers.times do |i|
          val = FpArithmetic.bits_to_float(@values[i])
          result["R#{i}"] = val if val != 0.0
        end
        result
      end

      # Return ALL register values as a hash of "R{n}" => float.
      #
      # Unlike dump, this includes zero-valued registers.
      def dump_all
        result = {}
        @num_registers.times do |i|
          result["R#{i}"] = FpArithmetic.bits_to_float(@values[i])
        end
        result
      end

      # Human-readable representation of the register file.
      def to_s
        non_zero = dump
        if non_zero.empty?
          "FPRegisterFile(#{@num_registers} regs, all zero)"
        else
          entries = non_zero.map { |k, v| "#{k}=#{v}" }.join(", ")
          "FPRegisterFile(#{entries})"
        end
      end

      def inspect
        to_s
      end

      private :check_index
    end
  end
end
