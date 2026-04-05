"""linear_memory.py --- WASM linear memory implementation.

===========================================================================
WHAT IS LINEAR MEMORY?
===========================================================================

WebAssembly's memory model is a contiguous, byte-addressable array of bytes.
Memory is measured in "pages" of 65,536 bytes (64 KiB). A module declares a
minimum number of pages and optionally a maximum. The ``memory.grow``
instruction can add pages at runtime.

Memory accesses are bounds-checked: reading or writing past the end causes
a trap (TrapError).

===========================================================================
LITTLE-ENDIAN BYTE ORDERING
===========================================================================

WASM always uses little-endian byte order. We use Python's ``struct`` module
with '<' (little-endian) format to handle typed reads/writes.
"""

from __future__ import annotations

import struct

from wasm_execution.host_interface import TrapError

# ===========================================================================
# Constants
# ===========================================================================

PAGE_SIZE = 65536
"""Bytes per WASM memory page: 64 KiB."""

MAX_PAGES = 65536
"""WASM spec maximum memory pages (4 GiB total)."""


# ===========================================================================
# LinearMemory
# ===========================================================================


class LinearMemory:
    """WASM linear memory: a bounds-checked, resizable byte array.

    All loads and stores are little-endian, matching the WASM specification.
    Uses Python's ``struct`` module for typed access and ``bytearray`` for
    the backing storage.
    """

    PAGE_SIZE = PAGE_SIZE

    def __init__(self, initial_pages: int, max_pages: int | None = None) -> None:
        self._current_pages = initial_pages
        self._max_pages = max_pages
        self._data = bytearray(initial_pages * PAGE_SIZE)

    # =====================================================================
    # Bounds Checking
    # =====================================================================

    def _bounds_check(self, offset: int, width: int) -> None:
        """Validate that accessing ``width`` bytes at ``offset`` is in bounds."""
        if offset < 0 or offset + width > len(self._data):
            msg = (
                f"Out of bounds memory access: offset={offset}, size={width}, "
                f"memory size={len(self._data)}"
            )
            raise TrapError(msg)

    # =====================================================================
    # Full-Width Loads
    # =====================================================================

    def load_i32(self, offset: int) -> int:
        """Load 4 bytes as a signed 32-bit integer (little-endian)."""
        self._bounds_check(offset, 4)
        return struct.unpack_from("<i", self._data, offset)[0]

    def load_i64(self, offset: int) -> int:
        """Load 8 bytes as a signed 64-bit integer (little-endian)."""
        self._bounds_check(offset, 8)
        return struct.unpack_from("<q", self._data, offset)[0]

    def load_f32(self, offset: int) -> float:
        """Load 4 bytes as a 32-bit float (little-endian)."""
        self._bounds_check(offset, 4)
        return struct.unpack_from("<f", self._data, offset)[0]

    def load_f64(self, offset: int) -> float:
        """Load 8 bytes as a 64-bit float (little-endian)."""
        self._bounds_check(offset, 8)
        return struct.unpack_from("<d", self._data, offset)[0]

    # =====================================================================
    # Narrow Loads for i32 (8-bit and 16-bit)
    # =====================================================================

    def load_i32_8s(self, offset: int) -> int:
        """Load 1 byte, sign-extend to i32."""
        self._bounds_check(offset, 1)
        return struct.unpack_from("<b", self._data, offset)[0]

    def load_i32_8u(self, offset: int) -> int:
        """Load 1 byte, zero-extend to i32."""
        self._bounds_check(offset, 1)
        return struct.unpack_from("<B", self._data, offset)[0]

    def load_i32_16s(self, offset: int) -> int:
        """Load 2 bytes (little-endian), sign-extend to i32."""
        self._bounds_check(offset, 2)
        return struct.unpack_from("<h", self._data, offset)[0]

    def load_i32_16u(self, offset: int) -> int:
        """Load 2 bytes (little-endian), zero-extend to i32."""
        self._bounds_check(offset, 2)
        return struct.unpack_from("<H", self._data, offset)[0]

    # =====================================================================
    # Narrow Loads for i64 (8-bit, 16-bit, 32-bit)
    # =====================================================================

    def load_i64_8s(self, offset: int) -> int:
        """Load 1 byte, sign-extend to i64."""
        self._bounds_check(offset, 1)
        return struct.unpack_from("<b", self._data, offset)[0]

    def load_i64_8u(self, offset: int) -> int:
        """Load 1 byte, zero-extend to i64."""
        self._bounds_check(offset, 1)
        return struct.unpack_from("<B", self._data, offset)[0]

    def load_i64_16s(self, offset: int) -> int:
        """Load 2 bytes (little-endian), sign-extend to i64."""
        self._bounds_check(offset, 2)
        return struct.unpack_from("<h", self._data, offset)[0]

    def load_i64_16u(self, offset: int) -> int:
        """Load 2 bytes (little-endian), zero-extend to i64."""
        self._bounds_check(offset, 2)
        return struct.unpack_from("<H", self._data, offset)[0]

    def load_i64_32s(self, offset: int) -> int:
        """Load 4 bytes (little-endian), sign-extend to i64."""
        self._bounds_check(offset, 4)
        return struct.unpack_from("<i", self._data, offset)[0]

    def load_i64_32u(self, offset: int) -> int:
        """Load 4 bytes (little-endian), zero-extend to i64."""
        self._bounds_check(offset, 4)
        return struct.unpack_from("<I", self._data, offset)[0]

    # =====================================================================
    # Full-Width Stores
    # =====================================================================

    def store_i32(self, offset: int, value: int) -> None:
        """Store a 32-bit integer (little-endian)."""
        self._bounds_check(offset, 4)
        struct.pack_into("<i", self._data, offset, value & 0xFFFFFFFF if value >= 0 else value)

    def store_i64(self, offset: int, value: int) -> None:
        """Store a 64-bit integer (little-endian)."""
        self._bounds_check(offset, 8)
        struct.pack_into("<q", self._data, offset, value)

    def store_f32(self, offset: int, value: float) -> None:
        """Store a 32-bit float (little-endian)."""
        self._bounds_check(offset, 4)
        struct.pack_into("<f", self._data, offset, value)

    def store_f64(self, offset: int, value: float) -> None:
        """Store a 64-bit float (little-endian)."""
        self._bounds_check(offset, 8)
        struct.pack_into("<d", self._data, offset, value)

    # =====================================================================
    # Narrow Stores (truncate to smaller width)
    # =====================================================================

    def store_i32_8(self, offset: int, value: int) -> None:
        """Store the low 8 bits of an i32."""
        self._bounds_check(offset, 1)
        struct.pack_into("<b", self._data, offset, (value & 0xFF) if (value & 0xFF) < 128 else (value & 0xFF) - 256)

    def store_i32_16(self, offset: int, value: int) -> None:
        """Store the low 16 bits of an i32 (little-endian)."""
        self._bounds_check(offset, 2)
        raw = value & 0xFFFF
        struct.pack_into("<h", self._data, offset, raw if raw < 0x8000 else raw - 0x10000)

    def store_i64_8(self, offset: int, value: int) -> None:
        """Store the low 8 bits of an i64."""
        self._bounds_check(offset, 1)
        raw = value & 0xFF
        struct.pack_into("<b", self._data, offset, raw if raw < 128 else raw - 256)

    def store_i64_16(self, offset: int, value: int) -> None:
        """Store the low 16 bits of an i64 (little-endian)."""
        self._bounds_check(offset, 2)
        raw = value & 0xFFFF
        struct.pack_into("<h", self._data, offset, raw if raw < 0x8000 else raw - 0x10000)

    def store_i64_32(self, offset: int, value: int) -> None:
        """Store the low 32 bits of an i64 (little-endian)."""
        self._bounds_check(offset, 4)
        raw = value & 0xFFFFFFFF
        struct.pack_into("<i", self._data, offset, raw if raw < 0x80000000 else raw - 0x100000000)

    # =====================================================================
    # Memory Growth
    # =====================================================================

    def grow(self, delta_pages: int) -> int:
        """Grow memory by ``delta_pages`` pages. Returns old page count or -1."""
        old_pages = self._current_pages
        new_pages = old_pages + delta_pages

        if self._max_pages is not None and new_pages > self._max_pages:
            return -1
        if new_pages > MAX_PAGES:
            return -1

        new_data = bytearray(new_pages * PAGE_SIZE)
        new_data[: len(self._data)] = self._data
        self._data = new_data
        self._current_pages = new_pages
        return old_pages

    # =====================================================================
    # Size Queries
    # =====================================================================

    def size(self) -> int:
        """Return current memory size in pages."""
        return self._current_pages

    def byte_length(self) -> int:
        """Return current memory size in bytes."""
        return len(self._data)

    # =====================================================================
    # Raw Byte Access
    # =====================================================================

    def write_bytes(self, offset: int, data: bytes | bytearray) -> None:
        """Write raw bytes into memory at the given offset."""
        self._bounds_check(offset, len(data))
        self._data[offset : offset + len(data)] = data
