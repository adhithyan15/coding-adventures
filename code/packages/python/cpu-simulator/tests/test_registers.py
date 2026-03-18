"""Tests for the register file."""

import pytest

from cpu_simulator.registers import RegisterFile


class TestRegisterFile:
    def test_initial_values_are_zero(self) -> None:
        regs = RegisterFile(num_registers=4)
        for i in range(4):
            assert regs.read(i) == 0

    def test_write_and_read(self) -> None:
        regs = RegisterFile(num_registers=4)
        regs.write(1, 42)
        assert regs.read(1) == 42

    def test_write_does_not_affect_other_registers(self) -> None:
        regs = RegisterFile(num_registers=4)
        regs.write(2, 99)
        assert regs.read(0) == 0
        assert regs.read(1) == 0
        assert regs.read(2) == 99
        assert regs.read(3) == 0

    def test_bit_width_masking(self) -> None:
        """Values exceeding bit width should wrap around."""
        regs = RegisterFile(num_registers=4, bit_width=8)
        regs.write(0, 256)  # 256 = 0x100, doesn't fit in 8 bits
        assert regs.read(0) == 0  # wraps to 0

    def test_bit_width_masking_32bit(self) -> None:
        regs = RegisterFile(num_registers=4, bit_width=32)
        regs.write(0, 0xFFFFFFFF)
        assert regs.read(0) == 0xFFFFFFFF
        regs.write(0, 0x1FFFFFFFF)  # 33 bits
        assert regs.read(0) == 0xFFFFFFFF  # masked to 32 bits

    def test_read_out_of_range(self) -> None:
        regs = RegisterFile(num_registers=4)
        with pytest.raises(IndexError, match="out of range"):
            regs.read(4)

    def test_write_out_of_range(self) -> None:
        regs = RegisterFile(num_registers=4)
        with pytest.raises(IndexError, match="out of range"):
            regs.write(4, 0)

    def test_negative_index(self) -> None:
        regs = RegisterFile(num_registers=4)
        with pytest.raises(IndexError):
            regs.read(-1)

    def test_dump(self) -> None:
        regs = RegisterFile(num_registers=4)
        regs.write(1, 5)
        regs.write(3, 10)
        assert regs.dump() == {"R0": 0, "R1": 5, "R2": 0, "R3": 10}
