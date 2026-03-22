"""Tests for IOBlock — bidirectional I/O pad.

Coverage targets:
- Input mode (external → fabric)
- Output mode (fabric → external)
- Tri-state mode (high impedance)
- Mode switching
- Validation
"""

from __future__ import annotations

import pytest

from fpga.io_block import IOBlock, IOMode

# ─── Creation ─────────────────────────────────────────────────────────

class TestIOBlockCreation:
    def test_default_input_mode(self) -> None:
        io = IOBlock("pin_0")
        assert io.name == "pin_0"
        assert io.mode == IOMode.INPUT

    def test_custom_mode(self) -> None:
        io = IOBlock("led_0", mode=IOMode.OUTPUT)
        assert io.mode == IOMode.OUTPUT

    def test_rejects_empty_name(self) -> None:
        with pytest.raises(ValueError, match="non-empty string"):
            IOBlock("")

    def test_rejects_non_string_name(self) -> None:
        with pytest.raises(ValueError, match="non-empty string"):
            IOBlock(123)  # type: ignore[arg-type]


# ─── Input Mode ───────────────────────────────────────────────────────

class TestInputMode:
    def test_drive_pad_read_internal(self) -> None:
        io = IOBlock("sensor", mode=IOMode.INPUT)
        io.drive_pad(1)
        assert io.read_internal() == 1

    def test_read_pad_returns_pad_value(self) -> None:
        io = IOBlock("sensor", mode=IOMode.INPUT)
        io.drive_pad(1)
        assert io.read_pad() == 1

    def test_default_pad_value_is_zero(self) -> None:
        io = IOBlock("sensor", mode=IOMode.INPUT)
        assert io.read_internal() == 0
        assert io.read_pad() == 0


# ─── Output Mode ──────────────────────────────────────────────────────

class TestOutputMode:
    def test_drive_internal_read_pad(self) -> None:
        io = IOBlock("led", mode=IOMode.OUTPUT)
        io.drive_internal(1)
        assert io.read_pad() == 1

    def test_read_internal_returns_driven_value(self) -> None:
        io = IOBlock("led", mode=IOMode.OUTPUT)
        io.drive_internal(1)
        assert io.read_internal() == 1

    def test_output_zero(self) -> None:
        io = IOBlock("led", mode=IOMode.OUTPUT)
        io.drive_internal(0)
        assert io.read_pad() == 0


# ─── Tri-state Mode ──────────────────────────────────────────────────

class TestTristateMode:
    def test_read_pad_returns_none(self) -> None:
        """In tri-state mode, the pad is high impedance (None)."""
        io = IOBlock("bus", mode=IOMode.TRISTATE)
        io.drive_internal(1)
        assert io.read_pad() is None

    def test_read_internal_returns_driven_value(self) -> None:
        """Internal side still sees the driven value."""
        io = IOBlock("bus", mode=IOMode.TRISTATE)
        io.drive_internal(1)
        assert io.read_internal() == 1


# ─── Mode Switching ──────────────────────────────────────────────────

class TestModeSwitching:
    def test_input_to_output(self) -> None:
        io = IOBlock("pin", mode=IOMode.INPUT)
        io.drive_internal(1)
        io.configure(IOMode.OUTPUT)
        assert io.mode == IOMode.OUTPUT
        assert io.read_pad() == 1

    def test_output_to_tristate(self) -> None:
        io = IOBlock("pin", mode=IOMode.OUTPUT)
        io.drive_internal(1)
        assert io.read_pad() == 1
        io.configure(IOMode.TRISTATE)
        assert io.read_pad() is None

    def test_tristate_to_input(self) -> None:
        io = IOBlock("pin", mode=IOMode.TRISTATE)
        io.drive_pad(1)
        io.configure(IOMode.INPUT)
        assert io.read_internal() == 1


# ─── Validation ──────────────────────────────────────────────────────

class TestIOBlockValidation:
    def test_drive_pad_rejects_invalid(self) -> None:
        io = IOBlock("pin")
        with pytest.raises(ValueError, match="must be 0 or 1"):
            io.drive_pad(2)

    def test_drive_internal_rejects_invalid(self) -> None:
        io = IOBlock("pin")
        with pytest.raises(ValueError, match="must be 0 or 1"):
            io.drive_internal(-1)

    def test_configure_rejects_non_enum(self) -> None:
        io = IOBlock("pin")
        with pytest.raises(TypeError, match="must be an IOMode"):
            io.configure("input")  # type: ignore[arg-type]


# ─── IOMode Enum ──────────────────────────────────────────────────────

class TestIOMode:
    def test_values(self) -> None:
        assert IOMode.INPUT.value == "input"
        assert IOMode.OUTPUT.value == "output"
        assert IOMode.TRISTATE.value == "tristate"
