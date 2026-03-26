"""MemoryController -- serializes memory requests from multiple cores.

# Why a Memory Controller?

In a multi-core system, multiple cores may request memory access in the
same clock cycle. Real memory (DRAM) can only handle a limited number of
concurrent requests, so the memory controller queues and serializes them.

The memory controller is like a librarian at a busy library: patrons
(cores) line up with their requests, and the librarian processes them
one at a time, delivering books (data) after a delay (latency).

# Latency Simulation

Each memory request takes ``latency`` cycles to complete. The controller
counts down the remaining cycles on each tick(). When a request reaches
zero remaining cycles, its data is delivered to the requester.

# Memory Model

The underlying memory is a flat byte array. Word reads/writes use
little-endian byte ordering, matching modern ARM and x86 architectures.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class MemoryRequest:
    """An in-flight read request."""

    address: int  # Starting byte address
    num_bytes: int  # Number of bytes to read
    requester_id: int  # Which core submitted the request
    cycles_left: int  # Cycles remaining until data is ready


@dataclass
class MemoryWriteRequest:
    """An in-flight write request."""

    address: int  # Starting byte address
    data: bytes  # Bytes to write
    requester_id: int  # Which core submitted the request
    cycles_left: int  # Cycles remaining until write completes


@dataclass
class MemoryReadResult:
    """A completed read -- data delivered to a requester."""

    requester_id: int  # Which core receives this data
    address: int  # The address that was read
    data: bytes  # The bytes that were read


class MemoryController:
    """Manages access to shared main memory from multiple cores.

    The memory slice is shared (not copied) -- multiple cores access the same
    underlying bytes. This models shared physical memory in a multi-core system.
    """

    def __init__(self, memory: bytearray, latency: int) -> None:
        """Create a memory controller with the given backing memory and latency.

        Args:
            memory: Shared backing memory (bytearray, modified in-place).
            latency: Number of cycles for a memory access to complete.
        """
        self._memory = memory
        self._latency = latency
        self._pending_reads: list[MemoryRequest] = []
        self._pending_writes: list[MemoryWriteRequest] = []

    def request_read(
        self, address: int, num_bytes: int, requester_id: int,
    ) -> None:
        """Submit an asynchronous read request.

        The read will complete after ``latency`` cycles. Call tick() each
        cycle and check the returned results for completed reads.

        Args:
            address: Starting byte address to read.
            num_bytes: Number of bytes to read.
            requester_id: ID of the requesting core.
        """
        self._pending_reads.append(
            MemoryRequest(
                address=address,
                num_bytes=num_bytes,
                requester_id=requester_id,
                cycles_left=self._latency,
            )
        )

    def request_write(
        self, address: int, data: bytes, requester_id: int,
    ) -> None:
        """Submit an asynchronous write request.

        The write completes after ``latency`` cycles. The data is committed
        to memory when the request finishes (not immediately).

        Args:
            address: Starting byte address to write.
            data: Bytes to write.
            requester_id: ID of the requesting core.
        """
        self._pending_writes.append(
            MemoryWriteRequest(
                address=address,
                data=bytes(data),  # defensive copy
                requester_id=requester_id,
                cycles_left=self._latency,
            )
        )

    def tick(self) -> list[MemoryReadResult]:
        """Advance the memory controller by one cycle.

        Decrements all pending request counters. When a request reaches zero
        remaining cycles, it is completed:
          - Reads: data is copied from memory and returned in the result list
          - Writes: data is committed to memory

        Returns:
            List of completed read results (requester ID + data).
        """
        completed: list[MemoryReadResult] = []

        # Process pending reads.
        remaining_reads: list[MemoryRequest] = []
        for req in self._pending_reads:
            req.cycles_left -= 1
            if req.cycles_left <= 0:
                data = self._read_memory(req.address, req.num_bytes)
                completed.append(
                    MemoryReadResult(
                        requester_id=req.requester_id,
                        address=req.address,
                        data=data,
                    )
                )
            else:
                remaining_reads.append(req)
        self._pending_reads = remaining_reads

        # Process pending writes.
        remaining_writes: list[MemoryWriteRequest] = []
        for req in self._pending_writes:
            req.cycles_left -= 1
            if req.cycles_left <= 0:
                self._write_memory(req.address, req.data)
            else:
                remaining_writes.append(req)
        self._pending_writes = remaining_writes

        return completed

    def read_word(self, address: int) -> int:
        """Read a 32-bit word from memory at the given address.

        Uses little-endian byte order.

        Args:
            address: Byte address to read from.

        Returns:
            The 32-bit integer value, or 0 if out of bounds.
        """
        if address < 0 or address + 4 > len(self._memory):
            return 0
        return (
            int(self._memory[address])
            | (int(self._memory[address + 1]) << 8)
            | (int(self._memory[address + 2]) << 16)
            | (int(self._memory[address + 3]) << 24)
        )

    def write_word(self, address: int, value: int) -> None:
        """Write a 32-bit word to memory at the given address.

        Uses little-endian byte order.

        Args:
            address: Byte address to write to.
            value: 32-bit integer value to write.
        """
        if address < 0 or address + 4 > len(self._memory):
            return
        self._memory[address] = value & 0xFF
        self._memory[address + 1] = (value >> 8) & 0xFF
        self._memory[address + 2] = (value >> 16) & 0xFF
        self._memory[address + 3] = (value >> 24) & 0xFF

    def load_program(self, program: bytes, start_address: int) -> None:
        """Copy program bytes into memory starting at the given address.

        Args:
            program: Raw program bytes to load.
            start_address: Starting byte address in memory.
        """
        if start_address < 0 or start_address + len(program) > len(self._memory):
            return
        self._memory[start_address : start_address + len(program)] = program

    @property
    def memory_size(self) -> int:
        """Return the total size of memory in bytes."""
        return len(self._memory)

    @property
    def pending_count(self) -> int:
        """Return the number of in-flight requests."""
        return len(self._pending_reads) + len(self._pending_writes)

    def _read_memory(self, address: int, num_bytes: int) -> bytes:
        """Read bytes from the backing memory array."""
        if address < 0 or address + num_bytes > len(self._memory):
            return bytes(num_bytes)
        return bytes(self._memory[address : address + num_bytes])

    def _write_memory(self, address: int, data: bytes) -> None:
        """Write bytes to the backing memory array."""
        if address < 0 or address + len(data) > len(self._memory):
            return
        self._memory[address : address + len(data)] = data
