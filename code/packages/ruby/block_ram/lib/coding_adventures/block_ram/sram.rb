# frozen_string_literal: true

# ---------------------------------------------------------------------------
# SRAM -- Static Random-Access Memory at the gate level.
# ---------------------------------------------------------------------------
#
# === What is SRAM? ===
#
# SRAM (Static Random-Access Memory) is the fastest type of memory in a
# computer. It is used for CPU caches (L1/L2/L3), register files, and FPGA
# Block RAM. "Static" means the memory holds its value as long as power is
# supplied -- unlike DRAM, which must be periodically refreshed.
#
# === The SRAM Cell -- 6 Transistors Holding 1 Bit ===
#
# In real hardware, each SRAM cell uses 6 transistors:
# - 2 cross-coupled inverters forming a bistable latch (stores the bit)
# - 2 access transistors controlled by the word line (gates read/write)
#
# We model this at the gate level:
# - Cross-coupled inverters = two NOT gates in a feedback loop
# - Access transistors = AND gates that pass data only when word_line=1
#
# The cell has three operations:
# - **Hold** (word_line=0): Access transistors block external access.
#   The inverter loop maintains the stored value indefinitely.
# - **Read** (word_line=1): Access transistors open. The stored value
#   appears on the bit lines without disturbing it.
# - **Write** (word_line=1 + drive bit lines): The external driver
#   overpowers the internal inverters, forcing a new value.
#
# === From Cell to Array ===
#
# A RAM chip is a 2D grid of SRAM cells. To access a specific cell:
# 1. A **row decoder** converts address bits into a one-hot word line signal
# 2. A **column MUX** selects which columns to read/write
#
# This module provides:
# - SRAMCell: single-bit storage at the gate level
# - SRAMArray: 2D grid with row/column addressing
# ---------------------------------------------------------------------------

module CodingAdventures
  module BlockRam
    # Single-bit storage element modeled at the gate level.
    #
    # Internally, this is a pair of cross-coupled inverters (forming a
    # bistable latch) gated by access transistors controlled by the word line.
    #
    # In our simulation, we model the steady-state behavior directly:
    # - word_line=0: cell is isolated, value is retained
    # - word_line=1, reading: value is output
    # - word_line=1, writing: new value overwrites stored value
    #
    # @example
    #   cell = SRAMCell.new
    #   cell.value         # => 0
    #   cell.write(1, 1)   # word_line=1, bit_line=1 -> stores 1
    #   cell.value         # => 1
    #   cell.read(1)       # => 1
    #   cell.read(0)       # => nil (not selected)
    class SRAMCell
      def initialize
        @value = 0
      end

      # Read the stored bit if the cell is selected.
      #
      # @param word_line [Integer] 1 = cell selected, 0 = cell not selected
      # @return [Integer, nil] stored bit (0 or 1) when selected, nil otherwise
      def read(word_line)
        BlockRam.validate_bit(word_line, "word_line")
        return nil if word_line == 0

        @value
      end

      # Write a bit to the cell if selected.
      #
      # When word_line=1, the access transistors open and the external
      # bit_line driver overpowers the internal inverter loop.
      # When word_line=0, the write has no effect.
      #
      # @param word_line [Integer] 1 = cell selected, 0 = cell not selected
      # @param bit_line [Integer] the value to store (0 or 1)
      def write(word_line, bit_line)
        BlockRam.validate_bit(word_line, "word_line")
        BlockRam.validate_bit(bit_line, "bit_line")

        @value = bit_line if word_line == 1
      end

      # Current stored value (for inspection/debugging).
      #
      # @return [Integer] the stored bit (0 or 1)
      attr_reader :value
    end

    # 2D grid of SRAM cells with row/column addressing.
    #
    # An SRAM array organizes cells into rows and columns:
    # - Each row shares a word line (activated by the row decoder)
    # - Each column shares a bit line (carries data in/out)
    #
    # Memory map (4x4 array example):
    #
    #     Row 0 (WL0): [Cell00] [Cell01] [Cell02] [Cell03]
    #     Row 1 (WL1): [Cell10] [Cell11] [Cell12] [Cell13]
    #     Row 2 (WL2): [Cell20] [Cell21] [Cell22] [Cell23]
    #     Row 3 (WL3): [Cell30] [Cell31] [Cell32] [Cell33]
    #
    # @example
    #   arr = SRAMArray.new(4, 8)
    #   arr.write(0, [1, 0, 1, 0, 0, 1, 0, 1])
    #   arr.read(0)   # => [1, 0, 1, 0, 0, 1, 0, 1]
    #   arr.read(1)   # => [0, 0, 0, 0, 0, 0, 0, 0]
    class SRAMArray
      # @param rows [Integer] number of rows (>= 1)
      # @param cols [Integer] number of columns (>= 1)
      # @raise [ArgumentError] if rows or cols < 1
      def initialize(rows, cols)
        if rows < 1
          raise ArgumentError, "rows must be >= 1, got #{rows}"
        end
        if cols < 1
          raise ArgumentError, "cols must be >= 1, got #{cols}"
        end

        @rows = rows
        @cols = cols
        @cells = Array.new(rows) { Array.new(cols) { SRAMCell.new } }
      end

      # Read all columns of a row.
      #
      # Activates the word line for the given row, causing all cells
      # in that row to output their stored values.
      #
      # @param row [Integer] row index (0 to rows-1)
      # @return [Array<Integer>] list of bits, one per column
      # @raise [ArgumentError] if row is out of range
      def read(row)
        validate_row(row)
        @cells[row].map { |cell| cell.read(1) }
      end

      # Write data to a row.
      #
      # Activates the word line for the given row and drives the bit
      # lines with the given data.
      #
      # @param row [Integer] row index (0 to rows-1)
      # @param data [Array<Integer>] bits to write, one per column
      # @raise [TypeError] if data is not an Array
      # @raise [ArgumentError] if data length does not match cols
      def write(row, data)
        validate_row(row)

        unless data.is_a?(Array)
          raise TypeError, "data must be an Array of bits"
        end

        if data.length != @cols
          raise ArgumentError,
            "data length #{data.length} does not match cols #{@cols}"
        end

        data.each_with_index { |bit, i| BlockRam.validate_bit(bit, "data[#{i}]") }

        data.each_with_index do |bit, col|
          @cells[row][col].write(1, bit)
        end
      end

      # Array dimensions as [rows, cols].
      #
      # @return [Array(Integer, Integer)]
      def shape
        [@rows, @cols]
      end

      private

      def validate_row(row)
        unless row.is_a?(Integer) && !row.is_a?(TrueClass) && !row.is_a?(FalseClass)
          raise TypeError, "row must be an Integer, got #{row.class}"
        end
        if row < 0 || row >= @rows
          raise ArgumentError, "row #{row} out of range [0, #{@rows - 1}]"
        end
      end
    end

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
