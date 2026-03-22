"""Tests for SimulatedDisplay."""

from device_driver_framework.device import DeviceType
from device_driver_framework.simulated_display import (
    DEFAULT_ATTRIBUTE,
    DISPLAY_COLS,
    DISPLAY_ROWS,
    FRAMEBUFFER_SIZE,
    SimulatedDisplay,
)


class TestSimulatedDisplay:
    """Verify SimulatedDisplay framebuffer, cursor, and write behavior."""

    def test_default_configuration(self) -> None:
        """Default display should have correct name, major, minor, no irq."""
        disp = SimulatedDisplay()
        assert disp.name == "display0"
        assert disp.device_type == DeviceType.CHARACTER
        assert disp.major == 1
        assert disp.minor == 0
        assert disp.interrupt_number == -1

    def test_framebuffer_size(self) -> None:
        """Framebuffer should be 80 * 25 * 2 = 4000 bytes."""
        disp = SimulatedDisplay()
        assert len(disp.framebuffer) == FRAMEBUFFER_SIZE
        assert FRAMEBUFFER_SIZE == 4000

    def test_init_clears_screen(self) -> None:
        """init() should fill screen with spaces and reset cursor."""
        disp = SimulatedDisplay()
        disp.write(b"Hello")
        disp.init()
        assert disp.initialized is True
        assert disp.cursor_position == (0, 0)
        # Every cell should be space (0x20) with default attribute
        assert disp.char_at(0, 0) == 0x20

    def test_write_single_char(self) -> None:
        """Writing one character should place it at (0,0) and advance cursor."""
        disp = SimulatedDisplay()
        disp.init()
        disp.write(b"H")
        assert disp.char_at(0, 0) == ord("H")
        assert disp.attr_at(0, 0) == DEFAULT_ATTRIBUTE
        assert disp.cursor_position == (0, 1)

    def test_write_multiple_chars(self) -> None:
        """Writing 'Hi' should place H at (0,0) and i at (0,1)."""
        disp = SimulatedDisplay()
        disp.init()
        count = disp.write(b"Hi")
        assert count == 2
        assert disp.char_at(0, 0) == ord("H")
        assert disp.char_at(0, 1) == ord("i")
        assert disp.cursor_position == (0, 2)

    def test_write_newline(self) -> None:
        """Newline (0x0A) should move cursor to start of next line."""
        disp = SimulatedDisplay()
        disp.init()
        disp.write(b"A\nB")
        assert disp.char_at(0, 0) == ord("A")
        assert disp.char_at(1, 0) == ord("B")
        assert disp.cursor_position == (1, 1)

    def test_write_wraps_at_end_of_line(self) -> None:
        """Writing past column 79 should wrap to the next line."""
        disp = SimulatedDisplay()
        disp.init()
        # Write 80 'X' characters to fill the first row
        disp.write(b"X" * DISPLAY_COLS)
        # Cursor should be at start of row 1
        assert disp.cursor_position == (1, 0)
        # Write one more character -- it should appear at row 1, col 0
        disp.write(b"Y")
        assert disp.char_at(1, 0) == ord("Y")

    def test_scroll_up(self) -> None:
        """Writing past the last row should scroll the display up."""
        disp = SimulatedDisplay()
        disp.init()
        # Fill all 25 rows with different characters
        for row in range(DISPLAY_ROWS):
            disp.write(bytes([ord("A") + row]) * DISPLAY_COLS)
        # After writing 25 full rows, the last write (row 24) wraps the cursor
        # to row 25, which triggers a scroll. So:
        # - Original row 0 ('A') has scrolled off the top
        # - Row 0 now contains 'B' (was row 1)
        assert disp.char_at(0, 0) == ord("B")
        # Row 23 should contain 'Y' (was row 24, the last written row)
        assert disp.char_at(DISPLAY_ROWS - 2, 0) == ord("A") + DISPLAY_ROWS - 1
        # Row 24 (last row) should be cleared (spaces) after scroll
        assert disp.char_at(DISPLAY_ROWS - 1, 0) == 0x20

    def test_read_returns_empty(self) -> None:
        """Reading from a display should return empty bytes (write-only)."""
        disp = SimulatedDisplay()
        assert disp.read(10) == b""

    def test_clear_screen(self) -> None:
        """clear_screen() should reset all cells and cursor."""
        disp = SimulatedDisplay()
        disp.write(b"Hello World")
        disp.clear_screen()
        assert disp.cursor_position == (0, 0)
        for col in range(11):
            assert disp.char_at(0, col) == 0x20

    def test_write_returns_byte_count(self) -> None:
        """write() should return the number of bytes written."""
        disp = SimulatedDisplay()
        disp.init()
        assert disp.write(b"ABC") == 3
        assert disp.write(b"") == 0

    def test_custom_name(self) -> None:
        """Custom name and minor should work."""
        disp = SimulatedDisplay(name="display1", minor=1)
        assert disp.name == "display1"
        assert disp.minor == 1
