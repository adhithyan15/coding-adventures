"""Tests for memory."""

import pytest

from cpu_simulator.memory import Memory


class TestMemoryByte:
    def test_initial_values_are_zero(self) -> None:
        mem = Memory(size=16)
        for i in range(16):
            assert mem.read_byte(i) == 0

    def test_write_and_read_byte(self) -> None:
        mem = Memory(size=16)
        mem.write_byte(0, 42)
        assert mem.read_byte(0) == 42

    def test_byte_masking(self) -> None:
        """Values > 255 should be masked to 8 bits."""
        mem = Memory(size=16)
        mem.write_byte(0, 256)
        assert mem.read_byte(0) == 0  # 256 & 0xFF = 0

    def test_out_of_bounds_read(self) -> None:
        mem = Memory(size=16)
        with pytest.raises(IndexError, match="out of bounds"):
            mem.read_byte(16)

    def test_out_of_bounds_write(self) -> None:
        mem = Memory(size=16)
        with pytest.raises(IndexError, match="out of bounds"):
            mem.write_byte(16, 0)


class TestMemoryWord:
    def test_write_and_read_word(self) -> None:
        mem = Memory(size=16)
        mem.write_word(0, 0x12345678)
        assert mem.read_word(0) == 0x12345678

    def test_little_endian_byte_order(self) -> None:
        """Verify little-endian: LSB at lowest address."""
        mem = Memory(size=16)
        mem.write_word(0, 0x12345678)
        assert mem.read_byte(0) == 0x78  # LSB
        assert mem.read_byte(1) == 0x56
        assert mem.read_byte(2) == 0x34
        assert mem.read_byte(3) == 0x12  # MSB

    def test_word_at_offset(self) -> None:
        mem = Memory(size=16)
        mem.write_word(4, 0xDEADBEEF)
        assert mem.read_word(4) == 0xDEADBEEF
        assert mem.read_word(0) == 0  # First word unaffected

    def test_small_value(self) -> None:
        """The value 3 stored as a 32-bit word."""
        mem = Memory(size=16)
        mem.write_word(0, 3)
        assert mem.read_byte(0) == 3
        assert mem.read_byte(1) == 0
        assert mem.read_byte(2) == 0
        assert mem.read_byte(3) == 0


class TestMemoryLoad:
    def test_load_bytes(self) -> None:
        mem = Memory(size=16)
        mem.load_bytes(0, b"\x01\x02\x03\x04")
        assert mem.read_byte(0) == 1
        assert mem.read_byte(1) == 2
        assert mem.read_byte(2) == 3
        assert mem.read_byte(3) == 4

    def test_load_at_offset(self) -> None:
        mem = Memory(size=16)
        mem.load_bytes(4, b"\xAA\xBB")
        assert mem.read_byte(4) == 0xAA
        assert mem.read_byte(5) == 0xBB

    def test_load_out_of_bounds(self) -> None:
        mem = Memory(size=4)
        with pytest.raises(IndexError, match="out of bounds"):
            mem.load_bytes(2, b"\x01\x02\x03")  # 3 bytes starting at 2 = needs 5


class TestMemoryDump:
    def test_dump(self) -> None:
        mem = Memory(size=16)
        mem.write_byte(0, 0xAB)
        mem.write_byte(1, 0xCD)
        assert mem.dump(0, 4) == [0xAB, 0xCD, 0, 0]


class TestMemoryValidation:
    def test_zero_size(self) -> None:
        with pytest.raises(ValueError, match="at least 1"):
            Memory(size=0)
