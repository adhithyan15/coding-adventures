"""SimulatedDisplay -- a character device representing a text-mode display.

==========================================================================
How Text-Mode Displays Work
==========================================================================

Early computers did not have graphical displays. Instead, they had
"text-mode" displays that showed a grid of characters -- typically 80
columns by 25 rows (inherited from the IBM PC's VGA text mode).

Each cell in the grid stores TWO bytes:
  - Byte 0: the ASCII character to display (e.g., 0x48 = 'H')
  - Byte 1: the attribute byte (foreground color, background color, blink)

So the total framebuffer size is: 80 * 25 * 2 = 4000 bytes.

  Memory layout of the framebuffer:
  ┌────────────────────────────────────────────────────────────────┐
  │ Row 0: [char0][attr0] [char1][attr1] ... [char79][attr79]     │
  │ Row 1: [char0][attr0] [char1][attr1] ... [char79][attr79]     │
  │ ...                                                            │
  │ Row 24: [char0][attr0] [char1][attr1] ... [char79][attr79]    │
  └────────────────────────────────────────────────────────────────┘

The cursor position tracks where the NEXT character will be written.
When you write a character, it goes at the cursor position, and the
cursor advances one cell to the right (wrapping to the next line at
column 80).

==========================================================================
SimulatedDisplay Design
==========================================================================

The display is a WRITE-ONLY character device: you write characters to it,
and they appear on screen. Reading from a display makes no sense (you
can't "read" light off a monitor), so read() returns -1.

The display does NOT generate interrupts (interrupt_number = -1). Unlike
a keyboard (which interrupts when a key is pressed) or a disk (which
interrupts when I/O completes), the display is always ready to accept
more characters. The CPU writes whenever it wants.
"""

from device_driver_framework.device import CharacterDevice

# Display dimensions (VGA text mode standard)
DISPLAY_COLS = 80
DISPLAY_ROWS = 25

# Each cell is 2 bytes: character + attribute
BYTES_PER_CELL = 2

# Total framebuffer size
FRAMEBUFFER_SIZE = DISPLAY_COLS * DISPLAY_ROWS * BYTES_PER_CELL

# Default attribute: light gray on black (standard VGA text mode)
DEFAULT_ATTRIBUTE = 0x07


class SimulatedDisplay(CharacterDevice):
    """A simulated text-mode display with an 80x25 framebuffer.

    Characters written to this device appear in the framebuffer at the
    current cursor position. The cursor automatically advances after
    each character.

    Args:
        name: Device name (default "display0").
        minor: Minor number (default 0).
    """

    def __init__(
        self,
        name: str = "display0",
        minor: int = 0,
    ) -> None:
        super().__init__(
            name=name,
            major=1,  # Major 1 = display driver (from the spec)
            minor=minor,
            interrupt_number=-1,  # Display does not generate interrupts
        )
        # The framebuffer: 80 * 25 * 2 = 4000 bytes
        # Initially filled with spaces (0x20) and default attributes (0x07)
        self._framebuffer = bytearray(FRAMEBUFFER_SIZE)
        # Cursor position (row, col)
        self._cursor_row = 0
        self._cursor_col = 0

    def init(self) -> None:
        """Initialize the display by clearing the screen.

        Sets every cell to a space character (0x20) with the default
        attribute (0x07 = light gray on black). Resets cursor to (0, 0).

        On a real VGA display, initialization would also involve:
        - Setting the video mode (text vs. graphics)
        - Programming the CRT controller registers
        - Configuring the cursor shape and blink rate
        """
        self.clear_screen()
        self.initialized = True

    def clear_screen(self) -> None:
        """Fill the entire framebuffer with spaces.

        Each cell gets: [0x20 (space), 0x07 (default attribute)]
        The cursor resets to position (0, 0) -- the top-left corner.
        """
        for i in range(DISPLAY_COLS * DISPLAY_ROWS):
            offset = i * BYTES_PER_CELL
            self._framebuffer[offset] = 0x20  # space character
            self._framebuffer[offset + 1] = DEFAULT_ATTRIBUTE
        self._cursor_row = 0
        self._cursor_col = 0

    def read(self, count: int) -> bytes:
        """Attempt to read from the display (always fails).

        You cannot read from a display -- it is an output-only device.
        Returns b"" to indicate no data available (matching the convention
        for character devices where read returns empty on failure for
        write-only devices).
        """
        return b""

    def write(self, data: bytes) -> int:
        """Write characters to the display at the current cursor position.

        Each byte in `data` is treated as an ASCII character. The character
        is placed in the framebuffer at the cursor position with the default
        attribute byte. The cursor advances one cell to the right after each
        character, wrapping to the next line at column 80.

        Special characters:
          - 0x0A (newline): moves cursor to start of next line
          - All other bytes: written as-is to the framebuffer

        Args:
            data: The bytes to write (each byte = one character).

        Returns:
            The number of bytes written (always equals len(data)).
        """
        bytes_written = 0
        for byte in data:
            if byte == 0x0A:  # newline
                self._cursor_col = 0
                self._cursor_row += 1
            else:
                self._put_char_at(self._cursor_row, self._cursor_col, byte)
                self._cursor_col += 1

            # Wrap to next line if past the right edge
            if self._cursor_col >= DISPLAY_COLS:
                self._cursor_col = 0
                self._cursor_row += 1

            # Scroll if past the bottom
            if self._cursor_row >= DISPLAY_ROWS:
                self._scroll_up()
                self._cursor_row = DISPLAY_ROWS - 1

            bytes_written += 1
        return bytes_written

    def _put_char_at(self, row: int, col: int, char: int) -> None:
        """Place a character at a specific (row, col) position.

        Each cell in the framebuffer is 2 bytes:
          [character byte, attribute byte]

        The offset calculation:
          offset = (row * DISPLAY_COLS + col) * BYTES_PER_CELL

        Args:
            row: Row (0 = top).
            col: Column (0 = left).
            char: ASCII value of the character.
        """
        offset = (row * DISPLAY_COLS + col) * BYTES_PER_CELL
        self._framebuffer[offset] = char
        self._framebuffer[offset + 1] = DEFAULT_ATTRIBUTE

    def _scroll_up(self) -> None:
        """Scroll the display up by one line.

        This is what happens when you print past the bottom of the screen:
        - Every row moves up by one (row 1 becomes row 0, row 2 becomes
          row 1, etc.)
        - The last row is filled with spaces
        - Row 0 is lost (scrolled off the top)

        The implementation copies each row's bytes to the row above it,
        then clears the last row.
        """
        # Copy rows 1..24 to rows 0..23
        row_bytes = DISPLAY_COLS * BYTES_PER_CELL
        self._framebuffer[0 : row_bytes * (DISPLAY_ROWS - 1)] = (
            self._framebuffer[row_bytes : row_bytes * DISPLAY_ROWS]
        )
        # Clear the last row
        last_row_start = (DISPLAY_ROWS - 1) * row_bytes
        for i in range(DISPLAY_COLS):
            offset = last_row_start + i * BYTES_PER_CELL
            self._framebuffer[offset] = 0x20
            self._framebuffer[offset + 1] = DEFAULT_ATTRIBUTE

    def char_at(self, row: int, col: int) -> int:
        """Read the character at a specific position (for testing).

        Args:
            row: Row (0 = top).
            col: Column (0 = left).

        Returns:
            The ASCII value of the character at that position.
        """
        offset = (row * DISPLAY_COLS + col) * BYTES_PER_CELL
        return self._framebuffer[offset]

    def attr_at(self, row: int, col: int) -> int:
        """Read the attribute byte at a specific position (for testing).

        Args:
            row: Row (0 = top).
            col: Column (0 = left).

        Returns:
            The attribute byte at that position.
        """
        offset = (row * DISPLAY_COLS + col) * BYTES_PER_CELL
        return self._framebuffer[offset + 1]

    @property
    def cursor_position(self) -> tuple[int, int]:
        """Return the current cursor position as (row, col)."""
        return (self._cursor_row, self._cursor_col)

    @property
    def framebuffer(self) -> bytearray:
        """Direct access to the framebuffer (for testing/debugging)."""
        return self._framebuffer
