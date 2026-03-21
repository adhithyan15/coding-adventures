# frozen_string_literal: true

# Display driver --- manages writing characters to the framebuffer.
#
# The display driver is the software layer between the OS kernel (which calls
# put_char for each character of output) and the raw framebuffer memory. It
# tracks the cursor position, handles special characters (newline, tab, etc.),
# and triggers scrolling when output exceeds the screen height.

module CodingAdventures
  module Display
    # Manages the framebuffer and cursor state.
    #
    # @example
    #   config = DisplayConfig.new
    #   memory = Array.new(config.columns * config.rows * 2, 0)
    #   driver = DisplayDriver.new(config, memory)
    #   driver.puts_str("Hello World")
    #   snap = driver.snapshot
    #   snap.lines[0]  # => "Hello World"
    class DisplayDriver
      attr_reader :config, :memory
      attr_accessor :cursor_row, :cursor_col

      # Create a display driver backed by the given memory array.
      #
      # The memory array must have at least columns * rows * 2 elements.
      # All cells are initialized to space + default attribute (cleared screen).
      #
      # @param config [DisplayConfig] display dimensions and settings
      # @param memory [Array<Integer>] the framebuffer memory
      def initialize(config, memory)
        @config = config
        @memory = memory
        @cursor_row = 0
        @cursor_col = 0
        clear
      end

      # ============================================================
      # Writing characters
      # ============================================================

      # Write a single character at the current cursor position using
      # the default attribute, then advance the cursor.
      #
      # Special characters:
      #   0x0A (newline):        move to column 0 of the next row
      #   0x0D (carriage return): move to column 0 of the current row
      #   0x09 (tab):            advance to the next multiple of 8
      #   0x08 (backspace):      move cursor left by 1 (does not erase)
      #
      # @param ch [Integer] ASCII character code (0-255)
      def put_char(ch)
        case ch
        when 0x0A # newline
          @cursor_col = 0
          @cursor_row += 1
        when 0x0D # carriage return
          @cursor_col = 0
        when 0x09 # tab
          @cursor_col = (@cursor_col / 8 + 1) * 8
          if @cursor_col >= @config.columns
            @cursor_col = 0
            @cursor_row += 1
          end
        when 0x08 # backspace
          @cursor_col -= 1 if @cursor_col > 0
        else
          # Regular character: write to framebuffer and advance cursor.
          offset = (@cursor_row * @config.columns + @cursor_col) * BYTES_PER_CELL
          if offset >= 0 && (offset + 1) < @memory.length
            @memory[offset] = ch & 0xFF
            @memory[offset + 1] = @config.default_attribute
          end
          @cursor_col += 1

          # Line wrap: past last column -> next row.
          if @cursor_col >= @config.columns
            @cursor_col = 0
            @cursor_row += 1
          end
        end

        # Scroll check: past last row -> scroll up.
        scroll if @cursor_row >= @config.rows
      end

      # Write a character with a specific attribute at the given position.
      # Does NOT move the cursor. Does NOT handle special characters.
      #
      # @param row [Integer] row position (0-based)
      # @param col [Integer] column position (0-based)
      # @param ch [Integer] ASCII character code (0-255)
      # @param attr [Integer] color attribute byte
      def put_char_at(row, col, ch, attr)
        return if row < 0 || row >= @config.rows
        return if col < 0 || col >= @config.columns

        offset = (row * @config.columns + col) * BYTES_PER_CELL
        @memory[offset] = ch & 0xFF
        @memory[offset + 1] = attr & 0xFF
      end

      # Write a string to the display, one character at a time.
      # Named puts_str to avoid conflict with Kernel#puts.
      #
      # @param s [String] the string to write
      def puts_str(s)
        s.each_byte { |ch| put_char(ch) }
      end

      # ============================================================
      # Screen management
      # ============================================================

      # Reset the entire display: fill all cells with space + default
      # attribute, reset cursor to (0, 0).
      def clear
        total_bytes = @config.columns * @config.rows * BYTES_PER_CELL
        (0...total_bytes).step(BYTES_PER_CELL) do |i|
          @memory[i] = 0x20 # space
          @memory[i + 1] = @config.default_attribute
        end
        @cursor_row = 0
        @cursor_col = 0
      end

      # Shift all rows up by one line. Row 1 becomes row 0, row 2 becomes
      # row 1, etc. The last row is cleared. Cursor moves to (last_row, 0).
      def scroll
        bytes_per_row = @config.columns * BYTES_PER_CELL
        total_bytes = @config.rows * bytes_per_row

        # Copy rows 1..N-1 into rows 0..N-2.
        (0...(total_bytes - bytes_per_row)).each do |i|
          @memory[i] = @memory[i + bytes_per_row]
        end

        # Clear the last row.
        last_row_start = (@config.rows - 1) * bytes_per_row
        (last_row_start...total_bytes).step(BYTES_PER_CELL) do |i|
          @memory[i] = 0x20 # space
          @memory[i + 1] = @config.default_attribute
        end

        # Place cursor at beginning of last row.
        @cursor_row = @config.rows - 1
        @cursor_col = 0
      end

      # ============================================================
      # Cursor management
      # ============================================================

      # Move the cursor to the given position, clamped to valid bounds.
      #
      # @param row [Integer] target row (clamped to [0, rows-1])
      # @param col [Integer] target column (clamped to [0, columns-1])
      def set_cursor(row, col)
        @cursor_row = [[row, 0].max, @config.rows - 1].min
        @cursor_col = [[col, 0].max, @config.columns - 1].min
      end

      # Return the current cursor position.
      #
      # @return [CursorPosition]
      def get_cursor
        CursorPosition.new(row: @cursor_row, col: @cursor_col)
      end

      # ============================================================
      # Reading cells
      # ============================================================

      # Return the character and attribute at the given position.
      # Returns Cell(' ', default_attribute) if out of bounds.
      #
      # @param row [Integer] row position
      # @param col [Integer] column position
      # @return [Cell]
      def get_cell(row, col)
        if row < 0 || row >= @config.rows || col < 0 || col >= @config.columns
          return Cell.new(character: 0x20, attribute: @config.default_attribute)
        end

        offset = (row * @config.columns + col) * BYTES_PER_CELL
        Cell.new(character: @memory[offset], attribute: @memory[offset + 1])
      end

      # ============================================================
      # Snapshot
      # ============================================================

      # Return a read-friendly view of the current display state.
      #
      # @return [DisplaySnapshot]
      def snapshot
        lines = (0...@config.rows).map do |row|
          chars = (0...@config.columns).map do |col|
            offset = (row * @config.columns + col) * BYTES_PER_CELL
            @memory[offset].chr
          end
          chars.join.rstrip
        end

        DisplaySnapshot.new(
          lines: lines,
          cursor: CursorPosition.new(row: @cursor_row, col: @cursor_col),
          rows: @config.rows,
          columns: @config.columns
        )
      end
    end
  end
end
