# frozen_string_literal: true

module CodingAdventures
  module DeviceDriverFramework
    # SimulatedDisplay — a character device that renders bytes to a framebuffer.
    #
    # In a real computer, the display is a memory-mapped device: a region of
    # RAM (the framebuffer) is directly connected to the display hardware.
    # Writing a byte to the framebuffer makes a character appear on screen.
    #
    # Our SimulatedDisplay uses an in-memory array as the framebuffer. The
    # classic text-mode display is 80 columns by 25 rows, with 2 bytes per
    # character cell (one for the ASCII character, one for the color attribute).
    # That is 80 * 25 * 2 = 4000 bytes total.
    #
    # Framebuffer layout (VGA text mode convention):
    #   Byte 0: character at row 0, col 0
    #   Byte 1: color attribute at row 0, col 0
    #   Byte 2: character at row 0, col 1
    #   Byte 3: color attribute at row 0, col 1
    #   ...
    #   Byte 158: character at row 0, col 79
    #   Byte 159: color attribute at row 0, col 79
    #   Byte 160: character at row 1, col 0
    #   ...
    #
    # Why is read() not supported? A display is an output-only device. You
    # write characters to it; you cannot "read" what is on screen through
    # the character device interface. (In practice, you could read the
    # framebuffer directly, but that is a different API.)
    #
    # Example:
    #   display = SimulatedDisplay.new
    #   display.init
    #   display.write([0x48, 0x69])  # Write "Hi" to the screen
    class SimulatedDisplay < CharacterDevice
      COLS = 80
      ROWS = 25
      BYTES_PER_CELL = 2
      FRAMEBUFFER_SIZE = COLS * ROWS * BYTES_PER_CELL  # 4000 bytes
      DEFAULT_COLOR = 0x07  # Light gray on black (VGA default)

      attr_reader :framebuffer, :cursor_row, :cursor_col

      # @param name [String] Device name (default "display0")
      # @param minor [Integer] Minor number (default 0)
      def initialize(name: "display0", minor: 0)
        super(
          name: name,
          major: 1,
          minor: minor,
          interrupt_number: -1  # Display does not generate interrupts
        )
        @framebuffer = Array.new(FRAMEBUFFER_SIZE, 0)
        @cursor_row = 0
        @cursor_col = 0
      end

      # Initialize the display by clearing the screen and resetting the cursor.
      #
      # In real hardware, init() would send a reset sequence to the display
      # controller, configure the video mode, and clear video RAM.
      def init
        super
        clear_screen
      end

      # Read from the display — always fails.
      #
      # Displays are output-only through the character device interface.
      #
      # @param _count [Integer] Ignored
      # @return [Integer] Always -1 (error)
      def read(_count)
        -1
      end

      # Write bytes to the display at the current cursor position.
      #
      # Each byte is treated as an ASCII character and placed into the
      # framebuffer at the current cursor position with the default color
      # attribute. The cursor advances after each character.
      #
      # When the cursor reaches the end of a row, it wraps to the next row.
      # When it reaches the bottom of the screen, it wraps back to the top.
      # (A real display would scroll; our simulation wraps for simplicity.)
      #
      # @param data [Array<Integer>] ASCII bytes to display
      # @return [Integer] Number of bytes written
      def write(data)
        data.each do |byte|
          put_char(byte)
        end
        data.length
      end

      # Place a single character at the current cursor position.
      #
      # The framebuffer uses VGA text mode layout:
      #   offset = (row * COLS + col) * BYTES_PER_CELL
      #   framebuffer[offset]     = character (ASCII)
      #   framebuffer[offset + 1] = color attribute
      #
      # @param byte [Integer] ASCII value of the character
      def put_char(byte)
        offset = (@cursor_row * COLS + @cursor_col) * BYTES_PER_CELL
        @framebuffer[offset] = byte
        @framebuffer[offset + 1] = DEFAULT_COLOR
        advance_cursor
      end

      # Clear the entire screen by zeroing the framebuffer and resetting
      # the cursor to position (0, 0).
      def clear_screen
        @framebuffer.fill(0)
        @cursor_row = 0
        @cursor_col = 0
      end

      # Read the character at a specific (row, col) position.
      #
      # @param row [Integer] Row (0-24)
      # @param col [Integer] Column (0-79)
      # @return [Integer] ASCII value at that position
      def char_at(row, col)
        offset = (row * COLS + col) * BYTES_PER_CELL
        @framebuffer[offset]
      end

      private

      # Move the cursor forward by one position. Wraps at end of row and
      # at bottom of screen.
      def advance_cursor
        @cursor_col += 1
        if @cursor_col >= COLS
          @cursor_col = 0
          @cursor_row += 1
          if @cursor_row >= ROWS
            @cursor_row = 0  # Wrap to top (simple, no scrolling)
          end
        end
      end
    end
  end
end
