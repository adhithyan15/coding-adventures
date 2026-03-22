"""Tests for SharedMemoryRegion -- named shared memory segments.

These tests verify:
1. Attach and detach by PID
2. Read and write at arbitrary offsets
3. Bounds checking (negative offset, beyond region size)
4. Multiple PIDs sharing the same region
5. Properties (name, size, owner_pid, attached_count, is_attached)
6. Zero-initialized data
"""

import pytest

from ipc import SharedMemoryRegion

# ========================================================================
# Attach / Detach
# ========================================================================


class TestSharedMemoryAttachDetach:
    """Test process attachment lifecycle."""

    def test_attach_succeeds(self) -> None:
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        assert shm.attach(1) is True
        assert shm.is_attached(1)

    def test_attach_duplicate_returns_false(self) -> None:
        """Attaching the same PID twice returns False (already attached)."""
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        assert shm.attach(1) is True
        assert shm.attach(1) is False  # already attached

    def test_detach_succeeds(self) -> None:
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        shm.attach(1)
        assert shm.detach(1) is True
        assert not shm.is_attached(1)

    def test_detach_not_attached_returns_false(self) -> None:
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        assert shm.detach(99) is False

    def test_multiple_pids(self) -> None:
        """Multiple processes can attach to the same region."""
        shm = SharedMemoryRegion("buffer", size=4096, owner_pid=1)
        shm.attach(1)
        shm.attach(2)
        shm.attach(3)
        assert shm.attached_count == 3
        assert shm.is_attached(1)
        assert shm.is_attached(2)
        assert shm.is_attached(3)

    def test_detach_reduces_count(self) -> None:
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        shm.attach(1)
        shm.attach(2)
        assert shm.attached_count == 2
        shm.detach(1)
        assert shm.attached_count == 1


# ========================================================================
# Read / Write
# ========================================================================


class TestSharedMemoryReadWrite:
    """Test reading and writing data at offsets."""

    def test_write_and_read(self) -> None:
        """Write bytes at offset 0, read them back."""
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        written = shm.write(0, b"hello")
        assert written == 5
        data = shm.read(0, 5)
        assert data == b"hello"

    def test_write_at_offset(self) -> None:
        """Write at a non-zero offset."""
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        shm.write(100, b"data")
        assert shm.read(100, 4) == b"data"
        # Bytes before the write are still zero
        assert shm.read(0, 4) == b"\x00\x00\x00\x00"

    def test_overwrite(self) -> None:
        """Writing to the same offset overwrites previous data."""
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        shm.write(0, b"old")
        shm.write(0, b"new")
        assert shm.read(0, 3) == b"new"

    def test_zero_initialized(self) -> None:
        """A fresh region is all zeros, like freshly allocated memory."""
        shm = SharedMemoryRegion("test", size=16, owner_pid=1)
        data = shm.read(0, 16)
        assert data == b"\x00" * 16

    def test_multi_process_visibility(self) -> None:
        """Write from one 'process', read from another -- sees same data.

        This is the core shared memory invariant: both processes see the
        same bytes because they map the same physical pages.
        """
        shm = SharedMemoryRegion("cache", size=4096, owner_pid=1)
        shm.attach(1)
        shm.attach(2)

        # Process 1 writes
        shm.write(0, b"shared data from process 1")

        # Process 2 reads -- sees the same data
        data = shm.read(0, 26)
        assert data == b"shared data from process 1"

    def test_read_returns_bytes(self) -> None:
        """read() returns a bytes object, not bytearray."""
        shm = SharedMemoryRegion("test", size=16, owner_pid=1)
        result = shm.read(0, 4)
        assert isinstance(result, bytes)


# ========================================================================
# Bounds checking
# ========================================================================


class TestSharedMemoryBounds:
    """Test that out-of-bounds access is caught."""

    def test_read_beyond_size(self) -> None:
        shm = SharedMemoryRegion("test", size=16, owner_pid=1)
        with pytest.raises(ValueError, match="beyond region bounds"):
            shm.read(10, 10)  # offset=10 + count=10 = 20 > 16

    def test_write_beyond_size(self) -> None:
        shm = SharedMemoryRegion("test", size=16, owner_pid=1)
        with pytest.raises(ValueError, match="beyond region bounds"):
            shm.write(10, b"0123456789")  # 10 bytes at offset 10 = 20 > 16

    def test_negative_read_offset(self) -> None:
        shm = SharedMemoryRegion("test", size=16, owner_pid=1)
        with pytest.raises(ValueError, match="negative offset"):
            shm.read(-1, 4)

    def test_negative_write_offset(self) -> None:
        shm = SharedMemoryRegion("test", size=16, owner_pid=1)
        with pytest.raises(ValueError, match="negative offset"):
            shm.write(-1, b"data")

    def test_read_exactly_at_boundary(self) -> None:
        """Reading up to exactly the region size is fine."""
        shm = SharedMemoryRegion("test", size=8, owner_pid=1)
        shm.write(0, b"12345678")
        data = shm.read(0, 8)
        assert data == b"12345678"

    def test_write_exactly_at_boundary(self) -> None:
        """Writing up to exactly the region size is fine."""
        shm = SharedMemoryRegion("test", size=8, owner_pid=1)
        written = shm.write(0, b"12345678")
        assert written == 8


# ========================================================================
# Properties
# ========================================================================


class TestSharedMemoryProperties:
    """Test property accessors."""

    def test_name(self) -> None:
        shm = SharedMemoryRegion("my_region", size=1024, owner_pid=42)
        assert shm.name == "my_region"

    def test_size(self) -> None:
        shm = SharedMemoryRegion("test", size=2048, owner_pid=1)
        assert shm.size == 2048

    def test_owner_pid(self) -> None:
        shm = SharedMemoryRegion("test", size=1024, owner_pid=42)
        assert shm.owner_pid == 42

    def test_attached_count_empty(self) -> None:
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        assert shm.attached_count == 0

    def test_is_attached_false(self) -> None:
        shm = SharedMemoryRegion("test", size=1024, owner_pid=1)
        assert not shm.is_attached(99)

    def test_default_owner_pid(self) -> None:
        shm = SharedMemoryRegion("test", size=1024)
        assert shm.owner_pid == 0
