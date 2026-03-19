"""Framebuffer data structures and constants for VGA text-mode display.

A framebuffer is a region of memory that directly maps to what appears on
screen. In VGA text mode, the framebuffer is an array of cells, where each
cell is 2 bytes: one byte for the ASCII character and one byte for the
color attribute. Writing a byte into the framebuffer instantly changes what
appears on screen.

Think of it like a wall of Post-it notes: 80 columns wide and 25 rows tall.
Each note holds one character and has a color. To display text, you write
characters one by one onto the notes, moving left to right, top to bottom.
"""

from __future__ import annotations

from dataclasses import dataclass, field

# ============================================================
# Constants --- the fundamental parameters of VGA text mode
# ============================================================

# Each cell is 2 bytes: byte 0 = character, byte 1 = attribute.
BYTES_PER_CELL: int = 2

# Standard VGA text mode dimensions.
DEFAULT_COLUMNS: int = 80
DEFAULT_ROWS: int = 25

# Memory-mapped base address. We use 0xFFFB0000 (a high address) to avoid
# conflicts with program memory. On real x86 hardware, VGA text mode
# lives at 0xB8000.
DEFAULT_FRAMEBUFFER_BASE: int = 0xFFFB0000

# Light gray on black (0x07). This matches the classic terminal appearance.
#   Foreground: 7 (light gray)  = bits 3-0
#   Background: 0 (black)       = bits 6-4
#   Attribute:  0000_0111       = 0x07
DEFAULT_ATTRIBUTE: int = 0x07

# ============================================================
# Color constants --- the VGA color palette
# ============================================================
#
# Foreground colors use 4 bits (0-15), allowing 16 colors.
# Background colors use 3 bits (0-7), allowing 8 colors.
# The bright variants (8-15) are only available as foreground.

COLOR_BLACK: int = 0
COLOR_BLUE: int = 1
COLOR_GREEN: int = 2
COLOR_CYAN: int = 3
COLOR_RED: int = 4
COLOR_MAGENTA: int = 5
COLOR_BROWN: int = 6
COLOR_LIGHT_GRAY: int = 7
COLOR_DARK_GRAY: int = 8
COLOR_LIGHT_BLUE: int = 9
COLOR_LIGHT_GREEN: int = 10
COLOR_LIGHT_CYAN: int = 11
COLOR_LIGHT_RED: int = 12
COLOR_LIGHT_MAGENTA: int = 13
COLOR_YELLOW: int = 14
COLOR_WHITE: int = 15


def make_attribute(fg: int, bg: int) -> int:
    """Combine foreground and background colors into an attribute byte.

    The foreground occupies the low 4 bits, the background occupies bits 4-6.

    Examples:
        >>> make_attribute(COLOR_WHITE, COLOR_BLUE)  # white on blue
        31
        >>> hex(make_attribute(COLOR_WHITE, COLOR_BLUE))
        '0x1f'
        >>> make_attribute(COLOR_LIGHT_GRAY, COLOR_BLACK)  # default
        7
    """
    return ((bg & 0x07) << 4) | (fg & 0x0F)


# ============================================================
# Data structures
# ============================================================


@dataclass
class Cell:
    """A single character position in the framebuffer.

    Each cell stores the visible character (as an integer 0-255) and
    its color attribute byte.
    """

    character: int = ord(" ")
    attribute: int = DEFAULT_ATTRIBUTE


@dataclass
class CursorPosition:
    """Tracks the row and column of the cursor.

    Row 0 is the top of the screen, column 0 is the left edge.
    """

    row: int = 0
    col: int = 0


@dataclass
class DisplayConfig:
    """Parameters for the display dimensions and memory mapping.

    The default configuration matches VGA text mode: 80 columns, 25 rows,
    framebuffer at 0xFFFB0000, light gray on black.
    """

    columns: int = field(default=DEFAULT_COLUMNS)
    rows: int = field(default=DEFAULT_ROWS)
    framebuffer_base: int = field(default=DEFAULT_FRAMEBUFFER_BASE)
    default_attribute: int = field(default=DEFAULT_ATTRIBUTE)


# Predefined configurations for common use cases.

#: Standard VGA text mode (80x25).
VGA_80x25 = DisplayConfig(
    columns=80,
    rows=25,
    framebuffer_base=DEFAULT_FRAMEBUFFER_BASE,
    default_attribute=DEFAULT_ATTRIBUTE,
)

#: Compact mode for testing (40x10). Fewer cells = faster to fill and verify.
COMPACT_40x10 = DisplayConfig(
    columns=40,
    rows=10,
    framebuffer_base=DEFAULT_FRAMEBUFFER_BASE,
    default_attribute=DEFAULT_ATTRIBUTE,
)
