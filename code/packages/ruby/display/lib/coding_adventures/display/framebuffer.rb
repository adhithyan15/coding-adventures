# frozen_string_literal: true

# Framebuffer data structures and constants for VGA text-mode display.
#
# A framebuffer is a region of memory that directly maps to what appears on
# screen. In VGA text mode, the framebuffer is an array of cells, where each
# cell is 2 bytes: one byte for the ASCII character and one byte for the
# color attribute.
#
# Think of it like a wall of Post-it notes: 80 columns wide and 25 rows tall.
# Each note holds one character and has a color.

module CodingAdventures
  module Display
    # ============================================================
    # Constants --- the fundamental parameters of VGA text mode
    # ============================================================

    # Each cell is 2 bytes: byte 0 = character, byte 1 = attribute.
    BYTES_PER_CELL = 2

    # Standard VGA text mode dimensions.
    DEFAULT_COLUMNS = 80
    DEFAULT_ROWS = 25

    # Memory-mapped base address. We use 0xFFFB0000 to avoid conflicts
    # with program memory. On real x86 hardware, VGA lives at 0xB8000.
    DEFAULT_FRAMEBUFFER_BASE = 0xFFFB0000

    # Light gray on black (0x07). The classic terminal appearance.
    DEFAULT_ATTRIBUTE = 0x07

    # ============================================================
    # Color constants --- the VGA color palette
    # ============================================================

    COLOR_BLACK = 0
    COLOR_BLUE = 1
    COLOR_GREEN = 2
    COLOR_CYAN = 3
    COLOR_RED = 4
    COLOR_MAGENTA = 5
    COLOR_BROWN = 6
    COLOR_LIGHT_GRAY = 7
    COLOR_DARK_GRAY = 8
    COLOR_LIGHT_BLUE = 9
    COLOR_LIGHT_GREEN = 10
    COLOR_LIGHT_CYAN = 11
    COLOR_LIGHT_RED = 12
    COLOR_LIGHT_MAGENTA = 13
    COLOR_YELLOW = 14
    COLOR_WHITE = 15

    # Combine foreground and background colors into an attribute byte.
    #
    # The foreground occupies the low 4 bits, the background occupies bits 4-6.
    #
    # @param fg [Integer] foreground color (0-15)
    # @param bg [Integer] background color (0-7)
    # @return [Integer] the combined attribute byte
    #
    # @example
    #   make_attribute(COLOR_WHITE, COLOR_BLUE)  #=> 0x1F
    def self.make_attribute(fg, bg)
      ((bg & 0x07) << 4) | (fg & 0x0F)
    end

    # ============================================================
    # Data structures
    # ============================================================

    # A single character position in the framebuffer.
    # Each cell stores the visible character (as integer 0-255) and
    # its color attribute byte.
    Cell = Data.define(:character, :attribute)

    # Tracks the row and column of the cursor.
    # Row 0 is the top of the screen, column 0 is the left edge.
    CursorPosition = Data.define(:row, :col)

    # Parameters for the display dimensions and memory mapping.
    # The default configuration matches VGA text mode: 80x25.
    class DisplayConfig
      attr_reader :columns, :rows, :framebuffer_base, :default_attribute

      def initialize(
        columns: DEFAULT_COLUMNS,
        rows: DEFAULT_ROWS,
        framebuffer_base: DEFAULT_FRAMEBUFFER_BASE,
        default_attribute: DEFAULT_ATTRIBUTE
      )
        @columns = columns
        @rows = rows
        @framebuffer_base = framebuffer_base
        @default_attribute = default_attribute
      end
    end

    # Predefined configurations.

    # Standard VGA text mode (80x25).
    VGA_80X25 = DisplayConfig.new

    # Compact mode for testing (40x10).
    COMPACT_40X10 = DisplayConfig.new(columns: 40, rows: 10)
  end
end
