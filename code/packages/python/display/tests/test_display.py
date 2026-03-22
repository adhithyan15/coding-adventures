"""Comprehensive tests for the display package.

Tests cover all operations specified in S05-display.md: PutChar, PutCharAt,
Puts, line wrap, scroll, clear, snapshot, attributes, cursor management,
and edge cases.
"""

from __future__ import annotations

from display.driver import DisplayDriver
from display.framebuffer import (
    BYTES_PER_CELL,
    COLOR_BLACK,
    COLOR_BLUE,
    COLOR_GREEN,
    COLOR_LIGHT_GRAY,
    COLOR_RED,
    COLOR_WHITE,
    COMPACT_40x10,
    DEFAULT_ATTRIBUTE,
    Cell,
    CursorPosition,
    DisplayConfig,
    VGA_80x25,
    make_attribute,
)


# ============================================================
# Test helpers
# ============================================================


def new_test_driver() -> DisplayDriver:
    """Create a display driver with the compact 40x10 config."""
    config = COMPACT_40x10
    mem = bytearray(config.columns * config.rows * BYTES_PER_CELL)
    return DisplayDriver(config, mem)


def new_standard_driver() -> DisplayDriver:
    """Create a display driver with the full 80x25 VGA config."""
    config = DisplayConfig()
    mem = bytearray(config.columns * config.rows * BYTES_PER_CELL)
    return DisplayDriver(config, mem)


# ============================================================
# Config / constant tests
# ============================================================


class TestConfig:
    """Tests for DisplayConfig and constants."""

    def test_default_config(self) -> None:
        config = DisplayConfig()
        assert config.columns == 80
        assert config.rows == 25
        assert config.framebuffer_base == 0xFFFB0000
        assert config.default_attribute == 0x07

    def test_make_attribute_white_on_blue(self) -> None:
        assert make_attribute(COLOR_WHITE, COLOR_BLUE) == 0x1F

    def test_make_attribute_default(self) -> None:
        assert make_attribute(COLOR_LIGHT_GRAY, COLOR_BLACK) == 0x07

    def test_make_attribute_white_on_red(self) -> None:
        assert make_attribute(COLOR_WHITE, COLOR_RED) == 0x4F

    def test_make_attribute_green_on_black(self) -> None:
        assert make_attribute(COLOR_GREEN, COLOR_BLACK) == 0x02

    def test_predefined_vga_80x25(self) -> None:
        assert VGA_80x25.columns == 80
        assert VGA_80x25.rows == 25

    def test_predefined_compact_40x10(self) -> None:
        assert COMPACT_40x10.columns == 40
        assert COMPACT_40x10.rows == 10


# ============================================================
# Constructor tests
# ============================================================


class TestConstructor:
    """Tests for DisplayDriver initialization."""

    def test_clears_screen(self) -> None:
        d = new_test_driver()
        for row in range(d.config.rows):
            for col in range(d.config.columns):
                cell = d.get_cell(row, col)
                assert cell.character == ord(" ")
                assert cell.attribute == DEFAULT_ATTRIBUTE

    def test_cursor_at_origin(self) -> None:
        d = new_test_driver()
        pos = d.get_cursor()
        assert pos.row == 0
        assert pos.col == 0


# ============================================================
# PutChar tests
# ============================================================


class TestPutChar:
    """Tests for put_char method."""

    def test_basic_write(self) -> None:
        d = new_test_driver()
        d.put_char(ord("A"))
        cell = d.get_cell(0, 0)
        assert cell.character == ord("A")
        assert cell.attribute == DEFAULT_ATTRIBUTE

    def test_cursor_advance(self) -> None:
        d = new_test_driver()
        d.put_char(ord("A"))
        pos = d.get_cursor()
        assert pos.row == 0
        assert pos.col == 1

    def test_multiple_characters(self) -> None:
        d = new_test_driver()
        d.put_char(ord("H"))
        d.put_char(ord("i"))
        assert d.get_cell(0, 0).character == ord("H")
        assert d.get_cell(0, 1).character == ord("i")
        pos = d.get_cursor()
        assert pos.col == 2

    def test_newline(self) -> None:
        d = new_test_driver()
        d.put_char(ord("A"))
        d.put_char(ord("\n"))
        pos = d.get_cursor()
        assert pos.row == 1
        assert pos.col == 0

    def test_carriage_return(self) -> None:
        d = new_test_driver()
        for _ in range(5):
            d.put_char(ord("x"))
        d.put_char(ord("\r"))
        pos = d.get_cursor()
        assert pos.row == 0
        assert pos.col == 0

    def test_tab(self) -> None:
        d = new_test_driver()
        d.put_char(ord("\t"))
        pos = d.get_cursor()
        assert pos.col == 8

    def test_tab_from_col_1(self) -> None:
        d = new_test_driver()
        d.put_char(ord("x"))
        d.put_char(ord("\t"))
        pos = d.get_cursor()
        assert pos.col == 8

    def test_backspace(self) -> None:
        d = new_test_driver()
        d.put_char(ord("A"))
        d.put_char(ord("B"))
        d.put_char(0x08)  # backspace
        pos = d.get_cursor()
        assert pos.col == 1

    def test_backspace_at_col_zero(self) -> None:
        d = new_test_driver()
        d.put_char(0x08)  # backspace at col 0
        pos = d.get_cursor()
        assert pos.col == 0


# ============================================================
# PutCharAt tests
# ============================================================


class TestPutCharAt:
    """Tests for put_char_at method."""

    def test_basic_write(self) -> None:
        d = new_test_driver()
        d.put_char_at(5, 10, ord("X"), 0x0F)
        cell = d.get_cell(5, 10)
        assert cell.character == ord("X")
        assert cell.attribute == 0x0F

    def test_does_not_move_cursor(self) -> None:
        d = new_test_driver()
        d.set_cursor(0, 0)
        d.put_char_at(5, 10, ord("X"), 0x07)
        pos = d.get_cursor()
        assert pos.row == 0
        assert pos.col == 0

    def test_out_of_bounds(self) -> None:
        d = new_test_driver()
        # Should not crash.
        d.put_char_at(30, 0, ord("X"), 0x07)
        d.put_char_at(-1, 0, ord("X"), 0x07)
        d.put_char_at(0, -1, ord("X"), 0x07)
        d.put_char_at(0, 100, ord("X"), 0x07)


# ============================================================
# Puts tests
# ============================================================


class TestPuts:
    """Tests for puts method."""

    def test_simple_string(self) -> None:
        d = new_test_driver()
        d.puts("Hello")
        for i, ch in enumerate("Hello"):
            assert d.get_cell(0, i).character == ord(ch)
        assert d.get_cursor().col == 5

    def test_with_newline(self) -> None:
        d = new_test_driver()
        d.puts("Hi\nBye")
        snap = d.snapshot()
        assert snap.lines[0] == "Hi"
        assert snap.lines[1] == "Bye"

    def test_empty_string(self) -> None:
        d = new_test_driver()
        d.puts("")
        pos = d.get_cursor()
        assert pos.row == 0
        assert pos.col == 0


# ============================================================
# Line wrap tests
# ============================================================


class TestLineWrap:
    """Tests for automatic line wrapping."""

    def test_wrap_at_end_of_row(self) -> None:
        d = new_test_driver()
        # Write exactly 40 characters (compact config).
        for _ in range(d.config.columns):
            d.put_char(ord("A"))
        pos = d.get_cursor()
        assert pos.row == 1
        assert pos.col == 0

    def test_wrap_next_char(self) -> None:
        d = new_test_driver()
        for _ in range(d.config.columns):
            d.put_char(ord("A"))
        d.put_char(ord("B"))
        cell = d.get_cell(1, 0)
        assert cell.character == ord("B")

    def test_multi_line_wrap(self) -> None:
        d = new_test_driver()
        total = d.config.columns * 2 + 1
        for _ in range(total):
            d.put_char(ord("x"))
        pos = d.get_cursor()
        assert pos.row == 2
        assert pos.col == 1


# ============================================================
# Scroll tests
# ============================================================


class TestScroll:
    """Tests for scrolling behavior."""

    def test_scroll_trigger(self) -> None:
        d = new_test_driver()
        # Write unique char on each row.
        for row in range(d.config.rows):
            d.put_char_at(row, 0, ord("A") + row, DEFAULT_ATTRIBUTE)
        row1_char = d.get_cell(1, 0).character

        # Trigger scroll.
        d.set_cursor(d.config.rows - 1, 0)
        d.put_char(ord("\n"))

        # Row 0 should now have what was on row 1.
        assert d.get_cell(0, 0).character == row1_char

    def test_last_row_cleared(self) -> None:
        d = new_test_driver()
        # Fill every cell.
        for row in range(d.config.rows):
            for col in range(d.config.columns):
                d.put_char_at(row, col, ord("X"), DEFAULT_ATTRIBUTE)
        # Trigger scroll.
        d.set_cursor(d.config.rows - 1, 0)
        d.put_char(ord("\n"))

        # Last row should be spaces.
        for col in range(d.config.columns):
            assert d.get_cell(d.config.rows - 1, col).character == ord(" ")

    def test_cursor_after_scroll(self) -> None:
        d = new_test_driver()
        d.set_cursor(d.config.rows - 1, 0)
        d.put_char(ord("\n"))
        pos = d.get_cursor()
        assert pos.row == d.config.rows - 1
        assert pos.col == 0

    def test_multiple_scrolls(self) -> None:
        d = new_test_driver()
        for i in range(30):
            d.puts("Line")
            d.put_char(ord("\n"))
        snap = d.snapshot()
        assert snap.contains("Line")

    def test_scroll_preserves_attributes(self) -> None:
        d = new_test_driver()
        custom_attr = make_attribute(COLOR_WHITE, COLOR_BLUE)
        d.put_char_at(1, 0, ord("Z"), custom_attr)

        d.set_cursor(d.config.rows - 1, 0)
        d.put_char(ord("\n"))

        cell = d.get_cell(0, 0)
        assert cell.character == ord("Z")
        assert cell.attribute == custom_attr


# ============================================================
# Clear tests
# ============================================================


class TestClear:
    """Tests for clear method."""

    def test_clear_display(self) -> None:
        d = new_test_driver()
        d.puts("Hello World")
        d.clear()

        for row in range(d.config.rows):
            for col in range(d.config.columns):
                cell = d.get_cell(row, col)
                assert cell.character == ord(" ")
                assert cell.attribute == DEFAULT_ATTRIBUTE

    def test_clear_resets_cursor(self) -> None:
        d = new_test_driver()
        d.puts("Hello")
        d.clear()
        pos = d.get_cursor()
        assert pos.row == 0
        assert pos.col == 0


# ============================================================
# Snapshot tests
# ============================================================


class TestSnapshot:
    """Tests for DisplaySnapshot."""

    def test_basic(self) -> None:
        d = new_test_driver()
        d.puts("Hello World")
        snap = d.snapshot()
        assert snap.lines[0] == "Hello World"

    def test_trailing_spaces_trimmed(self) -> None:
        d = new_test_driver()
        d.puts("Hi")
        snap = d.snapshot()
        assert snap.lines[0] == "Hi"

    def test_empty_lines(self) -> None:
        d = new_test_driver()
        snap = d.snapshot()
        for line in snap.lines:
            assert line == ""

    def test_contains_positive(self) -> None:
        d = new_test_driver()
        d.puts("Hello World")
        snap = d.snapshot()
        assert snap.contains("Hello World") is True

    def test_contains_negative(self) -> None:
        d = new_test_driver()
        d.puts("Hello World")
        snap = d.snapshot()
        assert snap.contains("Goodbye") is False

    def test_contains_partial(self) -> None:
        d = new_test_driver()
        d.puts("Hello World")
        snap = d.snapshot()
        assert snap.contains("World") is True

    def test_string_output(self) -> None:
        d = new_test_driver()
        d.puts("Hello")
        snap = d.snapshot()
        s = snap.string()
        lines = s.split("\n")
        assert len(lines) == d.config.rows
        for line in lines:
            assert len(line) == d.config.columns

    def test_cursor_in_snapshot(self) -> None:
        d = new_test_driver()
        d.set_cursor(5, 10)
        snap = d.snapshot()
        assert snap.cursor.row == 5
        assert snap.cursor.col == 10

    def test_line_at(self) -> None:
        d = new_test_driver()
        d.puts("Line 0")
        d.put_char(ord("\n"))
        d.puts("Line 1")
        snap = d.snapshot()
        assert snap.line_at(0) == "Line 0"
        assert snap.line_at(1) == "Line 1"
        assert snap.line_at(-1) == ""
        assert snap.line_at(100) == ""

    def test_rows_and_columns(self) -> None:
        d = new_test_driver()
        snap = d.snapshot()
        assert snap.rows == d.config.rows
        assert snap.columns == d.config.columns


# ============================================================
# Attribute tests
# ============================================================


class TestAttributes:
    """Tests for attribute byte handling."""

    def test_default_attribute(self) -> None:
        d = new_test_driver()
        d.put_char(ord("A"))
        cell = d.get_cell(0, 0)
        assert cell.attribute == 0x07

    def test_custom_attribute(self) -> None:
        d = new_test_driver()
        d.put_char_at(0, 0, ord("A"), 0x1F)
        cell = d.get_cell(0, 0)
        assert cell.attribute == 0x1F


# ============================================================
# Cursor management tests
# ============================================================


class TestCursorManagement:
    """Tests for cursor set/get and clamping."""

    def test_set_cursor_clamps_negative(self) -> None:
        d = new_test_driver()
        d.set_cursor(-5, -5)
        pos = d.get_cursor()
        assert pos.row == 0
        assert pos.col == 0

    def test_set_cursor_clamps_large(self) -> None:
        d = new_test_driver()
        d.set_cursor(100, 100)
        pos = d.get_cursor()
        assert pos.row == d.config.rows - 1
        assert pos.col == d.config.columns - 1


# ============================================================
# Edge case tests
# ============================================================


class TestEdgeCases:
    """Edge case and stress tests."""

    def test_full_framebuffer(self) -> None:
        d = new_test_driver()
        total = d.config.columns * d.config.rows
        for _ in range(total):
            d.put_char(ord("X"))
        snap = d.snapshot()
        assert snap.contains("X")

    def test_rapid_scrolling(self) -> None:
        d = new_test_driver()
        for _ in range(100):
            d.puts("Line")
            d.put_char(ord("\n"))
        snap = d.snapshot()
        assert snap.contains("Line")

    def test_null_character(self) -> None:
        d = new_test_driver()
        d.put_char(0x00)
        cell = d.get_cell(0, 0)
        assert cell.character == 0x00

    def test_all_ascii_values(self) -> None:
        d = new_standard_driver()
        for i in range(256):
            row = i // d.config.columns
            col = i % d.config.columns
            d.put_char_at(row, col, i, DEFAULT_ATTRIBUTE)
        for i in range(256):
            row = i // d.config.columns
            col = i % d.config.columns
            cell = d.get_cell(row, col)
            assert cell.character == i

    def test_get_cell_out_of_bounds(self) -> None:
        d = new_test_driver()
        cell = d.get_cell(-1, 0)
        assert cell.character == ord(" ")
        assert cell.attribute == DEFAULT_ATTRIBUTE

    def test_tab_wrap_to_next_row(self) -> None:
        d = new_test_driver()
        d.set_cursor(0, 39)  # Last column in 40-col display
        d.put_char(ord("\t"))
        pos = d.get_cursor()
        assert pos.row == 1
        assert pos.col == 0


# ============================================================
# Standard 80x25 tests
# ============================================================


class TestStandard80x25:
    """Tests using the full 80x25 configuration."""

    def test_put_char(self) -> None:
        d = new_standard_driver()
        d.put_char(ord("A"))
        cell = d.get_cell(0, 0)
        assert cell.character == ord("A")
        assert cell.attribute == 0x07

    def test_line_wrap(self) -> None:
        d = new_standard_driver()
        for _ in range(81):
            d.put_char(ord("A"))
        pos = d.get_cursor()
        assert pos.row == 1
        assert pos.col == 1

    def test_scroll(self) -> None:
        d = new_standard_driver()
        for row in range(25):
            d.put_char_at(row, 0, ord("A") + row % 26, DEFAULT_ATTRIBUTE)
        d.set_cursor(24, 0)
        d.put_char(ord("\n"))
        cell = d.get_cell(0, 0)
        assert cell.character == ord("B")
