"""Display driver --- manages writing characters to the framebuffer.

The display driver is the software layer between the OS kernel (which calls
put_char for each character of output) and the raw framebuffer memory. It
tracks the cursor position, handles special characters (newline, tab, etc.),
and triggers scrolling when output exceeds the screen height.

Usage:
    >>> config = DisplayConfig()
    >>> memory = bytearray(config.columns * config.rows * BYTES_PER_CELL)
    >>> driver = DisplayDriver(config, memory)
    >>> driver.puts("Hello World\\n")
    >>> snap = driver.snapshot()
    >>> snap.lines[0]
    'Hello World'
"""

from __future__ import annotations

from display.framebuffer import (
    BYTES_PER_CELL,
    Cell,
    CursorPosition,
    DisplayConfig,
)
from display.snapshot import DisplaySnapshot


class DisplayDriver:
    """Manages the framebuffer and cursor state.

    Attributes:
        config: Display dimensions and settings.
        memory: The framebuffer memory (columns * rows * 2 bytes).
        cursor: Current cursor position.
    """

    def __init__(self, config: DisplayConfig, memory: bytearray) -> None:
        """Create a display driver backed by the given memory region.

        The memory must be at least columns * rows * 2 bytes long.
        All cells are initialized to space + default attribute (cleared screen).
        """
        self.config = config
        self.memory = memory
        self.cursor = CursorPosition(row=0, col=0)
        self.clear()

    # ============================================================
    # Writing characters
    # ============================================================

    def put_char(self, ch: int) -> None:
        """Write a single character at the current cursor position.

        Uses the default attribute, then advances the cursor to the right.

        Special characters are handled as control codes, not written to screen:
            0x0A (newline):        move to column 0 of the next row
            0x0D (carriage return): move to column 0 of the current row
            0x09 (tab):            advance to the next multiple of 8
            0x08 (backspace):      move cursor left by 1 (does not erase)

        If the cursor moves past the last column, it wraps to the next row.
        If it moves past the last row, the display scrolls up.

        Args:
            ch: ASCII character code (0-255).
        """
        if ch == 0x0A:  # '\n' newline
            self.cursor.col = 0
            self.cursor.row += 1
        elif ch == 0x0D:  # '\r' carriage return
            self.cursor.col = 0
        elif ch == 0x09:  # '\t' tab
            self.cursor.col = (self.cursor.col // 8 + 1) * 8
            if self.cursor.col >= self.config.columns:
                self.cursor.col = 0
                self.cursor.row += 1
        elif ch == 0x08:  # '\b' backspace
            if self.cursor.col > 0:
                self.cursor.col -= 1
        else:
            # Regular character: write to framebuffer and advance cursor.
            offset = (
                self.cursor.row * self.config.columns + self.cursor.col
            ) * BYTES_PER_CELL
            if 0 <= offset and offset + 1 < len(self.memory):
                self.memory[offset] = ch & 0xFF
                self.memory[offset + 1] = self.config.default_attribute
            self.cursor.col += 1

            # Line wrap: past last column -> next row.
            if self.cursor.col >= self.config.columns:
                self.cursor.col = 0
                self.cursor.row += 1

        # Scroll check: past last row -> scroll up.
        if self.cursor.row >= self.config.rows:
            self.scroll()

    def put_char_at(self, row: int, col: int, ch: int, attr: int) -> None:
        """Write a character with a specific attribute at the given position.

        Unlike put_char, this does NOT move the cursor and does NOT handle
        special characters. It's a raw framebuffer write.

        Out-of-bounds positions are silently ignored.

        Args:
            row: Row position (0-based).
            col: Column position (0-based).
            ch: ASCII character code (0-255).
            attr: Color attribute byte.
        """
        if row < 0 or row >= self.config.rows:
            return
        if col < 0 or col >= self.config.columns:
            return
        offset = (row * self.config.columns + col) * BYTES_PER_CELL
        self.memory[offset] = ch & 0xFF
        self.memory[offset + 1] = attr & 0xFF

    def puts(self, s: str) -> None:
        """Write a string to the display, one character at a time.

        Each character goes through put_char's cursor-advance and
        special-character handling.

        Args:
            s: The string to write.
        """
        for ch in s:
            self.put_char(ord(ch))

    # ============================================================
    # Screen management
    # ============================================================

    def clear(self) -> None:
        """Reset the entire display.

        Every cell becomes space (0x20) with the default attribute,
        and the cursor returns to (0, 0). This is the equivalent of
        the "cls" command on DOS or "clear" on Unix.
        """
        total_bytes = self.config.columns * self.config.rows * BYTES_PER_CELL
        for i in range(0, total_bytes, BYTES_PER_CELL):
            if i + 1 < len(self.memory):
                self.memory[i] = ord(" ")
                self.memory[i + 1] = self.config.default_attribute
        self.cursor.row = 0
        self.cursor.col = 0

    def scroll(self) -> None:
        """Shift all rows up by one line.

        Row 1 becomes row 0, row 2 becomes row 1, etc. The last row
        is cleared (filled with spaces + default attribute). The cursor
        is placed at (last_row, 0).

        Implementation: a single memory copy moves rows 1-24 into
        rows 0-23, then we clear row 24.
        """
        bytes_per_row = self.config.columns * BYTES_PER_CELL
        total_bytes = self.config.rows * bytes_per_row

        # Copy rows 1..N-1 into rows 0..N-2.
        self.memory[0 : total_bytes - bytes_per_row] = self.memory[
            bytes_per_row:total_bytes
        ]

        # Clear the last row.
        last_row_start = (self.config.rows - 1) * bytes_per_row
        for i in range(last_row_start, total_bytes, BYTES_PER_CELL):
            self.memory[i] = ord(" ")
            self.memory[i + 1] = self.config.default_attribute

        # Place cursor at beginning of last row.
        self.cursor.row = self.config.rows - 1
        self.cursor.col = 0

    # ============================================================
    # Cursor management
    # ============================================================

    def set_cursor(self, row: int, col: int) -> None:
        """Move the cursor to the given position.

        Row and column are clamped to valid bounds.

        Args:
            row: Target row (clamped to [0, rows-1]).
            col: Target column (clamped to [0, columns-1]).
        """
        self.cursor.row = max(0, min(row, self.config.rows - 1))
        self.cursor.col = max(0, min(col, self.config.columns - 1))

    def get_cursor(self) -> CursorPosition:
        """Return the current cursor position."""
        return CursorPosition(row=self.cursor.row, col=self.cursor.col)

    # ============================================================
    # Reading cells
    # ============================================================

    def get_cell(self, row: int, col: int) -> Cell:
        """Return the character and attribute at the given position.

        Returns Cell(' ', default_attribute) if out of bounds.

        Args:
            row: Row position.
            col: Column position.

        Returns:
            Cell with character and attribute.
        """
        if row < 0 or row >= self.config.rows:
            return Cell(character=ord(" "), attribute=self.config.default_attribute)
        if col < 0 or col >= self.config.columns:
            return Cell(character=ord(" "), attribute=self.config.default_attribute)
        offset = (row * self.config.columns + col) * BYTES_PER_CELL
        return Cell(
            character=self.memory[offset],
            attribute=self.memory[offset + 1],
        )

    # ============================================================
    # Snapshot
    # ============================================================

    def snapshot(self) -> DisplaySnapshot:
        """Return a read-friendly view of the current display state.

        Each row is extracted as a string with trailing spaces trimmed.

        Returns:
            DisplaySnapshot with lines, cursor, rows, and columns.
        """
        lines: list[str] = []
        for row in range(self.config.rows):
            chars: list[str] = []
            for col in range(self.config.columns):
                offset = (row * self.config.columns + col) * BYTES_PER_CELL
                chars.append(chr(self.memory[offset]))
            lines.append("".join(chars).rstrip(" "))

        return DisplaySnapshot(
            lines=lines,
            cursor=CursorPosition(row=self.cursor.row, col=self.cursor.col),
            rows=self.config.rows,
            columns=self.config.columns,
        )
