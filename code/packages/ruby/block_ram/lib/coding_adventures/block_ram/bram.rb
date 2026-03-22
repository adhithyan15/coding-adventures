# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Configurable Block RAM -- FPGA-style memory with reconfigurable aspect ratio.
# ---------------------------------------------------------------------------
#
# === What is Block RAM? ===
#
# In an FPGA, Block RAM (BRAM) tiles are dedicated memory blocks separate
# from the configurable logic. Each tile has a fixed total storage (typically
# 18 Kbit or 36 Kbit) but can be configured with different width/depth ratios:
#
#     18 Kbit BRAM configurations:
#     +----------------+-------+-------+------------+
#     | Configuration  | Depth | Width | Total bits |
#     +----------------+-------+-------+------------+
#     | 16K x 1        | 16384 |     1 |      16384 |
#     |  8K x 2        |  8192 |     2 |      16384 |
#     |  4K x 4        |  4096 |     4 |      16384 |
#     |  2K x 8        |  2048 |     8 |      16384 |
#     |  1K x 16       |  1024 |    16 |      16384 |
#     | 512 x 32       |   512 |    32 |      16384 |
#     +----------------+-------+-------+------------+
#
# The total storage is fixed; you trade depth for width by changing how the
# address decoder and column MUX are configured.
#
# This module wraps DualPortRAM with reconfiguration support.
# ---------------------------------------------------------------------------

module CodingAdventures
  module BlockRam
    # Block RAM with configurable aspect ratio.
    #
    # Total storage is fixed at initialization. Width and depth can be
    # reconfigured as long as width * depth <= total_bits.
    #
    # Supports dual-port access when dual_port=true (default).
    #
    # @example
    #   bram = ConfigurableBRAM.new(total_bits: 1024, width: 8)
    #   bram.depth   # => 128
    #   bram.reconfigure(width: 16)
    #   bram.depth   # => 64
    class ConfigurableBRAM
      attr_reader :depth, :width, :total_bits

      # @param total_bits [Integer] total storage in bits (default 18432 = 18 Kbit)
      # @param width [Integer] initial bits per word (default 8)
      # @param dual_port [Boolean] enable dual-port access (default true)
      def initialize(total_bits: 18_432, width: 8, dual_port: true)
        if total_bits < 1
          raise ArgumentError, "total_bits must be >= 1, got #{total_bits}"
        end
        if width < 1
          raise ArgumentError, "width must be >= 1, got #{width}"
        end
        if total_bits % width != 0
          raise ArgumentError,
            "width #{width} does not evenly divide total_bits #{total_bits}"
        end

        @total_bits = total_bits
        @width = width
        @dual_port = dual_port
        @depth = total_bits / width
        @ram = DualPortRAM.new(depth: @depth, width: @width)
        @prev_clock = 0
        @last_read_a = Array.new(width, 0)
        @last_read_b = Array.new(width, 0)
      end

      # Change the aspect ratio. Clears all stored data.
      #
      # @param width [Integer] new bits per word. Must evenly divide total_bits.
      # @raise [ArgumentError] if width does not divide total_bits or is < 1
      def reconfigure(width:)
        if width < 1
          raise ArgumentError, "width must be >= 1, got #{width}"
        end
        if @total_bits % width != 0
          raise ArgumentError,
            "width #{width} does not evenly divide total_bits #{@total_bits}"
        end

        @width = width
        @depth = @total_bits / width
        @ram = DualPortRAM.new(depth: @depth, width: @width)
        @prev_clock = 0
        @last_read_a = Array.new(width, 0)
        @last_read_b = Array.new(width, 0)
      end

      # Port A operation.
      #
      # @param clock [Integer] clock signal (0 or 1)
      # @param address [Integer] word address (0 to depth-1)
      # @param data_in [Array<Integer>] write data (list of width bits)
      # @param write_enable [Integer] 0 = read, 1 = write
      # @return [Array<Integer>] data_out (list of width bits)
      def tick_a(clock, address:, data_in:, write_enable:)
        BlockRam.validate_bit(clock, "clock")
        zeros = Array.new(@width, 0)
        out_a, = @ram.tick(clock,
          address_a: address, data_in_a: data_in, write_enable_a: write_enable,
          address_b: 0, data_in_b: zeros, write_enable_b: 0)
        out_a
      end

      # Port B operation.
      #
      # @param clock [Integer] clock signal (0 or 1)
      # @param address [Integer] word address (0 to depth-1)
      # @param data_in [Array<Integer>] write data (list of width bits)
      # @param write_enable [Integer] 0 = read, 1 = write
      # @return [Array<Integer>] data_out (list of width bits)
      def tick_b(clock, address:, data_in:, write_enable:)
        BlockRam.validate_bit(clock, "clock")
        zeros = Array.new(@width, 0)
        _, out_b = @ram.tick(clock,
          address_a: 0, data_in_a: zeros, write_enable_a: 0,
          address_b: address, data_in_b: data_in, write_enable_b: write_enable)
        out_b
      end
    end
  end
end
