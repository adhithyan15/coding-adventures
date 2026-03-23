# frozen_string_literal: true

# RegisterFile -- general-purpose register file for the Core.
#
# = Why a Custom Register File?
#
# The cpu-simulator package has a RegisterFile, but it uses uint32 values
# and panics on out-of-range access. The Core needs a register file that:
#
#   - Uses integer values (matching PipelineToken fields)
#   - Supports configurable width (32 or 64 bit)
#   - Optionally hardwires register 0 to zero (RISC-V convention)
#   - Returns 0 instead of raising on out-of-range access
#
# = Zero Register Convention
#
# In RISC-V and MIPS, register x0 (or $zero) is hardwired to the value 0.
# Writes to it are silently discarded. This simplifies instruction encoding:
#
#   MOV Rd, Rs  = ADD Rd, Rs, x0   (add zero)
#   NOP         = ADD x0, x0, x0   (write nothing to zero register)
#   NEG Rd, Rs  = SUB Rd, x0, Rs   (subtract from zero)
#
# ARM does NOT have a zero register (all 31 registers are general-purpose).
# x86 does not have one either. The zero_register config controls this.

module CodingAdventures
  module Core
    class RegisterFile
      # @return [RegisterFileConfig] the register file configuration.
      attr_reader :config

      # Creates a new register file from the given configuration.
      #
      # All registers are initialized to 0. If config is nil, the default
      # configuration is used (16 registers, 32-bit, zero register enabled).
      #
      # @param config [RegisterFileConfig, nil] configuration for the register file.
      def initialize(config = nil)
        @config = config || CodingAdventures::Core.default_register_file_config
        @values = Array.new(@config.count, 0)

        # Compute the bit mask for the register width.
        # For 32-bit: mask = 0xFFFFFFFF
        # For 64-bit: mask = max integer (Ruby integers are arbitrary precision)
        @mask = if @config.width >= 64
          (1 << 63) - 1 # max signed 64-bit
        else
          (1 << @config.width) - 1
        end
      end

      # Returns the value of the register at the given index.
      #
      # If the zero register convention is enabled, reading register 0 always
      # returns 0. Returns 0 if the index is out of range (defensive).
      #
      # @param index [Integer] register index.
      # @return [Integer] register value.
      def read(index)
        return 0 if index < 0 || index >= @config.count
        return 0 if @config.zero_register && index == 0
        @values[index]
      end

      # Stores a value into the register at the given index.
      #
      # The value is masked to the register width. Writes to register 0 are
      # silently ignored when the zero register convention is enabled.
      # Writes to out-of-range indices are silently ignored.
      #
      # @param index [Integer] register index.
      # @param value [Integer] value to write.
      def write(index, value)
        return if index < 0 || index >= @config.count
        return if @config.zero_register && index == 0
        @values[index] = value & @mask
      end

      # Returns a copy of all register values (for inspection and debugging).
      #
      # @return [Array<Integer>] copy of all register values.
      def values
        @values.dup
      end

      # Returns the number of registers.
      #
      # @return [Integer] register count.
      def count
        @config.count
      end

      # Returns the bit width of each register.
      #
      # @return [Integer] bit width.
      def width
        @config.width
      end

      # Resets all registers to zero.
      def reset
        @values.fill(0)
      end

      # Returns a human-readable dump of all registers.
      #
      # Format:
      #   RegisterFile(16x32): R1=42 R2=100 ...
      #
      # @return [String] register dump.
      def to_s
        s = "RegisterFile(#{@config.count}x#{@config.width}):"
        @config.count.times do |i|
          s += " R#{i}=#{@values[i]}" if @values[i] != 0
        end
        s
      end
    end
  end
end
