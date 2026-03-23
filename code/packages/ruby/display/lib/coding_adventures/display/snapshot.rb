# frozen_string_literal: true

# Display snapshot --- a read-friendly view of the framebuffer.
#
# The snapshot converts raw framebuffer bytes into human-readable strings.
# It is the primary interface for tests and for the boot trace. Snapshots
# are immutable --- they capture the display state at one moment in time.

module CodingAdventures
  module Display
    # A frozen view of the display's text content.
    class DisplaySnapshot
      attr_reader :lines, :cursor, :rows, :columns

      # @param lines [Array<String>] text content of each row (trailing spaces trimmed)
      # @param cursor [CursorPosition] cursor position at snapshot time
      # @param rows [Integer] number of rows
      # @param columns [Integer] number of columns
      def initialize(lines:, cursor:, rows:, columns:)
        @lines = lines.freeze
        @cursor = cursor
        @rows = rows
        @columns = columns
      end

      # Return the full display as a multi-line string.
      # Each line is padded to the full column width.
      #
      # @return [String]
      def to_s
        @lines.map { |line| line.ljust(@columns) }.join("\n")
      end

      # Return true if the given text appears anywhere in the display.
      # Searches each line independently.
      #
      # @param text [String] the text to search for
      # @return [Boolean]
      def contains?(text)
        @lines.any? { |line| line.include?(text) }
      end

      # Return the text content of a specific row (trailing spaces trimmed).
      # Returns "" if the row is out of bounds.
      #
      # @param row [Integer] row index (0-based)
      # @return [String]
      def line_at(row)
        return "" if row < 0 || row >= @lines.length

        @lines[row]
      end
    end
  end
end
