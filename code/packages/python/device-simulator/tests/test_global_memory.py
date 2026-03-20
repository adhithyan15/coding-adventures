"""Tests for global memory — VRAM / HBM simulation."""

import struct

import pytest

from device_simulator import SimpleGlobalMemory, MemoryTransaction


# =========================================================================
# Basic read/write
# =========================================================================


class TestReadWrite:
    def test_write_and_read_back(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        mem.write(0, b"\x41\x42\x43\x44")
        data = mem.read(0, 4)
        assert data == b"\x41\x42\x43\x44"

    def test_read_uninitialized_returns_zeros(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        data = mem.read(0, 8)
        assert data == b"\x00" * 8

    def test_write_float(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        value = 3.14
        raw = struct.pack("<f", value)
        mem.write(0, raw)
        data = mem.read(0, 4)
        result = struct.unpack("<f", data)[0]
        assert abs(result - 3.14) < 0.01

    def test_read_out_of_range(self) -> None:
        mem = SimpleGlobalMemory(capacity=64)
        with pytest.raises(IndexError):
            mem.read(60, 8)  # 60+8=68 > 64

    def test_write_out_of_range(self) -> None:
        mem = SimpleGlobalMemory(capacity=64)
        with pytest.raises(IndexError):
            mem.write(60, b"\x00" * 8)

    def test_read_negative_address(self) -> None:
        mem = SimpleGlobalMemory(capacity=64)
        with pytest.raises(IndexError):
            mem.read(-1, 4)

    def test_write_negative_address(self) -> None:
        mem = SimpleGlobalMemory(capacity=64)
        with pytest.raises(IndexError):
            mem.write(-1, b"\x00")

    def test_multiple_writes_at_different_addresses(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        mem.write(0, b"\x01\x02")
        mem.write(100, b"\x03\x04")
        assert mem.read(0, 2) == b"\x01\x02"
        assert mem.read(100, 2) == b"\x03\x04"

    def test_overwrite(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        mem.write(0, b"\x01\x02")
        mem.write(0, b"\x03\x04")
        assert mem.read(0, 2) == b"\x03\x04"


# =========================================================================
# Allocation
# =========================================================================


class TestAllocation:
    def test_allocate_returns_aligned_address(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024 * 1024)
        addr = mem.allocate(256, alignment=256)
        assert addr % 256 == 0

    def test_sequential_allocations_dont_overlap(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024 * 1024)
        a1 = mem.allocate(256)
        a2 = mem.allocate(256)
        assert a2 >= a1 + 256

    def test_allocate_out_of_memory(self) -> None:
        mem = SimpleGlobalMemory(capacity=512)
        mem.allocate(256)
        with pytest.raises(MemoryError):
            mem.allocate(512)  # not enough left

    def test_free_tracked(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        addr = mem.allocate(128)
        mem.free(addr)
        # Free doesn't reclaim in bump allocator, but should not crash
        mem.free(addr)  # Double free is a no-op

    def test_allocate_with_default_alignment(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024 * 1024)
        addr = mem.allocate(64)
        assert addr % 256 == 0  # Default alignment is 256


# =========================================================================
# Host transfers
# =========================================================================


class TestHostTransfers:
    def test_copy_from_host(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024, host_bandwidth=64.0, host_latency=100)
        cycles = mem.copy_from_host(0, b"\x01" * 128)
        assert cycles > 0
        assert mem.read(0, 4) == b"\x01\x01\x01\x01"

    def test_copy_to_host(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024, host_bandwidth=64.0, host_latency=100)
        mem.write(0, b"\xAA\xBB\xCC\xDD")
        data, cycles = mem.copy_to_host(0, 4)
        assert data == b"\xAA\xBB\xCC\xDD"
        assert cycles > 0

    def test_unified_memory_zero_cost(self) -> None:
        """Apple-style unified memory: transfers are free."""
        mem = SimpleGlobalMemory(capacity=1024, unified=True)
        cycles = mem.copy_from_host(0, b"\x01" * 256)
        assert cycles == 0

        data, cycles = mem.copy_to_host(0, 256)
        assert cycles == 0
        assert data == b"\x01" * 256

    def test_transfer_stats_tracked(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024, host_bandwidth=64.0, host_latency=10)
        mem.copy_from_host(0, b"\x00" * 128)
        stats = mem.stats
        assert stats.host_to_device_bytes == 128
        assert stats.host_transfer_cycles > 0

    def test_device_to_host_stats(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024, host_bandwidth=64.0, host_latency=10)
        mem.write(0, b"\x00" * 64)
        _, _ = mem.copy_to_host(0, 64)
        stats = mem.stats
        assert stats.device_to_host_bytes == 64


# =========================================================================
# Coalescing
# =========================================================================


class TestCoalescing:
    def test_fully_coalesced_access(self) -> None:
        """32 threads accessing contiguous 4-byte values = 1 transaction."""
        mem = SimpleGlobalMemory(capacity=1024, transaction_size=128)
        addrs = [i * 4 for i in range(32)]  # 0, 4, 8, ..., 124
        transactions = mem.coalesce(addrs)
        assert len(transactions) == 1
        assert transactions[0].size == 128
        assert transactions[0].address == 0

    def test_scattered_access_many_transactions(self) -> None:
        """Threads accessing addresses in different 128B regions."""
        mem = SimpleGlobalMemory(capacity=1024 * 1024, transaction_size=128)
        addrs = [i * 512 for i in range(4)]  # 0, 512, 1024, 1536
        transactions = mem.coalesce(addrs)
        assert len(transactions) == 4

    def test_two_transactions_for_strided(self) -> None:
        """Addresses spanning two 128B regions."""
        mem = SimpleGlobalMemory(capacity=1024, transaction_size=128)
        # First 16 in region 0, next 16 in region 1
        addrs = [i * 4 for i in range(32)]  # 0..124 in region 0
        addrs.extend([128 + i * 4 for i in range(32)])  # 128..252 in region 1
        transactions = mem.coalesce(addrs)
        assert len(transactions) == 2

    def test_thread_mask_correct(self) -> None:
        """Thread mask indicates which threads are served."""
        mem = SimpleGlobalMemory(capacity=1024, transaction_size=128)
        addrs = [0, 4, 256]  # threads 0,1 in region 0; thread 2 in region 2
        transactions = mem.coalesce(addrs)
        assert len(transactions) == 2
        # Thread 0 and 1 should be in the first transaction
        first = [t for t in transactions if t.address == 0][0]
        assert first.thread_mask & 0b11 == 0b11  # threads 0 and 1

    def test_coalescing_stats(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024, transaction_size=128)
        mem.coalesce([i * 4 for i in range(32)])
        stats = mem.stats
        assert stats.total_requests == 32
        assert stats.total_transactions == 1
        assert stats.coalescing_efficiency == 32.0


# =========================================================================
# Partition conflicts
# =========================================================================


class TestPartitionConflicts:
    def test_no_partition_conflict(self) -> None:
        """Transactions spread across different channels."""
        mem = SimpleGlobalMemory(capacity=1024, channels=4, transaction_size=128)
        # Addresses that map to different channels
        addrs = [i * 128 for i in range(4)]
        mem.coalesce(addrs)
        stats = mem.stats
        assert stats.partition_conflicts == 0

    def test_partition_conflict_detected(self) -> None:
        """Multiple transactions hitting the same channel."""
        mem = SimpleGlobalMemory(capacity=4096, channels=4, transaction_size=128)
        # Addresses 0 and 512 both map to channel 0 (with 4 channels)
        addrs = [0, 512]
        mem.coalesce(addrs)
        stats = mem.stats
        assert stats.partition_conflicts >= 1


# =========================================================================
# Reset
# =========================================================================


class TestReset:
    def test_reset_clears_data(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        mem.write(0, b"\xFF" * 4)
        mem.reset()
        assert mem.read(0, 4) == b"\x00" * 4

    def test_reset_clears_stats(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        mem.write(0, b"\x00")
        mem.read(0, 1)
        mem.reset()
        stats = mem.stats
        assert stats.total_reads == 0
        assert stats.total_writes == 0

    def test_reset_clears_allocations(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024)
        mem.allocate(512)
        mem.reset()
        # Should be able to allocate from start again
        addr = mem.allocate(512)
        assert addr == 0


# =========================================================================
# Properties
# =========================================================================


class TestProperties:
    def test_capacity(self) -> None:
        mem = SimpleGlobalMemory(capacity=4096)
        assert mem.capacity == 4096

    def test_bandwidth(self) -> None:
        mem = SimpleGlobalMemory(capacity=1024, bandwidth=3350.0)
        assert mem.bandwidth == 3350.0
