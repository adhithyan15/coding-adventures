"""test_linear_memory.py --- Tests for WASM linear memory.

Covers: all load/store variants (full-width and narrow), grow, size,
byte_length, write_bytes, and out-of-bounds trap behavior.
"""

from __future__ import annotations

import struct

import pytest

from wasm_execution.host_interface import TrapError
from wasm_execution.linear_memory import MAX_PAGES, PAGE_SIZE, LinearMemory


# ===========================================================================
# Construction and size queries
# ===========================================================================


class TestLinearMemoryBasics:
    def test_initial_pages(self) -> None:
        mem = LinearMemory(1)
        assert mem.size() == 1
        assert mem.byte_length() == PAGE_SIZE

    def test_zero_pages(self) -> None:
        mem = LinearMemory(0)
        assert mem.size() == 0
        assert mem.byte_length() == 0

    def test_multiple_pages(self) -> None:
        mem = LinearMemory(3, max_pages=10)
        assert mem.size() == 3
        assert mem.byte_length() == 3 * PAGE_SIZE


# ===========================================================================
# Full-width stores and loads (i32, i64, f32, f64)
# ===========================================================================


class TestFullWidthLoadsStores:
    def test_store_load_i32(self) -> None:
        mem = LinearMemory(1)
        mem.store_i32(0, 42)
        assert mem.load_i32(0) == 42

    def test_store_load_i32_negative(self) -> None:
        mem = LinearMemory(1)
        mem.store_i32(0, -1)
        assert mem.load_i32(0) == -1

    def test_store_load_i64(self) -> None:
        mem = LinearMemory(1)
        mem.store_i64(0, 123456789012345)
        assert mem.load_i64(0) == 123456789012345

    def test_store_load_i64_negative(self) -> None:
        mem = LinearMemory(1)
        mem.store_i64(0, -1)
        assert mem.load_i64(0) == -1

    def test_store_load_f32(self) -> None:
        mem = LinearMemory(1)
        mem.store_f32(0, 3.14)
        loaded = mem.load_f32(0)
        assert loaded == pytest.approx(3.14, abs=1e-5)

    def test_store_load_f64(self) -> None:
        mem = LinearMemory(1)
        mem.store_f64(0, 3.141592653589793)
        assert mem.load_f64(0) == pytest.approx(3.141592653589793)


# ===========================================================================
# Narrow loads for i32 (8-bit and 16-bit)
# ===========================================================================


class TestNarrowI32Loads:
    def test_load_i32_8s_positive(self) -> None:
        mem = LinearMemory(1)
        mem._data[0] = 0x7F  # 127
        assert mem.load_i32_8s(0) == 127

    def test_load_i32_8s_negative(self) -> None:
        mem = LinearMemory(1)
        mem._data[0] = 0x80  # -128 signed
        assert mem.load_i32_8s(0) == -128

    def test_load_i32_8u(self) -> None:
        mem = LinearMemory(1)
        mem._data[0] = 0xFF
        assert mem.load_i32_8u(0) == 255

    def test_load_i32_16s_positive(self) -> None:
        mem = LinearMemory(1)
        struct.pack_into("<h", mem._data, 0, 1000)
        assert mem.load_i32_16s(0) == 1000

    def test_load_i32_16s_negative(self) -> None:
        mem = LinearMemory(1)
        struct.pack_into("<h", mem._data, 0, -1000)
        assert mem.load_i32_16s(0) == -1000

    def test_load_i32_16u(self) -> None:
        mem = LinearMemory(1)
        struct.pack_into("<H", mem._data, 0, 60000)
        assert mem.load_i32_16u(0) == 60000


# ===========================================================================
# Narrow loads for i64 (8-bit, 16-bit, 32-bit)
# ===========================================================================


class TestNarrowI64Loads:
    def test_load_i64_8s(self) -> None:
        mem = LinearMemory(1)
        mem._data[0] = 0xFF  # -1 signed
        assert mem.load_i64_8s(0) == -1

    def test_load_i64_8u(self) -> None:
        mem = LinearMemory(1)
        mem._data[0] = 0xFF
        assert mem.load_i64_8u(0) == 255

    def test_load_i64_16s(self) -> None:
        mem = LinearMemory(1)
        struct.pack_into("<h", mem._data, 0, -500)
        assert mem.load_i64_16s(0) == -500

    def test_load_i64_16u(self) -> None:
        mem = LinearMemory(1)
        struct.pack_into("<H", mem._data, 0, 50000)
        assert mem.load_i64_16u(0) == 50000

    def test_load_i64_32s(self) -> None:
        mem = LinearMemory(1)
        struct.pack_into("<i", mem._data, 0, -100000)
        assert mem.load_i64_32s(0) == -100000

    def test_load_i64_32u(self) -> None:
        mem = LinearMemory(1)
        struct.pack_into("<I", mem._data, 0, 3000000000)
        assert mem.load_i64_32u(0) == 3000000000


# ===========================================================================
# Narrow stores
# ===========================================================================


class TestNarrowStores:
    def test_store_i32_8(self) -> None:
        mem = LinearMemory(1)
        mem.store_i32_8(0, 0xFF)
        assert mem.load_i32_8s(0) == -1
        assert mem.load_i32_8u(0) == 255

    def test_store_i32_16(self) -> None:
        mem = LinearMemory(1)
        mem.store_i32_16(0, 0xFFFF)
        assert mem.load_i32_16s(0) == -1
        assert mem.load_i32_16u(0) == 65535

    def test_store_i64_8(self) -> None:
        mem = LinearMemory(1)
        mem.store_i64_8(0, 0xAB)
        assert mem.load_i64_8u(0) == 0xAB

    def test_store_i64_16(self) -> None:
        mem = LinearMemory(1)
        mem.store_i64_16(0, 0x1234)
        assert mem.load_i64_16u(0) == 0x1234

    def test_store_i64_32(self) -> None:
        mem = LinearMemory(1)
        mem.store_i64_32(0, 0xDEADBEEF)
        assert mem.load_i64_32u(0) == 0xDEADBEEF


# ===========================================================================
# Out-of-bounds access
# ===========================================================================


class TestOOBAccess:
    def test_load_i32_oob(self) -> None:
        mem = LinearMemory(1)
        with pytest.raises(TrapError, match="Out of bounds"):
            mem.load_i32(PAGE_SIZE)

    def test_store_i32_oob(self) -> None:
        mem = LinearMemory(1)
        with pytest.raises(TrapError, match="Out of bounds"):
            mem.store_i32(PAGE_SIZE, 0)

    def test_load_i64_oob(self) -> None:
        mem = LinearMemory(1)
        with pytest.raises(TrapError, match="Out of bounds"):
            mem.load_i64(PAGE_SIZE - 4)

    def test_store_f64_oob(self) -> None:
        mem = LinearMemory(1)
        with pytest.raises(TrapError, match="Out of bounds"):
            mem.store_f64(PAGE_SIZE, 0.0)

    def test_negative_offset(self) -> None:
        mem = LinearMemory(1)
        with pytest.raises(TrapError, match="Out of bounds"):
            mem.load_i32(-1)

    def test_zero_page_oob(self) -> None:
        mem = LinearMemory(0)
        with pytest.raises(TrapError, match="Out of bounds"):
            mem.load_i32(0)


# ===========================================================================
# Memory growth
# ===========================================================================


class TestMemoryGrow:
    def test_grow_returns_old_pages(self) -> None:
        mem = LinearMemory(1, max_pages=4)
        old = mem.grow(2)
        assert old == 1
        assert mem.size() == 3

    def test_grow_beyond_max_returns_minus_one(self) -> None:
        mem = LinearMemory(1, max_pages=2)
        result = mem.grow(3)
        assert result == -1
        assert mem.size() == 1

    def test_grow_preserves_data(self) -> None:
        mem = LinearMemory(1, max_pages=4)
        mem.store_i32(0, 12345)
        mem.grow(1)
        assert mem.load_i32(0) == 12345

    def test_grow_zero_pages(self) -> None:
        mem = LinearMemory(1)
        old = mem.grow(0)
        assert old == 1
        assert mem.size() == 1

    def test_grow_beyond_spec_max(self) -> None:
        """Growing past MAX_PAGES should fail even without explicit max."""
        mem = LinearMemory(1)
        result = mem.grow(MAX_PAGES + 1)
        assert result == -1


# ===========================================================================
# write_bytes
# ===========================================================================


class TestWriteBytes:
    def test_write_and_load(self) -> None:
        mem = LinearMemory(1)
        data = b"Hello"
        mem.write_bytes(0, data)
        assert mem.load_i32_8u(0) == ord("H")
        assert mem.load_i32_8u(4) == ord("o")

    def test_write_bytes_oob(self) -> None:
        mem = LinearMemory(1)
        with pytest.raises(TrapError, match="Out of bounds"):
            mem.write_bytes(PAGE_SIZE - 2, b"abc")
