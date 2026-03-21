"""Memory management — typed allocations, mapping, staging.

=== Memory Types on a GPU ===

Unlike a CPU where all RAM is equally accessible, GPUs have distinct memory
pools with different performance characteristics:

    ┌─────────────────────────────────────────────────────────────────┐
    │                    Discrete GPU (NVIDIA, AMD)                    │
    │                                                                 │
    │   CPU side (system RAM)              GPU side (VRAM)            │
    │   ┌──────────────────┐               ┌──────────────────┐      │
    │   │   HOST_VISIBLE   │◄──── PCIe ───►│   DEVICE_LOCAL   │      │
    │   │   HOST_COHERENT  │   ~32 GB/s    │   (HBM / GDDR6)  │      │
    │   │   (staging pool) │               │   1-3 TB/s        │      │
    │   └──────────────────┘               └──────────────────┘      │
    └─────────────────────────────────────────────────────────────────┘

    ┌─────────────────────────────────────────────────────────────────┐
    │                  Unified Memory (Apple M-series)                 │
    │                                                                 │
    │   ┌──────────────────────────────────────────────────────┐     │
    │   │        DEVICE_LOCAL + HOST_VISIBLE + HOST_COHERENT    │     │
    │   │        (shared physical RAM)                          │     │
    │   │        Both CPU and GPU see the same bytes            │     │
    │   │        No copy needed!                                │     │
    │   └──────────────────────────────────────────────────────┘     │
    └─────────────────────────────────────────────────────────────────┘

=== The Staging Buffer Pattern ===

On discrete GPUs, the standard way to get data onto the GPU is:

    1. Allocate a HOST_VISIBLE staging buffer (CPU can write to it)
    2. Map it, write your data, unmap it
    3. Record a cmd_copy_buffer from staging → DEVICE_LOCAL
    4. Submit and wait

This two-step dance is necessary because DEVICE_LOCAL memory (VRAM) is
not directly writable by the CPU. The staging buffer lives in PCIe-accessible
system RAM where the CPU can write, then the DMA engine copies it to VRAM.

On unified memory (Apple), you skip all of this — allocate DEVICE_LOCAL +
HOST_VISIBLE, write directly, and the GPU sees it immediately.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .protocols import (
    BufferUsage,
    MemoryHeap,
    MemoryProperties,
    MemoryType,
    RuntimeEventType,
    RuntimeStats,
    RuntimeTrace,
)


# =========================================================================
# Buffer — a typed allocation on the device
# =========================================================================


@dataclass
class Buffer:
    """A memory allocation on the device.

    === Buffer Lifecycle ===

        allocate() → Buffer (with device_address)
        map()      → MappedMemory (CPU can read/write)
        unmap()    → buffer is GPU-only again
        free()     → memory returned to pool

    Fields:
        buffer_id:       Unique identifier for this buffer.
        size:            Size in bytes.
        memory_type:     What kind of memory (DEVICE_LOCAL, HOST_VISIBLE, etc.).
        usage:           How it will be used (STORAGE, TRANSFER_SRC, etc.).
        device_address:  Address on the device (from Layer 6 malloc).
        mapped:          Whether this buffer is currently CPU-mapped.
        freed:           Whether this buffer has been freed.
    """

    buffer_id: int
    size: int
    memory_type: MemoryType
    usage: BufferUsage
    device_address: int = 0
    mapped: bool = False
    freed: bool = False


# =========================================================================
# MappedMemory — CPU-accessible view of a buffer
# =========================================================================


class MappedMemory:
    """CPU-accessible view of a mapped GPU buffer.

    === What is Memory Mapping? ===

    Mapping makes device memory accessible to the CPU. On discrete GPUs,
    this only works for HOST_VISIBLE memory (system RAM accessible via PCIe).
    On unified memory, any buffer can be mapped.

    After mapping, you can read() and write() bytes. After unmap(), the
    CPU can no longer access this memory.

    === Implementation ===

    Under the hood, we store the data in a Python bytearray. When the
    buffer is unmapped, we sync it back to the device's memory via Layer 6.
    """

    def __init__(self, buffer: Buffer, data: bytearray) -> None:
        self._buffer = buffer
        self._data = data
        self._dirty = False

    @property
    def buffer(self) -> Buffer:
        """The buffer this mapping refers to."""
        return self._buffer

    @property
    def size(self) -> int:
        """Size of the mapped region."""
        return len(self._data)

    @property
    def dirty(self) -> bool:
        """Whether any writes have been made since mapping."""
        return self._dirty

    def read(self, offset: int, size: int) -> bytes:
        """Read bytes from the mapped buffer.

        Args:
            offset: Byte offset from start of buffer.
            size:   Number of bytes to read.

        Returns:
            The requested bytes.

        Raises:
            ValueError: If offset + size exceeds buffer size.
        """
        if offset + size > len(self._data):
            raise ValueError(
                f"Read out of bounds: offset={offset}, size={size}, "
                f"buffer_size={len(self._data)}"
            )
        return bytes(self._data[offset : offset + size])

    def write(self, offset: int, data: bytes) -> None:
        """Write bytes to the mapped buffer.

        Args:
            offset: Byte offset from start of buffer.
            data:   Bytes to write.

        Raises:
            ValueError: If offset + len(data) exceeds buffer size.
        """
        if offset + len(data) > len(self._data):
            raise ValueError(
                f"Write out of bounds: offset={offset}, data_size={len(data)}, "
                f"buffer_size={len(self._data)}"
            )
        self._data[offset : offset + len(data)] = data
        self._dirty = True

    def get_data(self) -> bytes:
        """Get the full contents of the mapped buffer."""
        return bytes(self._data)


# =========================================================================
# MemoryManager — allocates, maps, frees device memory
# =========================================================================


class MemoryManager:
    """Manages typed memory allocations on a device.

    === How It Works ===

    The MemoryManager wraps Layer 6's raw malloc/free with type information.
    Each allocation is tagged with a MemoryType and BufferUsage, which the
    runtime uses for validation and optimization.

    For HOST_VISIBLE allocations, the manager supports mapping — making the
    buffer accessible to the CPU. For DEVICE_LOCAL-only allocations, mapping
    is not allowed (you must use a staging buffer + copy).

    === Memory Tracking ===

    The manager tracks all allocations, peak usage, and produces traces
    for every operation. This lets you:
    - Find memory leaks (allocated but never freed)
    - Measure peak memory usage
    - Understand data transfer patterns
    """

    def __init__(
        self,
        device: AcceleratorDevice,
        memory_properties: MemoryProperties,
        stats: RuntimeStats,
    ) -> None:
        self._device = device
        self._properties = memory_properties
        self._stats = stats
        self._buffers: dict[int, Buffer] = {}
        self._buffer_data: dict[int, bytearray] = {}
        self._next_id = 0
        self._current_bytes = 0

    @property
    def memory_properties(self) -> MemoryProperties:
        """Memory properties of the underlying device."""
        return self._properties

    def allocate(
        self,
        size: int,
        memory_type: MemoryType,
        usage: BufferUsage = BufferUsage.STORAGE,
    ) -> Buffer:
        """Allocate a buffer on the device.

        === The Allocation Flow ===

            MemoryManager.allocate(1024, DEVICE_LOCAL)
                │
                ├──► Validate: size > 0, memory type supported
                ├──► Layer 6: device.malloc(1024) → device_address
                ├──► Create Buffer object with metadata
                ├──► Track in _buffers dict
                └──► Log RuntimeTrace event

        Args:
            size:         Number of bytes to allocate.
            memory_type:  Where to allocate (DEVICE_LOCAL, HOST_VISIBLE, etc.).
            usage:        How the buffer will be used (STORAGE, TRANSFER_SRC, etc.).

        Returns:
            A Buffer with a valid device_address.

        Raises:
            ValueError: If size <= 0 or memory type not supported.
        """
        if size <= 0:
            raise ValueError(f"Allocation size must be positive, got {size}")

        # Allocate on the underlying device
        device_address = self._device.malloc(size)

        buf_id = self._next_id
        self._next_id += 1

        buf = Buffer(
            buffer_id=buf_id,
            size=size,
            memory_type=memory_type,
            usage=usage,
            device_address=device_address,
        )
        self._buffers[buf_id] = buf
        self._buffer_data[buf_id] = bytearray(size)

        # Track stats
        self._current_bytes += size
        self._stats.total_allocated_bytes += size
        self._stats.total_allocations += 1
        if self._current_bytes > self._stats.peak_allocated_bytes:
            self._stats.peak_allocated_bytes = self._current_bytes

        self._stats.traces.append(
            RuntimeTrace(
                event_type=RuntimeEventType.MEMORY_ALLOC,
                description=f"Allocated {size} bytes (buf#{buf_id}, {memory_type})",
            )
        )

        return buf

    def free(self, buffer: Buffer) -> None:
        """Free a device memory allocation.

        Args:
            buffer: The buffer to free.

        Raises:
            ValueError: If buffer is already freed or not found.
        """
        if buffer.freed:
            raise ValueError(f"Buffer {buffer.buffer_id} already freed")
        if buffer.buffer_id not in self._buffers:
            raise ValueError(f"Buffer {buffer.buffer_id} not found")
        if buffer.mapped:
            raise ValueError(
                f"Buffer {buffer.buffer_id} is still mapped — unmap before freeing"
            )

        self._device.free(buffer.device_address)
        buffer.freed = True
        self._current_bytes -= buffer.size
        del self._buffers[buffer.buffer_id]
        del self._buffer_data[buffer.buffer_id]

        self._stats.total_frees += 1
        self._stats.traces.append(
            RuntimeTrace(
                event_type=RuntimeEventType.MEMORY_FREE,
                description=f"Freed buf#{buffer.buffer_id} ({buffer.size} bytes)",
            )
        )

    def map(self, buffer: Buffer) -> MappedMemory:
        """Map a buffer for CPU access.

        === When Can You Map? ===

        Only HOST_VISIBLE buffers can be mapped. On discrete GPUs, this
        means staging buffers (in system RAM). On unified memory devices,
        all buffers are HOST_VISIBLE so everything can be mapped.

        Args:
            buffer: The buffer to map.

        Returns:
            A MappedMemory object for reading/writing.

        Raises:
            ValueError: If buffer is not HOST_VISIBLE, already mapped, or freed.
        """
        if buffer.freed:
            raise ValueError(f"Cannot map freed buffer {buffer.buffer_id}")
        if buffer.mapped:
            raise ValueError(f"Buffer {buffer.buffer_id} is already mapped")
        if not (MemoryType.HOST_VISIBLE in buffer.memory_type):
            raise ValueError(
                f"Cannot map buffer {buffer.buffer_id}: not HOST_VISIBLE "
                f"(type={buffer.memory_type})"
            )

        buffer.mapped = True
        self._stats.total_maps += 1

        self._stats.traces.append(
            RuntimeTrace(
                event_type=RuntimeEventType.MEMORY_MAP,
                description=f"Mapped buf#{buffer.buffer_id}",
            )
        )

        return MappedMemory(buffer, self._buffer_data[buffer.buffer_id])

    def unmap(self, buffer: Buffer) -> None:
        """Unmap a buffer, ending CPU access.

        If the mapped memory was written to (dirty), and the buffer has
        HOST_COHERENT, the data is automatically synced to the device.

        Args:
            buffer: The buffer to unmap.

        Raises:
            ValueError: If buffer is not currently mapped.
        """
        if not buffer.mapped:
            raise ValueError(f"Buffer {buffer.buffer_id} is not mapped")

        # If HOST_COHERENT, automatically sync to device
        if MemoryType.HOST_COHERENT in buffer.memory_type:
            data = bytes(self._buffer_data[buffer.buffer_id])
            self._device.memcpy_host_to_device(buffer.device_address, data)

        buffer.mapped = False

    def flush(self, buffer: Buffer, offset: int = 0, size: int = 0) -> None:
        """Flush CPU writes to make them visible to GPU.

        Only needed for HOST_VISIBLE buffers without HOST_COHERENT.
        For HOST_COHERENT buffers, writes are automatically visible.

        Args:
            buffer: The buffer to flush.
            offset: Start of the region to flush.
            size:   Size of the region (0 = whole buffer).
        """
        if buffer.freed:
            raise ValueError(f"Cannot flush freed buffer {buffer.buffer_id}")
        actual_size = size if size > 0 else buffer.size
        data = bytes(
            self._buffer_data[buffer.buffer_id][offset : offset + actual_size]
        )
        self._device.memcpy_host_to_device(buffer.device_address + offset, data)

    def invalidate(self, buffer: Buffer, offset: int = 0, size: int = 0) -> None:
        """Invalidate CPU cache so GPU writes become visible to CPU.

        After the GPU writes to a buffer and you want to read it back
        from the CPU, call invalidate() first to pull the latest data.

        Args:
            buffer: The buffer to invalidate.
            offset: Start of the region to invalidate.
            size:   Size of the region (0 = whole buffer).
        """
        if buffer.freed:
            raise ValueError(f"Cannot invalidate freed buffer {buffer.buffer_id}")
        actual_size = size if size > 0 else buffer.size
        data, _cycles = self._device.memcpy_device_to_host(
            buffer.device_address + offset, actual_size
        )
        self._buffer_data[buffer.buffer_id][
            offset : offset + actual_size
        ] = data

    def get_buffer(self, buffer_id: int) -> Buffer:
        """Look up a buffer by ID.

        Raises:
            ValueError: If buffer not found.
        """
        if buffer_id not in self._buffers:
            raise ValueError(f"Buffer {buffer_id} not found")
        return self._buffers[buffer_id]

    @property
    def allocated_buffer_count(self) -> int:
        """Number of currently allocated buffers."""
        return len(self._buffers)

    @property
    def current_allocated_bytes(self) -> int:
        """Current total bytes allocated."""
        return self._current_bytes

    def _get_buffer_data(self, buffer_id: int) -> bytearray:
        """Internal: get raw data for a buffer."""
        return self._buffer_data[buffer_id]

    def _sync_buffer_to_device(self, buffer: Buffer) -> int:
        """Internal: push buffer data to device. Returns cycles consumed."""
        data = bytes(self._buffer_data[buffer.buffer_id])
        return self._device.memcpy_host_to_device(buffer.device_address, data)

    def _sync_buffer_from_device(self, buffer: Buffer) -> int:
        """Internal: pull buffer data from device. Returns cycles consumed."""
        data, cycles = self._device.memcpy_device_to_host(
            buffer.device_address, buffer.size
        )
        self._buffer_data[buffer.buffer_id][:] = data
        return cycles
