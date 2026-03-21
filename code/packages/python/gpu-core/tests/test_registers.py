"""Tests for the FPRegisterFile."""

from __future__ import annotations

import pytest
from fp_arithmetic import BF16, FP16, FP32, float_to_bits

from gpu_core.registers import FPRegisterFile


class TestConstruction:
    """Test register file creation and configuration."""

    def test_default_construction(self) -> None:
        """Default: 32 FP32 registers, all zero."""
        rf = FPRegisterFile()
        assert rf.num_registers == 32
        assert rf.fmt == FP32
        assert rf.read_float(0) == 0.0
        assert rf.read_float(31) == 0.0

    def test_custom_register_count(self) -> None:
        """Can create register files with different sizes."""
        rf = FPRegisterFile(num_registers=64)
        assert rf.num_registers == 64
        rf.write_float(63, 1.0)
        assert rf.read_float(63) == 1.0

    def test_nvidia_scale(self) -> None:
        """NVIDIA cores support up to 255 registers."""
        rf = FPRegisterFile(num_registers=255)
        rf.write_float(254, 42.0)
        assert rf.read_float(254) == 42.0

    def test_max_256_registers(self) -> None:
        """Maximum is 256 registers."""
        rf = FPRegisterFile(num_registers=256)
        assert rf.num_registers == 256

    def test_invalid_zero_registers(self) -> None:
        """Cannot create a register file with 0 registers."""
        with pytest.raises(ValueError, match="num_registers must be 1-256"):
            FPRegisterFile(num_registers=0)

    def test_invalid_too_many_registers(self) -> None:
        """Cannot create a register file with >256 registers."""
        with pytest.raises(ValueError, match="num_registers must be 1-256"):
            FPRegisterFile(num_registers=257)

    def test_fp16_format(self) -> None:
        """Register file can use FP16 format."""
        rf = FPRegisterFile(fmt=FP16)
        assert rf.fmt == FP16
        rf.write_float(0, 1.0)
        assert rf.read_float(0) == 1.0

    def test_bf16_format(self) -> None:
        """Register file can use BF16 format."""
        rf = FPRegisterFile(fmt=BF16)
        assert rf.fmt == BF16
        rf.write_float(0, 1.0)
        assert rf.read_float(0) == 1.0


class TestReadWrite:
    """Test reading and writing register values."""

    def test_write_and_read_floatbits(self) -> None:
        """Write a FloatBits and read it back."""
        rf = FPRegisterFile()
        value = float_to_bits(3.14, FP32)
        rf.write(0, value)
        result = rf.read(0)
        assert result == value

    def test_write_and_read_float(self) -> None:
        """Write a Python float and read it back."""
        rf = FPRegisterFile()
        rf.write_float(5, 2.71828)
        result = rf.read_float(5)
        assert abs(result - 2.71828) < 1e-5

    def test_write_negative(self) -> None:
        """Write a negative value."""
        rf = FPRegisterFile()
        rf.write_float(0, -42.0)
        assert rf.read_float(0) == -42.0

    def test_write_zero(self) -> None:
        """Write zero explicitly."""
        rf = FPRegisterFile()
        rf.write_float(0, 99.0)
        rf.write_float(0, 0.0)
        assert rf.read_float(0) == 0.0

    def test_overwrite(self) -> None:
        """Writing to a register overwrites the previous value."""
        rf = FPRegisterFile()
        rf.write_float(0, 1.0)
        rf.write_float(0, 2.0)
        assert rf.read_float(0) == 2.0

    def test_independent_registers(self) -> None:
        """Writing to one register doesn't affect others."""
        rf = FPRegisterFile()
        rf.write_float(0, 1.0)
        rf.write_float(1, 2.0)
        assert rf.read_float(0) == 1.0
        assert rf.read_float(1) == 2.0

    def test_read_out_of_bounds(self) -> None:
        """Reading past the register count raises IndexError."""
        rf = FPRegisterFile(num_registers=8)
        with pytest.raises(IndexError, match="Register index 8"):
            rf.read(8)

    def test_write_out_of_bounds(self) -> None:
        """Writing past the register count raises IndexError."""
        rf = FPRegisterFile(num_registers=8)
        with pytest.raises(IndexError, match="Register index 8"):
            rf.write(8, float_to_bits(1.0, FP32))

    def test_negative_index(self) -> None:
        """Negative register indices are invalid."""
        rf = FPRegisterFile()
        with pytest.raises(IndexError):
            rf.read(-1)


class TestDump:
    """Test register file dump functionality."""

    def test_dump_empty(self) -> None:
        """Dump of all-zero registers returns empty dict."""
        rf = FPRegisterFile()
        assert rf.dump() == {}

    def test_dump_non_zero(self) -> None:
        """Dump includes only non-zero registers."""
        rf = FPRegisterFile()
        rf.write_float(0, 1.0)
        rf.write_float(5, 3.14)
        result = rf.dump()
        assert "R0" in result
        assert "R5" in result
        assert len(result) == 2

    def test_dump_all(self) -> None:
        """dump_all includes all registers including zeros."""
        rf = FPRegisterFile(num_registers=4)
        rf.write_float(0, 1.0)
        result = rf.dump_all()
        assert len(result) == 4
        assert result["R0"] == 1.0
        assert result["R1"] == 0.0

    def test_repr_empty(self) -> None:
        """Repr shows 'all zero' for fresh register file."""
        rf = FPRegisterFile()
        assert "all zero" in repr(rf)

    def test_repr_with_values(self) -> None:
        """Repr shows non-zero register values."""
        rf = FPRegisterFile()
        rf.write_float(0, 3.0)
        assert "R0=3.0" in repr(rf)
