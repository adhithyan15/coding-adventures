# frozen_string_literal: true

# Entry point for the coding_adventures_fpga gem.
#
# This gem implements a simplified but structurally accurate FPGA model:
# LUTs, Slices, CLBs, Switch Matrices, I/O Blocks, Bitstream configuration,
# and the top-level FPGA fabric.
#
# Usage:
#   require "coding_adventures_fpga"
#
#   lut = CodingAdventures::FPGA::LUT.new(k: 4)
#   lut.configure([0]*16.tap { |t| t[3] = 1 })  # AND gate
#   lut.evaluate([1, 1, 0, 0])  # => 1

require "coding_adventures_logic_gates"
require "coding_adventures_block_ram"

require_relative "coding_adventures/fpga/version"
require_relative "coding_adventures/fpga/lut"
require_relative "coding_adventures/fpga/slice"
require_relative "coding_adventures/fpga/clb"
require_relative "coding_adventures/fpga/switch_matrix"
require_relative "coding_adventures/fpga/io_block"
require_relative "coding_adventures/fpga/bitstream"
require_relative "coding_adventures/fpga/fabric"

module CodingAdventures
  module FPGA
    # Module-level validation helper
    #
    # @param value [Object] the value to validate
    # @param name [String] parameter name for error messages
    # @raise [TypeError] if value is not an Integer
    # @raise [ArgumentError] if value is not 0 or 1
    def self.validate_bit(value, name = "input")
      unless value.is_a?(Integer)
        raise TypeError, "#{name} must be an Integer, got #{value.class}"
      end

      unless value == 0 || value == 1
        raise ArgumentError, "#{name} must be 0 or 1, got #{value}"
      end
    end
  end
end
