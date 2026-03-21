"""Display snapshot --- a read-friendly view of the framebuffer.

The snapshot converts raw framebuffer bytes into human-readable strings.
It is the primary interface for tests and for the boot trace in the
SystemBoard (S06). When you want to check "what is currently on screen,"
you take a snapshot and inspect its lines or use contains().

Snapshots are immutable --- they capture the display state at one moment
in time. Subsequent writes to the framebuffer do not affect an existing
snapshot.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from display.framebuffer import CursorPosition


@dataclass
class DisplaySnapshot:
    """A frozen view of the display's text content.

    Attributes:
        lines: Text content of each row (trailing spaces trimmed).
        cursor: Cursor position at snapshot time.
        rows: Number of rows.
        columns: Number of columns.
    """

    lines: list[str] = field(default_factory=list)
    cursor: CursorPosition = field(default_factory=CursorPosition)
    rows: int = 0
    columns: int = 0

    def string(self) -> str:
        """Return the full display as a multi-line string.

        Each line is padded to the full column width. Lines are joined
        with newlines. This produces a faithful text rendering of the
        entire screen.
        """
        padded_lines: list[str] = []
        for line in self.lines:
            padded = line + " " * (self.columns - len(line))
            padded_lines.append(padded)
        return "\n".join(padded_lines)

    def contains(self, text: str) -> bool:
        """Return True if the given text appears anywhere in the display.

        Searches each line independently --- the text must fit within a
        single row (it does not span across line boundaries).

        Args:
            text: The text to search for.

        Returns:
            True if found on any row.
        """
        return any(text in line for line in self.lines)

    def line_at(self, row: int) -> str:
        """Return the text content of a specific row (trailing spaces trimmed).

        Returns "" if the row is out of bounds.

        Args:
            row: Row index (0-based).

        Returns:
            The row's text content.
        """
        if row < 0 or row >= len(self.lines):
            return ""
        return self.lines[row]
