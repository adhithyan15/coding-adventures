"""Display --- VGA text-mode framebuffer simulation.

This package simulates a VGA text-mode framebuffer display, modeled after
the classic 80x25 text mode that dominated personal computing from the 1980s
through the early 2000s. Each cell is 2 bytes: one for the ASCII character
and one for the color attribute.

Modules:
    framebuffer - DisplayConfig, Cell, CursorPosition, color constants
    driver      - DisplayDriver with PutChar, Puts, Clear, Scroll
    snapshot    - DisplaySnapshot with Contains, LineAt, String

Quick start:
    >>> from display import DisplayDriver, DefaultDisplayConfig, BytesPerCell
    >>> config = DefaultDisplayConfig()
    >>> memory = bytearray(config.columns * config.rows * BytesPerCell)
    >>> driver = DisplayDriver(config, memory)
    >>> driver.puts("Hello World")
    >>> snap = driver.snapshot()
    >>> snap.lines[0]
    'Hello World'
    >>> snap.contains("Hello")
    True
"""

from display.driver import DisplayDriver
from display.framebuffer import (
    BYTES_PER_CELL,
    COLOR_BLACK,
    COLOR_BLUE,
    COLOR_BROWN,
    COLOR_CYAN,
    COLOR_DARK_GRAY,
    COLOR_GREEN,
    COLOR_LIGHT_BLUE,
    COLOR_LIGHT_CYAN,
    COLOR_LIGHT_GRAY,
    COLOR_LIGHT_GREEN,
    COLOR_LIGHT_MAGENTA,
    COLOR_LIGHT_RED,
    COLOR_MAGENTA,
    COLOR_RED,
    COLOR_WHITE,
    COLOR_YELLOW,
    DEFAULT_ATTRIBUTE,
    DEFAULT_COLUMNS,
    DEFAULT_FRAMEBUFFER_BASE,
    DEFAULT_ROWS,
    Cell,
    CursorPosition,
    DisplayConfig,
    make_attribute,
)
from display.snapshot import DisplaySnapshot

# Re-export with friendly aliases matching the Go API
BytesPerCell = BYTES_PER_CELL
DefaultAttribute = DEFAULT_ATTRIBUTE
DefaultColumns = DEFAULT_COLUMNS
DefaultRows = DEFAULT_ROWS
DefaultFramebufferBase = DEFAULT_FRAMEBUFFER_BASE


def DefaultDisplayConfig() -> DisplayConfig:  # noqa: ANN201, N802
    """Return the standard 80x25 VGA text mode configuration."""
    return DisplayConfig()


__all__ = [
    "BYTES_PER_CELL",
    "BytesPerCell",
    "COLOR_BLACK",
    "COLOR_BLUE",
    "COLOR_BROWN",
    "COLOR_CYAN",
    "COLOR_DARK_GRAY",
    "COLOR_GREEN",
    "COLOR_LIGHT_BLUE",
    "COLOR_LIGHT_CYAN",
    "COLOR_LIGHT_GRAY",
    "COLOR_LIGHT_GREEN",
    "COLOR_LIGHT_MAGENTA",
    "COLOR_LIGHT_RED",
    "COLOR_MAGENTA",
    "COLOR_RED",
    "COLOR_WHITE",
    "COLOR_YELLOW",
    "Cell",
    "CursorPosition",
    "DEFAULT_ATTRIBUTE",
    "DEFAULT_COLUMNS",
    "DEFAULT_FRAMEBUFFER_BASE",
    "DEFAULT_ROWS",
    "DefaultAttribute",
    "DefaultColumns",
    "DefaultDisplayConfig",
    "DefaultFramebufferBase",
    "DefaultRows",
    "DisplayConfig",
    "DisplayDriver",
    "DisplaySnapshot",
    "make_attribute",
]
