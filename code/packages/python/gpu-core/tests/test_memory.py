"""Tests for LocalMemory."""

from __future__ import annotations

import pytest
from fp_arithmetic import BF16, FP16, FP32, bits_to_float, float_to_bits

from gpu_core.memory import LocalMemory


class TestConstruction:
    """Test memory creation."""

    def test_default_size(self) -> None:
        """Default memory is 4096 bytes."""
        mem = LocalMemory()
        assert mem.size == 4096

    def test_custom_size(self) -> None:
        """Can create memory with custom size."""
        mem = LocalMemory(size=256)
        assert mem.size == 256

    def test_invalid_size(self) -> None:
        """Cannot create memory with size < 1."""
        with pytest.raises(ValueError, match="positive"):
            LocalMemory(size=0)

    def test_initialized_to_zero(self) -> None:
        """Memory starts as all zeros."""
        mem = LocalMemory(size=16)
        for i in range(16):
            assert mem.read_byte(i) == 0


class TestByteAccess:
    """Test raw byte read/write operations."""

    def test_read_write_byte(self) -> None:
        """Write a byte and read it back."""
        mem = LocalMemory()
        mem.write_byte(0, 0x42)
        assert mem.read_byte(0) == 0x42

    def test_byte_masking(self) -> None:
        """Values are masked to 8 bits."""
        mem = LocalMemory()
        mem.write_byte(0, 0x1FF)  # 9 bits
        assert mem.read_byte(0) == 0xFF  # truncated to 8

    def test_read_write_bytes(self) -> None:
        """Write and read multiple bytes."""
        mem = LocalMemory()
        data = b"\x01\x02\x03\x04"
        mem.write_bytes(0, data)
        assert mem.read_bytes(0, 4) == data

    def test_out_of_bounds_read(self) -> None:
        """Reading past memory bounds raises IndexError."""
        mem = LocalMemory(size=8)
        with pytest.raises(IndexError, match="out of bounds"):
            mem.read_byte(8)

    def test_out_of_bounds_write(self) -> None:
        """Writing past memory bounds raises IndexError."""
        mem = LocalMemory(size=8)
        with pytest.raises(IndexError, match="out of bounds"):
            mem.write_byte(8, 0)

    def test_negative_address(self) -> None:
        """Negative addresses are out of bounds."""
        mem = LocalMemory()
        with pytest.raises(IndexError):
            mem.read_byte(-1)

    def test_multi_byte_out_of_bounds(self) -> None:
        """Multi-byte access that extends past the end fails."""
        mem = LocalMemory(size=8)
        with pytest.raises(IndexError):
            mem.read_bytes(6, 4)  # bytes 6,7,8,9 — 8 and 9 out of bounds


class TestFloatAccess:
    """Test floating-point load/store operations."""

    def test_store_load_fp32(self) -> None:
        """Store and load an FP32 value."""
        mem = LocalMemory()
        value = float_to_bits(3.14, FP32)
        mem.store_float(0, value)
        result = mem.load_float(0, FP32)
        assert bits_to_float(result) == pytest.approx(3.14, rel=1e-5)

    def test_store_load_fp16(self) -> None:
        """Store and load an FP16 value (2 bytes)."""
        mem = LocalMemory()
        value = float_to_bits(1.0, FP16)
        mem.store_float(0, value)
        result = mem.load_float(0, FP16)
        assert bits_to_float(result) == 1.0

    def test_store_load_bf16(self) -> None:
        """Store and load a BF16 value (2 bytes)."""
        mem = LocalMemory()
        value = float_to_bits(2.0, BF16)
        mem.store_float(0, value)
        result = mem.load_float(0, BF16)
        assert bits_to_float(result) == 2.0

    def test_fp32_uses_4_bytes(self) -> None:
        """FP32 store writes exactly 4 bytes."""
        mem = LocalMemory()
        value = float_to_bits(1.0, FP32)
        mem.store_float(0, value)
        # Check that bytes 0-3 are non-trivial (1.0 = 0x3F800000)
        raw = mem.read_bytes(0, 4)
        assert len(raw) == 4
        assert raw != b"\x00\x00\x00\x00"

    def test_fp16_uses_2_bytes(self) -> None:
        """FP16 store writes exactly 2 bytes."""
        mem = LocalMemory()
        value = float_to_bits(1.0, FP16)
        mem.store_float(0, value)
        raw = mem.read_bytes(0, 2)
        assert len(raw) == 2
        assert raw != b"\x00\x00"

    def test_multiple_floats_at_different_addresses(self) -> None:
        """Store multiple floats at non-overlapping addresses."""
        mem = LocalMemory()
        mem.store_python_float(0, 1.0)
        mem.store_python_float(4, 2.0)
        mem.store_python_float(8, 3.0)
        assert mem.load_float_as_python(0) == 1.0
        assert mem.load_float_as_python(4) == 2.0
        assert mem.load_float_as_python(8) == 3.0

    def test_store_negative(self) -> None:
        """Store and load a negative float."""
        mem = LocalMemory()
        mem.store_python_float(0, -42.5)
        assert mem.load_float_as_python(0) == -42.5

    def test_store_zero(self) -> None:
        """Store and load zero."""
        mem = LocalMemory()
        mem.store_python_float(0, 0.0)
        assert mem.load_float_as_python(0) == 0.0

    def test_convenience_methods(self) -> None:
        """Test store_python_float and load_float_as_python."""
        mem = LocalMemory()
        mem.store_python_float(0, 2.71828, FP32)
        result = mem.load_float_as_python(0, FP32)
        assert abs(result - 2.71828) < 1e-5

    def test_float_out_of_bounds(self) -> None:
        """Loading a float past memory end raises IndexError."""
        mem = LocalMemory(size=8)
        with pytest.raises(IndexError):
            mem.load_float(6, FP32)  # needs 4 bytes at 6, goes to 10


class TestDump:
    """Test memory dump functionality."""

    def test_dump_zeros(self) -> None:
        """Dump of fresh memory is all zeros."""
        mem = LocalMemory(size=16)
        assert mem.dump(0, 16) == [0] * 16

    def test_dump_after_write(self) -> None:
        """Dump reflects written bytes."""
        mem = LocalMemory()
        mem.write_byte(0, 0xFF)
        mem.write_byte(1, 0x42)
        d = mem.dump(0, 4)
        assert d[0] == 0xFF
        assert d[1] == 0x42
        assert d[2] == 0
        assert d[3] == 0

    def test_repr(self) -> None:
        """Repr shows size and non-zero count."""
        mem = LocalMemory(size=64)
        assert "64 bytes" in repr(mem)
        assert "0 non-zero" in repr(mem)
        mem.write_byte(0, 1)
        assert "1 non-zero" in repr(mem)
