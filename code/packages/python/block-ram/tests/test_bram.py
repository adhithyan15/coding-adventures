"""Tests for ConfigurableBRAM — FPGA-style reconfigurable memory.

Coverage targets:
- Initialization with various aspect ratios
- Reconfiguration (changes depth/width, clears data)
- Port A and Port B operations
- Validation (bad total_bits, bad width, non-dividing width)
"""

from __future__ import annotations

import pytest

from block_ram.bram import ConfigurableBRAM

# ─── Helper ───────────────────────────────────────────────────────────

def full_write_a(
    bram: ConfigurableBRAM,
    address: int,
    data: list[int],
) -> list[int]:
    """Full write cycle on port A (clock 0 then 1)."""
    bram.tick_a(0, address, data, write_enable=1)
    return bram.tick_a(1, address, data, write_enable=1)


def full_read_a(
    bram: ConfigurableBRAM,
    address: int,
) -> list[int]:
    """Full read cycle on port A."""
    zeros = [0] * bram.width
    bram.tick_a(0, address, zeros, write_enable=0)
    return bram.tick_a(1, address, zeros, write_enable=0)


def full_write_b(
    bram: ConfigurableBRAM,
    address: int,
    data: list[int],
) -> list[int]:
    """Full write cycle on port B."""
    bram.tick_b(0, address, data, write_enable=1)
    return bram.tick_b(1, address, data, write_enable=1)


def full_read_b(
    bram: ConfigurableBRAM,
    address: int,
) -> list[int]:
    """Full read cycle on port B."""
    zeros = [0] * bram.width
    bram.tick_b(0, address, zeros, write_enable=0)
    return bram.tick_b(1, address, zeros, write_enable=0)


# ─── Initialization ──────────────────────────────────────────────────

class TestConfigurableBRAMInit:
    """Test BRAM creation with various aspect ratios."""

    def test_default_18kbit(self) -> None:
        bram = ConfigurableBRAM()
        assert bram.total_bits == 18432
        assert bram.width == 8
        assert bram.depth == 2304

    def test_custom_size(self) -> None:
        bram = ConfigurableBRAM(total_bits=1024, width=8)
        assert bram.total_bits == 1024
        assert bram.width == 8
        assert bram.depth == 128

    def test_width_1(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=1)
        assert bram.depth == 256

    def test_width_equals_total(self) -> None:
        bram = ConfigurableBRAM(total_bits=32, width=32)
        assert bram.depth == 1

    def test_properties(self) -> None:
        bram = ConfigurableBRAM(total_bits=512, width=16)
        assert bram.total_bits == 512
        assert bram.width == 16
        assert bram.depth == 32

    # ── Validation ────────────────────────────────────────────────

    def test_rejects_zero_total_bits(self) -> None:
        with pytest.raises(ValueError, match="total_bits must be >= 1"):
            ConfigurableBRAM(total_bits=0)

    def test_rejects_negative_total_bits(self) -> None:
        with pytest.raises(ValueError, match="total_bits must be >= 1"):
            ConfigurableBRAM(total_bits=-1)

    def test_rejects_zero_width(self) -> None:
        with pytest.raises(ValueError, match="width must be >= 1"):
            ConfigurableBRAM(total_bits=1024, width=0)

    def test_rejects_non_dividing_width(self) -> None:
        with pytest.raises(ValueError, match="does not evenly divide"):
            ConfigurableBRAM(total_bits=1024, width=3)


# ─── Reconfiguration ─────────────────────────────────────────────────

class TestReconfigure:
    """Test aspect ratio reconfiguration."""

    def test_reconfigure_changes_dimensions(self) -> None:
        bram = ConfigurableBRAM(total_bits=1024, width=8)
        assert bram.depth == 128
        bram.reconfigure(width=16)
        assert bram.width == 16
        assert bram.depth == 64

    def test_reconfigure_clears_data(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        full_write_a(bram, 0, [1, 0, 1, 0, 0, 1, 0, 1])
        bram.reconfigure(width=8)  # Same width but data should clear
        result = full_read_a(bram, 0)
        assert result == [0] * 8

    def test_reconfigure_to_narrower(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        bram.reconfigure(width=4)
        assert bram.depth == 64
        assert bram.width == 4

    def test_reconfigure_to_wider(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        bram.reconfigure(width=32)
        assert bram.depth == 8
        assert bram.width == 32

    def test_reconfigure_to_width_1(self) -> None:
        bram = ConfigurableBRAM(total_bits=64, width=8)
        bram.reconfigure(width=1)
        assert bram.depth == 64

    def test_reconfigure_rejects_zero_width(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        with pytest.raises(ValueError, match="width must be >= 1"):
            bram.reconfigure(width=0)

    def test_reconfigure_rejects_non_dividing_width(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        with pytest.raises(ValueError, match="does not evenly divide"):
            bram.reconfigure(width=3)


# ─── Port A Operations ───────────────────────────────────────────────

class TestPortA:
    """Test port A read/write operations."""

    def test_write_and_read(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        data = [1, 0, 1, 0, 0, 1, 0, 1]
        full_write_a(bram, 0, data)
        result = full_read_a(bram, 0)
        assert result == data

    def test_multiple_addresses(self) -> None:
        bram = ConfigurableBRAM(total_bits=128, width=4)
        full_write_a(bram, 0, [1, 0, 0, 0])
        full_write_a(bram, 1, [0, 1, 0, 0])
        assert full_read_a(bram, 0) == [1, 0, 0, 0]
        assert full_read_a(bram, 1) == [0, 1, 0, 0]

    def test_validates_clock(self) -> None:
        bram = ConfigurableBRAM(total_bits=64, width=4)
        with pytest.raises(ValueError, match="must be 0 or 1"):
            bram.tick_a(2, 0, [0, 0, 0, 0], 0)


# ─── Port B Operations ───────────────────────────────────────────────

class TestPortB:
    """Test port B read/write operations."""

    def test_write_and_read(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        data = [0, 1, 1, 0, 1, 0, 0, 1]
        full_write_b(bram, 0, data)
        result = full_read_b(bram, 0)
        assert result == data

    def test_multiple_addresses(self) -> None:
        bram = ConfigurableBRAM(total_bits=128, width=4)
        full_write_b(bram, 0, [1, 1, 0, 0])
        full_write_b(bram, 1, [0, 0, 1, 1])
        assert full_read_b(bram, 0) == [1, 1, 0, 0]
        assert full_read_b(bram, 1) == [0, 0, 1, 1]

    def test_validates_clock(self) -> None:
        bram = ConfigurableBRAM(total_bits=64, width=4)
        with pytest.raises(ValueError, match="must be 0 or 1"):
            bram.tick_b(2, 0, [0, 0, 0, 0], 0)


# ─── Cross-Port Operations ───────────────────────────────────────────

class TestCrossPort:
    """Test that data written on one port is visible from the other."""

    def test_write_a_read_b(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        data = [1, 1, 0, 0, 1, 1, 0, 0]
        full_write_a(bram, 5, data)
        result = full_read_b(bram, 5)
        assert result == data

    def test_write_b_read_a(self) -> None:
        bram = ConfigurableBRAM(total_bits=256, width=8)
        data = [0, 0, 1, 1, 0, 0, 1, 1]
        full_write_b(bram, 10, data)
        result = full_read_a(bram, 10)
        assert result == data

    def test_alternating_port_writes(self) -> None:
        """Write alternating addresses via different ports."""
        bram = ConfigurableBRAM(total_bits=128, width=4)
        full_write_a(bram, 0, [1, 0, 0, 0])
        full_write_b(bram, 1, [0, 1, 0, 0])
        full_write_a(bram, 2, [0, 0, 1, 0])
        full_write_b(bram, 3, [0, 0, 0, 1])
        assert full_read_b(bram, 0) == [1, 0, 0, 0]
        assert full_read_a(bram, 1) == [0, 1, 0, 0]
        assert full_read_b(bram, 2) == [0, 0, 1, 0]
        assert full_read_a(bram, 3) == [0, 0, 0, 1]
