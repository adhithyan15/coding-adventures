"""Global Memory — device-wide VRAM / HBM simulator.

=== What is Global Memory? ===

Global memory is the large, high-bandwidth memory that serves the entire
accelerator device. Every compute unit can read from and write to global
memory, making it the shared data store for all parallel computation.

    NVIDIA: HBM3 (High Bandwidth Memory) — 80 GB on H100
    AMD:    GDDR6 — 24 GB on RX 7900 XTX
    Google: HBM2e — 32 GB per TPU v4 chip
    Intel:  GDDR6 — 16 GB on Arc A770
    Apple:  Unified LPDDR5 — shared with CPU/GPU, up to 192 GB

=== Key Properties ===

1. **High bandwidth**: 1-3 TB/s. Much faster than CPU memory (~50 GB/s).
   This is achieved through wide buses (4096-bit for HBM vs 64-bit DDR).

2. **High latency**: ~400-800 cycles to service a request. This is why
   GPUs need thousands of threads to hide the latency.

3. **Shared**: ALL compute units on the device share global memory.
   Unlike shared memory (per-CU, ~96 KB), global memory is device-wide.

4. **Coalescing**: The memory controller can merge multiple thread
   requests into fewer wide transactions if the addresses are contiguous.

5. **Partitioned**: Memory is physically split across channels/stacks.
   Accessing only one partition wastes bandwidth on the other partitions.

=== Memory Coalescing ===

Coalescing is the single most important optimization for GPU memory access.
When 32 threads in a warp access addresses that fall within the same
128-byte cache line, the hardware combines them into ONE transaction:

    Thread 0: addr 0x1000    ─┐
    Thread 1: addr 0x1004     │
    Thread 2: addr 0x1008     ├── All in same 128B line → 1 transaction
    ...                       │
    Thread 31: addr 0x107C   ─┘

    vs. scattered access:
    Thread 0: addr 0x1000    ── Transaction 1
    Thread 1: addr 0x5000    ── Transaction 2
    Thread 2: addr 0x9000    ── Transaction 3
    ...32 separate transactions = 32× more memory traffic!

=== Sparse Memory Representation ===

Real devices have 16-80 GB of VRAM. We obviously can't allocate that in
a simulator. Instead, we use a sparse dictionary: only addresses that have
been written to consume actual memory. A read to an uninitialized address
returns zeros (matching real hardware behavior after cudaMemset).
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

from device_simulator.protocols import (
    GlobalMemoryStats,
    MemoryTransaction,
)


class SimpleGlobalMemory:
    """Global memory implementation with coalescing and partitioning.

    This models the device-wide memory (VRAM/HBM) that all compute units
    share. It tracks access patterns, coalescing efficiency, and partition
    conflicts to help identify memory bottlenecks.

    === Usage ===

        mem = SimpleGlobalMemory(capacity=1024*1024, channels=4)

        # Allocate space
        addr = mem.allocate(256)

        # Copy data from host
        mem.copy_from_host(addr, b'\\x00' * 256, host_bandwidth=64.0)

        # Read/write
        mem.write(addr, b'\\x41\\x42\\x43\\x44')
        data = mem.read(addr, 4)

        # Check coalescing for a warp access
        transactions = mem.coalesce([addr + i*4 for i in range(32)])

    Args:
        capacity:       Total memory in bytes.
        bandwidth:      Peak bandwidth in bytes per cycle.
        latency:        Access latency in cycles.
        channels:       Number of memory partitions/channels.
        transaction_size: Width of a single memory transaction (bytes).
        host_bandwidth: PCIe/NVLink bandwidth in bytes per cycle.
        host_latency:   Initial latency for host transfers in cycles.
        unified:        If True, host transfers are zero-cost (Apple).
    """

    def __init__(
        self,
        capacity: int = 16 * 1024 * 1024,
        bandwidth: float = 1000.0,
        latency: int = 400,
        channels: int = 8,
        transaction_size: int = 128,
        host_bandwidth: float = 64.0,
        host_latency: int = 1000,
        unified: bool = False,
    ) -> None:
        self._capacity = capacity
        self._bandwidth = bandwidth
        self._latency = latency
        self._channels = channels
        self._transaction_size = transaction_size
        self._host_bandwidth = host_bandwidth
        self._host_latency = host_latency
        self._unified = unified

        # Sparse storage — only written addresses consume memory
        self._data: dict[int, int] = {}

        # Simple bump allocator
        self._next_free: int = 0
        self._allocations: dict[int, int] = {}  # start_addr -> size

        # Statistics
        self._stats = GlobalMemoryStats()

    # --- Properties ---

    @property
    def capacity(self) -> int:
        """Total memory in bytes."""
        return self._capacity

    @property
    def bandwidth(self) -> float:
        """Peak bandwidth in bytes per cycle."""
        return self._bandwidth

    @property
    def stats(self) -> GlobalMemoryStats:
        """Access statistics."""
        self._stats.update_efficiency()
        return self._stats

    # --- Allocation ---

    def allocate(self, size: int, alignment: int = 256) -> int:
        """Allocate memory. Returns the start address.

        Uses a simple bump allocator with alignment. Like cudaMalloc,
        this returns a device pointer that can be passed to kernels.

        Args:
            size:      Number of bytes to allocate.
            alignment: Alignment in bytes (default 256 for cache lines).

        Returns:
            Start address of the allocated region.

        Raises:
            MemoryError: If not enough memory remains.
        """
        # Align the next free pointer
        aligned = (self._next_free + alignment - 1) & ~(alignment - 1)

        if aligned + size > self._capacity:
            msg = (
                f"Out of device memory: requested {size} bytes "
                f"at {aligned}, capacity {self._capacity}"
            )
            raise MemoryError(msg)

        self._allocations[aligned] = size
        self._next_free = aligned + size
        return aligned

    def free(self, address: int) -> None:
        """Free a previous allocation.

        Note: our simple bump allocator doesn't reclaim memory. In a real
        implementation you'd use a more sophisticated allocator. But for
        simulation purposes, this tracks that the free was called.
        """
        if address in self._allocations:
            del self._allocations[address]

    # --- Read / Write ---

    def read(self, address: int, size: int) -> bytes:
        """Read bytes from global memory.

        Uninitialized addresses return zeros (like cudaMemset(0)).

        Args:
            address: Start address to read from.
            size:    Number of bytes to read.

        Returns:
            The data as bytes.

        Raises:
            IndexError: If address is out of range.
        """
        if address < 0 or address + size > self._capacity:
            msg = (
                f"Address {address}+{size} out of range "
                f"[0, {self._capacity})"
            )
            raise IndexError(msg)

        self._stats.total_reads += 1
        self._stats.bytes_transferred += size

        result = bytearray(size)
        for i in range(size):
            result[i] = self._data.get(address + i, 0)
        return bytes(result)

    def write(self, address: int, data: bytes) -> None:
        """Write bytes to global memory.

        Args:
            address: Start address to write to.
            data:    The data to write.

        Raises:
            IndexError: If address is out of range.
        """
        size = len(data)
        if address < 0 or address + size > self._capacity:
            msg = (
                f"Address {address}+{size} out of range "
                f"[0, {self._capacity})"
            )
            raise IndexError(msg)

        self._stats.total_writes += 1
        self._stats.bytes_transferred += size

        for i, byte_val in enumerate(data):
            self._data[address + i] = byte_val

    # --- Host transfers ---

    def copy_from_host(
        self,
        dst_addr: int,
        data: bytes,
        host_bandwidth: float | None = None,
    ) -> int:
        """Copy from host (CPU) to device memory.

        Like cudaMemcpy(dst, src, size, cudaMemcpyHostToDevice).

        For unified memory (Apple), this is zero-cost — no actual data
        movement, just a page table remap.

        Args:
            dst_addr:       Destination address in device memory.
            data:           The data to copy.
            host_bandwidth: Override for host bandwidth (bytes/cycle).

        Returns:
            Number of cycles consumed by the transfer.
        """
        self.write(dst_addr, data)

        bw = host_bandwidth or self._host_bandwidth
        size = len(data)
        self._stats.host_to_device_bytes += size

        if self._unified:
            # Unified memory: zero-copy
            return 0

        # Transfer time = latency + size / bandwidth
        cycles = self._host_latency + int(size / bw) if bw > 0 else 0
        self._stats.host_transfer_cycles += cycles
        return cycles

    def copy_to_host(
        self,
        src_addr: int,
        size: int,
        host_bandwidth: float | None = None,
    ) -> tuple[bytes, int]:
        """Copy from device memory to host (CPU).

        Like cudaMemcpy(dst, src, size, cudaMemcpyDeviceToHost).

        Returns:
            Tuple of (data, cycles_consumed).
        """
        data = self.read(src_addr, size)

        bw = host_bandwidth or self._host_bandwidth
        self._stats.device_to_host_bytes += size

        if self._unified:
            return data, 0

        cycles = self._host_latency + int(size / bw) if bw > 0 else 0
        self._stats.host_transfer_cycles += cycles
        return data, cycles

    # --- Coalescing ---

    def coalesce(
        self, addresses: list[int], size: int = 4
    ) -> list[MemoryTransaction]:
        """Given per-thread addresses, merge into coalesced transactions.

        === Coalescing Algorithm ===

        1. For each thread's address, compute which transaction-sized
           aligned region it falls in.
        2. Group threads by aligned region.
        3. Each group becomes one transaction.

        The fewer transactions, the better — ideal is 1 transaction
        for 32 threads (128 bytes of contiguous access).

        Args:
            addresses: List of addresses, one per thread.
            size:      Size of each thread's access in bytes.

        Returns:
            List of MemoryTransaction objects after coalescing.
        """
        ts = self._transaction_size

        # Group threads by aligned transaction address
        groups: dict[int, int] = {}  # aligned_addr -> thread_mask
        for thread_idx, addr in enumerate(addresses):
            aligned = (addr // ts) * ts
            if aligned not in groups:
                groups[aligned] = 0
            groups[aligned] |= 1 << thread_idx

        transactions = [
            MemoryTransaction(address=aligned, size=ts, thread_mask=mask)
            for aligned, mask in sorted(groups.items())
        ]

        # Track stats
        self._stats.total_requests += len(addresses)
        self._stats.total_transactions += len(transactions)

        # Check partition conflicts
        channels_hit: dict[int, int] = {}
        for txn in transactions:
            channel = (txn.address // ts) % self._channels
            channels_hit[channel] = channels_hit.get(channel, 0) + 1
        for count in channels_hit.values():
            if count > 1:
                self._stats.partition_conflicts += count - 1

        return transactions

    # --- Reset ---

    def reset(self) -> None:
        """Clear all data, allocations, and statistics."""
        self._data.clear()
        self._next_free = 0
        self._allocations.clear()
        self._stats = GlobalMemoryStats()
